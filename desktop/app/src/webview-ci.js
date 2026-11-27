// Sólo --ci-webview. Observar el DOM oficial y activar controles renderizados;
// no modificar el estado de OpenCode, las peticiones ni los datos del catálogo.
(async () => {
  if (location.hostname !== '127.0.0.1' || !location.pathname.endsWith('/session')) return;
  const visible = node => node?.getClientRects().length && getComputedStyle(node).visibility !== 'hidden';
  const wait = async fn => {
    for (let i = 0; i < 450; ++i) {
      const el = fn(); if (el) return el;
      await new Promise(r => setTimeout(r, 200));
    }
    throw Error('plazo');
  };
  const control = (sel, pred = () => true) => [...document.querySelectorAll(sel)].find(el => visible(el) && pred(el));
  const press = el => {
    el.focus();
    for (const type of ['pointerdown','mousedown','pointerup','mouseup','click']) {
      const opts = {bubbles:true, cancelable:true, button:0, buttons:type.endsWith('down') ? 1 : 0, pointerType:'mouse', pointerId:1, isPrimary:true};
      el.dispatchEvent(type.startsWith('pointer') ? new PointerEvent(type, opts) : new MouseEvent(type, opts));
    }
  };
  const variant = () => control('[data-action="prompt-model-variant"], button[aria-label="Choose model variant"]');
  let stage = 'catálogo';
  try {
    await wait(variant); // El control sólo aparece al terminar la consulta de proyecto.
    stage = 'modelo';
    press(await wait(() => control('[data-action="prompt-model"]')));
    const key = 'opencode-omniroute:labcpa/lab-reasoner';
    press(await wait(() => control(`[data-key="${key}"], [data-option-key="${key}"]`)));
    for (const level of ['low', 'high']) {
      stage = level;
      press(await wait(variant));
      press(await wait(() => control('[role="option"], [role="menuitemradio"]', el => el.textContent.trim().toLowerCase() === level)));
      await wait(() => variant()?.textContent.trim().toLowerCase() === level);
    }
    document.title = 'W1_DOM_OK';
  } catch {
    const statuses = performance.getEntriesByType('resource').filter(e => {
      const u = new URL(e.name); return u.pathname === '/provider' && u.searchParams.has('directory');
    }).map(e => e.responseStatus);
    document.title = `W1_DOM_ERROR:${stage}:${statuses.at(-1) || 0}`;
  }
})();
