'use strict';
// R00041 F0134 Bearbeitete und F0138 Gelöschte Nachrichten im Browser sehen: live über das Thema `aenderung` (B0606,
// B0621), nach dem Neuladen und nach dem Nachholen (B0607), Lesestelle, Reihenfolge, außerhalb des Fensters.
const { test, expect } = require('./rahmen');
const { sichtbarNach } = require('./hilfen');
const { netzAus, netzAn, leiste } = require('./r00012-hilfen');
const { sendenStarten, imBrowserBeitreten, nachrichten } = require('./senden-hilfen');
const {
  betaSendet, betaVerlauf, bearbeitenPerApi, loeschenPerApi, browserOeffnen, nachrichtMitZeit, zeigtBearbeitet,
  zeigtGeloescht, textknotenMit, mitAnhaengenSenden, PNG, webm,
} = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const GRENZE_MS = 1000;

for (const { titel, ohneWorker } of [{ titel: 'mit SharedWorker', ohneWorker: false }, { titel: 'ohne SharedWorker', ohneWorker: true }]) {
  test(`Live bearbeitet (B0606, ${titel}): A und B zeigen den neuen Text mit „bearbeitet“ unter 1 s, ohne Neuladen; nach dem Neuladen ebenso`, async ({ browser }) => {
    instanz = await sendenStarten();
    const beta = await betaSendet(instanz.adresse, ['Hallo']);
    const zeit = beta.zeiten[0];
    const a = await browserOeffnen(browser, { ohneWorker });
    const b = await browserOeffnen(browser, { ohneWorker });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    await expect(nachrichtMitZeit(a.page, zeit)).toHaveCount(1);
    await expect(nachrichtMitZeit(b.page, zeit)).toHaveCount(1);
    await a.page.evaluate(() => { window.nichtNeuGeladen = true; });

    const antwort = await bearbeitenPerApi(instanz.adresse, beta.token, zeit, 'Hallo zusammen');
    expect(antwort.status, JSON.stringify(antwort.daten)).toBe(200);
    const bisA = await sichtbarNach(a.page, zeigtBearbeitet, { argument: { zeit, text: 'Hallo zusammen' }, ab: antwort.empfangen, timeout: 10000 });
    const bisB = await sichtbarNach(b.page, zeigtBearbeitet, { argument: { zeit, text: 'Hallo zusammen' }, ab: antwort.empfangen, timeout: 10000 });
    expect(bisA).toBeLessThan(GRENZE_MS);
    expect(bisB).toBeLessThan(GRENZE_MS);
    expect(await textknotenMit(a.page, 'Hallo')).toEqual([]);
    expect(await textknotenMit(b.page, 'Hallo')).toEqual([]);
    expect(await a.page.evaluate(() => window.nichtNeuGeladen)).toBe(true);
    await expect(nachrichten(a.page)).toHaveCount(1);

    await a.page.reload();
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo zusammen');
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-marke', { hasText: 'bearbeitet' })).toHaveCount(1);
    expect(a.fehler).toEqual([]);
    expect(b.fehler).toEqual([]);
  });
}

test('Lesestelle (B0606): wächst eine Nachricht über dem sichtbaren Bereich auf fünf Zeilen, steht die oberste sichtbare höchstens 1 px verschoben', async ({ browser }) => {
  const verlauf = betaVerlauf(60);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const token = (await betaSendet(instanz.adresse, [])).token;
  const b = await browserOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(60);
  const ziel = verlauf.zeiten[9];
  await nachrichtMitZeit(b.page, verlauf.zeiten[20]).evaluate((knoten) => {
    const log = knoten.closest('[role="log"]');
    log.scrollTop += knoten.getBoundingClientRect().top - log.getBoundingClientRect().top;
  });
  const vorher = await obersteSichtbare(b.page);
  expect(vorher.zeit).toBe(verlauf.zeiten[20]);

  const antwort = await bearbeitenPerApi(instanz.adresse, token, ziel, 'Eins\nZwei\nDrei\nVier\nFünf');
  expect(antwort.status, JSON.stringify(antwort.daten)).toBe(200);
  await sichtbarNach(b.page, zeigtBearbeitet, { argument: { zeit: ziel, text: 'Eins\nZwei\nDrei\nVier\nFünf' }, ab: antwort.empfangen, timeout: 10000 });
  const nachher = await b.page.evaluate((zeit) => {
    const knoten = document.querySelector(`.lc-msg[data-zeit="${zeit}"]`);
    return knoten.getBoundingClientRect().top - document.querySelector('[role="log"]').getBoundingClientRect().top;
  }, vorher.zeit);
  expect(Math.abs(nachher - vorher.oben)).toBeLessThanOrEqual(1);
  expect(b.fehler).toEqual([]);
});

