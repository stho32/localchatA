'use strict';
// R00004 F0013 Teilnehmerliste im Browser (US-14 bis US-19): gegen das echte Programm, Chromium, je Kontext
// ein Browser; in jedem Szenario keine Konsolenfehler.
const { test, expect } = require('@playwright/test');
const { instanzStarten, freierPort, lanAdresse, fehlerSammeln, beitretenPerApi, fokusImBrowser } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const region = (page) => page.getByRole('complementary', { name: 'Teilnehmer' });
const zeileVon = (page, kennung) => region(page).locator('.lc-person').filter({ has: page.locator('.lc-id', { hasText: kennung }) });

async function beitreten(page, name, url = instanz.adresse) {
  await page.goto(url);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
}

async function istOnline(zeile) {
  await expect(zeile.locator('.lc-dot')).toHaveClass(/\bon\b/);
  await expect(zeile.locator('.lc-status')).toHaveCount(0);
}

async function istOffline(zeile, hoechstensMs) {
  await expect(zeile.locator('.lc-status')).toHaveText('offline', { timeout: hoechstensMs });
  await expect(zeile.locator('.lc-dot')).toHaveClass(/\boff\b/);
}

async function seiteMitFehlern(browser) {
  const kontext = await browser.newContext();
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page) };
}

test('US-14: zwei Menschen sehen sich unter „Menschen“ mit ●, die eigene Zeile ist „(du)“', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await seiteMitFehlern(browser);
  const b = await seiteMitFehlern(browser);

  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'mia');

  for (const { page, eigene } of [{ page: a.page, eigene: 'shoff@127.0.0.1' }, { page: b.page, eigene: 'mia@127.0.0.1' }]) {
    await expect(region(page).locator('.lc-sect').first()).toHaveText('Menschen · 2');
    await expect(region(page).locator('.lc-id')).toHaveText(['mia@127.0.0.1', 'shoff@127.0.0.1']);
    await istOnline(zeileVon(page, 'mia@127.0.0.1'));
    await istOnline(zeileVon(page, 'shoff@127.0.0.1'));
    await expect(region(page).locator('.lc-du')).toHaveCount(1);
    await expect(zeileVon(page, eigene).locator('.lc-du')).toHaveText('(du)');
  }
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('US-15: Browser B zu → A zeigt mia in < 2 s offline, nach der Gnadenfrist ist die Zeile in < 5 s weg', async ({ browser }) => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_GNADENFRIST_MS: '3000' } });
  const a = await seiteMitFehlern(browser);
  const b = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'mia');
  await istOnline(zeileVon(a.page, 'mia@127.0.0.1'));

  const geschlossen = Date.now();
  await b.kontext.close();
  await istOffline(zeileVon(a.page, 'mia@127.0.0.1'), 2000);
  const offlineNach = Date.now() - geschlossen;
  await expect(zeileVon(a.page, 'mia@127.0.0.1')).toHaveCount(0, { timeout: 5000 - (Date.now() - geschlossen) });

  expect(offlineNach).toBeLessThan(2000);
  await expect(region(a.page).locator('.lc-id')).toHaveText(['shoff@127.0.0.1']);
  expect(a.fehler).toEqual([]);
});

test('US-16: sieben Agenten ohne Unterkopf mit „Agent“, ab dem achten nach Herkunft gruppiert', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await beitreten(page, 'shoff');

  for (const name of ['a1', 'a2', 'a3', 'a4', 'a5', 'a6', 'a7']) {
    expect((await beitretenPerApi(instanz.adresse, name)).status).toBe(201);
  }
  await expect(region(page).locator('.lc-sect')).toHaveText(['Menschen · 1', 'Agenten · 7']);
  const agenten = region(page).locator('.lc-person').filter({ has: page.locator('.lc-tag', { hasText: 'Agent' }) });
  await expect(agenten).toHaveCount(7);
  await expect(agenten.locator('.lc-id')).toHaveText(['a1', 'a2', 'a3', 'a4', 'a5', 'a6', 'a7'].map((n) => `${n}@127.0.0.1`));

  expect((await beitretenPerApi(instanz.adresse, 'a8', '127.0.0.2')).status).toBe(201);
  await expect(region(page).locator('.lc-sect')).toHaveText(['Menschen · 1', 'Agenten · 8', '127.0.0.1', '127.0.0.2']);
  const gruppiert = await region(page).evaluate((element) => [...element.children].map((kind) => kind.textContent));
  expect(gruppiert.slice(gruppiert.indexOf('127.0.0.1') + 1, gruppiert.indexOf('127.0.0.2'))).toHaveLength(7);
  expect(gruppiert.slice(gruppiert.indexOf('127.0.0.2') + 1)).toHaveLength(1);
  expect(fehler).toEqual([]);
});

