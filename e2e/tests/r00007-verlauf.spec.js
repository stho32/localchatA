'use strict';
// R00007 F0029 Verlauf öffnen (US-9 bis US-13, L2): gegen das echte Programm, Chromium 1280 × 800, Uhr
// 2026-10-28 15:00 (+01:00). Nachrichten liegen vor dem Start als Tagesdateien; `gross` (100.000 über 7 Tage) kommt
// vom Testdaten-Erzeuger. Die Grenze L2 ist die der Anforderung (500 ms), gemessen im Browser vom Zeitstempel des
// click bis zum Bild, in dem die neueste Nachricht ganz sichtbar ist (requestAnimationFrame).
const { test, expect } = require('@playwright/test');
const {
  instanzStarten, fehlerSammeln, beitretenPerApi, channelAnlegen, grossAnlegen,
} = require('./hilfen');
const { neuesteSichtbarNachKlick, juengsteZeit } = require('./verlauf-hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const GRENZE_L2_MS = 500;
const ALPHA = 'alpha@127.0.0.2';
const BETA = 'beta@127.0.0.3';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function starten(vorbereiten) {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_JETZT: JETZT }, vorbereiten });
}

const US9 = {
  '2026-10-28': [
    ['2026-10-28T14:00:00.000+01:00', ALPHA, 'eins'],
    ['2026-10-28T14:20:00.000+01:00', BETA, 'zwei\ndrei'],
    ['2026-10-28T14:40:00.000+01:00', ALPHA, '<b>vier</b>'],
  ],
};

const link = (page, name) => page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true });
const verlauf = (page) => page.getByRole('log');
const nachrichten = (page) => verlauf(page).locator('.lc-msg');

async function beitreten(page, name = 'shoff', url = instanz.adresse) {
  await page.goto(url);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
}

async function seiteMitFehlern(browser, optionen = {}) {
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 }, ...optionen });
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page) };
}

// Ganz sichtbar heißt: innerhalb der sichtbaren Fläche des Verlaufs (± 1 px).
function ganzSichtbar(element) {
  return element.evaluate((nachricht) => {
    const log = nachricht.closest('[role="log"]').getBoundingClientRect();
    const lage = nachricht.getBoundingClientRect();
    return lage.top >= log.top - 1 && lage.bottom <= log.bottom + 1;
  });
}

test('Anzeige (US-9): drei Nachrichten mit Kennung in Mono und Uhrzeit des Servers, Text als Text, am Ende', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'allgemein', US9));
  const a = await seiteMitFehlern(browser, { timezoneId: 'Asia/Tokyo' });
  await beitreten(a.page);

  await link(a.page, 'allgemein').click();

  await expect(nachrichten(a.page)).toHaveCount(3);
  await expect(nachrichten(a.page).locator('.lc-id')).toHaveText([ALPHA, BETA, ALPHA]);
  await expect(nachrichten(a.page).locator('.lc-time')).toHaveText(['14:00', '14:20', '14:40']);
  await expect(nachrichten(a.page).locator('.lc-body')).toHaveText(['eins', 'zwei\ndrei', '<b>vier</b>']);
  const mono = await nachrichten(a.page).first().locator('.lc-id').evaluate((element) => getComputedStyle(element).fontFamily);
  expect(mono).toContain('IBM Plex Mono');
  await expect(nachrichten(a.page).first().locator('time')).toHaveAttribute('title', '28.10.2026, 14:00:00');
  await expect(nachrichten(a.page).first().locator('time')).toHaveAttribute('datetime', '2026-10-28T14:00:00.000+01:00');
  const zweiZeilen = await nachrichten(a.page).nth(1).locator('.lc-body').evaluate((element) => element.getClientRects()[0].height);
  const eineZeile = await nachrichten(a.page).first().locator('.lc-body').evaluate((element) => element.getClientRects()[0].height);
  expect(zweiZeilen).toBeGreaterThan(eineZeile * 1.5);
  await expect(verlauf(a.page).locator('b')).toHaveCount(0);
  expect(await ganzSichtbar(nachrichten(a.page).last())).toBe(true);
  expect(a.fehler).toEqual([]);
});

