'use strict';
// Hilfen der Specs zu R00030 F0165 „Video und Audio im Player“: den Player eines Anhangs finden, stumm abspielen (ohne
// Geste spielt Chromium nur stumm von selbst) und seinen Zustand lesen; Anfragen an /anhaenge/ mitschreiben.
const { expect } = require('./rahmen');

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

module.exports = { player, abspielen, wiedergabe, elementMerken, istDasselbe, anhangAnfragenMitschreiben };
