'use strict';
// Hilfen für R00011 (Nachricht senden): Instanz mit `allgemein` und `technik`, Beitritt und Senden per API von einer
// wählbaren Absenderadresse, Beitritt im Browser, viele Nachrichten über den Testdaten-Erzeuger aus R00007.
const http = require('http');
const path = require('path');
const { execFileSync } = require('child_process');
const { expect } = require('@playwright/test');
const { instanzStarten, channelAnlegen, fehlerSammeln } = require('./hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const ERZEUGER = path.resolve(__dirname, '../../target/debug/examples/verlauf-erzeugen');

/// Startet mit Uhr 15:00, `allgemein` mit `anzahl` erzeugten Nachrichten (0: leer), die bis `erzeugtBis` reichen
/// (Standard: die Uhr der Instanz), und leerem `technik`; ein fester `port` behält den Ursprung über einen Neustart.
async function sendenStarten({ anzahl = 0, erzeugtBis = JETZT, port = 0, vorbereiten = () => {} } = {}) {
  return instanzStarten({
    port,
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: async (ordner) => {
      channelAnlegen(ordner, 'allgemein');
      channelAnlegen(ordner, 'technik');
      if (anzahl > 0) {
        execFileSync(ERZEUGER, [ordner, 'allgemein', String(anzahl), '1'], {
          stdio: 'ignore',
          env: { ...process.env, TZ: 'Europe/Berlin', LOCALCHATA_JETZT: erzeugtBis },
        });
      }
      await vorbereiten(ordner);
    },
  });
}

/// JSON-Anfrage von `herkunft`; liefert Status, Daten und `empfangen` (Augenblick der vollständigen Antwort).
function anfragen(adresse, { methode, pfad, koerper, token, herkunft = '127.0.0.1' }) {
  const ziel = new URL(pfad, adresse);
  let text = '';
  if (koerper !== undefined) {
    text = JSON.stringify(koerper);
  }
  const headers = { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(text) };
  if (token) {
    headers['X-LocalchatA-Sitzung'] = token;
  }
  return new Promise((fertig, gescheitert) => {
    const anfrage = http.request({
      host: ziel.hostname, port: ziel.port, path: `${ziel.pathname}${ziel.search}`, method: methode, localAddress: herkunft, headers,
    }, (antwort) => {
      let inhalt = '';
      antwort.on('data', (teil) => { inhalt += teil; });
      antwort.on('end', () => fertig({ status: antwort.statusCode, daten: JSON.parse(inhalt || 'null'), empfangen: Date.now() }));
    });
    anfrage.once('error', gescheitert);
    anfrage.end(text);
  });
}

/// Tritt per API bei (ohne `art`: Agent) und liefert das Token.
async function beitreten(adresse, name, { art, herkunft } = {}) {
  const koerper = { name };
  if (art) {
    koerper.art = art;
  }
  const antwort = await anfragen(adresse, { methode: 'POST', pfad: 'api/beitreten', koerper, herkunft });
  expect(antwort.status, JSON.stringify(antwort.daten)).toBe(201);
  return antwort.daten.token;
}

function senden(adresse, token, channel, text, herkunft) {
  return anfragen(adresse, { methode: 'POST', pfad: `api/channels/${channel}/nachrichten`, koerper: { text }, token, herkunft });
}

async function verlaufLesen(adresse, token, channel, herkunft) {
  const antwort = await anfragen(adresse, { methode: 'GET', pfad: `api/channels/${channel}/nachrichten?n=200`, token, herkunft });
  return antwort.daten.nachrichten;
}

/// Ein Browser-Kontext (Chromium, 1280 × 800) mit Fehlersammler; `streng` und `zusaetzlich` wie fehlerSammeln.
async function kontextOeffnen(browser, { breite = 1280, ...sammeln } = {}) {
  const kontext = await browser.newContext({ viewport: { width: breite, height: 800 } });
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page, sammeln) };
}

/// Tritt im Browser bei und wählt `channel`.
async function imBrowserBeitreten(page, adresse, name, channel = 'allgemein') {
  await page.goto(`${adresse}#/c/${channel}`);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', `Verlauf # ${channel}`);
}

const eingabe = (page) => page.getByRole('textbox', { name: /^Nachricht an # / });
const nachrichten = (page) => page.getByRole('log').locator('.lc-msg');
/// Nachrichten, deren Text (ohne Kopf) `muster` trifft.
const mitText = (page, muster) => nachrichten(page).filter({ has: page.locator('.lc-body', { hasText: muster }) });

module.exports = {
  JETZT, sendenStarten, beitreten, senden, anfragen, verlaufLesen, kontextOeffnen, imBrowserBeitreten, eingabe, nachrichten, mitText,
};
