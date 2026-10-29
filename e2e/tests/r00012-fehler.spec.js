'use strict';
// R00012 F0046 Fehler und Wiederholung im Browser (US-2 bis US-5, Z7, Z8, Z11): gegen das echte Programm, Chromium
// 1280 × 800, Uhr ab 2026-10-28 15:00 (+01:00). A ist `shoff`, B ist `anna`. „Strom getrennt“: Netz des Kontexts aus,
// bis die Leiste „Verbindung verloren“ steht.
const { test, expect } = require('@playwright/test');
const { sendenStarten, beitreten, verlaufLesen, kontextOeffnen, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const {
  ROUTE, vorlaeufige, bestaetigt, netzAus, netzAn, saetzeMit, sendungenMitschreiben,
} = require('./r00012-hilfen');

/// Z11 aus R00012: 9 s nach Enter noch „wird gesendet …“, spätestens 13 s nach Enter fehlgeschlagen.
const Z11_NOCH_WARTEND_MS = 9000;
const Z11_SPAETESTENS_MS = 13000;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const NACHRICHTEN = '/api/channels/allgemein/nachrichten';

async function seiteMit(browser, name, sammeln = {}) {
  const seite = await kontextOeffnen(browser, sammeln);
  await imBrowserBeitreten(seite.page, instanz.adresse, name);
  return seite;
}

async function apiTexte() {
  const token = await beitreten(instanz.adresse, 'leser', { herkunft: '127.0.0.9' });
  const liste = await verlaufLesen(instanz.adresse, token, 'allgemein', '127.0.0.9');
  return liste.map((nachricht) => nachricht.text);
}

/// Nur Fehlerzeilen zu Anfragen, die bei getrenntem Netz oder durch die Route scheitern.
function ohneGescheiterteAnfragen(fehler) {
  return fehler.filter((zeile) => !/Failed to load resource: net::ERR_(INTERNET_DISCONNECTED|FAILED)/.test(zeile));
}

async function senden(page, text) {
  await eingabe(page).fill(text);
  await eingabe(page).press('Enter');
}

// „Gepuffert“ (US-2, Z7): A getrennt, sendet p1 und p2 → beide „wird gesendet …“, kein POST; Strom wieder → beide genau
// einmal bestätigt in dieser Folge bei A und B, die API kennt beide genau einmal.
test('US-2 (Z7): ohne Netz geschrieben, mit Netz genau einmal gesendet', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await seiteMit(browser, 'shoff');
  const b = await seiteMit(browser, 'anna');
  const sendungen = sendungenMitschreiben(a.page);
  await netzAus(a);

  await senden(a.page, 'p1');
  await senden(a.page, 'p2');
  await expect(vorlaeufige(a.page).locator('.lc-send')).toHaveText(['◌ wird gesendet …', '◌ wird gesendet …']);
  const waehrendGetrennt = sendungen.length;
  await netzAn(a);

  for (const seite of [a.page, b.page]) {
    await expect(bestaetigt(seite, 'p[12]').locator('.lc-body')).toHaveText(['p1', 'p2'], { timeout: 10000 });
  }
  await expect(vorlaeufige(a.page)).toHaveCount(0);
  expect(waehrendGetrennt).toBe(0);
  expect(await apiTexte()).toEqual(['p1', 'p2']);
  expect(ohneGescheiterteAnfragen(a.fehler)).toEqual([]);
  expect(b.fehler).toEqual([]);
});