test('Leer (US-10): ein Channel ohne Tagesdatei sagt „Noch keine Nachrichten in # leer.“', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'leer'));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);

  await link(a.page, 'leer').click();

  await expect(verlauf(a.page)).toHaveText('Noch keine Nachrichten in # leer.');
  await expect(nachrichten(a.page)).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

test('L2 (US-11): 100.000 Nachrichten — die neueste ist innerhalb der Grenze L2 nach dem Klick sichtbar, ein Abruf ohne Cursor', async ({ browser }, testInfo) => {
  await starten((ordner) => grossAnlegen(ordner));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  const abrufe = [];
  a.page.on('request', (anfrage) => {
    if (anfrage.url().includes('/nachrichten')) {
      abrufe.push(new URL(anfrage.url()).search);
    }
  });
  const juengste = await juengsteZeit(instanz.adresse, 'gross');

  const ms = await neuesteSichtbarNachKlick(a.page, 'gross', juengste);
  testInfo.annotations.push({ type: 'messung', description: `l2_ms=${ms}` });
  console.log(`messung l2_ms=${ms}`);

  expect(ms).toBeLessThan(GRENZE_L2_MS);
  await expect(nachrichten(a.page)).toHaveCount(100);
  await expect(nachrichten(a.page).last()).toHaveAttribute('data-zeit', juengste);
  await expect(nachrichten(a.page).last().locator('.lc-id')).toHaveText(/^claude@10\.0\.0\.\d$/);
  expect(abrufe).toEqual(['?n=100']);
  expect(a.fehler).toEqual([]);
});

test('Wechsel mit verzögerter Antwort (US-12): nach dem Ende des Abrufs für gross zeigt der Verlauf genau allgemein', async ({ browser }) => {
  await starten((ordner) => {
    grossAnlegen(ordner);
    channelAnlegen(ordner, 'allgemein', US9);
  });
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await a.page.route('**/api/channels/gross/nachrichten**', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.continue().catch(() => {});
  });
  const grossFertig = new Promise((fertig) => {
    const beendet = (anfrage) => {
      if (anfrage.url().includes('/api/channels/gross/nachrichten')) {
        fertig();
      }
    };
    a.page.on('requestfinished', beendet);
    a.page.on('requestfailed', beendet);
  });

  await link(a.page, 'gross').click();
  await link(a.page, 'allgemein').click();
  await grossFertig;
  await a.page.waitForTimeout(100);

  await expect(nachrichten(a.page)).toHaveCount(3);
  await expect(nachrichten(a.page).locator('.lc-body')).toHaveText(['eins', 'zwei\ndrei', '<b>vier</b>']);
  await expect(a.page.getByRole('banner').locator('.lc-head-chan')).toHaveText('# allgemein');
  const fehler = a.fehler.filter((zeile) => !(zeile.includes('Failed to load resource') && zeile.includes('/api/channels/gross/nachrichten')));
  expect(fehler).toEqual([]);
});

test('Am Ende trotz später Schrift: lädt die Schrift 1 s später und ändert die Höhen, bleibt die neueste Nachricht ganz sichtbar', async ({ browser }) => {
  await starten((ordner) => grossAnlegen(ordner));
  // Schmaler als 1280 px, damit die Zeilen umbrechen: die Zeilenhöhe ist fest, nur der Umbruch hängt an der Schrift.
  const a = await seiteMitFehlern(browser, { bypassCSP: true, viewport: { width: 800, height: 800 } });
  await a.page.route('**/web/test-schrift.ttf', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.fulfill({ path: '/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf', contentType: 'font/ttf' });
  });
  await beitreten(a.page);
  const juengste = await juengsteZeit(instanz.adresse, 'gross');
  await expect(link(a.page, 'gross')).toBeVisible();

  await a.page.addStyleTag({ content: '@font-face { font-family: "IBM Plex Sans"; src: url("/web/test-schrift.ttf"); }' });
  await link(a.page, 'gross').click();
  await expect(nachrichten(a.page).last()).toHaveAttribute('data-zeit', juengste);
  const vorher = await verlauf(a.page).evaluate((log) => ({ hoehe: log.scrollHeight, schrift: document.fonts.status }));
  await a.page.evaluate(() => document.fonts.ready);
  await a.page.evaluate(() => new Promise((weiter) => requestAnimationFrame(() => requestAnimationFrame(weiter))));

  const hoeheNachher = await verlauf(a.page).evaluate((log) => log.scrollHeight);
  expect(vorher.schrift).toBe('loading');
  expect(hoeheNachher).not.toBe(vorher.hoehe);
  expect(await ganzSichtbar(nachrichten(a.page).last())).toBe(true);
  expect(a.fehler).toEqual([]);
});

