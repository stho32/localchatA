// Unit-Tests des Fensters im Verlauf (R00007 F0031 B0162, B0163; ADR-0026 Regeln 1, 2; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { nachladen, einfuegen } from '../../web/fenster.js';

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

const block = (name, { aelter = true, neuer = true } = {}) => ({ nachrichten: [{ zeit: name }], aelter, neuer });
const fensterAus = (anzahl) => ({
  bloecke: Array.from({ length: anzahl }, (_, index) => [{ zeit: `b${index}` }]), aelter: true, neuer: false,
});
const ersteZeiten = (fenster) => fenster.bloecke.map((bloecke) => bloecke[0].zeit);

test('fünf Blöcke und einer oben: der unterste fällt weg, danach gibt es neuere', () => {
  const ergebnis = einfuegen(fensterAus(5), block('neu', { aelter: false }), 'aelter');

  assert.deepEqual(ersteZeiten(ergebnis.fenster), ['neu', 'b0', 'b1', 'b2', 'b3']);
  assert.deepEqual(ergebnis.abgeworfen, [{ zeit: 'b4' }]);
  assert.equal(ergebnis.fenster.aelter, false);
  assert.equal(ergebnis.fenster.neuer, true);
});

test('fünf Blöcke und einer unten: der oberste fällt weg, davor gibt es ältere', () => {
  const fenster = { ...fensterAus(5), aelter: false, neuer: true };

  const ergebnis = einfuegen(fenster, block('neu', { neuer: false }), 'neuer');

  assert.deepEqual(ersteZeiten(ergebnis.fenster), ['b1', 'b2', 'b3', 'b4', 'neu']);
  assert.deepEqual(ergebnis.abgeworfen, [{ zeit: 'b0' }]);
  assert.equal(ergebnis.fenster.aelter, true);
  assert.equal(ergebnis.fenster.neuer, false);
});

test('vier Blöcke und einer: nichts fällt weg (Spanne: höchstens fünf)', () => {
  const oben = einfuegen(fensterAus(4), block('neu', { aelter: false }), 'aelter');
  const unten = einfuegen({ ...fensterAus(4), neuer: true }, block('neu', { neuer: false }), 'neuer');

  assert.equal(oben.fenster.bloecke.length, 5);
  assert.equal(oben.abgeworfen, null);
  assert.equal(oben.fenster.neuer, false);
  assert.equal(unten.fenster.bloecke.length, 5);
  assert.equal(unten.abgeworfen, null);
  assert.equal(unten.fenster.aelter, true);
});

test('ein leerer Block ändert nur die Angaben, keinen Block', () => {
  const ergebnis = einfuegen(fensterAus(2), { nachrichten: [], aelter: false, neuer: true }, 'aelter');

  assert.equal(ergebnis.fenster.bloecke.length, 2);
  assert.equal(ergebnis.fenster.aelter, false);
  assert.equal(ergebnis.abgeworfen, null);
});
