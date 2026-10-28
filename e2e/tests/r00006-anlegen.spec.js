'use strict';
// R00006 F0025 Channel im Browser anlegen (US-19 bis US-23): gegen das echte Programm, Chromium. Konsole je
// Szenario wie im Kriterium: Fehlerantworten nur dort erlaubt, wo die Anforderung sie vorschreibt.
const { test, expect } = require('@playwright/test');
const { instanzStarten, fehlerSammeln, beitretenPerApi, channelAnlegenPerApi, fokusImBrowser } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const seitenleiste = (page) => page.getByRole('navigation', { name: 'Channels' });
const plus = (page) => seitenleiste(page).getByRole('button', { name: 'Channel anlegen' });
const dialog = (page) => page.getByRole('dialog', { name: 'Channel anlegen' });
const namensfeld = (page) => dialog(page).getByLabel('Name');
const fehlerzeile = (page) => dialog(page).locator('.lc-fehler');
const eintragVon = (page, name) => seitenleiste(page).locator(`.lc-item[title="${name}"]`);

async function beitreten(page, name) {
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
}

async function seiteMitFehlern(browser, optionen = {}) {
  const kontext = await browser.newContext();
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page, optionen) };
}

async function agent(name, herkunft) {
  const beitritt = await beitretenPerApi(instanz.adresse, name, herkunft);
  expect(beitritt.status).toBe(201);
  return (channel) => channelAnlegenPerApi(instanz.adresse, beitritt.daten.token, channel, herkunft);
}

async function aktiveBeschriftung(page) {
  return page.evaluate(() => document.activeElement.getAttribute('aria-label') || document.activeElement.textContent.trim());
}

async function gefahrenfarbe(page) {
  return page.evaluate(() => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--kd-danger)';
    document.body.append(probe);
    const farbe = getComputedStyle(probe).color;
    probe.remove();
    return farbe;
  });
}

test('US-19: Strg Alt N, Name, Enter — Dialog zu, Statuszeile, einmal in der Liste, Fokus zurück, B sieht ihn', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser);
  const b = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'mia');
  await a.page.getByRole('button', { name: /^Teilnehmer/ }).focus();

  await a.page.keyboard.press('Control+Alt+n');
  await expect(dialog(a.page)).toBeVisible();
  await expect(namensfeld(a.page)).toBeFocused();
  await a.page.keyboard.type('security-entwicklung');
  await a.page.keyboard.press('Enter');

  await expect(dialog(a.page)).toBeHidden();
  await expect(a.page.getByRole('status')).toHaveText('Channel # security-entwicklung angelegt.');
  await expect(eintragVon(a.page, 'security-entwicklung')).toHaveCount(1);
  await expect(a.page.getByRole('button', { name: /^Teilnehmer/ })).toBeFocused();
  await expect(eintragVon(b.page, 'security-entwicklung')).toHaveCount(1);
  await a.page.waitForTimeout(300);
  await expect(eintragVon(a.page, 'security-entwicklung')).toHaveCount(1);
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('US-20: Abbrechen legt nichts an — Esc und „Abbrechen“ schließen, Fokus auf +', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');

  for (const schliessen of [() => a.page.keyboard.press('Escape'), () => dialog(a.page).getByRole('button', { name: 'Abbrechen' }).click()]) {
    await plus(a.page).click();
    await expect(dialog(a.page)).toBeVisible();
    await namensfeld(a.page).fill('test');
    await schliessen();
    await expect(dialog(a.page)).toBeHidden();
    await expect(plus(a.page)).toBeFocused();
  }
  const beobachter = await beitretenPerApi(instanz.adresse, 'beobachter', '127.0.0.1');
  const liste = await a.page.request.get(`${instanz.adresse}api/channels`, { headers: { 'X-LocalchatA-Sitzung': beobachter.daten.token } });
  expect(await liste.json()).toEqual({ stand: 0, liste: [] });
  await expect(seitenleiste(a.page).locator('.lc-item')).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

test('US-21: ungültig mit „allgemein übernehmen“, dann vorhanden — Dialog bleibt offen, Eingabe bleibt', async ({ browser }) => {
  instanz = await instanzStarten();
  const alpha = await agent('alpha', '127.0.0.2');
  expect((await alpha('allgemein')).status).toBe(201);
  const a = await seiteMitFehlern(browser, { zusaetzlich: { '/api/channels': [409, 422] } });
  await beitreten(a.page, 'shoff');
  await plus(a.page).click();

  await namensfeld(a.page).fill('Allgemein');
  await namensfeld(a.page).press('Enter');

  await expect(fehlerzeile(a.page)).toBeVisible();
  await expect(fehlerzeile(a.page).locator('span').first()).toHaveText(/^⚠ /);
  expect(await fehlerzeile(a.page).locator('span').first().evaluate((element) => getComputedStyle(element).color)).toBe(await gefahrenfarbe(a.page));
  await expect(namensfeld(a.page)).toHaveAttribute('aria-invalid', 'true');
  const beschrieben = await namensfeld(a.page).getAttribute('aria-describedby');
  expect(beschrieben.split(' ')).toContain(await fehlerzeile(a.page).getAttribute('id'));
  await dialog(a.page).getByRole('button', { name: 'allgemein übernehmen' }).click();
  await expect(namensfeld(a.page)).toHaveValue('allgemein');
  await namensfeld(a.page).press('Enter');

  await expect(fehlerzeile(a.page).locator('span').first()).toHaveText('⚠ # allgemein gibt es schon.');
  await expect(dialog(a.page)).toBeVisible();
  await expect(namensfeld(a.page)).toHaveValue('allgemein');
  expect(a.fehler).toEqual([]);
});

test('US-22: an der Grenze sagt der Dialog „Zu viele Channels“ mit 32', async ({ browser }) => {
  instanz = await instanzStarten();
  const lokal = await agent('lokal', '127.0.0.1');
  for (let nummer = 1; nummer <= 32; nummer += 1) {
    expect((await lokal(`c${String(nummer).padStart(2, '0')}`)).status).toBe(201);
  }
  const a = await seiteMitFehlern(browser, { zusaetzlich: { '/api/channels': [429] } });
  await beitreten(a.page, 'shoff');
  await plus(a.page).click();

  await namensfeld(a.page).fill('noch-einer');
  await namensfeld(a.page).press('Enter');

  await expect(fehlerzeile(a.page).locator('span').first()).toHaveText(/^⚠ Zu viele Channels.*32/);
  await expect(dialog(a.page)).toBeVisible();
  expect(a.fehler).toEqual([]);
});

test('US-23: ist der Server weg, bleibt die Eingabe stehen', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser, { serverWegErwartet: true });
  await beitreten(a.page, 'shoff');
  await plus(a.page).click();
  await namensfeld(a.page).fill('spaeter');

  await instanz.beenden();
  await namensfeld(a.page).press('Enter');

  await expect(fehlerzeile(a.page).locator('span').first()).toHaveText('⚠ Nicht angelegt — Verbindung prüfen.');
  await expect(namensfeld(a.page)).toHaveValue('spaeter');
  await expect(dialog(a.page)).toBeVisible();
});

