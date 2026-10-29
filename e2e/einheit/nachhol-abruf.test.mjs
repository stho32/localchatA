// Unit-Tests des Nachhol-Abrufs mit Zeitgrenze (R00012 F0047 „Neuversuch“, Nacharbeit #22 B0392; ADR-0037 Ergänzung
// Karte #22/#36, 7a; node --test) mit gestellten Zeitgebern und einem Ersatz für fetch: bei 9 999 ms läuft der Abruf,
// bei 10 000 ms ist er abgebrochen (Mutanten 9 s und 11 s werden rot).
import test from 'node:test';
import assert from 'node:assert/strict';
import { verlaufNachholen } from '../../web/api.js';

const BLOCK = { channel: 'allgemein', nachrichten: [], aelter: true, neuer: false };
const CURSOR = { nach: '2026-10-28T14:00:00.000+01:00' };

function gestellteZeitgeber() {
  let jetzt = 0;
  let wartende = [];
  return {
    zeitgeber: {
      stellen: (ms, rueckruf) => {
        const eintrag = { faellig: jetzt + ms, rueckruf };
        wartende.push(eintrag);
        return eintrag;
      },
      loeschen: (eintrag) => { wartende = wartende.filter((wartend) => wartend !== eintrag); },
    },
    vorstellen: (ms) => {
      jetzt += ms;
      const faellige = wartende.filter((wartend) => wartend.faellig <= jetzt);
      wartende = wartende.filter((wartend) => wartend.faellig > jetzt);
      faellige.forEach((wartend) => wartend.rueckruf());
    },
    offen: () => wartende.length,
  };
}

// Wie fetch: antwortet nie, bricht aber mit einem AbortError ab, sobald das Signal abbricht.
function nieAntworten(pfad, optionen) {
  return new Promise((_, gescheitert) => {
    optionen.signal.addEventListener('abort', () => gescheitert(new DOMException('abgebrochen', 'AbortError')));
  });
}

const antwortMit = (status, daten) => async () => ({ status, ok: status < 300, json: async () => daten });
const naechsteRunde = () => new Promise((weiter) => setImmediate(weiter));

test('ohne Antwort: bei 9 999 ms läuft der Abruf noch, bei 10 000 ms Zeitüberschreitung', async () => {
  const uhr = gestellteZeitgeber();
  let ergebnis = null;
  verlaufNachholen('allgemein', CURSOR, { token: 't', n: 200, signal: new AbortController().signal, zeitgeber: uhr.zeitgeber }, nieAntworten)
    .then((fertig) => { ergebnis = fertig; });

  uhr.vorstellen(9999);
  await naechsteRunde();
  const bei9999 = ergebnis;
  uhr.vorstellen(1);
  await naechsteRunde();

  assert.equal(bei9999, null);
  assert.deepEqual(ergebnis, { art: 'zeitueberschreitung' });
});

test('ein Block vor der Zeitgrenze kommt durch und löscht den Zeitgeber; gefragt wird nach= mit n=200', async () => {
  const uhr = gestellteZeitgeber();
  const aufrufe = [];
  const holen = async (pfad, optionen) => {
    aufrufe.push(pfad);
    return antwortMit(200, BLOCK)(pfad, optionen);
  };

  const ergebnis = await verlaufNachholen('allgemein', CURSOR, { token: 't', n: 200, signal: new AbortController().signal, zeitgeber: uhr.zeitgeber }, holen);

  assert.deepEqual(ergebnis, { art: 'block', block: BLOCK });
  assert.equal(uhr.offen(), 0);
  assert.deepEqual(aufrufe, [`/api/channels/allgemein/nachrichten?nach=${encodeURIComponent(CURSOR.nach)}&n=200`]);
});

test('ein Status außer 200 und ein Netzfehler sind Fehler, kein Block', async () => {
  const uhr = gestellteZeitgeber();
  const optionen = { token: 't', n: 200, signal: new AbortController().signal, zeitgeber: uhr.zeitgeber };

  const status500 = await verlaufNachholen('allgemein', CURSOR, optionen, antwortMit(500, null));
  const status401 = await verlaufNachholen('allgemein', CURSOR, optionen, antwortMit(401, null));
  const netz = await verlaufNachholen('allgemein', CURSOR, optionen, async () => { throw new TypeError('Failed to fetch'); });

  assert.deepEqual([status500, status401, netz], [
    { art: 'fehler', status: 500 }, { art: 'fehler', status: 401 }, { art: 'fehler', status: 0 },
  ]);
  assert.equal(uhr.offen(), 0);
});

test('bricht der Aufrufer ab (Trennung, Channelwechsel), ist das keine Zeitüberschreitung', async () => {
  const uhr = gestellteZeitgeber();
  const abbruch = new AbortController();
  const laufend = verlaufNachholen('allgemein', CURSOR, { token: 't', n: 200, signal: abbruch.signal, zeitgeber: uhr.zeitgeber }, nieAntworten);

  uhr.vorstellen(500);
  abbruch.abort();

  assert.deepEqual(await laufend, { art: 'abgebrochen' });
  assert.equal(uhr.offen(), 0);
});
