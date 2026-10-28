'use strict';
// R00003 F0010, UX-Review (Sollte): schmale Fenster ohne Querscrollen, Kopfzeile mit Instanznamen.
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

async function querscrollen(page) {
  return page.evaluate(() => ({ breite: document.documentElement.scrollWidth, fenster: window.innerWidth }));
}

for (const breite of [320, 360, 375]) {
  test(`schmal ${breite} px: kein Querscrollen in den Zuständen leer, ungültig, vergeben, Server weg`, async ({ page, request }) => {
    instanz = await instanzStarten();
    expect((await request.post(instanz.adresse + 'api/beitreten', { data: { name: 'shoff' } })).status()).toBe(201);
    await page.setViewportSize({ width: breite, height: 640 });
    await page.goto(instanz.adresse);
    await expect(feld(page)).toBeFocused();
    const gemessen = [];

    gemessen.push(['leer', await querscrollen(page)]);
    await senden(page, 'Jörg');
    await expect(meldung(page)).toContainText('nicht erlaubt');
    gemessen.push(['ungültig', await querscrollen(page)]);
    await senden(page, 'shoff');
    await expect(meldung(page)).toContainText('vergeben');
    gemessen.push(['vergeben', await querscrollen(page)]);
    await instanz.beenden();
    await senden(page, 'shoff-3');
    await expect(meldung(page)).toContainText('Server nicht erreichbar');
    gemessen.push(['Server weg', await querscrollen(page)]);

    for (const [zustand, { breite: inhalt, fenster }] of gemessen) {
      expect(inhalt, zustand).toBeLessThanOrEqual(fenster);
    }
  });
}

test('Kopfzeile bei 480 px zeigt den Instanznamen ganz', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 480, height: 640 });
  await page.goto(instanz.adresse);
  await senden(page, 'shoff');
  await expect(page.getByRole('banner').locator('.lc-me')).toHaveText('shoff@127.0.0.1');

  const instanzname = await page.locator('.lc-head-inst').evaluate((element) => ({
    text: element.textContent, sichtbar: element.clientWidth, noetig: element.scrollWidth,
  }));

  expect(instanzname.text).toBe('localchatA');
  expect(instanzname.sichtbar).toBeGreaterThan(0);
  expect(instanzname.noetig).toBeLessThanOrEqual(instanzname.sichtbar);
});
