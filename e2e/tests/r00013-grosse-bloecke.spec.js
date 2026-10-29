'use strict';
// R00013 K8 L3c (US-7, US-8; ADR-0026 Ergänzung Abwurfschutz): `riesig` mit 200 Nachrichten zu 16.384 Zeichen in
// einem Fenster von 1280 × 2160 — ein Block endet am Budget (15 Nachrichten) und ist eingeklappt niedriger als zwei
// Sichthöhen. Das Fenster lädt nicht ohne Scrollen hin und her, hält höchstens 8 Blöcke und bleibt lückenlos.
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln, channelAnlegen } = require('./hilfen');
const { ankerBeobachten, ruhig } = require('./verlauf-hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const ALPHA = 'alpha@127.0.0.2';
const GRENZE_L4_PX = 1;
const HOECHSTENS_IM_DOM = 8 * 15;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

function satzzeit(index) {
  const minute = String(Math.floor(index / 60)).padStart(2, '0');
  const sekunde = String(index % 60).padStart(2, '0');
  return `2026-10-28T08:${minute}:${sekunde}.000+01:00`;
}

const ZEITEN = Array.from({ length: 200 }, (_, index) => satzzeit(index));
const verlauf = (page) => page.getByRole('log');
const nachrichten = (page) => verlauf(page).locator('.lc-msg');

async function riesigGeoeffnet(page) {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      channelAnlegen(ordner, 'riesig', {
        '2026-10-28': ZEITEN.map((zeit) => [zeit, ALPHA, 'x'.repeat(16384)]),
      });
    },
  });
  await page.setViewportSize({ width: 1280, height: 2160 });
  await page.goto(`${instanz.adresse}#/c/riesig`);
  await page.getByLabel('Name').fill('shoff');
  await page.getByLabel('Name').press('Enter');
  await expect(verlauf(page).locator(`.lc-msg[data-zeit="${ZEITEN[199]}"]`)).toBeVisible();
}

function abrufeZaehlen(page) {
  const abrufe = [];
  page.on('request', (anfrage) => {
    if (new URL(anfrage.url()).pathname.endsWith('/nachrichten')) {
      abrufe.push(Date.now());
    }
  });
  return abrufe;
}

// Höchstzahl der Nachrichten im DOM, bei jeder Änderung im Verlauf festgehalten (window.lcHoechstensImDom).
async function domZaehlen(page) {
  await page.evaluate(() => {
    const inhalt = document.querySelector('[role="log"] .lc-log-inhalt');
    window.lcHoechstensImDom = inhalt.querySelectorAll('.lc-msg').length;
    new MutationObserver(() => {
      window.lcHoechstensImDom = Math.max(window.lcHoechstensImDom, inhalt.querySelectorAll('.lc-msg').length);
    }).observe(inhalt, { childList: true, subtree: true });
  });
}

// Scrollt in Schritten, bis `bis` im Browser wahr ist; nach jedem Schritt Ruhe.
async function scrollenBis(page, schritt, bis) {
  for (let runde = 0; runde < 400; runde += 1) {
    if (await verlauf(page).evaluate(bis)) {
      return;
    }
    await verlauf(page).evaluate((log, weite) => log.scrollBy(0, weite), schritt);
    await ruhig(page);
  }
  expect(await verlauf(page).evaluate(bis)).toBe(true);
}

test('L3c (US-7): nach dem Öffnen und 2 s Ruhe lädt die Seite in den nächsten 2 s keinen Verlauf nach', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  const abrufe = abrufeZaehlen(page);
  await riesigGeoeffnet(page);

  await page.waitForTimeout(2000);
  const vorher = abrufe.length;
  await page.waitForTimeout(2000);

  expect(abrufe.length - vorher).toBe(0);
  expect(abrufe.length).toBeGreaterThanOrEqual(2);
  expect(fehler).toEqual([]);
});

test('L3c (US-8): hoch bis zur ältesten und zurück — jede Zeit genau einmal, höchstens 8 Blöcke, kein Sprung', async ({ page }) => {
  test.setTimeout(120000);
  const fehler = fehlerSammeln(page);
  await riesigGeoeffnet(page);
  await ruhig(page);
  await ankerBeobachten(page);
  await domZaehlen(page);

  await scrollenBis(page, -1000, (log) => log.scrollTop <= 1
    && log.querySelector(`.lc-msg[data-zeit="2026-10-28T08:00:00.000+01:00"]`) !== null);
  await scrollenBis(page, 1000, (log) => log.scrollHeight - log.scrollTop - log.clientHeight <= 1
    && log.querySelector(`.lc-msg[data-zeit="2026-10-28T08:03:19.000+01:00"]`) !== null);

  expect(await page.evaluate(() => window.lcOrdnungVerletzt)).toBe(false);
  const gesehen = await page.evaluate(() => [...window.lcGesehen].sort((links, rechts) => Date.parse(links) - Date.parse(rechts)));
  expect(gesehen).toEqual(ZEITEN);
  expect(await page.evaluate(() => window.lcHoechstensImDom)).toBeLessThanOrEqual(HOECHSTENS_IM_DOM);
  const spruenge = await page.evaluate(() => window.lcSpruenge);
  expect(spruenge.length).toBeGreaterThanOrEqual(10);
  for (const sprung of spruenge) {
    expect(Math.abs(sprung)).toBeLessThanOrEqual(GRENZE_L4_PX);
  }
  expect(fehler).toEqual([]);
});
