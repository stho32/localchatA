// Unit-Tests der Frist im Browser (R00008 F0037, B0193, B0194; ADR-0033 Regel 3; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { fristPruefen, fristText, bestaetigungText, statusText, inzwischenText } from '../../web/frist.js';

const FEHLER = 'Frist: ganze Zahl von 1 bis 3650 Tagen.';

test('fristPruefen: 1, 3650, 7 und 007 sind gültig (führende Nullen erlaubt)', () => {
  assert.deepEqual(fristPruefen('1'), { gueltig: true, tage: 1 });
  assert.deepEqual(fristPruefen('3650'), { gueltig: true, tage: 3650 });
  assert.deepEqual(fristPruefen('7'), { gueltig: true, tage: 7 });
  assert.deepEqual(fristPruefen('007'), { gueltig: true, tage: 7 });
});

test('fristPruefen: 0, 3651 und alles außer Ziffern ist ungültig mit der Fehlerzeile', () => {
  for (const text of ['0', '3651', '-1', '7.5', '7,5', '1e3', '', ' 7', '7 ', '+7', '0000', '99999999999999999999']) {
    assert.deepEqual(fristPruefen(text), { gueltig: false, fehler: FEHLER }, JSON.stringify(text));
  }
});

test('fristText: „· 7 Tage“, Singular „· 1 Tag“, gesperrt bei null', () => {
  assert.equal(fristText(7), '· 7 Tage');
  assert.equal(fristText(1), '· 1 Tag');
  assert.equal(fristText(3650), '· 3.650 Tage');
  assert.equal(fristText(null), '· ⚠ Frist gesperrt');
});

test('bestaetigungText: Zahlen deutsch, Datum ausgeschrieben, Singular bei 1', () => {
  assert.equal(
    bestaetigungText(7, { tage: 23, nachrichten: 4210, bis: '2026-10-21' }),
    'Bei 7 Tagen entfallen sofort 23 Tage bis einschließlich 21. Oktober 2026 mit 4.210 Nachrichten. Das lässt sich nicht rückgängig machen.',
  );
  assert.equal(
    bestaetigungText(1, { tage: 1, nachrichten: 1, bis: '2026-10-27' }),
    'Bei 1 Tag entfallen sofort 1 Tag bis einschließlich 27. Oktober 2026 mit 1 Nachricht. Das lässt sich nicht rückgängig machen.',
  );
  assert.equal(
    bestaetigungText(7, { tage: 2, nachrichten: 5, bis: '2026-10-21' }),
    'Bei 7 Tagen entfallen sofort 2 Tage bis einschließlich 21. Oktober 2026 mit 5 Nachrichten. Das lässt sich nicht rückgängig machen.',
  );
  assert.equal(
    bestaetigungText(30, { tage: 2, nachrichten: 0, bis: '2026-01-05' }),
    'Bei 30 Tagen entfallen sofort 2 Tage bis einschließlich 5. Januar 2026 mit 0 Nachrichten. Das lässt sich nicht rückgängig machen.',
  );
});

test('statusText: ohne Entfallen nur die Frist, sonst mit Zahlen', () => {
  assert.equal(statusText('allgemein', 30, { tage: 0, nachrichten: 0 }), 'Frist für # allgemein: 30 Tage.');
  assert.equal(statusText('allgemein', 7, { tage: 2, nachrichten: 5 }), 'Frist für # allgemein: 7 Tage — 2 Tage mit 5 Nachrichten entfallen.');
  assert.equal(statusText('a', 1, { tage: 1, nachrichten: 1 }), 'Frist für # a: 1 Tag — 1 Tag mit 1 Nachricht entfallen.');
  assert.equal(statusText('a', 7, { tage: 23, nachrichten: 4210 }), 'Frist für # a: 7 Tage — 23 Tage mit 4.210 Nachrichten entfallen.');
});

test('inzwischenText: die fremde Änderung im offenen Dialog', () => {
  assert.equal(inzwischenText(31), 'Inzwischen geändert: 31 Tage.');
  assert.equal(inzwischenText(1), 'Inzwischen geändert: 1 Tag.');
  assert.equal(inzwischenText(null), 'Inzwischen geändert: Frist gesperrt.');
});
