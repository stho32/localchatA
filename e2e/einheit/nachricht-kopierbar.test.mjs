// Unit-Tests „Nachricht kopierbar“ (R00046 I0042 B0699, ADR-0052 Regel 1; node --test): kopieren darf man den Text jeder
// bestätigten, nicht gelöschten Nachricht mit Text — eigene, fremde, von Agenten —, und zwar je Aktion gebunden
// (anders als bearbeitbar, das nur die eigene Kennung zulässt, PO 145).
import test from 'node:test';
import assert from 'node:assert/strict';
import { kopierbar, bearbeitbar } from '../../web/nachricht.js';

const ICH = 'shoff@127.0.0.1';
const nachricht = (extra) => ({ zeit: '2026-10-30T14:00:00.000+01:00', von: ICH, art: 'mensch', text: 'a', zustand: null, geaendert: null, ...extra });

test('bestätigte Nachricht mit Text → kopierbar, auch bearbeitet', () => {
  assert.equal(kopierbar(nachricht()), true);
  assert.equal(kopierbar(nachricht({ zustand: 'bearbeitet', geaendert: '2026-10-30T14:09:00.000+01:00' })), true);
});

test('gelöscht → nicht kopierbar', () => {
  assert.equal(kopierbar(nachricht({ text: '', zustand: 'geloescht', geaendert: '2026-10-30T14:09:00.000+01:00' })), false);
  assert.equal(kopierbar(nachricht({ zustand: 'geloescht' })), false);
});

test('Text leer (nur Anhänge) → nicht kopierbar; Text „Bild“ mit Anhang → kopierbar', () => {
  assert.equal(kopierbar(nachricht({ text: '', anhaenge: [{ id: 'x' }] })), false);
  assert.equal(kopierbar(nachricht({ text: 'Bild', anhaenge: [{ id: 'x' }] })), true);
});

test('„wird gesendet …“ und fehlgeschlagen (ohne Zeit) → nicht kopierbar', () => {
  assert.equal(kopierbar({ von: ICH, art: 'mensch', text: 'Unterwegs' }), false);
  assert.equal(kopierbar({ von: ICH, art: 'mensch', text: 'Unterwegs', zustand: 'fehlgeschlagen' }), false);
});

test('fremde Kennung und Agent → kopierbar, dort wo bearbeitbar falsch ist', () => {
  const fremd = nachricht({ von: 'beta@127.0.0.2' });
  const agent = nachricht({ von: 'claude@127.0.0.3', art: 'agent' });
  assert.equal(kopierbar(fremd), true);
  assert.equal(kopierbar(agent), true);
  assert.equal(bearbeitbar(fremd, ICH), false);
  assert.equal(bearbeitbar(agent, ICH), false);
});
