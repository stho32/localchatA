// Unit-Tests der Übernahme-Regel für geänderte Nachrichten (R00041 F0134 B0604, ADR-0048 Regel 11; node --test): eine
// neue Fassung gilt nur, wenn ihr `geaendert` größer ist als das bekannte (`null` kleiner als jede Zeit); gelöscht ist
// endgültig; eine Nachricht außerhalb des Fensters wird verworfen.
import test from 'node:test';
import assert from 'node:assert/strict';
import { fassungUebernehmen } from '../../web/live-verlauf.js';

const ZEIT = '2026-10-28T14:09:30.123+01:00';
const T4 = '2026-10-28T14:20:00.000+01:00';
const T5 = '2026-10-28T14:25:00.000+01:00';
const T6 = '2026-10-28T14:30:00.000+01:00';
const nachricht = (extra) => ({ zeit: ZEIT, von: 'beta@127.0.0.2', art: 'agent', text: 'Hallo', zustand: null, geaendert: null, anhaenge: [], ...extra });

test('bekannt ohne Änderung, neu bearbeitet → die neue Fassung gilt — auch bei gleicher Nachrichtenzeit', () => {
  const neu = nachricht({ text: 'Hallo zusammen', zustand: 'bearbeitet', geaendert: T5 });
  assert.equal(fassungUebernehmen(nachricht(), neu), neu);
});

test('eine ältere Änderung überschreibt nie eine neuere', () => {
  const bekannt = nachricht({ text: 'Neu', zustand: 'bearbeitet', geaendert: T5 });
  assert.equal(fassungUebernehmen(bekannt, nachricht({ text: 'Alt', zustand: 'bearbeitet', geaendert: T4 })), null);
});

test('dieselbe Änderung noch einmal wird verworfen', () => {
  const bekannt = nachricht({ text: 'Neu', zustand: 'bearbeitet', geaendert: T5 });
  assert.equal(fassungUebernehmen(bekannt, nachricht({ text: 'Neu', zustand: 'bearbeitet', geaendert: T5 })), null);
});

test('gelöscht wird nie zurückgenommen, auch nicht durch eine spätere Bearbeitung', () => {
  const bekannt = nachricht({ text: '', zustand: 'geloescht', geaendert: T5 });
  assert.equal(fassungUebernehmen(bekannt, nachricht({ text: 'Wieder da', zustand: 'bearbeitet', geaendert: T6 })), null);
});

test('eine neuere Fassung im Zustand gelöscht gilt', () => {
  const bekannt = nachricht({ text: 'Neu', zustand: 'bearbeitet', geaendert: T5 });
  const neu = nachricht({ text: '', zustand: 'geloescht', geaendert: T6 });
  assert.equal(fassungUebernehmen(bekannt, neu), neu);
});

test('eine Nachricht außerhalb des Fensters wird verworfen, ohne Fehler', () => {
  assert.equal(fassungUebernehmen(undefined, nachricht({ zustand: 'bearbeitet', geaendert: T5 })), null);
  assert.equal(fassungUebernehmen(null, nachricht({ zustand: 'bearbeitet', geaendert: T5 })), null);
});

test('ein vor dem Block gepuffertes Ereignis gilt nach dem Block, der die Nachricht noch ohne Änderung bringt', () => {
  const ausDemBlock = nachricht();
  const gepuffert = nachricht({ text: 'Hallo zusammen', zustand: 'bearbeitet', geaendert: T5 });
  assert.equal(fassungUebernehmen(ausDemBlock, gepuffert), gepuffert);
});

test('eine Fassung ohne Änderung ersetzt nichts', () => {
  assert.equal(fassungUebernehmen(nachricht(), nachricht({ text: 'Anders' })), null);
});

test('die Zeiten werden als Zeitpunkte verglichen, nicht als Text', () => {
  const bekannt = nachricht({ zustand: 'bearbeitet', geaendert: '2026-10-28T14:25:00.000+01:00' });
  const neu = nachricht({ text: 'Später', zustand: 'bearbeitet', geaendert: '2026-10-28T13:26:00.000Z' });
  assert.equal(fassungUebernehmen(bekannt, neu), neu);
});
