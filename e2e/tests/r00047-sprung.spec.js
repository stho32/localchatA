'use strict';
// R00047 I0044 F0159: der Sprung vom Zitat zum Original (ADR-0053 Regel 8, P10, Mutant Z14) — im Fenster (B0717):
// zentriert, 2000 ms hervorgehoben, Fokus; außerhalb (B0718): erst holen, dann ersetzen, auch im Verlauf mit 100.000
// Nachrichten; entfallen ohne Abruf; Fehlschlag mit Wiederholen.
const fs = require('fs');
const { test, expect } = require('./rahmen');
const { instanzStarten, channelAnlegen, grossAnlegen, sichtbarNach } = require('./hilfen');
const { JETZT, sendenStarten, anfragen, beitreten, imBrowserBeitreten } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const { browserOeffnen, nachrichtMitZeit, loeschenPerApi, ansagenMitschreiben } = require('./r00041-hilfen');
const {
  ANNA, CLAUDE, claudeSchreibt, mitBezugSenden, benOeffnen, zitatVon,
} = require('./r00047-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const HERVORHEBUNG_MS = 2000;
const NICHT_GELADEN = '⚠ Original nicht geladen: keine Antwort vom Server. Zum Wiederholen erneut anklicken.';
const ENTFALLEN = 'Die ältere Nachricht gibt es nicht mehr (Aufbewahrungsfrist abgelaufen).';

/// Erste und letzte Zeit des Fensters.
const fensterRand = (page) => page.getByRole('log').locator('.lc-msg[data-zeit]').evaluateAll((alle) => [alle[0].dataset.zeit, alle[alle.length - 1].dataset.zeit]);

/// Steht die Nachricht ganz im Verlauf, hervorgehoben (Grund kd-surface-2, 4-px-Balken kd-accent links, Fokusring) und
/// fokussiert? Läuft im Browser.
function hervorgehobenImBild({ zeit }) {
  const knoten = document.querySelector(`.lc-log .lc-msg[data-zeit="${zeit}"]`);
  if (knoten === null) {
    return false;
  }
  const log = knoten.closest('[role="log"]').getBoundingClientRect();
  const rand = knoten.getBoundingClientRect();
  const farbe = (wert) => {
    const probe = document.createElement('div');
    probe.style.color = wert;
    document.body.append(probe);
    const berechnet = getComputedStyle(probe).color;
    probe.remove();
    return berechnet;
  };
  const stil = getComputedStyle(knoten);
  const ganz = rand.top >= log.top - 0.5 && rand.bottom <= log.bottom + 0.5;
  return ganz && document.activeElement === knoten && stil.backgroundColor === farbe('var(--kd-surface-2)')
    && stil.boxShadow.includes(farbe('var(--kd-accent)')) && stil.boxShadow.includes('4px') && stil.outlineStyle === 'solid';
}

/// Schreibt mit, wann die Nachricht `zeit` die Hervorhebung bekommt und verliert (performance.now()).
function hervorhebungMitschreiben(page, zeit) {
  return page.evaluate((gesucht) => {
    window.hervorhebung = [];
    new MutationObserver(() => {
      const knoten = document.querySelector(`.lc-log .lc-msg[data-zeit="${gesucht}"]`);
      const an = knoten !== null && knoten.classList.contains('lc-hervorgehoben');
      const letzter = window.hervorhebung[window.hervorhebung.length - 1];
      if (letzter === undefined || letzter[0] !== an) {
        window.hervorhebung.push([an, performance.now()]);
      }
    }).observe(document.querySelector('.lc-log'), { attributes: true, attributeFilter: ['class'], subtree: true, childList: true });
  }, zeit);
}

/// `anna` hat 60 Nachrichten im Log (eine je Sekunde ab 14:00), `claude` antwortet auf die 25.: das Original liegt im
/// Fenster, aber außerhalb des Bildes, mit genug Inhalt darüber, um zentriert zu stehen.
async function antwortImFenster() {
  const zeiten = Array.from({ length: 60 }, (_, nummer) => `2026-10-28T14:${String(Math.floor(nummer / 60)).padStart(2, '0')}:${String(nummer % 60).padStart(2, '0')}.000+01:00`);
  instanz = await sendenStarten({
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein', {
      '2026-10-28': zeiten.map((zeit, nummer) => [zeit, 'anna@127.0.0.2', `Nachricht ${nummer + 1}`]),
    }),
  });
  const anna = { token: await beitreten(instanz.adresse, 'anna', { herkunft: ANNA, art: 'mensch' }), zeiten };
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Darauf', zeiten[24], CLAUDE);
  return { anna, claude, original: zeiten[24], antwort };
}

