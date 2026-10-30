// Unit-Tests des Aufrufs der Aktivität (R00045 F0161 B0688; ADR-0051 Regel 4; node --test) mit einem Ersatz für fetch:
// Setzen ist PUT {"zustand":"tippt"}, Löschen DELETE ohne Körper, beide mit Sitzung; 429 nennt Retry-After in s.
import test from 'node:test';
import assert from 'node:assert/strict';
import { aktivitaetMelden } from '../../web/api.js';

function antwortenMit(status, kopf = {}) {
  const aufrufe = [];
  const holen = async (pfad, optionen) => {
    aufrufe.push({ pfad, optionen });
    return { status, headers: new Headers(kopf), text: async () => '' };
  };
  return { holen, aufrufe };
}

test('Setzen schickt PUT {"zustand":"tippt"} mit Sitzung an …/aktivitaet', async () => {
  const { holen, aufrufe } = antwortenMit(200);

  const ergebnis = await aktivitaetMelden({ art: 'setzen', channel: 'a-b' }, { token: 't' }, holen);

  assert.deepEqual(ergebnis, { art: 'antwort', status: 200, wartenS: null });
  assert.equal(aufrufe[0].pfad, '/api/channels/a-b/aktivitaet');
  assert.equal(aufrufe[0].optionen.method, 'PUT');
  assert.equal(aufrufe[0].optionen.headers['X-LocalchatA-Sitzung'], 't');
  assert.equal(aufrufe[0].optionen.headers['Content-Type'], 'application/json');
  assert.deepEqual(JSON.parse(aufrufe[0].optionen.body), { zustand: 'tippt' });
});

test('Löschen schickt DELETE ohne Körper mit Sitzung', async () => {
  const { holen, aufrufe } = antwortenMit(204);

  const ergebnis = await aktivitaetMelden({ art: 'loeschen', channel: 'allgemein' }, { token: 't' }, holen);

  assert.deepEqual(ergebnis, { art: 'antwort', status: 204, wartenS: null });
  assert.equal(aufrufe[0].pfad, '/api/channels/allgemein/aktivitaet');
  assert.equal(aufrufe[0].optionen.method, 'DELETE');
  assert.equal(aufrufe[0].optionen.body, undefined);
  assert.equal(aufrufe[0].optionen.headers['X-LocalchatA-Sitzung'], 't');
});

test('429 nennt Retry-After in Sekunden; ohne lesbaren Kopf gilt 1 s', async () => {
  const mitKopf = await aktivitaetMelden({ art: 'setzen', channel: 'a' }, { token: 't' }, antwortenMit(429, { 'Retry-After': '3' }).holen);
  const ohneKopf = await aktivitaetMelden({ art: 'setzen', channel: 'a' }, { token: 't' }, antwortenMit(429).holen);

  assert.deepEqual(mitKopf, { art: 'antwort', status: 429, wartenS: 3 });
  assert.deepEqual(ohneKopf, { art: 'antwort', status: 429, wartenS: 1 });
});

test('ein unerreichbarer Server ist ein Netzfehler', async () => {
  const ergebnis = await aktivitaetMelden({ art: 'setzen', channel: 'a' }, { token: 't' }, async () => {
    throw new TypeError('Failed to fetch');
  });

  assert.deepEqual(ergebnis, { art: 'netzfehler' });
});
