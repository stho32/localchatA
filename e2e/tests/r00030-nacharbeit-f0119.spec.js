'use strict';
// R00030 F0119 Nacharbeit nach dem Code-Review #43 F0119 (B1, B2, Sollte 2, 3, 5): Hochladen nach dem Wiederverbinden und
// bei Selbstblockade — gegen das echte Programm, Desktop 1280 × 800. Die PUT-Antworten werden über page.route
// vorgeschrieben, die Verbindung über context.setOffline unterbrochen.
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, kontextOeffnen, eingabe } = require('./senden-hilfen');
const { knopf } = require('./r00027-hilfen');
const { saetzeMit } = require('./r00012-hilfen');
const { anhangStarten, anfragenMitschreiben, waehlen, perApi, betaBeitreten, perApiAblegen } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const MIB = 1024 * 1024;
const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const datei = (page, name) => eintrag(page).locator('.lc-hochladung-datei', { hasText: name });
const leiste = (page) => page.locator('.lc-leiste');
const bestaetigt = (page, text) => page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)').filter({ has: page.locator('.lc-body', { hasText: text }) });
const unerwartet = (fehler) => fehler.filter((zeile) => !/ERR_INTERNET_DISCONNECTED|ERR_FAILED|409|404|507/.test(zeile));
const kleineDatei = (name, bytes = 64 * 1024) => ({ name, mimeType: 'application/octet-stream', buffer: Buffer.alloc(bytes, 7) });

/// Ein Browser als `shoff`; `warnungGesehen()` sagt, ob je ein ⚠ an einer Datei des Eintrags stand.
async function seiteAuf(browser, argumente) {
  instanz = await anhangStarten(argumente === undefined ? {} : { argumente });
  const a = await kontextOeffnen(browser);
  await a.page.addInitScript(() => {
    window.warnungGesehen = false;
    new MutationObserver(() => {
      if (document.querySelector('.lc-hochladung-grund:not([hidden])') !== null) {
        window.warnungGesehen = true;
      }
    }).observe(document, { childList: true, attributes: true, subtree: true });
  });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  return { a, anfragen: anfragenMitschreiben(a.page), warnungGesehen: () => a.page.evaluate(() => window.warnungGesehen) };
}

async function sendenMit(page, text, dateien) {
  await eingabe(page).fill(text);
  await waehlen(page, dateien);
  await knopf(page).click();
  await expect(eintrag(page)).toHaveCount(1);
}

const nachrichtenMitAnhang = (anfragen) => anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad.endsWith('/nachrichten') && anfrage.koerper.anhaenge !== undefined);
const putZaehlen = (anfragen) => anfragen.filter((anfrage) => anfrage.methode === 'PUT');

const LAEUFT = { status: 409, contentType: 'application/json', body: JSON.stringify({ fehler: 'Für diese Anmeldung läuft schon ein Hochladen.', regel: 'laeuft' }) };

// B1 (a): der erste PUT geht durch (der Server hat alles), die Verbindung bricht vor der Antwort ab.
test('B1 (a): PUT kam an, Antwort verloren, Verbindung weg und wieder da → gesendet genau einmal, nie ⚠, höchstens 2 PUT', async ({ browser }) => {
  test.setTimeout(60000);
  const { a, anfragen, warnungGesehen } = await seiteAuf(browser);
  let puts = 0;
  await a.page.route('**/api/anhaenge/*', async (route) => {
    if (route.request().method() !== 'PUT') {
      await route.continue();
      return;
    }
    puts += 1;
    if (puts === 1) {
      await route.fetch();
      await a.kontext.setOffline(true);
      await route.abort('internetdisconnected');
      return;
    }
    await route.continue();
  });
  await sendenMit(a.page, 'Probe', [kleineDatei('probe.bin')]);
  await expect(datei(a.page, 'probe.bin').locator('.lc-hochladung-zustand')).toHaveText('wartet auf Verbindung');
  await a.kontext.setOffline(false);

  await expect(bestaetigt(a.page, 'Probe')).toHaveCount(1, { timeout: 20000 });
  await expect(eintrag(a.page)).toHaveCount(0);
  const gesendet = nachrichtenMitAnhang(anfragen);
  expect(gesendet).toHaveLength(1);
  expect(gesendet[0].koerper.text).toBe('Probe');
  expect(gesendet[0].koerper.anhaenge).toHaveLength(1);
  expect(await warnungGesehen(), '⚠ an der Datei').toBe(false);
  expect(putZaehlen(anfragen).length).toBeLessThanOrEqual(2);
  expect(saetzeMit(instanz.ordner, 'allgemein', 'Probe')).toBe(1);
  expect(unerwartet(a.fehler)).toEqual([]);
});

