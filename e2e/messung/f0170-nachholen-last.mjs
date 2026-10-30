'use strict';
// Lastprobe F0170 „Nachholen bleibt begrenzt“ (R00049, B0759; ADR-0054 Ergänzung „Nachholen von warten …“,
// Abnahme-Probe des security-reviewer und Z1–Z5). Messwert im Entwicklertest, kein Gate; Release-Build, nicht nachts.
//
//   node e2e/messung/f0170-nachholen-last.mjs <localchatA> <verlauf-erzeugen> <arbeitsordner> [szenario …]
//
// Szenarien: kontrolle, verlauf200, probe, z1, z5 (ohne Angabe alle). Je Szenario eine frische Instanz auf einem
// freien Port mit 100.000 Nachrichten in 10 Channels. Die 64 Schleifen laufen in einem eigenen Worker, damit ihre
// Last auf dem Client die Messung des Beobachters nicht verfälscht. Beobachter und Sender kommen von 127.0.0.3,
// weil eine Gegenstelle höchstens 64 Verbindungen hält und die 64 Schleifen 127.0.0.1 ausschöpfen.
import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import { fileURLToPath } from 'node:url';
import { kennwerte, wartezeitInMs } from './last-auswertung.mjs';

const ALTES_SEIT = '2000-01-01T00:00:00.000Z';
const NACHHOLEN = `api/warten?bis=0&seit=${ALTES_SEIT}&erwaehnung`;
const VERLAUF_50 = 'api/channels/allgemein/nachrichten?n=50';
const VERLAUF_200 = 'api/channels/allgemein/nachrichten?n=200';

if (isMainThread) {
  await hauptprogramm(process.argv.slice(2));
} else {
  parentPort.postMessage(await schleifenFahren(workerData));
}

async function hauptprogramm([programm, erzeuger, arbeitsordner, ...gewaehlt]) {
  const alle = { kontrolle, verlauf200, probe, z1, z5 };
  let namen = Object.keys(alle);
  if (gewaehlt.length > 0) {
    namen = gewaehlt;
  }
  for (const name of namen) {
    const umgebung = { programm, erzeuger, ordner: path.join(arbeitsordner, name) };
    const ergebnis = await alle[name](umgebung);
    console.log(JSON.stringify({ szenario: name, ...ergebnis }));
  }
}

// Kontrolle ohne Schleifen: Verlauf-Median unter 5 ms.
async function kontrolle(umgebung) {
  return mitInstanz(umgebung, async (instanz) => {
    const beobachter = await beitreten(instanz.adresse, 'beobachter', '127.0.0.3');
    return { verlauf: await beobachten(instanz.adresse, beobachter) };
  });
}

// Gleiche Last als Verlauf-Schleife n=200: Verlauf p90 unter 100 ms.
async function verlauf200(umgebung) {
  return unterLast(umgebung, { pfad: VERLAUF_200, retryAfterBeachten: true });
}

// Die Abnahme-Probe wörtlich (Schleifen beachten Retry-After) mit Z2, Z3 und Z4 im selben Lauf.
async function probe(umgebung) {
  return unterLast(umgebung, { pfad: NACHHOLEN, retryAfterBeachten: true });
}

// Z1: die Schleifen ignorieren Retry-After und rufen sofort wieder.
async function z1(umgebung) {
  return unterLast(umgebung, { pfad: NACHHOLEN, retryAfterBeachten: false });
}

// Z5: 2.000 Sätze zu je 16 KiB, die Erwähnung im letzten — ein Aufruf mit erwaehnung auf ruhiger Instanz.
async function z5(umgebung) {
  return mitInstanz({ ...umgebung, vorbereiten: grosseSaetzeSchreiben }, async (instanz) => {
    const agent = await beitreten(instanz.adresse, 'claude', '127.0.0.1');
    const dauern = [];
    for (let wiederholung = 0; wiederholung < 10; wiederholung += 1) {
      const antwort = await anfragen(instanz.adresse, { pfad: NACHHOLEN, token: agent });
      dauern.push(antwort.dauerInMs);
    }
    const stand = await gesundheit(instanz.adresse);
    return { aufruf: kennwerte(dauern), nachrichten: stand.nachrichten, defekte_zeilen: stand.defekte_zeilen, nachholen: stand.nachholen };
  });
}

