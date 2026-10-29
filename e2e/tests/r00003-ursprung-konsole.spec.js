'use strict';
// R00003 F0006, Nacharbeit 2 (Entscheidung Director): über jede Adresse, unter der die Instanz erreichbar ist —
// LAN-IP, 127.0.0.1, localhost, ein mit --erreichbar-als freigegebener Name — lädt die Seite und tritt bei,
// ohne Konsolenzeile auf Stufe error; der Header-Satz außer COOP steht auf jeder Antwort.
const { chromium } = require('@playwright/test');
const { test, expect } = require('./rahmen');
const { instanzStarten, lanAdresse, fehlerSammeln } = require('./hilfen');

const CSP = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";
const HEADER_SATZ = {
  'x-content-type-options': 'nosniff',
  'content-security-policy': CSP,
  'x-frame-options': 'DENY',
  'referrer-policy': 'no-referrer',
  'cross-origin-resource-policy': 'same-origin',
};
const FREIGEGEBENER_NAME = 'chat.lan';

let instanz;
let browser;
test.afterEach(async () => {
  if (browser) {
    await browser.close();
    browser = null;
  }
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function zugang(art) {
  if (art === 'LAN-IP') {
    const lan = await lanAdresse();
    instanz = await instanzStarten({ host: lan });
    return { url: instanz.adresse, herkunft: lan };
  }
  if (art === 'freigegebener Name') {
    instanz = await instanzStarten({ argumente: ['--erreichbar-als', FREIGEGEBENER_NAME] });
    return { url: instanz.adresse.replace('127.0.0.1', FREIGEGEBENER_NAME), herkunft: '127.0.0.1' };
  }
  instanz = await instanzStarten();
  if (art === 'localhost') {
    return { url: instanz.adresse.replace('127.0.0.1', 'localhost'), herkunft: '127.0.0.1' };
  }
  return { url: instanz.adresse, herkunft: '127.0.0.1' };
}

// Der freigegebene Name hat keinen DNS-Eintrag: der Browser löst ihn per Regel auf 127.0.0.1 auf.
async function seiteOeffnen() {
  browser = await chromium.launch({ args: [`--host-resolver-rules=MAP ${FREIGEGEBENER_NAME} 127.0.0.1`] });
  const kontext = await browser.newContext({ locale: 'de-DE', timezoneId: 'Europe/Berlin' });
  return kontext.newPage();
}

for (const art of ['LAN-IP', '127.0.0.1', 'localhost', 'freigegebener Name']) {
  test(`über ${art}: Laden und Beitreten ohne Konsolenfehler, Header-Satz auf jeder Antwort`, async () => {
    const { url, herkunft } = await zugang(art);
    const page = await seiteOeffnen();
    const fehler = fehlerSammeln(page);
    const antworten = [];
    page.on('response', (antwort) => antworten.push(antwort));

    await page.goto(url);
    await page.getByLabel('Name').fill('anna');
    await page.getByLabel('Name').press('Enter');
    await expect(page.getByRole('banner').locator('.lc-me')).toHaveText(`anna@${herkunft}`);
    await page.reload();
    await expect(page.getByRole('banner').locator('.lc-me')).toHaveText(`anna@${herkunft}`);

    expect(fehler).toEqual([]);
    expect(antworten.length).toBeGreaterThan(5);
    for (const antwort of antworten) {
      const kopf = await antwort.allHeaders();
      for (const [name, wert] of Object.entries(HEADER_SATZ)) {
        expect(kopf[name], `${name} an ${antwort.url()}`).toBe(wert);
      }
    }
  });
}
