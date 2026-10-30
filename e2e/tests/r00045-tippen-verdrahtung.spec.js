'use strict';
// R00045 F0161 (Karte #83 A1, Code-Review #77): die Verdrahtung „ohne Strom nicht melden“ in web/hauptansicht.js
// (`tippen.verbindung(…)`), die kein Test prüfte — ohne die Zeile blieben alle r00045-Tests grün.
const { test, expect } = require('./rahmen');
const { eingabe } = require('./senden-hilfen');
const { zweiBrowserStarten } = require('./r00045-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));
const leiste = (page) => page.locator('.lc-center > .lc-leiste');

/// Schreibt jeden Aufruf an /aktivitaet als „PUT allgemein“ | „DELETE allgemein“ mit (nach dem Absenden der Anfrage).
function aufrufeMitschreiben(page) {
  const aufrufe = [];
  page.on('request', (anfrage) => {
    const treffer = new URL(anfrage.url()).pathname.match(/^\/api\/channels\/([^/]+)\/aktivitaet$/);
    if (treffer !== null) {
      aufrufe.push(`${anfrage.method()} ${treffer[1]}`);
    }
  });
  return aufrufe;
}

test('Ohne Strom nicht melden (A1): Strom getrennt, anna tippt weiter und leert das Feld → kein weiterer Aufruf; Strom wieder da → die erste Taste meldet sofort', async ({ browser }) => {
  const paar = await zweiBrowserStarten(browser, { annaOhneWorker: true });
  instanz = paar.instanz;
  const { anna } = paar;
  const aufrufe = aufrufeMitschreiben(anna.page);

  await anna.page.keyboard.type('a');
  await expect.poll(() => aufrufe).toEqual(['PUT allgemein']);
  await anna.page.evaluate(() => window.stromTrennen());
  await expect(leiste(anna.page)).toHaveText(/^⚠ Verbindung verloren/);

  await anna.page.keyboard.type('bcd');
  await eingabe(anna.page).press('Control+a');
  await eingabe(anna.page).press('Backspace');
  await warten(1500);
  expect(aufrufe, 'ohne Strom weder PUT noch DELETE').toEqual(['PUT allgemein']);
  expect(anna.fehler).toEqual([]);

  await expect(leiste(anna.page)).toHaveText(/Wieder verbunden/, { timeout: 10000 });
  await anna.page.keyboard.type('x');
  await expect.poll(() => aufrufe, { intervals: [25] }).toEqual(['PUT allgemein', 'PUT allgemein']);
  expect(anna.fehler).toEqual([]);
});
