"""Host de ensayo ConPTY (sin terminal gráfica propia)."""
from pathlib import Path
import sys
import time

wheel, node, launcher = sys.argv[1:]
sys.path.insert(0, wheel)
from winpty import PtyProcess, Backend

# El wrapper usa `backend or env` y ConPTY tiene valor 0: pasar "0" evita autodetección.
pty = PtyProcess.spawn([node, launcher, '--isolated'], backend=str(Backend.ConPTY), dimensions=(30,100))
while pty.isalive():
    text = pty.read(8192)
    if '\x1b[' in text:
        pty.setwinsize(32,110)
        Path('pty.ok').touch()
        break
while pty.isalive():
    time.sleep(.2)
