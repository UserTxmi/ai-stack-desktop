"""PoC desechable Windows x64. Publica sólo contadores, marcas y resultados."""
import contextlib
import json
import os
from pathlib import Path
import secrets
import shutil
import socket
import subprocess
import sys
import threading
import time

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent/'phase1'))
from prepare import LAB, RUNTIME, check, env_for, extract_zip, verified, windows
from probes import Client
from provider import FIX, MODEL
from run import dump, ready
from winjob import Job
from fixture import GentleProvider

ASSETS = {
    'engram': ('Gentleman-Programming/engram', 'v3.7.2', 'engram_3.7.2_windows_amd64.zip',
               '515a716ce966202058d4b9f310626fde72583cfc64ccefbf0dcc81456260089c'),
    'git': ('git-for-windows/git', 'v2.56.1.windows.1', 'MinGit-2.56.1-64-bit.zip',
            '4a92850b6e917da4fd010669b614b55dccf5b0ec333a534cc4be4700dfefdafd'),
    'gpui': ('Gentleman-Programming/gentle-shell-gpui', 'v0.1.28', 'gentle-shell-gpui-0.1.28-windows-x86_64.zip',
             'd4294a0dc3b031de82a698377be81f21bbe01b28f95f9b30a44fc8daccbcc8de'),
}


def prepare(root, node):
    for key, (repo, tag, name, digest) in ASSETS.items():
        extract_zip(verified(dict(url=f'https://github.com/{repo}/releases/download/{tag}/{name}',
                                  sha256=digest)), root/key)
    for name in ('package.json', 'package-lock.json'):
        shutil.copy(HERE/name, root/name)
    wheel = verified({'url':'https://files.pythonhosted.org/packages/ce/dd/e5ee2cc1295fc76ad8cd26ae4ec001358edbc9d4c4e7a465c136133d38cf/pywinpty-3.1.2-cp312-cp312-win_amd64.whl',
                      'sha256':'220e30a8cccd7f0be3356d4e3e56fdcda2ae45edc1de526b440c4031fe9308da'})
    extract_zip(wheel,root/'pty')
    env = env_for(root/'build', node)
    subprocess.run([node, node.parent/'node_modules/npm/bin/npm-cli.js', 'ci', '--ignore-scripts',
                    '--no-audit', '--no-fund'], cwd=root, env=env, check=True)
    # El instalador oficial Windows valida el módulo Go, versión, suma y metadatos del EXE.
    env['PATH'] = str(Path(shutil.which('go.exe')).parent)+';'+env['PATH']
    env['TEMP'] = env['TMP'] = str(root/'tmp'); (root/'tmp').mkdir()
    subprocess.run([node, 'scripts/install-gentle-ai.mjs'], cwd=root/'node_modules/gentle-pi', env=env, check=True)


def endpoint():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return Client(s.getsockname()[1])


