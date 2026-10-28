// Unit-Tests des Strom-Halters (R00004 F0013, B0091; node --test) mit einem Ersatz für fetch, der echte
// Response-Objekte mit ReadableStream liefert.
import test from 'node:test';
import assert from 'node:assert/strict';
import { halten } from '../../web/live.js';

const kodierer = new TextEncoder();

// Ein Strom, dem der Test Teilstücke nachschiebt und den er schließen kann; wie bei fetch bricht das Signal
// des Abrufs den Körper ab (`antwortFuer(optionen)`).
function offenerStrom() {
  let steuerung;
  const body = new ReadableStream({ start(controller) { steuerung = controller; } });
  const antwort = new Response(body, { status: 200, headers: { 'Content-Type': 'text/event-stream' } });
  return {
    antwort,
    antwortFuer(optionen) {
      optionen.signal.addEventListener('abort', () => steuerung.error(new DOMException('abgebrochen', 'AbortError')));
      return antwort;
    },
    schieben: (text) => steuerung.enqueue(kodierer.encode(text)),
    schliessen: () => steuerung.close(),
  };
}

function bis(bedingung, hoechstensMs = 2000) {
  const anfang = Date.now();
  return new Promise((erfuellt, gescheitert) => {
    const pruefen = () => {
      if (bedingung()) {
        erfuellt();
      } else if (Date.now() - anfang > hoechstensMs) {
        gescheitert(new Error('Bedingung nicht erreicht'));
      } else {
        setTimeout(pruefen, 5);
      }
    };
    pruefen();
  });
}

function protokoll() {
  const ereignisse = [];
  const zustaende = [];
  const wartezeiten = [];
  return {
    ereignisse,
    zustaende,
    wartezeiten,
    beiEreignis: (ereignis) => ereignisse.push(ereignis),
    beiZustand: (zustand) => zustaende.push(zustand),
    schlafen: async (ms) => { wartezeiten.push(ms); },
  };
}

test('Header mit Token, Ereignisse als JSON, Herzschlag nicht weitergereicht, verbunden gemeldet', async () => {
  const strom = offenerStrom();
  const aufrufe = [];
  const p = protokoll();
  const halter = halten({
    token: 'abc', holen: async (pfad, optionen) => { aufrufe.push({ pfad, optionen }); return strom.antwortFuer(optionen); }, ...p,
  });

  strom.schieben('data: {"art":"teilnehmer","stand":1,"liste":[]}\n\nevent: herzschlag\ndata: {}\n\n');
  strom.schieben('data: {"art":"anwesenheit","st');
  strom.schieben('and":2,"teilnehmer":{}}\n\n');
  await bis(() => p.ereignisse.length === 2);
  halter.beenden();

  assert.equal(aufrufe[0].pfad, '/api/ereignisse');
  assert.equal(aufrufe[0].optionen.headers['X-LocalchatA-Sitzung'], 'abc');
  assert.deepEqual(p.ereignisse.map((ereignis) => ereignis.stand), [1, 2]);
  assert.deepEqual(p.zustaende, ['verbunden']);
});

test('Server weg: unterbrochen, dann neu nach 1 s, 2 s, 5 s, 10 s; danach wieder verbunden', async () => {
  const strom = offenerStrom();
  let versuche = 0;
  const p = protokoll();
  const holen = async (pfad, optionen) => {
    versuche += 1;
    if (versuche <= 4) {
      throw new TypeError('Failed to fetch');
    }
    return strom.antwortFuer(optionen);
  };
  const halter = halten({ token: 'abc', holen, ...p });

  await bis(() => p.zustaende.includes('verbunden'));
  halter.beenden();

  assert.deepEqual(p.wartezeiten.filter((ms) => ms > 0), [1000, 2000, 5000, 10000]);
  assert.deepEqual(p.zustaende, ['unterbrochen', 'unterbrochen', 'unterbrochen', 'unterbrochen', 'verbunden']);
});

test('endet der Strom, meldet der Halter unterbrochen und verbindet nach 1 s neu', async () => {
  const erster = offenerStrom();
  const zweiter = offenerStrom();
  const stroeme = [erster, zweiter];
  const p = protokoll();
  const halter = halten({ token: 'abc', holen: async (pfad, optionen) => stroeme.shift().antwortFuer(optionen), ...p });

  await bis(() => p.zustaende.length === 1);
  erster.schliessen();
  await bis(() => p.zustaende.length === 3);
  halter.beenden();

  assert.deepEqual(p.zustaende, ['verbunden', 'unterbrochen', 'verbunden']);
  assert.ok(p.wartezeiten.includes(1000), String(p.wartezeiten));
});

test('401: abgelehnt mit dem abgelehnten Token, kein neuer Versuch bis ein neues Token gesetzt ist', async () => {
  const strom = offenerStrom();
  const genutzteTokens = [];
  const p = protokoll();
  const holen = async (pfad, optionen) => {
    const token = optionen.headers['X-LocalchatA-Sitzung'];
    genutzteTokens.push(token);
    if (token === 'alt') {
      return new Response('{"fehler":"x"}', { status: 401 });
    }
    return strom.antwortFuer(optionen);
  };
  const abgelehnt = [];
  const halter = halten({ token: 'alt', holen, ...p, beiZustand: (zustand, token) => { p.zustaende.push(zustand); abgelehnt.push(token); } });

  await bis(() => p.zustaende.includes('abgelehnt'));
  await new Promise((weiter) => setTimeout(weiter, 50));
  const vorDemNeuenToken = genutzteTokens.length;
  halter.tokenSetzen('neu');
  await bis(() => p.zustaende.includes('verbunden'));
  halter.beenden();

  assert.equal(vorDemNeuenToken, 1);
  assert.deepEqual(genutzteTokens, ['alt', 'neu']);
  assert.equal(abgelehnt[0], 'alt');
});

