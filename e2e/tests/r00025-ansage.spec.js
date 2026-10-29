'use strict';
// R00025 F0084 „Eine Live-Region“ (UX-Vorprüfung #36 Muss V1): die Ansage-Region ist die einzige Live-Region der
// Hauptansicht; Hinweiszeile und Statuszeile sagen über sie an. Gegen das echte Programm, Chromium 1280 × 800.
const { test, expect } = require('@playwright/test');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { ansageRegion } = require('./r00012-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

// B0487: A fügt `rot ` + ESC + `[31m` ein → die Ansage-Region enthält „Steuerzeichen U+001B an Stelle 5 — bitte
// entfernen.“ (ohne ⚠); die Hinweiszeile zeigt den Text mit ⚠ und trägt selbst keine Live-Region, auch keine darin
// (Mutante „Hinweis nicht angesagt“ wird rot).
test('Hinweis: der Fehler der Hinweiszeile wird über die Ansage-Region angesagt', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');

  await eingabe(a.page).fill('rot \u001b[31m');

  await expect(a.page.locator('#lc-comp-hinweis')).toHaveText('⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.');
  await expect(ansageRegion(a.page)).toHaveText('Steuerzeichen U+001B an Stelle 5 — bitte entfernen.');
  const regionenInDerEingabe = await a.page.locator('.lc-comp').evaluate((eingabeRahmen) => eingabeRahmen
    .querySelectorAll('[aria-live]:not([aria-live="off"]), [role="status"], [role="alert"]').length);
  expect(regionenInDerEingabe).toBe(0);
  expect(a.fehler).toEqual([]);
});
