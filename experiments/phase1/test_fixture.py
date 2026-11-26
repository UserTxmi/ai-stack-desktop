import threading
import unittest

from probes import Client, stream
from provider import FIX, MODEL, Provider


class Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = Provider('test-only')
        cls.thread = threading.Thread(target=cls.fixture.serve_forever, daemon=True)
        cls.thread.start()
        cls.client = Client(cls.fixture.server_port, 'Bearer test-only')

    @classmethod
    def tearDownClass(cls):
        cls.fixture.stop.set()
        cls.fixture.shutdown()
        cls.fixture.server_close()
        cls.thread.join()

    def setUp(self):
        self.fixture.mode = 'normal'

    def test_access_and_catalog(self):
        with self.assertRaisesRegex(RuntimeError, '401'):
            Client(self.fixture.server_port).json('/v1/models')
        row, = self.client.json('/v1/models')['data']
        self.assertEqual((row['id'], row['capabilities']['effort_tiers']), (MODEL, ['low','high']))

    def test_efforts(self):
        for effort in FIX['efforts']:
            self.assertTrue(stream(self.client, MODEL, 'normal', self.fixture, effort)['done'])
        with self.assertRaisesRegex(RuntimeError, '400'):
            self.client.json('/v1/chat/completions', {'model': MODEL, 'reasoning_effort':'unsupported'})

    def test_delay_headers_and_stream(self):
        times = stream(self.client, MODEL, 'late', self.fixture)
        self.assertGreaterEqual(times['cabeceras_s'], 18)
        print('late:', times)

    def test_delay_content_only(self):
        times = stream(self.client, MODEL, 'body', self.fixture)
        self.assertLess(times['cabeceras_s'], 2)
        self.assertGreaterEqual(times['primer_contenido_s'], 18)
        print('body:', times)

    def test_cancel(self):
        self.assertFalse(stream(self.client, MODEL, 'cancel', self.fixture)['done'])

    def test_provider_error(self):
        self.fixture.mode = 'provider504'
        with self.assertRaisesRegex(RuntimeError, 'HTTP 504 synthetic_provider_timeout'):
            self.client.json('/v1/chat/completions', {'model': MODEL})

    def test_tool_consent_fixture(self):
        self.fixture.mode = 'tools'
        c, r = self.client.request('/v1/chat/completions', {'model':MODEL, 'stream':True,
                  'tools':[{'type':'function','function':{'name':'bash'}}]})
        try:
            wire = r.read()
            self.assertIn(b'"tool_calls"', wire)
            self.assertIn(b'echo LAB', wire)
            self.assertTrue(wire.endswith(b'data: [DONE]\n\n'))
        finally:
            r.close()
            c.close()


if __name__ == '__main__':
    unittest.main(verbosity=2)
