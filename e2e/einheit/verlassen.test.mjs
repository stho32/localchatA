// R00005 F0019 (ADR-0018 Regel 6; node --test): die Oberfläche ruft /api/verlassen nie — weder eine Seite noch
// der Worker; geprüft am ausgelieferten Code, weil Playwright Abrufe aus dem SharedWorker nicht sieht.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';

const WEB = new URL('../../web/', import.meta.url);

test('kein Modul der Oberfläche nennt /api/verlassen', () => {
  const module = readdirSync(WEB).filter((name) => name.endsWith('.js') || name.endsWith('.html'));
  const mitVerlassen = module.filter((name) => readFileSync(new URL(name, WEB), 'utf8').includes('verlassen'));

  assert.ok(module.includes('live.js') && module.includes('strom.js'), module.join(', '));
  assert.deepEqual(mitVerlassen, []);
});