test('US-17: Programm beendet → ⚠ in Fehlerfarbe; neu gestartet → ohne Neuladen wieder ● „(du)“, Hinweis weg', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  const fehler = fehlerSammeln(page, { serverWegErwartet: true });
  await beitreten(page, 'shoff');
  await istOnline(zeileVon(page, 'shoff@127.0.0.1'));
  const hinweis = page.locator('#lc-verbindung');
  await expect(hinweis).toBeHidden();
  await page.evaluate(() => { window.nichtNeuGeladen = true; });

  await instanz.beenden();
  await expect(hinweis).toBeVisible();
  await expect(hinweis).toHaveText(/^⚠ Verbindung unterbrochen/);
  await expect(hinweis).toHaveClass(/\blc-fehler\b/);
  const farbe = await hinweis.evaluate((element) => ({
    text: getComputedStyle(element).color,
    gefahr: getComputedStyle(element).getPropertyValue('--kd-danger').trim(),
  }));
  await instanz.starten();

  await expect(hinweis).toBeHidden({ timeout: 15000 });
  await istOnline(zeileVon(page, 'shoff@127.0.0.1'));
  await expect(zeileVon(page, 'shoff@127.0.0.1').locator('.lc-du')).toHaveText('(du)');
  expect(await page.evaluate(() => window.nichtNeuGeladen)).toBe(true);
  expect(farbe.text).toBe(hexAlsRgb(farbe.gefahr));
  expect(fehler).toEqual([]);
});

function hexAlsRgb(hex) {
  const [r, g, b] = [1, 3, 5].map((stelle) => parseInt(hex.slice(stelle, stelle + 2), 16));
  return `rgb(${r}, ${g}, ${b})`;
}

for (const thema of ['light', 'dark']) {
  test(`US-18: die Region „Teilnehmer“ ist per Tab erreichbar, Fokusrahmen ≥ 3:1 — ${thema}`, async ({ page }) => {
    instanz = await instanzStarten();
    await page.emulateMedia({ colorScheme: thema });
    await page.setViewportSize({ width: 1280, height: 800 });
    await beitreten(page, 'shoff');

    for (let schritt = 0; schritt < 10; schritt += 1) {
      if (await region(page).evaluate((element) => element === document.activeElement)) {
        break;
      }
      await page.keyboard.press('Tab');
    }

    await expect(region(page)).toBeFocused();
    const fokus = await page.evaluate(fokusImBrowser);
    expect(fokus.fokusSichtbar).toBe(true);
    expect(fokus.rahmen).toBe('solid 2px');
    expect(fokus.kontrast).toBeGreaterThanOrEqual(3);
  });
}

for (const breite of [320, 360, 375]) {
  test(`US-18: ${breite} px mit 32 Zeichen langem Agenten — kein Querscrollen, volle Kennung in der Zeile`, async ({ page }) => {
    instanz = await instanzStarten();
    const fehler = fehlerSammeln(page);
    const langerName = 'agent-mit-zweiunddreissig-zeichen'.slice(0, 32);
    expect(langerName).toHaveLength(32);
    expect((await beitretenPerApi(instanz.adresse, langerName)).status).toBe(201);
    await page.setViewportSize({ width: breite, height: 640 });
    await beitreten(page, 'shoff');
    await teilnehmerlisteZeigen(page);

    await expect(zeileVon(page, `${langerName}@127.0.0.1`)).toContainText(`${langerName}@127.0.0.1`);
    const masse = await page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));
    expect(masse.inhalt).toBeLessThanOrEqual(masse.fenster);
    expect(fehler).toEqual([]);
  });
}

// Unter 1100 px ist die Liste eingeklappt (F0015); `Strg .` blendet sie ein. Ohne F0015 ist sie immer sichtbar.
async function teilnehmerlisteZeigen(page) {
  if (!(await region(page).isVisible())) {
    await page.keyboard.press('Control+Period');
  }
  await expect(region(page)).toBeVisible();
}

test('US-18: Kennungen in der Liste in Mono wie die Kennung in der Kopfzeile', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page, 'shoff');
  await istOnline(zeileVon(page, 'shoff@127.0.0.1'));

  const schriften = await page.evaluate(() => ({
    liste: getComputedStyle(document.querySelector('.lc-members .lc-id')).fontFamily,
    kopf: getComputedStyle(document.querySelector('.lc-me')).fontFamily,
  }));

  expect(schriften.liste).toBe(schriften.kopf);
  expect(schriften.liste).toContain('Mono');
});

