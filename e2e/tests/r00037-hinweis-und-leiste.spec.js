'use strict';
// R00037 F0128 B0576 „Hinweis und Leiste sagen, was nach einem Ausfall geschieht“ (Abnahme-Probe wörtlich aus ADR-0037
// Ergänzung „Karte #65“, E2E, Ausnahmeweg, echtes Programm auf festem Port): jeder Fall einmal mit und einmal ohne
// SharedWorker (ohne wie e2e/tests/r00004-ein-strom-je-browser.spec.js:31). A ist `shoff`, die Nachrichten legt `beta` an.
//   (a) Strom getrennt (Instanz beendet), erster Abruf des Verlaufs gescheitert → „wird geladen, sobald die Verbindung
//       zurück ist.“
//   (b) Strom steht, Abruf antwortet 500 (page.route, kein Lauf) → „Channel erneut wählen, um es noch einmal zu versuchen.“
//   (c) Ein Versuch wird 11 s gehalten → die Leiste zeigt während des Versuchs „wird verbunden …“, danach den Countdown
//       bzw. nach Erfolg „✓ Wieder verbunden · …“; ihre Höhe ist in allen drei Zuständen gleich (± 1 px) bei 320, 375, 414 px.
//   Zustandswechsel: steht der Hinweis aus (b) und wird danach die Instanz beendet, zeigt derselbe Knoten den Text aus (a).
const { test, expect } = require('./rahmen');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten, beitreten, senden } = require('./senden-hilfen');
const { leiste, netzAus, netzAn } = require('./r00012-hilfen');
const { freierPort, sichtbarNach } = require('./hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');

const VERLAUF = '/api/channels/allgemein/nachrichten';
const GELADEN = '⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.';
const ERNEUT = '⚠ Verlauf nicht geladen — Channel erneut wählen, um es noch einmal zu versuchen.';
const WIRD_VERBUNDEN = '⚠ Verbindung verloren · wird verbunden …';
const ERSTER_ABRUF = (adresse) => adresse.pathname === VERLAUF && !adresse.searchParams.has('nach');
const GEHALTEN_MS = 11000;
const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Eine Instanz auf festem Port (der Ursprung bleibt über den Neustart) mit drei Nachrichten in # allgemein.
async function instanzMitVerlauf() {
  instanz = await sendenStarten({ port: await freierPort() });
  const token = await beitreten(instanz.adresse, 'beta');
  for (const text of ['v1', 'v2', 'v3']) {
    expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
  }
}

/// Ein Kontext für A, mit oder ohne SharedWorker; erlaubt die vorgeschriebenen Antworten des Falls.
async function kontextFuerA(browser, ohneSharedWorker, sammeln = {}) {
  const a = await kontextOeffnen(browser, { serverWegErwartet: true, ...sammeln });
  if (ohneSharedWorker) {
    await a.kontext.addInitScript(() => { delete window.SharedWorker; });
  }
  return a;
}

