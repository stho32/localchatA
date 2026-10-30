// Unit-Tests der Darstellung geänderter Nachrichten (R00041 F0134 B0605, F0138 B0620; node --test): eine bearbeitete
// Nachricht zeigt ihren neuen Text und die Marke „bearbeitet“ mit der Zeit der letzten Änderung, eine gelöschte statt
// Text und Anhängen die Marke „Nachricht gelöscht“.
import test from 'node:test';
import assert from 'node:assert/strict';
import { inhaltDarstellen } from '../../web/nachricht.js';

const nachricht = (extra) => ({
  zeit: '2026-10-30T14:00:00.000+01:00', von: 'beta@127.0.0.2', art: 'agent', text: 'Hallo', zustand: null, geaendert: null,
  anhaenge: [], ...extra,
});

test('bearbeitet: neuer Text, Marke „bearbeitet“, die Zeit der letzten Änderung als „Zuletzt bearbeitet: TT.MM.JJJJ, HH:MM“', () => {
  const darstellung = inhaltDarstellen(nachricht({ text: 'Hallo zusammen', zustand: 'bearbeitet', geaendert: '2026-10-30T14:09:00.000+01:00' }));
  assert.deepEqual(darstellung, { text: 'Hallo zusammen', marke: 'bearbeitet', zuletztBearbeitet: 'Zuletzt bearbeitet: 30.10.2026, 14:09' });
});

test('die Zeit der Marke steht im Versatz der Änderung, nicht in der Zeitzone des Rechners', () => {
  const darstellung = inhaltDarstellen(nachricht({ zustand: 'bearbeitet', geaendert: '2026-12-31T23:59:59.999+01:00' }));
  assert.equal(darstellung.zuletztBearbeitet, 'Zuletzt bearbeitet: 31.12.2026, 23:59');
});

test('ohne Zustand: der Text ohne Marke', () => {
  assert.deepEqual(inhaltDarstellen(nachricht()), { text: 'Hallo', marke: null, zuletztBearbeitet: null });
});

test('ohne das Feld zustand (ältere Antworten, eigene unterwegs): der Text ohne Marke', () => {
  assert.deepEqual(inhaltDarstellen({ von: 'shoff@127.0.0.1', text: 'Unterwegs' }), { text: 'Unterwegs', marke: null, zuletztBearbeitet: null });
});

test('gelöscht: kein Text, die Marke „Nachricht gelöscht“ — auch an einer eigenen Nachricht', () => {
  const darstellung = inhaltDarstellen(nachricht({ von: 'shoff@127.0.0.1', text: '', zustand: 'geloescht', geaendert: '2026-10-30T14:09:00.000+01:00' }));
  assert.deepEqual(darstellung, { text: null, marke: 'geloescht', zuletztBearbeitet: null });
});

test('gelöscht verwirft auch einen Text, den eine fehlerhafte Antwort noch trüge', () => {
  assert.equal(inhaltDarstellen(nachricht({ text: 'Geheim', zustand: 'geloescht', geaendert: '2026-10-30T14:09:00.000+01:00' })).text, null);
});
