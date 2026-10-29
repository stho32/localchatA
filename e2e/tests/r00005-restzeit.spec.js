'use strict';
// R00005 F0018 Restzeit in der Teilnehmerliste (US-10 bis US-14) und F0019 „die Oberfläche ruft verlassen nie“:
// gegen das echte Programm, Chromium, je Browser ein Kontext. Dauern ereignisgenau (sichtbarNach, Animations-
// rahmen) ab dem Ereignis des Kriteriums; die Regel selbst prüfen die Serverzeiten (tests/restzeit.rs).
const { test, expect } = require('./rahmen');
const { instanzStarten, freierPort, fehlerSammeln, beitretenPerApi, sichtbarNach, sichtbarMitWertNach } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const region = (page) => page.getByRole('complementary', { name: 'Teilnehmer' });
const zeileVon = (page, kennung) => region(page).locator('.lc-person').filter({ has: page.locator('.lc-id', { hasText: kennung }) });

async function beitreten(page, name) {
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
}

// Ein Browser je Kontext; jeder Abruf `POST /api/verlassen` aus einer Seite wird notiert (Rückfall und Seite —
// Abrufe aus dem SharedWorker sieht Playwright nicht, dafür prüft e2e/einheit/verlassen.test.mjs den Code).
async function browserMit(browser, { vorbereiten = async () => {} } = {}) {
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const verlassen = [];
  kontext.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && new URL(anfrage.url()).pathname === '/api/verlassen') {
      verlassen.push(anfrage.url());
    }
  });
  await vorbereiten(kontext);
  const page = await kontext.newPage();
  return { kontext, page, verlassen, fehler: fehlerSammeln(page) };
}

// Prüfungen für sichtbarNach — sie laufen im Browser und kennen nur ihr Argument und das DOM.
function gleichWegText({ kennung }) {
  const zeile = [...document.querySelectorAll('#lc-teilnehmer .lc-person')]
    .find((person) => person.querySelector('.lc-id').textContent.includes(kennung));
  if (zeile === undefined || zeile.querySelector('.lc-dot.gleich') === null) {
    return false;
  }
  return zeile.querySelector('.lc-status').textContent;
}

function kleinererWert({ kennung, sekunden }) {
  const zeile = [...document.querySelectorAll('#lc-teilnehmer .lc-person')]
    .find((person) => person.querySelector('.lc-id').textContent.includes(kennung));
  if (zeile === undefined) {
    return false;
  }
  const treffer = /^weg in (\d+):(\d\d)$/.exec(zeile.querySelector('.lc-status').textContent);
  if (treffer === null) {
    return false;
  }
  const jetzt = Number(treffer[1]) * 60 + Number(treffer[2]);
  return jetzt < sekunden && jetzt;
}

function zeileFehlt(kennung) {
  return ![...document.querySelectorAll('#lc-teilnehmer .lc-person .lc-id')].some((id) => id.textContent.includes(kennung));
}

function sekundenIn(text) {
  const treffer = /^weg in (\d+):(\d\d)$/.exec(text);
  expect(treffer, text).not.toBeNull();
  return Number(treffer[1]) * 60 + Number(treffer[2]);
}

// Höhe der Zeile, Breite der Statusspalte (letzte Spur des Rasters) und die Lage aller Zeilen und Überschriften.
async function lage(page, kennung) {
  return page.evaluate((gesucht) => {
    const zeilen = [...document.querySelectorAll('#lc-teilnehmer .lc-person')];
    const zeile = zeilen.find((person) => person.querySelector('.lc-id').textContent.includes(gesucht));
    const spuren = getComputedStyle(zeile).gridTemplateColumns.split(' ');
    const ueberschriften = [...document.querySelectorAll('#lc-teilnehmer .lc-sect')].map((kopf) => kopf.getBoundingClientRect().top);
    return {
      hoehe: zeile.getBoundingClientRect().height,
      statusspalte: spuren[spuren.length - 1],
      oben: [...ueberschriften, ...zeilen.map((person) => person.getBoundingClientRect().top)],
    };
  }, kennung);
}

