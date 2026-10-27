'use strict';
// Startet localchatA auf Port 0 in einem Wegwerf-Ordner und liest die Adresse aus der Startzeile.
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const PROGRAMM = path.resolve(__dirname, '../../target/debug/localchatA');
process.env.TZ = 'Europe/Berlin';

async function instanzStarten() {
  const wurzel = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-'));
  const ordner = path.join(wurzel, 'chat');
  const kind = spawn(PROGRAMM, [ordner, '--port', '0', '--host', '127.0.0.1', '--kein-browser'], {
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  const adresse = await adresseAusDerStartzeile(kind);
  return {
    ordner,
    adresse,
    stoppen: async () => {
      await beenden(kind);
      fs.rmSync(wurzel, { recursive: true, force: true });
    },
  };
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
    if (kind.exitCode !== null) {
      beendet();
      return;
    }
    kind.once('exit', beendet);
    kind.kill('SIGTERM');
  });
}

module.exports = { instanzStarten };
