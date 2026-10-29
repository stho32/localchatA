// Unit-Tests der Zustandswahl beim Laden und Beitreten (R00003 F0010, ADR-0012 „Browser“, node --test):
// localStorage-Ersatz und API-Ersatz mit festen Antworten.
import test from 'node:test';
import assert from 'node:assert/strict';
import { beimLaden, beitreten, nachAblehnung, SITZUNG, NAME } from '../../web/sitzung.js';

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

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'alt' });
  assert.deepEqual(api.aufrufe, [['ich', 'alt']]);
});

test('Token abgelaufen (401): still mit dem gemerkten Namen beitreten und das neue Token merken', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMit({ ich: { serverWeg: false, status: 401, daten: { fehler: 'x' } }, beitreten: angenommen(201) });

  const zustand = await beimLaden(speicher, api);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'neu' });
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
    assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'neu' }, String(status));
    assert.deepEqual(speicher.alles(), { [SITZUNG]: 'neu', [NAME]: 'shoff' });
  }
});

test('Beitreten mit Regelverstoß (422): Hinweis und Vorschlag des Servers, nichts gemerkt', async () => {
  const speicher = speicherMit();
  const abweisung = { fehler: 'Der Name "Jörg" ist nicht erlaubt', regel: 'r', vorschlag: 'joerg' };

  const zustand = await beitreten('Jörg', speicher, apiMit({ beitreten: { serverWeg: false, status: 422, daten: abweisung } }));

  assert.deepEqual(zustand, { art: 'ungueltig', name: 'Jörg', fehler: abweisung.fehler, regel: 'r', vorschlag: 'joerg' });
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

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'frisch' });
  assert.deepEqual(api.aufrufe, [['ich', 'alt'], ['beitreten', 'shoff'], ['ich', 'frisch']]);
  assert.deepEqual(speicher.alles(), { [SITZUNG]: 'frisch', [NAME]: 'shoff' });
});

test('B1: kommt das Token des anderen Tabs erst nach dem 409, wartet der Tab darauf und zeigt die Kopfzeile', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMitAnderemTab(speicher, { tokenDesAnderenTabs: 'frisch', schreibtVor409: false });
  const geschwister = { neuesToken: async () => { speicher.setItem(SITZUNG, 'frisch'); return 'frisch'; } };

  const zustand = await beimLaden(speicher, api, geschwister);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'frisch' });
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

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'frisch' });
  assert.deepEqual(speicher.alles(), { [SITZUNG]: 'frisch', [NAME]: 'shoff' });
});

// Code-Review A6: nur 401 heißt „Sitzung weg“; andere Antworten von ich führen nicht in den stillen Beitritt.
for (const [status, daten] of [[500, { fehler: 'Interner Fehler' }], [503, null], [403, { fehler: 'Host nicht freigegeben' }], [429, { fehler: 'Zu viele' }]]) {
  test(`A6: ich antwortet ${status} — Zustand Fehler mit Text, kein Beitritt, Token bleibt`, async () => {
    const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
    const api = apiMit({ ich: { serverWeg: false, status, daten }, beitreten: angenommen(201) });

    const zustand = await beimLaden(speicher, api);

    let erwarteterText = `Unerwartete Antwort ${status}`;
    if (daten) {
      erwarteterText = daten.fehler;
    }
    assert.deepEqual(zustand, { art: 'fehler', name: 'shoff', fehler: erwarteterText });
    assert.deepEqual(api.aufrufe, [['ich', 'alt']]);
    assert.deepEqual(speicher.alles(), { [SITZUNG]: 'alt', [NAME]: 'shoff' });
  });
}

test('A6: lässt sich die Sitzung nicht merken (setItem wirft), zeigt der Tab trotzdem die Kopfzeile und warnt', async (t) => {
  const speicher = speicherMit();
  speicher.setItem = () => { throw new Error('QuotaExceededError'); };
  const warnungen = [];
  t.mock.method(console, 'warn', (...teile) => warnungen.push(teile.join(' ')));

  const zustand = await beitreten('shoff', speicher, apiMit({ beitreten: angenommen(201) }));

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'neu' });
  assert.equal(warnungen.length, 1);
  assert.match(warnungen[0], /nicht gemerkt/);
});

