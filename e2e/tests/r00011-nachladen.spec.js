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

const pille = (page) => page.locator('.lc-pillen .lc-pill');

/// Hält die Antwort auf einen Abruf `nach=…` zurück — `neuesterBlock`: den, der den neuesten Block bringt (Antwort mit
/// `neuer: false`), sonst den ersten, nach dem noch einer kommt; die Antwort wird vor dem Senden beim Server geholt und
/// erst bei `freigeben()` ausgeliefert. Alle anderen Abrufe laufen durch.
async function nachladeAbrufZurueckhalten(page, channel, { neuesterBlock }) {
  let melden;
  const angekommen = new Promise((fertig) => { melden = fertig; });
  let freigabe;
  const freigegeben = new Promise((fertig) => { freigabe = fertig; });
  let gehalten = false;
  await page.route(`**/api/channels/${channel}/nachrichten?nach=*`, async (route) => {
    const antwort = await route.fetch();
    const block = await antwort.json();
    if (block.neuer === neuesterBlock || gehalten) {
      await route.fulfill({ response: antwort });
      return;
    }
    gehalten = true;
    melden();
    await freigegeben;
    await route.fulfill({ response: antwort });
  });
  return { angekommen, istGehalten: () => gehalten, freigeben: () => freigabe() };
}

/// Scrollt nach oben, bis die Nachricht mit `zeit` (aus dem neuesten Block) nicht mehr im DOM steht.
async function neuestenBlockAbwerfen(log, zeit) {
  await expect(async () => {
    await log.evaluate((element) => { element.scrollTop = 0; });
    await expect(log.locator(`.lc-msg[data-zeit="${zeit}"]`)).toHaveCount(0, { timeout: 200 });
  }).toPass({ timeout: 20000 });
}

/// Scrollt an den Anfang des Channels: oben, nichts lädt, und die erste Nachricht bleibt über zwei Blicke dieselbe.
async function ganzNachOben(log) {
  let vorher = null;
  await expect(async () => {
    await log.evaluate((element) => { element.scrollTop = 0; });
    await log.page().waitForTimeout(300);
    const jetzt = await log.evaluate((element) => ({
      ruhig: element.scrollTop === 0 && element.dataset.laedt === undefined,
      erste: element.querySelector('.lc-msg').dataset.zeit,
    }));
    const stabil = vorher !== null && jetzt.ruhig && jetzt.erste === vorher;
    vorher = jetzt.erste;
    expect(stabil).toBe(true);
  }).toPass({ intervals: [0], timeout: 20000 });
}

/// Scrollt nach unten, bis `fertig()` gilt (nachgeladen wird beim Scrollen von selbst).
async function nachUntenBis(log, fertig) {
  await expect(async () => {
    await log.evaluate((element) => { element.scrollTop = element.scrollHeight; });
    expect(await fertig()).toBe(true);
  }).toPass({ intervals: [100], timeout: 20000 });
}

function amEndeDesInhalts(log) {
  return log.evaluate((element) => element.dataset.laedt === undefined
    && element.scrollHeight - element.scrollTop - element.clientHeight <= 1);
}

/// 1.000 Nachrichten in `allgemein`; B oben, der neueste Block abgeworfen; Tokens für alpha und beta.
async function obenMitAbgeworfenemNeuestenBlock(browser) {
  instanz = await sendenStarten({ anzahl: 1000 });
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const log = b.page.getByRole('log');
  await expect(nachrichten(b.page)).not.toHaveCount(0);
  const letzteZeit = await nachrichten(b.page).last().getAttribute('data-zeit');
  await neuestenBlockAbwerfen(log, letzteZeit);
  const alpha = await beitreten(instanz.adresse, 'alpha');
  const beta = await beitreten(instanz.adresse, 'beta');
  return { b, log, alpha, beta, letzteZeit };
}

/// Am Ende stehen direkt hinter der vorher neuesten Nachricht (`letzteZeit`) genau die `texte` — nichts fehlt
/// dazwischen, nichts steht doppelt.
async function hinterDerNeuestenStehen(page, letzteZeit, texte) {
  const ende = await nachrichten(page).evaluateAll((liste, anzahl) => liste.slice(-anzahl).map((nachricht) => ({
    zeit: nachricht.dataset.zeit, text: nachricht.querySelector('.lc-body').textContent,
  })), texte.length + 1);
  expect(ende[0].zeit).toBe(letzteZeit);
  expect(ende.slice(1).map((nachricht) => nachricht.text)).toEqual(texte);
}

