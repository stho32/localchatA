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
import { pfeilRichtung, pfeilZiel, tabStopp, zuEigenerLetzten, eigeneLetzte } from '../../web/fokus.js';

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

// ↑ in der leeren Eingabe (R00041 F0141, B0633)
test('↑ allein in der leeren Eingabe führt zur eigenen letzten Nachricht, sonst nicht', () => {
  assert.equal(zuEigenerLetzten(taste('ArrowUp'), ''), true);
  assert.equal(zuEigenerLetzten(taste('ArrowUp'), 'a'), false, 'mit Text bewegt ↑ die Schreibmarke');
  assert.equal(zuEigenerLetzten(taste('ArrowUp'), ' '), false, 'auch Leerraum ist Text');
  assert.equal(zuEigenerLetzten(taste('ArrowDown'), ''), false);
  assert.equal(zuEigenerLetzten(taste('ArrowUp', { shiftKey: true }), ''), false);
  assert.equal(zuEigenerLetzten(taste('ArrowUp', { ctrlKey: true }), ''), false);
  assert.equal(zuEigenerLetzten(taste('ArrowUp', { altKey: true }), ''), false, 'Alt ↑ wechselt den Channel');
  assert.equal(zuEigenerLetzten(taste('ArrowUp', { metaKey: true }), ''), false);
  assert.equal(zuEigenerLetzten(taste('ArrowUp', { isComposing: true }), ''), false, 'IME-Komposition');
  assert.equal(zuEigenerLetzten(taste('ArrowUp', { keyCode: 229 }), ''), false);
});

const satz = (von, zustand) => ({ von, zustand });
const ICH = 'shoff@127.0.0.1';
const BETA = 'beta@127.0.0.2';

test('die eigene letzte ist die jüngste mit meiner Kennung, nicht die jüngste überhaupt', () => {
  const liste = [satz(ICH), satz(ICH), satz(BETA)];
  assert.equal(eigeneLetzte(liste, ICH), 1);
});

test('eine gelöschte eigene wird übersprungen, eine bearbeitete zählt', () => {
  assert.equal(eigeneLetzte([satz(ICH), satz(ICH, 'geloescht'), satz(BETA)], ICH), 0);
  assert.equal(eigeneLetzte([satz(ICH), satz(ICH, 'bearbeitet'), satz(BETA)], ICH), 1);
});

test('gesucht wird nur in den neuesten 100: eine eigene dahinter zählt nicht, die 100. von hinten noch', () => {
  const hinten = (anzahl) => Array.from({ length: anzahl }, () => satz(BETA));
  assert.equal(eigeneLetzte([satz(ICH), ...hinten(100)], ICH), null);
  assert.equal(eigeneLetzte([satz(ICH), ...hinten(99)], ICH), 0);
  assert.equal(eigeneLetzte([satz(ICH), ...hinten(299)], ICH, 300), 0);
});

test('ohne eigene Nachricht keine; eine ähnliche Kennung ist nicht meine', () => {
  assert.equal(eigeneLetzte([satz(BETA), satz(BETA)], ICH), null);
  assert.equal(eigeneLetzte([satz('shoff@127.0.0.2')], ICH), null);
  assert.equal(eigeneLetzte([satz(ICH, 'geloescht')], ICH), null);
  assert.equal(eigeneLetzte([], ICH), null);
});

// Taste C auf der fokussierten Nachricht (R00046 I0042 B0702, ADR-0052 Regel C1): nur Abkürzung, ausgewertet wird `key`.
import { kopierTaste } from '../../web/fokus.js';

const LAGE = { zielIstNachricht: true, markierungLeer: true, dialogOffen: false };
const cTaste = (extra = {}) => ({ key: 'c', code: 'KeyC', ctrlKey: false, altKey: false, metaKey: false, shiftKey: false, repeat: false, isComposing: false, ...extra });

test('c und C bei Feststelltaste (ohne Umschalt) auf der Nachricht → kopieren', () => {
  assert.equal(kopierTaste(cTaste(), LAGE), true);
  assert.equal(kopierTaste(cTaste({ key: 'C', shiftKey: false }), LAGE), true);
});

test('Strg, Alt, Meta oder Umschalt dazu → nicht (Strg+C bleibt dem Browser)', () => {
  assert.equal(kopierTaste(cTaste({ ctrlKey: true }), LAGE), false);
  assert.equal(kopierTaste(cTaste({ altKey: true }), LAGE), false);
  assert.equal(kopierTaste(cTaste({ metaKey: true }), LAGE), false);
  assert.equal(kopierTaste(cTaste({ key: 'C', shiftKey: true }), LAGE), false);
});

test('Wiederholung durch Halten und Eingabemethode → nicht', () => {
  assert.equal(kopierTaste(cTaste({ repeat: true }), LAGE), false);
  assert.equal(kopierTaste(cTaste({ isComposing: true }), LAGE), false);
});

test('ausgewertet wird key, nicht code: key „c“ an anderer Stelle der Tastatur kopiert, key „j“ auf KeyC nicht', () => {
  assert.equal(kopierTaste(cTaste({ key: 'c', code: 'KeyJ' }), LAGE), true);
  assert.equal(kopierTaste(cTaste({ key: 'j', code: 'KeyC' }), LAGE), false);
  assert.equal(kopierTaste(cTaste({ key: 'e', code: 'KeyC' }), LAGE), false);
});

test('Ziel nicht die Nachricht selbst (Eingabefeld, Bearbeiten-Feld, Knopf, Zitat) → nicht', () => {
  assert.equal(kopierTaste(cTaste(), { ...LAGE, zielIstNachricht: false }), false);
});

test('markierter Text → nicht (es gilt das gewohnte Kopieren des Browsers)', () => {
  assert.equal(kopierTaste(cTaste(), { ...LAGE, markierungLeer: false }), false);
});

test('offener Dialog → nicht', () => {
  assert.equal(kopierTaste(cTaste(), { ...LAGE, dialogOffen: true }), false);
});
