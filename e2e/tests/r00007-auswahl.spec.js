'use strict';
// R00007 F0028 Channel wählen und Kopfzeile (US-6 bis US-8): gegen das echte Programm, Chromium 1280 × 800,
// Uhr 2026-10-28 15:00 (+01:00). Channels liegen vor dem Start als Ordner. Zustellgrenze 2 s ab der 201,
// ereignisgenau (RAF). Konsole: keine Ausnahme, kein CSP-Verstoß, keine Fehler.
const fs = require('fs');
const path = require('path');
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln, beitretenPerApi, channelAnlegenPerApi, sichtbarNach } = require('./hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const ZUSTELLGRENZE_MS = 2000;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function starten(channels = []) {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => channels.forEach((name) => fs.mkdirSync(path.join(ordner, 'channels', name), { recursive: true })),
  });
}

const seitenleiste = (page) => page.getByRole('navigation', { name: 'Channels' });
const link = (page, name) => seitenleiste(page).getByRole('link', { name, exact: true });
const kopfChannel = (page) => page.getByRole('banner').locator('.lc-head-chan');
const mitte = (page) => page.locator('.lc-center');

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

async function istGewaehlt(page, name) {
  await expect(link(page, name)).toHaveAttribute('aria-current', 'page');
  await expect(kopfChannel(page)).toHaveText(`# ${name}`);
  await expect(page).toHaveTitle(`# ${name} — localchatA`);
}

test('Wählen (US-6): Klick auf # test setzt die Adresse, hebt den Eintrag hervor und zeigt ihn in der Kopfzeile', async ({ browser }) => {
  await starten(['allgemein', 'test']);
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await expect(link(a.page, 'test')).toHaveAttribute('href', '#/c/test');

  await link(a.page, 'test').click();

  await expect(a.page).toHaveURL(/#\/c\/test$/);
  await istGewaehlt(a.page, 'test');
  await expect(link(a.page, 'allgemein')).not.toHaveAttribute('aria-current', /.*/);
  const flaeche = await link(a.page, 'test').evaluate((element) => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--kd-accent)';
    document.body.append(probe);
    const akzent = getComputedStyle(probe).color;
    probe.remove();
    return { flaeche: getComputedStyle(element).backgroundColor, akzent };
  });
  expect(flaeche.flaeche).toBe(flaeche.akzent);
  expect(a.fehler).toEqual([]);
});

test('Ohne Auswahl (US-6): die Mitte sagt „Links einen Channel wählen.“, kein Eintrag ist gewählt, der Tab heißt localchatA', async ({ browser }) => {
  await starten(['allgemein', 'test']);
  const a = await seiteMitFehlern(browser);

  await beitreten(a.page, 'shoff');

  await expect(link(a.page, 'test')).toBeVisible();
  await expect(mitte(a.page)).toHaveText('Links einen Channel wählen.');
  await expect(seitenleiste(a.page).locator('[aria-current]')).toHaveCount(0);
  await expect(kopfChannel(a.page)).toHaveCount(0);
  await expect(a.page).toHaveTitle('localchatA');
  expect(a.fehler).toEqual([]);
});

