"""Variante del receptor W1 que solicita una delegación sintética."""
import io
import json
from provider import Handler, MODEL, Provider


class GentleProvider(Provider):
    def __init__(self, key):
        super().__init__(key)
        self.RequestHandlerClass = GentleHandler
        self.delegate = False
        self.markers = []


class GentleHandler(Handler):
    def do_POST(self):
        # Reutilizar validación/SSE/latencias de W1 en todos los turnos de texto.
        size = int(self.headers.get('Content-Length', '0'))
        if not 0 < size <= 2**20 or not self.authorized():
            return
        raw = self.rfile.read(size)
        data = json.loads(raw)
        users = json.dumps([m.get('content') for m in data.get('messages', []) if m.get('role') == 'user'])
        tags = [t for t in ('GS-LOW', 'GS-HIGH', 'GS-CHILD', 'GS-DELEGATE', 'GS-CANCEL') if t in users]
        self.server.markers.append((tags, data.get('reasoning_effort')))
        if self.server.delegate and 'GS-DELEGATE' in tags and data['model'] == MODEL:
            self.server.delegate = False
            names = {t['function']['name'] for t in data.get('tools', [])}
            if 'subagent_run' not in names:
                return self.reply(400, {'error': {'code': 'no_subagent_tool'}})
            args = {'agent': 'gentle-ai-explore', 'mode': 'task', 'task': 'GS-CHILD. Reply LAB only.'}
            call = {'index': 0, 'id': 'call_gs', 'type': 'function', 'function': {
                'name': 'subagent_run', 'arguments': json.dumps(args)}}
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            for delta, finish in [({'role': 'assistant', 'tool_calls': [call]}, None), ({}, 'tool_calls')]:
                body = {'id': 'chatcmpl-lab', 'object': 'chat.completion.chunk', 'model': MODEL,
                        'created': 1, 'choices': [{'index': 0, 'delta': delta, 'finish_reason': finish}]}
                self.wfile.write(b'data: ' + json.dumps(body).encode() + b'\n\n')
            self.wfile.write(b'data: [DONE]\n\n')
            return
        original = self.rfile
        try:
            self.rfile = io.BytesIO(raw)
            super().do_POST()
        finally:
            self.rfile = original