async function unterLast(umgebung, { pfad, retryAfterBeachten }) {
  return mitInstanz(umgebung, async (instanz) => {
    const sitzungen = [];
    for (let nummer = 0; nummer < 16; nummer += 1) {
      sitzungen.push(await beitreten(instanz.adresse, `schleife${nummer}`, '127.0.0.1'));
    }
    const beobachter = await beitreten(instanz.adresse, 'beobachter', '127.0.0.3');
    const sender = await beitreten(instanz.adresse, 'sender', '127.0.0.3');
    const fernagent = await beitreten(instanz.adresse, 'fernagent', '127.0.0.2');
    const fernsender = await beitreten(instanz.adresse, 'fernsender', '127.0.0.2');
    const ende = Date.now() + 20_000;
    const schleifen = schleifenStarten({ adresse: instanz.adresse, sitzungen, pfad, retryAfterBeachten, ende });

    const [verlauf, senden, zustellung, gesundheitNach8s] = await Promise.all([
      beobachten(instanz.adresse, beobachter),
      vierzigMalSenden(instanz.adresse, sender),
      zwanzigMalZustellen(instanz.adresse, fernagent, fernsender, ende),
      warten(8_000).then(() => gesundheit(instanz.adresse)),
    ]);
    return {
      verlauf,
      z2: gesundheitNach8s.nachholen,
      z3: zustellung,
      z4: senden,
      schleifen: await schleifen,
    };
  });
}

// Beobachter: 40 × Verlauf n=50 im Abstand von 50 ms.
async function beobachten(adresse, token) {
  const dauern = [];
  for (let nummer = 0; nummer < 40; nummer += 1) {
    const antwort = await anfragen(adresse, { pfad: VERLAUF_50, token, herkunft: '127.0.0.3' });
    dauern.push(antwort.dauerInMs);
    await warten(50);
  }
  return kennwerte(dauern);
}

// Z4: 40 Sendungen im Abstand von 200 ms; gemessen bis zur 201. Das Schreibkonto der Kennung (Vorrat 30, 1 je
// Sekunde) bremst einige davon mit 429 — gezählt, nicht gemessen.
async function vierzigMalSenden(adresse, token) {
  const dauern = [];
  const status = {};
  for (let nummer = 0; nummer < 40; nummer += 1) {
    const antwort = await anfragen(adresse, {
      methode: 'POST', pfad: 'api/channels/allgemein/nachrichten', token, herkunft: '127.0.0.3', koerper: { text: `z4-${nummer}` },
    });
    status[antwort.status] = (status[antwort.status] || 0) + 1;
    if (antwort.status === 201) {
      dauern.push(antwort.dauerInMs);
    }
    await warten(200);
  }
  return { ...kennwerte(dauern), status };
}

