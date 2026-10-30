// Unit-Tests der Größe in Worten nach R00030 „Größe in Worten“ (node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { groesseInWorten } from '../../web/groesse.js';

test('Größe in Worten (R00030): Byte, KiB ganz und kaufmännisch, ab 1 MiB eine Nachkommastelle mit Komma, GiB', () => {
  const faelle = [
    [0, '0 Byte'], [812, '812 Byte'], [1023, '1.023 Byte'], [1024, '1 KiB'], [307200, '300 KiB'], [1048063, '1.023 KiB'],
    [1048064, '1,0 MiB'], [1048575, '1,0 MiB'], [1048576, '1,0 MiB'], [2457600, '2,3 MiB'], [2764800, '2,6 MiB'],
    [1610612736, '1,5 GiB'],
  ];
  for (const [bytes, worte] of faelle) {
    assert.equal(groesseInWorten(bytes), worte, `${bytes}`);
  }
});
