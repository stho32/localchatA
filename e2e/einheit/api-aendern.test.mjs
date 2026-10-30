// Unit-Tests der Aufrufe Bearbeiten und Löschen (R00041 F0135 B0611, F0139 B0624; node --test) mit einem Ersatz für
// fetch: Pfad, Körper, Sitzung, Wiederholschlüssel; Fehlerantworten sind Antworten, ein unerreichbarer Server ein
// Netzfehler, ohne Antwort nach 10 s eine Zeitüberschreitung.
import test from 'node:test';
import assert from 'node:assert/strict';
import { nachrichtBearbeiten, nachrichtLoeschen } from '../../web/api.js';

function antwortenMit(status, text) {
  const aufrufe = [];
  const holen = async (pfad, optionen) => {
    aufrufe.push({ pfad, optionen });
    return { status, text: async () => text };
  };
  return { holen, aufrufe };
}

test('Bearbeiten schickt {zeit, text} mit Sitzung und Idempotency-Key an …/nachrichten/bearbeiten', async () => {
  const { holen, aufrufe } = antwortenMit(200, '{"channel":"a-b","nachricht":{"zeit":"z","zustand":"bearbeitet"}}');

  const ergebnis = await nachrichtBearbeiten('a-b', 'z', 'Neu', 'k-1', { token: 't' }, holen);

  assert.deepEqual(ergebnis, { art: 'antwort', status: 200, daten: { channel: 'a-b', nachricht: { zeit: 'z', zustand: 'bearbeitet' } } });
  assert.equal(aufrufe[0].pfad, '/api/channels/a-b/nachrichten/bearbeiten');
  assert.equal(aufrufe[0].optionen.method, 'POST');
  assert.equal(aufrufe[0].optionen.headers['X-LocalchatA-Sitzung'], 't');
  assert.equal(aufrufe[0].optionen.headers['Content-Type'], 'application/json');
  assert.equal(aufrufe[0].optionen.headers['Idempotency-Key'], 'k-1');
  assert.deepEqual(JSON.parse(aufrufe[0].optionen.body), { zeit: 'z', text: 'Neu' });
});

test('Löschen schickt genau {zeit} mit Sitzung, ohne Wiederholschlüssel, an …/nachrichten/loeschen', async () => {
  const { holen, aufrufe } = antwortenMit(200, '{"ergebnis":"geloescht"}');

  const ergebnis = await nachrichtLoeschen('a-b', 'z', { token: 't' }, holen);

  assert.deepEqual(ergebnis, { art: 'antwort', status: 200, daten: { ergebnis: 'geloescht' } });
  assert.equal(aufrufe[0].pfad, '/api/channels/a-b/nachrichten/loeschen');
  assert.equal(aufrufe[0].optionen.headers['Idempotency-Key'], undefined);
  assert.deepEqual(JSON.parse(aufrufe[0].optionen.body), { zeit: 'z' });
});

test('eine Fehlerantwort ist eine Antwort, ein unerreichbarer Server ein Netzfehler', async () => {
  const abgewiesen = await nachrichtBearbeiten('a', 'z', 'x', 'k', { token: 't' }, antwortenMit(409, '{"fehler":"gelöscht"}').holen);
  const weg = await nachrichtLoeschen('a', 'z', { token: 't' }, async () => { throw new TypeError('Failed to fetch'); });

  assert.deepEqual(abgewiesen, { art: 'antwort', status: 409, daten: { fehler: 'gelöscht' } });
  assert.deepEqual(weg, { art: 'netzfehler' });
});

test('ohne Antwort: nach 10 s eine Zeitüberschreitung', async () => {
  let ausloesen = null;
  const zeitgeber = { stellen: (ms, rueckruf) => { ausloesen = { ms, rueckruf }; return 1; }, loeschen: () => {} };
  const nieAntworten = (pfad, optionen) => new Promise((_, gescheitert) => {
    optionen.signal.addEventListener('abort', () => gescheitert(new DOMException('abgebrochen', 'AbortError')));
  });
  const laufend = nachrichtLoeschen('a', 'z', { token: 't', zeitgeber }, nieAntworten);
  assert.equal(ausloesen.ms, 10000);
  ausloesen.rueckruf();
  assert.deepEqual(await laufend, { art: 'zeitueberschreitung' });
});
