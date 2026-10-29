// Klassenwächter für den Testrahmen (Karte #49; node --test): jede Spec holt `test` aus e2e/tests/rahmen.js, damit
// jeder Browser-Kontext, den ein Test öffnet, mit seinem Test endet — auch in Specs, die erst noch entstehen.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const TESTS = path.resolve(import.meta.dirname, '../tests');

test('jede Spec holt test und expect aus dem Rahmen, keine direkt aus @playwright/test', () => {
  const specs = fs.readdirSync(TESTS).filter((datei) => datei.endsWith('.spec.js'));
  const ohneRahmen = specs.filter((datei) => {
    const quelltext = fs.readFileSync(path.join(TESTS, datei), 'utf8');
    return !quelltext.includes("require('./rahmen')") || /\{[^}]*\btest\b[^}]*\}\s*=\s*require\('@playwright\/test'\)/.test(quelltext);
  });

  assert.ok(specs.length > 0);
  assert.deepEqual(ohneRahmen, []);
});