// B1 (b): der Server hält den abgebrochenen PUT noch fest (409 `laeuft`): später noch einmal, ohne ⚠.
test('B1 (b): 409 „laeuft“ nach dem Wiederverbinden → kein ⚠, erneuter PUT, gesendet genau einmal', async ({ browser }) => {
  test.setTimeout(60000);
  const { a, anfragen, warnungGesehen } = await seiteAuf(browser);
  let puts = 0;
  await a.page.route('**/api/anhaenge/*', async (route) => {
    if (route.request().method() !== 'PUT') {
      await route.continue();
      return;
    }
    puts += 1;
    if (puts === 1) {
      await route.fetch();
      await a.kontext.setOffline(true);
      await route.abort('internetdisconnected');
      return;
    }
    if (puts === 2) {
      await route.fulfill(LAEUFT);
      return;
    }
    await route.continue();
  });
  await sendenMit(a.page, 'Probe', [kleineDatei('probe.bin')]);
  await expect(datei(a.page, 'probe.bin').locator('.lc-hochladung-zustand')).toHaveText('wartet auf Verbindung');
  await a.kontext.setOffline(false);

  await expect(bestaetigt(a.page, 'Probe')).toHaveCount(1, { timeout: 30000 });
  expect(nachrichtenMitAnhang(anfragen)).toHaveLength(1);
  expect(await warnungGesehen(), '⚠ an der Datei').toBe(false);
  expect(puts).toBeGreaterThanOrEqual(3);
  expect(saetzeMit(instanz.ordner, 'allgemein', 'Probe')).toBe(1);
  expect(unerwartet(a.fehler)).toEqual([]);
});

// B1 (c): die Antwort geht verloren, ohne dass die Verbindung abreißt; „Erneut hochladen“ stößt auf 409 `fertig`.
test('B1 (c): Antwort verloren, „Erneut hochladen“ → 409 „fertig“ gilt als Erfolg, gesendet, kein ⚠ danach', async ({ browser }) => {
  test.setTimeout(60000);
  const { a, anfragen } = await seiteAuf(browser);
  let puts = 0;
  await a.page.route('**/api/anhaenge/*', async (route) => {
    if (route.request().method() !== 'PUT') {
      await route.continue();
      return;
    }
    puts += 1;
    if (puts === 1) {
      await route.fetch();
      await route.abort('failed');
      return;
    }
    await route.continue();
  });
  await sendenMit(a.page, 'Probe', [kleineDatei('probe.bin')]);
  await datei(a.page, 'probe.bin').getByRole('button', { name: 'probe.bin erneut hochladen' }).click();

  await expect(bestaetigt(a.page, 'Probe')).toHaveCount(1, { timeout: 20000 });
  await expect(eintrag(a.page)).toHaveCount(0);
  expect(nachrichtenMitAnhang(anfragen)).toHaveLength(1);
  expect(saetzeMit(instanz.ordner, 'allgemein', 'Probe')).toBe(1);
  expect(unerwartet(a.fehler)).toEqual([]);
});

