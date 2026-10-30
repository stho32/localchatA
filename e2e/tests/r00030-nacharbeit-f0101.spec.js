'use strict';
// R00030 F0101 Nacharbeit aus dem Code-Review #43 (Sollte A1–A5, A7; Proben wörtlich aus dem Kommentar
// „code-reviewer #43 F0101“): Freigabe einer Anmeldung, die erst nach „Abbrechen“ oder „Verwerfen“ ankommt (A1).
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { anhangStarten, perApi, anfragenMitschreiben, waehlen, berichtPdf, textDatei } = require('./r00030-hilfen');

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

/// Zählt gleichzeitig laufende PUT-Hochladungen im Browser: +1 beim send, −1 wenn der Körper oben ist (`upload`
/// loadend) — der loadend der Antwort käme erst nach den Mikroaufgaben des load-Handlers der Seite, die den nächsten
/// PUT schon gestartet haben, und täuschte einen dritten vor; ohne
/// `ohneSperren` bleiben die Web Locks, mit ihm fehlt `navigator.locks` (kein sicherer Kontext).
async function hochladungenZaehlen(page, { ohneSperren }) {
  await page.addInitScript((sperrenEntfernen) => {
    if (sperrenEntfernen) {
      delete Navigator.prototype.locks;
    }
    window.hochladenZaehler = { laufend: 0, hoechstens: 0, fertig: 0 };
    const oeffnen = XMLHttpRequest.prototype.open;
    const senden = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function oeffnenMitMethode(methode, ...rest) {
      this.methodeR00030 = methode;
      return oeffnen.call(this, methode, ...rest);
    };
    XMLHttpRequest.prototype.send = function sendenGezaehlt(...rest) {
      if (this.methodeR00030 === 'PUT') {
        const zaehler = window.hochladenZaehler;
        zaehler.laufend += 1;
        zaehler.hoechstens = Math.max(zaehler.hoechstens, zaehler.laufend);
        this.upload.addEventListener('loadend', () => {
          zaehler.laufend -= 1;
          zaehler.fertig += 1;
        });
      }
      return senden.apply(this, rest);
    };
  }, ohneSperren);
}

// A3: „addInitScript setzt Navigator.prototype.locks undefined, 5 Dateien, XHR-Zähler im load-Handler → höchstens 2“.
test('A3 ohne Web Locks (Grenze je Tab): 5 Dateien → nie mehr als 2 PUT zugleich, alle 5 oben, gesendet', async ({ browser }) => {
  instanz = await anhangStarten();
  const { page, fehler } = await geraetOeffnen(browser, 'desktop');
  await hochladungenZaehlen(page, { ohneSperren: true });
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  expect(await page.evaluate(() => navigator.locks)).toBeUndefined();
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: 400 * 1024 });
  const anfragen = anfragenMitschreiben(page);

  await waehlen(page, ['a', 'b', 'c', 'd', 'e'].map((name) => textDatei(`${name}.txt`, 200 * 1024)));
  await knopf(page).click();

  await expect.poll(() => anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).length, { timeout: 20000 }).toBe(1);
  const zaehler = await page.evaluate(() => window.hochladenZaehler);
  expect(zaehler.fertig).toBe(5);
  expect(zaehler.hoechstens).toBe(2);
  expect(anfragen.find((anfrage) => anfrage.pfad.endsWith('/nachrichten')).koerper.anhaenge).toHaveLength(5);
  expect(fehler).toEqual([]);
});

// A4: eine Ausnahme beim Hochladen (hier: der erste PUT wirft schon beim send) endet sichtbar — ⚠ mit Grund und
// „Erneut hochladen“ statt ewig „lädt“ —, der Platz wird frei, die zweite Datei lädt; mit und ohne Web Locks.
for (const ohneSperren of [false, true]) {
  test(`A4 Ausnahme beim Hochladen (${ohneSperren ? 'ohne' : 'mit'} Web Locks): ⚠ und „Erneut hochladen“, die andere Datei lädt weiter`, async ({ browser }) => {
    instanz = await anhangStarten();
    const { page, fehler } = await geraetOeffnen(browser, 'desktop');
    await page.addInitScript((sperrenEntfernen) => {
      if (sperrenEntfernen) {
        delete Navigator.prototype.locks;
      }
      let ersterPut = true;
      const oeffnen = XMLHttpRequest.prototype.open;
      const senden = XMLHttpRequest.prototype.send;
      XMLHttpRequest.prototype.open = function oeffnenMitMethode(methode, ...rest) {
        this.methodeR00030 = methode;
        return oeffnen.call(this, methode, ...rest);
      };
      XMLHttpRequest.prototype.send = function sendenEinmalKaputt(...rest) {
        if (this.methodeR00030 === 'PUT' && ersterPut) {
          ersterPut = false;
          throw new Error('Probe A4');
        }
        return senden.apply(this, rest);
      };
    }, ohneSperren);
    await imBrowserBeitreten(page, instanz.adresse, 'shoff');
    const anfragen = anfragenMitschreiben(page);

    await waehlen(page, [textDatei('a.txt'), textDatei('b.txt')]);
    await knopf(page).click();

    const zeileA = eintrag(page).locator('.lc-hochladung-datei', { hasText: 'a.txt' });
    await expect(zeileA.locator('.lc-hochladung-grund')).toHaveText('⚠ Nicht hochgeladen (Probe A4).');
    await expect(eintrag(page).locator('.lc-hochladung-datei', { hasText: 'b.txt' }).locator('.lc-hochladung-zustand')).toHaveText('hochgeladen');
    await zeileA.getByRole('button', { name: 'a.txt erneut hochladen', exact: true }).click();
    await expect.poll(() => anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).length).toBe(1);
    expect(anfragen.find((anfrage) => anfrage.pfad.endsWith('/nachrichten')).koerper.anhaenge).toHaveLength(2);
    expect(fehler).toEqual([]);
  });
}
