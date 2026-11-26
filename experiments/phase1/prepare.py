"""Adquirir solo artefactos fijados; preparar dependencias en la VM desechable."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent
LAB = ROOT / '.lab'
RUNTIME = LAB / 'runtime'
ART = json.loads((ROOT / 'versions.json').read_text())['artifacts']
WORKSPACES = {'package.json': '943c419b3d1e9e81dc87ddda4f9e1fe888fd95de908c2b7bccb13bfbdf4978e0',
              'open-sse/package.json': '8ab2e596cfb8782ea0e20dacb4aa82babb81b7c3bed230076271227e58162817',
              'packages/browser-pool/package.json': '6d9cc996bac5f874b941c6e2c4e7a6bf01dcb96f1d507587333373d608dcddb2',
              'packages/ui-kit/package.json': '0c27fa221bdb35f7cb41f7c13cd50c4e6e9296a267f9afc580106f5cc635b4bd'}


def check(value, message):
    if not value:
        raise RuntimeError(message)


def windows():
    check(os.name == 'nt' and platform.machine().lower() in ('amd64', 'x86_64'), 'requiere Windows x64')


def verified(artifact):
    url = artifact['url']
    dest = LAB / 'downloads' / url.split('/')[-1]
    dest.parent.mkdir(parents=True, exist_ok=True)
    source = dest if dest.exists() else dest.with_suffix(dest.suffix + '.part')
    if source != dest:
        with urllib.request.urlopen(url, timeout=90) as response, source.open('wb') as out:
            shutil.copyfileobj(response, out)
    alg = 'sha256' if 'sha256' in artifact else 'sha512'
    with source.open('rb') as stream:
        digest = hashlib.file_digest(stream, alg)
    actual = digest.hexdigest() if alg == 'sha256' else 'sha512-' + base64.b64encode(digest.digest()).decode()
    check(actual == artifact.get('sha256', artifact.get('sri')), 'hash incorrecto: ' + url)
    if source != dest:
        source.replace(dest)
    return dest


def env_for(home, node):
    # No heredar credenciales, proxy, PATH de runtimes, npmrc, plugins ni perfil de usuario.
    env = {k: os.environ[k] for k in ('SystemRoot', 'WINDIR', 'COMSPEC', 'PATHEXT', 'PROCESSOR_ARCHITECTURE') if k in os.environ}
    for k, sub in {'USERPROFILE': '.', 'HOME': '.', 'APPDATA': 'roam', 'LOCALAPPDATA': 'local', 'TEMP': 'tmp', 'TMP': 'tmp',
                   'XDG_CONFIG_HOME': 'config', 'XDG_DATA_HOME': 'data', 'XDG_CACHE_HOME': 'cache', 'XDG_STATE_HOME': 'state'}.items():
        path = home / sub
        path.mkdir(parents=True, exist_ok=True)
        env[k] = str(path)
    env.update(PATH=os.pathsep.join(map(str, [node.parent, Path(env['SystemRoot']) / 'System32', env['SystemRoot']])),
               npm_config_cache=str(home / 'npm'), npm_config_userconfig=str(home / 'empty.npmrc'),
               npm_config_registry='https://registry.npmjs.org', npm_config_update_notifier='false')
    return env


def extract_zip(archive, directory):
    with zipfile.ZipFile(archive) as z:
        for member in z.namelist():
            check((directory / member).resolve().is_relative_to(directory), 'ruta fuera del ZIP')
        z.extractall(directory)


def main():
    opts = argparse.ArgumentParser()
    opts.add_argument('--inventory', action='store_true', help='verificar en disco sin ejecutar binarios')
    opts.add_argument('--ci-stage', choices=['binaries', 'gateway'], help='etapas separadas en runner efímero')
    args = opts.parse_args()
    if not args.inventory:
        windows()
    selected = [a for a in ART if not args.ci_stage or a['id'] != 'electron']
    if args.ci_stage == 'binaries':
        selected = [a for a in selected if a['id'] in ('node','oc','cpa')]
    files = {a['id']: verified(a) for a in selected}
    if not args.inventory and args.ci_stage != 'gateway':
        RUNTIME.mkdir(parents=True, exist_ok=False)
        for key in ('node', 'oc', 'cpa'):
            extract_zip(files[key], RUNTIME / key)
        (RUNTIME / 'binaries').touch()
        if args.ci_stage == 'binaries':
            return
    lock = json.loads(files['lock'].read_text(encoding='utf-8'))
    check(lock['packages']['node_modules/better-sqlite3']['integrity'] == next(a['sri'] for a in ART if a['id'] == 'sqlite'), 'SQLite no coincide con el lock')
    peers = json.loads((ROOT / 'peers/package-lock.json').read_text())['packages']
    check(peers['node_modules/@omniroute/opencode-plugin']['integrity'] == next(a['sri'] for a in ART if a['id'] == 'plugin'), 'plugin no coincide con el lock')
    print('Inventario íntegro:', ', '.join(files), flush=True)
    if args.inventory:
        return
    check((RUNTIME/'binaries').is_file(), 'preparar antes los binarios')
    node, = (RUNTIME / 'node').rglob('node.exe')
    npm = node.parent / 'node_modules/npm/bin/npm-cli.js'
    env = env_for(LAB / 'build-home', node)
    omni = RUNTIME / 'package'
    with tarfile.open(files['omni']) as tar:
        tar.extractall(RUNTIME, filter='data')
    # npm ci enlaza tres workspaces; el paquete publicado omite dos de sus directorios.
    with tarfile.open(files['workspaces'], mode='r|gz') as tar:
        for item in tar:
            path = item.name.partition('/')[2]
            if not item.isfile() or not (path in WORKSPACES or path.startswith(('packages/browser-pool/', 'packages/ui-kit/'))):
                continue
            target = (omni / path).resolve()
            check(target.is_relative_to(omni), 'ruta workspace inválida')
            target.parent.mkdir(exist_ok=True, parents=True)
            target.write_bytes(tar.extractfile(item).read())
    for path, digest in WORKSPACES.items():
        check(hashlib.sha256((omni / path).read_bytes()).hexdigest() == digest, 'manifiesto workspace alterado')
    shutil.copy(files['lock'], omni / 'package-lock.json')
    peers_dir = RUNTIME / 'peers'
    shutil.copytree(ROOT / 'peers', peers_dir)
    for folder in (omni, peers_dir):
        # El lock de OmniRoute se generó con legacy-peer-deps=true (v3.8.54/.npmrc).
        policy = ['--legacy-peer-deps'] if folder == omni else []
        subprocess.run([node, npm, 'ci', *policy, '--omit=dev', '--include=optional', '--ignore-scripts', '--no-audit', '--no-fund'],
                       cwd=folder, env=env, check=True)
    # Misma ubicación que fixBetterSqliteBinary upstream, sin ejecutar un postinstall que pueda recompilar.
    addon = omni / 'node_modules/better-sqlite3/prebuilds/win32-x64.node'
    with tarfile.open(files['sqlite']) as tar:
        check(addon.read_bytes() == tar.extractfile('package/prebuilds/win32-x64.node').read(), 'prebuild distinto')
    for directory in (omni / 'dist/node_modules/better-sqlite3', omni / 'node_modules/better-sqlite3'):
        if directory.exists():
            target = directory / 'build/Release/better_sqlite3.node'
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(addon, target)
    subprocess.run([node, '-e', "const db=require('better-sqlite3')(':memory:'); if(db.prepare('select 41+1 as n').get().n!==42)process.exit(1); db.close();console.log(process.version,process.arch,process.versions.modules)"],
                   cwd=omni / 'dist', env=env, check=True)
    (RUNTIME / 'prepared').touch()


if __name__ == '__main__':
    main()
