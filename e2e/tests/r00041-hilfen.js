'use strict';
// Hilfen für R00041 (eigene Nachrichten bearbeiten und löschen im Browser): Bearbeiten und Löschen per API (R00039,
// R00040), Browser mit und ohne SharedWorker, Senden im Browser, Textknoten des Verlaufs.
const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');
const { execFileSync } = require('child_process');
const { expect } = require('@playwright/test');
const { fehlerSammeln, channelAnlegen } = require('./hilfen');
const { anfragen, beitreten, senden, eingabe } = require('./senden-hilfen');

const BETA = '127.0.0.2';

/// `beta` tritt per API von 127.0.0.2 bei und sendet `texte` nach `allgemein`; → { token, zeiten }.
async function betaSendet(adresse, texte) {
  const token = await beitreten(adresse, 'beta', { herkunft: BETA });
  const zeiten = [];
  for (const text of texte) {
    const antwort = await senden(adresse, token, 'allgemein', text, BETA);
    expect(antwort.status, JSON.stringify(antwort.daten)).toBe(201);
    zeiten.push(antwort.daten.nachricht.zeit);
  }
  return { token, zeiten };
}

/// `POST …/nachrichten/bearbeiten {zeit, text}`; → Status, Daten, `empfangen` (Uhr der Zeitgrenze).
function bearbeitenPerApi(adresse, token, zeit, text, herkunft = BETA) {
  return anfragen(adresse, { methode: 'POST', pfad: 'api/channels/allgemein/nachrichten/bearbeiten', koerper: { zeit, text }, token, herkunft });
}

/// `POST …/nachrichten/loeschen {zeit}`.
function loeschenPerApi(adresse, token, zeit, herkunft = BETA) {
  return anfragen(adresse, { methode: 'POST', pfad: 'api/channels/allgemein/nachrichten/loeschen', koerper: { zeit }, token, herkunft });
}

/// Legt Anhänge über die API ab (R00029: anmelden, hochladen) und sendet sie mit `text` nach `allgemein`; dateien:
/// [{ name, inhalt: Buffer }] → die Zeit der Nachricht.
async function mitAnhaengenSenden(adresse, token, text, dateien, herkunft = BETA) {
  const ids = [];
  for (const { name, inhalt } of dateien) {
    const angemeldet = await anfragen(adresse, { methode: 'POST', pfad: 'api/anhaenge', koerper: { name, bytes: inhalt.length }, token, herkunft });
    expect(angemeldet.status, JSON.stringify(angemeldet.daten)).toBe(201);
    const { id } = angemeldet.daten.anhang;
    const hochgeladen = await bytesSenden(adresse, `api/anhaenge/${id}`, token, inhalt, herkunft);
    expect(hochgeladen, `Hochladen ${name}`).toBe(200);
    ids.push(id);
  }
  const gesendet = await anfragen(adresse, { methode: 'POST', pfad: 'api/channels/allgemein/nachrichten', koerper: { text, anhaenge: ids }, token, herkunft });
  expect(gesendet.status, JSON.stringify(gesendet.daten)).toBe(201);
  return gesendet.daten.nachricht.zeit;
}

function bytesSenden(adresse, pfad, token, inhalt, herkunft) {
  const ziel = new URL(pfad, adresse);
  return new Promise((fertig, gescheitert) => {
    const anfrage = http.request({
      host: ziel.hostname, port: ziel.port, path: ziel.pathname, method: 'PUT', localAddress: herkunft,
      headers: { 'Content-Type': 'application/octet-stream', 'Content-Length': inhalt.length, 'X-LocalchatA-Sitzung': token },
    }, (antwort) => {
      antwort.resume();
      antwort.on('end', () => fertig(antwort.statusCode));
    });
    anfrage.once('error', gescheitert);
    anfrage.end(inhalt);
  });
}

/// Ein PNG von 1 × 1 Pixel.
const PNG = Buffer.from('89504e470d0a1a0a0000000d4948445200000001000000010802000000907753de0000000c49444154789c63606060000000040001f61738550000000049454e44ae426082', 'hex');

/// Ein WebM aus ffmpeg (VP8, 1 s, ohne Ton), im eigenen Wegwerf-Ordner erzeugt.
function webm() {
  const ordner = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-webm-'));
  const pfad = path.join(ordner, 'film.webm');
  execFileSync('ffmpeg', ['-v', 'error', '-y', '-f', 'lavfi', '-i', 'testsrc=d=1:s=64x48:r=10', '-c:v', 'libvpx', '-an', pfad]);
  const inhalt = fs.readFileSync(pfad);
  fs.rmSync(ordner, { recursive: true, force: true });
  return inhalt;
}

