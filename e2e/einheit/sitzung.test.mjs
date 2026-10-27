// Unit-Tests der Zustandswahl beim Laden und Beitreten (R00003 F0010, ADR-0012 „Browser“, node --test):
// localStorage-Ersatz und API-Ersatz mit festen Antworten.
import test from 'node:test';
import assert from 'node:assert/strict';
import { beimLaden, beitreten, SITZUNG, NAME } from '../../web/sitzung.js';

const SHOFF = { kennung: 'shoff@127.0.0.1', name: 'shoff', herkunft: '127.0.0.1', art: 'mensch' };
const SERVER_WEG = { serverWeg: true, status: 0, daten: null };

function speicherMit(eintraege = {}) {
  const werte = new Map(Object.entries(eintraege));
  return {
    getItem: (schluessel) => {
      if (werte.has(schluessel)) {
        return werte.get(schluessel);
      }
      return null;
    },
    setItem: (schluessel, wert) => werte.set(schluessel, String(wert)),
    removeItem: (schluessel) => werte.delete(schluessel),
    alles: () => Object.fromEntries(werte),
  };
}

function apiMit({ ich, beitreten: beitrittsantwort }) {
  const aufrufe = [];
  return {
    aufrufe,
    ich: async (token) => { aufrufe.push(['ich', token]); return ich; },
    beitreten: async (name) => { aufrufe.push(['beitreten', name]); return beitrittsantwort; },
  };
}

const angenommen = (status) => ({ serverWeg: false, status, daten: { token: 'neu', ...SHOFF } });

test('ohne Token und ohne Namen: Beitreten-Bildschirm leer, kein Aufruf', async () => {
  const api = apiMit({});

  const zustand = await beimLaden(speicherMit(), api);

  assert.deepEqual(zustand, { art: 'beitreten', name: '' });
  assert.deepEqual(api.aufrufe, []);
});

test('Token gültig: Kopfzeile ohne Beitritt', async () => {
  const api = apiMit({ ich: { serverWeg: false, status: 200, daten: SHOFF } });

  const zustand = await beimLaden(speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' }), api);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF });
  assert.deepEqual(api.aufrufe, [['ich', 'alt']]);
});

test('Token abgelaufen (401): still mit dem gemerkten Namen beitreten und das neue Token merken', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMit({ ich: { serverWeg: false, status: 401, daten: { fehler: 'x' } }, beitreten: angenommen(201) });

  const zustand = await beimLaden(speicher, api);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF });
  assert.deepEqual(api.aufrufe, [['ich', 'alt'], ['beitreten', 'shoff']]);
  assert.deepEqual(speicher.alles(), { [SITZUNG]: 'neu', [NAME]: 'shoff' });
});

test('kein Token, aber ein gemerkter Name: still beitreten', async () => {
  const api = apiMit({ beitreten: angenommen(201) });

  const zustand = await beimLaden(speicherMit({ [NAME]: 'shoff' }), api);

  assert.equal(zustand.art, 'kopfzeile');
  assert.deepEqual(api.aufrufe, [['beitreten', 'shoff']]);
});

test('beim stillen Beitritt vergeben (409): Zustand Name vergeben mit Träger, seit und Vorschlag; altes Token weg', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const vergeben = { belegt_von: 'shoff@127.0.0.2', seit: '2026-10-27T09:14:00.000+01:00', vorschlag: 'shoff-2', fehler: 'x' };
  const api = apiMit({ ich: { serverWeg: false, status: 401, daten: {} }, beitreten: { serverWeg: false, status: 409, daten: vergeben } });

  const zustand = await beimLaden(speicher, api);

  assert.deepEqual(zustand, {
    art: 'vergeben', name: 'shoff', belegtVon: 'shoff@127.0.0.2', seit: '2026-10-27T09:14:00.000+01:00', vorschlag: 'shoff-2',
  });
  assert.equal(speicher.getItem(SITZUNG), null);
  assert.equal(speicher.getItem(NAME), 'shoff');
});

test('Server weg beim Laden: Zustand Server weg mit dem gemerkten Namen', async () => {
  const api = apiMit({ ich: SERVER_WEG });

  const zustand = await beimLaden(speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' }), api);

  assert.deepEqual(zustand, { art: 'serverWeg', name: 'shoff' });
});

test('Beitreten: 201 und 200 führen zur Kopfzeile und merken Token und Namen', async () => {
  for (const status of [201, 200]) {
    const speicher = speicherMit();
    const zustand = await beitreten('shoff', speicher, apiMit({ beitreten: angenommen(status) }));
    assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF }, String(status));
    assert.deepEqual(speicher.alles(), { [SITZUNG]: 'neu', [NAME]: 'shoff' });
  }
});

test('Beitreten mit Regelverstoß (422): Hinweis und Vorschlag des Servers, nichts gemerkt', async () => {
  const speicher = speicherMit();
  const abweisung = { fehler: 'Der Name "Jörg" ist nicht erlaubt', regel: 'r', vorschlag: 'joerg' };

  const zustand = await beitreten('Jörg', speicher, apiMit({ beitreten: { serverWeg: false, status: 422, daten: abweisung } }));

  assert.deepEqual(zustand, { art: 'ungueltig', name: 'Jörg', fehler: abweisung.fehler, vorschlag: 'joerg' });
  assert.deepEqual(speicher.alles(), {});
});

