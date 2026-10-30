'use strict';
// R00045 F0161 (Karte #83 A2, Code-Review #77): eine Nachricht beendet die Anzeige ihres Absenders im Client
// (ADR-0051 Regel 7 c, `tippanzeige.nachricht(…)` in web/hauptansicht.js). Die Ende-Ereignisse der Aktivität werden am
// Strom von ben unterdrückt; ohne die Verdrahtung bliebe die Zeile bis zum Ablauf stehen.
const { test, expect } = require('./rahmen');
const { mitText, eingabe } = require('./senden-hilfen');
const { sichtbarNach } = require('./hilfen');
const { ANNA_TIPPT, zeigtTipp, endeEreignisseUnterdruecken, zweiBrowserStarten } = require('./r00045-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const tippzeile = (page) => page.locator('.lc-comp .lc-comp-tipp');

test('Nachricht beendet die Anzeige im Client (A2, Regel 7 c): ohne Ende-Ereignisse am Strom ist die Zeile bei ben binnen 1 s leer, ein Eintrag in # technik bleibt', async ({ browser }) => {
  test.setTimeout(45000);
  const paar = await zweiBrowserStarten(browser, { benOhneWorker: true });
  instanz = paar.instanz;
  const { anna, ben } = paar;
  await endeEreignisseUnterdruecken(ben.kontext);
  await ben.page.reload();
  await expect(ben.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');

  let ab = Date.now();
  await anna.page.keyboard.type('a');
  expect(await sichtbarNach(ben.page, zeigtTipp, { argument: ANNA_TIPPT, ab })).toBeLessThanOrEqual(1000);
  await anna.page.evaluate(() => { window.location.hash = '#/c/technik'; });
  await anna.page.keyboard.type('b');
  await anna.page.evaluate(() => { window.location.hash = '#/c/allgemein'; });
  await anna.page.keyboard.type('c');
  await expect(tippzeile(ben.page)).toHaveText(ANNA_TIPPT);

  const text = await eingabe(anna.page).inputValue();
  ab = Date.now();
  await anna.page.keyboard.press('Enter');
  await expect(mitText(ben.page, new RegExp(`^${text}$`))).toHaveCount(1);
  expect(await sichtbarNach(ben.page, zeigtTipp, { argument: '', ab, timeout: 1000 }), 'Zeile leer nach der Nachricht').toBeLessThanOrEqual(1000);

  await ben.page.evaluate(() => { window.location.hash = '#/c/technik'; });
  await expect(ben.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(tippzeile(ben.page)).toHaveText(ANNA_TIPPT);
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});
