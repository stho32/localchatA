'use strict';
// R00030 F0165 „Video und Audio“ (B0460, US-12) in Playwright-Chromium und Google Chrome stable (Projekt `chrome-stabil`,
// ADR-0014 Ergänzung „Karte #71 (Chrome 154)“ C6): der Player im Verlauf bettet das Medium im Ursprung der Instanz ein,
// lädt nichts vor dem Abspielen, spielt und springt über einen Teilabruf; die App-CSP lässt `media-src 'self'` zu.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { anzeigeStarten, dateiZeile } = require('./r00030-anzeige-hilfen');
const { player, abspielen, wiedergabe, anhangAnfragenMitschreiben } = require('./r00030-player-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const rechteck = (locator) => locator.evaluate((element) => {
  const { width, height } = element.getBoundingClientRect();
  return { width, height };
});

test('Video und Audio (B0460, US-12): film.webm im Rahmen 360 × 202, ton.ogg in einer Zeile, ohne autoplay; vor dem Abspielen keine Anfrage; spielt, springt ans Ende mit Range und 206; Konsole ohne CORS und CSP', async ({ browser }) => {
  test.setTimeout(60000);
  const lauf = await anzeigeStarten(browser, [{ namen: ['film.webm'] }, { namen: ['ton.ogg'] }, { namen: ['film.ogv'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const anfragen = anhangAnfragenMitschreiben(page);
  const video = player(nachrichtMitZeit(page, lauf.zeiten[0]), 'film.webm');
  const audio = player(nachrichtMitZeit(page, lauf.zeiten[1]), 'ton.ogg');
  await expect(video).toHaveCount(1);
  await expect(audio).toHaveCount(1);
  for (const [medium, tag] of [[video, 'VIDEO'], [audio, 'AUDIO']]) {
    const merkmale = await medium.evaluate((element) => ({
      tag: element.tagName, controls: element.controls, preload: element.getAttribute('preload'), autoplay: element.hasAttribute('autoplay'),
      playsinline: element.hasAttribute('playsinline'), pausiert: element.paused,
    }));
    expect(merkmale).toEqual({ tag, controls: true, preload: 'none', autoplay: false, playsinline: tag === 'VIDEO', pausiert: true });
  }
  expect(await video.evaluate((element) => [element.getAttribute('width'), element.getAttribute('height')])).toEqual(['360', '202']);
  const rahmen = await rechteck(video);
  expect(Math.abs(rahmen.width - 360)).toBeLessThanOrEqual(1);
  expect(Math.abs(rahmen.height - 202)).toBeLessThanOrEqual(1);
  await expect(dateiZeile(nachrichtMitZeit(page, lauf.zeiten[0]), 'film.webm').getByRole('link', { name: 'film.webm laden', exact: true })).toHaveCount(1);
  await expect(nachrichtMitZeit(page, lauf.zeiten[2]).locator('video, audio')).toHaveCount(0);
  await page.waitForTimeout(500);
  expect(anfragen).toEqual([]);

  for (const medium of [video, audio]) {
    await abspielen(medium);
    expect((await wiedergabe(medium)).fehler).toBeNull();
    await medium.evaluate((element) => {
      element.currentTime = element.duration - 0.1;
    });
    await expect.poll(async () => {
      const { zeit, dauer } = await wiedergabe(medium);
      return dauer - zeit;
    }, { timeout: 15000 }).toBeLessThanOrEqual(0.2);
  }
  expect(anfragen.some(({ bereich, status }) => bereich !== undefined && status === 206), JSON.stringify(anfragen)).toBe(true);
  expect(lauf.a.fehler.filter((zeile) => /CORS|Content Security Policy|ERR_BLOCKED/.test(zeile))).toEqual([]);
  expect(lauf.a.fehler).toEqual([]);
});
