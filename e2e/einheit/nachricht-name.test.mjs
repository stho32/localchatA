// Unit-Tests des zugänglichen Namens einer Nachricht (R00041 F0141, B0632; node --test): Kennung, Uhrzeit im Versatz
// der Nachricht, Marke je Zustand, danach der Textanfang.
import test from 'node:test';
import assert from 'node:assert/strict';
import { zugaenglicherName } from '../../web/nachricht.js';

const ZEIT = '2026-10-28T14:09:30.123+01:00';
const nachricht = (extra = {}) => ({ von: 'beta@127.0.0.1', zeit: ZEIT, text: 'Hallo', ...extra });

test('Kennung, Uhrzeit und Text', () => {
  assert.equal(zugaenglicherName(nachricht()), 'beta@127.0.0.1, 14:09: Hallo');
});

test('bearbeitet steht hinter der Uhrzeit', () => {
  assert.equal(zugaenglicherName(nachricht({ zustand: 'bearbeitet' })), 'beta@127.0.0.1, 14:09, bearbeitet: Hallo');
});

test('gelöscht nennt die Marke und keinen Text', () => {
  assert.equal(zugaenglicherName(nachricht({ zustand: 'geloescht', text: 'Geheim' })), 'beta@127.0.0.1, 14:09, Nachricht gelöscht');
});

test('der Textanfang ist eine Zeile: Zeilenumbrüche und Leerraum werden ein Leerzeichen', () => {
  assert.equal(zugaenglicherName(nachricht({ text: 'Eins\n\nZwei   drei\t vier' })), 'beta@127.0.0.1, 14:09: Eins Zwei drei vier');
});

test('ein langer Text endet nach 120 Zeichen mit einem Auslassungszeichen, ohne ein Zeichenpaar zu zerreißen', () => {
  const lang = zugaenglicherName(nachricht({ text: 'a'.repeat(300) }));
  assert.equal(lang, `beta@127.0.0.1, 14:09: ${'a'.repeat(120)}…`);
  const mitPaaren = zugaenglicherName(nachricht({ text: '😀'.repeat(121) }));
  assert.equal(mitPaaren, `beta@127.0.0.1, 14:09: ${'😀'.repeat(120)}…`);
  const genau = zugaenglicherName(nachricht({ text: 'b'.repeat(120) }));
  assert.equal(genau, `beta@127.0.0.1, 14:09: ${'b'.repeat(120)}`);
});

test('ein Text nur aus Leerraum lässt den Doppelpunkt weg', () => {
  assert.equal(zugaenglicherName(nachricht({ text: ' \n ' })), 'beta@127.0.0.1, 14:09');
});
