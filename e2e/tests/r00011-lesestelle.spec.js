'use strict';
// R00011 F0044, Nacharbeit #8: „Lesestelle nach dem Sprung“ (UX-Review S1, US-27, US-28) — nach dem Klick auf die
// Pille bleibt die erste neue Nachricht an ihrer Stelle, wenn der Blick weiterscrollt oder weitere Nachrichten kommen.
// Gegen das echte Programm, Uhr ab 2026-10-28 15:00 (+01:00); B ist `anna`, `alpha` und `beta` senden per API.
const { test, expect } = require('./rahmen');
const { fehlerSammeln } = require('./hilfen');
const { sendenStarten, beitreten, senden, imBrowserBeitreten, nachrichten, mitText } = require('./senden-hilfen');

/// Die Scrollweite eines Schritts mit `↓` bzw. des Mausrads in der Probe.
const SCROLLWEITE_PX = 40;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const pille = (page) => page.locator('.lc-pillen .lc-pill');

/// Wartet, bis die Oberkante der Nachricht mit `text` zehn Bilder lang gleich bleibt, und liefert sie (px im Fenster).
async function ruhigeOberkante(page, text) {
  const ergebnis = await page.waitForFunction((gesucht) => {
    const treffer = [...document.querySelectorAll('[role="log"] .lc-msg')]
      .find((nachricht) => nachricht.querySelector('.lc-body').textContent === gesucht);
    if (treffer === undefined) {
      return false;
    }
    const oben = treffer.getBoundingClientRect().top;
    const verlauf = window.lcRuhe;
    if (verlauf === undefined || verlauf.oben !== oben) {
      window.lcRuhe = { oben, bilder: 0 };
      return false;
    }
    verlauf.bilder += 1;
    if (verlauf.bilder < 10) {
      return false;
    }
    window.lcRuhe = undefined;
    return { oben };
  }, text, { polling: 'raf', timeout: 10000 });
  return (await ergebnis.jsonValue()).oben;
}

/// `allgemein` mit 300 Nachrichten; B steht 2 Bildschirmhöhen über dem Ende; `beta` sendet „neu 1“ bis „neu 3“; Klick
/// auf die Pille. Liefert Seite, Fehler, Tokens und die ruhige Oberkante von „neu 1“ nach dem Sprung.
async function nachDemSprung(browser, viewport) {
  instanz = await sendenStarten({ anzahl: 300 });
  const kontext = await browser.newContext({ viewport });
  const page = await kontext.newPage();
  const fehler = fehlerSammeln(page, { streng: true });
  await imBrowserBeitreten(page, instanz.adresse, 'anna');
  const log = page.getByRole('log');
  await expect(nachrichten(page)).not.toHaveCount(0);
  const sicht = await log.evaluate((element) => element.clientHeight);
  await log.evaluate((element) => { element.scrollTop = element.scrollHeight - element.clientHeight * 3; });
  await expect.poll(() => log.evaluate((element) => element.scrollHeight - element.scrollTop - element.clientHeight))
    .toBeGreaterThanOrEqual(2 * sicht - 1);
  const alpha = await beitreten(instanz.adresse, 'alpha');
  const beta = await beitreten(instanz.adresse, 'beta');
  for (const text of ['neu 1', 'neu 2', 'neu 3']) {
    await senden(instanz.adresse, beta, 'allgemein', text);
  }
  await expect(pille(page)).toHaveText(/^↓ 3 neue Nachrichten/);
  await pille(page).click();
  await expect(mitText(page, /^neu 1$/)).toBeInViewport();
  const oben = await ruhigeOberkante(page, 'neu 1');
  return { page, log, fehler, alpha, beta, oben };
}

const FENSTER = [{ width: 1280, height: 800 }, { width: 375, height: 812 }];

