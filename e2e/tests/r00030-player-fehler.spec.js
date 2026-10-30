'use strict';
// R00030 F0165 Nacharbeit (ux-reviewer #43 F0165 S1, S2): spielt der Browser ein Medium nicht ab (Original 404), steht der
// Hinweis „Kann im Browser nicht abgespielt werden — „Laden“ nutzen.“ ganz in der Player-Fläche, auch bei 320 px; hatte das
// Medium den Fokus, geht er auf die Nachricht und der Hinweis wird einmal angesagt. Gegeben über die API.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, ansagenMitschreiben } = require('./r00041-hilfen');
const { anzeigeStarten } = require('./r00030-anzeige-hilfen');
const { player } = require('./r00030-player-hilfen');

const HINWEIS = 'Kann im Browser nicht abgespielt werden — „Laden“ nutzen.';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const GERAETE = [
  ['320 Touch', { viewport: { width: 320, height: 568 }, hasTouch: true, isMobile: true }],
  ['360 Touch', { viewport: { width: 360, height: 640 }, hasTouch: true, isMobile: true }],
  ['375 Touch', { viewport: { width: 375, height: 667 }, hasTouch: true, isMobile: true }],
  ['Desktop', { viewport: { width: 1280, height: 800 } }],
];

async function original404(page) {
  await page.route('**/anhaenge/**', (route) => route.fulfill({ status: 404, body: '' }));
}

for (const [titel, geraet] of GERAETE) {
  test(`Audio-Fehlertext ganz in der Zeile (ux S1, ${titel}): ton.ogg 404, play() → der Text liegt in der Player-Fläche, ohne Abschneiden, die Zeile bleibt mindestens 54 px hoch`, async ({ browser }) => {
    const lauf = await anzeigeStarten(browser, [{ namen: ['ton.ogg'] }], { geraet });
    instanz = lauf.instanz;
    const { page } = lauf.a;
    const flaeche = nachrichtMitZeit(page, lauf.zeiten[0]).locator('.lc-player-audio');
    const hoeheVorher = (await flaeche.boundingBox()).height;
    await original404(page);

    await page.evaluate(() => document.querySelector('audio').play().catch(() => {}));

    await expect(flaeche).toHaveAttribute('data-zustand', 'fehler');
    const lage = await flaeche.evaluate((element) => {
      const fehler = element.querySelector('.lc-player-fehler');
      const bereich = document.createRange();
      bereich.selectNodeContents(fehler);
      const text = bereich.getBoundingClientRect();
      const rahmen = element.getBoundingClientRect();
      return { textOben: text.top, textUnten: text.bottom, oben: rahmen.top, unten: rahmen.bottom, hoehe: rahmen.height, gescrollt: fehler.scrollHeight - fehler.clientHeight, text: fehler.textContent };
    });
    expect(lage.text).toBe(HINWEIS);
    expect(lage.textOben, 'Oberkante des Texts').toBeGreaterThanOrEqual(lage.oben - 0.5);
    expect(lage.textUnten, 'Unterkante des Texts').toBeLessThanOrEqual(lage.unten + 0.5);
    expect(lage.gescrollt, 'scrollHeight − clientHeight').toBeLessThanOrEqual(0);
    expect(lage.hoehe, 'Höhe der Zeile').toBeGreaterThanOrEqual(hoeheVorher - 1);
    expect(lauf.a.fehler.filter((meldung) => !/404/.test(meldung))).toEqual([]);
  });
}

test('Fehler angesagt, Fokus bleibt sichtbar (ux S2): film.webm 404, Fokus auf dem Video, Leertaste → der Fokus liegt auf einem sichtbaren Element derselben Nachricht, der Hinweis wird genau einmal angesagt', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['film.webm'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  await original404(page);
  const ansagen = await ansagenMitschreiben(page);
  await player(knoten, 'film.webm').focus();

  await page.keyboard.press('Space');
  await page.waitForTimeout(1500);

  await expect(knoten.locator('.lc-player-video')).toHaveAttribute('data-zustand', 'fehler');
  const fokus = await page.evaluate((zeit) => {
    const aktiv = document.activeElement;
    const lage = aktiv.getBoundingClientRect();
    return {
      inNachricht: aktiv.closest(`.lc-msg[data-zeit="${zeit}"]`) !== null,
      sichtbar: getComputedStyle(aktiv).visibility !== 'hidden' && lage.width > 0 && lage.height > 0,
      tag: aktiv.tagName,
    };
  }, lauf.zeiten[0]);
  expect(fokus, JSON.stringify(fokus)).toEqual({ inNachricht: true, sichtbar: true, tag: 'ARTICLE' });
  expect((await ansagen()).filter((einzeln) => einzeln.includes(HINWEIS))).toEqual([HINWEIS]);
});

test('Ohne Fokus im Medium keine Ansage (ux S2): film.webm 404, Fokus in der Eingabe, play() → der Hinweis steht da, die Region sagt nichts, der Fokus bleibt in der Eingabe', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['film.webm'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  await original404(page);
  const eingabe = page.getByRole('textbox', { name: /^Nachricht an # / });
  await eingabe.focus();
  const ansagen = await ansagenMitschreiben(page);

  await page.evaluate(() => document.querySelector('video').play().catch(() => {}));

  await expect(knoten.locator('.lc-player-video')).toHaveAttribute('data-zustand', 'fehler');
  await page.waitForTimeout(800);
  expect((await ansagen()).filter((einzeln) => einzeln.includes(HINWEIS))).toEqual([]);
  await expect(eingabe).toBeFocused();
});
