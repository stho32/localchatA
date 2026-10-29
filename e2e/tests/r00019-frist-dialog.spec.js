'use strict';
// R00019 F0083 Frist-Dialog nach dem Entwicklertest #12 (US-10, US-11, US-12, US-15, US-16): der zweite Schritt folgt
// fremden Änderungen, beide Schalter tragen den Satz, am Telefon bleibt der Dialog bedienbar. Heute ist der 28.10.2026.
const fs = require('fs');
const path = require('path');
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln, beitretenPerApi, fristPerApi, channelAnlegen, sichtbarNach } = require('./hilfen');

const JETZT = '2026-10-28T12:00:00.000+01:00';
const ROUTE = '/api/channels/allgemein/frist';
/// Z2 aus R00008: Antwort 200 an `beta` → Anzeige bei A, ereignisgenau (requestAnimationFrame).
const Z2_MS = 2000;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

// `allgemein` mit Frist 30 und den Tagen [tag, anzahl, versatz] → Vorbereitung für instanzStarten.
function fristDreissigMit(tageMitAnzahl) {
  return (ordner) => {
    const satz = (tag, nummer, versatz) => [`${tag}T09:0${nummer}:00.000${versatz}`, 'alpha@127.0.0.2', `${tag} n${nummer}`];
    const tage = {};
    for (const [tag, anzahl, versatz] of tageMitAnzahl) {
      tage[tag] = Array.from({ length: anzahl }, (_, nummer) => satz(tag, nummer, versatz));
    }
    channelAnlegen(ordner, 'allgemein', tage);
    fs.writeFileSync(path.join(ordner, 'channels', 'allgemein', 'einstellungen.toml'), 'frist_tage = 30\n');
  };
}

// 20.(2), 21.(3), 22.(1), 28.(1).
const verkuerzbar = fristDreissigMit([['2026-10-20', 2, '+02:00'], ['2026-10-21', 3, '+02:00'], ['2026-10-22', 1, '+02:00'], ['2026-10-28', 1, '+01:00']]);
// 21.(1), 22.(1), 28.(1).
const einTagVerkuerzbar = fristDreissigMit([['2026-10-21', 1, '+02:00'], ['2026-10-22', 1, '+02:00'], ['2026-10-28', 1, '+01:00']]);

async function starten(vorbereiten) {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_JETZT: JETZT }, vorbereiten });
}

async function dialogOeffnen(browser, { kontext = { viewport: { width: 1280, height: 800 } }, channel = 'allgemein' } = {}) {
  const neu = await browser.newContext(kontext);
  const page = await neu.newPage();
  const fehler = fehlerSammeln(page, { zusaetzlich: { [ROUTE]: [409] } });
  await page.goto(`${instanz.adresse}#/c/${channel}`);
  await page.getByLabel('Name').fill('abcdefghijklm');
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText('abcdefghijklm@');
  // Per Tastatur: den Schalter mit Finger treffen prüft die Kopfzeile (r00019-kopfzeile.spec.js).
  await page.getByRole('banner').getByRole('button', { name: `# ${channel} — Channel-Einstellungen` }).focus();
  await page.keyboard.press('Enter');
  await expect(dialog(page)).toBeVisible();
  return { page, fehler };
}

const dialog = (page) => page.getByRole('dialog');
const feld = (page) => dialog(page).getByLabel('Aufbewahrungsfrist');
const verkuerzen = (page) => dialog(page).getByRole('button', { name: 'Frist verkürzen' });
const zurueck = (page) => dialog(page).getByRole('button', { name: 'Zurück' });

async function zweiterSchritt(page, tage = '7') {
  await feld(page).fill(tage);
  await feld(page).press('Enter');
  await expect(verkuerzen(page)).toBeVisible();
}

// `beta` setzt per API die Frist bestätigt; liefert den Zeitpunkt der 200.
async function betaSetzt(tage) {
  const beta = await beitretenPerApi(instanz.adresse, 'beta');
  const gesetzt = await fristPerApi(instanz.adresse, beta.daten.token, 'allgemein', { tage, bestaetigt: true });
  expect(gesetzt.status).toBe(200);
  return Date.now();
}

