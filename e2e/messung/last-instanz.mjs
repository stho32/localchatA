'use strict';
// Gemeinsame Werkzeuge der Lastgeneratoren (R00049 F0167, F0170): Instanz auf freiem Port starten, beitreten,
// Gesundheit lesen, Anfragen von einer festen Herkunft mit gemessener Dauer, offener Ereignisstrom.
import { spawn } from 'node:child_process';
import http from 'node:http';

// Startet die Instanz auf einem freien Port; liefert Prozess und Adresse aus der Startzeile.
export function starten(programm, ordner) {
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

export async function beitreten(adresse, name, herkunft, art = 'agent') {
  const antwort = await anfragen(adresse, { methode: 'POST', pfad: 'api/beitreten', herkunft, koerper: { name, art } });
  if (antwort.status !== 201) {
    throw new Error(`Beitritt ${name}: ${antwort.status} ${JSON.stringify(antwort.daten)}`);
  }
  return antwort.daten.token;
}

export async function gesundheit(adresse) {
  return (await anfragen(adresse, { pfad: 'api/gesundheit' })).daten;
}

// Eine Anfrage; liefert Status, Daten, Retry-After, die Dauer bis zur vollständigen Antwort und deren Augenblick.
export function anfragen(adresse, { methode = 'GET', pfad, token, herkunft = '127.0.0.1', koerper, agent }) {
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

export function warten(ms) {
  return new Promise((weiter) => setTimeout(weiter, Math.max(ms, 0)));
}

// Ein offener GET /api/ereignisse: merkt jedes Ereignis mit seinem Augenblick; `naechstes` wartet auf das erste ab
// einer Stelle, das passt (Infinity nach `hoechstensInMs`).
export function stromOeffnen(adresse, token, herkunft) {
  const ziel = new URL('api/ereignisse', adresse);
  const ereignisse = [];
  const wartende = new Set();
  let anfrage;
  const offen = new Promise((bereit, gescheitert) => {
    anfrage = http.get({
      host: ziel.hostname, port: ziel.port, path: ziel.pathname, localAddress: herkunft,
      headers: { 'X-LocalchatA-Sitzung': token },
    }, (antwort) => {
      if (antwort.statusCode !== 200) {
        gescheitert(new Error(`Strom: ${antwort.statusCode}`));
        return;
      }
      bereit();
      let rest = '';
      antwort.setEncoding('utf8');
      antwort.on('data', (stueck) => {
        rest += stueck;
        let stelle = rest.indexOf('\n\n');
        while (stelle >= 0) {
          ereignisMerken(rest.slice(0, stelle), ereignisse, wartende);
          rest = rest.slice(stelle + 2);
          stelle = rest.indexOf('\n\n');
        }
      });
    });
    anfrage.once('error', gescheitert);
  });
  return {
    offen,
    schliessen: () => anfrage.destroy(),
    stand: () => ereignisse.length,
    naechstes: (ab, passt, hoechstensInMs) => {
      const schon = ereignisse.slice(ab).find((eintrag) => passt(eintrag.ereignis));
      if (schon) {
        return Promise.resolve(schon.zeit);
      }
      return new Promise((erledigt) => {
        const warter = { passt, erledigt };
        wartende.add(warter);
        setTimeout(() => {
          wartende.delete(warter);
          erledigt(Infinity);
        }, hoechstensInMs);
      });
    },
  };
}

function ereignisMerken(block, ereignisse, wartende) {
  const zeit = performance.now();
  const daten = block.split('\n').filter((zeile) => zeile.startsWith('data:')).map((zeile) => zeile.slice(5).trim()).join('');
  let ereignis;
  try {
    ereignis = JSON.parse(daten);
  } catch {
    return;
  }
  ereignisse.push({ ereignis, zeit });
  for (const warter of wartende) {
    if (warter.passt(ereignis)) {
      wartende.delete(warter);
      warter.erledigt(zeit);
    }
  }
}
