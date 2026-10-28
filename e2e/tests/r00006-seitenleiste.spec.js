'use strict';
// R00006 F0024 Seitenleiste im Browser (US-15 bis US-18): gegen das echte Programm, Chromium, je Kontext ein
// Browser; Agenten legen über die API an, damit die 201 im Testprozess ankommt. Dauern ereignisgenau (RAF) ab dem
// Ereignis des Kriteriums; die Grenze ist die des Kriteriums (2 s).
const { test, expect } = require('@playwright/test');
const { instanzStarten, fehlerSammeln, beitretenPerApi, channelAnlegenPerApi, sichtbarNach } = require('./hilfen');

const GRENZE_MS = 2000;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const seitenleiste = (page) => page.getByRole('navigation', { name: 'Channels' });
const eintraege = (page) => seitenleiste(page).locator('.lc-item');

async function beitreten(page, name, url = instanz.adresse) {
  await page.goto(url);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
}

async function seiteMitFehlern(browser) {
  const kontext = await browser.newContext();
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page) };
}

async function agent(name, herkunft) {
  const beitritt = await beitretenPerApi(instanz.adresse, name, herkunft);
  expect(beitritt.status).toBe(201);
  return (channel) => channelAnlegenPerApi(instanz.adresse, beitritt.daten.token, channel, herkunft);
}

// Prüfung für sichtbarNach — läuft im Browser und kennt nur ihr Argument und das DOM.
function zeigtChannel({ name }) {
  return [...document.querySelectorAll('nav[aria-label="Channels"] .lc-item')].some((eintrag) => eintrag.title === name);
}

function messen(testInfo, was, ms) {
  testInfo.annotations.push({ type: 'messung', description: `${was}=${ms}` });
  console.log(`messung ${was}=${ms}`);
}

test('US-15: ohne Channels zeigt die Seitenleiste „Channels“, den Schalter „Channel anlegen“ und den Weg', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser);

  await beitreten(a.page, 'shoff');

  await expect(seitenleiste(a.page).getByRole('heading', { name: 'Channels' })).toBeVisible();
  const plus = seitenleiste(a.page).getByRole('button', { name: 'Channel anlegen' });
  await expect(plus).toBeVisible();
  await expect(plus).toHaveAttribute('aria-keyshortcuts', 'Control+Alt+N');
  await expect(seitenleiste(a.page)).toContainText('Noch keine Channels.');
  await expect(eintraege(a.page)).toHaveCount(0);
  await expect(a.page.locator('.lc-center')).toBeEmpty();
  expect(a.fehler).toEqual([]);
});

test('US-16: ein Channel, den ein Agent anlegt, erscheint in B spätestens 2 s nach der 201, nach Name, ohne Fokuswechsel', async ({ browser }, testInfo) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser);
  const b = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'mia');
  const alpha = await agent('alpha', '127.0.0.2');
  await b.page.getByRole('button', { name: /^Teilnehmer/ }).focus();
  const fokusVorher = await b.page.evaluate(() => document.activeElement.textContent);

  const angelegt = await alpha('allgemein');
  expect(angelegt.status).toBe(201);
  const ms = await sichtbarNach(b.page, zeigtChannel, { argument: { name: 'allgemein' }, ab: angelegt.empfangen, timeout: 10000 });
  messen(testInfo, 'us16_zustellung_ms', ms);
  expect(ms).toBeLessThanOrEqual(GRENZE_MS);

  for (const name of ['zeta', 'beta']) {
    expect((await alpha(name)).status).toBe(201);
  }
  await expect(eintraege(b.page)).toHaveText(['#allgemein', '#beta', '#zeta']);
  expect(await b.page.evaluate(() => document.activeElement.textContent)).toBe(fokusVorher);
  await expect(b.page.locator('[role="alert"]:not([hidden]), [aria-live]:not([hidden])').filter({ hasText: 'allgemein' })).toHaveCount(0);
  await expect(eintraege(a.page)).toHaveText(['#allgemein', '#beta', '#zeta']);
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('US-17: ein zweiter Tab zeigt die Channels spätestens 2 s nach der Navigation, ohne neue Anlage', async ({ browser }, testInfo) => {
  instanz = await instanzStarten();
  const alpha = await agent('alpha', '127.0.0.2');
  expect((await alpha('allgemein')).status).toBe(201);
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await expect(eintraege(a.page)).toHaveText(['#allgemein']);

  const tab2 = await a.kontext.newPage();
  const fehlerTab2 = fehlerSammeln(tab2);
  await tab2.goto(instanz.adresse);
  const navigiert = Date.now();
  const ms = await sichtbarNach(tab2, zeigtChannel, { argument: { name: 'allgemein' }, ab: navigiert, timeout: 10000 });
  messen(testInfo, 'us17_neuer_tab_ms', ms);

  expect(ms).toBeLessThanOrEqual(GRENZE_MS);
  await expect(eintraege(tab2)).toHaveText(['#allgemein']);
  expect(a.fehler).toEqual([]);
  expect(fehlerTab2).toEqual([]);
});

test('Rückfall: ohne SharedWorker zeigt A die vorhandenen Channels und einen danach angelegten', async ({ browser }) => {
  instanz = await instanzStarten();
  const alpha = await agent('alpha', '127.0.0.2');
  expect((await alpha('allgemein')).status).toBe(201);
  const kontext = await browser.newContext();
  await kontext.addInitScript(() => { delete window.SharedWorker; });
  const page = await kontext.newPage();

  await beitreten(page, 'shoff');
  await expect(eintraege(page)).toHaveText(['#allgemein']);
  expect((await alpha('neu')).status).toBe(201);

  await expect(eintraege(page)).toHaveText(['#allgemein', '#neu']);
});

test('US-18: bei 320 px passt die Seitenleiste — Eintrag als #, Name als Tooltip und zugänglicher Name, + sichtbar', async ({ browser }) => {
  instanz = await instanzStarten();
  const lang = 'a'.repeat(16) + '-' + 'b'.repeat(15);
  const alpha = await agent('alpha', '127.0.0.2');
  expect((await alpha(lang)).status).toBe(201);
  const a = await seiteMitFehlern(browser);
  await a.page.setViewportSize({ width: 320, height: 640 });

  await beitreten(a.page, 'shoff');

  const eintrag = seitenleiste(a.page).getByRole('listitem', { name: lang });
  await expect(eintrag).toBeVisible();
  await expect(eintrag).toHaveAttribute('title', lang);
  expect((await eintrag.innerText()).trim()).toBe('#');
  await expect(seitenleiste(a.page).getByRole('button', { name: 'Channel anlegen' })).toBeVisible();
  const breiten = await a.page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));
  expect(breiten.inhalt).toBeLessThanOrEqual(breiten.fenster);
  expect(a.fehler).toEqual([]);
});

test('Tab-Reihenfolge: von der Kopfzeile erreicht Tab den Schalter + vor der Teilnehmerliste', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await a.page.getByRole('button', { name: /^Teilnehmer/ }).focus();

  const folge = [];
  for (let schritt = 0; schritt < 8; schritt += 1) {
    await a.page.keyboard.press('Tab');
    folge.push(await a.page.evaluate(() => document.activeElement.getAttribute('aria-label') || document.activeElement.tagName));
  }

  expect(folge[0]).toBe('Channel anlegen');
  expect(folge).toContain('Teilnehmer');
  expect(folge.indexOf('Channel anlegen')).toBeLessThan(folge.indexOf('Teilnehmer'));
  expect(a.fehler).toEqual([]);
});
