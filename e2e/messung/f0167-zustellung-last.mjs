'use strict';
// Messung F0167 „Zustellung unter Last“ (R00049 US-23; ADR-0054 QW2, W9; B0746). Messwert im Entwicklertest, kein
// Gate; Release-Build, nicht nachts.
//
//   node e2e/messung/f0167-zustellung-last.mjs <localchatA> <arbeitsordner>
//
// Lastprofil Team-LAN: 50 Agenten (je eine Kennung, alle von 127.0.0.1) warten mit erwaehnung in Schleife mit ihrem
// seit, 20 Menschen halten den Ereignisstrom offen, 30 Nachrichten/s über 60 s von 40 Sendern auf 5 Herkünften (das
// Schreibkonto gilt wie im Betrieb), jede zehnte erwähnt einen zufälligen Agenten. Gemessen wird die Zustellung von der
// 201 bis zum Ende der Antwort des Agenten; wartend aus der Gesundheit alle 10 s.
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { kennwerte } from './last-auswertung.mjs';
import { anfragen, beitreten, gesundheit, starten, stromOeffnen, warten } from './last-instanz.mjs';

const [programm, arbeitsordner] = process.argv.slice(2);
const ergebnis = await zustellung({ programm, ordner: path.join(arbeitsordner, 'zustellung') });
console.log(JSON.stringify({ szenario: 'zustellung', ...ergebnis }));

// B0746 (US-23): 50 Agenten warten, 20 Ströme offen, 30 Nachrichten/s über 60 s, jede zehnte mit Erwähnung.
async function zustellung({ programm, ordner }) {
  return mitInstanz({ programm, ordner }, async (instanz) => {
    const agenten = [];
    for (let nummer = 0; nummer < 50; nummer += 1) {
      agenten.push({ name: `agent${nummer}`, token: await beitreten(instanz.adresse, `agent${nummer}`, '127.0.0.1') });
    }
    const menschen = [];
    for (let nummer = 0; nummer < 20; nummer += 1) {
      const token = await beitreten(instanz.adresse, `mensch${nummer}`, '127.0.0.20', 'mensch');
      const strom = stromOeffnen(instanz.adresse, token, '127.0.0.20');
      await strom.offen;
      menschen.push(strom);
    }
    const sender = [];
    for (let herkunft = 11; herkunft <= 15; herkunft += 1) {
      for (let nummer = 0; nummer < 8; nummer += 1) {
        const name = `sender${herkunft}-${nummer}`;
        sender.push({ herkunft: `127.0.0.${herkunft}`, token: await beitreten(instanz.adresse, name, `127.0.0.${herkunft}`, 'mensch') });
      }
    }

    const ankunft = new Map();
    let laeuft = true;
    const schleifen = agenten.map((agent) => agentWartet(instanz.adresse, agent, ankunft, () => laeuft));
    await warten(500);
    const [gesendet, wartend] = await Promise.all([
      dreissigJeSekunde(instanz.adresse, sender, agenten, 60_000),
      wartendAbfragen(instanz.adresse, 6, 10_000),
    ]);
    await warten(2_500);
    laeuft = false;
    const zaehlung = await Promise.all(schleifen);
    for (const strom of menschen) {
      strom.schliessen();
    }

    const dauern = [...gesendet.erwaehnungen].map(([text, { zeit201 }]) => zustelldauer(zeit201, ankunft.get(text)));
    const abAbsenden = [...gesendet.erwaehnungen].map(([text, { abgeschickt }]) => zustelldauer(abgeschickt, ankunft.get(text)));
    const fehlend = [...gesendet.erwaehnungen.keys()].filter((text) => !ankunft.has(text));
    return {
      zustellung: kennwerte(dauern),
      ab_absenden: kennwerte(abAbsenden),
      erwaehnungen: gesendet.erwaehnungen.size,
      angekommen: dauern.length - fehlend.length,
      fehlend,
      gesendet: gesendet.status,
      sendetakt_s: gesendet.dauerInS,
      wartend_alle_10s: wartend,
      aufrufe: zaehlung.reduce((summe, teil) => summe + teil, 0),
    };
  });
}

