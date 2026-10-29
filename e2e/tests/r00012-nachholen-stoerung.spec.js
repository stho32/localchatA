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

// Wacht je Bild über die Leiste: steht dort „✓ Wieder verbunden“, solange eine der Nachrichten fehlt, merkt sie sich
// den Verstoß (window.lcVerstoesse).
async function meldungOhneLueckeBewachen(page, texte) {
  await page.evaluate((erwartet) => {
    window.lcVerstoesse = [];
    const pruefen = () => {
      const meldung = document.querySelector('.lc-verlauf .lc-leiste');
      const gezeigt = [...document.querySelectorAll('[role="log"] .lc-msg:not(.lc-vorlaeufig) .lc-body')].map((body) => body.textContent);
      const fehlt = erwartet.filter((text) => !gezeigt.includes(text));
      const wiederVerbunden = meldung !== null && !meldung.hidden && meldung.textContent.startsWith('✓ Wieder verbunden');
      if (wiederVerbunden && fehlt.length > 0) {
        window.lcVerstoesse.push(`${meldung.textContent} — es fehlen ${fehlt.join(', ')}`);
      }
      requestAnimationFrame(pruefen);
    };
    requestAnimationFrame(pruefen);
  }, texte);
}

/// Konsolenfehler außer den gestellten: abgebrochene Anfragen und 500 auf den Nachhol-Abruf (R00012 F0047 „Konsole“).
function ohneGestellteFehler(fehler) {
  return ohneGescheiterteAnfragen(fehler).filter((zeile) => !/Failed to load resource: net::ERR_FAILED/.test(zeile));
}

// „Gescheitertes Nachholen“ (Abnahme-Probe Code-Review #22 B2, wörtlich): „B getrennt, beta g1–g3, erster nach=-Abruf
// abort bzw. 500, danach Durchlass → nie „✓ Wieder verbunden“, solange g1–g3 fehlen; spätestens nach dem nächsten
// erfolgreichen Abruf je 1×; Variante zweimal scheitern; ohne Störung US-6 grün.“
const STOERUNGEN = [
  { name: 'erster Abruf abgebrochen', scheitern: ['abort'] },
  { name: 'erster Abruf 500', scheitern: [500] },
  { name: 'zweimal gescheitert (abort, 500)', scheitern: ['abort', 500] },
  { name: 'zweimal gescheitert (500, 500)', scheitern: [500, 500] },
];
for (const { name, scheitern } of STOERUNGEN) {
  test(`B2: ${name} — nie „✓ Wieder verbunden“, solange g1–g3 fehlen, danach je einmal`, async ({ browser }) => {
    instanz = await sendenStarten({ anzahl: 5 });
    const b = await kontextOeffnen(browser, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [500] } });
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    await expect(nachrichten(b.page)).toHaveCount(5);
    const token = await beitreten(instanz.adresse, 'beta');
    await netzAus(b);
    for (const text of ['g1', 'g2', 'g3']) {
      expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
    }
    const abrufe = [];
    await b.page.route(NACHHOLEN, async (route) => {
      const stoerung = scheitern[abrufe.length];
      abrufe.push(stoerung);
      if (stoerung === 'abort') {
        await route.abort();
        return;
      }
      if (stoerung === 500) {
        await route.fulfill({ status: 500, body: '' });
        return;
      }
      await route.continue();
    });
    await meldungOhneLueckeBewachen(b.page, ['g1', 'g2', 'g3']);

    await netzAn(b);

    await expect(leiste(b.page)).toHaveText('✓ Wieder verbunden · 3 nachgeladen', { timeout: 20000 });
    await expect(bestaetigt(b.page, 'g[123]').locator('.lc-body')).toHaveText(['g1', 'g2', 'g3']);
    await expect(nachrichten(b.page)).toHaveCount(8);
    expect(abrufe).toEqual([...scheitern, undefined]);
    expect(await b.page.evaluate(() => window.lcVerstoesse)).toEqual([]);
    expect(ohneGestellteFehler(b.fehler)).toEqual([]);
  });
}

// „Nachhol-Abruf hängt“ (R00012 F0047, Z13; Code-Review A3): wie „Gescheitertes Nachholen“, aber die Route antwortet auf
// den ersten Abruf nach= nie → 9 s nach seinem Beginn kein zweiter Abruf und kein „✓ Wieder verbunden“; spätestens
// 15 s nach seinem Beginn läuft der zweite (10 s Zeitgrenze + 1 s Rückzug), danach „✓ Wieder verbunden · 3 nachgeladen“
// und g1–g3 je einmal. Die Uhr startet am Beginn des ersten Abrufs (Ereignis request der Seite).
const Z13_FRUEHESTENS_MS = 9000;
const Z13_SPAETESTENS_MS = 15000;

test('Z13: hängt der erste Nachhol-Abruf, läuft der zweite frühestens nach 9 s und spätestens nach 15 s', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await sendenStarten({ anzahl: 5 });
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(5);
  const token = await beitreten(instanz.adresse, 'beta');
  await netzAus(b);
  for (const text of ['g1', 'g2', 'g3']) {
    expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
  }
  const beginne = [];
  b.page.on('request', (anfrage) => {
    if (NACHHOLEN(new URL(anfrage.url()))) {
      beginne.push(Date.now());
    }
  });
  let ersterGehalten = false;
  await b.page.route(NACHHOLEN, async (route) => {
    if (!ersterGehalten) {
      ersterGehalten = true;
      return;
    }
    await route.continue();
  });
  await meldungOhneLueckeBewachen(b.page, ['g1', 'g2', 'g3']);

  await netzAn(b);

  await expect(leiste(b.page)).toHaveText('✓ Wieder verbunden · 3 nachgeladen', { timeout: 30000 });
  const abstand = beginne[1] - beginne[0];
  console.log(`Z13 gemessen: zweiter Abruf ${abstand} ms nach dem ersten (Grenzen ${Z13_FRUEHESTENS_MS}–${Z13_SPAETESTENS_MS})`);
  expect(beginne).toHaveLength(2);
  expect(abstand).toBeGreaterThanOrEqual(Z13_FRUEHESTENS_MS);
  expect(abstand).toBeLessThanOrEqual(Z13_SPAETESTENS_MS);
  await expect(bestaetigt(b.page, 'g[123]').locator('.lc-body')).toHaveText(['g1', 'g2', 'g3']);
  expect(await b.page.evaluate(() => window.lcVerstoesse)).toEqual([]);
  expect(ohneGestellteFehler(b.fehler)).toEqual([]);
});
