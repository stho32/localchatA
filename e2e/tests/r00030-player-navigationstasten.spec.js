'use strict';
// R00030 F0185 / B0800 „Kürzel des Verlaufs lösen im Player nicht aus“ (PO #85, Zeile 182): hat der Fokus ein Video oder Audio
// im Verlauf, bewirken Pos1, Ende, Bild auf und Bild ab keinen Sprung des Verlaufs (kein Fensterwechsel weder zum Neuesten
// noch zum Anfang), kein Auf-/Zuklappen und keinen Nachrichtenwechsel (Rollfokus), und das Ereignis bleibt unverhindert
// (defaultPrevented false — das Browser-Verhalten wird nicht unterdrückt). Verlauf scrollbar (≥ 25 Füllnachrichten),
// Nachricht mit film.webm und ton.ogg; Ende nach begonnener Wiedergabe (davor verhindert es der Verlauf mit Absicht, PO 178,
// geprüft in r00030-player.spec.js). Das Browser-Scrollen von Bild ab und Pos1 bleibt erlaubt. Gegenprobe: Fokus auf der Nachricht selbst → die Kürzel wirken wie bisher.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { anzeigeStarten } = require('./r00030-anzeige-hilfen');
const { player, abspielen } = require('./r00030-player-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const TASTEN = ['Home', 'End', 'PageUp', 'PageDown'];

async function verlaufMitPlayern(browser) {
  const fuell = Array.from({ length: 30 }, (_, nummer) => ({ text: `Füllung ${nummer + 1} mit etwas mehr Text, damit der Verlauf wächst`, namen: [] }));
  const lauf = await anzeigeStarten(browser, [{ text: 'Medien', namen: ['film.webm', 'ton.ogg'] }, ...fuell]);
  instanz = lauf.instanz;
  return lauf;
}

/// Was die Kürzel des Verlaufs bewegen würden: die Nachrichten im Fenster (erste, letzte, Anzahl), der Rollfokus (die
/// Nachricht mit tabindex 0) und die geklappten Nachrichten.
const verlaufStand = (page) => page.evaluate(() => {
  const nachrichten = [...document.querySelectorAll('[role="log"] .lc-msg')];
  return {
    anzahl: nachrichten.length,
    erste: nachrichten[0].dataset.zeit,
    letzte: nachrichten[nachrichten.length - 1].dataset.zeit,
    rollfokus: nachrichten.filter((knoten) => knoten.getAttribute('tabindex') === '0').map((knoten) => knoten.dataset.zeit),
    geklappt: nachrichten.filter((knoten) => knoten.querySelector('[aria-expanded]') !== null).map((knoten) => knoten.querySelector('[aria-expanded]').getAttribute('aria-expanded')),
    pille: document.querySelector('.lc-pillen button') !== null && !document.querySelector('.lc-pillen').hidden,
  };
});

for (const [name, tag] of [['film.webm', 'Video'], ['ton.ogg', 'Audio']]) {
  for (const taste of TASTEN) {
    test(`Fokus im ${tag}: ${taste} löst kein Kürzel des Verlaufs aus (Fenster, Rollfokus, Klappen gleich), nichts verhindert, Fokus bleibt`, async ({ browser }) => {
      const lauf = await verlaufMitPlayern(browser);
      const { page } = lauf.a;
      const medium = player(nachrichtMitZeit(page, lauf.zeiten[0]), name);
      await medium.scrollIntoViewIfNeeded();
      if (taste === 'End') {
        // Ende wirkt erst, wenn die Wiedergabe begonnen hat (PO Zeile 178); davor bleibt es bewusst verhindert.
        await abspielen(medium);
        await medium.evaluate((element) => element.pause());
      }
      await medium.focus();
      await page.evaluate(() => {
        window.verhindert = [];
        window.addEventListener('keydown', (ereignis) => window.verhindert.push(`${ereignis.key}:${ereignis.defaultPrevented}`));
      });
      const vorher = await verlaufStand(page);

      await page.keyboard.press(taste);
      await page.waitForTimeout(400);

      expect(await verlaufStand(page)).toEqual(vorher);
      expect(await page.evaluate(() => window.verhindert)).toEqual([`${taste}:false`]);
      expect(await page.evaluate(() => document.activeElement.getAttribute('aria-label'))).toBe(name);
      expect(lauf.a.fehler).toEqual([]);
    });
  }
}

test('Gegenprobe: Ende auf der Nachricht selbst springt zum Neuesten wie bisher', async ({ browser }) => {
  const lauf = await verlaufMitPlayern(browser);
  const { page } = lauf.a;
  const log = page.getByRole('log');
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  await knoten.scrollIntoViewIfNeeded();
  await knoten.focus();
  const vorher = await log.evaluate((element) => element.scrollTop);

  await page.keyboard.press('End');

  await expect.poll(() => log.evaluate((element) => element.scrollTop)).toBeGreaterThan(vorher);
  expect(lauf.a.fehler).toEqual([]);
});
