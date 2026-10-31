'use strict';
// R00041 F0186 B0804 (Karte #85, Entwicklertester #80): das Bearbeiten-Feld der letzten eigenen Nachricht wächst bei fünf Zeilen
// nicht hinter die Eingabe — Unterkante Feld ≤ Oberkante Eingabe, der Verlauf führt nach, Feld und Knöpfe bleiben im Verlauf,
// die Schreibmarke (das Feld mit Fokus) bleibt sichtbar. Telefon 375 × 667 und 320 × 568 mit Touch, Desktop 1280 × 800, je mit 30
// Nachrichten davor. Mutante, die rot werden muss: hoeheAnpassen stellt die Rollstellung des Verlaufs nicht wieder her.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen, imBrowserSenden } = require('./r00041-hilfen');
const { feld, vermessen, imVerlauf, eigeneNachrichtBearbeiten } = require('./r00041-feld-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Fünf Zeilen per Umschalt+Enter, dann messen.
async function aufFuenfZeilen(page) {
  for (const zeile of ['zwei', 'drei', 'vier', 'fünf']) {
    await page.keyboard.press('Shift+Enter');
    await page.keyboard.type(zeile);
  }
  await expect(feld(page)).toHaveValue('Eins\nzwei\ndrei\nvier\nfünf');
  await page.waitForTimeout(300);
  return page.evaluate(vermessen);
}

for (const [breite, hoehe] of [[320, 568], [375, 667]]) {
  test(`Telefon ${breite} × ${hoehe} (B0804): bei fünf Zeilen Unterkante Feld ≤ Oberkante Eingabe, der Verlauf führt nach, das Feld behält den Fokus`, async ({ browser }) => {
    const gestartet = await eigeneNachrichtBearbeiten(browser, breite, hoehe, 'Eins');
    instanz = gestartet.instanz;
    const { a } = gestartet;
    const m = await aufFuenfZeilen(a.page);
    imVerlauf(m);
    expect(m.feldUnten, 'Unterkante Feld ≤ Oberkante Eingabe').toBeLessThanOrEqual(m.eingabeOben + 0.5);
    expect(m.feldHoehe).toBeGreaterThanOrEqual(5 * 24);
    expect(m.feldScrollt, 'bei fünf Zeilen ohne Platzmangel scrollt das Feld nicht').toBe(false);
    await expect(feld(a.page)).toBeFocused();
    expect(a.fehler).toEqual([]);
  });
}

test('Desktop 1280 × 800 (B0804): Feld der letzten Nachricht bei fünf Zeilen über der Eingabe', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 30 });
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Eins');
  await a.page.locator(`.lc-msg[data-zeit="${zeit}"]`).focus();
  await a.page.keyboard.press('e');
  await expect(feld(a.page)).toBeFocused();
  const m = await aufFuenfZeilen(a.page);
  imVerlauf(m);
  expect(m.feldUnten).toBeLessThanOrEqual(m.eingabeOben + 0.5);
  expect(a.fehler).toEqual([]);
});
