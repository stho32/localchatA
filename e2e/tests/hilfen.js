'use strict';
// Startet localchatA in einem Wegwerf-Ordner und liest die Adresse aus der Startzeile; Port 0 oder ein
// fester freier Port, damit ein Neustart denselben Ursprung behält.
const { spawn } = require('child_process');
const fs = require('fs');
const net = require('net');
const os = require('os');
const path = require('path');

const PROGRAMM = path.resolve(__dirname, '../../target/debug/localchatA');
process.env.TZ = 'Europe/Berlin';

async function instanzStarten({ port = 0, umgebung = {} } = {}) {
  const wurzel = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-'));
  const ordner = path.join(wurzel, 'chat');
  let kind = null;
  const instanz = {
    ordner,
    adresse: '',
    starten: async () => {
      kind = spawn(PROGRAMM, [ordner, '--port', String(port), '--host', '127.0.0.1', '--kein-browser'], {
        stdio: ['ignore', 'pipe', 'inherit'],
        env: { ...process.env, ...umgebung },
      });
      instanz.adresse = await adresseAusDerStartzeile(kind);
    },
    beenden: async () => {
      await beenden(kind);
    },
    stoppen: async () => {
      await beenden(kind);
      fs.rmSync(wurzel, { recursive: true, force: true });
    },
  };
  await instanz.starten();
  return instanz;
}

/// Ein Port, den das System gerade frei gibt — für Tests, die nach einem Neustart denselben Ursprung brauchen.
function freierPort() {
  return new Promise((gefunden, gescheitert) => {
    const server = net.createServer();
    server.once('error', gescheitert);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close(() => gefunden(port));
    });
  });
}

function adresseAusDerStartzeile(kind) {
  return new Promise((gefunden, gescheitert) => {
    let ausgabe = '';
    kind.stdout.on('data', (teil) => {
      ausgabe += teil;
      const treffer = /unter (http:\/\/\S+)/.exec(ausgabe);
      if (treffer) {
        gefunden(treffer[1]);
      }
    });
    kind.once('exit', (code) => gescheitert(new Error('localchatA beendet mit ' + code)));
  });
}

function beenden(kind) {
  return new Promise((beendet) => {
    if (kind === null || kind.exitCode !== null) {
      beendet();
      return;
    }
    kind.once('exit', beendet);
    kind.kill('SIGTERM');
  });
}

/// Antworten, die R00003 selbst vorschreibt: Chromium meldet jede Fetch-Antwort ab 400 als Konsolenzeile
/// „Failed to load resource“ — sie sind Ablauf, kein Fehler der Seite.
const VORGESCHRIEBENE_ANTWORTEN = { '/api/ich': [401], '/api/beitreten': [409, 422] };

/// Sammelt, was im Browser schiefgeht: Konsolenfehler (auch CSP-Verstöße) und Ausnahmen. Ausgenommen sind
/// nur die vorgeschriebenen API-Antworten und — wenn der Test den Server selbst beendet — die abgewiesene
/// Verbindung zur API.
function fehlerSammeln(page, { serverWegErwartet = false } = {}) {
  const fehler = [];
  page.on('console', (meldung) => {
    if (meldung.type() !== 'error' || vorgeschrieben(meldung, serverWegErwartet)) {
      return;
    }
    fehler.push('Konsole: ' + meldung.text() + ' (' + meldung.location().url + ')');
  });
  page.on('pageerror', (ausnahme) => fehler.push('Ausnahme: ' + ausnahme.message));
  return fehler;
}

function vorgeschrieben(meldung, serverWegErwartet) {
  const text = meldung.text();
  if (!text.startsWith('Failed to load resource: ')) {
    return false;
  }
  const pfad = new URL(meldung.location().url).pathname;
  const status = /status of (\d{3})/.exec(text);
  if (status !== null) {
    const erlaubt = VORGESCHRIEBENE_ANTWORTEN[pfad] || [];
    return erlaubt.includes(Number(status[1]));
  }
  return serverWegErwartet && pfad.startsWith('/api/') && text.includes('net::ERR_CONNECTION_REFUSED');
}

module.exports = { instanzStarten, freierPort, fehlerSammeln };
