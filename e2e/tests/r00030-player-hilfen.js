'use strict';
// Hilfen der Specs zu R00030 F0165 „Video und Audio im Player“: den Player eines Anhangs finden, stumm abspielen (ohne
// Geste spielt Chromium nur stumm von selbst) und seinen Zustand lesen; Anfragen an /anhaenge/ mitschreiben.
const path = require('path');
const { execFileSync } = require('child_process');
const { expect } = require('./rahmen');
const { JETZT, senden } = require('./senden-hilfen');

/// Der Player des Anhangs `name` in der Nachricht.
const player = (knoten, name) => knoten.locator(`video[aria-label="${name}"], audio[aria-label="${name}"]`);

async function abspielen(medium) {
  await medium.evaluate((element) => {
    element.muted = true;
    return element.play();
  });
  await expect.poll(() => wiedergabe(medium).then(({ zeit }) => zeit), { timeout: 15000 }).toBeGreaterThan(0.3);
}

function wiedergabe(medium) {
  return medium.evaluate((element) => ({
    pausiert: element.paused, zeit: element.currentTime, dauer: element.duration, fehler: element.error && element.error.code,
  }));
}

/// Merkt sich das Element unter `marke` im Fenster der Seite — „dasselbe Element“ heißt dann: `marke` zeigt auf den Knoten
/// im Verlauf.
function elementMerken(medium, marke) {
  return medium.evaluate((element, schluessel) => {
    window[schluessel] = element;
  }, marke);
}

function istDasselbe(medium, marke) {
  return medium.evaluate((element, schluessel) => window[schluessel] === element, marke);
}

/// Schreibt die Anfragen an /anhaenge/ im Kontext der Seite mit: { url, bereich, status }.
function anhangAnfragenMitschreiben(page) {
  const anfragen = [];
  page.context().on('request', (anfrage) => {
    if (new URL(anfrage.url()).pathname.startsWith('/anhaenge/')) {
      anfragen.push({ url: anfrage.url(), bereich: anfrage.headers().range, status: null });
    }
  });
  page.context().on('response', (antwort) => {
    const eintrag = anfragen.find(({ url, status }) => url === antwort.url() && status === null);
    if (eintrag !== undefined) {
      eintrag.status = antwort.status();
    }
  });
  return anfragen;
}

/// `vorbereiten` für anzeigeStarten: `anzahl` ältere Nachrichten in `allgemein` (Testdaten-Erzeuger aus R00007, bis zur
/// Uhr der Instanz) — der Verlauf ist dann höher als das Fenster und steht beim Öffnen am Ende.
function aeltereNachrichten(anzahl) {
  const erzeuger = path.resolve(__dirname, '../../target/debug/examples/verlauf-erzeugen');
  return (ordner) => {
    execFileSync(erzeuger, [ordner, 'allgemein', String(anzahl), '1'], {
      stdio: 'ignore',
      env: { ...process.env, TZ: 'Europe/Berlin', LOCALCHATA_JETZT: JETZT },
    });
  };
}

/// `beta` sendet `text` über die API; → die Zeit der Nachricht.
async function betaSchreibt(lauf, text) {
  const antwort = await senden(lauf.instanz.adresse, lauf.beta.token, 'allgemein', text, '127.0.0.2');
  expect(antwort.status, JSON.stringify(antwort.daten)).toBe(201);
  return antwort.daten.nachricht.zeit;
}

const scrollStand = (page) => page.getByRole('log').evaluate((log) => ({
  oben: log.scrollTop, untenAbstand: log.scrollHeight - log.scrollTop - log.clientHeight,
}));

module.exports = {
  player, abspielen, wiedergabe, elementMerken, istDasselbe, anhangAnfragenMitschreiben, aeltereNachrichten, betaSchreibt, scrollStand,
};
