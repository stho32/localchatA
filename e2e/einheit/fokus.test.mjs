// Unit-Tests der Tasten, die den Fokus ins Feld holen (R00012 F0048 „Fokus“, B0250; ADR-0038 Regel 8; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { fokusTaste } from '../../web/fokus.js';

const taste = (key, extra = {}) => ({ key, ctrlKey: false, altKey: false, metaKey: false, ...extra });

test('/ außerhalb von Textfeldern holt den Fokus ins Feld, im Textfeld wird es getippt', () => {
  assert.equal(fokusTaste(taste('/'), 'sonst'), 'ins-feld');
  assert.equal(fokusTaste(taste('/'), 'verlauf'), 'ins-feld');
  assert.equal(fokusTaste(taste('/'), 'textfeld'), null);
});

test('Esc im Verlauf holt den Fokus ins Feld, anderswo nicht', () => {
  assert.equal(fokusTaste(taste('Escape'), 'verlauf'), 'ins-feld');
  assert.equal(fokusTaste(taste('Escape'), 'sonst'), null);
  assert.equal(fokusTaste(taste('Escape'), 'textfeld'), null);
});

test('mit Strg, Alt oder Meta und in einem offenen Dialog bleibt alles, wie es ist', () => {
  assert.equal(fokusTaste(taste('/', { ctrlKey: true }), 'sonst'), null);
  assert.equal(fokusTaste(taste('/', { altKey: true }), 'sonst'), null);
  assert.equal(fokusTaste(taste('/', { metaKey: true }), 'sonst'), null);
  assert.equal(fokusTaste(taste('/'), 'dialog'), null);
  assert.equal(fokusTaste(taste('Escape'), 'dialog'), null);
  assert.equal(fokusTaste(taste('a'), 'sonst'), null);
});

// Rollfokus im Verlauf (R00041 F0141, B0631): ↑/↓ ohne Umschalter wechseln die Nachricht, genau eine Nachricht ist Tab-Stopp.
import { pfeilRichtung, pfeilZiel, tabStopp } from '../../web/fokus.js';

test('↑ und ↓ ohne Umschalter geben die Richtung, jede Taste mit Strg, Alt, Meta oder Umschalt keine', () => {
  assert.equal(pfeilRichtung(taste('ArrowUp', { shiftKey: false })), 'zurueck');
  assert.equal(pfeilRichtung(taste('ArrowDown', { shiftKey: false })), 'vor');
  assert.equal(pfeilRichtung(taste('ArrowUp', { shiftKey: true })), null);
  assert.equal(pfeilRichtung(taste('ArrowDown', { ctrlKey: true, shiftKey: false })), null);
  assert.equal(pfeilRichtung(taste('ArrowUp', { altKey: true, shiftKey: false })), null);
  assert.equal(pfeilRichtung(taste('ArrowDown', { metaKey: true, shiftKey: false })), null);
  assert.equal(pfeilRichtung(taste('ArrowLeft', { shiftKey: false })), null);
  assert.equal(pfeilRichtung(taste('a', { shiftKey: false })), null);
});

test('das Ziel ist die vorige oder nächste Stelle, am Rand und ohne Nachrichten keines', () => {
  assert.equal(pfeilZiel('zurueck', 5, 10), 4);
  assert.equal(pfeilZiel('vor', 5, 10), 6);
  assert.equal(pfeilZiel('zurueck', 0, 10), null);
  assert.equal(pfeilZiel('vor', 9, 10), null);
  assert.equal(pfeilZiel('vor', 0, 1), null);
  assert.equal(pfeilZiel(null, 5, 10), null);
});

test('Tab-Stopp ist die gemerkte Nachricht, sonst die neueste, ohne Nachrichten keiner', () => {
  const zeiten = ['a', 'b', 'c'];
  assert.equal(tabStopp(zeiten, 'b'), 1);
  assert.equal(tabStopp(zeiten, null), 2);
  assert.equal(tabStopp(zeiten, 'weg'), 2);
  assert.equal(tabStopp([], 'b'), null);
  assert.equal(tabStopp([], null), null);
});
