'use strict';
// R00012 F0047, Nacharbeit #22 (Code-Review #22 B1, B2, A3): Nachholen im Wettlauf mit dem Strom und unter Störung —
// gegen das echte Programm, Chromium 1280 × 800, Uhr ab 2026-10-28 15:00 (+01:00). B ist `anna`; `beta` sendet per API.
// Der Abruf `nach=` wird über page.route gehalten, abgebrochen oder mit 500 beantwortet.
const { test, expect } = require('@playwright/test');
const { sendenStarten, beitreten, senden, kontextOeffnen, imBrowserBeitreten, nachrichten } = require('./senden-hilfen');
const { leiste, bestaetigt, netzAus, netzAn } = require('./r00012-hilfen');

const NACHHOLEN = (adresse) => adresse.searchParams.has('nach');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

function ohneGescheiterteAnfragen(fehler) {
  return fehler.filter((zeile) => !/Failed to load resource: net::ERR_INTERNET_DISCONNECTED/.test(zeile));
}

const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));

// Schaltet sich zwischen Strom und Seite (MessagePort des SharedWorker): Ereignisse `nachricht` lassen sich verschlucken
// (gezählt in lcProbe.verschluckt), und der zuletzt gekommene Stand der Teilnehmer lässt sich noch einmal zustellen —
// so, wie der Server nach `NeuerStand` einen Stand schickt (src/http/ereignisse.rs).
async function stromProbeEinrichten(kontext) {
  await kontext.addInitScript(() => {
    const beschreibung = Object.getOwnPropertyDescriptor(MessagePort.prototype, 'onmessage');
    const probe = { verschlucken: false, verschluckt: 0, letzterStand: null, standWiederholen: null };
    window.lcProbe = probe;
    Object.defineProperty(MessagePort.prototype, 'onmessage', {
      configurable: true,
      get() {
        return beschreibung.get.call(this);
      },
      set(rueckruf) {
        const port = this;
        probe.standWiederholen = () => rueckruf.call(port, { data: probe.letzterStand });
        beschreibung.set.call(this, (nachricht) => {
          const daten = nachricht.data;
          const ereignis = daten !== null && daten.art === 'ereignis' && daten.ereignis;
          if (ereignis && ereignis.art === 'teilnehmer') {
            probe.letzterStand = daten;
          }
          if (ereignis && ereignis.art === 'nachricht' && probe.verschlucken) {
            probe.verschluckt += 1;
            return;
          }
          rueckruf.call(port, nachricht);
        });
      },
    });
  });
}

// „Während des Abrufs“ (Abnahme-Probe Code-Review #22 B1, wörtlich): „(1) B getrennt, beta sendet l1; Abruf nach= per
// route.fetch, danach beta „waehrend“, Antwort 0/300/1500 ms zurückgehalten → l1 und waehrend je 1× in Zeitfolge;
// (2) nachricht-Ereignisse verschluckt, beta s1, s2, dann NeuerStand, während des gehaltenen Abrufs beta „live“ → s1,
// s2, live je 1×; (3) ohne Halten unverändert. Mutante „Block nach Live-Ereignis verworfen“ rot.“
for (const haltenMs of [0, 300, 1500]) {
  test(`B1 (1): Antwort des Nachholens ${haltenMs} ms gehalten — l1 und waehrend je einmal in Zeitfolge`, async ({ browser }) => {
    instanz = await sendenStarten({ anzahl: 5 });
    const b = await kontextOeffnen(browser);
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    await expect(nachrichten(b.page)).toHaveCount(5);
    const token = await beitreten(instanz.adresse, 'beta');
    await netzAus(b);
    expect((await senden(instanz.adresse, token, 'allgemein', 'l1')).status).toBe(201);
    await b.page.route(NACHHOLEN, async (route) => {
      const antwort = await route.fetch();
      expect((await senden(instanz.adresse, token, 'allgemein', 'waehrend')).status).toBe(201);
      await warten(haltenMs);
      await route.fulfill({ response: antwort });
    });

    await netzAn(b);

    await expect(leiste(b.page)).toHaveText(/^✓ Wieder verbunden/, { timeout: 10000 });
    await expect(bestaetigt(b.page, '(l1|waehrend)').locator('.lc-body')).toHaveText(['l1', 'waehrend'], { timeout: 10000 });
    await warten(500);
    await expect(bestaetigt(b.page, '(l1|waehrend)').locator('.lc-body')).toHaveText(['l1', 'waehrend']);
    await expect(nachrichten(b.page)).toHaveCount(7);
    expect(ohneGescheiterteAnfragen(b.fehler)).toEqual([]);
  });
}

test('B1 (2): Ereignisse verschluckt, NeuerStand, während des gehaltenen Abrufs „live“ — s1, s2, live je einmal', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 5 });
  const b = await kontextOeffnen(browser);
  await stromProbeEinrichten(b.kontext);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(5);
  const token = await beitreten(instanz.adresse, 'beta');
  await b.page.evaluate(() => { window.lcProbe.verschlucken = true; });
  for (const text of ['s1', 's2']) {
    expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
  }
  await b.page.waitForFunction(() => window.lcProbe.verschluckt === 2);
  await b.page.evaluate(() => { window.lcProbe.verschlucken = false; });
  await b.page.route(NACHHOLEN, async (route) => {
    const antwort = await route.fetch();
    expect((await senden(instanz.adresse, token, 'allgemein', 'live')).status).toBe(201);
    await expect(bestaetigt(b.page, 'live')).toHaveCount(1);
    await route.fulfill({ response: antwort });
  });

  await b.page.evaluate(() => window.lcProbe.standWiederholen());

  await expect(bestaetigt(b.page, '(s1|s2|live)').locator('.lc-body')).toHaveText(['s1', 's2', 'live'], { timeout: 10000 });
  await warten(500);
  await expect(bestaetigt(b.page, '(s1|s2|live)').locator('.lc-body')).toHaveText(['s1', 's2', 'live']);
  await expect(nachrichten(b.page)).toHaveCount(8);
  expect(b.fehler).toEqual([]);
});

test('B1 (3): ohne Halten unverändert — l1 nachgeholt, danach waehrend live, je einmal in Zeitfolge', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 5 });
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(5);
  const token = await beitreten(instanz.adresse, 'beta');
  await netzAus(b);
  expect((await senden(instanz.adresse, token, 'allgemein', 'l1')).status).toBe(201);

  await netzAn(b);
  await expect(leiste(b.page)).toHaveText('✓ Wieder verbunden · 1 nachgeladen', { timeout: 10000 });
  expect((await senden(instanz.adresse, token, 'allgemein', 'waehrend')).status).toBe(201);

  await expect(bestaetigt(b.page, '(l1|waehrend)').locator('.lc-body')).toHaveText(['l1', 'waehrend'], { timeout: 10000 });
  await expect(nachrichten(b.page)).toHaveCount(7);
  expect(ohneGescheiterteAnfragen(b.fehler)).toEqual([]);
});
