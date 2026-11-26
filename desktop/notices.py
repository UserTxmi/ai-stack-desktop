"""Componer avisos del EXE desde el grafo Cargo fijado para MSVC x64, sin incluir runtimes W1."""
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent
ALLOWED = {'MIT', 'Apache-2.0', 'BSD-2-Clause', 'BSD-3-Clause', 'Unicode-3.0', 'MPL-2.0',
           'Unlicense', 'Zlib', '0BSD', 'CC0-1.0', 'MIT-0', 'BSL-1.0'}


def main(out):
    packages = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1',
        '--filter-platform', 'x86_64-pc-windows-msvc'], cwd=ROOT))['packages']
    pages = ['Avisos de terceros · AI Stack Desktop 0.0.1-dev\nFuentes sin modificaciones disponibles en los enlaces de cada versión.']
    for p in packages:
        if p['name'] in ('ai-stack-desktop', 'w1-supervisor') and not p['source']:
            continue
        ids = set(re.findall(r'[A-Za-z0-9.-]+', p['license'] or '')) - {'AND', 'OR'}
        if not ids or ids - ALLOWED:
            raise ValueError('Revisar licencia de '+p['name'])
        root = Path(p['manifest_path']).parent
        name, version = p['name'], p['version']
        pages.append(f'{name} {version} · {p["license"]}\nFuente: https://static.crates.io/crates/{name}/{name}-{version}.crate')
        # Sólo atribución pública de upstream, sin correos ni metadatos locales del build.
        pages.extend(a.split('<')[0].strip() for a in p['authors'])
        for f in sorted(root.iterdir()):
            if f.is_file() and f.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE','COPYRIGHT','UNLICENSE')):
                pages.append(f.read_text(encoding='utf-8',errors='replace'))
        if name == 'webview2-com-sys':
            loader = (root/'x64/WebView2LoaderStatic.lib').read_bytes()
    v = '1.0.3296.44'
    url = f'https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/{v}/microsoft.web.webview2.{v}.nupkg'
    archive = urllib.request.urlopen(url,timeout=45).read()
    if hashlib.sha256(archive).hexdigest() != '5b1b19c266ddff33c837ed7965a95bac3dae69b48b5b27f26e5d69f161e9f534':
        raise ValueError('SDK Microsoft no reconocido')
    with zipfile.ZipFile(io.BytesIO(archive)) as z:
        if z.read('build/native/x64/WebView2LoaderStatic.lib') != loader:
            raise ValueError('WebView2 Loader distinto del SDK revisado')
        pages.extend(['Microsoft.Web.WebView2 '+v, z.read('LICENSE.txt').decode(), z.read('NOTICE.txt').decode()])
    sysroot = Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())
    for license in ('LICENSE-APACHE','LICENSE-MIT','COPYRIGHT'):
        f=sysroot/'share/doc/rust'/license
        if f.is_file(): pages.extend(['Rust: '+license,f.read_text(encoding='utf-8')])
    out.write_text('\n\n'.join(pages),encoding='utf-8')


if __name__ == '__main__':
    main(Path(sys.argv[1]))
