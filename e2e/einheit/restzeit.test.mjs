// Unit-Tests der Restzeit (R00005 F0018, B0109, ADR-0019 Regel 5; node --test): rein, gestellte Uhr.
import test from 'node:test';
import assert from 'node:assert/strict';
import { restzeit } from '../../web/restzeit.js';

const T = Date.parse('2026-10-28T09:00:00.000+01:00');

function wegUm(millisekundenNachT) {
  return new Date(T + millisekundenNachT).toISOString();
}

function anzeige(millisekundenNachT, { uhr = T, versatz = 0 } = {}) {
  return restzeit({ wegUm: wegUm(millisekundenNachT), versatz, uhr });
}

test('Tabelle aus ADR-0019: 40 s und 39,001 s → 0:40, 60 s → 1:00, 60,001 s → offline, abgelaufen → 0:00', () => {
  assert.deepEqual(anzeige(40000), { art: 'gleich-weg', text: 'weg in 0:40' });
  assert.deepEqual(anzeige(39001), { art: 'gleich-weg', text: 'weg in 0:40' });
  assert.deepEqual(anzeige(60000), { art: 'gleich-weg', text: 'weg in 1:00' });
  assert.deepEqual(anzeige(60001), { art: 'offline', text: 'offline' });
  assert.deepEqual(anzeige(-1), { art: 'gleich-weg', text: 'weg in 0:00' });
});

test('geht die Uhr des Browsers 30 s vor, gleicht der Versatz sie aus — dieselben Anzeigen', () => {
  const vor = { uhr: T + 30000, versatz: -30000 };

  assert.deepEqual(anzeige(40000, vor), { art: 'gleich-weg', text: 'weg in 0:40' });
  assert.deepEqual(anzeige(39001, vor), { art: 'gleich-weg', text: 'weg in 0:40' });
  assert.deepEqual(anzeige(60000, vor), { art: 'gleich-weg', text: 'weg in 1:00' });
  assert.deepEqual(anzeige(60001, vor), { art: 'offline', text: 'offline' });
  assert.deepEqual(anzeige(-1, vor), { art: 'gleich-weg', text: 'weg in 0:00' });
});

test('einstellige Sekunden mit führender Null, ohne weg_um bleibt es offline', () => {
  assert.deepEqual(anzeige(5000), { art: 'gleich-weg', text: 'weg in 0:05' });
  assert.deepEqual(restzeit({ wegUm: null, versatz: 0, uhr: T }), { art: 'offline', text: 'offline' });
});
