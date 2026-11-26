"""Sondas HTTP solo loopback, sin redirecciones ni proxies del host."""
import http.client
from http.cookies import SimpleCookie
import json
import re
import time
from urllib.parse import urlsplit
from html.parser import HTMLParser
from urllib.parse import urljoin

from prepare import check
from provider import FIX, MODEL


class Client:
    def __init__(self, port, auth=None):
        self.port, self.auth, self.cookie, self.csrf = port, auth, '', ''
        self.origin = f'http://127.0.0.1:{port}'

    def request(self, path, data=None, method=None, timeout=90):
        c = http.client.HTTPConnection('127.0.0.1', self.port, timeout=timeout)
        headers = {'Origin': self.origin, 'Content-Type': 'application/json'}
        if self.auth:
            headers['Authorization'] = self.auth
        if self.cookie:
            headers['Cookie'] = self.cookie
        if self.csrf:
            headers['x-omniroute-csrf'] = self.csrf
        try:
            c.request(method or ('GET' if data is None else 'POST'), path,
                      None if data is None else json.dumps(data).encode(), headers)
            r = c.getresponse()
            for name, value in r.getheaders():
                if name.lower() == 'set-cookie':
                    cookies = SimpleCookie(self.cookie)
                    cookies.load(value)
                    self.cookie = '; '.join(f'{k}={v.value}' for k, v in cookies.items())
            if not 200 <= r.status < 300:
                # Conservar sólo códigos de error conocidos, nunca el cuerpo/headers.
                text = r.read(16384).decode(errors='replace')
                codes = re.findall(r'W1_UNSUPPORTED_VARIANT|RATE_LIMIT_(?:QUEUE|EXECUTION)_TIMEOUT|STREAM_READINESS_TIMEOUT|synthetic_provider_timeout', text)
                raise RuntimeError(f'{path.split("?")[0]}: HTTP {r.status} {",".join(sorted(set(codes)))}')
            return c, r
        except BaseException:
            c.close()
            raise

    def json(self, *args, **kwargs):
        c, r = self.request(*args, **kwargs)
        try:
            return json.loads(r.read())
        finally:
            r.close()
            c.close()


def stream(client, model, mode, provider, effort='low'):
    provider.mode = mode
    offset = len(provider.events)
    start = time.monotonic()
    body = {'model': model, 'stream': True, 'reasoning_effort': effort,
            'messages': [{'role': 'user', 'content': 'prueba sintética'}]}
    c, r = client.request('/v1/chat/completions', body)
    headers_at = time.monotonic() - start
    first, done, content = None, False, ''
    try:
        check('text/event-stream' in r.getheader('Content-Type', ''), 'respuesta sin SSE')
        while line := r.readline(65536):
            check(time.monotonic() - start < 100, 'plazo total del ensayo')
            if not line.startswith(b'data:'):
                continue
            raw = line[5:].strip()
            if raw == b'[DONE]':
                done = True
                break
            record = json.loads(raw)
            check(not record.get('error'), 'evento de error en SSE')
            for choice in record.get('choices', []):
                text = choice.get('delta', {}).get('content')
                if text:
                    content += text
                    if first is None:
                        first = time.monotonic() - start
            if first is not None and mode == 'cancel':
                break
    finally:
        r.close()
        c.close()
    elapsed = time.monotonic() - start
    if mode == 'cancel':
        deadline = time.monotonic() + 6
        while not any(x[0] == 'cancelled' for x in provider.since(offset)) and time.monotonic() < deadline:
            time.sleep(.1)
        check(first is not None and any(x[0] == 'cancelled' for x in provider.since(offset)), 'cancelación no llegó al proveedor')
    else:
        profile = FIX['scenarios'][mode]
        check(done and content == 'LAB'*profile['chunks'], 'SSE truncado o alterado')
        check(first is not None and first >= (profile['headers_ms']+profile['first_ms'])/1000-.1, 'TTFT incompatible con fixture')
        check(elapsed-first >= (profile['chunks']-1)*profile['interval_ms']/1000-.25, 'stream amortiguado o incompleto')
    check([x[1] for x in provider.since(offset) if x[0] == 'received'] == [effort], 'ruta, reintento o esfuerzo inesperado')
    return dict(cabeceras_s=round(headers_at, 3), primer_contenido_s=round(first, 3), total_s=round(elapsed, 3), done=done)


class Assets(HTMLParser):
    def __init__(self):
        super().__init__()
        self.refs = []

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == 'script' and a.get('src'):
            self.refs.append(a['src'])
        if tag == 'link' and a.get('rel') in ('stylesheet', 'modulepreload') and a.get('href'):
            self.refs.append(a['href'])


def offline_ui(client):
    c, r = client.request('/')
    try:
        check('text/html' in r.getheader('Content-Type', ''), 'falta HTML oficial')
        parser = Assets()
        parser.feed(r.read().decode())
    finally:
        r.close()
        c.close()
    check(parser.refs, 'HTML sin assets verificables')
    js = 0
    for ref in dict.fromkeys(parser.refs):
        url = urlsplit(urljoin(client.origin, ref))
        check(f'{url.scheme}://{url.netloc}' == client.origin, 'asset de origen externo')
        c, r = client.request(url.path + ('?' + url.query if url.query else ''))
        try:
            kind = r.getheader('Content-Type', '')
            check('text/html' not in kind and r.read(100), 'asset ausente')
            js += int('javascript' in kind)
        finally:
            r.close()
            c.close()
    check(js, 'ningún recurso JavaScript servido')
    return {'recursos_iniciales': len(set(parser.refs)), 'javascript': js}
