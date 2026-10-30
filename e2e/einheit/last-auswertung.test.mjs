// Unit-Tests der Auswertung des Lastgenerators (R00049 F0170 N5, B0759; node --test): Perzentile nach dem
// Rang-Verfahren (nächster Rang, ⌈p · n⌉) und die Wartezeit aus `Retry-After`.
import test from 'node:test';
import assert from 'node:assert/strict';
import { kennwerte, wartezeitInMs } from '../messung/last-auswertung.mjs';

test('Kennwerte von 1 bis 10 ms: Median 5, p90 9, Maximum 10, unabhängig von der Reihenfolge', () => {
  const dauern = [10, 1, 9, 2, 8, 3, 7, 4, 6, 5];

  assert.deepEqual(kennwerte(dauern), { anzahl: 10, median: 5, p90: 9, max: 10 });
});

test('Kennwerte von 40 Messungen: p90 ist die 36. kleinste', () => {
  const dauern = Array.from({ length: 40 }, (_, index) => 40 - index);

  assert.deepEqual(kennwerte(dauern), { anzahl: 40, median: 20, p90: 36, max: 40 });
});

test('Kennwerte ohne Messung sind null, nie 0', () => {
  assert.deepEqual(kennwerte([]), { anzahl: 0, median: null, p90: null, max: null });
});

test('Wartezeit aus Retry-After in Sekunden; ohne Kopf oder unlesbar 1 s', () => {
  assert.equal(wartezeitInMs('3'), 3000);
  assert.equal(wartezeitInMs(undefined), 1000);
  assert.equal(wartezeitInMs('bald'), 1000);
});