test('US-10: wer geht, zählt sichtbar herunter — gestrichelt „weg in 0:0s“, dann kleiner, dann weg; nichts springt', async ({ browser }) => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_GNADENFRIST_MS: '5000' } });
  const a = await browserMit(browser);
  const b = await browserMit(browser);
  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'mia');
  await expect(zeileVon(a.page, 'mia@127.0.0.1').locator('.lc-dot')).toHaveClass(/\bon\b/);
  const online = await lage(a.page, 'mia@127.0.0.1');

  await b.kontext.close();
  const geschlossen = Date.now();
  const erschienen = await sichtbarMitWertNach(a.page, gleichWegText, { argument: { kennung: 'mia@127.0.0.1' }, ab: geschlossen, timeout: 10000 });
  const erscheinenAb = geschlossen + erschienen.ms;
  const gleichWeg = await lage(a.page, 'mia@127.0.0.1');
  const ersterWert = sekundenIn(erschienen.wert);
  const kleiner = await sichtbarMitWertNach(a.page, kleinererWert, { argument: { kennung: 'mia@127.0.0.1', sekunden: ersterWert }, ab: erscheinenAb, timeout: 10000 });
  const mia = zeileVon(a.page, 'mia@127.0.0.1');
  const zugaenglich = await mia.getByRole('img', { name: 'offline, gleich weg' }).count();
  const ansage = await region(a.page).evaluate((liste) => liste.closest('[aria-live]') !== null || liste.querySelector('[aria-live]') !== null);
  const wegNach = await sichtbarNach(a.page, zeileFehlt, { argument: 'mia@127.0.0.1', ab: erscheinenAb, timeout: 10000 });
  test.info().annotations.push({ type: 'messung', description: `erscheinen ${erschienen.ms} ms (${erschienen.wert}), kleiner ${kleiner.ms} ms, weg ${wegNach} ms` });

  expect(erschienen.ms, 'gestrichelt spätestens 2 s nach dem Schließen').toBeLessThanOrEqual(2000);
  expect(ersterWert).toBeGreaterThanOrEqual(1);
  expect(ersterWert).toBeLessThanOrEqual(4);
  expect(kleiner.ms, 'kleinerer Wert spätestens 3 s nach dem ersten').toBeLessThanOrEqual(3000);
  expect(wegNach, 'Zeile weg spätestens 5 s nach dem Erscheinen').toBeLessThanOrEqual(5000);
  expect(gleichWeg).toEqual(online);
  expect(zugaenglich).toBe(1);
  expect(ansage).toBe(false);
  expect(a.verlassen).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('US-11: ein neuer Tab zeigt dieselbe Restzeit — binnen 2 s, höchstens 1 Sekunde Abweichung', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await instanzStarten();
  const a = await browserMit(browser);
  const b = await browserMit(browser);
  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'mia');
  await expect(zeileVon(a.page, 'mia@127.0.0.1').locator('.lc-dot')).toHaveClass(/\bon\b/);

  await b.kontext.close();
  const geschlossen = Date.now();
  await sichtbarNach(a.page, gleichWegText, { argument: { kennung: 'mia@127.0.0.1' }, ab: geschlossen, timeout: 10000 });
  await a.page.waitForTimeout(20000 - (Date.now() - geschlossen));
  const oeffnen = Date.now();
  const zweiter = await a.kontext.newPage();
  const fehlerZwei = fehlerSammeln(zweiter);
  await zweiter.goto(instanz.adresse);
  const zweiterZeigt = await sichtbarNach(zweiter, gleichWegText, { argument: { kennung: 'mia@127.0.0.1' }, ab: oeffnen, timeout: 10000 });
  const [eins, zwei] = await Promise.all([a.page, zweiter].map((tab) => tab.evaluate(gleichWegText, { kennung: 'mia@127.0.0.1' })));
  await zweiter.reload();
  await expect(zeileVon(zweiter, 'mia@127.0.0.1').locator('.lc-status')).toHaveText(/^weg in 0:\d\d$/);
  test.info().annotations.push({ type: 'messung', description: `Tab 2 nach ${zweiterZeigt} ms, ${eins} / ${zwei}` });

  expect(zweiterZeigt, 'Tab 2 zeigt binnen 2 s').toBeLessThanOrEqual(2000);
  expect(Math.abs(sekundenIn(eins) - sekundenIn(zwei))).toBeLessThanOrEqual(1);
  expect(a.verlassen).toEqual([]);
  expect(a.fehler).toEqual([]);
  expect(fehlerZwei).toEqual([]);
});

