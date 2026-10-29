'use strict';
// R00025 F0084 „Eine Verbindungswarnung mit Countdown, angesagt“: gegen das echte Programm, Chromium 1280 × 800. A ist
// `shoff`, B ist `anna`; „Strom getrennt“ ist `context.setOffline(true)`. Countdown-Text, Zeitpunkt und Quelle prüfen
// die Node-Tests mit gestellter Zeit (e2e/einheit/verbindungsleiste.test.mjs, live.test.mjs, verteiler.test.mjs).
const { test, expect } = require('@playwright/test');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten } = require('./senden-hilfen');
const { leiste } = require('./r00012-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

// „Countdown im Tab“: B getrennt → die Warnung nennt „neuer Versuch in“ mit einer Zahl von 1 bis 10 und zählt herunter
// (zwei Lesungen im Abstand von 1 s unterscheiden sich). Lesart: getrennt wird hier der Strom selbst — das Programm
// endet; `setOffline` erreicht den Strom im SharedWorker nicht, dann gilt „Netz aus ohne Meldung des Stroms“ (ohne
// Countdown, Node-Test „geltender Zeitpunkt“).
test('Countdown im Tab: getrennt zählt die Warnung bis zum nächsten Versuch herunter', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser, { serverWegErwartet: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');

  await instanz.stoppen();
  instanz = null;
  await expect(leiste(b.page)).toHaveText(/^⚠ Verbindung verloren · neuer Versuch in ([1-9]|10) s$/, { timeout: 20000 });
  const erste = await leiste(b.page).textContent();
  await b.page.waitForTimeout(1000);
  const zweite = await leiste(b.page).textContent();

  expect(zweite).not.toBe(erste);
  expect(b.fehler).toEqual([]);
});
