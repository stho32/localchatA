'use strict';
// R00003 F0010 Beitreten-Bildschirm — Playwright mit Chromium gegen das echte Programm (US-15 bis US-21).
const { test, expect } = require('@playwright/test');
const { instanzStarten, fehlerSammeln } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function beitretenPerApi(request, name) {
  return request.post(instanz.adresse + 'api/beitreten', { data: { name } });
}

test('Bildschirm: Namensfeld hat nach dem Öffnen den Fokus, Schalter Beitreten', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);

  await page.goto(instanz.adresse);

  await expect(page.getByLabel('Name')).toBeFocused();
  await expect(page.getByRole('button', { name: 'Beitreten' })).toBeVisible();
  expect(fehler).toEqual([]);
});

test('US-16: Jörg zeigt vor dem Senden den Hinweis mit joerg, Enter tritt nicht bei', async ({ page, request }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.goto(instanz.adresse);
  const feld = page.getByLabel('Name');

  await feld.pressSequentially('Jörg');
  const hinweis = page.locator('#lc-name-hinweis');
  await expect(hinweis).toContainText('a–z, 0–9 und -');
  await expect(hinweis).toContainText('joerg');
  const antwort = page.waitForResponse('**/api/beitreten');
  await feld.press('Enter');
  expect((await antwort).status()).toBe(422);

  await expect(hinweis).toContainText('joerg');
  await expect(feld).toHaveValue('Jörg');
  await expect(feld).toBeFocused();
  expect((await beitretenPerApi(request, 'joerg')).status()).toBe(201);
  expect(fehler).toEqual([]);
});

test('US-17: Name vergeben nennt Träger und Uhrzeit HH:MM und füllt den Vorschlag vor', async ({ page, request }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  expect((await beitretenPerApi(request, 'shoff')).status()).toBe(201);
  await page.goto(instanz.adresse);

  await page.getByLabel('Name').pressSequentially('shoff');
  const antwort = page.waitForResponse('**/api/beitreten');
  await page.getByLabel('Name').press('Enter');
  const vergeben = await (await antwort).json();

  const uhrzeit = new Intl.DateTimeFormat('de-DE', { hour: '2-digit', minute: '2-digit', timeZone: 'Europe/Berlin' })
    .format(new Date(vergeben.seit));
  const meldung = page.locator('#lc-meldung');
  await expect(meldung).toContainText('shoff@127.0.0.1');
  await expect(meldung).toContainText(uhrzeit);
  await expect(page.getByLabel('Name')).toHaveValue('shoff-2');
  await expect(page.getByLabel('Name')).toBeFocused();
  expect(fehler).toEqual([]);
});

test('US-20: Server weg zeigt ⚠ Server nicht erreichbar mit Erneut versuchen', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page, { serverWegErwartet: true });
  await page.goto(instanz.adresse);
  await expect(page.getByLabel('Name')).toBeFocused();
  await instanz.beenden();

  await page.getByLabel('Name').pressSequentially('shoff');
  await page.getByLabel('Name').press('Enter');

  const meldung = page.locator('#lc-meldung');
  await expect(meldung).toContainText('⚠');
  await expect(meldung).toContainText('Server nicht erreichbar');
  await expect(page.getByRole('button', { name: 'Erneut versuchen' })).toBeVisible();
  await expect(page.getByLabel('Name')).toHaveValue('shoff');
  expect(fehler).toEqual([]);
});
