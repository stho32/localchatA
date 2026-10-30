// Unit-Test der Tastenziele im Menü und in der Teilnehmerliste (R00050 F0178 B0785, V6; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { zielDerTaste } from '../../web/teilnehmermenue.js';

test('↑/↓ gehen zum Nachbarn und bleiben am Rand stehen, Home und Ende zum ersten und letzten, andere Tasten nicht', () => {
  const liste = ['ben', 'claude', 'cora'];
  assert.equal(zielDerTaste(liste, 'claude', 'ArrowDown'), 'cora');
  assert.equal(zielDerTaste(liste, 'claude', 'ArrowUp'), 'ben');
  assert.equal(zielDerTaste(liste, 'cora', 'ArrowDown'), 'cora');
  assert.equal(zielDerTaste(liste, 'ben', 'ArrowUp'), 'ben');
  assert.equal(zielDerTaste(liste, 'claude', 'Home'), 'ben');
  assert.equal(zielDerTaste(liste, 'claude', 'End'), 'cora');
  assert.equal(zielDerTaste(liste, 'claude', 'Enter'), null);
  assert.equal(zielDerTaste(liste, 'fremd', 'ArrowDown'), null);
});
