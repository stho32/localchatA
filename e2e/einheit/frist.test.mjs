// Unit-Tests der Frist im Browser (R00008 F0037, B0193, B0194; ADR-0033 Regel 3; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  fristPruefen, fristText, bestaetigungText, statusText, inzwischenText, inzwischenZurueckText, fehlerDerAntwort,
  wiederMoeglichAb, versuchZaehlen, mitVersuch,
} from '../../web/frist.js';

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

test('fristPruefen: Leerraum um oder in der Zahl ist ungültig (B0389: das Feld liefert den Rohtext)', () => {
  for (const text of [' 30', '30 ', '3 0', '30\t']) {
    assert.deepEqual(fristPruefen(text), { gueltig: false, fehler: FEHLER }, JSON.stringify(text));
  }
  assert.deepEqual(fristPruefen('30'), { gueltig: true, tage: 30 });
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
    'Bei 1 Tag entfällt sofort 1 Tag bis einschließlich 27. Oktober 2026 mit 1 Nachricht. Das lässt sich nicht rückgängig machen.',
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

// Abnahme-Probe Rechtschreib-Review #12, Muss 1 (wörtlich): das Verb folgt der Zahl der entfallenden Tage, nie der Frist.
test('bestaetigungText: „entfällt“ bei genau 1 entfallenden Tag, „entfallen“ ab 2 — gleich, wie lang die Frist ist', () => {
  assert.equal(
    bestaetigungText(7, { tage: 1, nachrichten: 1, bis: '2026-10-21' }),
    'Bei 7 Tagen entfällt sofort 1 Tag bis einschließlich 21. Oktober 2026 mit 1 Nachricht. Das lässt sich nicht rückgängig machen.',
  );
  assert.equal(
    bestaetigungText(1, { tage: 1, nachrichten: 3, bis: '2026-10-27' }),
    'Bei 1 Tag entfällt sofort 1 Tag bis einschließlich 27. Oktober 2026 mit 3 Nachrichten. Das lässt sich nicht rückgängig machen.',
  );
  assert.equal(
    bestaetigungText(1, { tage: 2, nachrichten: 1, bis: '2026-10-27' }),
    'Bei 1 Tag entfallen sofort 2 Tage bis einschließlich 27. Oktober 2026 mit 1 Nachricht. Das lässt sich nicht rückgängig machen.',
  );
  assert.equal(
    bestaetigungText(7, { tage: 2, nachrichten: 4, bis: '2026-10-21' }),
    'Bei 7 Tagen entfallen sofort 2 Tage bis einschließlich 21. Oktober 2026 mit 4 Nachrichten. Das lässt sich nicht rückgängig machen.',
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

// R00019 F0083 „Zweiter Schritt folgt fremder Änderung“: nach dem Rücksprung sagt der Hinweis, wie man erfährt, was
// jetzt entfiele.
test('inzwischenZurueckText: der Rücksprung aus dem zweiten Schritt verweist aufs erneute Speichern', () => {
  assert.equal(inzwischenZurueckText(7), 'Inzwischen geändert: 7 Tage — erneut speichern, um zu sehen, was entfällt.');
  assert.equal(inzwischenZurueckText(1), 'Inzwischen geändert: 1 Tag — erneut speichern, um zu sehen, was entfällt.');
  assert.equal(inzwischenZurueckText(null), 'Inzwischen geändert: Frist gesperrt.');
});

// R00019 F0066 „Fehlerzeile unterscheidet Eingabe und Server“ (ADR-0033 Ergänzung Karte #27, Regel 1): nur die 422
// `frist` markiert die Eingabe; der Text ist der deutsche `fehler`, sonst ein eigener Satz mit Ausweg.
test('fehlerDerAntwort: nur die 422 frist ist ein Prüffehler, jede andere Antwort lässt das Feld unmarkiert', () => {
  const antwort = (status, daten) => ({ serverWeg: false, status, daten });
  assert.deepEqual(
    fehlerDerAntwort(antwort(422, { regel: 'frist', fehler: 'Die Frist ist eine ganze Zahl.' })),
    { text: 'Die Frist ist eine ganze Zahl.', schluessel: 'Die Frist ist eine ganze Zahl.', eingabeFalsch: true },
  );
  for (const [status, daten] of [
    [500, { fehler: 'Frist nicht gesetzt — x.' }],
    [404, { fehler: '# x gibt es nicht.' }],
    [409, { regel: 'einstellungen-defekt', fehler: 'kaputt.' }],
    [422, { regel: 'bestaetigt', fehler: 'bestaetigt falsch.' }],
  ]) {
    assert.deepEqual(fehlerDerAntwort(antwort(status, daten)), { text: daten.fehler, schluessel: daten.fehler, eingabeFalsch: false }, String(status));
  }
});

test('fehlerDerAntwort: ohne fehler „Nicht gespeichert (Status n) — später erneut versuchen.“, ohne Antwort „Verbindung prüfen“', () => {
  assert.deepEqual(
    fehlerDerAntwort({ serverWeg: false, status: 502, daten: null }),
    { text: 'Nicht gespeichert (Status 502) — später erneut versuchen.', schluessel: 'Nicht gespeichert (Status 502) — später erneut versuchen.', eingabeFalsch: false },
  );
  assert.deepEqual(
    fehlerDerAntwort({ serverWeg: true, status: 0, daten: null }),
    { text: 'Nicht gespeichert — Verbindung prüfen.', schluessel: 'Nicht gespeichert — Verbindung prüfen.', eingabeFalsch: false },
  );
});

// R00019 F0066 „429 im Dialog“, Rechenbeispiele (ADR-0033 Ergänzung Karte #27, Regel 3): Eintreffen + wieder_in_ms,
// auf die volle Sekunde aufgerundet, Ortszeit, 24 h, über Mitternacht ohne Datum.
test('wiederMoeglichAb: Eintreffen plus Wartezeit, aufgerundet auf die volle Sekunde', () => {
  const eintreffen = new Date(2026, 9, 28, 14, 3, 12, 400).getTime();
  assert.equal(wiederMoeglichAb(eintreffen, 9600), '14:03:22');
  assert.equal(wiederMoeglichAb(eintreffen, 10000), '14:03:23');
  assert.equal(wiederMoeglichAb(new Date(2026, 9, 28, 23, 59, 58, 0).getTime(), 3000), '00:00:01');
  assert.equal(wiederMoeglichAb(new Date(2026, 9, 28, 9, 5, 7, 0).getTime(), 0), '09:05:07');
});

test('fehlerDerAntwort: die 429 nimmt den Teil vor „ — “ und setzt die Uhrzeit statt der Sekunden, Feld unmarkiert', () => {
  const eintreffen = new Date(2026, 9, 28, 14, 3, 12, 400).getTime();
  const antwort = {
    serverWeg: false,
    status: 429,
    daten: { fehler: 'Zu viele Änderungen von abcdefghijklm@127.0.0.1 — wieder möglich in 10 s.', grenze: 'kennung', wieder_in_ms: 9600 },
  };
  assert.deepEqual(
    fehlerDerAntwort(antwort, eintreffen),
    {
      text: 'Zu viele Änderungen von abcdefghijklm@127.0.0.1 — wieder möglich ab 14:03:22.',
      schluessel: 'Zu viele Änderungen von abcdefghijklm@127.0.0.1',
      eingabeFalsch: false,
    },
  );
});

// R00019 F0066 „Wiederholter Fehlversuch“ (ADR-0033 Ergänzung Karte #27, Regel 2): gleicher Schlüssel zählt weiter,
// ein anderer oder eine weggeräumte Zeile (vorher null) beginnt bei 1; die 429 zählt ohne die Uhrzeit.
test('versuchZaehlen und mitVersuch: S, S, S → ohne Zusatz, „(2. Versuch)“, „(3. Versuch)“; anderer Schlüssel und weggeräumt beginnen neu', () => {
  const erster = versuchZaehlen(null, 'S');
  const zweiter = versuchZaehlen(erster, 'S');
  const dritter = versuchZaehlen(zweiter, 'S');
  assert.equal(mitVersuch('S', erster.versuch), 'S');
  assert.equal(mitVersuch('S', zweiter.versuch), 'S (2. Versuch)');
  assert.equal(mitVersuch('S', dritter.versuch), 'S (3. Versuch)');
  assert.equal(versuchZaehlen(dritter, 'T').versuch, 1);
  assert.equal(versuchZaehlen(null, 'S').versuch, 1);
});

test('versuchZaehlen: zwei 429 mit verschiedener Uhrzeit zählen als zweiter Versuch', () => {
  const antwort = (wartezeit) => ({
    serverWeg: false,
    status: 429,
    daten: { fehler: 'Zu viele Änderungen von a@127.0.0.1 — wieder möglich in 10 s.', wieder_in_ms: wartezeit },
  });
  const eintreffen = new Date(2026, 9, 28, 14, 3, 12, 400).getTime();
  const erste = fehlerDerAntwort(antwort(9600), eintreffen);
  const zweite = fehlerDerAntwort(antwort(9600), eintreffen + 1000);
  assert.notEqual(erste.text, zweite.text);
  assert.equal(versuchZaehlen(versuchZaehlen(null, erste.schluessel), zweite.schluessel).versuch, 2);
});
