'use strict';
// R00007 F0030 Verlauf gegliedert (US-14, US-15): Gruppen, Tagestrenner nach dem Datum des Servers (der Browser
// läuft in Tokio), lange Nachrichten eingeklappt. Gegen das echte Programm, Uhr 2026-10-28 15:00 (+01:00).
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln, channelAnlegen } = require('./hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const ALPHA = 'alpha@127.0.0.2';
const BETA = 'beta@127.0.0.3';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const verlauf = (page) => page.getByRole('log');
const nachricht = (page, zeit) => verlauf(page).locator(`.lc-msg[data-zeit="${zeit}"]`);

async function geoeffnet(page, tage) {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein', tage),
  });
  await page.goto(`${instanz.adresse}#/c/allgemein`);
  await page.getByLabel('Name').fill('shoff');
  await page.getByLabel('Name').press('Enter');
  await expect(verlauf(page).locator('.lc-msg').first()).toBeVisible();
}

test.describe('in Tokio', () => {
  test.use({ timezoneId: 'Asia/Tokyo' });

  test('Ansicht (US-14): Tagestrenner nach dem Datum des Servers, Köpfe nur bei neuer Gruppe, Uhrzeit links beim Zeigen', async ({ page }) => {
    const fehler = fehlerSammeln(page);
    await geoeffnet(page, {
      '2026-10-24': [['2026-10-24T23:30:00.000+02:00', ALPHA, 'spät am Samstag']],
      '2026-10-27': [
        ['2026-10-27T10:00:00.000+01:00', ALPHA, 'eins'],
        ['2026-10-27T10:04:00.000+01:00', ALPHA, 'zwei'],
        ['2026-10-27T10:09:00.001+01:00', ALPHA, 'drei'],
        ['2026-10-27T10:10:00.000+01:00', BETA, 'vier'],
      ],
      '2026-10-28': [['2026-10-28T09:00:00.000+01:00', BETA, 'heute']],
    });

    await expect(verlauf(page).locator('.lc-day')).toHaveText(['Samstag, 24. Oktober 2026', 'Gestern', 'Heute']);
    await expect(verlauf(page).locator('.lc-meta .lc-time')).toHaveText(['23:30', '10:00', '10:09', '10:10', '09:00']);
    await expect(nachricht(page, '2026-10-27T10:04:00.000+01:00').locator('.lc-meta')).toHaveCount(0);
    const folge = await verlauf(page).locator('.lc-day, .lc-msg').evaluateAll((knoten) => knoten.map((k) => k.dataset.zeit || k.textContent));
    expect(folge.slice(0, 2)).toEqual(['Samstag, 24. Oktober 2026', '2026-10-24T23:30:00.000+02:00']);

    const links = nachricht(page, '2026-10-27T10:04:00.000+01:00').locator('.g time');
    await expect(links).toHaveText('10:04');
    expect(await links.evaluate((element) => getComputedStyle(element).opacity)).toBe('0');
    await nachricht(page, '2026-10-27T10:04:00.000+01:00').hover();
    await expect.poll(() => links.evaluate((element) => getComputedStyle(element).opacity)).toBe('1');
    expect(fehler).toEqual([]);
  });
});

test('Lange Nachricht (US-15): 240 px mit Leiste „Ganze Nachricht anzeigen · 30 Zeilen“, Aufklappen hält den Kopf', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  const lang = Array.from({ length: 30 }, (_, index) => `Zeile ${index + 1}`).join('\n');
  await geoeffnet(page, {
    '2026-10-28': [
      ['2026-10-28T14:00:00.000+01:00', ALPHA, lang],
      ['2026-10-28T14:10:00.000+01:00', BETA, 'eins\nzwei\ndrei'],
    ],
  });
  const lange = nachricht(page, '2026-10-28T14:00:00.000+01:00');
  const kurze = nachricht(page, '2026-10-28T14:10:00.000+01:00');
  const schalter = lange.getByRole('button');

  await expect(schalter).toHaveText('Ganze Nachricht anzeigen · 30 Zeilen');
  await expect(schalter).toHaveAttribute('aria-expanded', 'false');
  await expect(kurze.getByRole('button')).toHaveCount(0);
  const hoehe = () => lange.locator('.lc-body').evaluate((element) => element.getBoundingClientRect().height);
  expect(Math.abs((await hoehe()) - 240)).toBeLessThanOrEqual(1);
  const kopfVorher = await lange.locator('.lc-meta').evaluate((element) => element.getBoundingClientRect().top);

  await schalter.click();

  await expect(schalter).toHaveText('Einklappen');
  await expect(schalter).toHaveAttribute('aria-expanded', 'true');
  const offen = await lange.locator('.lc-body').evaluate((element) => ({
    ganz: element.scrollHeight <= element.clientHeight, text: element.textContent,
  }));
  expect(offen.ganz).toBe(true);
  expect(offen.text.endsWith('Zeile 30')).toBe(true);
  const kopfNachher = await lange.locator('.lc-meta').evaluate((element) => element.getBoundingClientRect().top);
  expect(Math.abs(kopfNachher - kopfVorher)).toBeLessThanOrEqual(1);

  await schalter.click();
  await expect(schalter).toHaveText('Ganze Nachricht anzeigen · 30 Zeilen');
  expect(Math.abs((await hoehe()) - 240)).toBeLessThanOrEqual(1);
  expect(fehler).toEqual([]);
});

test.describe('bei 1280 × 800', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  test('Lange Nachricht (Nacharbeit 2): eine einzige lange Zeile heißt „· 1 Zeile“, zwei lange Zeilen „· 2 Zeilen“', async ({ page }) => {
    const fehler = fehlerSammeln(page);
    const woerter = (anzahl) => Array.from({ length: anzahl }, (_, index) => `Wort${index + 1}`).join(' ');
    await geoeffnet(page, {
      '2026-10-28': [
        ['2026-10-28T14:00:00.000+01:00', ALPHA, woerter(400)],
        ['2026-10-28T14:10:00.000+01:00', BETA, `${woerter(300)}\n${woerter(300)}`],
      ],
    });
    const einzeilig = nachricht(page, '2026-10-28T14:00:00.000+01:00');
    const zweizeilig = nachricht(page, '2026-10-28T14:10:00.000+01:00');

    await expect(einzeilig.getByRole('button')).toHaveText('Ganze Nachricht anzeigen · 1 Zeile');
    await expect(zweizeilig.getByRole('button')).toHaveText('Ganze Nachricht anzeigen · 2 Zeilen');
    expect(fehler).toEqual([]);
  });
});
