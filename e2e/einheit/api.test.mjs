// Unit-Tests des API-Aufrufs der Oberfläche (R00003 F0010, node --test) mit einem Ersatz für fetch.
import test from 'node:test';
import assert from 'node:assert/strict';
import { rufen, nachrichtSenden } from '../../web/api.js';

function antwortenMit(status, text) {
  const aufrufe = [];
  const holen = async (pfad, optionen) => {
    aufrufe.push({ pfad, optionen });
    return { status, text: async () => text };
  };
  return { holen, aufrufe };
}

test('GET mit Token trägt den Sitzungs-Header und liefert Status und Daten', async () => {
  const { holen, aufrufe } = antwortenMit(200, '{"kennung":"shoff@127.0.0.1"}');

  const ergebnis = await rufen('/api/ich', { token: 'abc' }, holen);

  assert.deepEqual(ergebnis, { serverWeg: false, status: 200, daten: { kennung: 'shoff@127.0.0.1' } });
  assert.equal(aufrufe.length, 1);
  assert.equal(aufrufe[0].pfad, '/api/ich');
  assert.equal(aufrufe[0].optionen.method, 'GET');
  assert.equal(aufrufe[0].optionen.headers['X-LocalchatA-Sitzung'], 'abc');
  assert.equal(aufrufe[0].optionen.body, undefined);
});

test('POST mit Körper schickt JSON mit Content-Type und ohne Token keinen Sitzungs-Header', async () => {
  const { holen, aufrufe } = antwortenMit(201, '{"token":"t"}');

  const ergebnis = await rufen('/api/beitreten', { koerper: { name: 'shoff', art: 'mensch' } }, holen);

  assert.equal(ergebnis.status, 201);
  const { optionen } = aufrufe[0];
  assert.equal(optionen.method, 'POST');
  assert.equal(optionen.headers['Content-Type'], 'application/json');
  assert.equal(optionen.headers['X-LocalchatA-Sitzung'], undefined);
  assert.deepEqual(JSON.parse(optionen.body), { name: 'shoff', art: 'mensch' });
});

test('Fehlerstatus kommen als Antwort zurück, nicht als Ausnahme', async () => {
  const { holen } = antwortenMit(409, '{"vorschlag":"shoff-2"}');

  const ergebnis = await rufen('/api/beitreten', { koerper: { name: 'shoff' } }, holen);

  assert.deepEqual(ergebnis, { serverWeg: false, status: 409, daten: { vorschlag: 'shoff-2' } });
});

test('eine Antwort ohne JSON hat keine Daten', async () => {
  const { holen } = antwortenMit(415, 'Expected request with `Content-Type: application/json`');

  assert.deepEqual(await rufen('/api/x', {}, holen), { serverWeg: false, status: 415, daten: null });
});

test('ist der Server nicht erreichbar, heißt das Ergebnis Server weg', async () => {
  const holen = async () => { throw new TypeError('Failed to fetch'); };

  assert.deepEqual(await rufen('/api/ich', { token: 'abc' }, holen), { serverWeg: true, status: 0, daten: null });
});

test('fristSetzen: PUT mit Sitzung und {tage}; bestätigt mit bestaetigt:true; Name kodiert (R00008 B0195)', async () => {
  const { fristSetzen } = await import('../../web/api.js');
  const { holen, aufrufe } = antwortenMit(409, '{"regel":"bestaetigen","entfaellt":{"tage":2,"nachrichten":5,"bis":"2026-10-21"}}');

  const ergebnis = await fristSetzen('allgemein', 7, false, { token: 'abc' }, holen);
  await fristSetzen('allgemein', 7, true, { token: 'abc' }, holen);

  assert.deepEqual(ergebnis, {
    serverWeg: false, status: 409, daten: { regel: 'bestaetigen', entfaellt: { tage: 2, nachrichten: 5, bis: '2026-10-21' } },
  });
  assert.equal(aufrufe[0].pfad, '/api/channels/allgemein/frist');
  assert.equal(aufrufe[0].optionen.method, 'PUT');
  assert.equal(aufrufe[0].optionen.headers['X-LocalchatA-Sitzung'], 'abc');
  assert.equal(aufrufe[0].optionen.headers['Content-Type'], 'application/json');
  assert.deepEqual(JSON.parse(aufrufe[0].optionen.body), { tage: 7 });
  assert.deepEqual(JSON.parse(aufrufe[1].optionen.body), { tage: 7, bestaetigt: true });
});

test('fristSetzen: ein unerreichbarer Server ist serverWeg', async () => {
  const { fristSetzen } = await import('../../web/api.js');
  const holen = async () => {
    throw new TypeError('Failed to fetch');
  };

  assert.deepEqual(await fristSetzen('allgemein', 30, false, { token: 'abc' }, holen), { serverWeg: true, status: 0, daten: null });
});

// R00011 B0219: POST …/nachrichten mit Sitzung, JSON {text} und Idempotency-Key; Antwort oder Netzfehler.
test('Nachricht senden schickt text, Sitzung und Schlüssel und liefert Status und Daten', async () => {
  const { holen, aufrufe } = antwortenMit(201, '{"channel":"a-b","nachricht":{"zeit":"z"}}');

  const ergebnis = await nachrichtSenden('a-b', 'Hallo\nWelt', 'u-1', { token: 't' }, holen);

  assert.deepEqual(ergebnis, { art: 'antwort', status: 201, daten: { channel: 'a-b', nachricht: { zeit: 'z' } } });
  assert.equal(aufrufe[0].pfad, '/api/channels/a-b/nachrichten');
  assert.equal(aufrufe[0].optionen.method, 'POST');
  assert.equal(aufrufe[0].optionen.headers['X-LocalchatA-Sitzung'], 't');
  assert.equal(aufrufe[0].optionen.headers['Content-Type'], 'application/json');
  assert.equal(aufrufe[0].optionen.headers['Idempotency-Key'], 'u-1');
  assert.deepEqual(JSON.parse(aufrufe[0].optionen.body), { text: 'Hallo\nWelt' });
});

test('Nachricht senden: eine Fehlerantwort ist eine Antwort, ein unerreichbarer Server ein Netzfehler', async () => {
  const abgewiesen = await nachrichtSenden('a', 'x', 'u', { token: 't' }, antwortenMit(422, '{"fehler":"leer"}').holen);
  const weg = await nachrichtSenden('a', 'x', 'u', { token: 't' }, async () => { throw new TypeError('Failed to fetch'); });

  assert.deepEqual(abgewiesen, { art: 'antwort', status: 422, daten: { fehler: 'leer' } });
  assert.deepEqual(weg, { art: 'netzfehler' });
});