function abrufeMitschreiben(page) {
  const abrufe = [];
  page.on('request', (anfrage) => {
    if (anfrage.url().endsWith(ROUTE)) {
      abrufe.push(anfrage.method());
    }
  });
  return abrufe;
}

async function imErstenSchrittMit(page, eingabe, hinweis) {
  await expect(dialog(page)).toContainText(hinweis);
  await expect(feld(page)).toHaveValue(eingabe);
  await expect(feld(page)).toHaveJSProperty('readOnly', false);
  await expect(verkuerzen(page)).toBeHidden();
  await expect(dialog(page)).not.toContainText('entfallen sofort');
  await expect(feld(page)).toBeFocused();
}

test('Zweiter Schritt folgt fremder Änderung (US-10, B0387): beta setzt 7 bestätigt → bei A erster Schritt innerhalb Z2, Speichern → 7 Tage', async ({ browser }) => {
  await starten(verkuerzbar);
  const a = await dialogOeffnen(browser);
  await zweiterSchritt(a.page);

  const ab = await betaSetzt(7);
  const hinweis = 'Inzwischen geändert: 7 Tage — erneut speichern, um zu sehen, was entfällt.';
  const ms = await sichtbarNach(a.page, (text) => document.querySelector('#lc-frist-hinweis')?.textContent.includes(text), { argument: hinweis, ab, timeout: 10000 });

  console.log(`messung z2_ms: ${ms}`);
  expect(ms).toBeLessThanOrEqual(Z2_MS);
  await imErstenSchrittMit(a.page, '7', hinweis);
  await feld(a.page).press('Enter');
  await expect(a.page.locator('.lc-statuszeile')).toHaveText('Frist für # allgemein: 7 Tage.');
  expect(a.fehler).toEqual([]);
});

test('Zweiter Schritt folgt fremder Änderung, Variante 3 Tage (B0387): A sieht „Inzwischen geändert: 3 Tage — …“, bis A speichert, kein PUT', async ({ browser }) => {
  await starten(verkuerzbar);
  const a = await dialogOeffnen(browser);
  await zweiterSchritt(a.page);
  const abrufe = abrufeMitschreiben(a.page);

  await betaSetzt(3);

  await imErstenSchrittMit(a.page, '7', 'Inzwischen geändert: 3 Tage — erneut speichern, um zu sehen, was entfällt.');
  await expect(a.page.getByRole('banner').locator('.lc-head-frist')).toHaveText('· 3 Tage');
  expect(abrufe).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Gegenprobe ohne Rücksprung (B0387): A im ersten Schritt, beta setzt 9 → „Inzwischen geändert: 9 Tage.“ ohne Zusatz', async ({ browser }) => {
  await starten(verkuerzbar);
  const a = await dialogOeffnen(browser);
  await feld(a.page).fill('12');

  await betaSetzt(9);

  await expect(dialog(a.page).locator('#lc-frist-hinweis')).toHaveText('Ältere Tage entfallen. Heute zählt mit. Standard: 7 Tage. Inzwischen geändert: 9 Tage.');
  await expect(feld(a.page)).toHaveValue('12');
  expect(a.fehler).toEqual([]);
});

// Statt des Musters /2 Tage .* 5 Nachrichten .* nicht rückgängig/ der ganze Satz — das Muster verlangt ein Leerzeichen
// nach „Nachrichten“, im Satz steht dort der Punkt (wie in r00008-frist.spec.js, B0317).
for (const [art, vorbereiten, satz] of [
  ['Mehrzahl', verkuerzbar, 'Bei 7 Tagen entfallen sofort 2 Tage bis einschließlich 21. Oktober 2026 mit 5 Nachrichten. Das lässt sich nicht rückgängig machen.'],
  ['Einzahl', einTagVerkuerzbar, 'Bei 7 Tagen entfällt sofort 1 Tag bis einschließlich 21. Oktober 2026 mit 1 Nachricht. Das lässt sich nicht rückgängig machen.'],
]) {
  test(`„Frist verkürzen“ beschrieben (US-11, B0388, ${art}): beide Schalter des zweiten Schritts tragen den Satz als Beschreibung`, async ({ browser }) => {
    await starten(vorbereiten);
    const a = await dialogOeffnen(browser);

    await zweiterSchritt(a.page);

    await expect(verkuerzen(a.page)).toHaveAccessibleDescription(satz);
    await expect(zurueck(a.page)).toHaveAccessibleDescription(satz);
    expect(a.fehler).toEqual([]);
  });
}

