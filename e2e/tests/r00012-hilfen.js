'use strict';
// Hilfen für R00012 (Senden bei Störungen und mit Komfort): Netz eines Browser-Kontexts aus und an, die Leiste oben über
// der Mitte (seit R00025), Sätze in der Tagesdatei, die Statusregion der Ansage.
const fs = require('fs');
const path = require('path');
const { expect } = require('@playwright/test');

const ROUTE = '**/api/channels/*/nachrichten';

const leiste = (page) => page.locator('.lc-center > .lc-leiste');
const vorlaeufige = (page) => page.getByRole('log').locator('.lc-msg.lc-vorlaeufig');
/// Bestätigte (nicht vorläufige) Nachrichten, deren Text genau `text` ist.
const bestaetigt = (page, text) => page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)')
  .filter({ has: page.locator('.lc-body', { hasText: new RegExp(`^${text}$`) }) });
/// Die eine gemeinsame Ansage-Region (ADR-0038 Regel 10). Daneben steht seit R00045 (ux-reviewer C, Karte #77) genau eine
/// weitere, die eigene Region der Tippanzeige `.lc-tipp-ansage` — sie ist hier ausgenommen, jede andere zählt mit.
const ansageRegion = (page) => page.locator('[role="status"][aria-live="polite"]:not(.lc-tipp-ansage)');

/// „Strom getrennt“ (R00012): Netz des Kontexts aus, bis die Leiste „Verbindung verloren“ steht.
async function netzAus(seite) {
  await seite.kontext.setOffline(true);
  await expect(leiste(seite.page)).toHaveText(/^⚠ Verbindung verloren/);
}

async function netzAn(seite) {
  await seite.kontext.setOffline(false);
}

/// Zahl der Sätze mit genau diesem einzeiligen Text in der heutigen Tagesdatei (Uhr 2026-10-28 15:00 +01:00).
function saetzeMit(ordner, channel, text) {
  const datei = path.join(ordner, 'channels', channel, '2026-10-28.log');
  if (!fs.existsSync(datei)) {
    return 0;
  }
  return fs.readFileSync(datei, 'utf8').split('\n').filter((zeile) => zeile === `  ${text}`).length;
}

/// POST-Anfragen an …/nachrichten mit Text und Idempotency-Key, wie die Seite sie stellt.
function sendungenMitschreiben(page) {
  const sendungen = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten')) {
      sendungen.push({ text: anfrage.postDataJSON().text, schluessel: anfrage.headers()['idempotency-key'], beginn: Date.now() });
    }
  });
  return sendungen;
}

module.exports = { ROUTE, leiste, vorlaeufige, bestaetigt, ansageRegion, netzAus, netzAn, saetzeMit, sendungenMitschreiben };
