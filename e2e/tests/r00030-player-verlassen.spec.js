'use strict';
// R00030 F0165 Wiedergabe beim Verlassen (B0754; UX V7, product-owner Zeile 162; US-27), Desktop 1280 × 800, Gegeben
// über die API: der Channelwechsel hält jedes spielende Medium an, ohne Ansage und ohne Bedienfeld; zurück steht dasselbe
// Element angehalten an derselben Stelle. Ein Block mit spielendem Medium wird beim Scrollen nicht entladen.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, ansagenMitschreiben } = require('./r00041-hilfen');
const { ruhig } = require('./verlauf-hilfen');
const { anzeigeStarten } = require('./r00030-anzeige-hilfen');
const { player, abspielen, wiedergabe, elementMerken, istDasselbe, aeltereNachrichten, langesVideo, langerTon } = require('./r00030-player-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function channelWaehlen(page, name) {
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', `Verlauf # ${name}`);
}

test('Wiedergabe beim Verlassen (B0754, V7, US-27): B spielt film.webm und ton.ogg, wechselt nach # technik → beide paused, ohne Ansage, ohne Bedienfeld; zurück → dasselbe Element, angehalten an derselben Stelle, spielt nicht von selbst', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'Medien', namen: [langesVideo(), langerTon()] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = () => nachrichtMitZeit(page, lauf.zeiten[0]);
  const medien = [['film.webm', 'gemerktesVideo'], ['ton.ogg', 'gemerkterTon']];
  for (const [name, marke] of medien) {
    await abspielen(player(knoten(), name));
    await elementMerken(player(knoten(), name), marke);
  }
  const ansagen = await ansagenMitschreiben(page);
  const regionVorher = await page.locator('[role="status"][aria-live="polite"]').first().textContent();

  await channelWaehlen(page, 'technik');
  const angehalten = await page.evaluate(() => [window.gemerktesVideo, window.gemerkterTon].map((element) => ({
    pausiert: element.paused, zeit: element.currentTime,
  })));
  expect(angehalten.map(({ pausiert }) => pausiert)).toEqual([true, true]);
  await expect(page.locator('video, audio')).toHaveCount(0);
  expect(await ansagen()).toEqual([]);
  expect(await page.locator('[role="status"][aria-live="polite"]').first().textContent()).toBe(regionVorher);

  await channelWaehlen(page, 'allgemein');
  await expect(knoten()).toHaveCount(1);
  for (const [stelle, [name, marke]] of medien.entries()) {
    const medium = player(knoten(), name);
    expect(await istDasselbe(medium, marke), `${name}: dasselbe Element`).toBe(true);
    const { pausiert, zeit } = await wiedergabe(medium);
    expect(pausiert, name).toBe(true);
    expect(Math.abs(zeit - angehalten[stelle].zeit), name).toBeLessThanOrEqual(0.05);
  }
  await page.waitForTimeout(1000);
  expect((await wiedergabe(player(knoten(), 'film.webm'))).pausiert).toBe(true);
  expect(await ansagen()).toEqual([]);
  expect(lauf.a.fehler).toEqual([]);
});

test('Nicht entladen (B0754, V7, US-27): B spielt film.webm ganz unten in einem Channel mit 800 älteren Nachrichten und scrollt mehr als ein Fenster hinauf, bis oben alles geladen ist → das Element bleibt im DOM und spielt', async ({ browser }) => {
  test.setTimeout(90000);
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: [langesVideo()] }], { vorbereiten: aeltereNachrichten(800) });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const video = player(nachrichtMitZeit(page, lauf.zeiten[0]), 'film.webm');
  await abspielen(video);
  await elementMerken(video, 'gemerktesVideo');
  const nachOben = [];
  page.on('request', (anfrage) => {
    if (new URL(anfrage.url()).search.startsWith('?vor=')) {
      nachOben.push(anfrage.url());
    }
  });
  const log = page.getByRole('log');
  for (let runde = 0; runde < 60 && nachOben.length < 7; runde += 1) {
    await log.evaluate((element) => { element.scrollTop = 0; });
    await ruhig(page);
  }
  expect(nachOben.length).toBeGreaterThanOrEqual(7);
  expect(await page.evaluate(() => window.gemerktesVideo.isConnected)).toBe(true);
  expect(await page.evaluate(() => window.gemerktesVideo.paused)).toBe(false);
  // Grenze (code S4): das Fenster bleibt bei 8 Blöcken (der erste und 7 geholte) — der Block mit dem spielenden Video ist
  // gegen das Entladen geschützt, ein 8. Abruf nach oben bleibt aus, solange es spielt; nach der Pause geht es weiter.
  for (let runde = 0; runde < 5; runde += 1) {
    await log.evaluate((element) => { element.scrollTop = 0; });
    await ruhig(page);
  }
  expect(nachOben.length, 'weiterer Abruf nach oben bei laufendem Video').toBe(7);
  expect(await page.evaluate(() => window.gemerktesVideo.isConnected)).toBe(true);
  await page.evaluate(() => window.gemerktesVideo.pause());
  for (let runde = 0; runde < 10 && nachOben.length < 8; runde += 1) {
    await log.evaluate((element) => { element.scrollTop = 400; });
    await ruhig(page);
    await log.evaluate((element) => { element.scrollTop = 0; });
    await ruhig(page);
  }
  expect(nachOben.length, 'nach der Pause weiter nach oben').toBeGreaterThanOrEqual(8);
  expect(lauf.a.fehler).toEqual([]);
});

test('Gescheiterte Rückkehr (code S2): B spielt film.webm in allgemein, wechselt nach technik; der erste Abruf von allgemein bei der Rückkehr scheitert („Verlauf nicht geladen“), B wählt allgemein erneut → dasselbe Element, angehalten an derselben Stelle', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: [langesVideo()] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = () => nachrichtMitZeit(page, lauf.zeiten[0]);
  await abspielen(player(knoten(), 'film.webm'));
  await elementMerken(player(knoten(), 'film.webm'), 'gemerktesVideo');
  await channelWaehlen(page, 'technik');
  const zeitBeimVerlassen = await page.evaluate(() => window.gemerktesVideo.currentTime);
  let abgebrochen = false;
  await page.route(/\/api\/channels\/allgemein\/nachrichten\?/, async (route) => {
    if (abgebrochen) {
      await route.continue();
      return;
    }
    abgebrochen = true;
    await route.abort();
  });

  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'allgemein', exact: true }).click();
  await expect(page.getByRole('alert').filter({ hasText: 'Verlauf nicht geladen' })).toBeVisible();
  await channelWaehlen(page, 'allgemein');

  await expect(knoten()).toHaveCount(1);
  const medium = player(knoten(), 'film.webm');
  expect(await istDasselbe(medium, 'gemerktesVideo'), 'dasselbe Element').toBe(true);
  const { pausiert, zeit } = await wiedergabe(medium);
  expect(pausiert).toBe(true);
  expect(Math.abs(zeit - zeitBeimVerlassen)).toBeLessThanOrEqual(0.05);
});
