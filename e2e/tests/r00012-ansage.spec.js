'use strict';
// R00012 F0049 Gebündelte Ansage (US-13, US-14): gegen das echte Programm, Chromium 1280 × 800, Uhr ab 2026-10-28
// 15:00 (+01:00). A ist `shoff`, B ist `anna`; `beta` sendet per API. Die Bündelung selbst (10 s) prüft der Node-Test
// mit gestellter Zeit (e2e/einheit/ansage.test.mjs).
const { test, expect } = require('./rahmen');
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

// „Takt der Sammelansage“ (Nacharbeit #22; Abnahme-Probe Code-Review #22 B3 a, wörtlich): „Ruhe > 10 s, beta h1, dann
// h2–h4 in 3 s → zuerst „Neue Nachricht von beta@127.0.0.1 in # allgemein“, 10 s ± 1 s danach genau einmal „3 neue
// Nachrichten in # allgemein“.“ Die Regel prüft „Bündeln“ exakt mit gestellter Zeit, hier die Zustellung über den
// Zeitgeber der Seite. Die Uhr startet beim Erscheinen der ersten Ansage in der Region (MutationObserver im Browser);
// nach der Sammelansage wird bis 13 s nach der ersten gewartet, ob noch eine kommt. B hat vorher nichts angesagt
// bekommen — das ist die Ruhe. Mutante „Takt nicht gestellt“ wird rot.
const TAKT_MS = 10000;
const TAKT_SPIEL_MS = 1000;

test('Takt der Sammelansage: zuerst h1 einzeln, 10 s ± 1 s danach genau einmal „3 neue Nachrichten“', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const token = await beitreten(instanz.adresse, 'beta');
  await b.page.evaluate(() => {
    window.ansagenMitZeit = [];
    const region = document.querySelector('[role="status"][aria-live="polite"]');
    new MutationObserver(() => {
      if (region.textContent !== '') {
        window.ansagenMitZeit.push({ text: region.textContent, um: Date.now() });
      }
    }).observe(region, { childList: true, characterData: true, subtree: true });
  });

  expect((await senden(instanz.adresse, token, 'allgemein', 'h1')).status).toBe(201);
  await b.page.waitForFunction(() => window.ansagenMitZeit.length === 1, null, { polling: 'raf', timeout: 10000 });
  const gesendetAb = Date.now();
  for (const text of ['h2', 'h3', 'h4']) {
    expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
  }
  const sendedauer = Date.now() - gesendetAb;
  await expect(bestaetigt(b.page, 'h[1234]')).toHaveCount(4);
  await b.page.waitForFunction(() => window.ansagenMitZeit.length >= 2, null, { polling: 'raf', timeout: 20000 });
  await b.page.waitForFunction(() => Date.now() - window.ansagenMitZeit[0].um >= 13000, null, { timeout: 20000 });

  const ansagen = await b.page.evaluate(() => window.ansagenMitZeit);
  const abstand = ansagen[1].um - ansagen[0].um;
  console.log(`Takt der Sammelansage: ${abstand} ms nach der ersten (Regel ${TAKT_MS} ± ${TAKT_SPIEL_MS}); h2–h4 in ${sendedauer} ms`);
  expect(sendedauer).toBeLessThan(3000);
  expect(ansagen.map(({ text }) => text)).toEqual([
    'Neue Nachricht von beta@127.0.0.1 in # allgemein',
    '3 neue Nachrichten in # allgemein',
  ]);
  expect(Math.abs(abstand - TAKT_MS)).toBeLessThanOrEqual(TAKT_SPIEL_MS);
  expect(b.fehler).toEqual([]);
});