// Die Uhr des Rechners geht 30 s vor: Seite und Worker teilen sie — beide werden verschoben (der Worker über
// sein Skript, das Playwright im Kontext umleiten kann).
async function uhrGehtVor(kontext, millisekunden) {
  await kontext.addInitScript((vor) => {
    const echt = Date.now.bind(Date);
    Date.now = () => echt() + vor;
  }, millisekunden);
  await kontext.route('**/web/strom.js*', async (route) => {
    const antwort = await route.fetch();
    const vorspann = `const echteUhr = Date.now.bind(Date); Date.now = () => echteUhr() + ${millisekunden};\n`;
    await route.fulfill({ response: antwort, body: vorspann + (await antwort.text()) });
  });
}

test('US-12: eine falsch gehende Uhr im Browser stört nicht — „weg in 0:55“ bis „0:59“, nicht 0:29', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await browserMit(browser, { vorbereiten: (kontext) => uhrGehtVor(kontext, 30000) });
  const b = await browserMit(browser);
  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'mia');
  await expect(zeileVon(a.page, 'mia@127.0.0.1').locator('.lc-dot')).toHaveClass(/\bon\b/);
  const vorgestellt = await a.page.evaluate(() => Date.now() - new Date().getTime());

  await b.kontext.close();
  const geschlossen = Date.now();
  const erschienen = await sichtbarMitWertNach(a.page, gleichWegText, { argument: { kennung: 'mia@127.0.0.1' }, ab: geschlossen, timeout: 10000 });
  test.info().annotations.push({ type: 'messung', description: `beim Erscheinen ${erschienen.wert} nach ${erschienen.ms} ms` });

  expect(vorgestellt).toBeGreaterThanOrEqual(29000);
  expect(sekundenIn(erschienen.wert)).toBeGreaterThanOrEqual(55);
  expect(sekundenIn(erschienen.wert)).toBeLessThanOrEqual(59);
  expect(a.fehler).toEqual([]);
});

test('Rückfall: ohne SharedWorker zeigt A die Zeile gestrichelt mit „weg in …“', async ({ browser }) => {
  instanz = await instanzStarten();
  const a = await browserMit(browser, { vorbereiten: (kontext) => kontext.addInitScript(() => { delete window.SharedWorker; }) });
  const b = await browserMit(browser);
  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'mia');
  await expect(zeileVon(a.page, 'mia@127.0.0.1').locator('.lc-dot')).toHaveClass(/\bon\b/);

  await b.kontext.close();
  const mia = zeileVon(a.page, 'mia@127.0.0.1');

  await expect(mia.locator('.lc-dot.gleich')).toHaveCount(1, { timeout: 10000 });
  await expect(mia.locator('.lc-status')).toHaveText(/^weg in \d:\d\d$/);
  expect(a.verlassen).toEqual([]);
});

