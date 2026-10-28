'use strict';
// Startet localchatA in einem Wegwerf-Ordner und liest die Adresse aus der Startzeile; Port 0 oder ein
// fester freier Port, damit ein Neustart denselben Ursprung behält; Host und weitere Argumente wählbar.
const { spawn } = require('child_process');
const http = require('http');
const dgram = require('dgram');
const fs = require('fs');
const net = require('net');
const os = require('os');
const path = require('path');

const PROGRAMM = path.resolve(__dirname, '../../target/debug/localchatA');
process.env.TZ = 'Europe/Berlin';

async function instanzStarten({ port = 0, host = '127.0.0.1', argumente = [], umgebung = {} } = {}) {
  const wurzel = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-'));
  const ordner = path.join(wurzel, 'chat');
  let kind = null;
  const instanz = {
    ordner,
    adresse: '',
    starten: async () => {
      kind = spawn(PROGRAMM, [ordner, '--port', String(port), '--host', host, '--kein-browser', ...argumente], {
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

/// Die Adresse, über die dieser Rechner im LAN erreichbar ist (UDP-connect sendet nichts).
function lanAdresse() {
  return new Promise((gefunden, gescheitert) => {
    const sonde = dgram.createSocket('udp4');
    sonde.once('error', gescheitert);
    sonde.connect(9, '192.0.2.1', () => {
      const { address } = sonde.address();
      sonde.close();
      gefunden(address);
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

/// Antworten, die R00003 und R00004 selbst vorschreiben (401 auf den Strom nach einem Neustart): Chromium meldet jede Fetch-Antwort ab 400 als Konsolenzeile
/// „Failed to load resource“ — sie sind Ablauf, kein Fehler der Seite.
const VORGESCHRIEBENE_ANTWORTEN = { '/api/ich': [401], '/api/beitreten': [409, 422], '/api/ereignisse': [401] };

/// Sammelt, was im Browser schiefgeht: Konsolenfehler (auch CSP-Verstöße) und Ausnahmen. Ausgenommen sind
/// nur die vorgeschriebenen API-Antworten und — wenn der Test den Server selbst beendet — die abgewiesene
/// Verbindung zur API.
/// Mit `streng` gilt keine Ausnahme: jede Konsolenzeile auf Stufe error zählt (AK F0013 „ohne einen einzigen
/// Konsolenfehler“, auch nach einem Neustart).
function fehlerSammeln(page, { serverWegErwartet = false, streng = false } = {}) {
  const fehler = [];
  page.on('console', (meldung) => {
    if (meldung.type() !== 'error' || (!streng && vorgeschrieben(meldung, serverWegErwartet))) {
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

/// Tritt per API bei — von einer wählbaren Absenderadresse (Linux-Loopback 127.0.0.x); liefert Status und JSON.
function beitretenPerApi(adresse, name, herkunft = '127.0.0.1') {
  const koerper = JSON.stringify({ name });
  const ziel = new URL('api/beitreten', adresse);
  return new Promise((fertig, gescheitert) => {
    const anfrage = http.request({
      host: ziel.hostname, port: ziel.port, path: ziel.pathname, method: 'POST', localAddress: herkunft,
      headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(koerper) },
    }, (antwort) => {
      let text = '';
      antwort.on('data', (teil) => { text += teil; });
      antwort.on('end', () => fertig({ status: antwort.statusCode, daten: JSON.parse(text) }));
    });
    anfrage.once('error', gescheitert);
    anfrage.end(koerper);
  });
}

/// Rahmen und Kontrast des fokussierten Elements gegen die Fläche daneben (WCAG 2.x), im Browser gerechnet.
function fokusImBrowser() {
  function anteile(farbe) {
    return farbe.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number);
  }
  function leuchtdichte(farbe) {
    const [r, g, b] = anteile(farbe).map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }
  function flaecheUnter(element) {
    for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
      const farbe = getComputedStyle(knoten).backgroundColor;
      if (farbe !== 'rgba(0, 0, 0, 0)' && farbe !== 'transparent') {
        return farbe;
      }
    }
    return 'rgb(255, 255, 255)';
  }
  const fokussiert = document.activeElement;
  const stil = getComputedStyle(fokussiert);
  let angrenzend = flaecheUnter(fokussiert);
  if (parseFloat(stil.outlineOffset) > 0) {
    angrenzend = flaecheUnter(fokussiert.parentElement);
  }
  const hell = Math.max(leuchtdichte(stil.outlineColor), leuchtdichte(angrenzend));
  const dunkel = Math.min(leuchtdichte(stil.outlineColor), leuchtdichte(angrenzend));
  return {
    beschriftung: fokussiert.getAttribute('aria-label') || fokussiert.textContent,
    fokusSichtbar: fokussiert.matches(':focus-visible'),
    rahmen: `${stil.outlineStyle} ${stil.outlineWidth}`,
    kontrast: (hell + 0.05) / (dunkel + 0.05),
    obenImFenster: fokussiert.getBoundingClientRect().top,
  };
}

/// Hält einen Ereignisstrom einer per API beigetretenen Sitzung offen (Agent mit Strom); `schliessen()` beendet ihn.
function stromOeffnen(adresse, token, herkunft = '127.0.0.1') {
  const ziel = new URL('api/ereignisse', adresse);
  return new Promise((offen, gescheitert) => {
    const anfrage = http.get({
      host: ziel.hostname, port: ziel.port, path: ziel.pathname, localAddress: herkunft,
      headers: { 'X-LocalchatA-Sitzung': token },
    }, (antwort) => {
      antwort.on('data', () => {});
      offen({ schliessen: () => anfrage.destroy() });
    });
    anfrage.once('error', gescheitert);
  });
}

/// Wartet ereignisgenau, bis `pruefung(argument)` im Browser wahr ist — geprüft vor jedem Bildaufbau
/// (requestAnimationFrame), nicht im Pollraster von expect (0,1 / 0,25 / 0,5 / 1 s, bis zu 1 s zu spät) —, und
/// liefert die Millisekunden seit `ab`, dem Ende des auslösenden Schritts. `pruefung` läuft im Browser und
/// darf nur `argument` und das DOM benutzen.
async function sichtbarNach(seite, pruefung, { argument, ab, timeout = 5000 }) {
  await seite.waitForFunction(pruefung, argument, { polling: 'raf', timeout });
  return Date.now() - ab;
}

/// Wie sichtbarNach, liefert zusätzlich, was `pruefung` im Augenblick des Erfüllens zurückgab (etwa den gezeigten
/// Text): { ms, wert }.
async function sichtbarMitWertNach(seite, pruefung, { argument, ab, timeout = 5000 }) {
  const ergebnis = await seite.waitForFunction(pruefung, argument, { polling: 'raf', timeout });
  const ms = Date.now() - ab;
  return { ms, wert: await ergebnis.jsonValue() };
}

/// „Nicht mehr online“ (R00005, Änderungen gegenüber R00004): nach dem Schließen ○ „offline“ oder — mit der
/// Restzeit — gestrichelt „weg in m:ss“; für toHaveText.
const NICHT_MEHR_ONLINE = /^(offline|weg in \d:\d\d)$/;

/// Prüfung für sichtbarNach: die Zeile mit `kennung` steht nicht mehr online (○ „offline“ oder „weg in m:ss“).
function zeigtNichtMehrOnline({ kennung }) {
  const zeile = [...document.querySelectorAll('#lc-teilnehmer .lc-person')]
    .find((person) => person.querySelector('.lc-id').textContent.includes(kennung));
  if (zeile === undefined) {
    return false;
  }
  const anzeige = zeile.querySelector('.lc-status');
  return anzeige !== null && /^(offline|weg in \d:\d\d)$/.test(anzeige.textContent.trim());
}

module.exports = {
  instanzStarten, freierPort, lanAdresse, fehlerSammeln, beitretenPerApi, fokusImBrowser, stromOeffnen, sichtbarNach,
  sichtbarMitWertNach, zeigtNichtMehrOnline, NICHT_MEHR_ONLINE,
};