test('Fehler (US-13): ist der Server weg, sagt die Mitte „Verlauf nicht geladen — Verbindung prüfen.“ mit ⚠ in kd-danger', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'allgemein', US9));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await expect(link(a.page, 'allgemein')).toBeVisible();

  await instanz.beenden();
  await link(a.page, 'allgemein').click();

  const meldung = a.page.locator('.lc-center .lc-fehler');
  await expect(meldung).toHaveText('⚠ Verlauf nicht geladen — Verbindung prüfen.');
  await expect(a.page.locator('.lc-center')).not.toContainText('Noch keine Nachrichten');
  const farben = await meldung.evaluate((element) => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--kd-danger)';
    document.body.append(probe);
    const gefahr = getComputedStyle(probe).color;
    probe.remove();
    return { text: getComputedStyle(element).color, gefahr };
  });
  expect(farben.text).toBe(farben.gefahr);
});

test('Fokus: Tab von der Seitenleiste erreicht den Verlauf (role=log, aria-live=off), Bild ↑ scrollt ihn', async ({ browser }) => {
  await starten((ordner) => grossAnlegen(ordner));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await link(a.page, 'gross').click();
  await expect(nachrichten(a.page)).toHaveCount(100);
  await expect(verlauf(a.page)).toHaveAttribute('aria-live', 'off');

  await link(a.page, 'gross').focus();
  await a.page.keyboard.press('Tab');
  await expect(verlauf(a.page)).toBeFocused();
  const vorher = await verlauf(a.page).evaluate((log) => log.scrollTop);
  await a.page.keyboard.press('PageUp');

  await expect.poll(() => verlauf(a.page).evaluate((log) => log.scrollTop)).toBeLessThan(vorher);
  const fokus = await a.page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);
  expect(fokus).toBe('solid');
  expect(a.fehler).toEqual([]);
});

test('Tab-Reihenfolge: Kopfzeile → Seitenleiste → Verlauf → Teilnehmerliste', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'allgemein', US9));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await link(a.page, 'allgemein').click();
  await expect(nachrichten(a.page)).toHaveCount(3);
  await a.page.getByRole('button', { name: /^Teilnehmer/ }).focus();

  const folge = [];
  for (let schritt = 0; schritt < 4; schritt += 1) {
    await a.page.keyboard.press('Tab');
    folge.push(await a.page.evaluate(() => document.activeElement.getAttribute('aria-label') || document.activeElement.tagName));
  }

  expect(folge).toEqual(['Channel anlegen', 'allgemein', 'Verlauf # allgemein', 'Teilnehmer']);
  expect(a.fehler).toEqual([]);
});

test('Schmal: bei 320 px mit geöffnetem Verlauf voller langer Nachrichten kein Querscrollen, die neueste ganz sichtbar', async ({ browser }) => {
  await starten((ordner) => grossAnlegen(ordner));
  const a = await seiteMitFehlern(browser, { viewport: { width: 320, height: 640 } });
  await beitreten(a.page, 'shoff', `${instanz.adresse}#/c/gross`);
  const juengste = await juengsteZeit(instanz.adresse, 'gross');

  await expect(nachrichten(a.page).last()).toHaveAttribute('data-zeit', juengste);

  const breiten = await a.page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));
  expect(breiten.inhalt).toBeLessThanOrEqual(breiten.fenster);
  expect(await ganzSichtbar(nachrichten(a.page).last())).toBe(true);
  expect(a.fehler).toEqual([]);
});
