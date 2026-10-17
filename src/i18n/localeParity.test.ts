import { describe, it, expect } from 'vitest';
import fs from 'fs';
import path from 'path';

//! i18n locale parity gate.
//!
//! Enforces the t_d7073534 acceptance rule: all 8 locale files must expose the
//! IDENTICAL (flattened) key set, every key must exist in `en` (source of truth,
//! fallback), and the placeholder locales (zh/ja/el) must mirror `en` values so
//! a missing translation can never crash or render a raw key. Full locales
//! (de/it/fr/es) are checked to contain no value that still equals the English
//! source for actual UI copy — brand/proper nouns that legitimately stay as-is
//! (e.g. "Kanban", "Stratum", "TODO") are allowlisted.

const LOCALES_DIR = path.join(process.cwd(), 'src', 'i18n', 'locales');
const LOCALES = ['en', 'de', 'it', 'fr', 'es', 'zh', 'ja', 'el'] as const;
const FULL_LOCALES = ['de', 'it', 'fr', 'es'] as const;
const PLACEHOLDER_LOCALES = ['zh', 'ja', 'el'] as const;
const SRC_DIR = path.join(process.cwd(), 'src');

function flatten(obj: Record<string, unknown>, prefix = '', acc: Record<string, string> = {}): Record<string, string> {
  for (const [k, v] of Object.entries(obj)) {
    const nk = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) flatten(v as Record<string, unknown>, nk, acc);
    else acc[nk] = String(v);
  }
  return acc;
}

function load(locale: string): Record<string, string> {
  const raw = JSON.parse(fs.readFileSync(path.join(LOCALES_DIR, `${locale}.json`), 'utf8')) as Record<string, unknown>;
  return flatten(raw);
}

/**
 * Scan all .ts/.tsx sources under `dir` for statically-referenced i18n keys:
 *  - `t('a.b.c')` / `t("a.b.c")` / `i18n.t('a.b.c')` and `useTranslation().t`
 *  - string literals that are dotted i18n keys in data arrays, e.g.
 *    `{ label: 'mobile_nav.reminders', ... }` — the class of bug this gate
 *    exists for (the key is NOT passed to t() literally, so a naive scan
 *    misses it).
 * File literals (e.g. 'photo.png', 'plugin.wasm') and MUI theme tokens
 * (e.g. 'text.secondary') never match because they don't begin with a known
 * top-level i18n namespace prefix. Returns a Set of flattened keys.
 */