test('Neuladen und Merker (US-7): der Channel bleibt nach Neuladen, ohne Adresse setzt der Tab seinen Merker', async ({ browser }) => {
  await starten(['allgemein', 'test']);
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await link(a.page, 'test').click();
  await istGewaehlt(a.page, 'test');

  await a.page.reload();
  await istGewaehlt(a.page, 'test');

  await a.page.goto(instanz.adresse);
  await expect(a.page).toHaveURL(/#\/c\/test$/);
  await istGewaehlt(a.page, 'test');
  expect(a.fehler).toEqual([]);
});

test('Zurück (US-7): erst allgemein, dann test, Zurück wählt allgemein', async ({ browser }) => {
  await starten(['allgemein', 'test']);
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await link(a.page, 'allgemein').click();
  await istGewaehlt(a.page, 'allgemein');
  await link(a.page, 'test').click();
  await istGewaehlt(a.page, 'test');

  await a.page.goBack();

  await istGewaehlt(a.page, 'allgemein');
  await expect(link(a.page, 'test')).not.toHaveAttribute('aria-current', /.*/);
  expect(a.fehler).toEqual([]);
});

test('Zwei Tabs (US-7): Tab 1 zeigt allgemein, Tab 2 test — auch nach Neuladen beider', async ({ browser }) => {
  await starten(['allgemein', 'test']);
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await link(a.page, 'allgemein').click();
  await istGewaehlt(a.page, 'allgemein');

  const tab2 = await a.kontext.newPage();
  const fehlerTab2 = fehlerSammeln(tab2);
  await tab2.goto(`${instanz.adresse}#/c/test`);

  await istGewaehlt(tab2, 'test');
  await istGewaehlt(a.page, 'allgemein');
  await a.page.reload();
  await tab2.reload();
  await istGewaehlt(a.page, 'allgemein');
  await istGewaehlt(tab2, 'test');
  expect(a.fehler).toEqual([]);
  expect(fehlerTab2).toEqual([]);
});

// Prüfung für sichtbarNach — läuft im Browser und kennt nur ihr Argument und das DOM.
function zeigtGewaehlt({ name }) {
  const kopf = document.querySelector('.lc-head .lc-head-chan');
  const eintrag = document.querySelector(`nav[aria-label="Channels"] a[href="#/c/${name}"]`);
  return kopf !== null && kopf.textContent === `# ${name}` && eintrag !== null && eintrag.getAttribute('aria-current') === 'page';
}

test('Unbekannt, dann angelegt (US-8): # spaeter gibt es nicht, nach dem Anlegen ist er gewählt — innerhalb der Zustellgrenze', async ({ browser }, testInfo) => {
  await starten();
  const a = await seiteMitFehlern(browser);
  const beitritt = await beitretenPerApi(instanz.adresse, 'alpha', '127.0.0.2');
  expect(beitritt.status).toBe(201);
  await beitreten(a.page, 'shoff', `${instanz.adresse}#/c/spaeter`);

  await expect(mitte(a.page)).toHaveText('# spaeter gibt es nicht.');
  await expect(kopfChannel(a.page)).toHaveCount(0);
  await expect(a.page).toHaveTitle('localchatA');

  const angelegt = await channelAnlegenPerApi(instanz.adresse, beitritt.daten.token, 'spaeter', '127.0.0.2');
  expect(angelegt.status).toBe(201);
  const ms = await sichtbarNach(a.page, zeigtGewaehlt, { argument: { name: 'spaeter' }, ab: angelegt.empfangen, timeout: 10000 });
  testInfo.annotations.push({ type: 'messung', description: `us8_zustellung_ms=${ms}` });
  console.log(`messung us8_zustellung_ms=${ms}`);

  expect(ms).toBeLessThanOrEqual(ZUSTELLGRENZE_MS);
  await expect(mitte(a.page)).not.toContainText('gibt es nicht');
  expect(a.fehler).toEqual([]);
});

test('Schmal: bei 320 px mit gewähltem 32-Zeichen-Channel kein Querscrollen, der Eintrag trägt aria-current', async ({ browser }) => {
  const lang = 'a'.repeat(16) + '-' + 'b'.repeat(15);
  await starten([lang, 'test']);
  const a = await seiteMitFehlern(browser);
  await a.page.setViewportSize({ width: 320, height: 640 });
  await beitreten(a.page, 'shoff', `${instanz.adresse}#/c/${lang}`);

  await expect(link(a.page, lang)).toHaveAttribute('aria-current', 'page');
  await expect(kopfChannel(a.page)).toHaveText(`# ${lang}`);
  const breiten = await a.page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));
  expect(breiten.inhalt).toBeLessThanOrEqual(breiten.fenster);
  expect(a.fehler).toEqual([]);
});

test('Fokus auf einem Eintrag bleibt, wenn ein anderer Channel dazukommt', async ({ browser }) => {
  await starten(['allgemein', 'test']);
  const a = await seiteMitFehlern(browser);
  const beitritt = await beitretenPerApi(instanz.adresse, 'alpha', '127.0.0.2');
  await beitreten(a.page, 'shoff');
  await link(a.page, 'test').focus();

  expect((await channelAnlegenPerApi(instanz.adresse, beitritt.daten.token, 'neu', '127.0.0.2')).status).toBe(201);

  await expect(link(a.page, 'neu')).toBeVisible();
  await expect(link(a.page, 'test')).toBeFocused();
  expect(a.fehler).toEqual([]);
});
