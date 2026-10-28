// Unit-Tests des Verlaufsabrufs (R00007 F0029 B0153, ADR-0025; node --test) mit einem Ersatz für fetch.
import test from 'node:test';
import assert from 'node:assert/strict';
import { verlaufAbrufen } from '../../web/api.js';

const BLOCK = { channel: 'allgemein', nachrichten: [], aelter: false, neuer: false };

function holenMit(antwort) {
  const aufrufe = [];
  const holen = async (pfad, optionen) => {
    aufrufe.push({ pfad, optionen });
    return antwort(optionen);
  };
  return { holen, aufrufe };
}

const json = (status, daten) => () => ({ status, ok: status < 300, json: async () => daten });

test('ohne Cursor die neuesten 100, mit Sitzungs-Header und Abbruchsignal', async () => {
  const { holen, aufrufe } = holenMit(json(200, BLOCK));
  const abbruch = new AbortController();

  const ergebnis = await verlaufAbrufen('allgemein', {}, { token: 'abc', signal: abbruch.signal }, holen);

  assert.deepEqual(ergebnis, { art: 'block', block: BLOCK });
  assert.equal(aufrufe[0].pfad, '/api/channels/allgemein/nachrichten?n=100');
  assert.equal(aufrufe[0].optionen.headers['X-LocalchatA-Sitzung'], 'abc');
  assert.equal(aufrufe[0].optionen.signal, abbruch.signal);
});

test('der Cursor ist kodiert — das + im Versatz wird %2B', async () => {
  const { holen, aufrufe } = holenMit(json(200, BLOCK));

  await verlaufAbrufen('gross', { vor: '2026-10-28T14:00:00.000+01:00' }, { token: 't' }, holen);
  await verlaufAbrufen('gross', { nach: '2026-10-28T14:00:00.000+01:00' }, { token: 't' }, holen);

  assert.equal(aufrufe[0].pfad, '/api/channels/gross/nachrichten?vor=2026-10-28T14%3A00%3A00.000%2B01%3A00&n=100');
  assert.equal(aufrufe[1].pfad, '/api/channels/gross/nachrichten?nach=2026-10-28T14%3A00%3A00.000%2B01%3A00&n=100');
});

test('eine Fehlerantwort ist ein Fehler mit Status, ein unerreichbarer Server einer mit Status 0', async () => {
  const fehlerantwort = await verlaufAbrufen('fehlt', {}, { token: 't' }, holenMit(json(404, { fehler: 'x' })).holen);
  const weg = await verlaufAbrufen('allgemein', {}, { token: 't' }, async () => { throw new TypeError('Failed to fetch'); });

  assert.deepEqual(fehlerantwort, { art: 'fehler', status: 404 });
  assert.deepEqual(weg, { art: 'fehler', status: 0 });
});

test('ein abgebrochener Abruf ist abgebrochen, kein Fehler', async () => {
  const abbruch = new AbortController();
  const holen = async (pfad, optionen) => new Promise((_, scheitern) => {
    optionen.signal.addEventListener('abort', () => scheitern(new DOMException('abgebrochen', 'AbortError')));
  });

  const laufend = verlaufAbrufen('gross', {}, { token: 't', signal: abbruch.signal }, holen);
  abbruch.abort();

  assert.deepEqual(await laufend, { art: 'abgebrochen' });
});