test('40 s ohne Byte: der Wächter bricht ab und verbindet neu', async () => {
  const erster = offenerStrom();
  const zweiter = offenerStrom();
  const stroeme = [erster, zweiter];
  let uhr = 0;
  const p = protokoll();
  const halter = halten({
    token: 'abc', holen: async (pfad, optionen) => stroeme.shift().antwortFuer(optionen), ...p, jetzt: () => uhr, pruefAbstandMs: 5,
  });

  await bis(() => p.zustaende.length === 1);
  uhr = 40001;
  await bis(() => p.zustaende.length === 3);
  halter.beenden();

  assert.deepEqual(p.zustaende, ['verbunden', 'unterbrochen', 'verbunden']);
});

test('nach abgelehnt öffnet der Strom mit dem neuen Token sofort, auch nach vielen Fehlversuchen (Nacharbeit 1)', async () => {
  const strom = offenerStrom();
  const p = protokoll();
  let versuche = 0;
  const holen = async (pfad, optionen) => {
    versuche += 1;
    if (versuche <= 4) {
      throw new TypeError('Failed to fetch');
    }
    if (optionen.headers['X-LocalchatA-Sitzung'] === 'alt') {
      return new Response('{"fehler":"x"}', { status: 401 });
    }
    return strom.antwortFuer(optionen);
  };
  let wartezeitenBisAbgelehnt = null;
  const beiZustand = (zustand) => {
    p.zustaende.push(zustand);
    if (zustand === 'abgelehnt') {
      wartezeitenBisAbgelehnt = p.wartezeiten.length;
    }
  };
  const halter = halten({ token: 'alt', holen, ...p, beiZustand });

  await bis(() => p.zustaende.includes('abgelehnt'));
  halter.tokenSetzen('neu');
  await bis(() => p.zustaende.includes('verbunden'));
  halter.beenden();

  assert.deepEqual(p.wartezeiten.slice(wartezeitenBisAbgelehnt).filter((ms) => ms > 0), []);
});

test('dauerhaft 401 bei sofortigem Wiederbeitritt: höchstens 3 Abrufe in 1 s (Nacharbeit 2, Untergrenze)', async () => {
  let abrufe = 0;
  let nummer = 0;
  let halter = null;
  // Ohne Untergrenze dreht die Schleife nur über Mikrotasks und ließe den Zeitgeber nie laufen — ab 10
  // Abrufen beendet der Ersatz den Halter selbst, damit der Test rot wird statt zu hängen.
  const holen = async () => {
    abrufe += 1;
    if (abrufe > 10) {
      halter.beenden();
    }
    return new Response('{"fehler":"x"}', { status: 401 });
  };
  halter = halten({
    token: 't0',
    holen,
    beiEreignis: () => {},
    beiZustand: (zustand) => {
      if (zustand === 'abgelehnt') {
        nummer += 1;
        queueMicrotask(() => halter.tokenSetzen(`t${nummer}`));
      }
    },
  });

  await new Promise((weiter) => setTimeout(weiter, 1000));
  halter.beenden();

  assert.ok(abrufe <= 3, `${abrufe} Abrufe in 1 s`);
});

test('nach erfolgreichem Wiederbeitritt verbindet der Halter sofort — auch beim zweiten 401 im Fenster (Nacharbeit 3, A2)', async () => {
  const strom = offenerStrom();
  const verlauf = [];
  let versuche = 0;
  const holen = async (pfad, optionen) => {
    versuche += 1;
    const token = optionen.headers['X-LocalchatA-Sitzung'];
    verlauf.push(`holen ${token}`);
    if (versuche <= 3) {
      throw new TypeError('Failed to fetch');
    }
    if (token !== 'neu') {
      return new Response('{"fehler":"x"}', { status: 401 });
    }
    return strom.antwortFuer(optionen);
  };
  let halter = null;
  const ersatzToken = { alt: 'mittel', mittel: 'neu' };
  halter = halten({
    token: 'alt',
    holen,
    beiEreignis: () => {},
    schlafen: async (ms) => { verlauf.push(`warten ${ms}`); },
    beiZustand: (zustand, abgelehnt) => {
      if (zustand === 'abgelehnt') {
        queueMicrotask(() => {
          verlauf.push(`neues Token ${ersatzToken[abgelehnt]}`);
          halter.tokenSetzen(ersatzToken[abgelehnt]);
        });
      }
    },
  });

  await bis(() => verlauf.includes('holen neu'));
  halter.beenden();

  const ab = (eintrag) => verlauf.indexOf(eintrag);
  const zwischen = verlauf.slice(ab('neues Token neu') + 1, ab('holen neu'));
  assert.deepEqual(zwischen.filter((eintrag) => eintrag.startsWith('warten') && eintrag !== 'warten 0'), [], verlauf.join(' | '));
  const zwischenMittel = verlauf.slice(ab('neues Token mittel') + 1, ab('holen mittel'));
  assert.deepEqual(zwischenMittel.filter((eintrag) => eintrag.startsWith('warten') && eintrag !== 'warten 0'), [], verlauf.join(' | '));
});
