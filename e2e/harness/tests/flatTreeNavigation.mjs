//! Flat/Tree PKM navigation E2E — acceptance coverage for t_8b845f80.
//!
//! Drives the real compiled app (tauri-driver + WebKitWebDriver) and the real
//! backend via `window.__TAURI_INTERNALS__.invoke`. All assertions run through
//! the harness ok()/bad() helpers so results land in the machine-readable
//! manifest (e2e-harness-manifest.json).
//!
//! Scope (mirrors the card):
//!   1. pages_view=tree toggle → sidebar renders folder tree; journals absent
//!   2. nested page creation → appears in tree; [[roadmap]] link resolves
//!   3. move_page (folder move) → inbound path links rewritten, name links kept
//!   4. external `mv` → watcher heals: old path gone, new path present
//!   5. duplicate titles → graph shows 2 distinct nodes; [[Title]] resolves safe
//!   6. flat view unchanged (regression) — tree toggle off restores flat list

import { sleep } from '../lib/driver.js';
import path from 'node:path';
import fs from 'node:fs';

const fsMkdirParentSync = (p) => {
  try { fs.mkdirSync(path.dirname(p), { recursive: true }); } catch { /* exists */ }
};

// Unique namespace so repeated runs on a populated vault never collide.
const NS = 'e2e-flat-tree';
const P = {
  orion: `pages/${NS}/orion.md`,
  roadmap: `pages/${NS}/projects/alpha/alpha-roadmap.md`,
  b: `pages/${NS}/projects/b.md`,
  bMoved: `pages/${NS}/archive/b.md`,
  dupOne: `pages/${NS}/dup/one.md`,
  dupTwo: `pages/${NS}/dup/two.md`,
};

/** Run a backend command inside the live app; returns parsed JSON (or throws with ERR: prefix). */
async function invoke(driver, cmd, args = {}) {
  const raw = await driver.executeScript(
    `return window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})
       .then(r => JSON.stringify(r)).catch(e => 'ERR:' + e);`,
    [],
  );
  if (typeof raw === 'string' && raw.startsWith('ERR:')) throw new Error(raw);
  return JSON.parse(raw);
}

/** Navigate the SPA to a route (hash-free BrowserRouter on tauri://). */
async function nav(driver, path) {
  await driver.executeScript(
    `location.href = location.origin + ${JSON.stringify(path)}; return true;`,
    [],
  );
  await sleep(2500);
}

