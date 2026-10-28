'use strict';
// R00007 F0026 Beschreibung im Browser (US-23, US-24): Feld im Dialog „Channel anlegen“ und die Beschreibung in
// der Kopfzeile nach # name — einzeilig mit Ellipse, isoliert in <bdi>, ganz im title, unter 720 px nicht.
const fs = require('fs');
const path = require('path');
const { test, expect } = require('@playwright/test');
const { instanzStarten, fehlerSammeln } = require('./hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const SATZ = 'Absprachen zwischen Security- und Entwicklungs-Factory';
const LANG = 'Befunde '.repeat(25);

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

function mitBeschreibung(ordner, name, beschreibung) {
  const pfad = path.join(ordner, 'channels', name);
  fs.mkdirSync(pfad, { recursive: true });
  if (beschreibung !== null) {
    fs.writeFileSync(path.join(pfad, 'einstellungen.toml'), `beschreibung = ${JSON.stringify(beschreibung)}\n`);
  }
}

async function starten() {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      mitBeschreibung(ordner, 'security-entwicklung', SATZ);
      mitBeschreibung(ordner, 'lang', LANG.slice(0, 200));
      mitBeschreibung(ordner, 'ohne', null);
    },
  });
}

async function beitreten(page, url = instanz.adresse) {
  await page.goto(url);
  await page.getByLabel('Name').fill('shoff');
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText('shoff@');
}

const kopf = (page) => page.getByRole('banner');
const beschreibung = (page) => kopf(page).locator('.lc-head-desc');

test('Kopfzeile (US-23): nach # security-entwicklung steht die Beschreibung in <bdi>, ganz im title', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await starten();
  await beitreten(page, `${instanz.adresse}#/c/security-entwicklung`);

  await expect(kopf(page).locator('.lc-head-chan')).toHaveText('# security-entwicklung');
  await expect(beschreibung(page)).toHaveText(SATZ);
  await expect(beschreibung(page).locator('bdi')).toHaveText(SATZ);
  await expect(beschreibung(page)).toHaveAttribute('title', SATZ);
  await expect(beschreibung(page)).toBeVisible();
  const reihenfolge = await kopf(page).locator('.lc-head-ort > *').evaluateAll((teile) => teile.map((teil) => teil.className));
  expect(reihenfolge).toEqual(['lc-head-inst', 'lc-head-schalter', 'lc-head-frist', 'lc-head-desc']);
  expect(fehler).toEqual([]);
});

test('Kopfzeile: 200 Zeichen Beschreibung sind einzeilig mit Ellipse, ohne Beschreibung kein lc-head-desc', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await starten();
  await beitreten(page, `${instanz.adresse}#/c/lang`);

  await expect(beschreibung(page)).toHaveAttribute('title', LANG.slice(0, 200));
  const mass = await beschreibung(page).evaluate((element) => ({
    hoehe: element.getBoundingClientRect().height,
    zeile: parseFloat(getComputedStyle(element).lineHeight),
    abgeschnitten: element.scrollWidth > element.clientWidth,
    ellipse: getComputedStyle(element).textOverflow,
  }));
  expect(mass.hoehe).toBeLessThanOrEqual(mass.zeile + 1);
  expect(mass.abgeschnitten).toBe(true);
  expect(mass.ellipse).toBe('ellipsis');

  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'ohne', exact: true }).click();
  await expect(kopf(page).locator('.lc-head-chan')).toHaveText('# ohne');
  await expect(beschreibung(page)).toHaveCount(0);
  expect(fehler).toEqual([]);
});

test('Schmal (US-23): bei 719 px ist die Beschreibung weg, bei 720 px da, bei 320 px kein Querscrollen', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await starten();
  await beitreten(page, `${instanz.adresse}#/c/security-entwicklung`);
  await expect(beschreibung(page)).toBeVisible();

  await page.setViewportSize({ width: 719, height: 800 });
  await expect(beschreibung(page)).toBeHidden();
  await page.setViewportSize({ width: 720, height: 800 });
  await expect(beschreibung(page)).toBeVisible();
  await page.setViewportSize({ width: 320, height: 640 });
  await expect(beschreibung(page)).toBeHidden();
  const breiten = await page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));
  expect(breiten.inhalt).toBeLessThanOrEqual(breiten.fenster);
  expect(fehler).toEqual([]);
});

test('Dialog (US-24): Name und Beschreibung, Enter legt an; a<Tab>b wird abgewiesen, beide Eingaben bleiben', async ({ page }) => {
  const fehler = fehlerSammeln(page, { zusaetzlich: { '/api/channels': [422] } });
  await starten();
  await beitreten(page);
  await page.keyboard.press('Control+Alt+n');
  const dialog = page.getByRole('dialog', { name: 'Channel anlegen' });
  const feld = dialog.getByLabel('Beschreibung');
  await expect(feld).toHaveAttribute('maxlength', '200');

  await dialog.getByLabel('Name').fill('doku');
  await feld.fill('Notizen zur Doku');
  await feld.press('Enter');

  await expect(page.getByRole('status')).toHaveText('Channel # doku angelegt.');
  await expect(dialog).toHaveCount(0);
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'doku', exact: true }).click();
  await expect(beschreibung(page)).toHaveText('Notizen zur Doku');

  await page.keyboard.press('Control+Alt+n');
  await dialog.getByLabel('Name').fill('test2');
  await feld.fill('a\tb');
  await feld.press('Enter');

  const fehlerzeile = dialog.locator('.lc-fehler');
  await expect(fehlerzeile).toContainText('⚠');
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel('Name')).toHaveValue('test2');
  await expect(feld).toHaveValue('a\tb');
  const farben = await fehlerzeile.evaluate((element) => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--kd-danger)';
    document.body.append(probe);
    const gefahr = getComputedStyle(probe).color;
    probe.remove();
    return { text: getComputedStyle(element).color, gefahr };
  });
  expect(farben.text).toBe(farben.gefahr);
  expect(fs.existsSync(path.join(instanz.ordner, 'channels', 'test2'))).toBe(false);
  expect(fehler).toEqual([]);
});
