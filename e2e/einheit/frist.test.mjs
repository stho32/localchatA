// Unit-Tests der Frist im Browser (R00008 F0037, B0193, B0194; ADR-0033 Regel 3; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { fristPruefen } from '../../web/frist.js';

const FEHLER = 'Frist: ganze Zahl von 1 bis 3650 Tagen.';

test('fristPruefen: 1, 3650, 7 und 007 sind gültig (führende Nullen erlaubt)', () => {
  assert.deepEqual(fristPruefen('1'), { gueltig: true, tage: 1 });
  assert.deepEqual(fristPruefen('3650'), { gueltig: true, tage: 3650 });
  assert.deepEqual(fristPruefen('7'), { gueltig: true, tage: 7 });
  assert.deepEqual(fristPruefen('007'), { gueltig: true, tage: 7 });
});

test('fristPruefen: 0, 3651 und alles außer Ziffern ist ungültig mit der Fehlerzeile', () => {
  for (const text of ['0', '3651', '-1', '7.5', '7,5', '1e3', '', ' 7', '7 ', '+7', '0000', '99999999999999999999']) {
    assert.deepEqual(fristPruefen(text), { gueltig: false, fehler: FEHLER }, JSON.stringify(text));
  }
});