// B2 (a): 200 KiB frei, vier Dateien zu 1 KiB — jede zählt 64 KiB, die vierte passt neben den eigenen drei nie.
test('B2 (a): 4 Dateien zu 1 KiB bei 200 KiB frei → binnen 75 s steht die vierte abgewiesen mit „Platz“ und „Entfernen“; danach gehen die übrigen hinaus', async ({ browser }) => {
  test.setTimeout(120000);
  const { a, anfragen } = await seiteAuf(browser);
  const token = await betaBeitreten(instanz.adresse);
  await perApiAblegen(instanz.adresse, token, [...Array.from({ length: 9 }, () => MIB), 843776]);
  const gesundheit = await perApi(instanz.adresse, { methode: 'GET', pfad: 'api/gesundheit' });
  expect(gesundheit.daten.anhang_belegt).toBe(10280960);

  await sendenMit(a.page, 'viele', ['k1.bin', 'k2.bin', 'k3.bin', 'k4.bin'].map((name) => kleineDatei(name, 1024)));

  const abgewiesen = eintrag(a.page).locator('.lc-hochladung-datei.lc-fehlgeschlagen');
  await expect(abgewiesen).toHaveCount(1, { timeout: 75000 });
  await expect(abgewiesen.locator('.lc-hochladung-grund')).toContainText('Platz');
  await abgewiesen.getByRole('button', { name: /entfernen$/ }).click();

  await expect(bestaetigt(a.page, 'viele')).toHaveCount(1, { timeout: 30000 });
  const gesendet = nachrichtenMitAnhang(anfragen);
  expect(gesendet).toHaveLength(1);
  expect(gesendet[0].koerper.anhaenge).toHaveLength(3);
  expect(unerwartet(a.fehler)).toEqual([]);
});

// Sollte 2, Probe 1: die Datei scheiterte ohne Antwort (⚠), danach reißt die Verbindung ab → sie wartet, ohne ⚠.
test('Sollte 2 (Probe 1): PUT ohne Antwort (⚠ „Keine Antwort“), erst danach offline → „wartet auf Verbindung“ ohne ⚠', async ({ browser }) => {
  test.setTimeout(60000);
  const { a } = await seiteAuf(browser);
  await a.page.route('**/api/anhaenge/*', (route) => {
    if (route.request().method() === 'PUT') {
      return route.abort('failed');
    }
    return route.continue();
  });
  await sendenMit(a.page, 'Probe', [kleineDatei('probe.bin')]);
  await expect(datei(a.page, 'probe.bin').locator('.lc-hochladung-grund')).toContainText('Keine Antwort vom Server.');

  await a.kontext.setOffline(true);

  await expect(datei(a.page, 'probe.bin').locator('.lc-hochladung-zustand')).toHaveText('wartet auf Verbindung');
  await expect(datei(a.page, 'probe.bin').locator('.lc-hochladung-grund')).toBeHidden();
  await expect(datei(a.page, 'probe.bin')).not.toContainText('⚠');
});

// Sollte 3: „Abbrechen“ ohne Verbindung gibt die Anmeldung frei, sobald die Verbindung wieder da ist.
test('Sollte 3: „Abbrechen“ in „wartet auf Verbindung“ → nach dem Wiederverbinden ist nichts mehr reserviert', async ({ browser }) => {
  test.setTimeout(60000);
  const { a } = await seiteAuf(browser);
  let puts = 0;
  await a.page.route('**/api/anhaenge/*', async (route) => {
    if (route.request().method() !== 'PUT') {
      await route.continue();
      return;
    }
    puts += 1;
    await a.kontext.setOffline(true);
    await route.abort('internetdisconnected');
  });
  await sendenMit(a.page, 'Probe', [kleineDatei('probe.bin')]);
  await expect(datei(a.page, 'probe.bin').locator('.lc-hochladung-zustand')).toHaveText('wartet auf Verbindung');
  await expect(leiste(a.page)).toContainText('Verbindung verloren');
  const vorher = await perApi(instanz.adresse, { methode: 'GET', pfad: 'api/gesundheit' });
  expect(vorher.daten.reserviert).toBeGreaterThan(0);

  await datei(a.page, 'probe.bin').getByRole('button', { name: 'probe.bin: Hochladen abbrechen' }).click();
  await a.kontext.setOffline(false);

  await expect.poll(async () => (await perApi(instanz.adresse, { methode: 'GET', pfad: 'api/gesundheit' })).daten.reserviert, { timeout: 20000 }).toBe(0);
  expect(puts).toBe(1);
});

