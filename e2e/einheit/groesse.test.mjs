// Unit-Tests der Größe in Worten nach R00030 „Größe in Worten“ (node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { groesseInWorten, groesseInByte } from '../../web/groesse.js';

test('Größe in Worten (R00030): Byte, KiB ganz und kaufmännisch, ab 1 MiB eine Nachkommastelle mit Komma, GiB', () => {
  const faelle = [
    [0, '0 Byte'], [812, '812 Byte'], [1023, '1.023 Byte'], [1024, '1 KiB'], [307200, '300 KiB'], [1048063, '1.023 KiB'],
    [1048064, '1,0 MiB'], [1048575, '1,0 MiB'], [1048576, '1,0 MiB'], [2457600, '2,3 MiB'], [2764800, '2,6 MiB'],
    [1610612736, '1,5 GiB'],
  ];
  for (const [bytes, worte] of faelle) {
    assert.equal(groesseInWorten(bytes).replaceAll('\u00a0', ' '), worte, `${bytes}`);
  }
});

// product-owner #85 (Zeile 186), ux-reviewer O1: in der Oberfläche trennt ein geschütztes Leerzeichen (U+00A0) Zahl und
// Einheit — „64 KiB“ bricht nie zwischen den Wörtern; der Server (Rust `groesse_in_worten`) behält das normale Leerzeichen.
test('Größe in Worten trennt Zahl und Einheit mit U+00A0 (Byte, KiB, MiB, GiB)', () => {
  const faelle = [[812, '812\u00a0Byte'], [65536, '64\u00a0KiB'], [2457600, '2,3\u00a0MiB'], [1610612736, '1,5\u00a0GiB']];
  for (const [bytes, worte] of faelle) {
    assert.equal(groesseInWorten(bytes), worte, `${bytes}`);
    assert.doesNotMatch(worte.replace('\u00a0', ''), / /, 'sonst kein Leerzeichen');
  }
  assert.equal(groesseInByte(2998927), '2.998.927\u00a0Byte');
});