const LANGER_NAME = 'm'.repeat(32);
const telefon = (breite, hoehe) => ({ viewport: { width: breite, height: hoehe }, hasTouch: true, isMobile: true });

// Liegt das Element ganz im Fenster (oben, unten, links, rechts)?
function ganzImFenster(locator) {
  return locator.evaluate((element) => {
    const rand = element.getBoundingClientRect();
    return rand.width > 0 && rand.top >= 0 && rand.left >= 0 && rand.bottom <= window.innerHeight && rand.right <= window.innerWidth;
  });
}

test('Dialog am Telefon (US-15, B0544): Titel aus 32 × m bei 320 × 640 ganz im Dialog umbrochen, kein Querscrollen', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, LANGER_NAME));
  const a = await dialogOeffnen(browser, { kontext: telefon(320, 640), channel: LANGER_NAME });

  const titel = dialog(a.page).getByRole('heading');
  await expect(titel).toHaveText(`# ${LANGER_NAME} — Einstellungen`);
  const mass = await dialog(a.page).evaluate((fenster) => {
    const titelRand = fenster.querySelector('.lc-dialog-titel').getBoundingClientRect();
    const fensterRand = fenster.getBoundingClientRect();
    return {
      titelRechts: titelRand.right,
      titelNoetig: fenster.querySelector('.lc-dialog-titel').scrollWidth,
      titelBreite: fenster.querySelector('.lc-dialog-titel').clientWidth,
      dialogRechts: fensterRand.right,
      dialogNoetig: fenster.scrollWidth,
      dialogBreite: fenster.clientWidth,
      seite: document.documentElement.scrollWidth,
      fenster: window.innerWidth,
    };
  });
  expect(mass.titelRechts).toBeLessThanOrEqual(mass.dialogRechts);
  expect(mass.titelNoetig, 'Titel nicht abgeschnitten').toBeLessThanOrEqual(mass.titelBreite);
  expect(mass.dialogNoetig, 'Dialog scrollt nicht quer').toBeLessThanOrEqual(mass.dialogBreite);
  expect(mass.seite, 'Seite scrollt nicht quer').toBeLessThanOrEqual(mass.fenster);
  expect(a.fehler).toEqual([]);
});

// Bildschirmtastatur genähert (Playwright zeigt keine): Telefon 375 × 667 abzüglich rund 260 px → Fenster 375 × 400.
test('Dialog mit Bildschirmtastatur (US-16, B0544): 0 + Enter → Fehlerzeile und „Speichern“ ganz im Fenster 375 × 400', async ({ browser }) => {
  await starten(verkuerzbar);
  const a = await dialogOeffnen(browser, { kontext: telefon(375, 400) });

  await feld(a.page).fill('0');
  await feld(a.page).press('Enter');

  await expect(dialog(a.page).getByRole('alert')).toHaveText('⚠ Frist: ganze Zahl von 1 bis 3650 Tagen.');
  expect(await ganzImFenster(dialog(a.page).getByRole('alert'))).toBe(true);
  expect(await ganzImFenster(dialog(a.page).getByRole('button', { name: 'Speichern' }))).toBe(true);
  expect(a.fehler).toEqual([]);
});

test('Dialog mit Bildschirmtastatur, zweiter Schritt (US-16, B0544): „Zurück“ und „Frist verkürzen“ ganz im Fenster 375 × 400', async ({ browser }) => {
  await starten(verkuerzbar);
  const a = await dialogOeffnen(browser, { kontext: telefon(375, 400) });

  await zweiterSchritt(a.page);

  expect(await ganzImFenster(zurueck(a.page))).toBe(true);
  expect(await ganzImFenster(verkuerzen(a.page))).toBe(true);
  expect(a.fehler).toEqual([]);
});
