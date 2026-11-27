"""Compilar sólo el auxiliar oficial Windows; el EXE prepara el resto desde cero."""
import json
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent/'experiments/phase1'))
from prepare import ART, check, env_for, extract_zip, verified, windows


def main(root, out):
    windows()
    root.mkdir()
    extract_zip(verified(next(a for a in ART if a['id']=='node')), root/'node')
    node, = (root/'node').rglob('node.exe')
    for name in ('package.json', 'package-lock.json'):
        shutil.copyfile(HERE.parent/'experiments/gentle'/name, root/name)
    env = env_for(root/'home', node)
    subprocess.run([node, node.parent/'node_modules/npm/bin/npm-cli.js', 'ci', '--ignore-scripts',
                    '--no-audit', '--no-fund'], cwd=root, env=env, check=True)
    go = shutil.which('go.exe')
    check(go, 'Falta Go en el runner')
    env['PATH'] = str(Path(go).parent)+';'+env['PATH']
    tmp = root/'tmp'
    tmp.mkdir()
    env['TEMP'] = env['TMP'] = str(tmp)
    pkg = root/'node_modules/gentle-pi'
    subprocess.run([node, 'scripts/install-gentle-ai.mjs'], cwd=pkg, env=env, check=True)
    source = pkg/'.gentle-ai/v5.0.0'
    meta = json.loads((source/'integrity.json').read_bytes())
    check(meta['version']=='5.0.0' and meta['goos']=='windows' and meta['goarch']=='amd64', 'Auxiliar distinto del esperado')
    for name in ('gentle-ai.exe', 'integrity.json'):
        shutil.copyfile(source/name, out/name)


if __name__=='__main__':
    main(*(Path(p).resolve() for p in sys.argv[1:]))
