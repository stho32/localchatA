// Unit-Tests des Fensters im Verlauf (R00007 F0031 B0162, B0163; ADR-0026 Regeln 1, 2; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { nachladen } from '../../web/fenster.js';

const SICHT = 800;
const lage = (werte) => ({
  bloecke: 1, aelter: false, neuer: false, abstandOben: 5000, abstandUnten: 5000, sicht: SICHT, laeuft: false, ...werte,
});

test('oben: unter 2 Sichthöhen mit älteren lädt älter, ab 2 Sichthöhen oder ohne ältere nicht', () => {
  assert.equal(nachladen(lage({ abstandOben: 1599, aelter: true })), 'aelter');
  assert.equal(nachladen(lage({ abstandOben: 0, aelter: true })), 'aelter');
  assert.equal(nachladen(lage({ abstandOben: 1600, aelter: true })), null);
  assert.equal(nachladen(lage({ abstandOben: 1599, aelter: false })), null);
});

test('unten: unter 2 Sichthöhen mit neueren lädt neuer, ab 2 Sichthöhen oder ohne neuere nicht', () => {
  assert.equal(nachladen(lage({ abstandUnten: 1599, neuer: true })), 'neuer');
  assert.equal(nachladen(lage({ abstandUnten: 1600, neuer: true })), null);
  assert.equal(nachladen(lage({ abstandUnten: 1599, neuer: false })), null);
});

test('die Schwelle folgt der Sichthöhe', () => {
  assert.equal(nachladen(lage({ abstandOben: 799, aelter: true, sicht: 400 })), 'aelter');
  assert.equal(nachladen(lage({ abstandOben: 800, aelter: true, sicht: 400 })), null);
});

test('läuft ein Abruf oder ist nichts geladen, wird nicht nachgeladen', () => {
  assert.equal(nachladen(lage({ abstandOben: 0, aelter: true, laeuft: true })), null);
  assert.equal(nachladen(lage({ abstandUnten: 0, neuer: true, laeuft: true })), null);
  assert.equal(nachladen(lage({ abstandOben: 0, aelter: true, bloecke: 0 })), null);
});
