// Unit-Tests des Bearbeiten-Felds (R00041 F0135 B0610; node --test): Tasten wie in der Eingabe (Enter speichert, auf
// Touch-Geräten neue Zeile, Strg Enter speichert überall, Esc bricht ab) und die Bereit-Regel (leer nur mit Anhang).
import test from 'node:test';
import assert from 'node:assert/strict';
import { feldTaste, speicherHinweis } from '../../web/bearbeiten.js';
import { nachrichtTaste } from '../../web/fokus.js';

const taste = (key, extra = {}) => ({ key, ctrlKey: false, altKey: false, metaKey: false, shiftKey: false, isComposing: false, keyCode: 0, ...extra });

test('feiner Zeiger: Enter speichert, Umschalt Enter macht eine Zeile', () => {
  assert.equal(feldTaste(taste('Enter'), 'fein'), 'speichern');
  assert.equal(feldTaste(taste('Enter', { shiftKey: true }), 'fein'), 'zeile');
});

test('grober Zeiger: Enter macht eine Zeile, Strg und Meta Enter speichern', () => {
  assert.equal(feldTaste(taste('Enter'), 'grob'), 'zeile');
  assert.equal(feldTaste(taste('Enter', { ctrlKey: true }), 'grob'), 'speichern');
  assert.equal(feldTaste(taste('Enter', { metaKey: true }), 'grob'), 'speichern');
  assert.equal(feldTaste(taste('Enter', { ctrlKey: true }), 'fein'), 'speichern');
});

test('Esc bricht ab; Alt Enter und Enter während einer Komposition tun nichts', () => {
  assert.equal(feldTaste(taste('Escape'), 'fein'), 'abbrechen');
  assert.equal(feldTaste(taste('Enter', { altKey: true }), 'fein'), null);
  assert.equal(feldTaste(taste('Enter', { isComposing: true }), 'fein'), null);
  assert.equal(feldTaste(taste('Enter', { keyCode: 229 }), 'fein'), null);
  assert.equal(feldTaste(taste('a'), 'fein'), null);
});

test('leer ohne Anhang: Hinweis „Leer geht nicht — zum Entfernen „Löschen“ wählen.“; mit Anhang darf leer gespeichert werden', () => {
  assert.equal(speicherHinweis('', 0), 'Leer geht nicht — zum Entfernen „Löschen“ wählen.');
  assert.equal(speicherHinweis('  \n ', 0), 'Leer geht nicht — zum Entfernen „Löschen“ wählen.');
  assert.equal(speicherHinweis('', 1), null);
  assert.equal(speicherHinweis('Hallo', 0), null);
});

test('Textregeln wie die Eingabe: zu lang und verbotene Zeichen sperren mit dem Hinweis der Eingabe', () => {
  assert.match(speicherHinweis('a'.repeat(16385), 0), /^⚠ Zu lang/);
  assert.match(speicherHinweis('a\u001bb', 0), /Steuerzeichen|U\+001B/);
});

test('Taste auf der fokussierten Nachricht: E bearbeitet, Entf löscht — nur ohne Strg, Alt und Meta', () => {
  assert.equal(nachrichtTaste(taste('e')), 'bearbeiten');
  assert.equal(nachrichtTaste(taste('E', { shiftKey: true })), 'bearbeiten');
  assert.equal(nachrichtTaste(taste('Delete')), 'loeschen');
  for (const umschalter of ['ctrlKey', 'altKey', 'metaKey']) {
    assert.equal(nachrichtTaste(taste('e', { [umschalter]: true })), null);
    assert.equal(nachrichtTaste(taste('Delete', { [umschalter]: true })), null);
  }
  assert.equal(nachrichtTaste(taste('r')), null);
});
