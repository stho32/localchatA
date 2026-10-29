// Unit-Tests der Leiste oben im Verlauf (R00012 F0047 „Meldung“, B0244; Designsystem VerlaufsMarken; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { wiederVerbundenText, leisteInhalt, warnungText, geltenderZeitpunkt } from '../../web/verbindungsleiste.js';

test('Text nach dem Nachholen: Zahl mit Tausenderpunkt, ohne Zahl bei 0, „neueste geladen“ nach Neu-Öffnen', () => {
  assert.equal(wiederVerbundenText({ art: 'nachgeholt', zahl: 3 }), '✓ Wieder verbunden · 3 nachgeladen');
  assert.equal(wiederVerbundenText({ art: 'nachgeholt', zahl: 1205 }), '✓ Wieder verbunden · 1.205 nachgeladen');
  assert.equal(wiederVerbundenText({ art: 'nachgeholt', zahl: 0 }), '✓ Wieder verbunden');
  assert.equal(wiederVerbundenText({ art: 'neu' }), '✓ Wieder verbunden · neueste geladen');
});

test('die Meldung steht 4 s: bei 3.999 ms sichtbar, bei 4.000 ms weg', () => {
  const zustand = { getrennt: false, meldung: '✓ Wieder verbunden · 3 nachgeladen', meldungSeitMs: 1000 };

  assert.deepEqual(leisteInhalt(zustand, 1000 + 3999), { text: '✓ Wieder verbunden · 3 nachgeladen', fehler: false });
  assert.equal(leisteInhalt(zustand, 1000 + 4000), null);
});

test('getrennt zeigt „Verbindung verloren“ als Fehler, auch über einer alten Meldung; sonst ohne Meldung nichts', () => {
  assert.deepEqual(
    leisteInhalt({ getrennt: true, meldung: '✓ Wieder verbunden', meldungSeitMs: 0 }, 10),
    { text: '⚠ Verbindung verloren', fehler: true },
  );
  assert.equal(leisteInhalt({ getrennt: false, meldung: null, meldungSeitMs: 0 }, 10), null);
});

// R00025 F0084 „Countdown-Text“ (B0396): Rest bis naechsterVersuchUm in ganzen Sekunden, aufgerundet, begrenzt auf
// 0–10 s; erreicht → „wird verbunden …“; ohne Zeitpunkt nur „⚠ Verbindung verloren“ (Mutanten „abgerundet“, „ohne
// Begrenzung“ und „ohne Zeitpunkt wie wird verbunden“ werden je rot).
test('Countdown-Text aus dem Zeitpunkt des nächsten Versuchs, gestellte Zeit', () => {
  const jetzt = 1_000_000;
  const mitRest = (restMs) => warnungText(jetzt + restMs, jetzt);

  assert.equal(mitRest(5000), '⚠ Verbindung verloren · neuer Versuch in 5 s');
  assert.equal(mitRest(4001), '⚠ Verbindung verloren · neuer Versuch in 5 s');
  assert.equal(mitRest(4000), '⚠ Verbindung verloren · neuer Versuch in 4 s');
  assert.equal(mitRest(1), '⚠ Verbindung verloren · neuer Versuch in 1 s');
  assert.equal(mitRest(0), '⚠ Verbindung verloren · wird verbunden …');
  assert.equal(mitRest(-300), '⚠ Verbindung verloren · wird verbunden …');
  assert.equal(mitRest(25000), '⚠ Verbindung verloren · neuer Versuch in 10 s');
  assert.equal(warnungText(null, jetzt), '⚠ Verbindung verloren');
});

test('getrennt mit Zeitpunkt zeigt den Countdown als Fehler', () => {
  const zustand = { getrennt: true, naechsterVersuchUm: 7000, meldung: null, meldungSeitMs: 0 };

  assert.deepEqual(leisteInhalt(zustand, 5000), { text: '⚠ Verbindung verloren · neuer Versuch in 2 s', fehler: true });
});

// R00025 F0084 „Geltender Zeitpunkt“ (B0397, ADR-0015 Ergänzung Karte #36 Regel 5): Strom unterbrochen → der des
// Stroms; Strom verbunden, Nachholen gescheitert → der des nächsten Nachhol-Versuchs; Netz des Tabs aus ohne
// `unterbrochen` → keiner (Mutante „Quelle immer der Strom“ wird rot).
test('geltender Zeitpunkt nach Quelle', () => {
  assert.equal(geltenderZeitpunkt({ stromUnterbrochen: true, strom: 7000, nachholen: 3000 }), 7000);
  assert.equal(geltenderZeitpunkt({ stromUnterbrochen: false, strom: 7000, nachholen: 3000 }), 3000);
  assert.equal(geltenderZeitpunkt({ stromUnterbrochen: false, strom: null, nachholen: null }), null);
});
