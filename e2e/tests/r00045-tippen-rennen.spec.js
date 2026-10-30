'use strict';
// R00045 F0161 (Karte #83 A3, Code-Review #77): Rennen zwischen Setzen und Löschen. Im WLAN überholt ein schnelles
// DELETE ein langsames PUT — das PUT käme danach am Server an und ließe „anna tippt …“ bis zu 6 s stehen, obwohl anna
// das Feld geleert, gesendet oder den Channel gewechselt hat. Probe: PUT auf /aktivitaet 400 ms verzögert, alles
// andere unverzögert; 1,5 s nach dem Ende steht bei ben sofort (kein Wiederholen bis 6 s) nichts mehr.
const { test, expect } = require('./rahmen');
const { mitText } = require('./senden-hilfen');
const { sichtbarNach } = require('./hilfen');
const { ANNA_TIPPT, zeigtTipp, zweiBrowserStarten } = require('./r00045-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const PUT_VERZOEGERT_MS = 400;
const NACH_DEM_ENDE_MS = 1500;
const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));
const tippzeile = (page) => page.locator('.lc-comp .lc-comp-tipp');

/// annas PUT an /aktivitaet kommt erst nach `PUT_VERZOEGERT_MS` an, DELETE und alles andere sofort.
async function putVerzoegern(page) {
  await page.route('**/api/channels/*/aktivitaet', async (route) => {
    if (route.request().method() === 'PUT') {
      await warten(PUT_VERZOEGERT_MS);
    }
    await route.continue();
  });
}

async function annaTippt(anna) {
  await putVerzoegern(anna.page);
  await anna.page.keyboard.type('a');
}

async function benSiehtNachDemEndeNichts(ben, ab) {
  const verbleibend = ab + NACH_DEM_ENDE_MS - Date.now();
  await warten(Math.max(verbleibend, 0));
  expect(await tippzeile(ben.page).textContent(), '1,5 s nach dem Ende').toBe('');
}

async function starten(browser) {
  const paar = await zweiBrowserStarten(browser);
  instanz = paar.instanz;
  return paar;
}

test('Feld leeren (US-5): PUT 400 ms verzögert, Backspace sofort → 1,5 s danach ist die Zeile bei ben leer', async ({ browser }) => {
  const { anna, ben } = await starten(browser);

  await annaTippt(anna);
  await anna.page.keyboard.press('Backspace');
  await benSiehtNachDemEndeNichts(ben, Date.now());
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Senden (US-5): PUT 400 ms verzögert, Enter sofort → 1,5 s nach Eintreffen der Nachricht ist die Zeile bei ben leer', async ({ browser }) => {
  const { anna, ben } = await starten(browser);

  await annaTippt(anna);
  await anna.page.keyboard.press('Enter');
  await expect(mitText(ben.page, /^a$/)).toHaveCount(1);
  await benSiehtNachDemEndeNichts(ben, Date.now());
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Channel-Wechsel (US-5): PUT 400 ms verzögert, Wechsel nach # technik sofort → 1,5 s danach ist die Zeile bei ben leer', async ({ browser }) => {
  const { anna, ben } = await starten(browser);

  await annaTippt(anna);
  await anna.page.evaluate(() => { window.location.hash = '#/c/technik'; });
  await benSiehtNachDemEndeNichts(ben, Date.now());
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Kontrolle: ohne Eile (PUT verzögert, dann erst Backspace) erscheint die Zeile und geht binnen 1 s wieder', async ({ browser }) => {
  const { anna, ben } = await starten(browser);

  await annaTippt(anna);
  await sichtbarNach(ben.page, zeigtTipp, { argument: ANNA_TIPPT, ab: Date.now() });
  const ab = Date.now();
  await anna.page.keyboard.press('Backspace');
  const ms = await sichtbarNach(ben.page, zeigtTipp, { argument: '', ab });
  expect(ms).toBeLessThanOrEqual(1000);
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});
