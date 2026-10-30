// Unit-Tests des Bezugs im Tab (R00047 I0044, ADR-0053 Regeln 6, 7; node --test): aus einer Nachricht des Fensters wird
// der Bezug der Antwortzeile, und was eine Zeile oder ein Zitat zu einem Bezug zeigt.
import test from 'node:test';
import assert from 'node:assert/strict';
import { bezugAusNachricht, zitatText, zitatName, zitatNachfuehren, bezugNachfuehren } from '../../web/zitat.js';

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

// Zitat lebt mit dem Original (R00047 F0160 B0720, ADR-0053 Regel 5): für jede Nachricht mit antwort_auf.zeit gleich
// ereignis.nachricht.zeit (als Zeitpunkt) gilt ereignis.zitat, wenn sein geaendert größer ist als das bekannte (null ist
// kleiner als jede Zeit); gelöscht wird nie zurückgenommen; dieselbe Regel für die offene Antwortzeile.
const ORIGINAL = '2026-10-30T14:03:00.000+01:00';
const bezugAuf = (extra = {}) => ({ zeit: ORIGINAL, von: 'anna@127.0.0.2', zustand: 'da', textanfang: 'Frage?', anhang: null, geaendert: null, ...extra });
const antwortMit = (zeit, antwortAuf) => ({ zeit, von: 'ben@127.0.0.1', text: 'Ja', zustand: null, geaendert: null, antwort_auf: antwortAuf });
const aenderung = (zitat, zeit = ORIGINAL) => ({ channel: 'allgemein', nachricht: { zeit, von: 'anna@127.0.0.2', text: zitat.textanfang, zustand: zitat.zustand, geaendert: zitat.geaendert }, zitat });
const BEARBEITET = bezugAuf({ zustand: 'bearbeitet', textanfang: 'Frage!', geaendert: '2026-10-30T14:09:00.000+01:00' });
const GELOESCHT = bezugAuf({ zustand: 'geloescht', textanfang: '', geaendert: '2026-10-30T14:10:00.000+01:00' });

test('zitatNachfuehren: bearbeitet ersetzt das Zitat jeder Antwort auf dieses Original; andere Bezüge und Nachrichten ohne Bezug bleiben dieselben Objekte', () => {
  const fenster = [
    antwortMit('2026-10-30T14:04:00.000+01:00', bezugAuf()),
    antwortMit('2026-10-30T14:05:00.000+01:00', bezugAuf({ zeit: '2026-10-30T14:01:00.000+01:00' })),
    antwortMit('2026-10-30T14:06:00.000+01:00', null),
    antwortMit('2026-10-30T14:07:00.000+01:00', bezugAuf()),
  ];
  const danach = zitatNachfuehren(fenster, aenderung(BEARBEITET));
  assert.deepEqual(danach[0].antwort_auf, BEARBEITET);
  assert.deepEqual(danach[3].antwort_auf, BEARBEITET);
  assert.equal(danach[0].text, 'Ja');
  assert.equal(danach[1], fenster[1]);
  assert.equal(danach[2], fenster[2]);
  assert.deepEqual(fenster[0].antwort_auf, bezugAuf(), 'das Fenster selbst bleibt unverändert');
});

test('zitatNachfuehren: die Zeit gilt als Zeitpunkt — eine andere Schreibweise desselben Augenblicks trifft', () => {
  const fenster = [antwortMit('2026-10-30T14:04:00.000+01:00', bezugAuf())];
  const danach = zitatNachfuehren(fenster, aenderung(BEARBEITET, '2026-10-30T13:03:00.000+00:00'));
  assert.deepEqual(danach[0].antwort_auf, BEARBEITET);
});

test('zitatNachfuehren: ein älteres oder gleich altes geaendert ändert nichts; null ist kleiner als jede Zeit', () => {
  const neuer = [antwortMit('2026-10-30T14:04:00.000+01:00', BEARBEITET)];
  const aelter = bezugAuf({ zustand: 'bearbeitet', textanfang: 'Alt', geaendert: '2026-10-30T14:08:00.000+01:00' });
  assert.equal(zitatNachfuehren(neuer, aenderung(aelter))[0], neuer[0]);
  assert.equal(zitatNachfuehren(neuer, aenderung({ ...BEARBEITET, textanfang: 'Gleich alt' }))[0], neuer[0]);
  assert.equal(zitatNachfuehren([antwortMit('x', bezugAuf())], aenderung(bezugAuf({ textanfang: 'Ohne Zeit' })))[0].antwort_auf.textanfang, 'Frage?');
});

test('zitatNachfuehren: gelöscht ersetzt und wird nie zurückgenommen, auch nicht von einem späteren bearbeitet', () => {
  const geloescht = zitatNachfuehren([antwortMit('x', BEARBEITET)], aenderung(GELOESCHT));
  assert.deepEqual(geloescht[0].antwort_auf, GELOESCHT);
  const spaeter = bezugAuf({ zustand: 'bearbeitet', textanfang: 'Zurück?', geaendert: '2026-10-30T14:11:00.000+01:00' });
  assert.equal(zitatNachfuehren(geloescht, aenderung(spaeter))[0], geloescht[0]);
});

test('zitatNachfuehren: ein Ereignis ohne zitat ändert nichts (Mutant Z10 im Tab)', () => {
  const fenster = [antwortMit('x', bezugAuf())];
  const { zitat, ...ohneZitat } = aenderung(BEARBEITET);
  assert.equal(zitatNachfuehren(fenster, ohneZitat)[0], fenster[0]);
});

test('bezugNachfuehren: dieselbe Regel für die offene Antwortzeile; ein anderer Bezug und kein Bezug bleiben', () => {
  assert.deepEqual(bezugNachfuehren(bezugAuf(), aenderung(BEARBEITET)), BEARBEITET);
  assert.deepEqual(bezugNachfuehren(BEARBEITET, aenderung(GELOESCHT)), GELOESCHT);
  const anderer = bezugAuf({ zeit: '2026-10-30T14:01:00.000+01:00' });
  assert.equal(bezugNachfuehren(anderer, aenderung(BEARBEITET)), anderer);
  assert.equal(bezugNachfuehren(null, aenderung(BEARBEITET)), null);
});
