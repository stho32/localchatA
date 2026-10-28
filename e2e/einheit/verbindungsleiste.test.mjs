// Unit-Tests der Leiste oben im Verlauf (R00012 F0047 „Meldung“, B0244; Designsystem VerlaufsMarken; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { wiederVerbundenText, leisteInhalt } from '../../web/verbindungsleiste.js';

test('Text nach dem Nachholen: Zahl mit Tausenderpunkt, ohne Zahl bei 0, „neueste geladen“ nach Neu-Öffnen', () => {
  assert.equal(wiederVerbundenText({ art: 'nachgeholt', zahl: 3 }), '✓ Wieder verbunden · 3 nachgeladen');
  assert.equal(wiederVerbundenText({ art: 'nachgeholt', zahl: 1205 }), '✓ Wieder verbunden · 1.205 nachgeladen');
  assert.equal(wiederVerbundenText({ art: 'nachgeholt', zahl: 0 }), '✓ Wieder verbunden');
  assert.equal(wiederVerbundenText({ art: 'neu' }), '✓ Wieder verbunden · neueste geladen');
});

test('die Meldung steht 4 s: bei 3 999 ms sichtbar, bei 4 000 ms weg', () => {
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
