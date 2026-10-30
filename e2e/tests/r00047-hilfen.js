'use strict';
// Hilfen für R00047 I0044 (im Browser mit Zitat antworten): `anna` (Mensch über die API von 127.0.0.2) und `claude`
// (Agent von 127.0.0.3) schreiben, `ben` antwortet im Browser; Senden mit Bezug über die API, die Antwortzeile, das Zitat.
const { expect } = require('@playwright/test');
const { anfragen, beitreten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { browserOeffnen, nachrichtMitZeit } = require('./r00041-hilfen');

const ANNA = '127.0.0.2';
const CLAUDE = '127.0.0.3';

/// `name` tritt per API bei (von `herkunft`, `art` wie beim Beitreten) und sendet `texte` nach `allgemein`; → { token, zeiten }.
async function perApiSchreiben(adresse, name, texte, { herkunft, art } = {}) {
  const token = await beitreten(adresse, name, { herkunft, art });
  const zeiten = [];
  for (const text of texte) {
    zeiten.push(await mitBezugSenden(adresse, token, text, null, herkunft));
  }
  return { token, zeiten };
}

/// `anna` (Mensch, 127.0.0.2) schreibt `texte`.
const annaSchreibt = (adresse, texte) => perApiSchreiben(adresse, 'anna', texte, { herkunft: ANNA, art: 'mensch' });

/// `claude` (Agent, 127.0.0.3) schreibt `texte`.
const claudeSchreibt = (adresse, texte) => perApiSchreiben(adresse, 'claude', texte, { herkunft: CLAUDE });

/// `POST …/nachrichten {text, antwort_auf}` nach `allgemein`, erwartet 201; → die Zeit der Nachricht.
async function mitBezugSenden(adresse, token, text, antwortAuf, herkunft) {
  const antwort = await anfragen(adresse, {
    methode: 'POST', pfad: 'api/channels/allgemein/nachrichten', koerper: { text, antwort_auf: antwortAuf }, token, herkunft,
  });
  expect(antwort.status, JSON.stringify(antwort.daten)).toBe(201);
  return antwort.daten.nachricht.zeit;
}

/// `ben` im Browser (1280 × 800 oder `geraet`), beigetreten in `allgemein`.
async function benOeffnen(browser, adresse, optionen = { streng: true }) {
  const b = await browserOeffnen(browser, optionen);
  await imBrowserBeitreten(b.page, adresse, 'ben');
  return b;
}

const antwortenKnopf = (page, zeit) => nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Antworten', exact: true });
const antwortzeile = (page) => page.locator('.lc-comp').getByRole('group', { name: /^Antwort an / });
const zitatVon = (page, zeit) => nachrichtMitZeit(page, zeit).locator('.lc-zitat');

/// Fährt mit der Maus über die Nachricht und klickt „Antworten“.
async function ueberHoverAntworten(page, zeit) {
  await nachrichtMitZeit(page, zeit).hover();
  await antwortenKnopf(page, zeit).click();
}

/// Tippt `text` ins Eingabefeld und sendet mit Enter; wartet, bis die eigene Nachricht bestätigt ist; → ihre Zeit.
async function tippenUndSenden(page, text) {
  await eingabe(page).fill(text);
  await eingabe(page).press('Enter');
  const bestaetigt = page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)')
    .filter({ has: page.locator('.lc-body', { hasText: new RegExp(`^${text}$`) }) });
  await expect(bestaetigt).toHaveCount(1);
  return bestaetigt.getAttribute('data-zeit');
}

/// Zählt die POST-Aufrufe der Seite an `…/nachrichten`.
function sendeaufrufeZaehlen(page) {
  const aufrufe = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().includes('/nachrichten')) {
      aufrufe.push(anfrage.url());
    }
  });
  return aufrufe;
}

/// Liest die Nachricht `zeit` per API (`GET …?n=200`).
async function perApiLesen(adresse, token, zeit, herkunft) {
  const antwort = await anfragen(adresse, { methode: 'GET', pfad: 'api/channels/allgemein/nachrichten?n=200', token, herkunft });
  return antwort.daten.nachrichten.find((nachricht) => Date.parse(nachricht.zeit) === Date.parse(zeit));
}

module.exports = {
  ANNA, CLAUDE, annaSchreibt, claudeSchreibt, perApiSchreiben, mitBezugSenden, benOeffnen, antwortenKnopf, antwortzeile, zitatVon,
  ueberHoverAntworten, tippenUndSenden, perApiLesen, sendeaufrufeZaehlen,
};
