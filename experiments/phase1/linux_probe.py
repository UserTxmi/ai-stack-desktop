"""Diagnóstico W1 en bwrap, sin montar HOME, /work, /run ni la red del host."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
from prepare import ART, LAB, ROOT, WORKSPACES, check, verified

BOX = LAB/'w1-linux'
BIN = {
    'node': ('https://nodejs.org/dist/v24.21.0/node-v24.21.0-linux-x64.tar.xz', 'fd8e59d5a511510f6a298afb548f18c7d2b1be404d8b4a27d94fbe49f56cb2d6'),
    'cpa': ('https://github.com/router-for-me/CLIProxyAPI/releases/download/v8.1.7/CLIProxyAPI_8.1.7_linux_amd64.tar.gz','c2ffd3845c15a100a6ac5e2a2c192aca81d617033652bf2499e40783029de149'),
    'oc': ('https://github.com/anomalyco/opencode/releases/download/v1.19.6/opencode-linux-x64.tar.gz','47734e08968b30cc22a0bffb6d9feeed76ad4aa743213a12122040392d09a057'),
}


def prepare():
    BOX.mkdir(exist_ok=False)
    for name,(url,digest) in BIN.items():
        with tarfile.open(verified(dict(url=url,sha256=digest))) as tar:
            tar.extractall(BOX/name, filter='data')
    paths = {a['id']:verified(a) for a in ART if a['id'] in ('omni','workspaces','lock')}
    with tarfile.open(paths['omni']) as tar:
        tar.extractall(BOX,filter='data')
    omni = BOX/'package'
    with tarfile.open(paths['workspaces'], mode='r|gz') as tar:
        for m in tar:
            p = m.name.partition('/')[2]
            if m.isfile() and (p in WORKSPACES or p.startswith(('packages/ui-kit/','packages/browser-pool/'))):
                target = (omni/p).resolve()
                check(target.is_relative_to(omni), 'ruta inválida')
                target.parent.mkdir(parents=True,exist_ok=True)
                target.write_bytes(tar.extractfile(m).read())
    shutil.copy(paths['lock'],omni/'package-lock.json')
    shutil.copytree(ROOT/'peers',BOX/'peers')
    node, = (BOX/'node').rglob('bin/node')
    home = BOX/'home'
    home.mkdir()
    env = dict(PATH=str(node.parent)+':/usr/bin',HOME=str(home),npm_config_cache=str(BOX/'cache'),
               npm_config_userconfig=str(home/'empty.npmrc'),npm_config_registry='https://registry.npmjs.org')
    for folder in (omni,BOX/'peers'):
        subprocess.run([node,node.parent/'npm','ci','--omit=dev','--ignore-scripts','--no-audit','--no-fund',
                        *(['--legacy-peer-deps'] if folder==omni else [])], cwd=folder,env=env,check=True)


def isolated(args):
    # El script interno comparte las sondas con Windows, no las garantías de un Job Win32.
    command = ['bwrap','--die-with-parent','--unshare-all','--ro-bind','/usr','/usr','--symlink','usr/lib','/lib',
               '--symlink','usr/lib','/lib64','--symlink','usr/bin','/bin','--dev','/dev','--proc','/proc',
               '--tmpfs','/tmp','--dir','/etc','--ro-bind','/etc/ssl','/etc/ssl','--dir','/lab',
               '--bind',str(BOX),'/box','--chdir','/lab','--clearenv','--setenv','HOME','/box/home',
               '--setenv','PATH','/usr/bin','--setenv','TMPDIR','/tmp']
    for p in [*ROOT.glob('*.py'),ROOT/'fixtures.json',ROOT/'versions.json']:
        command.extend(['--ro-bind',str(p),'/lab/'+p.name])
    for p in (ROOT/'peers').glob('*.?js'):
        command.extend(['--ro-bind',str(p),'/box/peers/'+p.name])
    subprocess.run(command+['/usr/bin/python3','-B','/lab/linux_probe.py','inside',*args],check=True)


def inside():
    import socket
    import signal
    import run
    check(socket.if_nameindex()==[(1,'lo')] and Path('/box').is_dir(), 'requiere namespace privado')
    box = Path('/box')
    run.windows = lambda: None
    run.LAB = box
    run.RUNTIME = box
    node, = (box/'node').rglob('bin/node')
    oc, = (box/'oc').rglob('opencode')
    cpa, = (box/'cpa').rglob('cli-proxy-api')
    run.binaries = lambda: (node,oc,cpa)
    def child_env(home, _node):
        home.mkdir(parents=True)
        return dict(PATH=str(node.parent)+':/usr/bin', HOME=str(home),XDG_DATA_HOME=str(home/'data'),
                    XDG_CACHE_HOME=str(home/'cache'),XDG_CONFIG_HOME=str(home/'config'),XDG_STATE_HOME=str(home/'state'))
    class Group:
        def __init__(self): self.proc = None
        def launch(self,argv,cwd,env):
            self.proc = subprocess.Popen(list(map(str,argv)),cwd=cwd,env=env,start_new_session=True,
                                         stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            return self.proc
        def finish(self):
            if self.proc is not None:
                os.killpg(self.proc.pid,signal.SIGTERM)
                try: self.proc.wait(4)
                except subprocess.TimeoutExpired:
                    os.killpg(self.proc.pid,signal.SIGKILL)
                    self.proc.wait(2)
        def owns_listener(self,port): return True  # sólo namespace desechable; sin atribución Win32
        def sample(self): return []
    run.env_for, run.Job = child_env, Group
    (box/'prepared').touch()
    run.main()


if __name__=='__main__':
    cmd = sys.argv.pop(1)
    if cmd=='prepare': prepare()
    elif cmd=='fix':
        from omni_queue_fix import apply_to
        apply_to(BOX/'package/dist')
    elif cmd=='inside': inside()
    elif cmd=='probe': isolated(['--isolated-offline','--focus',*sys.argv[1:]])
    else: raise SystemExit('prepare | probe')
