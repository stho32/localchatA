'use strict';
// R00020 F0067 Sprung zur ersten neuen Nachricht lesbar (US-1, US-2): gegen das echte Programm, Uhr ab 2026-10-28
// 15:00 (+01:00). B ist `anna`, `beta` sendet per API; `allgemein` hat 300 Nachrichten.
const { test, expect } = require('@playwright/test');
const { sendenStarten, beitreten, senden, kontextOeffnen, imBrowserBeitreten, nachrichten, mitText } = require('./senden-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const pille = (page) => page.locator('.lc-pillen .lc-pill');

function abstandZumEnde(log) {
  return log.evaluate((element) => element.scrollHeight - element.scrollTop - element.clientHeight);
}

/// B liest zwei Bildschirmhöhen über dem Ende.
async function zweiHoehenHoch(log) {
  const sicht = await log.evaluate((element) => element.clientHeight);
  await log.evaluate((element) => { element.scrollTop = element.scrollHeight - element.clientHeight * 3; });
  await expect.poll(() => abstandZumEnde(log)).toBeGreaterThanOrEqual(2 * sicht - 1);
}

/// B in `allgemein` mit 300 Nachrichten, Fenster `viewport`; liefert Seite, Verlauf, Fehler und das Token von `beta`.
async function bLiest(browser, viewport = { width: 1280, height: 800 }) {
  instanz = await sendenStarten({ anzahl: 300 });
  const b = await kontextOeffnen(browser, { breite: viewport.width, streng: true });
  await b.page.setViewportSize(viewport);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const log = b.page.getByRole('log');
  await expect(nachrichten(b.page)).not.toHaveCount(0);
  const beta = await beitreten(instanz.adresse, 'beta');
  return { page: b.page, log, fehler: b.fehler, beta };
}

async function dreiNeue(page, beta) {
  for (const text of ['neu 1', 'neu 2', 'neu 3']) {
    await senden(instanz.adresse, beta, 'allgemein', text);
  }
  await expect(pille(page)).toContainText('3 neue Nachrichten');
}

// US-1, „Erste neue mit Kopf“ (B0331): beta sendet 1, B scrollt 2 Höhen hoch, beta sendet 3 innerhalb einer Minute;
// nach dem Klick auf die Pille trägt die erste der drei Absender und Uhrzeit im Kopf, die zweite und dritte keinen.
test('US-1: die erste neue trägt nach dem Sprung ihren Kopf, die zweite und dritte nicht', async ({ browser }) => {
  const { page, log, fehler, beta } = await bLiest(browser);
  await senden(instanz.adresse, beta, 'allgemein', 'davor');
  await expect(mitText(page, /^davor$/)).toBeInViewport();
  await expect.poll(() => abstandZumEnde(log)).toBeLessThanOrEqual(1);
  await zweiHoehenHoch(log);
  await dreiNeue(page, beta);

  await pille(page).click();

  const erste = mitText(page, /^neu 1$/);
  await expect(erste).toBeInViewport();
  await expect(erste.locator('.lc-meta .lc-id')).toHaveText('beta@127.0.0.1');
  await expect(erste.locator('.lc-meta .lc-time')).toHaveText(/^\d\d:\d\d$/);
  await expect(mitText(page, /^davor$/).locator('.lc-meta .lc-id')).toHaveText('beta@127.0.0.1');
  await expect(mitText(page, /^neu 2$/).locator('.lc-meta')).toHaveCount(0);
  await expect(mitText(page, /^neu 3$/).locator('.lc-meta')).toHaveCount(0);
  expect(fehler).toEqual([]);
});

/// Wartet, bis die Oberkante der Nachricht `text` zehn Bilder lang gleich bleibt, und misst dann im Fenster (px):
/// `oberkante` — obere Kante ihres Platzes, mit Außenabstand (eine Nachricht mit Kopf hat Abstand nach oben);
/// `trennerUnten` — Unterkante des klebenden Tagestrenners (des zuletzt gezeichneten, der am oberen Rand klebt), ohne
/// ihn der obere Rand des Verlaufs; `zeile` — Zeilenhöhe des Verlaufs; `getroffen` — was `elementFromPoint` eine
/// halbe Zeile über der Oberkante trifft.
async function lageNachDemSprung(page, text) {
  const ergebnis = await page.waitForFunction((gesucht) => {
    const log = document.querySelector('[role="log"]');
    const erste = [...log.querySelectorAll('.lc-msg')].find((nachricht) => nachricht.querySelector('.lc-body').textContent === gesucht);
    if (erste === undefined) {
      return false;
    }
    const rechteck = erste.getBoundingClientRect();
    const oberkante = rechteck.top - parseFloat(getComputedStyle(erste).marginTop);
    const ruhe = window.lcRuhe;
    if (ruhe === undefined || ruhe.oberkante !== oberkante) {
      window.lcRuhe = { oberkante, bilder: 0 };
      return false;
    }
    ruhe.bilder += 1;
    if (ruhe.bilder < 10) {
      return false;
    }
    window.lcRuhe = undefined;
    const rand = log.getBoundingClientRect().top;
    const klebend = [...log.querySelectorAll('.lc-day')]
      .filter((trenner) => Math.abs(trenner.getBoundingClientRect().top - rand - parseFloat(getComputedStyle(trenner).top)) <= 1);
    let trennerUnten = rand;
    if (klebend.length > 0) {
      trennerUnten = klebend[klebend.length - 1].getBoundingClientRect().bottom;
    }
    const zeile = parseFloat(getComputedStyle(log).lineHeight);
    const punkt = document.elementFromPoint(rechteck.left + rechteck.width / 2, oberkante - zeile / 2);
    const getroffen = { nachricht: punkt !== null && punkt.closest('.lc-msg') !== null, trenner: punkt !== null && punkt.closest('.lc-day') !== null };
    return { oberkante, trennerUnten, zeile, getroffen, klebend: klebend.length };
  }, text, { polling: 'raf', timeout: 10000 });
  return ergebnis.jsonValue();
}

/// Abstand vom Ende des Inhalts (Oberkante des Auslaufs) zur Unterkante des Verlaufs (px).
function abstandZumEndeDesInhalts(log) {
  return log.evaluate((element) => element.querySelector('.lc-auslauf').getBoundingClientRect().top - element.getBoundingClientRect().bottom);
}

const FENSTER = [{ width: 1280, height: 800 }, { width: 375, height: 812 }];

for (const viewport of FENSTER) {
  const groesse = `${viewport.width} × ${viewport.height}`;

  // „Kontext unter dem Trenner“ (B0330): nach dem Klick liegt die Oberkante der ersten neuen eine Zeile (± 1 px) unter
  // der Unterkante des klebenden Tagestrenners; eine halbe Zeile darüber steht eine Nachricht, nicht der Trenner.
  test(`US-1 (${groesse}): nach dem Sprung steht eine Zeile Kontext unter dem klebenden Tagestrenner`, async ({ browser }) => {
    const { page, log, fehler, beta } = await bLiest(browser, viewport);
    await zweiHoehenHoch(log);
    await dreiNeue(page, beta);

    await pille(page).click();

    const lage = await lageNachDemSprung(page, 'neu 1');
    expect(lage.klebend).toBeGreaterThan(0);
    expect(lage.oberkante - lage.trennerUnten).toBeGreaterThanOrEqual(lage.zeile - 1);
    expect(lage.oberkante - lage.trennerUnten).toBeLessThanOrEqual(lage.zeile + 1);
    expect(lage.getroffen).toEqual({ nachricht: true, trenner: false });
    expect(fehler).toEqual([]);
  });
}

// US-2, „Zweiter Klick ans Ende“ (B0332): nach dem Sprung bringt ein zweiter Klick auf die Pille bzw. `Ende` im Verlauf
// den Blick ans Ende des Inhalts; die Pille ist verborgen, alle drei neuen sind sichtbar.
for (const zweiterZug of ['ein zweiter Klick auf die Pille', 'Ende im Verlauf']) {
  test(`US-2: nach dem Sprung führt ${zweiterZug} ans Ende`, async ({ browser }) => {
    const { page, log, fehler, beta } = await bLiest(browser);
    await zweiHoehenHoch(log);
    await dreiNeue(page, beta);
    await pille(page).click();
    await lageNachDemSprung(page, 'neu 1');

    if (zweiterZug === 'ein zweiter Klick auf die Pille') {
      await pille(page).click();
    } else {
      await log.press('End');
    }

    await expect.poll(() => abstandZumEndeDesInhalts(log)).toBeGreaterThanOrEqual(-1);
    expect(await abstandZumEndeDesInhalts(log)).toBeLessThanOrEqual(1);
    await expect(pille(page)).toBeHidden();
    for (const text of [/^neu 1$/, /^neu 2$/, /^neu 3$/]) {
      await expect(mitText(page, text)).toBeInViewport();
    }
    expect(fehler).toEqual([]);
  });
}

// R00020 F0068 „Beschriftung“ (US-3, B0333): sichtbar „↓ n neue Nachrichten“ bzw. „↓ Zum Neuesten“, mit Leerraum vor
// der Taste `Ende`; der zugängliche Name ist genau „n neue Nachrichten“ bzw. „Zum Neuesten“, die Taste nennt
// aria-keyshortcuts.
test('US-3: die Pille heißt „3 neue Nachrichten“ und zeigt „↓ … Ende“', async ({ browser }) => {
  const { page, log, fehler, beta } = await bLiest(browser);
  await zweiHoehenHoch(log);
  await dreiNeue(page, beta);

  const benannt = page.getByRole('button', { name: '3 neue Nachrichten', exact: true });
  await expect(benannt).toBeVisible();
  expect(await benannt.evaluate((schalter) => schalter.innerText)).toMatch(/^↓ 3 neue Nachrichten\s+Ende$/);
  await expect(benannt).toHaveAttribute('aria-keyshortcuts', 'End');
  expect(fehler).toEqual([]);
});

test('US-3: ohne neue heißt die Pille „Zum Neuesten“, nach einer neuen „1 neue Nachricht“', async ({ browser }) => {
  const { page, log, fehler, beta } = await bLiest(browser);
  await zweiHoehenHoch(log);

  const zumNeuesten = page.getByRole('button', { name: 'Zum Neuesten', exact: true });
  await expect(zumNeuesten).toBeVisible();
  expect(await zumNeuesten.evaluate((schalter) => schalter.innerText)).toMatch(/^↓ Zum Neuesten\s+Ende$/);
  await expect(zumNeuesten).toHaveAttribute('aria-keyshortcuts', 'End');

  await senden(instanz.adresse, beta, 'allgemein', 'neu 1');

  const eine = page.getByRole('button', { name: '1 neue Nachricht', exact: true });
  await expect(eine).toBeVisible();
  expect(await eine.evaluate((schalter) => schalter.innerText)).toMatch(/^↓ 1 neue Nachricht\s+Ende$/);
  expect(fehler).toEqual([]);
});
