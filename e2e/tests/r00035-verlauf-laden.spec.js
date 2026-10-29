'use strict';
// R00035 B0560 „Verlauf meldet sein Laden“ (ADR-0037 Ergänzung „Karte #61“, 7h, 7j, 7k): das Modul web/verlauf.js im
// echten Browser, mit einem Ersatz für den Abruf, den der Test Schritt für Schritt beantwortet. `oeffnen(channel,
// signal)` liefert { art: 'geladen' | 'fehler' | 'abgebrochen' }; `stand()` meldet `gescheitert`.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { freierPort, fehlerSammeln } = require('./hilfen');
const { leiste } = require('./r00012-hilfen');

const VERLAUF = '/api/channels/allgemein/nachrichten';
const link = (page, name) => page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true });

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Legt im Browser einen Verlauf an, dessen Abrufe in `window.abrufe` warten, bis der Test sie mit
/// `window.antworten(stelle, antwort)` beantwortet; ein abgebrochenes Signal beantwortet sie mit „abgebrochen“.
async function verlaufImBrowser(page) {
  await page.goto(instanz.adresse);
  await page.evaluate(async () => {
    const { verlaufAnlegen } = await import('/web/verlauf.js');
    window.abrufe = [];
    const abrufen = (name, cursor, signal) => new Promise((liefern) => {
      window.abrufe.push({ name, liefern });
      signal.addEventListener('abort', () => liefern({ art: 'abgebrochen' }));
    });
    window.antworten = (stelle, antwort) => window.abrufe[stelle].liefern(antwort);
    window.verlauf = verlaufAnlegen({ abrufen });
    document.body.replaceChildren(window.verlauf.element);
  });
}

const BLOCK = {
  art: 'block',
  block: {
    channel: 'allgemein', aelter: false, neuer: false,
    nachrichten: ['v1', 'v2', 'v3'].map((text, stelle) => ({
      zeit: `2026-10-28T14:0${stelle}:00.000+01:00`, von: 'beta@127.0.0.1', art: 'agent', text,
    })),
  },
};
const NICHT_GELADEN = '⚠ Verlauf nicht geladen — Verbindung prüfen.';
const texte = (page) => page.locator('.lc-log-inhalt .lc-msg .lc-body').allTextContents();

test('Fehlschlag → { art: fehler }, stand meldet gescheitert, der Hinweis steht', async ({ page }) => {
  instanz = await sendenStarten();
  await verlaufImBrowser(page);

  const ergebnis = await page.evaluate(async () => {
    const geoeffnet = window.verlauf.oeffnen('allgemein');
    window.antworten(0, { art: 'fehler', status: 0 });
    return { ergebnis: await geoeffnet, stand: window.verlauf.stand() };
  });

  expect(ergebnis.ergebnis).toEqual({ art: 'fehler' });
  expect(ergebnis.stand).toMatchObject({ channel: 'allgemein', geladen: false, gescheitert: true });
  await expect(page.getByRole('alert')).toHaveText(NICHT_GELADEN);
});

test('erneutes Öffnen nach dem Fehlschlag: der Hinweis bleibt derselbe Knoten bis zum Ergebnis, dann { art: geladen }', async ({ page }) => {
  instanz = await sendenStarten();
  await verlaufImBrowser(page);

  const beobachtet = await page.evaluate(async () => {
    const erster = window.verlauf.oeffnen('allgemein');
    window.antworten(0, { art: 'fehler', status: 0 });
    await erster;
    const hinweis = document.querySelector('[role="alert"]');
    const zweiter = window.verlauf.oeffnen('allgemein');
    const waehrendDesVersuchs = { derselbe: document.querySelector('[role="alert"]') === hinweis, stand: window.verlauf.stand() };
    const dritter = window.verlauf.oeffnen('allgemein');
    window.antworten(1, { art: 'fehler', status: 500 });
    window.antworten(2, { art: 'fehler', status: 500 });
    await zweiter;
    await dritter;
    const nachDemZweitenFehlschlag = document.querySelector('[role="alert"]') === hinweis;
    return { waehrendDesVersuchs, nachDemZweitenFehlschlag };
  });
  expect(beobachtet.waehrendDesVersuchs.derselbe).toBe(true);
  expect(beobachtet.waehrendDesVersuchs.stand).toMatchObject({ geladen: false, gescheitert: false });
  expect(beobachtet.nachDemZweitenFehlschlag).toBe(true);

  const geladen = await page.evaluate(async (block) => {
    const vierter = window.verlauf.oeffnen('allgemein');
    window.antworten(3, block);
    return { ergebnis: await vierter, stand: window.verlauf.stand() };
  }, BLOCK);
  expect(geladen.ergebnis).toEqual({ art: 'geladen' });
  expect(geladen.stand).toMatchObject({ geladen: true, gescheitert: false });
  await expect(page.getByRole('alert')).toHaveCount(0);
  expect(await texte(page)).toEqual(['v1', 'v2', 'v3']);
});