// Ein Agent: warten?bis=3&erwaehnung in Schleife mit seinem seit, bis `laeuft` endet; merkt jede Ankunft.
async function agentWartet(adresse, agent, ankunft, laeuft) {
  const verbindung = new http.Agent({ keepAlive: true, maxSockets: 1 });
  const anfang = await anfragen(adresse, { pfad: 'api/warten?bis=0&erwaehnung', token: agent.token, agent: verbindung });
  let seit = anfang.daten.seit;
  let aufrufe = 0;
  while (laeuft()) {
    let seitTeil = '';
    if (seit) {
      seitTeil = `&seit=${encodeURIComponent(seit)}`;
    }
    const antwort = await anfragen(adresse, { pfad: `api/warten?bis=3&erwaehnung${seitTeil}`, token: agent.token, agent: verbindung });
    aufrufe += 1;
    if (antwort.status !== 200) {
      throw new Error(`${agent.name}: ${antwort.status} ${JSON.stringify(antwort.daten)}`);
    }
    seit = antwort.daten.seit;
    for (const eintrag of antwort.daten.nachrichten) {
      ankunft.set(eintrag.nachricht.text, antwort.empfangen);
    }
  }
  verbindung.destroy();
  return aufrufe;
}

// Sendet `dauerInMs` lang 30 Nachrichten je Sekunde (alle 33,3 ms eine, nicht auf Antworten wartend), reihum über
// die Sender; jede zehnte erwähnt einen zufälligen Agenten. Liefert je Erwähnung den Augenblick des Absendens und der
// 201 (kommt die Antwort des Agenten vor der 201 an, zählt die Zustellung ab der 201 als 0 — darum beide).
async function dreissigJeSekunde(adresse, sender, agenten, dauerInMs) {
  const erwaehnungen = new Map();
  const status = {};
  const offen = [];
  const beginn = performance.now();
  let nummer = 0;
  while (performance.now() - beginn < dauerInMs) {
    const soll = beginn + (nummer * 1000) / 30;
    await warten(soll - performance.now());
    const von = sender[nummer % sender.length];
    let text = `last-${nummer}`;
    const erwaehnt = nummer % 10 === 0;
    if (erwaehnt) {
      const agent = agenten[Math.floor(Math.random() * agenten.length)];
      text = `@${agent.name} last-${nummer}`;
    }
    const channel = ['allgemein', 'technik'][nummer % 2];
    const abgeschickt = performance.now();
    offen.push(anfragen(adresse, {
      methode: 'POST', pfad: `api/channels/${channel}/nachrichten`, token: von.token, herkunft: von.herkunft, koerper: { text },
    }).then((antwort) => {
      status[antwort.status] = (status[antwort.status] || 0) + 1;
      if (erwaehnt && antwort.status === 201) {
        erwaehnungen.set(text, { zeit201: antwort.empfangen, abgeschickt });
      }
    }));
    nummer += 1;
  }
  await Promise.all(offen);
  return { erwaehnungen, status, dauerInS: (performance.now() - beginn) / 1000 };
}

// `wartend` aus der Gesundheit, `anzahl` mal im Abstand `abstandInMs`, beginnend nach dem ersten Abstand.
async function wartendAbfragen(adresse, anzahl, abstandInMs) {
  const werte = [];
  for (let nummer = 0; nummer < anzahl; nummer += 1) {
    await warten(abstandInMs - 50);
    werte.push((await gesundheit(adresse)).wartend);
  }
  return werte;
}

function zustelldauer(gesendet, angekommen) {
  if (angekommen === undefined) {
    return Infinity;
  }
  return Math.max(0, angekommen - gesendet);
}

async function mitInstanz({ programm, ordner }, messen) {
  fs.rmSync(ordner, { recursive: true, force: true });
  for (const channel of ['allgemein', 'technik']) {
    fs.mkdirSync(path.join(ordner, 'channels', channel), { recursive: true });
  }
  const instanz = await starten(programm, ordner);
  try {
    return await messen(instanz);
  } finally {
    instanz.prozess.kill('SIGTERM');
    fs.rmSync(ordner, { recursive: true, force: true });
  }
}
