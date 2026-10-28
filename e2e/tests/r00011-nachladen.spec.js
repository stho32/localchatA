'use strict';
// R00011 F0044, Nacharbeit #8: fremde Nachrichten im Wettlauf mit dem Laden des Verlaufs — „Puffer vor dem ersten
// Block“ (Code-Review B2). Gegen das echte Programm, Chromium 1280 × 800, Uhr ab 2026-10-28 15:00 (+01:00); B ist
// `anna`, `beta` sendet per API von 127.0.0.1. Die Antwort des Abrufs hält der Test über `page.route` zurück.
const { test, expect } = require('@playwright/test');
const {
  sendenStarten, beitreten, senden, kontextOeffnen, imBrowserBeitreten, nachrichten, mitText,
} = require('./senden-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Hält die erste Antwort auf `GET …/channels/<channel>/nachrichten` zurück. `angekommen` erfüllt sich, sobald der
/// Abruf beim Test ist; `freigeben()` liefert die Antwort aus. `zuerstHolen`: die Antwort sofort beim Server holen
/// (Stand vor dem Senden), sonst erst bei der Freigabe (Stand danach).
async function ersteAntwortZurueckhalten(page, channel, { zuerstHolen }) {
  let melden;
  const angekommen = new Promise((fertig) => { melden = fertig; });
  let freigabe;
  const freigegeben = new Promise((fertig) => { freigabe = fertig; });
  let schonGehalten = false;
  await page.route(`**/api/channels/${channel}/nachrichten*`, async (route) => {
    if (schonGehalten || route.request().method() !== 'GET') {
      await route.continue();
      return;
    }
    schonGehalten = true;
    let antwort = null;
    if (zuerstHolen) {
      antwort = await route.fetch();
    }
    melden();
    await freigegeben;
    if (antwort === null) {
      antwort = await route.fetch();
    }
    await route.fulfill({ response: antwort });
  });
  return { angekommen, freigeben: () => freigabe() };
}

function channelWaehlen(page, name) {
  return page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true }).click();
}

// Abnahme-Probe B2 wörtlich: „B wählt # technik, erste GET-Antwort zurückgehalten, beta sendet „PROBE-P“, 500 ms,
// fulfill → genau 1×, keine Konsolenfehler; Variante schon im Block → nicht doppelt.“ — je im leeren `technik` und in
// `technik` mit drei Nachrichten (der Puffer geht dann den Weg über den ersten Block bzw. hinter ihn).
for (const { vorher, titel } of [{ vorher: 0, titel: 'leer' }, { vorher: 3, titel: 'mit drei Nachrichten' }]) {
  test(`B2 Puffer vor dem ersten Block (technik ${titel}): Nachricht während des ersten Abrufs genau 1×`, async ({ browser }) => {
    instanz = await sendenStarten();
    const beta = await beitreten(instanz.adresse, 'beta');
    for (let nummer = 1; nummer <= vorher; nummer += 1) {
      await senden(instanz.adresse, beta, 'technik', `vorher ${nummer}`);
    }
    const b = await kontextOeffnen(browser, { streng: true });
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    const gehalten = await ersteAntwortZurueckhalten(b.page, 'technik', { zuerstHolen: true });
    await channelWaehlen(b.page, 'technik');
    await gehalten.angekommen;

    await senden(instanz.adresse, beta, 'technik', 'PROBE-P');
    await b.page.waitForTimeout(500);
    gehalten.freigeben();

    await expect(nachrichten(b.page)).toHaveCount(vorher + 1);
    await expect(mitText(b.page, /^PROBE-P$/)).toHaveCount(1);
    await expect(nachrichten(b.page).last().locator('.lc-body')).toHaveText('PROBE-P');
    expect(b.fehler).toEqual([]);
  });

  test(`B2 Puffer vor dem ersten Block (technik ${titel}): schon im Block enthalten → nicht doppelt`, async ({ browser }) => {
    instanz = await sendenStarten();
    const beta = await beitreten(instanz.adresse, 'beta');
    for (let nummer = 1; nummer <= vorher; nummer += 1) {
      await senden(instanz.adresse, beta, 'technik', `vorher ${nummer}`);
    }
    const b = await kontextOeffnen(browser, { streng: true });
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    const gehalten = await ersteAntwortZurueckhalten(b.page, 'technik', { zuerstHolen: false });
    await channelWaehlen(b.page, 'technik');
    await gehalten.angekommen;

    await senden(instanz.adresse, beta, 'technik', 'PROBE-P');
    await b.page.waitForTimeout(500);
    gehalten.freigeben();

    await expect(nachrichten(b.page)).toHaveCount(vorher + 1);
    await b.page.waitForTimeout(500);
    await expect(mitText(b.page, /^PROBE-P$/)).toHaveCount(1);
    await expect(nachrichten(b.page)).toHaveCount(vorher + 1);
    expect(b.fehler).toEqual([]);
  });
}
