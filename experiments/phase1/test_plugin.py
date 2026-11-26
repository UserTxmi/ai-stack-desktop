"""Copiar peers/ a un directorio privado y ejecutar allí npm ci antes de esta sonda."""
import json
from pathlib import Path
import secrets
import subprocess
import sys
import threading
from provider import FIX, MODEL, Provider

key = secrets.token_hex(20)
with Provider(key) as fixture:
    threading.Thread(target=fixture.serve_forever, daemon=True).start()
    try:
        subprocess.run([sys.argv[1], Path(sys.argv[2]).resolve()/'probe.mjs'], check=True, timeout=15,
                       input=json.dumps(dict(port=fixture.server_port, key=key, model=MODEL, efforts=FIX['efforts'])), text=True)
    finally:
        fixture.shutdown()
