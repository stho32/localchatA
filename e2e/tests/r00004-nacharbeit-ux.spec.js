'use strict';
// R00004 Nacharbeit 1 aus dem UX-Review (S1–S6, O1): Zeilen, Anzahl am Schalter, Liste als Fläche unter
// 720 px, „Stand unbekannt“ bei unterbrochener Verbindung, ruhiger Alarmtext, Fokus beim Einklappen.
const { test, expect } = require('@playwright/test');
const { instanzStarten, freierPort, fehlerSammeln, beitretenPerApi, stromOeffnen } = require('./hilfen');

let instanz;
const offeneStroeme = [];
test.afterEach(async () => {
  offeneStroeme.splice(0).forEach((strom) => strom.schliessen());
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const region = (page) => page.getByRole('complementary', { name: 'Teilnehmer' });
const schalter = (page) => page.getByRole('banner').getByRole('button', { name: /^Teilnehmer/ });
const zeileVon = (page, kennung) => region(page).locator('.lc-person').filter({ has: page.locator('.lc-id', { hasText: kennung }) });

async function beitreten(page, name = 'shoff') {
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toHaveText(`${name}@127.0.0.1`);
}

async function agentMitStrom(name) {
  const { daten } = await beitretenPerApi(instanz.adresse, name);
  const strom = await stromOeffnen(instanz.adresse, daten.token);
  offeneStroeme.push(strom);
  return strom;
}

async function listeZeigen(page) {
  if (!(await region(page).isVisible())) {
    await schalter(page).click();
  }
  await expect(region(page)).toBeVisible();
}

test('S1: „(du)“ und „Agent“ stehen in der Zeile hinter der Kennung — je 28 px hoch bei 1280 px', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 1280, height: 800 });
  await agentMitStrom('claude');
  await beitreten(page);
  await expect(zeileVon(page, 'claude@127.0.0.1').locator('.lc-dot')).toHaveClass(/\bon\b/);

  const hoehen = await Promise.all(['shoff@127.0.0.1', 'claude@127.0.0.1'].map((kennung) =>
    zeileVon(page, kennung).evaluate((zeile) => zeile.getBoundingClientRect().height)));

  expect(hoehen).toEqual([28, 28]);
  await expect(zeileVon(page, 'shoff@127.0.0.1')).toContainText('(du)');
  await expect(zeileVon(page, 'claude@127.0.0.1').locator('.lc-tag')).toHaveText('Agent');
});

for (const breite of [1280, 320]) {
  test(`S2: geht ein Agent mit 32 Zeichen offline, verschiebt sich die Zeile darunter um 0 px — ${breite} px`, async ({ page }) => {
    instanz = await instanzStarten();
    await page.setViewportSize({ width: breite, height: 800 });
    const langerName = 'agent-mit-zweiunddreissig-zeichen'.slice(0, 32);
    const strom = await agentMitStrom(langerName);
    await agentMitStrom('zz-darunter');
    await beitreten(page);
    await listeZeigen(page);
    const lang = zeileVon(page, `${langerName}@127.0.0.1`);
    const darunter = zeileVon(page, 'zz-darunter@127.0.0.1');
    await expect(lang.locator('.lc-dot')).toHaveClass(/\bon\b/);
    const vorher = await darunter.evaluate((zeile) => zeile.getBoundingClientRect().top);

    strom.schliessen();
    await expect(lang.locator('.lc-status')).toHaveText('offline', { timeout: 3000 });
    const nachher = await darunter.evaluate((zeile) => zeile.getBoundingClientRect().top);

    expect(nachher - vorher).toBe(0);
  });
}

test('S3: der Schalter nennt die Anzahl — 3, nach einem „weg“ 2, ohne Neuladen', async ({ page }) => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_GNADENFRIST_MS: '3000' } });
  await page.setViewportSize({ width: 1024, height: 800 });
  await agentMitStrom('claude');
  expect((await beitretenPerApi(instanz.adresse, 'kurz')).status).toBe(201);
  await beitreten(page);

  await expect(schalter(page)).toHaveAccessibleName('Teilnehmer 3');
  await expect(schalter(page).locator('.lc-count')).toHaveText('3');
  await expect(schalter(page)).toHaveAccessibleName('Teilnehmer 2', { timeout: 6000 });
});