// Sollte 5: das Zurücksetzen von neuAngemeldet bei einem Abbruch ohne Verbindung.
test('Sollte 5: 404 „abgelaufen“, neu angemeldet, PUT bricht offline ab, nach dem Wiederverbinden wieder 404 „abgelaufen“ → noch einmal neu angemeldet, gesendet, kein ⚠', async ({ browser }) => {
  test.setTimeout(60000);
  const { a, anfragen, warnungGesehen } = await seiteAuf(browser);
  const ABGELAUFEN = {
    status: 404, contentType: 'application/json', body: JSON.stringify({ fehler: 'Die Anmeldung ist abgelaufen.', grund: 'abgelaufen' }),
  };
  let puts = 0;
  await a.page.route('**/api/anhaenge/*', async (route) => {
    if (route.request().method() !== 'PUT') {
      await route.continue();
      return;
    }
    puts += 1;
    if (puts === 1 || puts === 3) {
      await route.fulfill(ABGELAUFEN);
      return;
    }
    if (puts === 2) {
      await a.kontext.setOffline(true);
      await route.abort('internetdisconnected');
      return;
    }
    await route.continue();
  });
  await sendenMit(a.page, 'Probe', [kleineDatei('probe.bin')]);
  await expect(datei(a.page, 'probe.bin').locator('.lc-hochladung-zustand')).toHaveText('wartet auf Verbindung');
  await a.kontext.setOffline(false);

  await expect(bestaetigt(a.page, 'Probe')).toHaveCount(1, { timeout: 30000 });
  expect(anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge')).toHaveLength(3);
  expect(await warnungGesehen(), '⚠ an der Datei').toBe(false);
  expect(saetzeMit(instanz.ordner, 'allgemein', 'Probe')).toBe(1);
});

// Sollte 2, Probe 2: weicht die Hauptansicht dem Beitreten-Bildschirm, während die Anmeldung noch unterwegs ist, wird sie
// danach nicht mit dem alten Token freigegeben (das Token gilt nicht mehr, die Anmeldung verfällt am Server).
test('Sollte 2 (Probe 2): 401 während der POST /api/anhaenge → nach der Antwort kein DELETE mit dem alten Token', async ({ browser }) => {
  test.setTimeout(60000);
  const NACHRICHTEN = '/api/channels/allgemein/nachrichten';
  const BEITRETEN = '/api/beitreten';
  instanz = await anhangStarten();
  const a = await kontextOeffnen(browser, { zusaetzlich: { [NACHRICHTEN]: [401], [BEITRETEN]: [409], '/api/channels/allgemein/aktivitaet': [401] } });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const anfragen = anfragenMitschreiben(a.page);
  let freigeben;
  const tor = new Promise((offen) => { freigeben = offen; });
  await a.page.route('**/api/anhaenge', async (route) => {
    await tor;
    await route.continue();
  });
  await sendenMit(a.page, 'Bericht', [kleineDatei('probe.bin')]);
  await a.page.route(`**${NACHRICHTEN}`, (route) => route.fulfill({
    status: 401, contentType: 'application/json', body: JSON.stringify({ fehler: 'Sitzung unbekannt.' }),
  }));
  await a.page.route(`**${BEITRETEN}`, (route) => route.fulfill({
    status: 409,
    contentType: 'application/json',
    body: JSON.stringify({ fehler: 'Der Name shoff ist vergeben.', belegt_von: '127.0.0.9', seit: '2026-10-28T15:00:00+01:00', vorschlag: 'shoff-2' }),
  }));
  await eingabe(a.page).fill('eins');
  await eingabe(a.page).press('Enter');
  await expect(a.page.getByLabel('Name')).toBeVisible();

  freigeben();
  await a.page.waitForTimeout(1500);

  expect(anfragen.filter((anfrage) => anfrage.methode === 'DELETE')).toEqual([]);
});
