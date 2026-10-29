'use strict';
// R00007 F0032 Channel per Tastatur wechseln (US-19, US-20): Alt ↓ / Alt ↑ wie ein Klick über die Adresse, der
// Fokus bleibt; nicht bei offenem Dialog. Konsole ohne Fehler.
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln, channelAnlegen } = require('./hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const ALPHA = 'alpha@127.0.0.2';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const link = (page, name) => page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true });
const verlauf = (page) => page.getByRole('log');

async function vorbereitet(page) {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      for (const name of ['a', 'b', 'c']) {
        channelAnlegen(ordner, name, { '2026-10-28': [['2026-10-28T14:00:00.000+01:00', ALPHA, `in ${name}`]] });
      }
    },
  });
  await page.goto(`${instanz.adresse}#/c/a`);
  await page.getByLabel('Name').fill('shoff');
  await page.getByLabel('Name').press('Enter');
  await expect(verlauf(page).locator('.lc-body')).toHaveText(['in a']);
}

async function istGewaehlt(page, name) {
  await expect(page).toHaveURL(new RegExp(`#/c/${name}$`));
  await expect(link(page, name)).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('banner').locator('.lc-head-chan')).toHaveText(`# ${name}`);
  await expect(verlauf(page).locator('.lc-body')).toHaveText([`in ${name}`]);
}

test('Wechseln (US-19): Alt ↓ aus dem Verlauf wählt b, dann c, am Ende nichts; der Fokus bleibt im Verlauf; Zurück → b', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await vorbereitet(page);
  await verlauf(page).focus();

  await page.keyboard.press('Alt+ArrowDown');
  await istGewaehlt(page, 'b');
  await expect(verlauf(page)).toBeFocused();
  await page.keyboard.press('Alt+ArrowDown');
  await istGewaehlt(page, 'c');
  await page.keyboard.press('Alt+ArrowDown');
  await page.waitForTimeout(200);
  await istGewaehlt(page, 'c');
  await expect(verlauf(page)).toBeFocused();

  await page.goBack();
  await istGewaehlt(page, 'b');
  await page.keyboard.press('Alt+ArrowUp');
  await istGewaehlt(page, 'a');
  expect(fehler).toEqual([]);
});

test('Mit Strg oder Umschalt wirkt Alt ↓ nicht', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await vorbereitet(page);
  await verlauf(page).focus();

  await page.keyboard.press('Control+Alt+ArrowDown');
  await page.keyboard.press('Shift+Alt+ArrowDown');
  await page.waitForTimeout(200);

  await istGewaehlt(page, 'a');
  expect(fehler).toEqual([]);
});

test('Dialog offen (US-20): Alt ↓ ändert die Auswahl nicht, der Dialog bleibt offen', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await vorbereitet(page);
  await page.keyboard.press('Control+Alt+n');
  const dialog = page.getByRole('dialog', { name: 'Channel anlegen' });
  await expect(dialog).toBeVisible();

  await page.keyboard.press('Alt+ArrowDown');
  await page.waitForTimeout(200);

  await expect(dialog).toBeVisible();
  await expect(page).toHaveURL(/#\/c\/a$/);
  await expect(link(page, 'a')).toHaveAttribute('aria-current', 'page');
  expect(fehler).toEqual([]);
});