for (const [ohneSharedWorker, weg] of [[false, 'mit'], [true, 'ohne']]) {
  // (a) + (d: Mutanten „alter Wortlaut“ und „Hinweis ohne Unterscheidung“)
  test(`(a) Strom getrennt, erster Abruf gescheitert (${weg} SharedWorker): „wird geladen, sobald die Verbindung zurück ist.“`, async ({ browser }) => {
    test.setTimeout(60000);
    await instanzMitVerlauf();
    const a = await kontextFuerA(browser, ohneSharedWorker);
    let abrufBegonnen = null;
    const begonnen = new Promise((melden) => { abrufBegonnen = melden; });
    await a.page.route(ERSTER_ABRUF, async (route) => {
      abrufBegonnen();
      await warten(2500);
      await route.continue().catch(() => {});
    }, { times: 1 });

    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await begonnen;
    await a.page.waitForTimeout(500);
    await instanz.beenden();

    await expect(a.page.getByRole('alert')).toHaveText(GELADEN, { timeout: 15000 });
    await expect(leiste(a.page)).toHaveText(/^⚠ Verbindung verloren/);
    await expect(a.page.getByRole('alert')).toHaveCount(1);
    expect(a.fehler).toEqual([]);
  });

  // (b)
  test(`(b) Strom steht, Abruf antwortet 500 (${weg} SharedWorker): „Channel erneut wählen, um es noch einmal zu versuchen.“, kein Lauf`, async ({ browser }) => {
    test.setTimeout(60000);
    await instanzMitVerlauf();
    const a = await kontextFuerA(browser, ohneSharedWorker, { zusaetzlich: { [VERLAUF]: [500] } });
    const abrufe = [];
    a.page.on('request', (anfrage) => {
      if (ERSTER_ABRUF(new URL(anfrage.url()))) {
        abrufe.push(Date.now());
      }
    });
    await a.page.route(ERSTER_ABRUF, (route) => route.fulfill({ status: 500, body: '' }));

    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');

    await expect(a.page.getByRole('alert')).toHaveText(ERNEUT);
    await expect(leiste(a.page)).toBeHidden();
    await a.page.waitForTimeout(2500);
    expect(abrufe.length, 'kein Lauf: genau ein Abruf').toBe(1);
    await expect(a.page.getByRole('alert')).toHaveText(ERNEUT);
    expect(a.fehler).toEqual([]);
  });

  // Zustandswechsel bei stehendem Hinweis (b) → (a): derselbe Knoten, nur der Text.
  test(`Zustandswechsel (${weg} SharedWorker): steht der Hinweis aus (b) und endet die Instanz, zeigt derselbe Knoten den Text aus (a)`, async ({ browser }) => {
    test.setTimeout(60000);
    await instanzMitVerlauf();
    const a = await kontextFuerA(browser, ohneSharedWorker, { zusaetzlich: { [VERLAUF]: [500] } });
    await a.page.route(ERSTER_ABRUF, (route) => route.fulfill({ status: 500, body: '' }));
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await expect(a.page.getByRole('alert')).toHaveText(ERNEUT);
    await a.page.getByRole('alert').evaluate((knoten) => { window.hinweisKnoten = knoten; });

    await instanz.beenden();

    await expect(a.page.getByRole('alert')).toHaveText(GELADEN, { timeout: 15000 });
    const derselbe = await a.page.evaluate(() => {
      const alarme = document.querySelectorAll('[role="alert"]');
      return alarme.length === 1 && alarme[0] === window.hinweisKnoten && window.hinweisKnoten.isConnected;
    });
    expect(derselbe).toBe(true);
    expect(a.fehler).toEqual([]);
  });

  // Zustandswechsel durch das Netz des Tabs (Karte #65, A1): der Strom bleibt, der Tab ist offline — der Tab lädt von
  // selbst nach, sobald das Netz zurück ist, der Hinweis sagt es.
  test(`Netz des Tabs aus (${weg} SharedWorker): steht der Hinweis aus (b), zeigt derselbe Knoten offline den Text aus (a), online ist er weg`, async ({ browser }) => {
    test.setTimeout(90000);
    await instanzMitVerlauf();
    const a = await kontextFuerA(browser, ohneSharedWorker, { zusaetzlich: { [VERLAUF]: [500] } });
    await a.page.route(ERSTER_ABRUF, (route) => route.fulfill({ status: 500, body: '' }));
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await expect(a.page.getByRole('alert')).toHaveText(ERNEUT);
    await a.page.getByRole('alert').evaluate((knoten) => { window.hinweisKnoten = knoten; });

    await netzAus(a);

    await expect(a.page.getByRole('alert')).toHaveText(GELADEN);
    const derselbe = await a.page.evaluate(() => {
      const alarme = document.querySelectorAll('[role="alert"]');
      return alarme.length === 1 && alarme[0] === window.hinweisKnoten && window.hinweisKnoten.isConnected;
    });
    expect(derselbe).toBe(true);

    await a.page.unroute(ERSTER_ABRUF);
    await netzAn(a);

    await expect(a.page.getByRole('alert')).toHaveCount(0, { timeout: 30000 });
    await expect(a.page.getByRole('log').locator('.lc-body').filter({ hasText: /^v3$/ })).toHaveCount(1);
  });

  // (c) Stand der Leiste während des Versuchs: Strom weg, Neustart, der Versuch des Laden-Laufs wird 11 s gehalten.
  test(`(c) Versuch 11 s gehalten (${weg} SharedWorker): „wird verbunden …“ binnen 1 s, noch nach 5 s, nach 10 s der Countdown, dann „✓ Wieder verbunden“`, async ({ browser }) => {
    test.setTimeout(120000);
    await instanzMitVerlauf();
    const a = await kontextFuerA(browser, ohneSharedWorker);
    let abrufBegonnen = null;
    const begonnen = new Promise((melden) => { abrufBegonnen = melden; });
    let versuchBegonnen = null;
    const versuchSeit = new Promise((melden) => { versuchBegonnen = melden; });
    let abrufe = 0;
    await a.page.route(ERSTER_ABRUF, async (route) => {
      abrufe += 1;
      if (abrufe === 1) {
        abrufBegonnen();
        await warten(2500);
      } else if (abrufe === 2) {
        versuchBegonnen(Date.now());
        await warten(GEHALTEN_MS);
      }
      await route.continue().catch(() => {});
    });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await begonnen;
    await a.page.waitForTimeout(500);
    await instanz.beenden();
    await expect(a.page.getByRole('alert')).toHaveText(GELADEN, { timeout: 15000 });
    await a.page.evaluate(() => {
      const leisteElement = document.querySelector('.lc-center > .lc-leiste');
      window.proben = [];
      new MutationObserver(() => {
        if (!leisteElement.hidden) {
          window.proben.push({ text: leisteElement.textContent, hoehe: leisteElement.getBoundingClientRect().height });
        }
      }).observe(leisteElement, { childList: true, characterData: true, subtree: true, attributes: true });
    });

    await instanz.starten();
    const beginn = await versuchSeit;
    const standNach = await sichtbarNach(a.page, (erwartet) => {
      const text = document.querySelector('.lc-center > .lc-leiste').textContent;
      return text === erwartet;
    }, { argument: WIRD_VERBUNDEN, ab: beginn, timeout: 5000 });
    test.info().annotations.push({ type: 'Stand „wird verbunden …“ nach ms', description: String(standNach) });
    expect(standNach).toBeLessThanOrEqual(1000);
    await a.page.waitForTimeout(Math.max(0, beginn + 5000 - Date.now()));
    await expect(leiste(a.page)).toHaveText(WIRD_VERBUNDEN, { timeout: 500 });
    await expect(a.page.getByRole('alert')).toHaveText(GELADEN);

    const countdownNach = await sichtbarNach(a.page, () => /^⚠ Verbindung verloren · neuer Versuch in \d+ s$/.test(
      document.querySelector('.lc-center > .lc-leiste').textContent,
    ), { ab: beginn, timeout: 10000 });
    test.info().annotations.push({ type: 'Countdown nach ms', description: String(countdownNach) });
    expect(countdownNach).toBeGreaterThanOrEqual(9000);
    await expect(leiste(a.page)).toHaveText('✓ Wieder verbunden · Verlauf geladen', { timeout: 15000 });
    await expect(a.page.getByRole('alert')).toHaveCount(0);

    const proben = await a.page.evaluate(() => window.proben);
    const hoehen = (muster) => proben.filter((probe) => muster.test(probe.text)).map((probe) => probe.hoehe);
    const verbindend = hoehen(/wird verbunden …$/);
    const zaehlend = hoehen(/neuer Versuch in \d+ s$/);
    const abschluss = hoehen(/^✓ Wieder verbunden/);
    expect(verbindend.length, 'Stand „wird verbunden …“ gesehen').toBeGreaterThan(0);
    expect(zaehlend.length, 'Countdown gesehen').toBeGreaterThan(0);
    expect(abschluss.length, '„Wieder verbunden“ gesehen').toBeGreaterThan(0);
    const alle = [...verbindend, ...zaehlend, ...abschluss];
    expect(Math.max(...alle) - Math.min(...alle), `Höhen ${[...new Set(alle)]}`).toBeLessThanOrEqual(1);
    expect(a.fehler.filter((zeile) => !zeile.includes(VERLAUF))).toEqual([]);
  });
}