test('verzögerte Antwort (1 s): der Dialog bleibt bis zur 201 offen, dann Statuszeile und Fokus auf +', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await a.page.route('**/api/channels', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.continue();
  });
  await plus(a.page).click();

  await namensfeld(a.page).fill('langsam');
  await namensfeld(a.page).press('Enter');
  await a.page.waitForTimeout(500);
  await expect(dialog(a.page)).toBeVisible();

  await expect(dialog(a.page)).toBeHidden({ timeout: 5000 });
  await expect(a.page.getByRole('status')).toHaveText('Channel # langsam angelegt.');
  await expect(plus(a.page)).toBeFocused();
  await expect(eintragVon(a.page, 'langsam')).toHaveCount(1);
  expect(a.fehler).toEqual([]);
});

test('schmal: bei 320 px ist der Dialog ohne Querscrollen bedienbar, auch mit Fehlerzeile und Vorschlag', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser, { zusaetzlich: { '/api/channels': [422] } });
  await a.page.setViewportSize({ width: 320, height: 640 });
  await beitreten(a.page, 'shoff');
  await plus(a.page).click();
  const passt = () => a.page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);

  expect(await passt()).toBe(true);
  await expect(dialog(a.page).getByRole('button', { name: 'Channel anlegen' })).toBeInViewport();
  await namensfeld(a.page).fill('Ein sehr langer Name mit Leerzeichen und Ümläuten');
  await namensfeld(a.page).press('Enter');
  await expect(fehlerzeile(a.page)).toBeVisible();
  await expect(dialog(a.page).getByRole('button', { name: /übernehmen$/ })).toBeVisible();

  expect(await passt()).toBe(true);
  await expect(dialog(a.page).getByRole('button', { name: 'Channel anlegen' })).toBeInViewport();
  expect(a.fehler).toEqual([]);
});

for (const thema of ['light', 'dark']) {
  test(`Gestaltung ${thema}: Fokus auf „Channel anlegen“ sichtbar (≥ 3:1), Fehlertext ≥ 4,5:1 auf dem Dialog`, async ({ browser }) => {
    instanz = await instanzStarten();
    const a = await seiteMitFehlern(browser, { zusaetzlich: { '/api/channels': [422] } });
    await a.page.emulateMedia({ colorScheme: thema });
    await beitreten(a.page, 'shoff');
    await plus(a.page).focus();
    await a.page.keyboard.press('Enter');
    await namensfeld(a.page).fill('Allgemein');
    await namensfeld(a.page).press('Enter');
    await expect(fehlerzeile(a.page)).toBeVisible();

    await a.page.keyboard.press('Tab');
    await a.page.keyboard.press('Tab');
    await a.page.keyboard.press('Tab');
    const fokus = await a.page.evaluate(fokusImBrowser);
    const fehlerKontrast = await fehlerzeile(a.page).locator('span').first().evaluate((element) => {
      const anteile = (farbe) => farbe.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number);
      const leuchtdichte = (farbe) => {
        const [r, g, b] = anteile(farbe).map((wert) => {
          const kanal = wert / 255;
          if (kanal <= 0.03928) {
            return kanal / 12.92;
          }
          return ((kanal + 0.055) / 1.055) ** 2.4;
        });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
      };
      const flaeche = getComputedStyle(element.closest('dialog')).backgroundColor;
      const vorne = leuchtdichte(getComputedStyle(element).color);
      const hinten = leuchtdichte(flaeche);
      return (Math.max(vorne, hinten) + 0.05) / (Math.min(vorne, hinten) + 0.05);
    });

    expect(await aktiveBeschriftung(a.page)).toBe('Channel anlegen');
    expect(fokus.fokusSichtbar).toBe(true);
    expect(fokus.rahmen).toMatch(/^solid [1-9]/);
    expect(fokus.kontrast).toBeGreaterThanOrEqual(3);
    expect(fehlerKontrast).toBeGreaterThanOrEqual(4.5);
    expect(a.fehler).toEqual([]);
  });
}
