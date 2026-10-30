'use strict';
// R00030 F0165 Nacharbeit (code-reviewer #43 F0165 S1): der Player im jüngsten Block überlebt, wenn der Block neu gezeichnet
// wird — Probe wie R00012 „B1 (2)“ (Ereignisse verschluckt, NeuerStand, während des gehaltenen Abrufs kommt „live“),
// mit einem spielenden Video im jüngsten Block. Desktop 1280 × 800, Gegeben über die API.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { senden, nachrichten } = require('./senden-hilfen');
const { bestaetigt, stromProbeEinrichten } = require('./r00012-hilfen');
const { anzeigeStarten } = require('./r00030-anzeige-hilfen');
const { player, abspielen, wiedergabe, elementMerken, istDasselbe, langesVideo } = require('./r00030-player-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

test('Jüngster Block neu gezeichnet (code S1): B spielt film.webm; Ereignisse verschluckt, NeuerStand, während des gehaltenen Abrufs „live“ → s1, s2, live je einmal in Zeitfolge, dasselbe Video-Element, paused false, currentTime nicht zurück', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: [langesVideo()] }], { vorSeite: (page) => stromProbeEinrichten(page.context()) });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const video = player(nachrichtMitZeit(page, lauf.zeiten[0]), 'film.webm');
  await abspielen(video);
  await elementMerken(video, 'gemerkterPlayer');
  const zeitVorher = (await wiedergabe(video)).zeit;
  await page.evaluate(() => { window.lcProbe.verschlucken = true; });
  for (const text of ['s1', 's2']) {
    expect((await senden(instanz.adresse, lauf.beta.token, 'allgemein', text, '127.0.0.2')).status).toBe(201);
  }
  await page.waitForFunction(() => window.lcProbe.verschluckt === 2);
  await page.evaluate(() => { window.lcProbe.verschlucken = false; });
  await page.route((adresse) => adresse.searchParams.has('nach'), async (route) => {
    const antwort = await route.fetch();
    expect((await senden(instanz.adresse, lauf.beta.token, 'allgemein', 'live', '127.0.0.2')).status).toBe(201);
    await expect(bestaetigt(page, 'live')).toHaveCount(1);
    await route.fulfill({ response: antwort });
  });

  await page.evaluate(() => window.lcProbe.standWiederholen());

  await expect(bestaetigt(page, '(s1|s2|live)').locator('.lc-body')).toHaveText(['s1', 's2', 'live'], { timeout: 10000 });
  await page.waitForTimeout(500);
  await expect(bestaetigt(page, '(s1|s2|live)').locator('.lc-body')).toHaveText(['s1', 's2', 'live']);
  await expect(nachrichten(page)).toHaveCount(4);
  expect(await istDasselbe(video, 'gemerkterPlayer'), 'dasselbe Element').toBe(true);
  const nachher = await wiedergabe(video);
  expect(nachher.pausiert, 'paused').toBe(false);
  expect(nachher.zeit, 'currentTime').toBeGreaterThanOrEqual(zeitVorher);
  expect(lauf.a.fehler).toEqual([]);
});