// „Antwort verloren“ (US-3, Z8): die Route trennt zuerst den Strom, leitet den POST weiter und bricht die Antwort an
// die Seite ab → `einmal` bleibt „wird gesendet …“; Strom wieder → genau einmal bestätigt, ein Satz im Log; die
// Wiederholung trug denselben Schlüssel und bekam 200.
test('US-3 (Z8): die Antwort ging verloren — trotzdem nur einmal', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await seiteMit(browser, 'shoff');
  const sendungen = sendungenMitschreiben(a.page);
  const antworten = [];
  a.page.on('response', (antwort) => {
    if (antwort.request().method() === 'POST' && antwort.url().endsWith('/nachrichten')) {
      antworten.push(antwort.status());
    }
  });
  let erste = true;
  let weitergeleitet = null;
  await a.page.route(ROUTE, async (route) => {
    if (route.request().method() !== 'POST' || !erste) {
      await route.continue();
      return;
    }
    erste = false;
    await a.kontext.setOffline(true);
    weitergeleitet = await route.fetch();
    await route.abort();
  });

  await senden(a.page, 'einmal');
  await expect.poll(() => weitergeleitet !== null).toBe(true);
  await expect(vorlaeufige(a.page).locator('.lc-send')).toHaveText('◌ wird gesendet …');
  await expect(a.page.locator('.lc-leiste')).toHaveText(/^⚠ Verbindung verloren/);
  await netzAn(a);

  await expect(vorlaeufige(a.page)).toHaveCount(0, { timeout: 10000 });
  await expect(bestaetigt(a.page, 'einmal')).toHaveCount(1);
  expect(weitergeleitet.status()).toBe(201);
  expect(saetzeMit(instanz.ordner, 'allgemein', 'einmal')).toBe(1);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['einmal', 'einmal']);
  expect(sendungen[1].schluessel).toBe(sendungen[0].schluessel);
  expect(antworten).toEqual([200]);
  expect(ohneGescheiterteAnfragen(a.fehler)).toEqual([]);
});

/// Die Route bricht den ersten POST ab, ohne ihn weiterzuleiten (Strom verbunden), danach lässt sie durch.
async function erstenPostAbbrechen(page) {
  let erste = true;
  await page.route(ROUTE, async (route) => {
    if (route.request().method() !== 'POST' || !erste) {
      await route.continue();
      return;
    }
    erste = false;
    await route.abort();
  });
}

// „Erneut senden“ (US-4): ⚠ „Keine Antwort vom Server.“ mit beiden Schaltern; „Erneut senden“ → genau einmal
// bestätigt, ein Satz im Log, der Fokus im Feld.
test('US-4: erneut senden', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await seiteMit(browser, 'shoff');
  await erstenPostAbbrechen(a.page);

  await senden(a.page, 'zwei');
  const fehlgeschlagen = vorlaeufige(a.page);
  await expect(fehlgeschlagen.locator('.lc-send')).toHaveText('⚠ Keine Antwort vom Server.');
  await expect(fehlgeschlagen.getByRole('button')).toHaveText(['Erneut senden', 'Verwerfen']);
  await fehlgeschlagen.getByRole('button', { name: 'Erneut senden' }).click();

  await expect(vorlaeufige(a.page)).toHaveCount(0);
  await expect(bestaetigt(a.page, 'zwei')).toHaveCount(1);
  await expect(eingabe(a.page)).toBeFocused();
  expect(saetzeMit(instanz.ordner, 'allgemein', 'zwei')).toBe(1);
  expect(ohneGescheiterteAnfragen(a.fehler)).toEqual([]);
});

// „Verwerfen“ (US-4): `drei` ist danach nicht mehr im Verlauf, das Feld leer, der Fokus im Feld; kein Satz im Log.
test('US-4: verwerfen', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await seiteMit(browser, 'shoff');
  await erstenPostAbbrechen(a.page);

  await senden(a.page, 'drei');
  await expect(vorlaeufige(a.page).locator('.lc-send')).toHaveText('⚠ Keine Antwort vom Server.');
  await vorlaeufige(a.page).getByRole('button', { name: 'Verwerfen' }).click();

  await expect(a.page.getByRole('log').locator('.lc-body', { hasText: 'drei' })).toHaveCount(0);
  await expect(eingabe(a.page)).toHaveValue('');
  await expect(eingabe(a.page)).toBeFocused();
  expect(saetzeMit(instanz.ordner, 'allgemein', 'drei')).toBe(0);
  expect(ohneGescheiterteAnfragen(a.fehler)).toEqual([]);
});

