'use strict';
// R00024 Instanz über ihren Netznamen öffnen (ADR-0045) im Testbrowser: Chromium löst die Namen per Regel auf
// 127.0.0.1 auf, welchen Namen die Instanz annimmt, entscheiden die Testnamen (LOCALCHATA_TESTNAMEN, Debug-Build).
const { chromium } = require('@playwright/test');
const { test, expect } = require('./rahmen');
const { instanzStarten } = require('./hilfen');

const TESTNAMEN = '127.0.0.1>nuc5.fritz.box;chat.local=127.0.0.1;drucker.local=192.0.2.1;boese.example=127.0.0.1';
const NAMEN = ['nuc5.fritz.box', 'chat.local', 'drucker.local', 'boese.example'];
const UEBERSCHRIFT = 'Diese Adresse ist für localchatA nicht freigegeben';

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

async function testbrowser() {
  const regeln = NAMEN.map((name) => `MAP ${name} 127.0.0.1`).join(', ');
  browser = await chromium.launch({ args: [`--host-resolver-rules=${regeln}`] });
  const kontext = await browser.newContext({ locale: 'de-DE', timezoneId: 'Europe/Berlin' });
  return kontext.newPage();
}

// Jede Konsolenzeile auf Stufe error und jede unbehandelte Ausnahme, mit der Adresse, zu der sie gehört.
function konsoleMitschreiben(page) {
  const zeilen = [];
  page.on('console', (meldung) => {
    if (meldung.type() === 'error') {
      zeilen.push({ text: meldung.text(), url: meldung.location().url });
    }
  });
  page.on('pageerror', (ausnahme) => zeilen.push({ text: 'Ausnahme: ' + ausnahme.message, url: '' }));
  return zeilen;
}

function unterName(name) {
  return instanz.adresse.replace('127.0.0.1', name);
}

test('US-2: der Rückwärtsname öffnet die Oberfläche, auch groß geschrieben', async () => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_TESTNAMEN: TESTNAMEN } });
  const page = await testbrowser();

  await page.goto(unterName('nuc5.fritz.box'));
  await expect(page).toHaveTitle('localchatA');
  await expect(page.getByLabel('Name')).toBeVisible();
  await page.goto(unterName('NUC5.Fritz.Box'));
  await expect(page).toHaveTitle('localchatA');
});

test('US-4: ein .local-Name auf die Instanz öffnet sie', async () => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_TESTNAMEN: TESTNAMEN } });
  const page = await testbrowser();

  await page.goto(unterName('chat.local'));

  await expect(page).toHaveTitle('localchatA');
});

test('US-5: ein .local-Name auf einen anderen Rechner zeigt die Seite', async () => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_TESTNAMEN: TESTNAMEN } });
  const page = await testbrowser();

  await page.goto(unterName('drucker.local'));

  await expect(page.getByRole('heading', { level: 1 })).toHaveText(UEBERSCHRIFT);
});

test('US-6, US-7: ein öffentlicher Name zeigt die deutsche Seite, der Link führt zur Instanz', async () => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_TESTNAMEN: TESTNAMEN } });
  const page = await testbrowser();
  const konsole = konsoleMitschreiben(page);

  const antwort = await page.goto(unterName('boese.example'));
  expect(antwort.status()).toBe(403);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(UEBERSCHRIFT);
  await expect(page.locator('html')).toHaveAttribute('lang', 'de');
  await expect(page.getByText('Der Betreiber kann den Namen boese.example mit --erreichbar-als boese.example beim Start freigeben.')).toBeVisible();
  await expect(page.getByText(/fremde Websites/)).toBeVisible();
  expect(await page.locator('style, script, link[rel="stylesheet"]').count()).toBe(0);
  const link = page.getByRole('link', { name: instanz.adresse });
  await expect(link).toHaveAttribute('href', instanz.adresse);

  await link.click();
  await expect(page).toHaveURL(instanz.adresse);
  await expect(page).toHaveTitle('localchatA');

  // Erlaubt sind nur „Failed to load resource“ zu den 403 auf das Dokument und /favicon.ico unter boese.example.
  const fremd = konsole.filter(({ text, url }) => {
    const ort = url ? new URL(url) : null;
    const erlaubt = text.startsWith('Failed to load resource: ') && text.includes('403')
      && ort !== null && ort.hostname === 'boese.example' && ['/', '/favicon.ico'].includes(ort.pathname);
    return !erlaubt;
  });
  expect(fremd).toEqual([]);
});