test('A2: Beitreten schickt das gemerkte Token mit — eine noch lebende Sitzung kehrt wieder statt 409', async () => {
  const aufrufe = [];
  const api = {
    ich: async () => assert.fail('ich wird beim Beitreten nicht gefragt'),
    beitreten: async (name, token) => { aufrufe.push([name, token]); return angenommen(200); },
  };

  const mitToken = await beitreten('shoff', speicherMit({ [SITZUNG]: 'lebt', [NAME]: 'shoff' }), api);
  const ohneToken = await beitreten('anna', speicherMit(), api);

  assert.equal(mitToken.art, 'kopfzeile');
  assert.equal(ohneToken.art, 'kopfzeile');
  assert.deepEqual(aufrufe, [['shoff', 'lebt'], ['anna', null]]);
});

// Nacharbeit 2 (Entwicklertest, AK F0013 „ohne einen einzigen Konsolenfehler“): nach dem 401 aus dem Strom
// tritt die Seite ohne GET /api/ich wieder bei — das 401 auf /api/ich (Vertrag aus R00003) fällt so nicht an.
test('nach Ablehnung im Strom: Beitritt mit gemerktem Namen, ohne /api/ich', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = apiMit({ beitreten: angenommen(201) });

  const zustand = await nachAblehnung(speicher, api, 'alt');

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'neu' });
  assert.deepEqual(api.aufrufe, [['beitreten', 'shoff']]);
  assert.equal(speicher.getItem(SITZUNG), 'neu');
});

test('nach Ablehnung im Strom: hat ein anderer Tab schon ein neues Token, kein Aufruf', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'neu-vom-nachbarn', [NAME]: 'shoff' });
  const api = apiMit({});

  const zustand = await nachAblehnung(speicher, api, 'alt');

  assert.equal(zustand.art, 'kopfzeile');
  assert.deepEqual(api.aufrufe, []);
});

test('nach Ablehnung im Strom: ohne gemerkten Namen der Beitreten-Bildschirm, ohne Aufruf', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt' });
  const api = apiMit({});

  const zustand = await nachAblehnung(speicher, api, 'alt');

  assert.deepEqual(zustand, { art: 'beitreten', name: '' });
  assert.deepEqual(api.aufrufe, []);
});

// Nacharbeit 3, Code-Review B2: auch nach einer Ablehnung im Strom fängt der Wiederbeitritt den 409 eines
// Nachbar-Tabs ab (LAN-Rückfall ohne SharedWorker und ohne Web Locks) und übernimmt dessen Token.
const VERGEBEN = { serverWeg: false, status: 409, daten: { belegt_von: 'shoff@127.0.0.1', seit: 'x', vorschlag: 'shoff-2' } };

test('nach Ablehnung im Strom: 409, der Nachbar hat schon geschrieben → Kopfzeile mit dessen Token', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const aufrufe = [];
  const api = {
    beitreten: async (name) => {
      aufrufe.push(['beitreten', name]);
      speicher.setItem(SITZUNG, 'neu-vom-nachbarn');
      return VERGEBEN;
    },
    ich: async (token) => { aufrufe.push(['ich', token]); return { serverWeg: false, status: 200, daten: SHOFF }; },
  };

  const zustand = await nachAblehnung(speicher, api, 'alt');

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'neu-vom-nachbarn' });
  assert.deepEqual(aufrufe, [['beitreten', 'shoff'], ['ich', 'neu-vom-nachbarn']]);
});

test('nach Ablehnung im Strom: 409, der Nachbar schreibt erst danach → warten, dann Kopfzeile', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const aufrufe = [];
  const api = {
    beitreten: async (name) => { aufrufe.push(['beitreten', name]); return VERGEBEN; },
    ich: async (token) => { aufrufe.push(['ich', token]); return { serverWeg: false, status: 200, daten: SHOFF }; },
  };
  const geschwister = { neuesToken: async () => 'neu-vom-nachbarn' };

  const zustand = await nachAblehnung(speicher, api, 'alt', geschwister);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'neu-vom-nachbarn' });
  assert.deepEqual(aufrufe, [['beitreten', 'shoff'], ['ich', 'neu-vom-nachbarn']]);
});

