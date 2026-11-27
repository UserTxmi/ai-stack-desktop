"""Puerta G2: dos GPUI publicadas, teclado de ventana y lectura inocua por un hijo real."""
import ctypes as C
from ctypes import wintypes as W
import hashlib
import json
from pathlib import Path
import shutil
import time

from prepare import check, env_for
from winjob import Job


def wait(test,seconds,message):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        if value:=test(): return value
        time.sleep(.2)
    raise RuntimeError(message)


def window(pid):
    u=C.windll.user32
    found=[]
    @C.WINFUNCTYPE(W.BOOL,W.HWND,W.LPARAM)
    def visit(hwnd,_):
        owner=W.DWORD(); u.GetWindowThreadProcessId(W.HWND(hwnd),C.byref(owner))
        if owner.value==pid and u.IsWindowVisible(W.HWND(hwnd)): found.append(hwnd)
        return True
    u.EnumWindows(visit,0)
    return found[0] if found else None


def verify(root, project, node, env, sources, report):
    r=report['g2']={}; ext=root/'externo'; ext.mkdir()
    foreign = env_for(ext/'home',node)
    exe=next((root/'gpui').rglob('gentle-shell-gpui.exe')); external_exe=ext/exe.name
    shutil.copyfile(exe,external_exe)
    eh=ext/'agent'; eh.mkdir(); (eh/'sessions').mkdir()
    sentinels={eh/'auth.json':b'{"outside":{"type":"api_key","key":"G2-EXTERNO"}}',
               eh/'settings.json':b'{"defaultProvider":"outside"}',eh/'sessions/test.jsonl':b'{}\n'}
    for path,data in sentinels.items(): path.write_bytes(data)
    foreign.update(GENTLE_SHELL_GPUI_DATA_DIR=str(ext/'ui'), GENTLE_SHELL_HOME=str(eh),
                   GENTLE_PI_CONFIG_HOME=str(ext/'config'), GENTLE_SHELL_BIN=env['GENTLE_SHELL_BIN'],
                   PI_OFFLINE='1',GENTLE_SHELL_NO_AUTO_SETUP='1',ENGRAM_STARTUP_POLICY='disabled')
    def record(folder,j):
        path=folder/'instance.json'
        if path.exists():
            rec=json.loads(path.read_bytes())
            if rec['pid'] in j.pids() and j.owns_listener(rec['port']): return rec
    def snapshot(): return {str(p):hashlib.sha256(p.read_bytes()).digest() for p in sentinels}
    outside=Job(); parent=sources['main']; child=sources['child']; errors=[]
    try:
        outside.launch([external_exe,ext],ext,foreign)
        external=wait(lambda:record(ext/'ui',outside),60,'sin instancia externa')
        pristine=snapshot()
        for phase in ('deny','allow','active-close'):
            ours=Job(); r[phase]={}; rr=r[phase]
            parent.delegate=True; child.consent_answer=None; child.consent_offered=False
            # Fuera del proyecto, dentro de la raíz de ensayo; ningún fichero del usuario.
            child.consent_path=root/(phase+'.txt') if phase!='active-close' else None
            if child.consent_path: child.consent_path.write_text('G2-CONTENIDO')
            child.mode='cancel' if phase=='active-close' else 'normal'; offset=len(child.events)
            try:
                ours.launch([exe,project,'--prompt','GS-DELEGATE '+phase],project,env)
                rec=wait(lambda:record(root/'desktop-data',ours),60,'sin instancia propia')
                check(rec['port']!=external['port'] and rec['pid']!=external['pid'],'instancia compartida')
                hwnd=wait(lambda:window(rec['pid']),60,'sin ventana propia')
                if phase=='active-close':
                    wait(lambda:any(k=='first' and e=='high' for k,e,_ in child.since(offset)),100,'hijo no activo')
                    check(not any(k=='done' for k,_,_ in child.since(offset)), 'hijo ya terminado')
                    rr['procesos_en_delegacion']=len(ours.pids())
                else:
                    wait(lambda:child.consent_offered,110,'hijo sin petición read'); rr['read_ofrecido']=True
                    time.sleep(2)
                    check(child.consent_answer is None,'read resuelto antes de pulsar tecla: '+str(child.consent_answer))
                    key=ord('Y' if phase=='allow' else 'N')
                    # Enviar a la ventana del PID del Job, nunca al escritorio global.
                    for _ in range(8):
                        C.windll.user32.PostMessageW(W.HWND(hwnd),0x100,key,1)
                        C.windll.user32.PostMessageW(W.HWND(hwnd),0x101,key,0xC0000001)
                        time.sleep(.6)
                        if child.consent_answer: break
                    wait(lambda:child.consent_answer,15,'sin resultado posterior a la tecla')
                    check(child.consent_answer==phase,'resultado inesperado: '+str(child.consent_answer))
                    rr['resultado']=phase
            except RuntimeError as e:
                errors.append(phase); rr['fallo']=str(e)
            finally:
                ours.finish(); rr['job_vacio']=True
            if phase=='active-close' and 'fallo' not in rr:
                wait(lambda:any(k=='cancelled' and e=='high' for k,e,_ in child.since(offset)),18,'receptor hijo no cancelado')
                rr['desconexion']=True
            check(record(ext/'ui',outside)==external and window(external['pid']), 'instancia externa afectada')
            check(snapshot()==pristine, 'configuración externa modificada')
        r['instancia_externa_intacta']=True
    finally:
        outside.finish(); child.consent_path=None; child.mode='normal'
    check(not errors,'G2 pendiente: '+','.join(errors))