// (c) Höhe bei 320, 375 und 414 px: Anlass allein, Anlass + „neuer Versuch in n s“ und Anlass + „wird verbunden …“ —
// die Texte aus dem Modul der Seite, gesetzt über die echte Zeichenfunktion der Leiste (zwei Teile), in einem Zug gemessen.
for (const breite of [320, 375, 414]) {
  test(`(c) Gleiche Höhe (${breite} px): Anlass allein, Countdown und „wird verbunden …“ sind gleich hoch (± 1 px)`, async ({ browser }) => {
    instanz = await sendenStarten();
    const b = await geraetOeffnen(browser, 'telefon', { fenster: { width: breite, height: 812 } });
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    await netzAus(b);

    const messungen = await b.page.evaluate(async () => {
      const { warnungText, leisteFuellen } = await import('/web/verbindungsleiste.js');
      const jetzt = 1000000;
      const texte = [warnungText(null, jetzt), warnungText(jetzt + 10000, jetzt), warnungText(jetzt + 1000, jetzt), warnungText(null, jetzt, true)];
      const leisteElement = document.querySelector('.lc-center > .lc-leiste');
      return texte.map((text) => {
        leisteFuellen(leisteElement, text);
        return { text, hoehe: leisteElement.getBoundingClientRect().height };
      });
    });

    expect(messungen.map((messung) => messung.text)).toEqual([
      '⚠ Verbindung verloren', '⚠ Verbindung verloren · neuer Versuch in 10 s',
      '⚠ Verbindung verloren · neuer Versuch in 1 s', WIRD_VERBUNDEN,
    ]);
    const hoehen = messungen.map((messung) => messung.hoehe);
    expect(Math.max(...hoehen) - Math.min(...hoehen), `Höhen ${hoehen}`).toBeLessThanOrEqual(1);
  });
}
