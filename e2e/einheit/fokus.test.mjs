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
