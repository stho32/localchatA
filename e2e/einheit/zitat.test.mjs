// Unit-Tests des Bezugs im Tab (R00047 I0044, ADR-0053 Regeln 6, 7; node --test): aus einer Nachricht des Fensters wird
// der Bezug der Antwortzeile, und was eine Zeile oder ein Zitat zu einem Bezug zeigt.
import test from 'node:test';
import assert from 'node:assert/strict';
import { bezugAusNachricht, zitatText, zitatName } from '../../web/zitat.js';

const ZEIT = '2026-10-30T14:03:00.000+01:00';

test('Bezug aus einer Nachricht des Fensters: Zeit, Kennung, Zustand da, der ganze Text als Textanfang (gekürzt wird per CSS)', () => {
  const text = `Frage?\n${'x'.repeat(300)}`;
  assert.deepEqual(bezugAusNachricht({ zeit: ZEIT, von: 'anna@127.0.0.2', art: 'mensch', text, zustand: null, geaendert: null }), {
    zeit: ZEIT, von: 'anna@127.0.0.2', zustand: 'da', textanfang: text, anhang: null, geaendert: null,
  });
});

test('Bezug aus einer bearbeiteten Nachricht trägt ihr geaendert, aus einer nur mit Anhängen den Namen des ersten vorhandenen', () => {
  const geaendert = '2026-10-30T14:09:00.000+01:00';
  assert.equal(bezugAusNachricht({ zeit: ZEIT, von: 'a@b', text: 'Neu', zustand: 'bearbeitet', geaendert }).geaendert, geaendert);
  const nurAnhang = bezugAusNachricht({
    zeit: ZEIT, von: 'a@b', text: '', anhaenge: [{ name: 'weg.pdf', zustand: 'geloescht' }, { name: 'bericht.pdf', zustand: 'vorhanden' }],
  });
  assert.equal(nurAnhang.anhang, 'bericht.pdf');
  assert.equal(nurAnhang.textanfang, '');
  assert.equal(bezugAusNachricht({ zeit: ZEIT, von: 'a@b', text: 'Mit Bild', anhaenge: [{ name: 'b.png', zustand: 'vorhanden' }] }).anhang, null);
});

test('zitatText: Textanfang; gelöscht, entfallen und nur Anhang mit ihrem Wortlaut (Vorprüfung ux-reviewer)', () => {
  const bezug = { zeit: ZEIT, von: 'a@b', zustand: 'da', textanfang: 'Frage?', anhang: null, geaendert: null };
  assert.equal(zitatText(bezug), 'Frage?');
  assert.equal(zitatText({ ...bezug, zustand: 'bearbeitet', textanfang: 'Frage!' }), 'Frage!');
  assert.equal(zitatText({ ...bezug, zustand: 'geloescht', textanfang: '' }), 'Nachricht gelöscht');
  assert.equal(zitatText({ ...bezug, zustand: 'entfallen', textanfang: '' }), 'Ältere Nachricht, nicht mehr vorhanden');
  assert.equal(zitatText({ ...bezug, textanfang: '', anhang: 'bericht.pdf' }), 'Anhang: bericht.pdf');
});

test('zitatText: ein Zustand, den der Tab nicht kennt, ist ein Fehler, kein leeres Zitat', () => {
  assert.throws(() => zitatText({ zeit: ZEIT, von: 'a@b', zustand: 'verschoben', textanfang: 'x', anhang: null }), /verschoben/);
});

test('zitatName: „Zur Nachricht springen: <Kennung> <HH:MM>, <was das Zitat zeigt>“ — ohne das ↩, die Uhrzeit im Versatz des Originals', () => {
  const bezug = { zeit: '2026-10-28T14:03:59.999+01:00', von: 'anna@127.0.0.2', zustand: 'da', textanfang: 'Frage?', anhang: null, geaendert: null };
  assert.equal(zitatName(bezug), 'Zur Nachricht springen: anna@127.0.0.2 14:03, Frage?');
  assert.equal(zitatName({ ...bezug, zustand: 'geloescht', textanfang: '' }), 'Zur Nachricht springen: anna@127.0.0.2 14:03, Nachricht gelöscht');
  assert.equal(zitatName({ ...bezug, textanfang: '', anhang: 'bericht.pdf' }), 'Zur Nachricht springen: anna@127.0.0.2 14:03, Anhang: bericht.pdf');
});