for (const weg of ['Klick', 'Enter']) {
  test(`Sprung im Fenster per ${weg} (P10, B0717): das Original steht im Bild, fokussiert, 2000 ms hervorgehoben (Grund, Balken, Fokusring), danach normal; keine Ansage`, async ({ browser }) => {
    const { original, antwort } = await antwortImFenster();
    const b = await benOeffnen(browser, instanz.adresse);
    await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
    await expect(nachrichtMitZeit(b.page, original)).not.toBeInViewport();
    const angesagt = await ansagenMitschreiben(b.page);
    await hervorhebungMitschreiben(b.page, original);
    const randVorher = await fensterRand(b.page);

    if (weg === 'Klick') {
      await zitatVon(b.page, antwort).click();
    } else {
      await zitatVon(b.page, antwort).focus();
      await b.page.keyboard.press('Enter');
    }

    await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab: Date.now(), timeout: 1000 });
    const mitte = await nachrichtMitZeit(b.page, original).evaluate((knoten) => {
      const log = knoten.closest('[role="log"]').getBoundingClientRect();
      const rand = knoten.getBoundingClientRect();
      return Math.abs((rand.top + rand.bottom) / 2 - (log.top + log.bottom) / 2);
    });
    expect(mitte, 'zentriert').toBeLessThanOrEqual(30);
    await b.page.waitForFunction(() => window.hervorhebung.length >= 2, null, { timeout: 5000 });
    const wechsel = await b.page.evaluate(() => window.hervorhebung);
    expect(wechsel[0][0]).toBe(true);
    expect(wechsel[1][0]).toBe(false);
    expect(wechsel[1][1] - wechsel[0][1]).toBeGreaterThanOrEqual(HERVORHEBUNG_MS - 300);
    expect(wechsel[1][1] - wechsel[0][1]).toBeLessThanOrEqual(HERVORHEBUNG_MS + 300);
    const stil = await nachrichtMitZeit(b.page, original).evaluate((knoten) => getComputedStyle(knoten).boxShadow);
    expect(stil).toBe('none');
    expect(await angesagt()).toEqual([]);
    expect(await fensterRand(b.page)).toEqual(randVorher);
    expect(b.fehler).toEqual([]);
  });
}

test('Sprung per Antippen auf dem Telefon (P10, B0717): das Original steht im Bild und ist hervorgehoben', async ({ browser }) => {
  const { original, antwort } = await antwortImFenster();
  const b = await geraetOeffnen(browser, 'telefon', { fenster: { width: 375, height: 667 }, streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'ben');
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
  await zitatVon(b.page, antwort).tap();
  await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab: Date.now(), timeout: 1000 });
  expect(b.fehler).toEqual([]);
});

test('Gelöschtes Original (Z4, B0717): das Zitat bleibt ein Knopf und springt zur Marke „Nachricht gelöscht“', async ({ browser }) => {
  const { anna, original, antwort } = await antwortImFenster();
  expect((await loeschenPerApi(instanz.adresse, anna.token, original, ANNA)).status).toBe(200);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
  await zitatVon(b.page, antwort).click();
  await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab: Date.now(), timeout: 1000 });
  await expect(nachrichtMitZeit(b.page, original).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect(b.fehler).toEqual([]);
});

// Außerhalb des Fensters (B0718, ADR-0053 Regel 8 b und c).

/// `allgemein` mit 300 erzeugten Nachrichten; `claude` antwortet auf die älteste — sie liegt vor dem Fenster der neuesten 100.
async function antwortAufDieAelteste() {
  instanz = await sendenStarten({ anzahl: 300 });
  const claude = await claudeSchreibt(instanz.adresse, []);
  const aelteste = await anfragen(instanz.adresse, { methode: 'GET', pfad: 'api/channels/allgemein/nachrichten?nach=2000-01-01T00%3A00%3A00.000%2B00%3A00&n=1', token: claude.token, herkunft: CLAUDE });
  const original = aelteste.daten.nachrichten[0].zeit;
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Darauf', original, CLAUDE);
  return { original, antwort };
}

/// Sammelt ab jetzt die Long Tasks der Seite (Dauer in ms).
const longTasksMitschreiben = (page) => page.evaluate(() => {
  window.longTasks = [];
  new PerformanceObserver((liste) => liste.getEntries().forEach((eintrag) => window.longTasks.push(eintrag.duration))).observe({ type: 'longtask' });
});