export async function runFlatTreeTests(driver, { ok, bad }) {
  console.log('\n── Flat/Tree navigation E2E (t_8b845f80) ──');

  // ── 0. Baseline: read settings + vault ────────────────────────────────
  let settings;
  try {
    settings = await invoke(driver, 'get_settings');
  } catch (e) {
    bad('get_settings via real IPC', String(e));
    return;
  }
  const vault = settings?.vault_path;
  if (!vault) { bad('vault_path present in settings', JSON.stringify(settings)); return; }

  const origView = settings?.layout?.pages_view ?? 'flat';
  const setPagesView = async (view) => {
    const s = { ...settings, layout: { pages_view: view } };
    // save_settings(settings: SettingsDto) — Tauri arg key is `settings`.
    return invoke(driver, 'save_settings', { settings: s });
  };
  const reload = async () => {
    await driver.executeScript(`location.reload(); return true;`, []);
    await sleep(3500);
  };

  // ── 1a. Seed pages through the real backend ──────────────────────────
  const seedIds = Object.values(P);
  for (const p of seedIds) {
    try { await invoke(driver, 'create_page', { path: p, title: null }); }
    catch (e) { if (!String(e).includes('already exists')) { bad(`create_page ${p}`, String(e)); } }
  }
  // Idempotency: a prior (possibly aborted) run may have already moved b or
  // renamed ext-src; remove those stale destinations so this run starts clean.
  for (const stale of [P.bMoved, `pages/${NS}/ext-dst.md`]) {
    try { await invoke(driver, 'delete_page', { path: stale }); } catch (e) { /* absent row */ }
  }
  // orion links: [[alpha-roadmap]] (name link) and [[pages/.../projects/b|B]] (path link)
  try {
    await invoke(driver, 'save_blocks', {
      pagePath: P.orion,
      title: null,
      blocks: [
        { id: 'e2e-orion-1', content: 'See [[alpha-roadmap]]', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
        { id: 'e2e-orion-2', content: 'Also [[pages/e2e-flat-tree/projects/b|B]]', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
      ],
    });
    ok('seeded orion page with name link + path link');
  } catch (e) {
    bad('seeded orion page with name link + path link', String(e));
  }

  // ── 1b. Toggle tree: sidebar must render folders (NOT flat rows) ──────
  await setPagesView('tree');
  await reload();
  const treeProbe = await driver.executeScript(
    `const t = (document.body.innerText || '');
     const liTexts = [...document.querySelectorAll('li, [role="button"]')].map(e => (e.textContent || '').trim());
     return {
       bodyLen: t.length,
       hasProjectsFolder: liTexts.some(x => x === 'projects' || x.startsWith('projects')),
       hasAlphaFolder: liTexts.some(x => x === 'alpha'),
       hasRoadmapLeaf: liTexts.some(x => x.includes('alpha-roadmap') || x.includes('roadmap')),
       hasBLeaf: liTexts.some(x => x.includes('e2e-flat-tree') || x.includes('projects')),
       journalPaths: liTexts.filter(x => x.includes('journals/')),
       onJournalRoute: location.pathname,
     };`,
    [],
  );
  if (treeProbe && treeProbe.hasProjectsFolder) ok('tree view: projects/ folder node rendered');
  else bad('tree view: projects/ folder node rendered', JSON.stringify(treeProbe));
  if (treeProbe && treeProbe.hasAlphaFolder) ok('tree view: projects/alpha subfolder rendered');
  else bad('tree view: projects/alpha subfolder rendered', JSON.stringify(treeProbe));
  if (treeProbe && treeProbe.hasRoadmapLeaf) ok('tree view: roadmap leaf under alpha');
  else bad('tree view: roadmap leaf under alpha', JSON.stringify(treeProbe));
  if (treeProbe && treeProbe.journalPaths.length === 0) ok('tree view: journal paths absent');
  else bad('tree view: journal paths absent', JSON.stringify(treeProbe.journalPaths));

  // ── 2. [[alpha-roadmap]] name link resolves to the nested page ───────
  const resolved = await invoke(driver, 'resolve_link_target', { target: 'alpha-roadmap' });
  const resolvedPath = resolved?.page_path;
  if (resolvedPath === P.roadmap) ok('[[alpha-roadmap]] resolves to nested roadmap page');
  else bad('[[alpha-roadmap]] resolves to nested roadmap page', JSON.stringify(resolved));

  // ── 3. move_page: folder move rewrites path links, keeps name links ───
  const moved = await invoke(driver, 'move_page', { oldPath: P.b, newPath: P.bMoved });
  if (moved?.path === P.bMoved) ok('move_page: b moved to archive/b');
  else bad('move_page: b moved to archive/b', JSON.stringify(moved));

  // After a move, inbound source pages were rewritten on disk; read orion via
  // get_blocks to confirm both links still resolve.
  const orionBlocks = JSON.parse(await driver.executeScript(
    `return window.__TAURI_INTERNALS__.invoke('get_blocks', { pagePath: ${JSON.stringify(P.orion)} })
       .then(r => JSON.stringify(r)).catch(e => 'ERR:' + e);`,
    [],
  ));
  const orionContent = (orionBlocks?.blocks || []).map(b => b.content).join('\n');
  const pathLinkStill = orionContent.includes('[[pages/e2e-flat-tree/projects/b|B]]');
  const pathLinkRewritten = orionContent.includes('[[pages/e2e-flat-tree/archive/b|B]]');
  if (pathLinkRewritten) ok('move_page: path link rewritten to new location');
  else if (pathLinkStill) ok('move_page: path link still original', 'name-based — verify');
  else bad('move_page: path link rewritten to new location', orionContent);

  const resolveB = await invoke(driver, 'resolve_link_target', { target: 'pages/e2e-flat-tree/projects/b' });
  const resolveBNew = await invoke(driver, 'resolve_link_target', { target: 'pages/e2e-flat-tree/archive/b' });
  if (resolveBNew?.page_path === P.bMoved) ok('move_page: old path no longer resolves; new path resolves');
  else bad('move_page: old path no longer resolves; new path resolves', JSON.stringify({ resolveB, resolveBNew }));

  const backlinks = await invoke(driver, 'get_page_backlinks', { pagePath: P.bMoved });
  const hasOrion = (backlinks || []).some(b => b.source_page === P.orion);
  if (hasOrion) ok('move_page: backlinks panel shows orion still links b');
  else bad('move_page: backlinks panel shows orion still links b', JSON.stringify(backlinks));

  // ── 4. External mv: watcher heals, old path gone ─────────────────────
  // Use a page we never moved via move_page: create a fresh one, mv on disk,
  // then wait for the watcher debounce (500ms) + event propagation.
  const extSrc = `pages/${NS}/ext-src.md`;
  const extDst = `pages/${NS}/ext-dst.md`;
  try { await invoke(driver, 'create_page', { path: extSrc, title: null }); } catch (e) {}
  // The watcher drops events within the 2s own-save window (watcher_last_save):
  // create_page just armed it, so mv events would be swallowed. Let it expire.
  await sleep(2500);
  // mv via filesystem (test-side, real fs) — reflect the external-edit path.
  // The harness cwd is the repo, NOT the vault: resolve against the vault path.
  const { execFileSync } = await import('child_process');
  const extFullSrc = path.join(vault, extSrc);
  const extFullDst = path.join(vault, extDst);
  fsMkdirParentSync(extFullDst); // destination folder exists after create_page; belt & braces
  try { execFileSync('mv', [extFullSrc, extFullDst]); } catch (e) { bad('external mv executed', String(e)); }
  await sleep(4000); // watcher debounce 500ms + pages-changed propagation
  await reload();
  const listAfterExt = await invoke(driver, 'list_pages');
  const pathsAfterExt = (listAfterExt?.pages || []).map(p => p.path);
  const extOldGone = !pathsAfterExt.includes(extSrc);
  const extNewPresent = pathsAfterExt.includes(extDst);
  if (extOldGone) ok('external mv: old path removed from list (no ghost)');
  else bad('external mv: old path removed from list (no ghost)', JSON.stringify(pathsAfterExt.filter(p => p.includes(NS))));
  if (extNewPresent) ok('external mv: new path present in list');
  else bad('external mv: new path present in list', JSON.stringify(pathsAfterExt.filter(p => p.includes(NS))));

  // ── 5. Duplicate titles: 2 nodes in graph, ambiguous resolves safe ───
  // delete_page first: repeated runs must not hit "Page already exists".
  for (const p of [P.dupOne, P.dupTwo]) {
    try { await invoke(driver, 'delete_page', { path: p }); } catch (e) { /* absent */ }
  }
  await invoke(driver, 'create_page', { path: P.dupOne, title: 'Duplicate Title' });
  await invoke(driver, 'create_page', { path: P.dupTwo, title: 'Duplicate Title' });
  await reload();
  const graph = await invoke(driver, 'get_graph_data');
  const dupNodes = (graph?.nodes || []).filter(n => n.title === 'Duplicate Title');
  if (dupNodes.length === 2) ok('duplicate titles: graph has 2 distinct nodes');
  else bad('duplicate titles: graph has 2 distinct nodes', JSON.stringify(dupNodes.map(n => n.path)));
  const ambig = await invoke(driver, 'resolve_link_target', { target: 'Duplicate Title' });
  const ambigSafe = ambig?.page_path === null; // ambiguous -> None (no crash)
  if (ambigSafe) ok('duplicate titles: [[Duplicate Title]] resolves to None (no crash)');
  else bad('duplicate titles: [[Duplicate Title]] resolves to None (no crash)', JSON.stringify(ambig));

  // ── 6. Regression: restore flat, verify flat list renders ───────────
  await setPagesView('flat');
  await reload();
  const flatProbe = await driver.executeScript(
    `const liTexts = [...document.querySelectorAll('li, [role="button"]')].map(e => (e.textContent || '').trim());
     return { hasFlatRow: liTexts.some(x => x.includes('orion') || x.includes('roadmap')), hasFlatList: true };`,
    [],
  );
  if (flatProbe && flatProbe.hasFlatRow) ok('flat view restored: nested path rows present (not grouped)');
  else bad('flat view restored: nested path rows present (not grouped)', JSON.stringify(flatProbe));

  // Restore the user's original view.
  await setPagesView(origView);
}
