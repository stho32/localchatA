// Unit-Tests „eigene Nachricht erkennen“ und der Aktionen einer Nachricht (R00041 F0135 B0608; node --test): bearbeiten
// und löschen darf nur, wessen Kennung die Nachricht trägt — bestätigt, nicht gelöscht, auch aus einer früheren
// Sitzung derselben Kennung; die Bindung gilt je Aktion (PO 145).
import test from 'node:test';
import assert from 'node:assert/strict';
import { bearbeitbar, aktionenFuer } from '../../web/nachricht.js';

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

test('Aktionen: an der eigenen Bearbeiten und Löschen in dieser Reihenfolge, an fremden und gelöschten keine', () => {
  assert.deepEqual(aktionenFuer(nachricht(), ICH), ['bearbeiten', 'loeschen']);
  assert.deepEqual(aktionenFuer(nachricht({ von: 'beta@127.0.0.2' }), ICH), []);
  assert.deepEqual(aktionenFuer(nachricht({ zustand: 'geloescht' }), ICH), []);
  assert.deepEqual(aktionenFuer({ von: ICH, text: 'Unterwegs' }, ICH), []);
});
