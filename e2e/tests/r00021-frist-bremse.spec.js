'use strict';
// R00021 F0072 Bremse im Dialog (US-3): eine Friständerung kostet 10 von 30 Einheiten der Kennung; nach drei
// Änderungen über die Sitzung von A zeigt der Dialog „Channel-Einstellungen“ den `fehler` der 429.
const fs = require('fs');
const path = require('path');
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln, fristPerApi } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

test('Bremse im Dialog (US-3): nach 8, 9, 8 per API zeigt Speichern mit 9 „⚠ Zu viele Änderungen …“, der Dialog bleibt, die Kopfzeile zeigt · 8 Tage', async ({ browser }) => {
  instanz = await instanzStarten({
    vorbereiten: (ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }),
  });
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await kontext.newPage();
  const fehler = fehlerSammeln(page, { zusaetzlich: { '/api/channels/allgemein/frist': [429] } });
  await page.goto(`${instanz.adresse}#/c/allgemein`);
  await page.getByLabel('Name').fill('shoff');
  await page.getByLabel('Name').press('Enter');
  const kopf = page.getByRole('banner');
  await expect(kopf.locator('.lc-me')).toContainText('shoff@127.0.0.1');
  await kopf.getByRole('button', { name: '# allgemein — Channel-Einstellungen' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog.getByRole('heading')).toHaveText('# allgemein — Einstellungen');
  const token = await page.evaluate(() => window.localStorage.getItem('localchatA.sitzung'));
  const gesetzt = [];
  for (const tage of [8, 9, 8]) {
    gesetzt.push((await fristPerApi(instanz.adresse, token, 'allgemein', { tage })).status);
  }
  expect(gesetzt).toEqual([200, 200, 200]);
  await expect(kopf.locator('.lc-head-frist')).toHaveText('· 8 Tage');

  await dialog.getByLabel('Aufbewahrungsfrist').fill('9');
  const antwort = page.waitForResponse((r) => r.url().endsWith('/api/channels/allgemein/frist'));
  await dialog.getByRole('button', { name: 'Speichern' }).click();

  expect((await antwort).status()).toBe(429);
  await expect(dialog.getByRole('alert')).toHaveText(/^⚠ Zu viele Änderungen von shoff@127\.0\.0\.1 — wieder möglich in (9|10) s\.$/);
  await expect(dialog).toBeVisible();
  await expect(kopf.locator('.lc-head-frist')).toHaveText('· 8 Tage');
  expect(fehler).toEqual([]);
});
