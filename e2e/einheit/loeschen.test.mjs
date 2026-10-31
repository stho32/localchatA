// Unit-Tests der Rückfrage vor dem Löschen (R00041 F0139 B0622, UX S1; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { rueckfrageText as rueckfrageTextRoh } from '../../web/loeschen.js';
import normalModul from '../normal.js';

const rueckfrageText = normalModul.normalisiert(rueckfrageTextRoh);

const ZEIT = '2026-10-30T14:09:12.345+01:00';
const nachricht = (extra) => ({ zeit: ZEIT, von: 'shoff@127.0.0.1', text: 'Bericht', zustand: null, geaendert: null, anhaenge: [], ...extra });
const anhang = (name, bytes, zustand = 'vorhanden') => ({ id: name, name, bytes, zustand });

test('ohne Anhänge: „Diese Nachricht löschen?“, darunter Uhrzeit und Textanfang, zuletzt „Das lässt sich nicht rückgängig machen.“', () => {
  assert.deepEqual(rueckfrageText(nachricht()), {
    frage: 'Diese Nachricht löschen?', auszug: '14:09: Bericht', warnung: 'Das lässt sich nicht rückgängig machen.',
  });
});

test('zwei Anhänge: Zahl und Summe ihrer Größe in Worten, Plural', () => {
  const mitZwei = nachricht({ anhaenge: [anhang('foto.png', 307200), anhang('bericht.pdf', 2457600)] });
  assert.equal(rueckfrageText(mitZwei).frage, 'Diese Nachricht mit 2 Anhängen (2,6 MiB) löschen? Die Anhänge werden mitgelöscht.');
});

test('ein Anhang: Singular', () => {
  assert.equal(rueckfrageText(nachricht({ anhaenge: [anhang('foto.png', 307200)] })).frage,
    'Diese Nachricht mit 1 Anhang (300 KiB) löschen? Der Anhang wird mitgelöscht.');
});

test('gelöschte Anhänge zählen nicht', () => {
  const einerGeloescht = nachricht({ anhaenge: [anhang('foto.png', 307200), anhang('bericht.pdf', 2457600, 'geloescht')] });
  assert.equal(rueckfrageText(einerGeloescht).frage, 'Diese Nachricht mit 1 Anhang (300 KiB) löschen? Der Anhang wird mitgelöscht.');
  const beideGeloescht = nachricht({ anhaenge: [anhang('foto.png', 307200, 'geloescht'), anhang('bericht.pdf', 2457600, 'geloescht')] });
  assert.equal(rueckfrageText(beideGeloescht).frage, 'Diese Nachricht löschen?');
});

test('Textanfang: die ersten 40 Zeichen und „…“; eine Zeile; ohne Text nur die Uhrzeit', () => {
  assert.equal(rueckfrageText(nachricht({ text: 'a'.repeat(45) })).auszug, `14:09: ${'a'.repeat(40)}…`);
  assert.equal(rueckfrageText(nachricht({ text: 'a'.repeat(40) })).auszug, `14:09: ${'a'.repeat(40)}`);
  assert.equal(rueckfrageText(nachricht({ text: 'Eins\nZwei' })).auszug, '14:09: Eins Zwei');
  assert.equal(rueckfrageText(nachricht({ text: '' })).auszug, '14:09');
});

// R00041 F0139 B0624: was die Antwort des Löschens bedeutet.
import { loeschenAuswerten } from '../../web/loeschen.js';

test('200 → gelöscht mit der Fassung der Antwort (auch „schon gelöscht“)', () => {
  const nachricht = { zeit: 'z', text: '', zustand: 'geloescht', geaendert: 'g', anhaenge: [] };
  assert.deepEqual(loeschenAuswerten({ art: 'antwort', status: 200, daten: { channel: 'a', nachricht, ergebnis: 'geloescht' } }), { art: 'geloescht', nachricht });
});

test('Abweisung → „⚠ <Text der Instanz>“; ohne Text mit Status', () => {
  assert.deepEqual(loeschenAuswerten({ art: 'antwort', status: 403, daten: { fehler: 'Nur eigene Nachrichten.' } }), { art: 'abgewiesen', text: '⚠ Nur eigene Nachrichten.' });
  assert.deepEqual(loeschenAuswerten({ art: 'antwort', status: 500, daten: null }), { art: 'abgewiesen', text: '⚠ Nicht gelöscht (Status 500).' });
});

test('keine Antwort → „⚠ Keine Antwort vom Server beim Löschen.“', () => {
  for (const art of ['netzfehler', 'zeitueberschreitung']) {
    assert.deepEqual(loeschenAuswerten({ art }), { art: 'abgewiesen', text: '⚠ Keine Antwort vom Server beim Löschen.' });
  }
});
