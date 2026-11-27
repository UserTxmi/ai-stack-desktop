"""Bytes de archivos distintos: sin duplicar enlaces duros ni seguir junctions."""
import os
from pathlib import Path
import sys

seen = set()
for name in ('runtimes', 'cache', 'datos', 'preparacion', 'logs', 'config'):
    size = 0
    for root, dirs, files in os.walk(Path(sys.argv[1]) / name):
        dirs[:] = [d for d in dirs if not os.path.isjunction(os.path.join(root, d))]
        for f in files:
            st = os.stat(os.path.join(root, f), follow_symlinks=False)
            key = st.st_dev, st.st_ino
            if key not in seen:
                seen.add(key)
                size += st.st_size
    print(f'DISCO {name}: {size / 2**20:.1f} MiB')
