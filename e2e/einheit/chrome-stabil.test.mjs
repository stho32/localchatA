// Wächter für das Projekt `chrome-stabil` (ADR-0014 Ergänzung „Karte #71 (Chrome 154)“, C6; R00029 „`chrome-stabil`
// im Gate“; node --test): der Direktlink auf Medien läuft auch in Google Chrome stable, und keine Spec kann sich dem
// still entziehen — ein übersprungener Test wäre im Gate grün.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const E2E = path.resolve(import.meta.dirname, '..');
const DIREKTLINK_SPECS = [
  'r00029-direktlink-spielt.spec.js',
  'r00029-direktlink-kein-skript.spec.js',
  'r00029-direktlink-fremdseite.spec.js',
];

function projekte() {
  const konfiguration = createRequire(import.meta.url)(path.join(E2E, 'playwright.config.js'));
  return konfiguration.projects;
}

test('neben chromium läuft das Projekt chrome-stabil mit Google Chrome stable über die Direktlink-Specs', () => {
  const namen = projekte().map((projekt) => projekt.name);
  const chromeStabil = projekte().find((projekt) => projekt.name === 'chrome-stabil');

  assert.deepEqual(namen, ['chromium', 'chrome-stabil']);
  assert.equal(chromeStabil.use.channel, 'chrome');
  for (const spec of DIREKTLINK_SPECS) {
    assert.ok(chromeStabil.testMatch.includes(spec), spec);
  }
});

test('keine Spec überspringt, markiert oder verzweigt nach Projekt oder Browser', () => {
  const TESTS = path.join(E2E, 'tests');
  const ausweichen = /\btest\s*\.\s*(skip|fixme|fail)\b|\bfixme\b|\.project\b|\bbrowserName\b|\bchannel\s*[!=]==?/;
  const specs = fs.readdirSync(TESTS).filter((datei) => datei.endsWith('.spec.js'));
  const ausweichende = specs.filter((datei) => ausweichen.test(fs.readFileSync(path.join(TESTS, datei), 'utf8')));

  assert.ok(specs.includes(DIREKTLINK_SPECS[0]));
  assert.deepEqual(ausweichende, []);
});
