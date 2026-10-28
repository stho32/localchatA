'use strict';
// R00012 F0049 Gebündelte Ansage (US-13, US-14): gegen das echte Programm, Chromium 1280 × 800, Uhr ab 2026-10-28
// 15:00 (+01:00). A ist `shoff`, B ist `anna`; `beta` sendet per API. Die Bündelung selbst (10 s) prüft der Node-Test
// mit gestellter Zeit (e2e/einheit/ansage.test.mjs).
const { test, expect } = require('@playwright/test');
const { sendenStarten, beitreten, senden, kontextOeffnen, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { ROUTE, ansageRegion, bestaetigt } = require('./r00012-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Schreibt jeden Inhalt der Statusregion mit; liefert eine Funktion, die die Liste liest.
async function ansagenMitschreiben(page) {
  await page.evaluate(() => {
    window.ansagen = [];
    const region = document.querySelector('[role="status"][aria-live="polite"]');
    new MutationObserver(() => {
      if (region.textContent !== '') {
        window.ansagen.push(region.textContent);
      }
    }).observe(region, { childList: true, characterData: true, subtree: true });
  });
  return () => page.evaluate(() => window.ansagen);
}

// „Region“: genau eine Region role="status" mit aria-live="polite" außerhalb des Verlaufs, visuell verborgen (höchstens
// 1 × 1 px, nicht display:none); der Verlauf bleibt aria-live="off".
test('Region: eine verborgene Statusregion außerhalb des Verlaufs', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');

  await expect(ansageRegion(b.page)).toHaveCount(1);
  const lage = await ansageRegion(b.page).evaluate((region) => {
    const rahmen = region.getBoundingClientRect();
    return {
      imVerlauf: region.closest('[role="log"]') !== null,
      breite: rahmen.width,
      hoehe: rahmen.height,
      anzeige: getComputedStyle(region).display,
    };
  });
  expect(lage.imVerlauf).toBe(false);
  expect(lage.breite).toBeLessThanOrEqual(1);
  expect(lage.hoehe).toBeLessThanOrEqual(1);
  expect(lage.anzeige).not.toBe('none');
  await expect(b.page.getByRole('log')).toHaveAttribute('aria-live', 'off');
  expect(b.fehler).toEqual([]);
});

// „Ansage“ (US-13): `beta` sendet `Hallo` → „Neue Nachricht von beta@127.0.0.1 in # allgemein“; `beta` sendet an
// technik → keine weitere Ansage; B sendet selbst → keine weitere Ansage.
test('US-13: Neues wird angesagt — andere Channels und Eigenes nicht', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const angesagt = await ansagenMitschreiben(b.page);
  const token = await beitreten(instanz.adresse, 'beta');

  await senden(instanz.adresse, token, 'allgemein', 'Hallo');
  await expect(ansageRegion(b.page)).toHaveText('Neue Nachricht von beta@127.0.0.1 in # allgemein');
  await senden(instanz.adresse, token, 'technik', 'anderswo');
  await eingabe(b.page).fill('selbst');
  await eingabe(b.page).press('Enter');
  await expect(bestaetigt(b.page, 'selbst')).toHaveCount(1);
  await b.page.waitForTimeout(500);

  expect(await angesagt()).toEqual(['Neue Nachricht von beta@127.0.0.1 in # allgemein']);
  expect(b.fehler).toEqual([]);
});

// „Fehlschlag“ (US-14): A hat eben eine Ansage bekommen; die Route antwortet auf den POST 500 → die Region sagt sofort
// „Nachricht nicht gesendet: Nicht gespeichert — der Server konnte nicht schreiben.“
test('US-14: Fehlschlag sofort, auch kurz nach einer Ansage', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [500] } });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const token = await beitreten(instanz.adresse, 'beta');
  await senden(instanz.adresse, token, 'allgemein', 'Hallo');
  await expect(ansageRegion(a.page)).toHaveText('Neue Nachricht von beta@127.0.0.1 in # allgemein');
  await a.page.route(ROUTE, (route) => {
    if (route.request().method() !== 'POST') {
      return route.continue();
    }
    return route.fulfill({ status: 500, contentType: 'application/json', body: '{"fehler":"…"}' });
  });

  await eingabe(a.page).fill('kaputt');
  await eingabe(a.page).press('Enter');

  await expect(ansageRegion(a.page)).toHaveText('Nachricht nicht gesendet: Nicht gespeichert — der Server konnte nicht schreiben.');
  expect(a.fehler).toEqual([]);
});
