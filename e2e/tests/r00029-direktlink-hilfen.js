'use strict';
// Hilfen der Direktlink-Specs (R00029 F0100, Karte #71; ADR-0014 Ergänzung „Karte #71 (Chrome 154)“): eine Instanz
// mit `allgemein`, Anhänge über die API ablegen und senden, Medien aus ffmpeg und kleine Testserver auf freiem Port
// für Kontrollpaare und Mutanten. Die CSP-Werte stehen als Literal hier, nie aus dem Programm (C1).
const fs = require('fs');
const http = require('http');
const path = require('path');
const { execFileSync } = require('child_process');
const { expect } = require('./rahmen');
const { instanzStarten } = require('./hilfen');

const ANHANG_CSP = "sandbox; default-src 'none'";
const ANHANG_CSP_MEDIUM = "sandbox allow-same-origin; default-src 'none'; media-src 'self'";

function mitAllgemein() {
  return instanzStarten({ vorbereiten: (ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }) });
}

/// Je Anfrage eine eigene Verbindung (`agent: false`): ffmpeg läuft synchron zwischen den Anfragen, und ein Socket aus
/// dem Keep-Alive-Vorrat, den die Instanz nach 10 s Leerlauf geschlossen hat, endete sonst in `socket hang up`.
function anfrage(adresse, { methode, pfad, kopf = {}, koerper = Buffer.alloc(0) }) {
  const ziel = new URL(pfad, adresse);
  return new Promise((fertig, gescheitert) => {
    const laufend = http.request({
      host: ziel.hostname,
      port: ziel.port,
      path: ziel.pathname,
      method: methode,
      agent: false,
      localAddress: '127.0.0.1',
      headers: { ...kopf, 'Content-Length': koerper.length },
    }, (antwort) => {
      const teile = [];
      antwort.on('data', (teil) => teile.push(teil));
      antwort.on('end', () => {
        const text = Buffer.concat(teile).toString('utf8');
        fertig({ status: antwort.statusCode, daten: text.startsWith('{') ? JSON.parse(text) : text });
      });
    });
    laufend.once('error', (fehler) => gescheitert(new Error(`${methode} ${pfad}: ${fehler.message}`)));
    laufend.end(koerper);
  });
}

/// Anmelden, hochladen und als Nachricht ohne Text senden; liefert die ID.
async function ablegen(adresse, token, name, inhalt) {
  const json = { 'Content-Type': 'application/json', 'X-LocalchatA-Sitzung': token };
  const angemeldet = await anfrage(adresse, {
    methode: 'POST', pfad: 'api/anhaenge', kopf: json, koerper: Buffer.from(JSON.stringify({ name, bytes: inhalt.length })),
  });
  expect(angemeldet.status, JSON.stringify(angemeldet.daten)).toBe(201);
  const { id } = angemeldet.daten.anhang;
  const hochgeladen = await anfrage(adresse, {
    methode: 'PUT',
    pfad: `api/anhaenge/${id}`,
    kopf: { 'Content-Type': 'application/octet-stream', 'X-LocalchatA-Sitzung': token },
    koerper: inhalt,
  });
  expect(hochgeladen.status, JSON.stringify(hochgeladen.daten)).toBe(200);
  const gesendet = await anfrage(adresse, {
    methode: 'POST',
    pfad: 'api/channels/allgemein/nachrichten',
    kopf: json,
    koerper: Buffer.from(JSON.stringify({ text: '', anhaenge: [id] })),
  });
  expect(gesendet.status, JSON.stringify(gesendet.daten)).toBe(201);
  return id;
}

/// Eine Datei aus ffmpeg, 20 s Testbild und Ton (Namen `ton.*`: nur Ton); `kodierung` kann mit `-t` kürzen.
function medium(werkstatt, name, kodierung) {
  const pfad = path.join(werkstatt, name);
  const bild = name.startsWith('ton.') ? [] : ['-f', 'lavfi', '-i', 'testsrc=d=20:s=320x240:r=25'];
  execFileSync('ffmpeg', ['-v', 'error', '-y', ...bild, '-f', 'lavfi', '-i', 'sine=d=20', '-shortest', ...kodierung, pfad]);
  return fs.readFileSync(pfad);
}

/// Ein Testserver auf freiem Port; `beantworten(anfrage, antwort)` schreibt die Antwort.
function testserverStarten(host, beantworten) {
  return new Promise((fertig) => {
    const server = http.createServer(beantworten);
    server.listen(0, host, () => fertig(server));
  });
}

function testserverStoppen(server) {
  server.closeAllConnections();
  return new Promise((fertig) => server.close(fertig));
}

module.exports = { ANHANG_CSP, ANHANG_CSP_MEDIUM, mitAllgemein, ablegen, medium, testserverStarten, testserverStoppen };
