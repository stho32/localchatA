// Unit-Tests „eigene Nachricht erkennen“ und der Aktionen einer Nachricht (R00041 F0135 B0608; node --test): bearbeiten
// und löschen darf nur, wessen Kennung die Nachricht trägt — bestätigt, nicht gelöscht, auch aus einer früheren
// Sitzung derselben Kennung; die Bindung gilt je Aktion (PO 145): Antworten (R00047) und Kopieren (R00046) stehen auch an
// fremden.
import test from 'node:test';
import assert from 'node:assert/strict';
import { bearbeitbar, antwortbar, aktionenFuer } from '../../web/nachricht.js';

const ICH = 'shoff@127.0.0.1';
const nachricht = (extra) => ({ zeit: '2026-10-30T14:00:00.000+01:00', von: ICH, art: 'mensch', text: 'Hallo', zustand: null, geaendert: null, ...extra });

test('eigene, bestätigte Nachricht → bearbeitbar, auch bearbeitet', () => {
  assert.equal(bearbeitbar(nachricht(), ICH), true);
  assert.equal(bearbeitbar(nachricht({ zustand: 'bearbeitet', geaendert: '2026-10-30T14:09:00.000+01:00' }), ICH), true);
});

test('fremde Kennung → nicht bearbeitbar, auch bei gleichem Namen von anderer Herkunft', () => {
  assert.equal(bearbeitbar(nachricht({ von: 'beta@127.0.0.2' }), ICH), false);
  assert.equal(bearbeitbar(nachricht({ von: 'shoff@127.0.0.2' }), ICH), false);
});

test('„wird gesendet …“ und fehlgeschlagen (ohne Zeit) → nicht bearbeitbar', () => {
  assert.equal(bearbeitbar({ von: ICH, art: 'mensch', text: 'Unterwegs' }, ICH), false);
});

test('gelöscht → nicht bearbeitbar', () => {
  assert.equal(bearbeitbar(nachricht({ text: '', zustand: 'geloescht', geaendert: '2026-10-30T14:09:00.000+01:00' }), ICH), false);
});

test('ohne bekannte eigene Kennung → nicht bearbeitbar', () => {
  assert.equal(bearbeitbar(nachricht(), ''), false);
});

test('Aktionen: an der eigenen Antworten, Kopieren, Bearbeiten und Löschen in dieser Reihenfolge, an fremden Antworten und Kopieren, an gelöschten keine', () => {
  assert.deepEqual(aktionenFuer(nachricht(), ICH), ['antworten', 'kopieren', 'bearbeiten', 'loeschen']);
  assert.deepEqual(aktionenFuer(nachricht({ von: 'beta@127.0.0.2' }), ICH), ['antworten', 'kopieren']);
  assert.deepEqual(aktionenFuer(nachricht({ von: 'claude@127.0.0.3', art: 'agent' }), ICH), ['antworten', 'kopieren']);
  assert.deepEqual(aktionenFuer(nachricht({ zustand: 'geloescht' }), ICH), []);
  assert.deepEqual(aktionenFuer({ von: ICH, text: 'Unterwegs' }, ICH), []);
});

test('Aktionen: eine Nachricht nur aus Anhängen trägt Antworten, Bearbeiten und Löschen (eigene), aber kein Kopieren', () => {
  assert.deepEqual(aktionenFuer(nachricht({ text: '', anhaenge: [{ id: 'x' }] }), ICH), ['antworten', 'bearbeiten', 'loeschen']);
  assert.deepEqual(aktionenFuer(nachricht({ von: 'beta@127.0.0.2', text: '', anhaenge: [{ id: 'x' }] }), ICH), ['antworten']);
});

test('antwortbar (R00047 B0713): jede bestätigte, nicht gelöschte Nachricht — eigene, fremde, bearbeitete, nur Anhänge', () => {
  assert.equal(antwortbar(nachricht()), true);
  assert.equal(antwortbar(nachricht({ von: 'claude@127.0.0.3', art: 'agent' })), true);
  assert.equal(antwortbar(nachricht({ zustand: 'bearbeitet', geaendert: '2026-10-30T14:09:00.000+01:00' })), true);
  assert.equal(antwortbar(nachricht({ text: '', anhaenge: [{ id: 'x' }] })), true);
});

test('antwortbar: gelöscht, „wird gesendet …“ und fehlgeschlagen (ohne Zeit) nicht', () => {
  assert.equal(antwortbar(nachricht({ text: '', zustand: 'geloescht' })), false);
  assert.equal(antwortbar({ von: ICH, art: 'mensch', text: 'Unterwegs' }), false);
});