// „Zeitgrenze“ (US-5, Z11): die Route hält den POST ohne Antwort fest → 9 s nach Enter noch „wird gesendet …“,
// spätestens 13 s nach Enter ⚠ „Keine Antwort vom Server.“. Gemessen im Browser: Enter (keydown) bis zum ersten Bild
// mit der Fehlerzeile.
test('US-5 (Z11): der Server schweigt', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await sendenStarten();
  const a = await seiteMit(browser, 'shoff');
  await a.page.route(ROUTE, async (route) => {
    if (route.request().method() !== 'POST') {
      await route.continue();
    }
  });
  await eingabe(a.page).fill('still');
  await eingabe(a.page).evaluate((feld) => {
    feld.addEventListener('keydown', () => { window.enterUm = Date.now(); }, { once: true });
  });

  await eingabe(a.page).press('Enter');
  const fehlgeschlagenUm = await a.page.waitForFunction(() => {
    const zeile = document.querySelector('[role="log"] .lc-send.lc-fehlgeschlagen');
    if (zeile === null || zeile.textContent !== '⚠ Keine Antwort vom Server.') {
      return false;
    }
    return Date.now();
  }, null, { polling: 'raf', timeout: 30000 }).then((ergebnis) => ergebnis.jsonValue());
  const dauer = fehlgeschlagenUm - await a.page.evaluate(() => window.enterUm);

  console.log(`Z11 gemessen: ${dauer} ms (noch wartend bis ${Z11_NOCH_WARTEND_MS}, spätestens ${Z11_SPAETESTENS_MS})`);
  expect(dauer).toBeGreaterThan(Z11_NOCH_WARTEND_MS);
  expect(dauer).toBeLessThanOrEqual(Z11_SPAETESTENS_MS);
  expect(ohneGescheiterteAnfragen(a.fehler)).toEqual([]);
});

// „401“: die Route antwortet auf den ersten POST 401, danach lässt sie durch → A tritt still wieder bei (POST
// /api/beitreten mit dem gemerkten Namen), der zweite POST trägt denselben Schlüssel, `vier` genau einmal bestätigt;
// keine Fehlermeldung.
test('401: still wieder beitreten und genau einmal neu senden', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await seiteMit(browser, 'shoff', { zusaetzlich: { [NACHRICHTEN]: [401] } });
  const sendungen = sendungenMitschreiben(a.page);
  const beitritte = [];
  a.page.on('request', (anfrage) => {
    if (anfrage.url().endsWith('/api/beitreten')) {
      beitritte.push(anfrage.postDataJSON().name);
    }
  });
  let erste = true;
  await a.page.route(ROUTE, async (route) => {
    if (route.request().method() !== 'POST' || !erste) {
      await route.continue();
      return;
    }
    erste = false;
    await route.fulfill({ status: 401, contentType: 'application/json', body: JSON.stringify({ fehler: 'Sitzung unbekannt.' }) });
  });

  await senden(a.page, 'vier');

  await expect(bestaetigt(a.page, 'vier')).toHaveCount(1);
  await expect(vorlaeufige(a.page)).toHaveCount(0);
  expect(beitritte).toEqual(['shoff']);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['vier', 'vier']);
  expect(sendungen[1].schluessel).toBe(sendungen[0].schluessel);
  await expect(a.page.getByRole('log').locator('.lc-send.lc-fehlgeschlagen')).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

// „Schmal“: bei 320 px mit einer fehlgeschlagenen Nachricht samt Schaltern kein Querscrollen.
test('Schmal: fehlgeschlagen mit Schaltern bei 320 px ohne Querscrollen', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { breite: 320 });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await erstenPostAbbrechen(a.page);

  await senden(a.page, 'schmal');
  await expect(vorlaeufige(a.page).getByRole('button')).toHaveCount(2);

  const breiten = await a.page.evaluate(() => [document.documentElement.scrollWidth, window.innerWidth]);
  expect(breiten[0]).toBeLessThanOrEqual(breiten[1]);
  expect(ohneGescheiterteAnfragen(a.fehler)).toEqual([]);
});