test('Beitreten ohne Vorschlag des Servers: Vorschlag leer', async () => {
  const zustand = await beitreten('___', speicherMit(), apiMit({ beitreten: { serverWeg: false, status: 422, daten: { fehler: 'f', regel: 'r' } } }));

  assert.equal(zustand.vorschlag, '');
});

test('Beitreten, Server weg: Zustand Server weg mit dem getippten Namen', async () => {
  const zustand = await beitreten('shoff', speicherMit(), apiMit({ beitreten: SERVER_WEG }));

  assert.deepEqual(zustand, { art: 'serverWeg', name: 'shoff' });
});

test('andere Antworten (429, 503) werden mit dem Text des Servers gemeldet', async () => {
  const zustand = await beitreten('n65', speicherMit(), apiMit({ beitreten: { serverWeg: false, status: 429, daten: { fehler: 'Zu viele Sitzungen' } } }));

  assert.deepEqual(zustand, { art: 'fehler', name: 'n65', fehler: 'Zu viele Sitzungen' });
});

// Code-Review B1: zwei Tabs laden gleichzeitig, beide bekommen 401; der andere Tab tritt zuerst bei.
function apiMitAnderemTab(speicher, { tokenDesAnderenTabs, schreibtVor409 }) {
  const aufrufe = [];
  const vergeben = { belegt_von: 'shoff@127.0.0.1', seit: '2026-10-28T00:10:00.000+01:00', vorschlag: 'shoff-2', fehler: 'x' };
  return {
    aufrufe,
    ich: async (token) => {
      aufrufe.push(['ich', token]);
      if (token === tokenDesAnderenTabs) {
        return { serverWeg: false, status: 200, daten: SHOFF };
      }
      return { serverWeg: false, status: 401, daten: {} };
    },
    beitreten: async (name) => {
      aufrufe.push(['beitreten', name]);
      if (schreibtVor409) {
        speicher.setItem(SITZUNG, tokenDesAnderenTabs);
      }
      return { serverWeg: false, status: 409, daten: vergeben };
    },
  };
}

test('B1: schreibt ein anderer Tab zwischen 401 und 409 ein gültiges Token, zeigt der Tab die Kopfzeile, nicht vergeben', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMitAnderemTab(speicher, { tokenDesAnderenTabs: 'frisch', schreibtVor409: true });

  const zustand = await beimLaden(speicher, api);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF });
  assert.deepEqual(api.aufrufe, [['ich', 'alt'], ['beitreten', 'shoff'], ['ich', 'frisch']]);
  assert.deepEqual(speicher.alles(), { [SITZUNG]: 'frisch', [NAME]: 'shoff' });
});

test('B1: kommt das Token des anderen Tabs erst nach dem 409, wartet der Tab darauf und zeigt die Kopfzeile', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMitAnderemTab(speicher, { tokenDesAnderenTabs: 'frisch', schreibtVor409: false });
  const geschwister = { neuesToken: async () => { speicher.setItem(SITZUNG, 'frisch'); return 'frisch'; } };

  const zustand = await beimLaden(speicher, api, geschwister);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF });
  assert.deepEqual(api.aufrufe, [['ich', 'alt'], ['beitreten', 'shoff'], ['ich', 'frisch']]);
});

test('B1: kommt kein Token eines anderen Tabs, bleibt es beim echten Name vergeben', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMitAnderemTab(speicher, { tokenDesAnderenTabs: 'frisch', schreibtVor409: false });
  const geschwister = { neuesToken: async () => null };

  const zustand = await beimLaden(speicher, api, geschwister);

  assert.equal(zustand.art, 'vergeben');
  assert.equal(zustand.vorschlag, 'shoff-2');
  assert.deepEqual(api.aufrufe, [['ich', 'alt'], ['beitreten', 'shoff']]);
});

test('B1: ist das Token des anderen Tabs selbst ungültig, bleibt es bei Name vergeben', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMitAnderemTab(speicher, { tokenDesAnderenTabs: 'frisch', schreibtVor409: false });
  const geschwister = { neuesToken: async () => 'kaputt' };

  const zustand = await beimLaden(speicher, api, geschwister);

  assert.equal(zustand.art, 'vergeben');
  assert.deepEqual(api.aufrufe.at(-1), ['ich', 'kaputt']);
});

test('B1: schreibt ein anderer Tab sein Token, bevor das eigene 401 ankommt, bleibt dessen Token stehen', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMitAnderemTab(speicher, { tokenDesAnderenTabs: 'frisch', schreibtVor409: false });
  const ichMitAnderemTab = api.ich;
  api.ich = async (token) => {
    if (token === 'alt') {
      speicher.setItem(SITZUNG, 'frisch');
    }
    return ichMitAnderemTab(token);
  };

  const zustand = await beimLaden(speicher, api);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF });
  assert.deepEqual(speicher.alles(), { [SITZUNG]: 'frisch', [NAME]: 'shoff' });
});
