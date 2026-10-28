'use strict';
// R00003 F0010, UX-Review (Sollte): Kennung, Name und Vorschlag in Mono, Titel nach lc-title, Zweck des
// Bildschirms.
const { test, expect } = require('@playwright/test');
const { instanzStarten } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const feld = (page) => page.getByLabel('Name');
const meldung = (page) => page.locator('#lc-meldung');

async function senden(page, name) {
  await feld(page).fill(name);
  await feld(page).press('Enter');
}

async function schriftart(locator) {
  return locator.evaluate((element) => getComputedStyle(element).fontFamily);
}

test('Name vergeben: Name, Kennung und Vorschlag in Mono', async ({ page, request }) => {
  instanz = await instanzStarten();
  expect((await request.post(instanz.adresse + 'api/beitreten', { data: { name: 'shoff' } })).status()).toBe(201);
  await page.goto(instanz.adresse);

  await senden(page, 'shoff');

  const mono = meldung(page).locator('.lc-id');
  await expect(mono).toHaveText(['shoff', 'shoff@127.0.0.1', 'shoff-2']);
  for (const index of [0, 1, 2]) {
    expect(await schriftart(mono.nth(index))).toContain('IBM Plex Mono');
  }
  await expect(meldung(page)).toContainText(/^⚠ Der Name shoff ist vergeben — shoff@127\.0\.0\.1 ist seit \d\d:\d\d dabei\. Wie wäre shoff-2\?$/);
});

test('ungültig: Name und Vorschlag in Mono, auch im Hinweis beim Tippen', async ({ page }) => {
  instanz = await instanzStarten();
  await page.goto(instanz.adresse);

  await feld(page).pressSequentially('Jörg');
  const imHinweis = page.locator('#lc-name-hinweis .lc-id');
  await expect(imHinweis).toHaveText(['joerg']);
  expect(await schriftart(imHinweis)).toContain('IBM Plex Mono');
  await feld(page).press('Enter');

  const mono = meldung(page).locator('.lc-id');
  await expect(mono).toHaveText(['Jörg', 'joerg']);
  expect(await schriftart(mono.first())).toContain('IBM Plex Mono');
  expect(await schriftart(mono.last())).toContain('IBM Plex Mono');
});

test('Titel nach lc-title und ein Satz zu Zweck und erlaubten Zeichen', async ({ page }) => {
  instanz = await instanzStarten();
  await page.goto(instanz.adresse);

  const titel = await page.locator('h1').evaluate((element) => {
    const stil = getComputedStyle(element);
    return [stil.fontSize, stil.lineHeight, stil.fontWeight, stil.fontFamily.includes('IBM Plex Sans')];
  });
  expect(titel).toEqual(['14px', '20px', '600', true]);
  const zweck = page.locator('#lc-name-zweck');
  await expect(zweck).toBeVisible();
  await expect(zweck).toContainText('name@herkunft');
  await expect(zweck).toContainText('a–z, 0–9 und -');
  await expect(feld(page)).toHaveAttribute('aria-describedby', /lc-name-zweck/);
});
