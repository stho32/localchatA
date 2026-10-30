'use strict';
// R00045 F0161 B0689 (Karte #77): „anna tippt …“ an der Eingabe — zwei Browser (`anna`, `ben`) in `# allgemein` gegen
// das echte Programm. Erscheinen binnen 1 s, nie beim Tippenden; weg binnen 1 s mit Senden, leerem Feld,
// Channel-Wechsel und Stromabriss; durch Ablauf nicht vor `bis` und spätestens 8 s nach der Meldung; flüchtig beim
// Neuladen und beim neuen Strom (Mutante M12).
const { test, expect } = require('./rahmen');
const { sichtbarNach } = require('./hilfen');
const { eingabe, mitText } = require('./senden-hilfen');
const { ANNA_TIPPT, zeigtTipp, zweiBrowserStarten } = require('./r00045-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ZUSTELLUNG_MS = 1000;
const tippzeile = (page) => page.locator('.lc-comp .lc-comp-tipp');
const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));

async function zweiBrowser(browser, optionen) {
  const paar = await zweiBrowserStarten(browser, optionen);
  instanz = paar.instanz;
  return paar;
}

async function annaTipptBenSieht(anna, ben, taste = 'H') {
  const ab = Date.now();
  await anna.page.keyboard.type(taste);
  const ms = await sichtbarNach(ben.page, zeigtTipp, { argument: ANNA_TIPPT, ab });
  expect(ms, 'Erscheinen bei ben').toBeLessThanOrEqual(ZUSTELLUNG_MS);
}

async function benSiehtNichtsMehrBinnen1s(ben, ab) {
  const ms = await sichtbarNach(ben.page, zeigtTipp, { argument: '', ab });
  expect(ms, 'Verschwinden bei ben').toBeLessThanOrEqual(ZUSTELLUNG_MS);
}

test('Erscheinen und Senden (US-5): bei ben binnen 1 s „anna tippt …“, bei anna nichts; Senden → weg binnen 1 s, die Nachricht steht', async ({ browser }) => {
  const { anna, ben } = await zweiBrowser(browser);

  await annaTipptBenSieht(anna, ben);
  await expect(tippzeile(anna.page)).toHaveText('');
  await anna.page.keyboard.type('allo');
  const ab = Date.now();
  await anna.page.keyboard.press('Enter');

  await benSiehtNichtsMehrBinnen1s(ben, ab);
  await expect(mitText(ben.page, /^Hallo$/)).toHaveCount(1);
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Leeres Feld und Channel-Wechsel (US-5): anna leert das Feld bzw. wechselt nach # technik → bei ben weg binnen 1 s', async ({ browser }) => {
  const { anna, ben } = await zweiBrowser(browser);

  await annaTipptBenSieht(anna, ben);
  let ab = Date.now();
  await anna.page.keyboard.press('Backspace');
  await benSiehtNichtsMehrBinnen1s(ben, ab);

  await annaTipptBenSieht(anna, ben);
  ab = Date.now();
  await anna.page.evaluate(() => { window.location.hash = '#/c/technik'; });
  await benSiehtNichtsMehrBinnen1s(ben, ab);
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Ablauf (US-5): anna tippt zwei Zeichen und hört auf → bei ben nicht vor bis, spätestens 8 s nach der Meldung weg', async ({ browser }) => {
  test.setTimeout(45000);
  const { anna, ben } = await zweiBrowser(browser);
  await ben.page.evaluate(() => {
    window.tippzeileLeerUm = [];
    const zeile = document.querySelector('.lc-comp .lc-comp-tipp');
    new MutationObserver(() => {
      if (zeile.textContent === '') {
        window.tippzeileLeerUm.push(Date.now());
      }
    }).observe(zeile, { childList: true, characterData: true, subtree: true });
  });
  const meldungen = [];
  anna.page.on('request', (anfrage) => {
    if (anfrage.url().endsWith('/aktivitaet')) {
      meldungen.push({ methode: anfrage.method(), gesendetMs: Date.now() });
    }
  });
  const antworten = [];
  anna.page.on('response', async (antwort) => {
    if (antwort.url().endsWith('/aktivitaet')) {
      antworten.push(await antwort.json());
    }
  });

  const tasteMs = Date.now();
  await anna.page.keyboard.type('Hi');
  await expect(tippzeile(ben.page)).toHaveText(ANNA_TIPPT);
  await sichtbarNach(ben.page, zeigtTipp, { argument: '', ab: tasteMs, timeout: 10000 });
  const leerUm = await ben.page.evaluate(() => window.tippzeileLeerUm);

  expect(meldungen.map((meldung) => meldung.methode)).toEqual(['PUT']);
  await expect.poll(() => antworten.length).toBe(1);
  const [antwort] = antworten;
  const gueltigMs = Date.parse(antwort.bis) - Date.parse(antwort.jetzt);
  expect(gueltigMs).toBe(6000);
  expect(leerUm).toHaveLength(1);
  expect(leerUm[0], 'nie vor bis').toBeGreaterThanOrEqual(meldungen[0].gesendetMs + gueltigMs);
  expect(leerUm[0] - tasteMs, 'spätestens 8 s').toBeLessThanOrEqual(8000);
  test.info().annotations.push({ type: 'weg nach ms', description: String(leerUm[0] - meldungen[0].gesendetMs) });
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Flüchtig (US-5): ben lädt neu, während anna tippt → keine Anzeige, bis anna wieder meldet', async ({ browser }) => {
  const { anna, ben } = await zweiBrowser(browser);
  await annaTipptBenSieht(anna, ben);

  await ben.page.reload();
  await expect(ben.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await warten(ZUSTELLUNG_MS);
  await expect(tippzeile(ben.page)).toHaveText('');

  await anna.page.keyboard.press('Backspace');
  await annaTipptBenSieht(anna, ben, 'W');
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Strom getrennt (ux-reviewer E, M12): reißt bens Strom ab, ist die Anzeige binnen 1 s weg und kommt erst mit einer neuen Meldung wieder', async ({ browser }) => {
  const { anna, ben } = await zweiBrowser(browser, { benOhneWorker: true });
  await annaTipptBenSieht(anna, ben);

  const ab = Date.now();
  await ben.page.evaluate(() => window.stromTrennen());
  await benSiehtNichtsMehrBinnen1s(ben, ab);
  await expect(ben.page.locator('.lc-center > .lc-leiste')).toHaveText(/^⚠ Verbindung verloren/);
  await expect(ben.page.locator('.lc-center > .lc-leiste')).toHaveText(/Wieder verbunden/, { timeout: 10000 });
  await warten(ZUSTELLUNG_MS);
  await expect(tippzeile(ben.page)).toHaveText('');

  await anna.page.keyboard.press('Backspace');
  await annaTipptBenSieht(anna, ben, 'W');
  expect(anna.fehler).toEqual([]);
});