/// Die oberste Nachricht, die im Verlauf (ganz oder teilweise) zu sehen ist: Zeit und Abstand zum oberen Rand.
const obersteSichtbare = (page) => page.evaluate(() => {
  const log = document.querySelector('[role="log"]').getBoundingClientRect();
  const knoten = [...document.querySelectorAll('.lc-log .lc-msg')].find((einzeln) => einzeln.getBoundingClientRect().bottom > log.top);
  return { zeit: knoten.dataset.zeit, oben: knoten.getBoundingClientRect().top - log.top };
});

test('Reihenfolge (B0606): zwei Bearbeitungen kurz nacheinander, die erste um 300 ms verzögert ausgeliefert → am Ende steht die zweite', async ({ browser }) => {
  instanz = await sendenStarten();
  const beta = await betaSendet(instanz.adresse, ['Hallo']);
  const zeit = beta.zeiten[0];
  const a = await browserOeffnen(browser, { ohneWorker: true });
  await stromMitVerzoegerung(a.page, '"text":"Eins"', 300);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichtMitZeit(a.page, zeit)).toHaveCount(1);

  expect((await bearbeitenPerApi(instanz.adresse, beta.token, zeit, 'Eins')).status).toBe(200);
  const zwei = await bearbeitenPerApi(instanz.adresse, beta.token, zeit, 'Zwei');
  expect(zwei.status).toBe(200);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Zwei');
  await a.page.waitForFunction(() => window.verzoegertAusgeliefert === true, null, { timeout: 5000 });
  await a.page.waitForTimeout(100);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Zwei');
  expect(a.fehler).toEqual([]);
});

/// Liefert im Ereignisstrom des Tabs (ohne SharedWorker) jedes Ereignis, dessen Text `merkmal` enthält, um `ms` verzögert
/// aus — spätere Ereignisse überholen es. Ein Init-Skript statt page.route: page.route kann keinen offenen Strom
/// ausliefern, nur eine fertige Antwort.
async function stromMitVerzoegerung(page, merkmal, ms) {
  await page.addInitScript(({ merkmal: gesucht, ms: verzoegerung }) => {
    const echtesFetch = window.fetch.bind(window);
    window.fetch = async (pfad, optionen) => {
      const antwort = await echtesFetch(pfad, optionen);
      if (!String(pfad).endsWith('/api/ereignisse') || antwort.body === null) {
        return antwort;
      }
      const leser = antwort.body.pipeThrough(new TextDecoderStream()).getReader();
      const kodierer = new TextEncoder();
      const strom = new ReadableStream({
        async start(steuerung) {
          let rest = '';
          for (;;) {
            const { value, done } = await leser.read().catch(() => ({ done: true }));
            if (done) {
              steuerung.close();
              return;
            }
            rest += value;
            const bloecke = rest.split('\n\n');
            rest = bloecke.pop();
            for (const block of bloecke) {
              const bytes = kodierer.encode(`${block}\n\n`);
              if (block.includes(gesucht)) {
                setTimeout(() => { steuerung.enqueue(bytes); window.verzoegertAusgeliefert = true; }, verzoegerung);
              } else {
                steuerung.enqueue(bytes);
              }
            }
          }
        },
      });
      return new Response(strom, { status: antwort.status, headers: antwort.headers });
    };
  }, { merkmal, ms });
}

test('Vor dem Block (B0606): eine Änderung, die vor dem ersten Block ankommt, gilt nach dem Block, der noch die alte Fassung bringt', async ({ browser }) => {
  instanz = await sendenStarten();
  const beta = await betaSendet(instanz.adresse, ['Hallo']);
  const zeit = beta.zeiten[0];
  const a = await browserOeffnen(browser, { ohneWorker: true });
  let freigeben;
  const tor = new Promise((offen) => { freigeben = offen; });
  let blockGeholt;
  const geholt = new Promise((fertig) => { blockGeholt = fertig; });
  await a.page.route((url) => url.pathname === '/api/channels/allgemein/nachrichten' && !url.searchParams.has('vor') && !url.searchParams.has('nach'),
    async (route) => {
      const alteFassung = await route.fetch();
      blockGeholt();
      await tor;
      await route.fulfill({ response: alteFassung });
    }, { times: 1 });
  await a.page.goto(`${instanz.adresse}#/c/allgemein`);
  await a.page.getByLabel('Name').fill('shoff');
  await a.page.getByLabel('Name').press('Enter');
  await geholt;
  await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  expect((await bearbeitenPerApi(instanz.adresse, beta.token, zeit, 'Hallo zusammen')).status).toBe(200);
  await a.page.waitForTimeout(300);

  freigeben();
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo zusammen');
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-marke', { hasText: 'bearbeitet' })).toHaveCount(1);
  expect(a.fehler).toEqual([]);
});

