'use strict';
// R00047 I0044 Nacharbeit Karte #79 (Browserteil): Abnahme-Proben aus ux-reviewer und code-reviewer.
const { test, expect } = require('./rahmen');
const { sendenStarten, eingabe } = require('./senden-hilfen');
const { nachrichtMitZeit, ansagenMitschreiben } = require('./r00041-hilfen');
const { annaSchreibt, benOeffnen, antwortzeile, ueberHoverAntworten } = require('./r00047-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ANNA_KENNUNG = 'anna@127.0.0.2';

/// Schneiden sich zwei Rechtecke (getBoundingClientRect) mit Fläche?
const schneiden = (a, b) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

for (const fenster of [{ width: 375, height: 667 }, { width: 320, height: 568 }, { width: 1280, height: 800 }]) {
  test(`UX S1 bei ${fenster.width} × ${fenster.height}: „Die Antwort an … wurde beim Wechsel verworfen.“ steht in der Hinweiszeile unter dem Feld, verdeckt weder Feld noch „Senden“ und wird einmal angesagt`, async ({ browser }) => {
    instanz = await sendenStarten();
    const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
    const b = await benOeffnen(browser, instanz.adresse, { geraet: { viewport: fenster }, streng: true });
    await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
    await nachrichtMitZeit(b.page, anna.zeiten[0]).focus();
    await b.page.keyboard.press('r');
    await expect(antwortzeile(b.page)).toHaveCount(1);
    const angesagt = await ansagenMitschreiben(b.page);
    const verworfen = `Die Antwort an ${ANNA_KENNUNG} wurde beim Wechsel verworfen.`;

    await b.page.evaluate(() => {
      window.location.hash = '#/c/technik';
    });
    await expect(b.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');

    const lage = await b.page.evaluate((text) => {
      const knopf = document.querySelector('.lc-comp-senden');
      const knopfRand = knopf.getBoundingClientRect();
      const mitte = document.elementFromPoint(knopfRand.left + knopfRand.width / 2, knopfRand.top + knopfRand.height / 2);
      const traeger = [...document.querySelectorAll('body *')]
        .filter((element) => element.textContent === text && element.children.length === 0 && element.closest('[aria-live]') === null)
        .filter((element) => element.closest('[hidden]') === null);
      return {
        mitteIstSenden: mitte === knopf,
        meldung: traeger.map((element) => element.getBoundingClientRect().toJSON()),
        feld: document.querySelector('.lc-comp-feld').getBoundingClientRect().toJSON(),
        senden: knopfRand.toJSON(),
      };
    }, verworfen);
    expect(lage.mitteIstSenden, JSON.stringify(lage)).toBe(true);
    await expect(b.page.locator('#lc-comp-hinweis')).toHaveText(verworfen);
    await expect(b.page.locator('#lc-comp-hinweis')).toBeInViewport({ ratio: 1 });
    expect(lage.meldung.length, JSON.stringify(lage)).toBe(1);
    expect(schneiden(lage.meldung[0], lage.feld), JSON.stringify(lage)).toBe(false);
    expect(schneiden(lage.meldung[0], lage.senden), JSON.stringify(lage)).toBe(false);
    await b.page.waitForTimeout(500);
    expect((await angesagt()).filter((text) => text.includes(verworfen)).length, JSON.stringify(await angesagt())).toBe(1);
    expect((await angesagt()).join(' ').split(verworfen).length - 1).toBe(1);
    await expect(eingabe(b.page)).toHaveValue('');
    expect(b.fehler).toEqual([]);
  });
}

test('UX S1: die Meldung „… wurde beim Wechsel verworfen.“ geht nach 6 s von selbst und weicht sofort der nächsten Eingabe', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  const hinweisZeile = b.page.locator('#lc-comp-hinweis');
  const verworfen = `Die Antwort an ${ANNA_KENNUNG} wurde beim Wechsel verworfen.`;

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await b.page.getByRole('link', { name: 'technik' }).click();
  await expect(hinweisZeile).toHaveText(verworfen);
  const gezeigtAb = Date.now();
  await expect(hinweisZeile).toHaveText('', { timeout: 10000 });
  expect(Date.now() - gezeigtAb).toBeGreaterThan(5000);

  await b.page.getByRole('link', { name: 'allgemein' }).click();
  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await b.page.getByRole('link', { name: 'technik' }).click();
  await expect(hinweisZeile).toHaveText(verworfen);
  await eingabe(b.page).press('x');
  await expect(hinweisZeile).toHaveText('');
  await expect(hinweisZeile).not.toHaveClass(/lc-fehler/);
  expect(b.fehler).toEqual([]);
});
