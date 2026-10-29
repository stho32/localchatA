'use strict';
// R00035 B0560 „Verlauf meldet sein Laden“ (ADR-0037 Ergänzung „Karte #61“, 7h, 7j, 7k): das Modul web/verlauf.js im
// echten Browser, mit einem Ersatz für den Abruf, den der Test Schritt für Schritt beantwortet. `oeffnen(channel,
// signal)` liefert { art: 'geladen' | 'fehler' | 'abgebrochen' }; `stand()` meldet `gescheitert`.
const { test, expect } = require('./rahmen');
const { sendenStarten } = require('./senden-hilfen');

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
