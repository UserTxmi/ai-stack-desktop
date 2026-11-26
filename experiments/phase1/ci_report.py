"""Sólo agrega mediciones y campos enumerados: nunca exportar perfiles o texto de errores."""
import json
import math
import platform
import re
from pathlib import Path
from prepare import LAB

CASES = set('CPA-perfil OmniRoute-salud gateway-aprovisionado catalogo-OmniRoute gateway-normal gateway-late gateway-body gateway-cancel control-cola OpenCode-salud plugin-efectivo turno-low turno-high metricas cierre-omni cierre-oc cierre-cpa'.split())


def summary(run):
    tests, peaks = {}, {}
    file = run/'results.json'
    for k, v in (json.loads(file.read_text(encoding='utf8')) if file.exists() else {}).items():
        if k in ('diagnostico-variantes','diagnostico-low','diagnostico-high'):
            key = 'observadas' if k == 'diagnostico-variantes' else 'recibidos'
            tests[k] = {key:[s for s in v.get(key,[]) if s in ('low','medium','high','none','minimal','xhigh',None)][:20]}
            continue
        if k not in CASES or v.get('estado') not in ('aprobada','fallida'):
            continue
        tests[k] = {'estado':v['estado']}
        if v['estado']=='fallida':
            msg = v.get('error','')
            tests[k]['clase'] = next((tag for tag, phrase in {
                'cola_sin_expirar':'no expiró en cola','rechazo_no_demostrado':'rechazo no atribuible',
                'modelo_ausente':'no entregó el modelo','tiers_distintos':'variantes no coinciden',
                'esfuerzo_distinto':'esfuerzo no llegó','turno_sin_respuesta':'turno sin respuesta',
                'presupuesto_no_observado':'plazo de cola no observado'}.items() if phrase in msg),'otro')
            tests[k]['codigos'] = re.findall(r'HTTP [45]\d\d|RATE_LIMIT_(?:QUEUE|EXECUTION)_TIMEOUT',msg)[:3]
        for field in ('cabeceras_s','primer_contenido_s','total_s','espera_s','cola_ms','ejecucion_configurada_ms','linea'):
            n = v.get(field)
            if type(n) in (int,float) and math.isfinite(n) and n >= 0:
                tests[k][field] = n
        if k.startswith('turno-') and isinstance(v.get('recibidos'),list):
            tests[k]['recibidos'] = [s for s in v['recibidos'] if s in ('low','high',None)][:20]
    metrics = run/'metrics.jsonl'
    if metrics.exists():
        with metrics.open(encoding='utf8') as rows:
            for line in rows:
                r = json.loads(line)
                if r['role'] not in ('oc','cpa','omni'):
                    continue
                peak = peaks.setdefault(r['role'], {})
                for k in ('private_bytes','working_set','cpu_s'):
                    values = [p[k] for p in r['processes']]
                    if all(type(n) in (int,float) and math.isfinite(n) and n >= 0 for n in values):
                        peak[k] = max(peak.get(k,0), sum(values))
                peak['procesos'] = max(peak.get('procesos',0),len(r['processes']))
    return {'casos':tests,'maximos_por_muestra':peaks}


if __name__ == '__main__':
    out = LAB/'export'
    out.mkdir(parents=True,exist_ok=True)
    report = dict(os=platform.version(), arch=platform.machine(), offline=False,
                  sqlite=(LAB/'runtime/prepared').is_file(), ensayos=[summary(p) for p in sorted(LAB.glob('run-*'))])
    (out/'informe.json').write_text(json.dumps(report,indent=2),encoding='utf8')
