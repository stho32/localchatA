'use strict';
// R00001 Instanz starten — US-8: der Browser auf der Adresse der Startzeile sieht die Oberfläche ohne Fehler.
const { test, expect } = require('./rahmen');
const { instanzStarten } = require('./hilfen');

test('US-8: Seite mit Titel localchatA, keine Fehler in Konsole und Netz', async ({ page }) => {
  const instanz = await instanzStarten();
  const fehler = [];
  page.on('console', (meldung) => {
    if (meldung.type() === 'error') {
      fehler.push('Konsole: ' + meldung.text());
    }
  });
  page.on('pageerror', (ausnahme) => fehler.push('Ausnahme: ' + ausnahme.message));
  page.on('requestfailed', (anfrage) => fehler.push('Netz: ' + anfrage.url()));
  page.on('response', (antwort) => {
    if (antwort.status() >= 400) {
      fehler.push('Netz ' + antwort.status() + ': ' + antwort.url());
    }
  });
  try {
    await page.goto(instanz.adresse);

    await expect(page).toHaveTitle('localchatA');
    await expect(page.locator('html')).toHaveAttribute('lang', 'de');
    await page.waitForLoadState('networkidle');
    expect(fehler).toEqual([]);
  } finally {
    await instanz.stoppen();
  }
});
