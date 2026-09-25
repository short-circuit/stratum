// Minimal CDP client over the adb-forwarded websocket
const url = process.argv[2];
const ws = new WebSocket(url);
let id = 0;
const pending = new Map();
function send(method, params={}) {
  return new Promise((resolve) => {
    const mid = ++id;
    pending.set(mid, resolve);
    ws.send(JSON.stringify({id: mid, method, params}));
  });
}
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg.result); pending.delete(msg.id); }
};
ws.onopen = async () => {
  await send('Runtime.enable');
  const expr = `
    (() => {
      const cs = getComputedStyle(document.documentElement);
      const topBar = [...document.querySelectorAll('div')].find(d => {
        const s = getComputedStyle(d);
        return s.position === 'absolute' && s.height === '48px' && d.innerText && /Journal|Pages|Search|Stratum|Kanban/.test(d.innerText);
      });
      const navBar = [...document.querySelectorAll('div')].find(d => {
        const s = getComputedStyle(d);
        return s.position === 'fixed' && s.bottom === '0px';
      });
      const rect = topBar ? topBar.getBoundingClientRect() : null;
      const navRect = navBar ? navBar.getBoundingClientRect() : null;
      return JSON.stringify({
        safeAreaTop: cs.getPropertyValue('--safe-area-top'),
        safeAreaBottom: cs.getPropertyValue('--safe-area-bottom'),
        insetTop: cs.getPropertyValue('--safe-area-inset-top'),
        insetBottom: cs.getPropertyValue('--safe-area-inset-bottom'),
        topBarRect: rect ? {top: rect.top, height: rect.height} : null,
        navBarBottom: navRect ? {bottom: navRect.bottom, height: navRect.height, top: navRect.top} : null,
        devicePixelRatio: window.devicePixelRatio,
        innerHeight: window.innerHeight
      }, null, 2);
    })()
  `;
  const r = await send('Runtime.evaluate', {expression: expr, returnByValue: true});
  console.log('=== LIVE DOM STATE ===');
  console.log(r.result.value);
  ws.close();
  process.exit(0);
};