test('Sprung im Verlauf mit 100.000 Nachrichten (P10, B0718): die älteste ist binnen 1000 ms sichtbar und hervorgehoben, kein Long Task über 200 ms', async ({ browser }) => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_JETZT: JETZT }, vorbereiten: (ordner) => grossAnlegen(ordner) });
  const token = await beitreten(instanz.adresse, 'claude', { herkunft: CLAUDE });
  const aelteste = await anfragen(instanz.adresse, { methode: 'GET', pfad: 'api/channels/gross/nachrichten?nach=2000-01-01T00%3A00%3A00.000%2B00%3A00&n=1', token, herkunft: CLAUDE });
  const original = aelteste.daten.nachrichten[0].zeit;
  const gesendet = await anfragen(instanz.adresse, { methode: 'POST', pfad: 'api/channels/gross/nachrichten', koerper: { text: 'Auf die Älteste', antwort_auf: original }, token, herkunft: CLAUDE });
  expect(gesendet.status, JSON.stringify(gesendet.daten)).toBe(201);
  const antwort = gesendet.daten.nachricht.zeit;
  const b = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'ben', 'gross');
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1, { timeout: 10000 });
  await longTasksMitschreiben(b.page);

  const ab = Date.now();
  await zitatVon(b.page, antwort).click();
  const ms = await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab, timeout: 10000 });

  await b.page.waitForTimeout(1000);
  const laengste = await b.page.evaluate(() => Math.max(0, ...window.longTasks));
  console.log(`Messwert Sprung 100.000: sichtbar nach ${ms} ms, längster Long Task ${laengste} ms`);
  expect(ms, 'Original sichtbar nach dem Klick (ms)').toBeLessThanOrEqual(1000);
  expect(laengste, 'längster Long Task (ms)').toBeLessThanOrEqual(200);
  expect(b.fehler).toEqual([]);
});

test('Antwortet die Route für nach= 2 s verzögert, bleibt der alte Inhalt bis zur Antwort stehen (P10, Mutant Z14, B0718)', async ({ browser }) => {
  const { original, antwort } = await antwortAufDieAelteste();
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
  await b.page.route(/\/nachrichten\?nach=/, async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 2000));
    await route.continue();
  });
  const vorher = await fensterRand(b.page);

  const geklickt = Date.now();
  await zitatVon(b.page, antwort).click();
  for (const nach of [200, 1000, 1700]) {
    await b.page.waitForTimeout(Math.max(0, geklickt + nach - Date.now()));
    expect(await fensterRand(b.page), `${nach} ms`).toEqual(vorher);
    await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
  }
  await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab: Date.now(), timeout: 5000 });
  expect(b.fehler).toEqual([]);
});

test('Scheitert der Abruf, steht der Hinweis am Zitat, das Fenster bleibt; der nächste Klick springt (Z4, US-28, B0718)', async ({ browser }) => {
  const { original, antwort } = await antwortAufDieAelteste();
  const b = await benOeffnen(browser, instanz.adresse, { serverWegErwartet: true });
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
  await b.page.route(/\/nachrichten\?nach=/, (route) => route.abort('connectionrefused'));
  const vorher = await fensterRand(b.page);

  await zitatVon(b.page, antwort).click();
  await expect(nachrichtMitZeit(b.page, antwort).locator('.lc-zitat-hinweis')).toHaveText(NICHT_GELADEN);
  expect(await fensterRand(b.page)).toEqual(vorher);

  await b.page.unroute(/\/nachrichten\?nach=/);
  await zitatVon(b.page, antwort).click();
  await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab: Date.now(), timeout: 5000 });
  expect(b.fehler.filter((fehler) => !fehler.includes('nach='))).toEqual([]);
});

test('Entfallenes Original (Z4, B0718): kein Abruf, Hinweis am Zitat und in der Region, der Verlauf bleibt, wo er ist', async ({ browser }) => {
  const entfallenAuf = '2026-10-20T10:00:00.000+01:00';
  const antwort = '2026-10-28T14:05:00.000+01:00';
  instanz = await sendenStarten({
    vorbereiten: (ordner) => {
      fs.writeFileSync(`${ordner}/channels/allgemein/2026-10-28.log`,
        `@ ${antwort} anna@127.0.0.2\n+ art: mensch\n+ antwort_auf: ${entfallenAuf} claude@127.0.0.3\n  Weiß ich nicht\n\n`);
    },
  });
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
  const abrufe = [];
  b.page.on('request', (anfrage) => {
    if (anfrage.url().includes('nach=')) {
      abrufe.push(anfrage.url());
    }
  });
  const angesagt = await ansagenMitschreiben(b.page);
  const vorher = await fensterRand(b.page);
  const scrollTop = () => b.page.getByRole('log').evaluate((log) => log.scrollTop);
  const stand = await scrollTop();

  await zitatVon(b.page, antwort).click();

  await expect(nachrichtMitZeit(b.page, antwort).locator('.lc-zitat-hinweis')).toHaveText(ENTFALLEN);
  await expect.poll(angesagt).toEqual([ENTFALLEN]);
  expect(abrufe).toEqual([]);
  expect(await fensterRand(b.page)).toEqual(vorher);
  expect(await scrollTop()).toBe(stand);
  expect(b.fehler).toEqual([]);
});
