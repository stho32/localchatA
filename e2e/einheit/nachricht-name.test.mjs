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

// Antwort (R00047 B0716, Z1): der Name nennt das Original — „<Kennung>, <HH:MM>, Antwort auf <Kennung> <HH:MM>: <Text>“.
const bezug = (extra = {}) => ({ zeit: '2026-10-28T14:03:00.000+01:00', von: 'anna@127.0.0.2', zustand: 'da', textanfang: 'Frage?', anhang: null, geaendert: null, ...extra });

test('Antwort: nach Kennung und Uhrzeit „Antwort auf <Kennung> <HH:MM>“, dann der eigene Text', () => {
  assert.equal(zugaenglicherName(nachricht({ text: 'Ja', antwort_auf: bezug() })), 'beta@127.0.0.1, 14:09, Antwort auf anna@127.0.0.2 14:03: Ja');
  assert.equal(zugaenglicherName(nachricht({ text: 'Ja', antwort_auf: bezug({ zustand: 'bearbeitet', textanfang: 'Frage!' }) })), 'beta@127.0.0.1, 14:09, Antwort auf anna@127.0.0.2 14:03: Ja');
});

test('Antwort auf ein gelöschtes Original: „…, Nachricht gelöscht: <Text>“; auf ein entfallenes: „Antwort auf ältere Nachricht, nicht mehr vorhanden“', () => {
  assert.equal(zugaenglicherName(nachricht({ text: 'Ja', antwort_auf: bezug({ zustand: 'geloescht', textanfang: '' }) })),
    'beta@127.0.0.1, 14:09, Antwort auf anna@127.0.0.2 14:03, Nachricht gelöscht: Ja');
  assert.equal(zugaenglicherName(nachricht({ text: 'Ja', antwort_auf: bezug({ zustand: 'entfallen', textanfang: '' }) })),
    'beta@127.0.0.1, 14:09, Antwort auf ältere Nachricht, nicht mehr vorhanden: Ja');
});

test('Antwort, selbst bearbeitet: die Marke steht vor dem Bezug; ohne Bezug (null) wie bisher', () => {
  assert.equal(zugaenglicherName(nachricht({ text: 'Ja', zustand: 'bearbeitet', antwort_auf: bezug() })), 'beta@127.0.0.1, 14:09, bearbeitet, Antwort auf anna@127.0.0.2 14:03: Ja');
  assert.equal(zugaenglicherName(nachricht({ antwort_auf: null })), 'beta@127.0.0.1, 14:09: Hallo');
});

// Anhänge (R00030 F0102 B0461, UX V5): ohne Text nennt der Name die Anhänge, mit Text ihre Zahl im Kopf.
const datei = (name, zustand = 'vorhanden') => ({ id: name, name, bytes: 1, art: 'image/png', breite: 1, hoehe: 1, url: `/anhaenge/${name}`, zustand });

test('ohne Text, ein Anhang: „<Kennung>, <HH:MM>: Anhang: <name>“', () => {
  assert.equal(zugaenglicherName(nachricht({ text: '', anhaenge: [datei('foto.png')] })), 'beta@127.0.0.1, 14:09: Anhang: foto.png');
});

test('ohne Text, zwei Anhänge: Zahl vorn, Namen in Sendefolge', () => {
  assert.equal(zugaenglicherName(nachricht({ text: '', anhaenge: [datei('foto.png'), datei('bericht.pdf')] })),
    'beta@127.0.0.1, 14:09: 2 Anhänge: foto.png, bericht.pdf');
});

test('ohne Text, zwölf lange Namen: die Namensliste endet nach 120 Zeichen mit „…“, die Zahl bleibt vorn', () => {
  const namen = Array.from({ length: 12 }, (_, nummer) => `bericht-aus-dem-dritten-quartal-${nummer + 1}.pdf`);
  const liste = namen.join(', ');
  assert.equal(zugaenglicherName(nachricht({ text: ' ', anhaenge: namen.map((name) => datei(name)) })),
    `beta@127.0.0.1, 14:09: 12 Anhänge: ${Array.from(liste).slice(0, 120).join('')}…`);
});

test('mit Text: der Kopf nennt die Zahl — „, 1 Anhang“ bzw. „, 2 Anhänge“ — dann der Textanfang', () => {
  assert.equal(zugaenglicherName(nachricht({ text: 'Bericht Q3', anhaenge: [datei('bericht.pdf')] })), 'beta@127.0.0.1, 14:09, 1 Anhang: Bericht Q3');
  assert.equal(zugaenglicherName(nachricht({ text: 'Bericht Q3', anhaenge: [datei('a.pdf'), datei('b.pdf')] })), 'beta@127.0.0.1, 14:09, 2 Anhänge: Bericht Q3');
});

test('bearbeitet mit Anhängen: „…, bearbeitet, 2 Anhänge: …“', () => {
  assert.equal(zugaenglicherName(nachricht({ text: 'Neu', zustand: 'bearbeitet', anhaenge: [datei('a.pdf'), datei('b.pdf')] })),
    'beta@127.0.0.1, 14:09, bearbeitet, 2 Anhänge: Neu');
});

test('ein gelöschter Anhang heißt „<name> (gelöscht)“; eine gelöschte Nachricht nennt keine Anhänge', () => {
  assert.equal(zugaenglicherName(nachricht({ text: '', anhaenge: [datei('foto.png', 'geloescht'), datei('log.txt')] })),
    'beta@127.0.0.1, 14:09: 2 Anhänge: foto.png (gelöscht), log.txt');
  assert.equal(zugaenglicherName(nachricht({ zustand: 'geloescht', text: '', anhaenge: [datei('foto.png')] })), 'beta@127.0.0.1, 14:09, Nachricht gelöscht');
});

test('ein Name mit Markup bleibt Text', () => {
  assert.equal(zugaenglicherName(nachricht({ text: '', anhaenge: [datei('<img src=x onerror=alert(1)>.png')] })),
    'beta@127.0.0.1, 14:09: Anhang: <img src=x onerror=alert(1)>.png');
});
