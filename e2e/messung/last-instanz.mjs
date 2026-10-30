'use strict';
// Gemeinsame Werkzeuge der Lastgeneratoren (R00049 F0167, F0170): Instanz auf freiem Port starten, beitreten,
// Gesundheit lesen, Anfragen von einer festen Herkunft mit gemessener Dauer.
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
