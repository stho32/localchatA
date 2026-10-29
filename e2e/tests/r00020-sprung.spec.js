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