// Abnahme-Probe B1 wörtlich: „verzögerte Antwort des letzten Abrufs via route.fetch/fulfill: Nachricht während des
// Abrufs → nach Scrollen ans Ende genau 1× im DOM“; die Pille zählt sie weiter, bis der Blick das Ende erreicht.
test('B1 Nachricht während des letzten Nachlade-Abrufs: nach Scrollen ans Ende genau 1×', async ({ browser }) => {
  const { b, log, beta, letzteZeit } = await obenMitAbgeworfenemNeuestenBlock(browser);
  const gehalten = await nachladeAbrufZurueckhalten(b.page, 'allgemein', { neuesterBlock: true });
  await nachUntenBis(log, async () => gehalten.istGehalten());

  await senden(instanz.adresse, beta, 'allgemein', 'während 1');
  await expect(pille(b.page)).toHaveText(/^1 neue Nachricht/);
  gehalten.freigeben();
  await expect(log).not.toHaveAttribute('data-laedt');
  await expect(pille(b.page)).toHaveText(/^1 neue Nachricht/);
  await nachUntenBis(log, () => amEndeDesInhalts(log));

  await expect(mitText(b.page, /^während 1$/)).toHaveCount(1);
  await hinterDerNeuestenStehen(b.page, letzteZeit, ['während 1']);
  expect(b.fehler).toEqual([]);
});

// Abnahme-Probe B1 wörtlich: „zwei im Flug → beide in Zeitfolge je 1×“ — von zwei Absendern.
test('B1 zwei Nachrichten während des letzten Nachlade-Abrufs: beide in Zeitfolge je 1×', async ({ browser }) => {
  const { b, log, alpha, beta, letzteZeit } = await obenMitAbgeworfenemNeuestenBlock(browser);
  const gehalten = await nachladeAbrufZurueckhalten(b.page, 'allgemein', { neuesterBlock: true });
  await nachUntenBis(log, async () => gehalten.istGehalten());

  await senden(instanz.adresse, beta, 'allgemein', 'während 1');
  await senden(instanz.adresse, alpha, 'allgemein', 'während 2');
  await expect(pille(b.page)).toHaveText(/^2 neue Nachrichten/);
  gehalten.freigeben();
  await expect(log).not.toHaveAttribute('data-laedt');
  await nachUntenBis(log, () => amEndeDesInhalts(log));

  await expect(mitText(b.page, /^während [12]$/)).toHaveCount(2);
  const letzte = await nachrichten(b.page).evaluateAll((liste) => liste.slice(-2).map((nachricht) => ({
    text: nachricht.querySelector('.lc-body').textContent, zeit: nachricht.dataset.zeit, von: nachricht.dataset.von,
  })));
  expect(letzte.map((nachricht) => [nachricht.text, nachricht.von])).toEqual([
    ['während 1', 'beta@127.0.0.1'], ['während 2', 'alpha@127.0.0.1'],
  ]);
  expect(Date.parse(letzte[0].zeit)).toBeLessThan(Date.parse(letzte[1].zeit));
  await hinterDerNeuestenStehen(b.page, letzteZeit, ['während 1', 'während 2']);
  expect(b.fehler).toEqual([]);
});

// Abnahme-Probe B1 wörtlich: „vor dem Abruf gesendet → genau 1×“ — die Nachricht kommt mit dem Block, nicht doppelt.
test('B1 Nachricht vor dem letzten Nachlade-Abruf gesendet: genau 1×', async ({ browser }) => {
  const { b, log, beta, letzteZeit } = await obenMitAbgeworfenemNeuestenBlock(browser);

  await senden(instanz.adresse, beta, 'allgemein', 'vorher 1');
  await expect(pille(b.page)).toHaveText(/^1 neue Nachricht/);
  await nachUntenBis(log, () => amEndeDesInhalts(log));

  await expect(mitText(b.page, /^vorher 1$/)).toHaveCount(1);
  await hinterDerNeuestenStehen(b.page, letzteZeit, ['vorher 1']);
  expect(b.fehler).toEqual([]);
});

// Klasse zu B1 (Zeit): eine Nachricht während eines früheren Nachlade-Abrufs, nach dem noch Blöcke kommen, steht am
// Ende genau einmal und als letzte — sie wird erst hinter dem neuesten Block angefügt, nicht dazwischen.
test('B1 Nachricht während eines früheren Nachlade-Abrufs: am Ende genau 1×, direkt hinter der vorher neuesten', async ({ browser }) => {
  const { b, log, beta, letzteZeit } = await obenMitAbgeworfenemNeuestenBlock(browser);
  await ganzNachOben(log);
  const gehalten = await nachladeAbrufZurueckhalten(b.page, 'allgemein', { neuesterBlock: false });
  await nachUntenBis(log, async () => gehalten.istGehalten());

  await senden(instanz.adresse, beta, 'allgemein', 'zwischendurch 1');
  await expect(pille(b.page)).toHaveText(/^1 neue Nachricht/);
  gehalten.freigeben();
  await nachUntenBis(log, () => amEndeDesInhalts(log));

  await expect(mitText(b.page, /^zwischendurch 1$/)).toHaveCount(1);
  await hinterDerNeuestenStehen(b.page, letzteZeit, ['zwischendurch 1']);
  const zeiten = await nachrichten(b.page).evaluateAll((liste) => liste.map((nachricht) => Date.parse(nachricht.dataset.zeit)));
  expect(zeiten.every((zeit, index) => index === 0 || zeiten[index - 1] < zeit)).toBe(true);
  expect(b.fehler).toEqual([]);
});