def run(root, node, report, stack):
    project = root/'Proyecto ñ'
    project.mkdir()
    def job(label):
        j = Job()
        def close():
            j.finish()  # consulta el Job hasta que queda vacío
            report.setdefault('arboles_vacios', []).append(label)
        stack.callback(close)
        return j

    engram = next((root/'engram').rglob('engram.exe'))
    mem = endpoint()
    env = env_for(root/'home', node)
    home = root/'agent'
    config = root/'config'
    home.mkdir(); config.mkdir()
    # MinGit publica un sh POSIX privado, no el bash de PortableGit.
    shell = (root/'git/usr/bin/sh.exe').resolve(strict=True)
    env.update(GENTLE_SHELL_HOME=str(home), GENTLE_PI_CONFIG_HOME=str(config), GENTLE_SHELL_NO_AUTO_SETUP='1',
               GENTLE_SHELL_GPUI_DATA_DIR=str(root/'desktop-data'), GENTLE_SHELL_ARGS='--isolated',
               PI_OFFLINE='1', ENGRAM_DATA_DIR=str(root/'memory'), ENGRAM_PROJECT='gentle-lab',
               ENGRAM_BIN=str(engram), ENGRAM_URL=mem.origin, GENTLE_SHELL_INTERACTIVE_HOST='1',
               PATH=f'{root}/git/cmd;{shell.parent};'+env['PATH'])
    def memory_server(j):
        p = j.launch([engram, 'serve', mem.port], project, env)
        check(ready(j, p, mem, '/health')['service'] == 'engram', 'memoria no disponible')
    first = Job()
    try:
        memory_server(first)
        mem.json('/sessions', {'id': 'gs-memory', 'project': 'gentle-lab'})
        mem.json('/observations', {'session_id':'gs-memory','project':'gentle-lab','type':'decision',
                                   'title':'gsmarker', 'content':'gsmarker synthetic'})
    finally:
        first.finish()
    memory_server(job('engram'))
    check(any(x['title']=='gsmarker' for x in mem.json('/search?q=gsmarker&project=gentle-lab')), 'memoria no persistida')
    report['memoria_tras_reinicio'] = True

    gateway = endpoint()
    password = secrets.token_hex(24)
    omni = RUNTIME/'package/dist'
    oe = env_for(root/'omni-home', node)
    oe.update(PORT=str(gateway.port), HOSTNAME='127.0.0.1', OMNIROUTE_SERVER_HOST='127.0.0.1',
              DATA_DIR=str(root/'omni-data'), NODE_ENV='production', INITIAL_PASSWORD=password,
              REQUIRE_API_KEY='true', JWT_SECRET=secrets.token_hex(32), STORAGE_ENCRYPTION_KEY=secrets.token_hex(32),
              NEXT_PUBLIC_BASE_URL=gateway.origin, NEXT_TELEMETRY_DISABLED='1', ENABLE_REQUEST_LOGS='false', APP_LOG_TO_FILE='false')
    j = job('omni'); ready(j, j.launch([node, omni/'server-ws.mjs'], omni, oe), gateway, '/healthz')
    gateway.json('/api/auth/login', {'password':password}); gateway.csrf = gateway.json('/api/auth/csrf')['token']
    providers, connections = {}, []
    for prefix in ('main','child'):
        key = secrets.token_hex(24)
        src = providers[prefix] = GentleProvider(secrets.token_hex(24))
        threading.Thread(target=src.serve_forever, daemon=True).start()
        stack.callback(src.server_close); stack.callback(src.shutdown); stack.callback(src.stop.set)
        api = endpoint(); api.auth = 'Bearer '+key
        cfg = root/(prefix+'.yaml')
        dump(cfg, {'server':{'host':'127.0.0.1','port':api.port}, 'management':{'disable-control-panel':True},
                   'access':{'api-keys':[key]}, 'oauth':{'auth-dir':str(root/(prefix+'-auth'))},
                   'api-keys':{'openai-compatibility':[{'name':'fixture','base-url':f'http://127.0.0.1:{src.server_port}/v1',
                   'keys':[{'api-key':src.key}], 'models':[{'name':MODEL,'alias':MODEL,'thinking':{'levels':FIX['efforts']}}]}]}})
        j = job('cpa-'+prefix)
        ready(j, j.launch([next((RUNTIME/'cpa').rglob('cli-proxy-api.exe')), '-config',cfg], project,
                          env_for(root/(prefix+'-home'),node)), api, '/v1/models')
        nid = gateway.json('/api/provider-nodes', {'name':prefix,'type':'openai-compatible','apiType':'chat',
                           'prefix':prefix,'baseUrl':api.origin+'/v1'})['node']['id']
        conn = gateway.json('/api/providers', {'provider':nid,'name':prefix,'apiKey':key})['connection']['id']
        connections.append(conn)
        gateway.json(f'/api/providers/{conn}/sync-models?mode=import', {})
        for k,v in [('reasoning_efforts',','.join(FIX['efforts'])),('context_length',32768),('max_output_tokens',2048)]:
            gateway.json('/api/model-capability-overrides', {'target':prefix+'/'+MODEL,'key':k,'value':v}, method='PATCH')
    env['GS_INFERENCE'] = gateway.json('/api/keys',{'name':'gentle','allowedConnections':connections})['key']
    inference = Client(gateway.port, 'Bearer '+env['GS_INFERENCE'])
    targets = {p+'/'+MODEL for p in providers}
    models = []
    for row in inference.json('/v1/models')['data']:
        if row['id'] not in targets: continue
        tiers = row['capabilities']['effort_tiers']
        check(set(tiers)==set(FIX['efforts']), 'tiers del gateway distintos')
        models.append(dict(id=row['id'], reasoning=True, contextWindow=row['context_length'], maxTokens=row['max_output_tokens'],
                           thinkingLevelMap={v:v if v in tiers else None for v in ('off','minimal','low','medium','high','xhigh','max')}))
    check({m['id'] for m in models}==targets, 'faltan rutas en el catálogo')
    dump(home/'models.json', {'providers':{'omniroute':{'api':'openai-completions','baseUrl':inference.origin+'/v1',
         'apiKey':'$GS_INFERENCE','compat':{'thinkingFormat':'openai','supportsReasoningEffort':True},'models':models}}})
    pkg = root/'node_modules/gentle-pi'; launcher = pkg/'bin/gentle-shell.mjs'
    dump(home/'settings.json', {'defaultProvider':'omniroute','defaultModel':'main/'+MODEL,'defaultThinkingLevel':'low',
         'enabledModels':['omniroute/*'], 'shellPath':str(shell), 'packages':[str(root/'node_modules/gentle-engram')]})
    dump(config/'models.json', {'gentle-ai-explore':{'model':'omniroute/child/'+MODEL,'thinking':'high'}})
    report['catalogo_gateway'] = sorted(targets)
    sessions = {}
    failures = []
    for phase in ('low','high','resume','delegate','cancel'):
        print('Ensayo:',phase,flush=True)
        src = providers['main']; src.delegate = phase=='delegate'; src.mode = 'cancel' if phase=='cancel' else 'normal'
        offsets = {p:len(f.markers) for p,f in providers.items()}; event_offset = len(src.events)
        j = Job()
        try:
            result = root/(phase+'.json')
            args = ['--session',sessions['low']['sessionFile']] if phase=='resume' else []
            proc = j.launch([node,HERE/'rpc.mjs',launcher,phase,result,*args],project,env)
            peak = count = 0; cpu = {}; start = time.monotonic()
            while proc.poll() is None and time.monotonic()-start < 125:
                rows = j.sample(); peak=max(peak,sum(x['private_bytes'] for x in rows)); count=max(count,len(rows))
                for x in rows: cpu[(x['pid'],x['created'])]=x['cpu_s']
                time.sleep(.2)
            report.setdefault('muestras',{})[phase] = dict(privados_pico_MiB=round(peak/2**20,1),procesos_max=count,
                                                        cpu_s=round(sum(cpu.values()),2),duracion_s=round(time.monotonic()-start,2))
            saved = json.loads(result.read_bytes()) if result.exists() else {}
            check(proc.poll()==0 and saved and 'failure' not in saved, f"RPC {phase}: exit={proc.poll()}, linea={saved.get('failure','sin resultado')}")
            sessions[phase]=saved
            observed = {p:f.markers[offsets[p]:] for p,f in providers.items()}
            if phase=='resume':
                check(saved['sessionId']==sessions['low']['sessionId'], 'sesión distinta')
            else:
                effort = 'high' if phase=='high' else 'low'
                check(observed['main'] and all('GS-'+phase.upper() in tags and v==effort for tags,v in observed['main']), 'atribución principal')
                if phase=='delegate': check(observed['child']==[(['GS-CHILD'],'high')], 'atribución hijo')
                else: check(not observed['child'] and len(observed['main'])==1, 'despacho inesperado')
                if phase=='cancel':
                    until=time.monotonic()+10
                    while not any(e[0]=='cancelled' for e in src.since(event_offset)) and time.monotonic()<until: time.sleep(.1)
                    check(any(e[0]=='cancelled' for e in src.since(event_offset)), 'receptor no desconectado')
                check(Path(saved['sessionFile']).resolve(strict=True).is_relative_to(home), 'sesión fuera del perfil')
            check(not j.pids(), 'descendientes tras EOF')
            report[phase]='aprobada'
        except (RuntimeError, KeyError) as exc:
            report[phase]=str(exc); failures.append(phase)
        finally:
            j.finish()
    # Mismo perfil y proyecto, ventanas separadas. Arranque observable, no un benchmark de usabilidad.
    env['GENTLE_SHELL_BIN'] = str(root/'node_modules/.bin/gentle-shell.cmd')
    providers['main'].mode = 'normal'
    for label, cmd in [('conpty',[sys.executable,'-IB',HERE/'pty_probe.py',root/'pty',node,launcher]),
                       ('gpui',[next((root/'gpui').rglob('gentle-shell-gpui.exe')),project,'--prompt','GS-GPUI'])]:
        env.pop('GENTLE_SHELL_INTERACTIVE_HOST',None)
        j = Job()
        try:
            offset=len(providers['main'].markers)
            worker = j.launch(cmd,project,env)
            # Un PE vivo o un byte VT al arrancar no prueban que la UI comuniqué con el agente.
            deadline=time.monotonic()+90
            marker='GS-'+label.upper()
            def received(): return any(marker in tags and v=='low' for tags,v in providers['main'].markers[offset:])
            while time.monotonic()<deadline and worker.poll() is None and not received(): time.sleep(.25)
            if received(): time.sleep(2)
            rows = j.sample()
            report[label] = dict(procesos=len(rows), privados_MiB=round(sum(x['private_bytes'] for x in rows)/2**20,1),
                                 cpu_s=round(sum(x['cpu_s'] for x in rows),2),activo=worker.poll() is None, turno_en_receptor=received())
            if label=='conpty': report[label]['salida_vt']=(project/'pty.ok').exists()
            if not received() or worker.poll() is not None or (label=='conpty' and not report[label]['salida_vt']): failures.append(label)
        finally:
            j.finish(); report.setdefault('arboles_vacios',[]).append(label)
    check(not failures,'ensayos fallidos: '+','.join(failures))
    from g2 import verify
    verify(root,project,node,env,providers,report)


if __name__=='__main__':
    windows()
    root = Path(os.environ['RUNNER_TEMP'])/'gs'; root.mkdir(exist_ok=False)
    node = next((RUNTIME/'node').rglob('node.exe'))
    report = {}
    try:
        prepare(root,node)
        with contextlib.ExitStack() as stack: run(root,node,report,stack)
    finally:
        dump(LAB/'gentle-public.json',report)
        print(json.dumps(report, ensure_ascii=True), flush=True)
