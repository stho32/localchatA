// Unit-Tests des Bearbeiten-Felds (R00041 F0135 B0610; node --test): Tasten wie in der Eingabe (Enter speichert, auf
// Touch-Geräten neue Zeile, Strg Enter speichert überall, Esc bricht ab) und die Bereit-Regel (leer nur mit Anhang).
import test from 'node:test';
import assert from 'node:assert/strict';
import { feldTaste, feldHoehe, speicherHinweis, zweitesBearbeitenHinweis } from '../../web/bearbeiten.js';
import { direktgespraech } from '../../web/gespraech.js';
import { nachrichtTaste } from '../../web/fokus.js';

const taste = (key, extra = {}) => ({ key, ctrlKey: false, altKey: false, metaKey: false, shiftKey: false, isComposing: false, keyCode: 0, ...extra });

test('feiner Zeiger: Enter speichert, Umschalt Enter macht eine Zeile', () => {
  assert.equal(feldTaste(taste('Enter'), 'fein'), 'speichern');
  assert.equal(feldTaste(taste('Enter', { shiftKey: true }), 'fein'), 'zeile');
});

test('grober Zeiger: Enter macht eine Zeile, Strg und Meta Enter speichern', () => {
  assert.equal(feldTaste(taste('Enter'), 'grob'), 'zeile');
  assert.equal(feldTaste(taste('Enter', { ctrlKey: true }), 'grob'), 'speichern');
  assert.equal(feldTaste(taste('Enter', { metaKey: true }), 'grob'), 'speichern');
  assert.equal(feldTaste(taste('Enter', { ctrlKey: true }), 'fein'), 'speichern');
});

test('Esc bricht ab; Alt Enter und Enter während einer Komposition tun nichts', () => {
  assert.equal(feldTaste(taste('Escape'), 'fein'), 'abbrechen');
  assert.equal(feldTaste(taste('Enter', { altKey: true }), 'fein'), null);
  assert.equal(feldTaste(taste('Enter', { isComposing: true }), 'fein'), null);
  assert.equal(feldTaste(taste('Enter', { keyCode: 229 }), 'fein'), null);
  assert.equal(feldTaste(taste('a'), 'fein'), null);
});

test('leer ohne Anhang: Hinweis „Leer geht nicht — zum Entfernen ‚Löschen‘ wählen.“; mit Anhang darf leer gespeichert werden', () => {
  assert.equal(speicherHinweis('', 0), 'Leer geht nicht — zum Entfernen ‚Löschen‘ wählen.');
  assert.equal(speicherHinweis('  \n ', 0), 'Leer geht nicht — zum Entfernen ‚Löschen‘ wählen.');
  assert.equal(speicherHinweis('', 1), null);
  assert.equal(speicherHinweis('Hallo', 0), null);
});

test('Textregeln wie die Eingabe: zu lang und verbotene Zeichen sperren mit dem Hinweis der Eingabe', () => {
  assert.match(speicherHinweis('a'.repeat(16385), 0), /^⚠ Zu lang/);
  assert.match(speicherHinweis('a\u001bb', 0), /Steuerzeichen|U\+001B/);
});

test('Taste auf der fokussierten Nachricht: E bearbeitet, Entf löscht — nur ohne Strg, Alt und Meta', () => {
  assert.equal(nachrichtTaste(taste('e')), 'bearbeiten');
  assert.equal(nachrichtTaste(taste('E', { shiftKey: true })), 'bearbeiten');
  assert.equal(nachrichtTaste(taste('Delete')), 'loeschen');
  for (const umschalter of ['ctrlKey', 'altKey', 'metaKey']) {
    assert.equal(nachrichtTaste(taste('e', { [umschalter]: true })), null);
    assert.equal(nachrichtTaste(taste('Delete', { [umschalter]: true })), null);
  }
  assert.equal(nachrichtTaste(taste('r')), null);
});

// R00041 F0135 B0611: was die Antwort des Speicherns bedeutet.
import { speichernAuswerten } from '../../web/bearbeiten.js';

const VORHER = '2026-10-30T14:05:00.000+01:00';
const NACHHER = '2026-10-30T14:09:00.000+01:00';
const antwort = (status, daten) => ({ art: 'antwort', status, daten });

test('200 mit neuem geaendert → gespeichert mit der neuen Fassung; mit gleichem → unverändert', () => {
  const nachricht = { zeit: 'z', text: 'Neu', zustand: 'bearbeitet', geaendert: NACHHER };
  assert.deepEqual(speichernAuswerten(antwort(200, { channel: 'a', nachricht }), VORHER), { art: 'gespeichert', nachricht });
  const gleich = { zeit: 'z', text: 'Alt', zustand: 'bearbeitet', geaendert: VORHER };
  assert.deepEqual(speichernAuswerten(antwort(200, { channel: 'a', nachricht: gleich }), VORHER), { art: 'unveraendert', nachricht: gleich });
  const nieGeaendert = { zeit: 'z', text: 'Alt', zustand: null, geaendert: null };
  assert.deepEqual(speichernAuswerten(antwort(200, { channel: 'a', nachricht: nieGeaendert }), null), { art: 'unveraendert', nachricht: nieGeaendert });
});