test('S4: unter 720 px legt sich die Liste über die Mitte; Fokus hinein, Esc schließt, Fokus zurück auf den Schalter', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 320, height: 640 });
  await beitreten(page);
  await expect(region(page)).toBeHidden();

  await schalter(page).focus();
  await page.keyboard.press('Enter');
  await expect(region(page)).toBeVisible();
  await expect(region(page)).toBeFocused();
  const flaeche = await page.evaluate(() => {
    const liste = document.querySelector('.lc-members').getBoundingClientRect();
    const mitte = document.querySelector('.lc-center').getBoundingClientRect();
    return { links: liste.left, breite: liste.width, mitteBreite: mitte.width, fenster: window.innerWidth };
  });
  await page.keyboard.press('Escape');

  expect(flaeche.links).toBe(0);
  expect(flaeche.breite).toBe(flaeche.fenster);
  await expect(region(page)).toBeHidden();
  await expect(schalter(page)).toBeFocused();
});

test('S4: bei 320 px mit eingeblendeter Liste ist der Hinweis bei unterbrochener Verbindung lesbar, ohne Querscrollen', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  fehlerSammeln(page, { serverWegErwartet: true });
  await page.setViewportSize({ width: 320, height: 640 });
  await beitreten(page);
  await listeZeigen(page);

  await instanz.beenden();
  const hinweis = page.locator('#lc-verbindung');
  await expect(hinweis).toBeVisible();

  const lage = await hinweis.evaluate((element) => {
    const rechteck = element.getBoundingClientRect();
    const oben = document.elementFromPoint(rechteck.left + rechteck.width / 2, rechteck.top + rechteck.height / 2);
    return { vorn: element.contains(oben), inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth };
  });
  expect(lage.vorn).toBe(true);
  expect(lage.inhalt).toBeLessThanOrEqual(lage.fenster);
});

test('S5: Server beendet — nach < 2 s ist die Liste als „Stand unbekannt“ markiert, kein ● mehr unmarkiert', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  fehlerSammeln(page, { serverWegErwartet: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  await agentMitStrom('claude');
  await beitreten(page);
  await expect(region(page).locator('.lc-dot.on')).toHaveCount(2);

  await instanz.beenden();

  await expect(region(page).locator('.lc-stand')).toHaveText(/Stand unbekannt/, { timeout: 2000 });
  const unmarkiert = await page.evaluate(() =>
    [...document.querySelectorAll('.lc-dot.on')].filter((punkt) => punkt.closest('.lc-unbekannt') === null).length);
  expect(unmarkiert).toBe(0);
  const punkt = await region(page).locator('.lc-dot.on').first().evaluate((element) => getComputedStyle(element).backgroundColor);
  expect(punkt).toBe('rgba(0, 0, 0, 0)');
});

test('S6: der Alarmtext ändert sich nur beim Wechsel — 1 Mutation beim Unterbrechen, 1 beim Wiederverbinden', async ({ page }) => {
  test.setTimeout(90000);
  instanz = await instanzStarten({ port: await freierPort() });
  fehlerSammeln(page, { serverWegErwartet: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page);
  await expect(region(page).locator('.lc-dot.on')).toHaveCount(1);
  await page.locator('#lc-verbindung').evaluate((hinweis) => {
    window.alarmMutationen = [];
    new MutationObserver((liste) => window.alarmMutationen.push(...liste.map(() => hinweis.textContent)))
      .observe(hinweis, { childList: true, characterData: true, subtree: true });
  });

  await instanz.beenden();
  await page.waitForTimeout(30000);
  const waehrendAus = await page.evaluate(() => window.alarmMutationen.length);
  await instanz.starten();
  await expect(page.locator('#lc-verbindung')).toBeHidden({ timeout: 15000 });

  expect(waehrendAus).toBe(1);
  expect(await page.evaluate(() => window.alarmMutationen.length)).toBe(2);
});

test('O1: klappt die Liste durch Verkleinern ein, während der Fokus in ihr steht, geht der Fokus auf den Schalter', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page);
  await region(page).focus();

  await page.setViewportSize({ width: 1024, height: 800 });

  await expect(region(page)).toBeHidden();
  await expect(schalter(page)).toBeFocused();
});