const ohneNetzfehler = (fehler) => fehler.filter((zeile) => !/ERR_INTERNET_DISCONNECTED/.test(zeile));

test('Nachholen bearbeitet (B0607): B offline, beta bearbeitet, B online → nach „Wieder verbunden“ zeigt B den neuen Text mit „bearbeitet“, ohne Neuladen', async ({ browser }) => {
  instanz = await sendenStarten();
  const beta = await betaSendet(instanz.adresse, ['Hallo']);
  const zeit = beta.zeiten[0];
  const b = await browserOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichtMitZeit(b.page, zeit)).toHaveCount(1);
  await b.page.evaluate(() => { window.nichtNeuGeladen = true; });

  await netzAus(b);
  expect((await bearbeitenPerApi(instanz.adresse, beta.token, zeit, 'Hallo zusammen')).status).toBe(200);
  await netzAn(b);
  await expect(leiste(b.page)).toHaveText(/Wieder verbunden/);
  await expect(nachrichtMitZeit(b.page, zeit).locator('.lc-body')).toHaveText('Hallo zusammen');
  await expect(nachrichtMitZeit(b.page, zeit).locator('.lc-marke', { hasText: 'bearbeitet' })).toHaveCount(1);
  expect(await b.page.evaluate(() => window.nichtNeuGeladen)).toBe(true);
  expect(ohneNetzfehler(b.fehler)).toEqual([]);
});

test('Außerhalb des Fensters (B0604): 600 Nachrichten, B am Ende; beta bearbeitet die erste → kein Fehler; am Anfang zeigt sie den neuen Text', async ({ browser }) => {
  const verlauf = betaVerlauf(600);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const token = (await betaSendet(instanz.adresse, [])).token;
  const b = await browserOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(100);
  const erste = verlauf.zeiten[0];
  await expect(nachrichtMitZeit(b.page, erste)).toHaveCount(0);

  expect((await bearbeitenPerApi(instanz.adresse, token, erste, 'Ganz am Anfang')).status).toBe(200);
  await b.page.waitForTimeout(300);
  await b.page.getByRole('log').focus();
  for (let versuch = 0; versuch < 40 && await nachrichtMitZeit(b.page, erste).count() === 0; versuch += 1) {
    await b.page.getByRole('log').evaluate((log) => { log.scrollTop = 0; });
    await b.page.waitForTimeout(150);
  }
  await expect(nachrichtMitZeit(b.page, erste).locator('.lc-body')).toHaveText('Ganz am Anfang');
  await expect(nachrichtMitZeit(b.page, erste).locator('.lc-marke', { hasText: 'bearbeitet' })).toHaveCount(1);
  expect(b.fehler).toEqual([]);
});

for (const { titel, ohneWorker } of [{ titel: 'mit SharedWorker', ohneWorker: false }, { titel: 'ohne SharedWorker', ohneWorker: true }]) {
  test(`Live gelöscht (B0621, ${titel}): A und B zeigen „Nachricht gelöscht“ unter 1 s, ohne Text; nach dem Neuladen ebenso`, async ({ browser }) => {
    instanz = await sendenStarten();
    const beta = await betaSendet(instanz.adresse, ['Hallo']);
    const zeit = beta.zeiten[0];
    const a = await browserOeffnen(browser, { ohneWorker });
    const b = await browserOeffnen(browser, { ohneWorker });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    await expect(nachrichtMitZeit(a.page, zeit)).toHaveCount(1);
    await expect(nachrichtMitZeit(b.page, zeit)).toHaveCount(1);

    const antwort = await loeschenPerApi(instanz.adresse, beta.token, zeit);
    expect(antwort.status, JSON.stringify(antwort.daten)).toBe(200);
    const bisA = await sichtbarNach(a.page, zeigtGeloescht, { argument: { zeit }, ab: antwort.empfangen, timeout: 10000 });
    const bisB = await sichtbarNach(b.page, zeigtGeloescht, { argument: { zeit }, ab: antwort.empfangen, timeout: 10000 });
    expect(bisA).toBeLessThan(GRENZE_MS);
    expect(bisB).toBeLessThan(GRENZE_MS);
    expect(await textknotenMit(a.page, 'Hallo')).toEqual([]);
    expect(await textknotenMit(b.page, 'Hallo')).toEqual([]);
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-meta .lc-id')).toContainText('beta');

    await a.page.reload();
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
    expect(await textknotenMit(a.page, 'Hallo')).toEqual([]);
    expect(a.fehler).toEqual([]);
    expect(b.fehler).toEqual([]);
  });
}