/// Vorbereitung für sendenStarten: `allgemein` mit `anzahl` Nachrichten von `beta@127.0.0.2` (eine je Sekunde ab 14:00
/// am Tag der Uhr) — `beta` kann sie nach dem Beitritt von 127.0.0.2 bearbeiten und löschen; → die Zeiten.
function betaVerlauf(anzahl) {
  const zeiten = [];
  for (let nummer = 0; nummer < anzahl; nummer += 1) {
    const minute = String(Math.floor(nummer / 60)).padStart(2, '0');
    const sekunde = String(nummer % 60).padStart(2, '0');
    zeiten.push(`2026-10-28T14:${minute}:${sekunde}.000+01:00`);
  }
  return {
    zeiten,
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein', {
      '2026-10-28': zeiten.map((zeit, nummer) => [zeit, 'beta@127.0.0.2', `Nachricht ${nummer + 1}`]),
    }),
  };
}

/// Ein Browser-Kontext (1280 × 800 oder `geraet`), auf Wunsch ohne SharedWorker (Rückfall, R00026).
async function browserOeffnen(browser, { ohneWorker = false, geraet = { viewport: { width: 1280, height: 800 } }, ...sammeln } = {}) {
  const kontext = await browser.newContext(geraet);
  if (ohneWorker) {
    await kontext.addInitScript(() => { delete window.SharedWorker; });
  }
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page, sammeln) };
}

/// Sendet im Browser und wartet auf die Bestätigung; → die Zeit der Nachricht.
async function imBrowserSenden(page, text) {
  await eingabe(page).fill(text);
  await eingabe(page).press('Enter');
  const bestaetigt = page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)')
    .filter({ has: page.locator('.lc-body', { hasText: new RegExp(`^${text}$`) }) });
  await expect(bestaetigt).toHaveCount(1);
  return bestaetigt.getAttribute('data-zeit');
}

/// Die Nachricht mit dieser Zeit im Verlauf.
const nachrichtMitZeit = (page, zeit) => page.getByRole('log').locator(`.lc-msg[data-zeit="${zeit}"]`);

/// Prüfung für sichtbarNach: die Nachricht `zeit` zeigt `text` und die Marke „bearbeitet“.
function zeigtBearbeitet({ zeit, text }) {
  const knoten = document.querySelector(`.lc-log .lc-msg[data-zeit="${zeit}"]`);
  if (knoten === null) {
    return false;
  }
  const marke = [...knoten.querySelectorAll('.lc-marke')].some((einzeln) => einzeln.textContent === 'bearbeitet');
  return knoten.querySelector('.lc-body').textContent === text && marke;
}

/// Prüfung für sichtbarNach: die Nachricht `zeit` zeigt „Nachricht gelöscht“.
function zeigtGeloescht({ zeit }) {
  const knoten = document.querySelector(`.lc-log .lc-msg[data-zeit="${zeit}"]`);
  return knoten !== null && knoten.querySelector('.lc-body').textContent === 'Nachricht gelöscht';
}

/// Kontrast (WCAG 2.x) der Schrift eines Elements gegen die Fläche darunter, im Browser gerechnet.
const schriftKontrast = (locator) => locator.evaluate((element) => {
  const anteile = (farbe) => farbe.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number);
  const leuchtdichte = (farbe) => {
    const [r, g, b] = anteile(farbe).map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  let flaeche = 'rgb(255, 255, 255)';
  for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
    const farbe = getComputedStyle(knoten).backgroundColor;
    if (farbe !== 'rgba(0, 0, 0, 0)' && farbe !== 'transparent') {
      flaeche = farbe;
      break;
    }
  }
  const vorn = leuchtdichte(getComputedStyle(element).color);
  const hinten = leuchtdichte(flaeche);
  return (Math.max(vorn, hinten) + 0.05) / (Math.min(vorn, hinten) + 0.05);
});

/// Schreibt jeden nicht leeren Inhalt der Ansage-Region mit; → Abfrage der bisherigen Ansagen.
async function ansagenMitschreiben(page) {
  await page.evaluate(() => {
    window.ansagen = [];
    const region = document.querySelector('[role="status"][aria-live="polite"]');
    new MutationObserver(() => {
      if (region.textContent !== '') {
        window.ansagen.push(region.textContent);
      }
    }).observe(region, { childList: true, characterData: true, subtree: true });
  });
  return () => page.evaluate(() => window.ansagen);
}

/// Alle Textknoten des Verlaufs, deren Text genau `text` ist (getrimmt).
const textknotenMit = (page, text) => page.evaluate((gesucht) => {
  const laeufer = document.createTreeWalker(document.querySelector('[role="log"]'), NodeFilter.SHOW_TEXT);
  const treffer = [];
  for (let knoten = laeufer.nextNode(); knoten !== null; knoten = laeufer.nextNode()) {
    if (knoten.textContent.trim() === gesucht) {
      treffer.push(knoten.textContent);
    }
  }
  return treffer;
}, text);

module.exports = {
  BETA, betaVerlauf, mitAnhaengenSenden, PNG, webm, betaSendet, bearbeitenPerApi, loeschenPerApi, browserOeffnen, imBrowserSenden, nachrichtMitZeit, zeigtBearbeitet,
  zeigtGeloescht, textknotenMit, schriftKontrast, ansagenMitschreiben,
};