// Z3: ein Agent von 127.0.0.2 wartet in Schleife mit laufendem seit; ein Sender von dort sendet 20 Nachrichten in 20 s.
// Der Agent holt sich vor dem ersten Senden einen Stand, sonst verpasst „ab jetzt“ eine Nachricht, die vor seinem
// ersten Aufruf ankommt. Gemessen von der 201 bis zur Ankunft (kommt die Nachricht vor der 201 an, zählt 0). Statt erwaehnung (B0742 noch
// nicht gebaut) wartet der Agent auf `channel=z3` — derselbe Weg über Reservierung, Nachholbank und Bus.
async function zwanzigMalZustellen(adresse, agent, sender, ende) {
  const zeit201 = new Map();
  const ankunft = new Map();
  const aufrufe = {};
  const anfang = await anfragen(adresse, { pfad: 'api/warten?bis=0&channel=z3', token: agent, herkunft: '127.0.0.2' });
  const empfangen = (async () => {
    let seit = anfang.daten.seit;
    while (ankunft.size < 20 && Date.now() < ende + 5_000) {
      let seitTeil = '';
      if (seit) {
        seitTeil = `&seit=${encodeURIComponent(seit)}`;
      }
      const antwort = await anfragen(adresse, { pfad: `api/warten?bis=5&channel=z3${seitTeil}`, token: agent, herkunft: '127.0.0.2' });
      const art = `${antwort.status} ${antwort.daten.ende || antwort.daten.grenze}`;
      aufrufe[art] = (aufrufe[art] || 0) + 1;
      if (antwort.status !== 200) {
        await warten(wartezeitInMs(antwort.retryAfter));
        continue;
      }
      seit = antwort.daten.seit;
      for (const eintrag of antwort.daten.nachrichten) {
        ankunft.set(eintrag.nachricht.text, antwort.empfangen);
      }
    }
  })();
  for (let nummer = 0; nummer < 20; nummer += 1) {
    const text = `z3-${nummer}`;
    const antwort = await anfragen(adresse, {
      methode: 'POST', pfad: 'api/channels/z3/nachrichten', token: sender, herkunft: '127.0.0.2', koerper: { text },
    });
    if (antwort.status === 201) {
      zeit201.set(text, antwort.empfangen);
    }
    await warten(1_000);
  }
  await empfangen;
  const dauern = [...zeit201].map(([text, gesendet]) => zustelldauer(gesendet, ankunft.get(text)));
  const fehlend = [...zeit201.keys()].filter((text) => !ankunft.has(text));
  return { ...kennwerte(dauern), gesendet: zeit201.size, angekommen: ankunft.size, fehlend, aufrufe };
}

function zustelldauer(gesendet, angekommen) {
  if (angekommen === undefined) {
    return Infinity;
  }
  return Math.max(0, angekommen - gesendet);
}

function schleifenStarten(auftrag) {
  return new Promise((erledigt, gescheitert) => {
    const worker = new Worker(fileURLToPath(import.meta.url), { workerData: auftrag });
    worker.once('message', erledigt);
    worker.once('error', gescheitert);
  });
}

// Im Worker: 16 Sitzungen × 4 Schleifen, jede sequentiell mit eigener Verbindung, bis `ende`.
async function schleifenFahren({ adresse, sitzungen, pfad, retryAfterBeachten, ende }) {
  const zaehler = { ok: 0, nachholen429: 0, warten429: 0, andere: 0 };
  const schleife = async (token) => {
    const agent = new http.Agent({ keepAlive: true, maxSockets: 1 });
    while (Date.now() < ende) {
      const antwort = await anfragen(adresse, { pfad, token, agent });
      zaehlen(zaehler, antwort);
      if (antwort.status === 429 && retryAfterBeachten) {
        await warten(Math.min(wartezeitInMs(antwort.retryAfter), ende - Date.now()));
      }
    }
    agent.destroy();
  };
  await Promise.all(sitzungen.flatMap((token) => [schleife(token), schleife(token), schleife(token), schleife(token)]));
  return zaehler;
}

function zaehlen(zaehler, antwort) {
  if (antwort.status === 200) {
    zaehler.ok += 1;
  } else if (antwort.status === 429 && antwort.daten.grenze === 'nachholen') {
    zaehler.nachholen429 += 1;
  } else if (antwort.status === 429) {
    zaehler.warten429 += 1;
  } else {
    zaehler.andere += 1;
  }
}

async function mitInstanz({ programm, erzeuger, ordner, vorbereiten = hunderttausendNachrichten }, messen) {
  fs.rmSync(ordner, { recursive: true, force: true });
  fs.mkdirSync(path.join(ordner, 'channels', 'z3'), { recursive: true });
  vorbereiten(ordner, erzeuger);
  const instanz = await starten(programm, ordner);
  try {
    return await messen(instanz);
  } finally {
    instanz.prozess.kill('SIGTERM');
    fs.rmSync(ordner, { recursive: true, force: true });
  }
}

// 100.000 Nachrichten in 10 Channels (allgemein, c1 … c9), je 10.000 über einen Tag.
function hunderttausendNachrichten(ordner, erzeuger) {
  const channels = ['allgemein', ...Array.from({ length: 9 }, (_, index) => `c${index + 1}`)];
  for (const channel of channels) {
    execFileSync(erzeuger, [ordner, channel, '10000', '1'], { stdio: 'ignore' });
  }
}

