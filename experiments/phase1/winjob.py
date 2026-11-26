"""Job experimental: un trabajador espera permiso antes de crear el proceso real."""
import ctypes as C
from ctypes import wintypes as W
import json
import os
import socket
import struct
import subprocess
import sys
import time
from pathlib import Path


if __name__ == '__main__':
    # El propietario envía argv SOLO después de AssignProcessToJobObject.
    spec = sys.stdin.buffer.readline()
    if spec:
        raise SystemExit(subprocess.call(json.loads(spec), stdin=subprocess.DEVNULL))
    raise SystemExit(1)

if os.name == 'nt':
    k = C.WinDLL('kernel32', use_last_error=True)
    p = C.WinDLL('psapi', use_last_error=True)
    size = C.c_size_t

    class Limits(C.Structure):
        _fields_ = [('per_process', C.c_int64), ('per_job', C.c_int64), ('flags', W.DWORD),
                    ('min_ws', size), ('max_ws', size), ('processes', W.DWORD), ('affinity', size),
                    ('priority', W.DWORD), ('schedule', W.DWORD)]

    class Extended(C.Structure):
        _fields_ = [('basic', Limits), ('io', C.c_uint64 * 6), ('memory', size * 4)]

    class Memory(C.Structure):
        _fields_ = [('cb', W.DWORD), ('faults', W.DWORD), ('peak_ws', size), ('ws', size),
                    ('pools_and_pagefile', size * 6), ('private', size)]

    class Pids(C.Structure):
        _fields_ = [('assigned', W.DWORD), ('count', W.DWORD), ('ids', size * 512)]

    def api(lib, name, restype, *args):
        fn = getattr(lib, name)
        fn.restype, fn.argtypes = restype, args
        return fn

    H, P, D = W.HANDLE, C.c_void_p, W.DWORD
    create = api(k, 'CreateJobObjectW', H, P, W.LPCWSTR)
    assign = api(k, 'AssignProcessToJobObject', W.BOOL, H, H)
    limits = api(k, 'SetInformationJobObject', W.BOOL, H, C.c_int, P, D)
    query = api(k, 'QueryInformationJobObject', W.BOOL, H, C.c_int, P, D, P)
    terminate = api(k, 'TerminateJobObject', W.BOOL, H, W.UINT)
    close = api(k, 'CloseHandle', W.BOOL, H)
    op = api(k, 'OpenProcess', H, D, W.BOOL, D)
    times = api(k, 'GetProcessTimes', W.BOOL, H, P, P, P, P)
    memory = api(p, 'GetProcessMemoryInfo', W.BOOL, H, P, D)
    tcp = api(C.WinDLL('iphlpapi'), 'GetExtendedTcpTable', D, P, C.POINTER(D), W.BOOL, D, D, D)


def must(result):
    if not result:
        raise C.WinError(C.get_last_error())
    return result


class Job:
    def __init__(self):
        self.handle = must(create(None, None))
        value = Extended()
        value.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        try:
            must(limits(self.handle, 9, C.byref(value), C.sizeof(value)))
        except BaseException:
            close(self.handle)
            raise
        self.workers = []

    def launch(self, argv, cwd, env):
        child = subprocess.Popen([sys.executable, '-IB', str(Path(__file__).resolve())],
                                 cwd=cwd, env=env, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                                 stderr=subprocess.DEVNULL, creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            must(assign(self.handle, int(child._handle)))
            child.stdin.write((json.dumps([str(x) for x in argv]) + '\n').encode())
            child.stdin.close()
            self.workers.append(child)
        except BaseException:
            child.kill()
            child.wait()
            raise
        return child

    def pids(self):
        entries = Pids()
        must(query(self.handle, 3, C.byref(entries), C.sizeof(entries), None))
        return entries.ids[:entries.count]

    def sample(self):
        rows = []
        for pid in self.pids():
            h = op(0x410, False, pid)
            if not h:
                continue  # un proceso efímero pudo terminar entre las dos lecturas
            try:
                t = [C.c_uint64() for _ in range(4)]
                m = Memory()
                m.cb = C.sizeof(m)
                if times(h, *(C.byref(v) for v in t)) and memory(h, C.byref(m), m.cb):
                    rows.append(dict(pid=pid, created=t[0].value, cpu_s=(t[2].value+t[3].value)/1e7,
                                     private_bytes=m.private, working_set=m.ws))
            finally:
                close(h)
        return rows

    def owns_listener(self, port):
        owners = set(self.pids())
        found = False
        # MIB_TCP{,6}ROW_OWNER_PID, clase TCP_TABLE_OWNER_PID_LISTENER.
        for family, width, addr, localport, owner in ((socket.AF_INET, 24, 4, 8, 20),
                                                     (socket.AF_INET6, 56, 0, 20, 52)):
            length = D()
            code = tcp(None, C.byref(length), False, family, 3, 0)
            if code not in (0, 122):
                raise C.WinError(code)
            buf = C.create_string_buffer(length.value)
            code = tcp(buf, C.byref(length), False, family, 3, 0)
            if code == 122:
                return False  # tabla cambió; repetir en el plazo del llamador
            if code:
                raise C.WinError(code)
            count, = struct.unpack_from('<I', buf)
            for i in range(count):
                ix = 4+i*width
                pid, = struct.unpack_from('<I', buf, ix+owner)
                if pid not in owners:
                    continue
                raw = buf.raw[ix+addr:ix+addr+(4 if family == socket.AF_INET else 16)]
                ip = socket.inet_ntop(family, raw)
                if ip not in ('127.0.0.1', '::1'):
                    raise RuntimeError('listener del Job fuera de loopback')
                actual, = struct.unpack_from('!H', buf, ix+localport)
                found |= (ip == '127.0.0.1' and actual == port)
        return found

    def finish(self):
        try:
            must(terminate(self.handle, 0))
            for worker in self.workers:
                worker.wait(5)
            deadline = time.monotonic() + 5
            while self.pids():
                if time.monotonic() >= deadline:
                    raise RuntimeError('árbol del Job aún activo')
                time.sleep(.05)
        finally:
            must(close(self.handle))
