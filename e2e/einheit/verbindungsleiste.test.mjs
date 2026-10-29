// Unit-Tests der Leiste oben im Verlauf (R00012 F0047 „Meldung“, B0244; Designsystem VerlaufsMarken; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { wiederVerbundenText, leisteInhalt, warnungText, geltenderZeitpunkt, leisteTeile, leisteFuellen } from '../../web/verbindungsleiste.js';

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

// R00033 F0124 B0550 (ux-reviewer #36 S1): die Leiste besteht aus Anlass (kürzt bei Platzmangel mit „…“) und Stand (nie
// gekürzt); zusammen ergeben sie wörtlich den Text von R00025.
test('Teilung der Warnung in Anlass und Stand: der Stand ist „ · neuer Versuch in n s“ bzw. „ · wird verbunden …“', () => {
  const jetzt = 1_000_000;

  assert.deepEqual(leisteTeile(warnungText(jetzt + 4001, jetzt)), { anlass: '⚠ Verbindung verloren', stand: ' · neuer Versuch in 5 s' });
  assert.deepEqual(leisteTeile(warnungText(jetzt + 25000, jetzt)), { anlass: '⚠ Verbindung verloren', stand: ' · neuer Versuch in 10 s' });
  assert.deepEqual(leisteTeile(warnungText(jetzt, jetzt)), { anlass: '⚠ Verbindung verloren', stand: ' · wird verbunden …' });
  assert.deepEqual(leisteTeile(warnungText(null, jetzt)), { anlass: '⚠ Verbindung verloren', stand: '' });
  assert.deepEqual(leisteTeile('✓ Wieder verbunden · 3 nachgeladen'), { anlass: '✓ Wieder verbunden · 3 nachgeladen', stand: '' });
});

// Ein DOM-Ersatz genügt, um zu lesen, was die Leiste setzt.
class Knoten {
  constructor(tag) {
    this.tag = tag;
    this.kinder = [];
  }
  replaceChildren(...kinder) { this.kinder = kinder; }
  get textContent() { return this.kinder.map((k) => (typeof k === 'string' ? k : k.textContent)).join(''); }
  set textContent(wert) { this.kinder = [wert]; }
}

test('die Leiste setzt Anlass und Stand als zwei Teile, deren Text genau dem Text von R00025 entspricht', () => {
  const vorher = globalThis.document;
  globalThis.document = { createElement: (tag) => new Knoten(tag) };
  try {
    const jetzt = 1_000_000;
    for (const text of [warnungText(jetzt + 7000, jetzt), warnungText(jetzt, jetzt), warnungText(null, jetzt), '✓ Wieder verbunden']) {
      const leiste = new Knoten('div');

      leisteFuellen(leiste, text);

      assert.equal(leiste.textContent, text);
      assert.equal(leiste.kinder[0].className, 'lc-leiste-anlass');
    }
    const leiste = new Knoten('div');
    leisteFuellen(leiste, warnungText(1_007_000, 1_000_000));
    assert.deepEqual(leiste.kinder.map((k) => [k.className, k.textContent]), [
      ['lc-leiste-anlass', '⚠ Verbindung verloren'],
      ['lc-leiste-stand', ' · neuer Versuch in 7 s'],
    ]);
  } finally {
    globalThis.document = vorher;
  }
});