// Z5: die Kopfzeilen von 2.000 erzeugten Sätzen, je mit 16 KiB Text in einer Zeile; der letzte erwähnt claude.
function grosseSaetzeSchreiben(ordner, erzeuger) {
  const muster = path.join(ordner, 'muster');
  execFileSync(erzeuger, [muster, 'allgemein', '2000', '1'], { stdio: 'ignore' });
  const tagesdatei = fs.readdirSync(path.join(muster, 'channels', 'allgemein'))[0];
  const kopfzeilen = fs.readFileSync(path.join(muster, 'channels', 'allgemein', tagesdatei), 'utf8')
    .split('\n')
    .filter((zeile) => zeile.startsWith('@ '));
  const fuellung = 'Befund Scan läuft '.repeat(1024).slice(0, 16 * 1024);
  const saetze = kopfzeilen.map((kopf, index) => {
    let erwaehnung = '';
    if (index === kopfzeilen.length - 1) {
      erwaehnung = '@claude ';
    }
    return `${kopf}\n  ${erwaehnung}${fuellung}\n\n`;
  });
  fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true });
  fs.writeFileSync(path.join(ordner, 'channels', 'allgemein', tagesdatei), saetze.join(''));
  fs.rmSync(muster, { recursive: true, force: true });
}

function starten(programm, ordner) {
  return new Promise((bereit, gescheitert) => {
    const prozess = spawn(programm, [ordner, '--port', '0', '--host', '127.0.0.1', '--kein-browser'], {
      env: { ...process.env, TZ: 'Europe/Berlin' },
      stdio: ['ignore', 'pipe', 'inherit'],
    });
    let ausgabe = '';
    prozess.stdout.on('data', (stueck) => {
      ausgabe += stueck;
      const treffer = ausgabe.match(/unter (http:\/\/[^\s]+\/)/);
      if (treffer) {
        bereit({ prozess, adresse: treffer[1] });
      }
    });
    prozess.once('exit', (code) => gescheitert(new Error(`Instanz endete vor der Startzeile mit ${code}`)));
  });
}

async function beitreten(adresse, name, herkunft) {
  const antwort = await anfragen(adresse, { methode: 'POST', pfad: 'api/beitreten', herkunft, koerper: { name, art: 'agent' } });
  if (antwort.status !== 201) {
    throw new Error(`Beitritt ${name}: ${antwort.status} ${JSON.stringify(antwort.daten)}`);
  }
  return antwort.daten.token;
}

async function gesundheit(adresse) {
  return (await anfragen(adresse, { pfad: 'api/gesundheit' })).daten;
}

// Eine Anfrage; liefert Status, Daten, Retry-After, die Dauer bis zur vollständigen Antwort und deren Augenblick.
function anfragen(adresse, { methode = 'GET', pfad, token, herkunft = '127.0.0.1', koerper, agent }) {
  const ziel = new URL(pfad, adresse);
  const kopf = {};
  if (token) {
    kopf['X-LocalchatA-Sitzung'] = token;
  }
  let inhalt;
  if (koerper) {
    inhalt = JSON.stringify(koerper);
    kopf['Content-Type'] = 'application/json';
  }
  return new Promise((erledigt, gescheitert) => {
    const beginn = performance.now();
    const anfrage = http.request({
      host: ziel.hostname, port: ziel.port, path: `${ziel.pathname}${ziel.search}`, method: methode,
      localAddress: herkunft, headers: kopf, agent,
    }, (antwort) => {
      let text = '';
      antwort.setEncoding('utf8');
      antwort.on('data', (stueck) => { text += stueck; });
      antwort.on('end', () => {
        const empfangen = performance.now();
        erledigt({
          status: antwort.statusCode,
          daten: datenLesen(text),
          retryAfter: antwort.headers['retry-after'],
          dauerInMs: empfangen - beginn,
          empfangen,
        });
      });
    });
    anfrage.once('error', gescheitert);
    anfrage.end(inhalt);
  });
}

function datenLesen(text) {
  try {
    return JSON.parse(text);
  } catch {
    return {};
  }
}

function warten(ms) {
  return new Promise((weiter) => setTimeout(weiter, Math.max(ms, 0)));
}
