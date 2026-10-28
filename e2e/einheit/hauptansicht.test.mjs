// Unit-Tests der Hauptansicht (R00004 F0015, B0097; node --test): wann die Teilnehmerliste sichtbar ist.
import test from 'node:test';
import assert from 'node:assert/strict';
import { teilnehmerSichtbar } from '../../web/hauptansicht.js';

test('ohne Wunsch ab 1100 px sichtbar, darunter eingeklappt', () => {
  assert.equal(teilnehmerSichtbar(1280, null), true);
  assert.equal(teilnehmerSichtbar(1100, null), true);
  assert.equal(teilnehmerSichtbar(1099, null), false);
  assert.equal(teilnehmerSichtbar(1024, null), false);
  assert.equal(teilnehmerSichtbar(320, null), false);
});

test('der Wunsch des Menschen gilt vor der Breite', () => {
  assert.equal(teilnehmerSichtbar(320, 'ein'), true);
  assert.equal(teilnehmerSichtbar(1280, 'aus'), false);
  assert.equal(teilnehmerSichtbar(1280, 'ein'), true);
  assert.equal(teilnehmerSichtbar(1024, 'aus'), false);
});
