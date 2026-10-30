// Unit-Tests der Liste gewählter Dateien (R00030 F0101 B0449, node --test): höchstens 10 je Nachricht, die Ansage der
// Auswahl (UX M2), der Hinweis über die Zehn hinaus, das Typkürzel.
import test from 'node:test';
import assert from 'node:assert/strict';
import { uebernehmen, auswahlAnsage, uebernahmeHinweis, typkuerzel } from '../../web/dateiliste.js';

const namen = (anzahl, ab = 1) => Array.from({ length: anzahl }, (_, stelle) => `d${stelle + ab}.txt`);

test('zwölf gewählt: die ersten zehn kommen in die Liste, zwei nicht', () => {
  const ergebnis = uebernehmen([], namen(12));
  assert.deepEqual(ergebnis.dateien, namen(10));
  assert.deepEqual(ergebnis.uebernommen, namen(10));
  assert.equal(ergebnis.nichtUebernommen, 2);
});

test('acht stehen schon da, vier gewählt: zwei kommen dazu, zwei nicht', () => {
  const ergebnis = uebernehmen(namen(8), namen(4, 9));
  assert.deepEqual(ergebnis.dateien, namen(10));
  assert.deepEqual(ergebnis.uebernommen, ['d9.txt', 'd10.txt']);
  assert.equal(ergebnis.nichtUebernommen, 2);
});

test('unter der Zehn wird alles übernommen', () => {
  assert.deepEqual(uebernehmen(['a'], ['b', 'c']), { dateien: ['a', 'b', 'c'], uebernommen: ['b', 'c'], nichtUebernommen: 0 });
});

test('Ansage der Auswahl: eine, zwei, zwölf Dateien (UX M2, wörtlich)', () => {
  assert.equal(auswahlAnsage(['log.txt'], 0), 'Datei angehängt: log.txt');
  assert.equal(auswahlAnsage(['foto.png', 'log.txt'], 0), '2 Dateien angehängt: foto.png, log.txt');
  assert.equal(
    auswahlAnsage(namen(10), 2),
    `10 Dateien angehängt: ${namen(10).join(', ')} — 2 nicht übernommen, höchstens 10 je Nachricht`,
  );
});

test('Hinweiszeile nur, wenn etwas nicht übernommen wurde', () => {
  assert.equal(uebernahmeHinweis(0), '');
  assert.equal(uebernahmeHinweis(2), 'Höchstens 10 Dateien je Nachricht — 2 nicht übernommen.');
});

test('Typkürzel aus der letzten Endung, höchstens 5 Zeichen, ohne Endung DATEI', () => {
  assert.equal(typkuerzel('foto.png'), 'PNG');
  assert.equal(typkuerzel('archiv.tar.gz'), 'GZ');
  assert.equal(typkuerzel('film.ogv'), 'OGV');
  assert.equal(typkuerzel('Makefile'), 'DATEI');
  assert.equal(typkuerzel('.bashrc'), 'DATEI');
  assert.equal(typkuerzel('punkt.'), 'DATEI');
  assert.equal(typkuerzel('daten.sehrlang'), 'SEHRL');
});
