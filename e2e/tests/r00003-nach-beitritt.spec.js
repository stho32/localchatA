'use strict';
// R00003 F0010, UX-Review (Sollte): Fokus nach dem Beitritt, Live-Hinweis wiederholt den Regeltext nicht.
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

async function senden(page, name) {
  await feld(page).fill(name);
  await feld(page).press('Enter');
}

test('nach dem Beitritt per Enter liegt der Fokus auf der eigenen Kennung in der Kopfzeile', async ({ page }) => {
  instanz = await instanzStarten();
  await page.goto(instanz.adresse);

  await senden(page, 'shoff');

  const kennung = page.getByRole('banner').locator('.lc-me');
  await expect(kennung).toHaveText('shoff@127.0.0.1');
  await expect(kennung).toBeFocused();
});

test('Live-Hinweis: der Regeltext bleibt beim Weitertippen derselbe Knoten, nur der Vorschlag ändert sich', async ({ page }) => {
  instanz = await instanzStarten();
  await page.goto(instanz.adresse);
  await feld(page).pressSequentially('J');
  const regel = page.locator('#lc-name-hinweis .lc-hinweis-regel');
  await expect(regel).toContainText('a–z, 0–9 und -');
  await regel.evaluate((element) => {
    window.regelAenderungen = 0;
    new MutationObserver((liste) => { window.regelAenderungen += liste.length; })
      .observe(element, { childList: true, characterData: true, subtree: true });
    window.regelKnoten = element;
  });

  await feld(page).pressSequentially('örg!');

  await expect(page.locator('#lc-name-hinweis .lc-id')).toHaveText(['joerg']);
  const ruhe = await page.evaluate(() => ({
    aenderungen: window.regelAenderungen,
    derselbe: window.regelKnoten === document.querySelector('#lc-name-hinweis .lc-hinweis-regel'),
    verbunden: window.regelKnoten.isConnected,
  }));
  expect(ruhe).toEqual({ aenderungen: 0, derselbe: true, verbunden: true });
});
