// Sólo en el runner: WebView2 real del EXE, sin instrumentar el bundle oficial.
import { chromium } from 'playwright-core';
import { spawn } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { join, resolve } from 'node:path';
import assert from 'node:assert/strict';

const [root, project, report] = process.argv.slice(2).map(p => resolve(p));
const delay = ms => new Promise(r => setTimeout(r, ms));
let stage = 'arranque', browser;
const probe = createServer();
await new Promise((ok, fail) => probe.once('error', fail).listen(0, '127.0.0.1', ok));
const port = probe.address().port;
await new Promise(ok => probe.close(ok));
const exe = spawn(join(root, 'bin/ai-stack-desktop.exe'), [], {env: {...process.env,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`}});
async function until(fn, ms = 90000) {
  const end = Date.now() + ms;
  while (Date.now() < end && exe.exitCode === null) {
    const result = await fn();
    if (result) return result;
    await delay(250);
  }
  throw Error('plazo');
}
try {
  await until(() => fetch(`http://127.0.0.1:${port}/json/version`).then(r => r.ok, () => false));
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  stage = 'shell';
  const pages = () => browser.contexts().flatMap(c => c.pages());
  const shell = await until(() => pages().find(p => p.url().includes('tauri.localhost')));
  stage = 'servicios';
  await shell.locator('[data-view="stack"]').click();
  await shell.locator('#start').click({timeout: 120000});
  await shell.waitForFunction(() => !document.querySelector('#test').disabled, null, {timeout: 180000});

  stage = 'abrir proyecto';
  // Sólo estados HTTP de rutas fijas. La distinción global/proyecto evita otro falso verde.
  const watch = page => page.on('response', r => {
    const u = new URL(r.url());
    if (['/global/health','/provider','/path'].includes(u.pathname))
      console.log(`WebView2 ${u.pathname} ${u.searchParams.has('directory') ? 'proyecto' : 'global'} ${r.status()}`);
  });
  for (const c of browser.contexts()) c.on('page', watch);
  // Sustituye sólo el diálogo COM por su resultado; usa el mismo comando abrir,
  // ticket de un uso y WebView hijo privado de la aplicación distribuida.
  await shell.evaluate(carpeta => window.__TAURI__.core.invoke('abrir', {carpeta}), project);
  const ui = await until(() => pages().find(p => /^http:\/\/127\.0\.0\.1:\d+\//.test(p.url())));
  await ui.waitForURL(/\/session(?:[/?#]|$)/);
  assert.equal(Buffer.from(new URL(ui.url()).pathname.split('/')[1], 'base64url').toString(), project);
  const model = ui.locator('button[data-action="prompt-model"]:not([data-control-type="dialog"])');
  stage = 'modelo conectado';
  await model.click({timeout: 90000});
  const key = 'opencode-omniroute:labcpa/lab-reasoner';
  await ui.locator(`[data-key="${key}"], [data-option-key="${key}"]`).click();
  stage = 'esfuerzos';
  const variant = ui.locator('[data-action="prompt-model-variant"], button[aria-label="Choose model variant"]');
  for (const level of ['low', 'high']) {
    await variant.click();
    await ui.locator('[role="option"], [role="menuitemradio"]').filter({hasText: new RegExp(`^${level}$`, 'i')}).click();
    assert.equal((await variant.innerText()).trim().toLowerCase(), level);
  }
  await writeFile(report, 'WebView2: proyecto Unicode, modelo conectado seleccionado y esfuerzos low/high seleccionados en la UI oficial.\n');
} catch {
  console.error(`WebView2: fallo en ${stage}`); // No publicar URL de bootstrap, cookies ni perfiles.
  process.exitCode = 1;
} finally {
  await browser?.close().catch(() => {});
  exe.kill();
}
