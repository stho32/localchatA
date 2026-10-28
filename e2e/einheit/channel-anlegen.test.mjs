// Unit-Tests zum Anlegen im Browser (R00006 F0025, B0137, B0139; ADR-0024 Regeln 5 und 7; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { kuerzelOeffnet } from '../../web/channel-anlegen.js';

function taste({ ctrlKey = false, altKey = false, key, altGraph = false }) {
  return { ctrlKey, altKey, key, getModifierState: (name) => name === 'AltGraph' && altGraph };
}

test('Strg Alt N öffnet, auch mit Umschalt (N)', () => {
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, altKey: true, key: 'n' })), true);
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, altKey: true, key: 'N' })), true);
});

test('AltGr N öffnet nicht — auf deutschen Tastaturen ist AltGr Strg Alt und tippt ein Zeichen', () => {
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, altKey: true, key: 'n', altGraph: true })), false);
});

test('ohne Alt, ohne Strg oder mit anderer Taste öffnet nichts', () => {
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, key: 'n' })), false);
  assert.equal(kuerzelOeffnet(taste({ altKey: true, key: 'n' })), false);
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, altKey: true, key: 'm' })), false);
});
