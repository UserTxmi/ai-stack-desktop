"""Ensayo Windows: VM offline o runner efímero de CI con proveedor sintético."""
import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
import contextlib
import json
from pathlib import Path
import secrets
import socket
import threading
import time
import traceback

from prepare import LAB, RUNTIME, check, env_for, windows
from probes import Client, offline_ui, stream
from provider import FIX, MODEL, Provider
from winjob import Job


def dump(path, value):
    path.write_text(json.dumps(value, indent=2), encoding='utf-8')


def ready(job, worker, client, path):
    deadline = time.monotonic()+90
    last = None
    while time.monotonic() < deadline:
        check(worker.poll() is None, f'proceso terminó durante {path}: {worker.returncode}')
        if not job.owns_listener(client.port):
            time.sleep(.2)
            continue
        try:
            if path == '/healthz':
                c, r = client.request(path, timeout=2)
                try:
                    check(r.read(64).strip() == b'ok', 'gateway no listo')
                    return
                finally:
                    r.close()
                    c.close()
            return client.json(path, timeout=2)
        except (OSError, ValueError, RuntimeError) as e:
            last = e
        time.sleep(.25)
    raise RuntimeError(f'no listo: {path} ({last})')


def main():
    parser = argparse.ArgumentParser()
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--isolated-offline', action='store_true')
    modes.add_argument('--hosted-ci', action='store_true')
    parser.add_argument('--interactive', action='store_true')
    parser.add_argument('--only', choices=['cpa', 'oc', 'omni', 'all'], default='all')
    args = parser.parse_args()
    windows()
    check(not args.hosted_ci or not args.interactive, 'CI no admite consola interactiva')
    marker = 'binaries' if args.only in ('cpa','oc') else 'prepared'
    check((RUNTIME/marker).is_file(), 'ejecutar primero prepare.py')
    run = LAB / ('run-' + secrets.token_hex(6))
    run.mkdir()
    project = run/'Proyecto de ensayo ñ'
    project.mkdir()
    results = {}
    print('Evidencia de esta ejecución:', run, flush=True)
    dump(run/'results.json', results)

    def test(name, fn):
        try:
            value = fn()
            results[name] = {'estado': 'aprobada', **(value if isinstance(value, dict) else {})}
        except Exception as exc:
            results[name] = {'estado': 'fallida', 'error': str(exc)[:280] if isinstance(exc, RuntimeError) else type(exc).__name__,
                             'linea': traceback.extract_tb(exc.__traceback__)[-1].lineno}
            raise
        finally:
            dump(run/'results.json', results)
            print(name, results.get(name, {}).get('estado'), flush=True)

    node, = (RUNTIME/'node').rglob('node.exe')
    oc, = (RUNTIME/'oc').rglob('opencode.exe')
    cpa, = (RUNTIME/'cpa').rglob('cli-proxy-api.exe')
    sourcekey, cpakey, password, ocpass = [secrets.token_hex(24) for _ in range(4)]
    provider = Provider(sourcekey)
    threading.Thread(target=provider.serve_forever, daemon=True).start()
    with contextlib.ExitStack() as stack:
        stack.callback(provider.server_close)
        stack.callback(provider.shutdown)
        stack.callback(provider.stop.set)
        # En el perfil desechable se reservan puertos distintos hasta el spawn correspondiente.
        reserved = []
        for _ in range(3):
            s = stack.enter_context(socket.socket())
            s.bind(('127.0.0.1', 0))
            reserved.append(s)
        api, gateway, agent = [Client(s.getsockname()[1]) for s in reserved]
        api.auth = 'Bearer '+cpakey
        agent.auth = 'Basic '+base64.b64encode(('opencode:'+ocpass).encode()).decode()
        jobs = {}
        stop = threading.Event()
        sampling_errors = []

        def monitor():
            try:
                with (run/'metrics.jsonl').open('w', encoding='utf-8') as f:
                    while not stop.is_set():
                        for role, job in jobs.items():
                            f.write(json.dumps(dict(t=time.monotonic(), role=role, processes=job.sample()))+'\n')
                        f.flush()
                        stop.wait(1)
            except Exception as exc:
                sampling_errors.append(type(exc).__name__)

        for role in (('cpa', 'omni', 'oc') if args.only == 'all' else (args.only,)):
            job = jobs[role] = Job()
            stack.callback(test, 'cierre-'+role, job.finish)
        monitor_thread = threading.Thread(target=monitor)
        monitor_thread.start()
        stack.callback(monitor_thread.join)
        stack.callback(stop.set)
        cpaenv = env_for(run/'home-cpa', node)
        config = run/'cpa.yaml'
        # JSON también es YAML; quoting de rutas Windows sin interpolación.
        dump(config, {'server': {'host': '127.0.0.1', 'port': api.port}, 'management': {'disable-control-panel': True},
                      'access': {'api-keys': [cpakey]}, 'oauth': {'auth-dir': str(run/'cpa-auth')},
                      'api-keys': {'openai-compatibility': [{'name': 'fixture', 'base-url': f'http://127.0.0.1:{provider.server_port}/v1',
                          'keys': [{'api-key': sourcekey}], 'models': [{'name': MODEL, 'alias': MODEL, 'thinking': {'levels': FIX['efforts']}}]}]}})
        reserved[0].close()

        def cpa_probe():
            catalog = ready(jobs['cpa'], worker, api, '/v1/models')
            check(MODEL in [row['id'] for row in catalog['data']], 'CPA no anuncia fixture')
            return stream(api, MODEL, 'normal', provider)
        if args.only in ('cpa', 'all'):
            worker = jobs['cpa'].launch([cpa, '-config', config], project, cpaenv)
            test('CPA-perfil', cpa_probe)
            if args.only == 'cpa':
                return
        omnienv = env_for(run/'home-omni', node)
        omnienv.update(PORT=str(gateway.port), HOSTNAME='127.0.0.1', OMNIROUTE_SERVER_HOST='127.0.0.1',
                       DATA_DIR=str(run/'omni-data'), NODE_ENV='production', INITIAL_PASSWORD=password,
                       REQUIRE_API_KEY='true', JWT_SECRET=secrets.token_hex(32), STORAGE_ENCRYPTION_KEY=secrets.token_hex(32),
                       NEXT_PUBLIC_BASE_URL=gateway.origin, NEXT_TELEMETRY_DISABLED='1', APP_LOG_TO_FILE='false',
                       ENABLE_REQUEST_LOGS='false', CALL_LOG_PIPELINE_CAPTURE_STREAM_CHUNKS='false')
        reserved[1].close()
        omni = RUNTIME/'package/dist'
        def omni_health():
            ready(jobs['omni'], worker, gateway, '/healthz')
            return {'runtime': 'Node privado'}
        if args.only in ('omni', 'all'):
            worker = jobs['omni'].launch([node, omni/'server-ws.mjs'], omni, omnienv)
            test('OmniRoute-salud', omni_health)
            if args.only == 'omni':
                return
        client = Client(gateway.port)
        target = 'labcpa/'+MODEL

        def configure():
            gateway.json('/api/auth/login', {'password': password})
            gateway.csrf = gateway.json('/api/auth/csrf')['token']
            node_id = gateway.json('/api/provider-nodes', {'name': 'CPA laboratorio', 'type': 'openai-compatible',
                'apiType': 'chat', 'prefix': 'labcpa', 'baseUrl': api.origin+'/v1'})['node']['id']
            conn = gateway.json('/api/providers', {'provider': node_id, 'name': 'fixture', 'apiKey': cpakey})['connection']['id']
            check(gateway.json(f'/api/providers/{conn}/test', {'validationModelId': MODEL}).get('valid'), 'validación de conexión fallida')
            gateway.json(f'/api/providers/{conn}/sync-models?mode=import', {})
            # CPA sólo anuncia IDs en su listado genérico: declarar las capacidades CONOCIDAS del fixture.
            for key, val in [('reasoning_efforts', 'low,high'), ('context_length', 32768), ('max_output_tokens', 2048)]:
                gateway.json('/api/model-capability-overrides', {'target': target, 'key': key, 'value': val}, method='PATCH')
            requested = dict(maxWaitMs=15000, executionMaxWaitMs=90000, concurrentRequests=1, autoEnableApiKeyProviders=True)
            gateway.json('/api/resilience', {'requestQueue': requested}, method='PATCH')
            actual = gateway.json('/api/resilience')['requestQueue']
            check(all(actual.get(k) == v for k, v in requested.items()), 'presupuestos efectivos distintos')
            key = gateway.json('/api/keys', {'name': 'fixture', 'allowedConnections': [conn]})['key']
            client.auth = 'Bearer '+key
            return {'cola_ms': actual['maxWaitMs'], 'ejecucion_configurada_ms': actual['executionMaxWaitMs']}
        if args.only == 'all':
            test('gateway-aprovisionado', configure)
        def catalog():
            row = next((m for m in client.json('/v1/models')['data'] if m['id'] == target), None)
            check(row is not None, 'modelo sintético no anunciado por OmniRoute')
            check(set(row.get('capabilities', {}).get('effort_tiers', [])) == set(FIX['efforts']), 'tiers no anunciados por OmniRoute')
            return {'id': target, 'tiers': FIX['efforts']}
        # Un fallo en el catálogo no debe impedir obtener evidencia del streaming independiente.
        failures = []
        def independent(name, fn):
            try:
                test(name, fn)
            except Exception:
                failures.append(name)
        if args.only == 'all':
            independent('catalogo-OmniRoute', catalog)
            for mode in ('normal', 'late', 'body', 'cancel'):
                independent('gateway-'+mode, lambda m=mode: stream(client, target, m, provider))
        def queue_control():
            # Sin cabeceras del proveedor durante 18 s, el primer trabajo aún ocupa el slot.
            offset = len(provider.events)
            with ThreadPoolExecutor(max_workers=1) as pool:
                held = pool.submit(stream, client, target, 'late', provider)
                deadline = time.monotonic()+5
                while not provider.since(offset) and time.monotonic() < deadline and not held.done():
                    time.sleep(.025)
                check(provider.since(offset), 'primera petición no despachada')
                start = time.monotonic()
                try:
                    client.json('/v1/chat/completions', {'model': target, 'stream': False,
                                'messages': [{'role':'user','content':'cola'}]})
                except RuntimeError as e:
                    check('RATE_LIMIT_QUEUE_TIMEOUT' in str(e), 'rechazo no atribuible a la cola: '+str(e))
                else:
                    raise RuntimeError('la segunda petición no expiró en cola')
                waited = time.monotonic()-start
                check(14 <= waited < 18, 'plazo de cola no observado')
                held.result()
            return {'espera_s': round(waited, 3)}
        if args.only == 'all':
            independent('control-cola', queue_control)
        provider.mode = 'normal'
        ocenv = env_for(run/'home-oc', node)
        conf = run/'opencode.json'
        plugin = RUNTIME/'peers/node_modules/@omniroute/opencode-plugin/dist/index.js'
        datadir = Path(ocenv['XDG_DATA_HOME'])/'opencode'
        datadir.mkdir(exist_ok=True, parents=True)
        pid = 'opencode-omniroute'
        if client.auth:
            dump(datadir/'auth.json', {pid: {'type': 'api', 'key': client.auth.removeprefix('Bearer ')}})
        dump(conf, {'permission': 'ask', 'enabled_providers': []} if args.only == 'oc' else {
                   'plugin': [[plugin.as_uri(), {'providerId': 'omniroute', 'baseURL': gateway.origin,
                   'autoSyncIntervalMs': 0, 'features': {'combos': False, 'autoCombos': False, 'enrichment': False,
                   'diskCache': False, 'mcpAutoEmit': False}}]], 'permission': 'ask', 'enabled_providers': [pid]})
        ocenv.update(OPENCODE_CONFIG=str(conf), OPENCODE_SERVER_PASSWORD=ocpass, OPENCODE_DATA_DIR=str(datadir),
                     OPENCODE_DISABLE_AUTOUPDATE='1', OPENCODE_DISABLE_MODELS_FETCH='1')
        reserved[2].close()
        worker = jobs['oc'].launch([oc, 'serve', '--hostname', '127.0.0.1', '--port', agent.port], project, ocenv)
        def oc_health():
            check(ready(jobs['oc'], worker, agent, '/global/health').get('healthy'), 'OpenCode no sano')
        test('OpenCode-salud', oc_health)
        if args.isolated_offline:
            independent('UI-offline-assets', lambda: offline_ui(agent))

        def models():
            found = next((p for p in agent.json('/provider')['all'] if p['id'] == pid), {})
            model = found.get('models', {}).get(target)
            check(model is not None, 'plugin no entregó el modelo')
            check(set(model.get('variants', {})) == set(FIX['efforts']), 'variantes no coinciden con tiers de la fixture')
            return {'modelo': target, 'variantes': sorted(model['variants'])}
        if args.only == 'all':
            independent('plugin-efectivo', models)
        def turn(effort):
            session = agent.json('/session', {})['id']
            offset = len(provider.events)
            response = agent.json(f'/session/{session}/message', {'model': {'providerID': pid, 'modelID': target},
                'variant': effort, 'parts': [{'type': 'text', 'text': 'Devuelve LAB.'}]})
            check(not response['info'].get('error') and any('LAB' in p.get('text', '') for p in response['parts'] if p['type']=='text'), 'turno sin respuesta del fixture')
            received = [x[1] for x in provider.since(offset) if x[0]=='received']
            check(effort in received and not any(x not in (effort, None) for x in received), 'esfuerzo no llegó al receptor')
            return {'seleccion': effort, 'recibidos': received}
        if args.only == 'all':
            for effort in FIX['efforts']:
                independent('turno-'+effort, lambda v=effort: turn(v))
        if args.interactive:
            print('Workspace local:', agent.origin, '| usuario opencode | contraseña efímera:', ocpass)
            print('Métricas:', run/'metrics.jsonl', flush=True)
            while cmd := input('Escenario normal/tools/cancel; Entrar cierra todos los Jobs: ').strip():
                if cmd in ('normal', 'tools', 'cancel'):
                    provider.mode = cmd
                    print('Fixture:', cmd, flush=True)
        def metrics():
            check(not sampling_errors and all(job.sample() for job in jobs.values()), 'faltan muestras PSAPI')
        independent('metricas', metrics)
    print('Evidencia:', run, flush=True)
    check(not failures, 'ensayos fallidos: '+', '.join(failures))


if __name__ == '__main__':
    main()
