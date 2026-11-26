"""Receptor OpenAI sintético. Sin cliente de red, logs de texto ni claves persistidas."""
import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

FIX = json.loads(Path(__file__).with_name('fixtures.json').read_text())
MODEL = FIX['model']


class Provider(ThreadingHTTPServer):
    def __init__(self, key):
        super().__init__(('127.0.0.1', 0), Handler)
        self.key = key
        self.mode = 'normal'
        self.events = []
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.held = threading.Event()
        self.release = threading.Event()
        self.requests = []

    def classify(self, data):
        # Sólo igualdad/presencia de marcadores de la fixture; no persistir mensajes ni ID de sesión.
        roles = {m.get('role'): [] for m in data.get('messages', [])}
        for m in data.get('messages', []):
            text = m.get('content', '')
            roles[m.get('role')].extend([text] if isinstance(text,str) else [p.get('text','') for p in (text or []) if isinstance(p,dict)])
        users = roles.get('user', [])
        for tag in ('held','waiting','low','high','invalid'):
            if any('W1-TEST:'+tag+':END' in t for t in users):
                title = any('Generate a title for this conversation:' in t for t in users) or any('You are a title generator.' in t for t in roles.get('system', []))
                main = any('W1-MAIN:'+tag+':END' in t for t in roles.get('system', [])+roles.get('developer', []))
                # ensureTitle reutiliza firstInfo, que puede contener el sistema del turno.
                return tag, 'title' if title else 'main' if main else 'other'
        return 'other','other'

    def record(self, kind, effort):
        with self.lock:
            self.events.append((kind, effort, time.monotonic()))

    def since(self, offset):
        with self.lock:
            return self.events[offset:]


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def reply(self, status, obj):
        data = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def authorized(self):
        if self.headers.get('Authorization') == 'Bearer ' + self.server.key:
            return True
        self.reply(401, {'error': {'code': 'unauthorized'}})
        return False

    def do_GET(self):
        if not self.authorized():
            return
        if self.path != '/v1/models':
            return self.reply(404, {})
        self.reply(200, {'object': 'list', 'data': [{'id': MODEL, 'object': 'model', 'owned_by': 'lab',
                   'context_length': 32768, 'max_output_tokens': 2048,
                   'capabilities': {'reasoning': True, 'effort_tiers': FIX['efforts']}}]})

    def do_POST(self):
        if not self.authorized():
            return
        if self.path != '/v1/chat/completions':
            return self.reply(404, {})
        try:
            n = int(self.headers.get('Content-Length', '0'))
            if not 0 < n <= 2**20:
                return self.reply(413, {})
            data = json.loads(self.rfile.read(n))
            if not isinstance(data, dict) or data.get('model') != MODEL:
                return self.reply(400, {'error': {'code': 'unknown_model'}})
            effort = data.get('reasoning_effort')
            if effort is not None and effort not in FIX['efforts']:
                return self.reply(400, {'error': {'code': 'unsupported_effort'}})
        except (TypeError, ValueError):
            return self.reply(400, {})
        p = FIX['scenarios'][self.server.mode]
        self.server.record('received', effort)
        tag, role = self.server.classify(data)
        with self.server.lock:
            self.server.requests.append((tag,role,effort))
        if tag == 'held':
            self.server.held.set()
            if not self.server.release.wait(18):
                return self.reply(504, {'error':{'code':'fixture_barrier_timeout'}})
        if 'status' in p:
            return self.reply(p['status'], {'error': {'code': 'synthetic_provider_timeout'}})
        if self.server.stop.wait(p['headers_ms']/1000):
            return
        base = {'id': 'chatcmpl-lab', 'model': MODEL, 'created': 1}
        try:
            if not data.get('stream'):
                return self.reply(200, {**base, 'object': 'chat.completion', 'choices': [{'index': 0,
                    'message': {'role': 'assistant', 'content': 'LAB'}, 'finish_reason': 'stop'}]})
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            if self.server.stop.wait(p['first_ms']/1000):
                return
            if self.server.mode == 'tools' and any(t.get('function', {}).get('name') == 'bash' for t in data.get('tools', [])) and not any(m.get('role') == 'tool' for m in data.get('messages', [])):
                # Sin argumentos del usuario ni modificación de archivos; negar en el ensayo de permisos.
                call = {'index': 0, 'type': 'function', 'id': 'call_lab', 'function': {'name': 'bash',
                        'arguments': json.dumps({'command': 'echo LAB', 'description': 'Mostrar LAB'})}}
                for delta, reason in [({'role':'assistant','tool_calls':[call]}, None), ({}, 'tool_calls')]:
                    rec = {**base, 'object':'chat.completion.chunk', 'choices':[{'index':0, 'delta':delta, 'finish_reason':reason}]}
                    self.wfile.write(b'data: '+json.dumps(rec).encode()+b'\n\n')
                self.wfile.write(b'data: [DONE]\n\n')
                self.wfile.flush()
                self.server.record('tool-offered', effort)
                return
            for i in range(p['chunks'] + 1):
                end = i == p['chunks']
                delta = {} if end else {'content': 'LAB', **({'role': 'assistant'} if not i else {})}
                chunk = {**base, 'object': 'chat.completion.chunk', 'choices': [{'index': 0,
                    'delta': delta, 'finish_reason': 'stop' if end else None}]}
                self.wfile.write(b'data: ' + json.dumps(chunk).encode() + b'\n\n')
                self.wfile.flush()
                if not i:
                    self.server.record('first', effort)
                if not end and self.server.stop.wait(p['interval_ms']/1000):
                    return
            self.wfile.write(b'data: [DONE]\n\n')
            self.wfile.flush()
            self.server.record('done', effort)
        except (ConnectionError, OSError):
            self.server.record('cancelled', effort)