test('409 → inzwischen gelöscht', () => {
  assert.deepEqual(speichernAuswerten(antwort(409, { fehler: 'Die Nachricht ist gelöscht.', regel: 'geloescht' }), null), { art: 'geloescht' });
});

test('Prüffehler 422 → Text der Instanz mit ⚠ und aria-invalid; 507 und andere ohne', () => {
  assert.deepEqual(speichernAuswerten(antwort(422, { fehler: 'Zu lang.' }), null), { art: 'abgewiesen', text: '⚠ Zu lang.', pruefFehler: true });
  assert.deepEqual(speichernAuswerten(antwort(507, { fehler: 'Platte voll.' }), null), { art: 'abgewiesen', text: '⚠ Platte voll.', pruefFehler: false });
  assert.deepEqual(speichernAuswerten(antwort(500, null), null), { art: 'abgewiesen', text: '⚠ Nicht gespeichert (Status 500).', pruefFehler: false });
});

test('keine Antwort (Netzfehler, Zeitgrenze) → „⚠ Nicht gespeichert: keine Antwort vom Server.“', () => {
  for (const art of ['netzfehler', 'zeitueberschreitung']) {
    assert.deepEqual(speichernAuswerten({ art }, null), { art: 'abgewiesen', text: '⚠ Nicht gespeichert: keine Antwort vom Server.', pruefFehler: false });
  }
});

const ZEIT = '2026-10-28T14:07:30.000+01:00';

test('zweites Bearbeiten im selben Channel: der Hinweis nennt die Nachricht von 14:07 (B0801)', () => {
  assert.equal(
    zweitesBearbeitenHinweis({ feldChannel: 'allgemein', offenerChannel: 'allgemein', zeit: ZEIT }),
    'Erst speichern oder abbrechen: offenes Feld bei der Nachricht von 14:07.',
  );
});

test('zweites Bearbeiten in einem anderen Channel: der Hinweis nennt den Ort des Felds und die Uhrzeit (B0801)', () => {
  assert.equal(
    zweitesBearbeitenHinweis({ feldChannel: 'technik', offenerChannel: 'allgemein', zeit: ZEIT }),
    'Erst speichern oder abbrechen: offenes Feld in # technik, Nachricht von 14:07.',
  );
});

test('Channelnamen über 16 Zeichen werden mit „…“ auf 16 gekürzt, 16 Zeichen bleiben ganz (B0801)', () => {
  assert.equal(
    zweitesBearbeitenHinweis({ feldChannel: 'abcdefghijklmnop', offenerChannel: 'allgemein', zeit: ZEIT }),
    'Erst speichern oder abbrechen: offenes Feld in # abcdefghijklmnop, Nachricht von 14:07.',
  );
  assert.equal(
    zweitesBearbeitenHinweis({ feldChannel: 'abcdefghijklmnopq', offenerChannel: 'allgemein', zeit: ZEIT }),
    'Erst speichern oder abbrechen: offenes Feld in # abcdefghijklmno…, Nachricht von 14:07.',
  );
});

test('Feld in einem Direktgespräch, zweites Bearbeiten im Channel: der Hinweis nennt das Gespräch (B0801)', () => {
  assert.equal(
    zweitesBearbeitenHinweis({ feldChannel: direktgespraech('ben@127.0.0.3'), offenerChannel: 'allgemein', zeit: ZEIT }),
    'Erst speichern oder abbrechen: offenes Feld in der Direktnachricht mit ben@127.0.0.3, Nachricht von 14:07.',
  );
});

// code-reviewer #85 Slice B Sollte 1: die Höhe des Bearbeiten-Felds als reine Rechnung (B0803, B0804).
test('feldHoehe: genug Platz → die gewünschte Höhe; zu wenig → Platz minus Rahmen; Platz unter dem Minimum → das Minimum', () => {
  assert.equal(feldHoehe({ gewuenscht: 138, platz: 400, rahmenOhneFeld: 90, mindestens: 40 }), 138);
  assert.equal(feldHoehe({ gewuenscht: 138, platz: 200, rahmenOhneFeld: 90, mindestens: 40 }), 110);
  assert.equal(feldHoehe({ gewuenscht: 138, platz: 100, rahmenOhneFeld: 90, mindestens: 40 }), 40);
  assert.equal(feldHoehe({ gewuenscht: 138, platz: 228, rahmenOhneFeld: 90, mindestens: 40 }), 138, 'genau passend bleibt unverändert');
});
