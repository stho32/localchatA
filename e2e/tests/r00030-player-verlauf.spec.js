'use strict';
// R00030 F0165 Player und Verlauf (Desktop 1280 × 800, Gegeben über die API; US-22): solange ein Medium spielt, folgt der
// Verlauf neuen Nachrichten nicht (B0733, product-owner Zeile 81, U1, UX V7). Vorher liegen 60 ältere Nachrichten im
// Channel — der Verlauf ist höher als das Fenster, ein Folgen wäre sichtbar.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, browserOeffnen, imBrowserSenden } = require('./r00041-hilfen');
const { imBrowserBeitreten } = require('./senden-hilfen');
const { anzeigeStarten } = require('./r00030-anzeige-hilfen');
const { player, abspielen, wiedergabe, aeltereNachrichten, betaSchreibt, scrollStand } = require('./r00030-player-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const pille = (page) => page.locator('.lc-pill');

/// Steht das Element ganz im sichtbaren Bereich des Verlaufs?
const imVerlaufSichtbar = (locator) => locator.evaluate((element) => {
  const log = element.closest('[role="log"]').getBoundingClientRect();
  const rahmen = element.getBoundingClientRect();
  return rahmen.top >= log.top - 0.5 && rahmen.bottom <= log.bottom + 0.5;
});

async function videoUnten(browser) {
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: ['film.webm'] }], { vorbereiten: aeltereNachrichten(60) });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const video = player(nachrichtMitZeit(page, lauf.zeiten[0]), 'film.webm');
  await expect.poll(async () => (await scrollStand(page)).untenAbstand).toBeLessThanOrEqual(1);
  return { lauf, page, video };
}

test('Verlauf folgt nicht, solange ein Player spielt (B0733, U1): B spielt film.webm unten, beta sendet drei → Player bleibt an seiner Stelle sichtbar, kein Scrollen, Pille „3 neue Nachrichten“; Pause springt nicht, die nächste wird nur gezählt', async ({ browser }) => {
  const { lauf, page, video } = await videoUnten(browser);
  await abspielen(video);
  const vorher = await scrollStand(page);
  const zeiten = [];
  for (const text of ['eins', 'zwei', 'drei']) {
    zeiten.push(await betaSchreibt(lauf, text));
  }
  await expect(nachrichtMitZeit(page, zeiten[2])).toHaveCount(1);
  await expect(pille(page)).toBeVisible();
  await expect(pille(page)).toContainText('3 neue Nachrichten');
  expect((await scrollStand(page)).oben).toBe(vorher.oben);
  expect(await imVerlaufSichtbar(video)).toBe(true);
  expect((await wiedergabe(video)).pausiert).toBe(false);

  await video.evaluate((element) => element.pause());
  await page.waitForTimeout(300);
  expect((await scrollStand(page)).oben).toBe(vorher.oben);
  const vierte = await betaSchreibt(lauf, 'vier');
  await expect(nachrichtMitZeit(page, vierte)).toHaveCount(1);
  await expect(pille(page)).toContainText('4 neue Nachrichten');
  expect((await scrollStand(page)).oben).toBe(vorher.oben);
  expect(lauf.a.fehler).toEqual([]);
});

for (const ende of ['pause', 'ended']) {
  test(`Nach ${ende} am Ende (B0733, V7): der Verlauf springt dabei nicht und folgt der nächsten neuen Nachricht wieder`, async ({ browser }) => {
    const { lauf, page, video } = await videoUnten(browser);
    await abspielen(video);
    const vorher = await scrollStand(page);
    if (ende === 'pause') {
      await video.evaluate((element) => element.pause());
    } else {
      await video.evaluate((element) => {
        element.currentTime = element.duration - 0.2;
      });
      await expect.poll(() => video.evaluate((element) => element.ended), { timeout: 10000 }).toBe(true);
    }
    await page.waitForTimeout(300);
    expect((await scrollStand(page)).oben).toBe(vorher.oben);
    const neue = await betaSchreibt(lauf, 'danach');
    await expect(nachrichtMitZeit(page, neue)).toHaveCount(1);
    await expect.poll(async () => (await scrollStand(page)).untenAbstand).toBeLessThanOrEqual(1);
    expect(await imVerlaufSichtbar(nachrichtMitZeit(page, neue))).toBe(true);
    await expect(pille(page)).toBeHidden();
    expect(lauf.a.fehler).toEqual([]);
  });
}

test('Eigenes Senden folgt auch bei Wiedergabe (B0733, V7, US-22): A (anna) sendet, während B spielt → bei A folgt der Verlauf, bei B läuft die Wiedergabe weiter; B sendet selbst → B folgt, die Wiedergabe läuft weiter', async ({ browser }) => {
  const { lauf, page, video } = await videoUnten(browser);
  const anna = await browserOeffnen(browser);
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await abspielen(video);
  const vorher = await scrollStand(page);

  const vonAnna = await imBrowserSenden(anna.page, 'von anna');
  await expect.poll(async () => (await scrollStand(anna.page)).untenAbstand).toBeLessThanOrEqual(1);
  await expect(nachrichtMitZeit(page, vonAnna)).toHaveCount(1);
  expect((await scrollStand(page)).oben).toBe(vorher.oben);
  const zeitVorher = (await wiedergabe(video)).zeit;
  await expect.poll(async () => (await wiedergabe(video)).zeit).toBeGreaterThan(zeitVorher);
  expect((await wiedergabe(video)).pausiert).toBe(false);

  const vonB = await imBrowserSenden(page, 'von B');
  await expect.poll(async () => (await scrollStand(page)).untenAbstand).toBeLessThanOrEqual(1);
  expect(await imVerlaufSichtbar(nachrichtMitZeit(page, vonB))).toBe(true);
  expect((await wiedergabe(video)).pausiert).toBe(false);
  expect(lauf.a.fehler).toEqual([]);
  expect(anna.fehler).toEqual([]);
});