test('Gelöscht mit Anhängen (B0621): der Knoten enthält kein Bild, keinen Film, keinen Ton und keinen Verweis auf /anhaenge/; danach fragt A nichts mehr dort an', async ({ browser }) => {
  instanz = await sendenStarten();
  const beta = await betaSendet(instanz.adresse, []);
  const zeit = await mitAnhaengenSenden(instanz.adresse, beta.token, 'Mit Dateien', [{ name: 'foto.png', inhalt: PNG }, { name: 'film.webm', inhalt: webm() }]);
  const a = await browserOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichtMitZeit(a.page, zeit)).toHaveCount(1);
  const anhangAbrufe = [];

  const antwort = await loeschenPerApi(instanz.adresse, beta.token, zeit);
  expect(antwort.status, JSON.stringify(antwort.daten)).toBe(200);
  a.page.on('request', (anfrage) => {
    if (new URL(anfrage.url()).pathname.startsWith('/anhaenge/')) {
      anhangAbrufe.push(anfrage.url());
    }
  });
  await sichtbarNach(a.page, zeigtGeloescht, { argument: { zeit }, ab: antwort.empfangen, timeout: 10000 });
  const knoten = nachrichtMitZeit(a.page, zeit);
  await expect(knoten.locator('img, video, audio')).toHaveCount(0);
  await expect(knoten.locator('a[href*="/anhaenge/"]')).toHaveCount(0);
  await a.page.waitForTimeout(500);
  expect(anhangAbrufe).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Nachholen gelöscht (B0621): B offline, beta löscht, B online → nach „Wieder verbunden“ „Nachricht gelöscht“, ohne Neuladen', async ({ browser }) => {
  instanz = await sendenStarten();
  const beta = await betaSendet(instanz.adresse, ['Hallo']);
  const zeit = beta.zeiten[0];
  const b = await browserOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichtMitZeit(b.page, zeit)).toHaveCount(1);
  await b.page.evaluate(() => { window.nichtNeuGeladen = true; });

  await netzAus(b);
  expect((await loeschenPerApi(instanz.adresse, beta.token, zeit)).status).toBe(200);
  await netzAn(b);
  await expect(leiste(b.page)).toHaveText(/Wieder verbunden/);
  await expect(nachrichtMitZeit(b.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect(await b.page.evaluate(() => window.nichtNeuGeladen)).toBe(true);
  expect(ohneNetzfehler(b.fehler)).toEqual([]);
});

test('Endgültig (B0621): ein verspätetes aenderung „bearbeitet“ mit kleinerem geaendert nach dem Löschen lässt „Nachricht gelöscht“ stehen', async ({ browser }) => {
  instanz = await sendenStarten();
  const beta = await betaSendet(instanz.adresse, ['Hallo']);
  const zeit = beta.zeiten[0];
  const a = await browserOeffnen(browser, { ohneWorker: true });
  await stromMitVerzoegerung(a.page, '"text":"Verspätet"', 300);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichtMitZeit(a.page, zeit)).toHaveCount(1);

  expect((await bearbeitenPerApi(instanz.adresse, beta.token, zeit, 'Verspätet')).status).toBe(200);
  expect((await loeschenPerApi(instanz.adresse, beta.token, zeit)).status).toBe(200);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await a.page.waitForFunction(() => window.verzoegertAusgeliefert === true, null, { timeout: 5000 });
  await a.page.waitForTimeout(100);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect(await textknotenMit(a.page, 'Verspätet')).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Fokus beim Ersetzen (F0141 nach F0134): eine fokussierte fremde Nachricht, die beta bearbeitet, behält den Fokus auf dem neuen Knoten derselben Zeit', async ({ browser }) => {
  instanz = await sendenStarten();
  const beta = await betaSendet(instanz.adresse, ['Hallo']);
  const zeit = beta.zeiten[0];
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await nachrichtMitZeit(a.page, zeit).focus();
  await a.page.evaluate(() => { window.alterKnoten = document.activeElement; });

  expect((await bearbeitenPerApi(instanz.adresse, beta.token, zeit, 'Hallo zusammen')).status).toBe(200);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo zusammen');
  const lage = await a.page.evaluate(() => ({ zeit: document.activeElement.dataset.zeit, neu: document.activeElement !== window.alterKnoten }));
  expect(lage).toEqual({ zeit, neu: true });
  expect(a.fehler).toEqual([]);
});