test('US-13: „zuletzt online“ ist ein grau gefüllter Punkt, kein gestrichelter Kreis', async ({ browser }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await kontext.newPage();
  const fehler = fehlerSammeln(page, { serverWegErwartet: true });
  const b = await browserMit(browser);
  await beitreten(page, 'shoff');
  await beitreten(b.page, 'mia');
  await expect(zeileVon(page, 'mia@127.0.0.1').locator('.lc-dot')).toHaveClass(/\bon\b/);

  await instanz.beenden();

  // Seit R00025 F0084: die Leiste „⚠ Verbindung verloren“ statt des Hinweises „Verbindung unterbrochen“.
  await expect(page.locator('.lc-center > .lc-leiste')).toHaveText(/^⚠ Verbindung verloren/);
  await expect(page.locator('#lc-verbindung')).toHaveCount(0);
  const punkte = await region(page).locator('.lc-person').evaluateAll((zeilen) => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--lc-offline)';
    document.body.append(probe);
    const token = getComputedStyle(probe).color;
    probe.remove();
    return zeilen.map((zeile) => {
      const punkt = zeile.querySelector('.lc-dot');
      const stil = getComputedStyle(punkt);
      return {
        kennung: zeile.querySelector('.lc-id').textContent,
        name: punkt.getAttribute('aria-label'),
        gefuelltMitToken: stil.backgroundColor === token,
        rand: stil.borderTopStyle,
      };
    });
  });

  expect(punkte).toEqual([
    { kennung: 'mia@127.0.0.1', name: 'zuletzt online', gefuelltMitToken: true, rand: 'none' },
    { kennung: 'shoff@127.0.0.1', name: 'zuletzt online', gefuelltMitToken: true, rand: 'none' },
  ]);
  expect(fehler).toEqual([]);
});

test('US-14: auch bei 320 px passt „weg in 0:ss“ mit einem Namen von 32 Zeichen — kein Querscrollen', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.setViewportSize({ width: 320, height: 640 });
  await beitreten(page, 'shoff');
  const langerName = 'agent-mit-zweiunddreissig-zeichen'.slice(0, 32);
  expect((await beitretenPerApi(instanz.adresse, langerName)).status).toBe(201);

  await page.keyboard.press('Control+.');

  await expect(region(page)).toBeVisible();
  await expect(zeileVon(page, `${langerName}@127.0.0.1`).locator('.lc-status')).toHaveText(/^weg in 0:\d\d$/, { timeout: 5000 });
  const breiten = await page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));
  expect(breiten.inhalt).toBeLessThanOrEqual(breiten.fenster);
  expect(fehler).toEqual([]);
});

// ADR-0019 Regel 7: die feste Statusspalte fasst den längsten Statustext „weg in 1:00“ samt Punkt, in hell und
// dunkel gleich — gezeichnet im echten Browser mit den Stilen des Programms.
test('Statusspalte: „weg in 1:00“ samt Punkt passt in die feste Spalte, ohne Überlauf', async ({ page }) => {
  instanz = await instanzStarten();
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').waitFor();

  const masse = await page.evaluate(async () => {
    const { zeichnen } = await import('/web/teilnehmerliste.js');
    const liste = document.createElement('aside');
    liste.className = 'lc lc-members';
    document.body.replaceChildren(liste);
    const jetzt = Date.now();
    const zeile = { kennung: 'mia@127.0.0.1', name: 'mia', herkunft: '127.0.0.1', art: 'mensch', status: 'offline', seit: '', weg_um: new Date(jetzt + 60000).toISOString(), stand: 1, eigene: false };
    zeichnen(liste, { menschen: [zeile], agenten: [] }, { versatz: 0, jetzt });
    const status = liste.querySelector('.lc-person-status');
    const spuren = getComputedStyle(liste.querySelector('.lc-person')).gridTemplateColumns.split(' ');
    return { text: status.textContent, inhalt: status.scrollWidth, spalte: parseFloat(spuren[spuren.length - 1]) };
  });

  expect(masse.text).toBe('weg in 1:00');
  expect(masse.inhalt).toBeLessThanOrEqual(masse.spalte);
});
