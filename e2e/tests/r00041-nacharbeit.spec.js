'use strict';
// R00041 Nacharbeit #68 (code-reviewer B1, B2, A2, A3; ux-reviewer S3): das Nachholen frischt das ganze Fenster auf
// (F0134), das offene Feld steht bei der Rückkehr in den Channel wieder im Verlauf (F0142), die Listener der Ansicht
// hängen an ihrem Signal, Löschen sendet höchstens einen Aufruf zugleich, eigenes Löschen sagt nicht „inzwischen“.
const { test, expect } = require('./rahmen');
const { netzAus, netzAn, leiste } = require('./r00012-hilfen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const {
  betaVerlauf, betaSendet, bearbeitenPerApi, loeschenPerApi, browserOeffnen, imBrowserSenden, nachrichtMitZeit,
} = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const feld = (page) => page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' });
const ohneNetzfehler = (fehler) => fehler.filter((zeile) => !/ERR_INTERNET_DISCONNECTED/.test(zeile));

async function feldOeffnen(page, zeit) {
  await nachrichtMitZeit(page, zeit).hover();
  await nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).click();
  await expect(feld(page)).toBeFocused();
}

/// Scrollt den Verlauf an den Anfang, bis die Nachricht `zeit` im Verlauf steht.
async function nachObenBis(page, zeit) {
  const log = page.getByRole('log');
  for (let schritt = 0; schritt < 80 && await nachrichtMitZeit(page, zeit).count() === 0; schritt += 1) {
    await log.evaluate((verlauf) => { verlauf.scrollTop = 0; });
    await page.waitForTimeout(100);
  }
  await expect(nachrichtMitZeit(page, zeit)).toHaveCount(1);
}

// B1 (F0134, ADR-0048 Regel 11): das offene Feld an der neuesten Nachricht schützt das untere Ende, das Fenster wächst
// beim Scrollen an den Anfang auf acht Blöcke (751 Nachrichten). Offline bearbeitet beta die älteste, löscht die
// zweitälteste und bearbeitet eine aus der Mitte; nach „Wieder verbunden“ zeigt A alle drei Änderungen, ohne Neuladen,
// und das Feld steht mit dem getippten Text.
test('B1 Nachholen frischt das ganze Fenster auf: acht Blöcke, älteste bearbeitet, zweitälteste gelöscht, Mitte bearbeitet', async ({ browser }) => {
  test.setTimeout(90000);
  const verlauf = betaVerlauf(750);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const beta = (await betaSendet(instanz.adresse, [])).token;
  const a = await browserOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const meine = await imBrowserSenden(a.page, 'Meine');
  await feldOeffnen(a.page, meine);
  await a.page.keyboard.type(' getippt');
  const [aelteste, zweite, mitte] = [verlauf.zeiten[0], verlauf.zeiten[1], verlauf.zeiten[150]];
  await nachObenBis(a.page, aelteste);
  await expect(nachrichtMitZeit(a.page, meine)).toHaveCount(1);
  await a.page.evaluate(() => { window.nichtNeuGeladen = true; });

  await netzAus(a);
  expect((await bearbeitenPerApi(instanz.adresse, beta, aelteste, 'Älteste neu')).status).toBe(200);
  expect((await loeschenPerApi(instanz.adresse, beta, zweite)).status).toBe(200);
  expect((await bearbeitenPerApi(instanz.adresse, beta, mitte, 'Mitte neu')).status).toBe(200);
  await netzAn(a);
  await expect(leiste(a.page)).toHaveText(/Wieder verbunden/);

  await expect(nachrichtMitZeit(a.page, aelteste).locator('.lc-body')).toHaveText('Älteste neu');
  await expect(nachrichtMitZeit(a.page, aelteste).locator('.lc-marke', { hasText: 'bearbeitet' })).toHaveCount(1);
  await expect(nachrichtMitZeit(a.page, zweite).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await expect(nachrichtMitZeit(a.page, mitte).locator('.lc-body')).toHaveText('Mitte neu');
  await expect(feld(a.page)).toHaveValue('Meine getippt');
  expect(await a.page.evaluate(() => window.nichtNeuGeladen)).toBe(true);
  expect(ohneNetzfehler(a.fehler)).toEqual([]);
});
