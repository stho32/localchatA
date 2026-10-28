// Unit-Tests des Wächters der Live-Verbindung (R00004 F0013, ADR-0015 Regel 4; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { entscheiden } from '../../web/verbindung.js';

test('getrennt: neu verbinden sofort beim ersten Versuch, dann nach 1 s, 2 s, 5 s und alle 10 s', () => {
  const wartezeiten = [0, 1, 2, 3, 4, 5, 9].map((fehlversuche) =>
    entscheiden({ verbunden: false, letzteAktivitaetMs: 0, jetztMs: 0, fehlversuche }));

  assert.deepEqual(wartezeiten.map((entscheidung) => entscheidung.aktion), Array(7).fill('neu'));
  assert.deepEqual(wartezeiten.map((entscheidung) => entscheidung.wartenMs), [0, 1000, 2000, 5000, 10000, 10000, 10000]);
});

// R00005 F0016 (ADR-0017 Regel 6): drei Herzschläge zu 5 s — 15 000 ms ohne Byte weiter, 15 001 ms neu verbinden.
test('verbunden: bis 15 s ohne Byte weiter, danach gilt die Verbindung als abgerissen', () => {
  const zustand = { verbunden: true, letzteAktivitaetMs: 1000, fehlversuche: 0 };

  assert.deepEqual(entscheiden({ ...zustand, jetztMs: 1000 }), { aktion: 'weiter', wartenMs: 0 });
  assert.deepEqual(entscheiden({ ...zustand, jetztMs: 16000 }), { aktion: 'weiter', wartenMs: 0 });
  assert.deepEqual(entscheiden({ ...zustand, jetztMs: 16001 }), { aktion: 'neu', wartenMs: 0 });
});