// Befund #58, zweite Gestalt (r00004 B2 mit 3 Tabs unter Last, Trace: 409, kein GET ich, nach 2 s Beitreten-Bildschirm):
// ohne Web Locks darf der Tab nach dem 409 den Speicher nicht anfassen, solange er auf den Nachbar-Tab wartet — ein
// eigenes removeItem aus einem veralteten Stand löscht im Browser das eben geschriebene Token des Nachbarn, und dessen
// storage-Ereignis bleibt aus. Das abgelehnte Token gilt beim Lesen schlicht nicht; vergessen wird es erst, wenn nichts
// gelungen ist.
test('Befund #58: nach dem 409 kein removeItem vor oder während des Wartens auf den Nachbar-Tab, Erfolg ohne removeItem', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const ablauf = [];
  const entfernen = speicher.removeItem;
  speicher.removeItem = (schluessel) => { ablauf.push(['removeItem', schluessel]); entfernen(schluessel); };
  const api = {
    beitreten: async () => VERGEBEN,
    ich: async (token) => { ablauf.push(['ich', token]); return { serverWeg: false, status: 200, daten: SHOFF }; },
  };
  const geschwister = { neuesToken: async () => { ablauf.push(['warten']); return 'neu-vom-nachbarn'; } };

  const zustand = await nachAblehnung(speicher, api, 'alt', geschwister);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'neu-vom-nachbarn' });
  assert.deepEqual(ablauf, [['warten'], ['ich', 'neu-vom-nachbarn']]);
});

test('Befund #58: gelingt nach dem 409 nichts, ist das abgelehnte Token danach vergessen', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = { beitreten: async () => VERGEBEN, ich: async () => ({ serverWeg: false, status: 401, daten: null }) };

  const zustand = await nachAblehnung(speicher, api, 'alt', { neuesToken: async () => null });

  assert.equal(zustand.art, 'vergeben');
  assert.equal(speicher.getItem(SITZUNG), null);
});

// Befund #58, Muster-Suche im Ladeweg: laden mehrere Tabs ohne Web Locks mit abgelaufenem Token, gilt dasselbe — kein
// removeItem zwischen dem 401 und dem Ende des Wartens auf den Nachbar-Tab; scheitert alles, ist das Token vergessen.
test('Befund #58 beim Laden: 401, 409, Token des Nachbar-Tabs — kein removeItem bis zum Erfolg', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const ablauf = [];
  const entfernen = speicher.removeItem;
  speicher.removeItem = (schluessel) => { ablauf.push(['removeItem', schluessel]); entfernen(schluessel); };
  const api = {
    ich: async (token) => {
      ablauf.push(['ich', token]);
      if (token === 'neu-vom-nachbarn') {
        return { serverWeg: false, status: 200, daten: SHOFF };
      }
      return { serverWeg: false, status: 401, daten: {} };
    },
    beitreten: async () => VERGEBEN,
  };
  const geschwister = { neuesToken: async () => { ablauf.push(['warten']); return 'neu-vom-nachbarn'; } };

  const zustand = await beimLaden(speicher, api, geschwister);

  assert.deepEqual(zustand, { art: 'kopfzeile', teilnehmer: SHOFF, token: 'neu-vom-nachbarn' });
  assert.deepEqual(ablauf, [['ich', 'alt'], ['warten'], ['ich', 'neu-vom-nachbarn']]);
});

test('Befund #58 beim Laden: 401 und nichts gelingt — das abgelaufene Token ist danach vergessen, mit und ohne Namen', async () => {
  for (const name of ['shoff', '']) {
    const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: name });
    const api = apiMit({ ich: { serverWeg: false, status: 401, daten: {} }, beitreten: VERGEBEN });

    await beimLaden(speicher, api);

    assert.equal(speicher.getItem(SITZUNG), null, name);
  }
});
