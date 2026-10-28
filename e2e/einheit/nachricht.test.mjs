// Unit-Tests der Beschriftung langer Nachrichten (R00007 F0030, Nacharbeit 2; node --test): die Leiste nennt die
// Zahl der Zeilen im richtigen Numerus.
import test from 'node:test';
import assert from 'node:assert/strict';
import { leistenBeschriftung } from '../../web/nachricht.js';

test('eine Zeile steht im Singular', () => {
  assert.equal(leistenBeschriftung(1), 'Ganze Nachricht anzeigen · 1 Zeile');
});

test('ab zwei Zeilen steht der Plural', () => {
  assert.equal(leistenBeschriftung(2), 'Ganze Nachricht anzeigen · 2 Zeilen');
  assert.equal(leistenBeschriftung(30), 'Ganze Nachricht anzeigen · 30 Zeilen');
});
