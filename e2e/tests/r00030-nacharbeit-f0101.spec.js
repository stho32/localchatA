'use strict';
// R00030 F0101 Nacharbeit aus dem Code-Review #43 (Sollte A1–A5, A7; Proben wörtlich aus dem Kommentar
// „code-reviewer #43 F0101“): Freigabe einer Anmeldung, die erst nach „Abbrechen“ oder „Verwerfen“ ankommt (A1).
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { anhangStarten, perApi, anfragenMitschreiben, waehlen, berichtPdf } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const abbrechen = (page, name) => eintrag(page).getByRole('button', { name: `${name}: Hochladen abbrechen`, exact: true });
const loeschungen = (anfragen) => anfragen.filter((anfrage) => anfrage.methode === 'DELETE').map((anfrage) => anfrage.pfad);
const reserviert = async () => (await perApi(instanz.adresse, { methode: 'GET', pfad: 'api/gesundheit' })).daten.reserviert;

async function seiteAuf(browser, { bytesJeSekunde = 100 * 1024 } = {}) {
  instanz = await anhangStarten();
  const seite = await geraetOeffnen(browser, 'desktop');
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  const cdp = await seite.page.context().newCDPSession(seite.page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: bytesJeSekunde });
  return { ...seite, anfragen: anfragenMitschreiben(seite.page) };
}

/// Hält jeden POST /api/anhaenge zurück, bis `freigeben()`; liefert die IDs aus den Antworten.
async function anmeldungZurueckhalten(page) {
  let freigeben = null;
  const freigegeben = new Promise((weiter) => { freigeben = weiter; });
  const ids = [];
  await page.route('**/api/anhaenge', async (route) => {
    if (route.request().method() !== 'POST') {
      await route.continue();
      return;
    }
    await freigegeben;
    const antwort = await route.fetch();
    const daten = await antwort.json();
    ids.push(daten.anhang.id);
    await route.fulfill({ response: antwort, json: daten });
  });
  return { freigeben: () => freigeben(), ids };
}

// A1: „POST /api/anhaenge zurückhalten, „Abbrechen“ klicken, POST freigeben → danach DELETE /api/anhaenge/<id> und
// /api/gesundheit reserviert 0. Gleich für „Verwerfen“.“
test('A1 Abbrechen während der Anmeldung: die später angekommene Anmeldung wird freigegeben, reserviert 0', async ({ browser }) => {
  const { page, anfragen, fehler } = await seiteAuf(browser);
  const anmeldung = await anmeldungZurueckhalten(page);

  await waehlen(page, [berichtPdf()]);
  await eingabe(page).fill('Hallo');
  await knopf(page).click();
  await abbrechen(page, 'bericht.pdf').click();
  await expect(eintrag(page)).toHaveCount(0);
  anmeldung.freigeben();

  await expect.poll(() => anmeldung.ids.length).toBe(1);
  await expect.poll(() => loeschungen(anfragen)).toEqual([`/api/anhaenge/${anmeldung.ids[0]}`]);
  await expect.poll(reserviert).toBe(0);
  expect(anfragen.filter((anfrage) => anfrage.methode === 'PUT')).toEqual([]);
  expect(fehler).toEqual([]);
});

test('A1 Verwerfen während der Anmeldung: die später angekommene Anmeldung wird freigegeben, reserviert 0', async ({ browser }) => {
  const { page, anfragen, fehler } = await seiteAuf(browser);
  const anmeldung = await anmeldungZurueckhalten(page);

  await waehlen(page, [berichtPdf()]);
  await eingabe(page).fill('Hallo');
  await knopf(page).click();
  await eintrag(page).getByRole('button', { name: 'Verwerfen', exact: true }).click();
  await expect(eingabe(page)).toHaveValue('Hallo');
  anmeldung.freigeben();

  await expect.poll(() => anmeldung.ids.length).toBe(1);
  await expect.poll(() => loeschungen(anfragen)).toEqual([`/api/anhaenge/${anmeldung.ids[0]}`]);
  await expect.poll(reserviert).toBe(0);
  expect(anfragen.filter((anfrage) => anfrage.methode === 'PUT')).toEqual([]);
  expect(fehler).toEqual([]);
});

// A2: „erster POST per route.abort, „Erneut hochladen“ → kein PUT auf null, ein neuer POST /api/anhaenge, danach PUT
// auf die neue ID und Senden.“
test('A2 Erneut nach gescheiterter Anmeldung: kein PUT auf null, eine neue Anmeldung, PUT auf deren ID, gesendet', async ({ browser }) => {
  const { page, anfragen, fehler } = await seiteAuf(browser, { bytesJeSekunde: 1024 * 1024 });
  let erster = true;
  await page.route('**/api/anhaenge', (route) => {
    if (route.request().method() === 'POST' && erster) {
      erster = false;
      return route.abort();
    }
    return route.continue();
  });
  const ids = [];
  page.on('response', async (antwort) => {
    if (antwort.request().method() === 'POST' && new URL(antwort.url()).pathname === '/api/anhaenge' && antwort.status() === 201) {
      ids.push((await antwort.json()).anhang.id);
    }
  });

  await waehlen(page, [berichtPdf()]);
  await knopf(page).click();
  const erneut = eintrag(page).getByRole('button', { name: 'bericht.pdf erneut hochladen', exact: true });
  await expect(erneut).toBeVisible();
  await erneut.click();

  await expect.poll(() => anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).length, { timeout: 15000 }).toBe(1);
  const puts = anfragen.filter((anfrage) => anfrage.methode === 'PUT').map((anfrage) => anfrage.pfad);
  expect(puts).toEqual([`/api/anhaenge/${ids[0]}`]);
  expect(anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge')).toHaveLength(2);
  expect(anfragen.find((anfrage) => anfrage.pfad.endsWith('/nachrichten')).koerper).toEqual({ text: '', anhaenge: [ids[0]] });
  expect(fehler.filter((zeile) => !zeile.includes('net::ERR_FAILED'))).toEqual([]);
});