for (const viewport of FENSTER) {
  const groesse = `${viewport.width} × ${viewport.height}`;

  // US-27, Probe (a) mit `↓`: die erste neue rückt höchstens um die Scrollweite + 1 px nach oben, nie nach unten.
  test(`US-27 (${groesse}): nach dem Sprung und einmal ↓ bleibt die erste neue an ihrer Stelle`, async ({ browser }) => {
    const { page, fehler, oben } = await nachDemSprung(browser, viewport);

    await page.keyboard.press('ArrowDown');

    const verschoben = await ruhigeOberkante(page, 'neu 1') - oben;
    expect(verschoben).toBeLessThanOrEqual(1);
    expect(-verschoben).toBeLessThanOrEqual(SCROLLWEITE_PX + 1);
    expect(fehler).toEqual([]);
  });

  // US-27, Probe (a) mit dem Mausrad, 40 px.
  test(`US-27 (${groesse}): nach dem Sprung und 40 px Mausrad bleibt die erste neue an ihrer Stelle`, async ({ browser }) => {
    const { page, log, fehler, oben } = await nachDemSprung(browser, viewport);
    const mitte = await log.boundingBox();
    await page.mouse.move(mitte.x + mitte.width / 2, mitte.y + mitte.height / 2);

    await page.mouse.wheel(0, SCROLLWEITE_PX);

    const verschoben = await ruhigeOberkante(page, 'neu 1') - oben;
    expect(verschoben).toBeLessThanOrEqual(1);
    expect(-verschoben).toBeLessThanOrEqual(SCROLLWEITE_PX + 1);
    expect(fehler).toEqual([]);
  });

  // US-28, Proben (b) und (c): beta sendet eine 4., alpha danach 5 hintereinander → die erste neue bleibt (± 1 px), die
  // Pille zählt weiter.
  test(`US-28 (${groesse}): nach dem Sprung bleibt die erste neue, wenn weitere kommen`, async ({ browser }) => {
    const { page, fehler, alpha, beta, oben } = await nachDemSprung(browser, viewport);

    await senden(instanz.adresse, beta, 'allgemein', 'neu 4');
    await expect(mitText(page, /^neu 4$/)).toHaveCount(1);
    expect(Math.abs(await ruhigeOberkante(page, 'neu 1') - oben)).toBeLessThanOrEqual(1);
    await expect(pille(page)).toHaveText(/^↓ 4 neue Nachrichten/);
    for (let nummer = 5; nummer <= 9; nummer += 1) {
      await senden(instanz.adresse, alpha, 'allgemein', `neu ${nummer}`);
    }
    await expect(mitText(page, /^neu 9$/)).toHaveCount(1);

    expect(Math.abs(await ruhigeOberkante(page, 'neu 1') - oben)).toBeLessThanOrEqual(1);
    await expect(pille(page)).toHaveText(/^↓ 9 neue Nachrichten/);
    expect(fehler).toEqual([]);
  });
}

/// Höhe des Auslaufs, des leeren Raums unter dem Inhalt des Verlaufs (px).
function auslaufHoehe(log) {
  return log.evaluate((element) => element.querySelector('.lc-auslauf').getBoundingClientRect().height);
}

// B0329 Fertig-Kriterium: die 4. Nachricht nimmt den Auslauf ein (die Scrollfläche wächst nicht); gerät er ganz aus
// dem Fenster, fällt er weg.
test('Auslauf (1280 × 800): neue Nachrichten nehmen ihn ein, ganz aus dem Fenster gescrollt fällt er weg', async ({ browser }) => {
  const { page, log, fehler, beta } = await nachDemSprung(browser, FENSTER[0]);
  const auslaufVorher = await auslaufHoehe(log);
  const flaecheVorher = await log.evaluate((element) => element.scrollHeight);
  expect(auslaufVorher).toBeGreaterThan(0);

  await senden(instanz.adresse, beta, 'allgemein', 'neu 4');
  await expect(mitText(page, /^neu 4$/)).toHaveCount(1);
  await ruhigeOberkante(page, 'neu 1');

  const hoeheDerVierten = await mitText(page, /^neu 4$/).evaluate((nachricht) => nachricht.getBoundingClientRect().height);
  expect(Math.abs(await log.evaluate((element) => element.scrollHeight) - flaecheVorher)).toBeLessThanOrEqual(1);
  expect(Math.abs(await auslaufHoehe(log) - (auslaufVorher - hoeheDerVierten))).toBeLessThanOrEqual(1);

  await log.evaluate((element) => { element.scrollTop -= 2 * element.clientHeight; });
  await expect.poll(() => auslaufHoehe(log)).toBe(0);
  expect(fehler).toEqual([]);
});
