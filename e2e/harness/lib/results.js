#!/usr/bin/env node
/**
 * Stratum Automated E2E Harness — machine-readable results reporter.
 *
 * Collects structured test-case events as the harness runs and writes two
 * artifacts at the end of a run:
 *
 *   - <repo>/test-results/e2e-harness-manifest.json  — JSON metadata + outcomes
 *   - <repo>/test-results/e2e-harness-junit.xml      — JUnit XML (CI consumers)
 *
 * The JSON manifest is the authoritative machine-readable report (the
 * acceptance criterion for this harness): it records per-test outcome, pass/
 * fail counts, the environment (app binary, driver, display), and timing.
 * The JUnit XML is a convenience for tooling that speaks JUnit.
 *
 * Nothing in this module modifies files outside the repo's (gitignored)
 * test-results/ directory.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(__dirname, '../../..');
const RESULTS_DIR = path.join(REPO_ROOT, 'test-results');
const MANIFEST = path.join(RESULTS_DIR, 'e2e-harness-manifest.json');
const JUNIT = path.join(RESULTS_DIR, 'e2e-harness-junit.xml');

/** Per-test results accumulator. Not exported directly; use the helpers below. */
const _results = [];
let _startedAt = null;

/** Begin a run: record start time and reset any prior accumulation. */
export function beginRun(meta = {}) {
  _startedAt = new Date().toISOString();
  _results.length = 0;
  return { startedAt: _startedAt, ...meta };
}

/** Record a single test-case outcome. */
export function recordTest({ suite, name, status, detail = '', durationMs = 0 }) {
  const st = status === 'pass' ? 'pass' : 'fail';
  _results.push({ suite, name, status: st, detail: String(detail || ''), durationMs });
}

/**
 * Finalize and write the machine-readable artifacts.
 * Returns the manifest object (also written to disk).
 */
export async function writeResults({ status, summary = '', startedAt = _startedAt, fatal = '' }) {
  const finishedAt = new Date().toISOString();
  const passed = _results.filter((r) => r.status === 'pass').length;
  const failed = _results.filter((r) => r.status === 'fail').length;
  const skipped = 0;

  const manifest = {
    schema: 'stratum-e2e-harness/manifest/v1',
    app: 'stratum-tauri',
    suite: summary || 'stratum automated e2e',
    status,
    fatal: fatal || undefined,
    startedAt,
    finishedAt,
    counts: { passed, failed, skipped, total: _results.length },
    tests: _results,
  };

  const junit = renderJunit(manifest);

  try {
    fs.mkdirSync(RESULTS_DIR, { recursive: true });
    fs.writeFileSync(MANIFEST, JSON.stringify(manifest, null, 2));
    fs.writeFileSync(JUNIT, junit);
  } catch (e) {
    // Never fail the run because the report couldn't be written.
    process.stderr.write(`[harness] warning: could not write results: ${e.message}\n`);
  }

  return manifest;
}

/** Render the accumulated results as JUnit XML (escaping entities). */
function renderJunit(m) {
  const esc = (s) => String(s ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
  const suite = m.tests.reduce(
    (acc, t) => {
      acc.tests++;
      if (t.status === 'pass') acc.passed++;
      else acc.failed++;
      return acc;
    },
    { tests: 0, passed: 0, failed: 0 },
  );
  const body = m.tests
    .map((t) => {
      const props = t.detail
        ? `<properties><property name="detail" value="${esc(t.detail)}"/><property name="durationMs" value="${esc(t.durationMs)}"/></properties>`
        : `<properties><property name="durationMs" value="${esc(t.durationMs)}"/></properties>`;
      const inner = t.status === 'fail' ? `<failure message="${esc(t.detail || 'failed')}"/>` : '';
      return `    <testcase classname="${esc(t.suite)}" name="${esc(t.name)}" time="${(t.durationMs / 1000).toFixed(3)}">${props}${inner}</testcase>`;
    })
    .join('\n');
  return (
    `<?xml version="1.0" encoding="UTF-8"?>\n` +
    `<testsuites name="${esc(m.suite)}" tests="${suite.tests}" failures="${suite.failed}" errors="0" skipped="0" time="${(0).toFixed(3)}">\n` +
    `  <testsuite name="${esc(m.suite)}" tests="${suite.tests}" failures="${suite.failed}" errors="0" skipped="0" time="${(0).toFixed(3)}">\n` +
    `${body}\n` +
    `  </testsuite>\n</testsuites>\n`
  );
}

/** Paths the CI job / docs reference. */
export function resultsPaths() {
  return { dir: RESULTS_DIR, manifest: MANIFEST, junit: JUNIT };
}
