// Unit-Tests der Rückfrage vor dem Löschen (R00041 F0139 B0622, UX S1; node --test) und der Größe in Worten nach R00030.
import test from 'node:test';
import assert from 'node:assert/strict';
import { rueckfrageText, groesseInWorten } from '../../web/loeschen.js';

const ZEIT = '2026-10-30T14:09:12.345+01:00';
const nachricht = (extra) => ({ zeit: ZEIT, von: 'shoff@127.0.0.1', text: 'Bericht', zustand: null, geaendert: null, anhaenge: [], ...extra });
const anhang = (name, bytes, zustand = 'vorhanden') => ({ id: name, name, bytes, zustand });

test('ohne Anhänge: „Diese Nachricht löschen?“, darunter Uhrzeit und Textanfang, zuletzt „Das lässt sich nicht rückgängig machen.“', () => {
  assert.deepEqual(rueckfrageText(nachricht()), {
    frage: 'Diese Nachricht löschen?', auszug: '14:09: Bericht', warnung: 'Das lässt sich nicht rückgängig machen.',
  });
});

test('zwei Anhänge: Zahl und Summe ihrer Größe in Worten, Plural', () => {
  const mitZwei = nachricht({ anhaenge: [anhang('foto.png', 307200), anhang('bericht.pdf', 2457600)] });
  assert.equal(rueckfrageText(mitZwei).frage, 'Diese Nachricht mit 2 Anhängen (2,6 MiB) löschen? Die Anhänge werden mitgelöscht.');
});

test('ein Anhang: Singular', () => {
  assert.equal(rueckfrageText(nachricht({ anhaenge: [anhang('foto.png', 307200)] })).frage,
    'Diese Nachricht mit 1 Anhang (300 KiB) löschen? Der Anhang wird mitgelöscht.');
});

test('gelöschte Anhänge zählen nicht', () => {
  const einerGeloescht = nachricht({ anhaenge: [anhang('foto.png', 307200), anhang('bericht.pdf', 2457600, 'geloescht')] });
  assert.equal(rueckfrageText(einerGeloescht).frage, 'Diese Nachricht mit 1 Anhang (300 KiB) löschen? Der Anhang wird mitgelöscht.');
  const beideGeloescht = nachricht({ anhaenge: [anhang('foto.png', 307200, 'geloescht'), anhang('bericht.pdf', 2457600, 'geloescht')] });
  assert.equal(rueckfrageText(beideGeloescht).frage, 'Diese Nachricht löschen?');
});

test('Textanfang: die ersten 40 Zeichen und „…“; eine Zeile; ohne Text nur die Uhrzeit', () => {
  assert.equal(rueckfrageText(nachricht({ text: 'a'.repeat(45) })).auszug, `14:09: ${'a'.repeat(40)}…`);
  assert.equal(rueckfrageText(nachricht({ text: 'a'.repeat(40) })).auszug, `14:09: ${'a'.repeat(40)}`);
  assert.equal(rueckfrageText(nachricht({ text: 'Eins\nZwei' })).auszug, '14:09: Eins Zwei');
  assert.equal(rueckfrageText(nachricht({ text: '' })).auszug, '14:09');
});

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