test('ein abgebrochenes Signal des Aufrufers (Zeitgrenze) → { art: fehler } mit Hinweis', async ({ page }) => {
  instanz = await sendenStarten();
  await verlaufImBrowser(page);

  const beobachtet = await page.evaluate(async () => {
    const abbruch = new AbortController();
    const geoeffnet = window.verlauf.oeffnen('allgemein', abbruch.signal);
    abbruch.abort();
    return { ergebnis: await geoeffnet, stand: window.verlauf.stand() };
  });

  expect(beobachtet.ergebnis).toEqual({ art: 'fehler' });
  expect(beobachtet.stand).toMatchObject({ geladen: false, gescheitert: true });
  await expect(page.getByRole('alert')).toHaveText(NICHT_GELADEN);
});

test('übernimmt ein anderes Öffnen, liefert das erste { art: abgebrochen } und zeichnet nichts', async ({ page }) => {
  instanz = await sendenStarten();
  await verlaufImBrowser(page);

  const beobachtet = await page.evaluate(async (block) => {
    const abbruch = new AbortController();
    const erster = window.verlauf.oeffnen('allgemein', abbruch.signal);
    const zweiter = window.verlauf.oeffnen('technik');
    window.antworten(1, { ...block, block: { ...block.block, channel: 'technik' } });
    return { erster: await erster, zweiter: await zweiter, stand: window.verlauf.stand() };
  }, BLOCK);

  expect(beobachtet.erster).toEqual({ art: 'abgebrochen' });
  expect(beobachtet.zweiter).toEqual({ art: 'geladen' });
  expect(beobachtet.stand).toMatchObject({ channel: 'technik', geladen: true, gescheitert: false });
  await expect(page.getByRole('alert')).toHaveCount(0);
});

test('Schließen während des Abrufs: { art: abgebrochen }, kein Hinweis', async ({ page }) => {
  instanz = await sendenStarten();
  await verlaufImBrowser(page);

  const ergebnis = await page.evaluate(async () => {
    const geoeffnet = window.verlauf.oeffnen('allgemein');
    window.verlauf.schliessen();
    return geoeffnet;
  });

  expect(ergebnis).toEqual({ art: 'abgebrochen' });
  await expect(page.getByRole('alert')).toHaveCount(0);
});

test('während des neuen Ladens live Angekommenes steht nach dem Laden genau einmal, auch wenn der Block es bringt', async ({ page }) => {
  instanz = await sendenStarten();
  await verlaufImBrowser(page);

  await page.evaluate(async (block) => {
    const erster = window.verlauf.oeffnen('allgemein');
    window.antworten(0, { art: 'fehler', status: 0 });
    await erster;
    window.verlauf.nachrichtAnnehmen({ ...block.block.nachrichten[0], text: 'v1' }, false);
    const zweiter = window.verlauf.oeffnen('allgemein');
    window.verlauf.nachrichtAnnehmen(block.block.nachrichten[2], false);
    window.verlauf.nachrichtAnnehmen({ zeit: '2026-10-28T14:09:00.000+01:00', von: 'beta@127.0.0.1', art: 'agent', text: 'v4' }, false);
    window.antworten(1, block);
    await zweiter;
  }, BLOCK);

  await expect.poll(() => texte(page)).toEqual(['v1', 'v2', 'v3', 'v4']);
});

// R00035 B0562 „Hauptansicht verdrahtet das Laden“ (ADR-0037 7i; Pre-Mortem 1): scheitert das Laden nach der Rückkehr
// weiter (500), zieht eine neue Wahl desselben Channels den nächsten Versuch vor — ein Abruf sofort, der geplante
// entfällt, kein zweites Öffnen daneben. Mutante „die Wahl ruft erneutVersuchen“ → ein Abruf mehr beim geplanten
// Neuversuch.
test('Wahl desselben Channels während des Laden-Laufs: sofort ein Versuch, der geplante entfällt', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await sendenStarten({ port: await freierPort() });
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await kontext.newPage();
  const fehler = fehlerSammeln(page, { serverWegErwartet: true, zusaetzlich: { [VERLAUF]: [500] } });
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  await expect(page.getByText('Noch keine Nachrichten in # allgemein.')).toBeVisible();
  const oeffnungen = [];
  page.on('request', (anfrage) => {
    const ziel = new URL(anfrage.url());
    if (ziel.pathname === VERLAUF && !ziel.searchParams.has('nach')) {
      oeffnungen.push(Date.now());
    }
  });
  const verlaufsAbruf = (adresse) => adresse.pathname === VERLAUF;
  await page.route(verlaufsAbruf, (route) => route.fulfill({ status: 500, body: '' }));
  await link(page, 'technik').click();
  await link(page, 'allgemein').click();
  await expect(page.getByRole('alert')).toHaveText('⚠ Verlauf nicht geladen — Verbindung prüfen.');

  await instanz.beenden();
  await expect(leiste(page)).toHaveText(/^⚠ Verbindung verloren/);
  await instanz.starten();
  await expect.poll(() => oeffnungen.length, { timeout: 10000 }).toBe(3);
  await link(page, 'allgemein').click();
  await expect.poll(() => oeffnungen.length, { timeout: 1000 }).toBe(4);
  await page.waitForTimeout(2500);

  expect(oeffnungen.length).toBe(4);
  await expect(leiste(page)).toHaveText(/^⚠ Verbindung verloren/);
  await page.unroute(verlaufsAbruf);
  await link(page, 'allgemein').click();
  await expect(leiste(page)).toHaveText('✓ Wieder verbunden · Verlauf geladen');
  await expect(page.getByText('Noch keine Nachrichten in # allgemein.')).toBeVisible();
  expect(fehler).toEqual([]);
});