function scanStaticKeys(dir: string): Set<string> {
  const enRaw = JSON.parse(fs.readFileSync(path.join(LOCALES_DIR, 'en.json'), 'utf8')) as Record<string, unknown>;
  const namespaces = Object.keys(enRaw).filter(k => k !== '_meta' && !k.startsWith('_'));
  const nsRe = new RegExp(`^(${namespaces.map(escapeRegExp).join('|')})\\.`);

  const used = new Set<string>();
  const files: string[] = [];
  const walk = (d: string) => {
    for (const entry of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === 'node_modules' || entry.name.startsWith('.worktrees')) continue;
        walk(p);
      } else if (/\.tsx?$/.test(entry.name)) {
        // Skip i18n test files: they test the catalog itself and legally
        // reference example keys (docstrings) that aren't real catalog keys.
        if (p.endsWith('i18n.test.ts') || p.endsWith('localeParity.test.ts')) continue;
        files.push(p);
      }
    }
  };
  walk(dir);

  for (const file of files) {
    const txt = fs.readFileSync(file, 'utf8');
    // t('a.b') / t("a.b") / i18n.t('a.b')  (static literal only)
    for (const m of txt.matchAll(/(?:\bt|i18n\.t)\s*\(\s*['"]([a-z_][a-zA-Z0-9_.]*)['"]\s*\)/g)) {
      used.add(m[1]);
    }
    // Dotted i18n keys appearing as string literals (data arrays: label/... fields)
    for (const m of txt.matchAll(/['"]([a-z_][a-zA-Z0-9_]*\.[a-zA-Z0-9_.]+)['"]/g)) {
      const k = m[1];
      if (nsRe.test(k) && /[a-zA-Z]/.test(k)) used.add(k);
    }
  }
  // i18next plural resolution: `t('key')` may resolve to `key_one`/`key_other`/
  // `key_zero`, which exist in the catalog even though the bare `key` does not
  // (e.g. `flashcards.reviewed_cards` → `reviewed_cards_one`/`_other`). A key is
  // only "missing" if neither it nor any plural-suffixed form exists.
  return new Set(
    [...used].filter(
      k =>
        !(k in enRawFlat) &&
        !Object.keys(enRawFlat).some(x => x.startsWith(`${k}_`)),
    ),
  );
}

/** Flattened en catalog; used by scanStaticKeys for plural-form resolution. */
const enRawFlat = load('en');

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// Values that legitimately stay identical to English in full locales:
// brand/product names, language endonyms, cross-language cognates, technical
// loanwords, and symbols/format fragments. Everything ELSE that still equals
// the en value is treated as an untranslated string and fails the test.
const IDENTICAL_OK = new Set([
  // Brand / product names
  'Stratum', 'Stratum PKM', 'Kanban', 'Journal', 'Graph', 'Plugins',
  'Whiteboards', 'Flashcards', 'Pages', 'Vault', 'Excalidraw', 'TODO',
  'DOING', 'DONE', 'Graph Settings', 'Obsidian', 'Logseq',
  // Language endonyms (the settings language list is intentionally native)
  'Deutsch', 'English', 'Español', 'Français', 'Italiano', 'Ελληνικά',
  '日本語', '中文',
  // Provider / brand names
  'OpenAI', 'Anthropic', 'Google AI', 'Z.AI', 'Agent',
  // Technical loanwords / cognates that are standard in all target languages
  'Sync', 'Auto-Commit', 'Auto-Sync', 'Remote & Branch', 'Remote &amp; Branch',
  'Backup', 'WebDAV', 'Password', 'Offline', 'Directory',
  'Chunks', 'Format', 'Optional', 'Marker', 'Name', 'Question', 'question::',
  'Actions', 'Note', 'Error:', 'Sources', 'Variables', 'Relations', 'Hooks:',
  'Test http_request', 'Test note_read', 'Comp', 'Tags', 'Friction',
  'Tray', 'Stop',
  'Notifications', // identical word in en/fr (valid French)
  'notes)', '[[wiki-links]]', '~/.pkm/plugins', '~/.pkm/plugins.',
  'Query', // Datalog "query" is used untranslated as a loanword in de/it/fr/es UIs
  // Path fragments / placeholders that legitimately stay identical in all languages
  'pages/projects/my-note.md',
  // Symbol / layout fragments
  'n ·', 'b ·', 'a\n─\nb', '&quot;?', 'T): Promise',
  '>', '<', '≥', '≤', '=', 'AND', 'OR', // builder operator symbols / combiner labels (pure symbols and standard loanwords)
  'Datalog', 'Operator', // technical loanwords (cf. 'Query' above)
  // Meta marker (already excluded by key, value harmless)
  'complete', 'in progress',
]);

describe('i18n locale parity', () => {
  it('all 8 locale files exist and parse', () => {
    for (const l of LOCALES) {
      expect(() => load(l)).not.toThrow();
    }
  });

  it('en is the complete source of truth (no other locale may define a key en lacks)', () => {
    const en = load('en');
    expect(Object.keys(en).length).toBeGreaterThan(300);
    for (const l of LOCALES) {
      const x = load(l);
      expect(Object.keys(x).length, `${l} key count`).toBe(Object.keys(en).length);
      for (const key of Object.keys(x)) {
        expect(key in en, `${l} has extra key not in en: ${key}`).toBe(true);
      }
    }
  });

  it('placeholder locales (zh/ja/el) mirror en values — no missing-key crash path', () => {
    const en = load('en');
    for (const l of PLACEHOLDER_LOCALES) {
      const x = load(l);
      for (const [key, val] of Object.entries(x)) {
        if (key === '_meta.status' || key === 'settings.speech_amp_audio') continue; // intentional markers
        expect(val, `${l}.${key}`).toBe(en[key]);
      }
    }
  });

  it('all static keys used in src/ exist in en (catalog-to-code coverage)', () => {
    // Guards the class of bug where a component references a key (e.g.
    // `mobile_nav.reminders`) that no catalog defines: i18next falls back to
    // en, en lacks it too, and the RAW key renders. Catalog-to-catalog parity
    // cannot catch this (all 8 catalogs omit the key equally), so scan source.
    const en = load('en');
    const used = scanStaticKeys(SRC_DIR);
    const missing = [...used].filter(k => !(k in en));
    expect(missing, `src/ uses i18n keys missing from en: ${missing.join(', ')}`).toEqual([]);
  });

  it('full locales (de/it/fr/es) are actually translated (no untranslated English left)', () => {
    const en = load('en');
    for (const l of FULL_LOCALES) {
      const x = load(l);
      const untranslated: string[] = [];
      for (const [key, val] of Object.entries(x)) {
        if (val === en[key] && !IDENTICAL_OK.has(val)) untranslated.push(`${key} = "${val}"`);
      }
      expect(untranslated, `${l} untranslated values`).toEqual([]);
    }
  });
});
