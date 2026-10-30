'use strict';
// Hilfen für R00041 (eigene Nachrichten bearbeiten und löschen im Browser): Bearbeiten und Löschen per API (R00039,
// R00040), Browser mit und ohne SharedWorker, Senden im Browser, Textknoten des Verlaufs.
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
  BETA, betaVerlauf, betaSendet, bearbeitenPerApi, loeschenPerApi, browserOeffnen, imBrowserSenden, nachrichtMitZeit, zeigtBearbeitet,
  zeigtGeloescht, textknotenMit,
};
