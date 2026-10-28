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

test('verbunden: bis 40 s ohne Byte weiter, danach gilt die Verbindung als abgerissen', () => {
  const zustand = { verbunden: true, letzteAktivitaetMs: 1000, fehlversuche: 0 };

  assert.deepEqual(entscheiden({ ...zustand, jetztMs: 1000 }), { aktion: 'weiter', wartenMs: 0 });
  assert.deepEqual(entscheiden({ ...zustand, jetztMs: 41000 }), { aktion: 'weiter', wartenMs: 0 });
  assert.deepEqual(entscheiden({ ...zustand, jetztMs: 41001 }), { aktion: 'neu', wartenMs: 0 });
});
