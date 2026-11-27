"""Host de ensayo ConPTY (sin terminal gráfica propia)."""
from pathlib import Path
import sys
import time

wheel, node, launcher = sys.argv[1:]
sys.path.insert(0, wheel)
from winpty import PtyProcess, Backend

# El wrapper usa `backend or env` y ConPTY tiene valor 0: pasar "0" evita autodetección.
pty = PtyProcess.spawn([node, launcher, '--isolated', 'GS-CONPTY'], backend=str(Backend.ConPTY), dimensions=(30,100))
while pty.isalive():
    text = pty.read(8192)
    if '\x1b[' in text:
        pty.setwinsize(32,110)
        Path('pty.ok').touch()
        # Seguir drenando la salida para no bloquear la TUI por contrapresión.
        while pty.isalive(): pty.read(8192)