test('US-19: über die LAN-Adresse laden Beitreten und Liste ohne einen Konsolenfehler', async ({ page }) => {
  const lan = await lanAdresse();
  instanz = await instanzStarten({ host: '0.0.0.0' });
  const url = instanz.adresse.replace(/\/\/[^:/]+:/, `//${lan}:`);
  const fehler = fehlerSammeln(page);
  await page.setViewportSize({ width: 1280, height: 800 });

  await beitreten(page, 'shoff', url);

  const eigene = zeileVon(page, `shoff@${lan}`);
  await istOnline(eigene);
  await expect(eigene.locator('.lc-du')).toHaveText('(du)');
  await page.reload();
  await istOnline(zeileVon(page, `shoff@${lan}`));
  expect(fehler).toEqual([]);
});

// Abrufe aus einem SharedWorker sieht Playwright nicht (weder route noch request) — verzögert wird deshalb der
// Strom im Tab (Rückfall ohne SharedWorker); Zusammenführen und Zeichnen sind auf beiden Wegen dieselben.
test('verzögerter Strom: der erste Stand kommt 1 s später — die Seite steht, die Liste füllt sich danach', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.addInitScript(() => { delete window.SharedWorker; });
  let verzoegert = 0;
  await page.route('**/api/ereignisse', async (route) => {
    verzoegert += 1;
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.continue();
  });
  await page.setViewportSize({ width: 1280, height: 800 });

  await beitreten(page, 'shoff');
  await expect(region(page)).toBeVisible();
  const vorDemStand = await region(page).locator('.lc-person').count();
  await istOnline(zeileVon(page, 'shoff@127.0.0.1'));

  expect(verzoegert).toBe(1);
  expect(vorDemStand).toBe(0);
  await expect(page.locator('#lc-verbindung')).toBeHidden();
  expect(fehler).toEqual([]);
});

// Selbstprüfung gegen das Designsystem (AI-Team/entwickler.md, #3): bei üblichen Längen bleibt der Name in
// einer Zeile und die Herkunft in einer — umbrochen wird höchstens vor dem @, nie mitten im Namen.
test('Zeilen bleiben lesbar: der Name bricht nicht, umbrochen wird höchstens vor dem @', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 1280, height: 800 });
  for (const name of ['claude-entwickler', 'review-bot']) {
    expect((await beitretenPerApi(instanz.adresse, name)).status).toBe(201);
  }
  await beitreten(page, 'shoff');
  await istOnline(zeileVon(page, 'shoff@127.0.0.1'));
  await expect(region(page).locator('.lc-person')).toHaveCount(3);

  const zeilen = await region(page).locator('.lc-id').evaluateAll((kennungen) => kennungen.map((kennung) => {
    function zeilenVon(knoten) {
      const bereich = document.createRange();
      bereich.selectNodeContents(knoten);
      return new Set([...bereich.getClientRects()].map((rechteck) => Math.round(rechteck.top))).size;
    }
    return { kennung: kennung.textContent, name: zeilenVon(kennung.firstChild), herkunft: zeilenVon(kennung.querySelector('i')) };
  }));

  for (const zeile of zeilen) {
    expect(zeile, zeile.kennung).toEqual({ kennung: zeile.kennung, name: 1, herkunft: 1 });
  }
});

// Nacharbeit 1 (UX-Review): mit kurzer Gnadenfrist findet die Seite nach einem Neustart zurück — der Strom
// öffnet nach dem stillen Wiederbeitritt sofort, nicht erst nach dem Rückzug, und hält die neue Sitzung.
test('Neustart mit Gnadenfrist 3 s: Hinweis binnen ca. 3 s weg, Strom hält die neue Sitzung online', async ({ page, request }) => {
  instanz = await instanzStarten({ port: await freierPort(), umgebung: { LOCALCHATA_GNADENFRIST_MS: '3000' } });
  const fehler = fehlerSammeln(page, { serverWegErwartet: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page, 'shoff');
  await istOnline(zeileVon(page, 'shoff@127.0.0.1'));

  await instanz.beenden();
  await expect(page.locator('#lc-verbindung')).toBeVisible();
  await page.waitForTimeout(6000);
  await instanz.starten();
  const neu = Date.now();
  await expect(page.locator('#lc-verbindung')).toBeHidden({ timeout: 3500 });
  const wiederDa = Date.now() - neu;
  await page.waitForTimeout(4000);

  expect(wiederDa).toBeLessThan(3500);
  await istOnline(zeileVon(page, 'shoff@127.0.0.1'));
  await expect(page.locator('#lc-verbindung')).toBeHidden();
  expect((await (await request.get(instanz.adresse + 'api/gesundheit')).json()).halter).toBeGreaterThanOrEqual(1);
  expect(fehler).toEqual([]);
});
