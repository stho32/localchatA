'use strict';
// R00004 F0015 Teilnehmerliste ein- und ausblenden (US-25 bis US-27; Designsystem README, 1-tastatur.md).
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln, fokusImBrowser } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const region = (page) => page.getByRole('complementary', { name: 'Teilnehmer' });
const schalter = (page) => page.getByRole('banner').getByRole('button', { name: 'Teilnehmer' });

async function beitreten(page) {
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').fill('shoff');
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toHaveText('shoff@127.0.0.1');
}

async function istSichtbar(page, sichtbar) {
  if (sichtbar) {
    await expect(region(page)).toBeVisible();
  } else {
    await expect(region(page)).toBeHidden();
  }
  await expect(schalter(page)).toHaveAttribute('aria-expanded', String(sichtbar));
}

test('US-25: bei 1280 px ist die Liste sichtbar, bei 1024 px eingeklappt', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page);
  await istSichtbar(page, true);

  await page.setViewportSize({ width: 1024, height: 800 });
  await page.reload();

  await expect(page.getByRole('banner').locator('.lc-me')).toHaveText('shoff@127.0.0.1');
  await istSichtbar(page, false);
  expect(fehler).toEqual([]);
});

test('US-26: Strg . blendet aus und ein, der Schalter mit Enter ebenso; aria-expanded folgt, der Fokus bleibt', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page);
  await istSichtbar(page, true);
  const fokusVorher = await page.evaluate(() => document.activeElement.className);

  await page.keyboard.press('Control+Period');
  await istSichtbar(page, false);
  await page.keyboard.press('Control+Period');
  await istSichtbar(page, true);
  const fokusNachStrgPunkt = await page.evaluate(() => document.activeElement.className);
  for (let schritt = 0; schritt < 10 && !(await schalter(page).evaluate((el) => el === document.activeElement)); schritt += 1) {
    await page.keyboard.press('Tab');
  }
  await expect(schalter(page)).toBeFocused();
  await page.keyboard.press('Enter');

  await istSichtbar(page, false);
  await expect(schalter(page)).toBeFocused();
  const fokus = await page.evaluate(fokusImBrowser);
  expect(fokusNachStrgPunkt).toBe(fokusVorher);
  expect(fokus.fokusSichtbar).toBe(true);
  expect(fokus.kontrast).toBeGreaterThanOrEqual(3);
  expect(fehler).toEqual([]);
});

test('Strg . mit dem Fokus in der Liste: ausgeblendet, der Fokus geht auf den Schalter statt an den Seitenanfang', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page);
  await region(page).focus();

  await page.keyboard.press('Control+Period');

  await istSichtbar(page, false);
  await expect(schalter(page)).toBeFocused();
});

test('US-27: bei 320 px eingeklappt, Schalter erreichbar; mit Strg . eingeblendet ohne Querscrollen', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.setViewportSize({ width: 320, height: 640 });
  await beitreten(page);
  await istSichtbar(page, false);
  await expect(schalter(page)).toBeVisible();
  const vorher = await page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));

  await page.keyboard.press('Control+Period');

  await istSichtbar(page, true);
  const nachher = await page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));
  expect(vorher.inhalt).toBeLessThanOrEqual(vorher.fenster);
  expect(nachher.inhalt).toBeLessThanOrEqual(nachher.fenster);
  expect(fehler).toEqual([]);
});
