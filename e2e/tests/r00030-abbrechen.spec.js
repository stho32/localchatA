'use strict';
// R00030 F0101 B0452 — Abbrechen, Erneut und Verwerfen am Eintrag „wird hochgeladen …“: „<name>: Hochladen abbrechen“
// bricht ab und gibt die Anmeldung frei (`DELETE`), der Fokus geht auf das nächste „Abbrechen“, ohne eines ins Feld,
// nie auf body; die letzte Datei ohne Text nimmt den Eintrag weg, mit Text wird der Text gesendet. „<name> erneut
// hochladen“ lädt dieselbe Anmeldung neu, nach 404 `abgelaufen` meldet der Browser neu an. „Verwerfen“ (product-owner
// Zeile 82, U2) gibt alle Anmeldungen frei und legt den Text zurück ins leere Feld, sonst „Verworfen: <Anfang>“.
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { anhangStarten, perApi, anfragenMitschreiben, waehlen, fotoPng, berichtPdf } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const abbrechen = (page, name) => eintrag(page).getByRole('button', { name: `${name}: Hochladen abbrechen`, exact: true });
const erneut = (page, name) => eintrag(page).getByRole('button', { name: `${name} erneut hochladen`, exact: true });
const ERLAUBT = { zusaetzlich: { '/api/anhaenge': [422, 507] } };

async function seiteAuf(browser, geraet, { bytesJeSekunde = 100 * 1024 } = {}) {
  instanz = await anhangStarten();
  const seite = await geraetOeffnen(browser, geraet, ERLAUBT);
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  const cdp = await seite.page.context().newCDPSession(seite.page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: bytesJeSekunde });
  return { ...seite, anfragen: anfragenMitschreiben(seite.page) };
}

/// Die ID, unter der `name` angemeldet wurde, sobald ihr PUT läuft (`namen` aus namenMerken).
async function angemeldeteId(anfragen, namen, name) {
  const gesucht = () => [...namen.entries()].find(([, gemerkt]) => gemerkt === name);
  await expect.poll(() => {
    const treffer = gesucht();
    return treffer !== undefined && anfragen.some((anfrage) => anfrage.methode === 'PUT' && anfrage.pfad.endsWith(treffer[0]));
  }).toBe(true);
  return gesucht()[0];
}

/// ID → Name aus den Antworten der Anmeldung.
function namenMerken(page) {
  const namen = new Map();
  page.on('response', async (antwort) => {
    if (antwort.request().method() === 'POST' && new URL(antwort.url()).pathname === '/api/anhaenge' && antwort.status() === 201) {
      const { anhang } = await antwort.json();
      namen.set(anhang.id, anhang.name);
    }
  });
  return namen;
}

test('Abbrechen (Telefon, gedrosselt): an bericht.pdf → DELETE für dessen ID, die Datei ist weg, der Fokus auf dem nächsten „Abbrechen“, nie body', async ({ browser }) => {
  const { page, anfragen, fehler } = await seiteAuf(browser, 'telefon');
  const namen = namenMerken(page);

  await waehlen(page, [fotoPng(), berichtPdf()], { geste: 'tippen' });
  await knopf(page).tap();
  const id = await angemeldeteId(anfragen, namen, 'bericht.pdf');
  await expect(abbrechen(page, 'bericht.pdf')).toBeVisible();
  await abbrechen(page, 'bericht.pdf').tap();

  await expect(eintrag(page).locator('.lc-hochladung-datei', { hasText: 'bericht.pdf' })).toHaveCount(0);
  await expect(abbrechen(page, 'foto.png')).toBeFocused();
  await expect.poll(() => anfragen.filter((anfrage) => anfrage.methode === 'DELETE').map((anfrage) => anfrage.pfad)).toEqual([`/api/anhaenge/${id}`]);
  expect(await page.evaluate(() => document.activeElement !== document.body)).toBe(true);

  await expect.poll(() => anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).length, { timeout: 15000 }).toBe(1);
  const gesendet = anfragen.find((anfrage) => anfrage.pfad.endsWith('/nachrichten')).koerper;
  const fotoId = [...namen.entries()].find(([, name]) => name === 'foto.png')[0];
  expect(gesendet).toEqual({ text: '', anhaenge: [fotoId] });
  await expect.poll(async () => (await perApi(instanz.adresse, { methode: 'GET', pfad: 'api/gesundheit' })).daten.reserviert).toBe(0);
  expect(fehler).toEqual([]);
});

test('Abbrechen der letzten Datei: mit Text `nur Text` wird der Text gesendet, der Fokus geht ins Feld', async ({ browser }) => {
  const { page, anfragen } = await seiteAuf(browser, 'telefon');

  await waehlen(page, [berichtPdf()], { geste: 'tippen' });
  await eingabe(page).fill('nur Text');
  await knopf(page).tap();
  await expect(abbrechen(page, 'bericht.pdf')).toBeVisible();
  await abbrechen(page, 'bericht.pdf').tap();

  await expect(eintrag(page)).toHaveCount(0);
  await expect(page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)', { hasText: 'nur Text' })).toHaveCount(1);
  expect(anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).map((anfrage) => anfrage.koerper)).toEqual([{ text: 'nur Text' }]);
  await expect(eingabe(page)).toBeFocused();
});

test('Verwerfen (Desktop, gedrosselt): DELETE für die Anmeldung, der Eintrag ist weg, `Hallo` steht wieder im Feld', async ({ browser }) => {
  const { page, anfragen } = await seiteAuf(browser, 'desktop');
  const namen = namenMerken(page);

  await waehlen(page, [berichtPdf()]);
  await eingabe(page).fill('Hallo');
  await eingabe(page).press('Enter');
  const id = await angemeldeteId(anfragen, namen, 'bericht.pdf');
  await eintrag(page).getByRole('button', { name: 'Verwerfen', exact: true }).click();

  await expect(eintrag(page)).toHaveCount(0);
  await expect(eingabe(page)).toHaveValue('Hallo');
  await expect(eingabe(page)).toBeFocused();
  await expect.poll(() => anfragen.filter((anfrage) => anfrage.methode === 'DELETE').map((anfrage) => anfrage.pfad)).toEqual([`/api/anhaenge/${id}`]);
  expect(anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten'))).toEqual([]);
});

test('Verwerfen mit `Neu` im Feld: die Hinweiszeile sagt „Verworfen: Hallo“, `Neu` bleibt', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'desktop');

  await waehlen(page, [berichtPdf()]);
  await eingabe(page).fill('Hallo');
  await eingabe(page).press('Enter');
  await expect(eintrag(page)).toHaveCount(1);
  await eingabe(page).fill('Neu');
  await eintrag(page).getByRole('button', { name: 'Verwerfen', exact: true }).click();

  await expect(eintrag(page)).toHaveCount(0);
  await expect(eingabe(page)).toHaveValue('Neu');
  await expect(page.locator('#lc-comp-hinweis')).toHaveText('Verworfen: Hallo');
});

test('Fehlschlag und Erneut (Desktop): erster PUT abgebrochen → „Erneut hochladen“ → ein neuer PUT auf dieselbe ID, Fokus auf „Abbrechen“ derselben Datei', async ({ browser }) => {
  const { page, anfragen } = await seiteAuf(browser, 'desktop', { bytesJeSekunde: 200 * 1024 });
  let erster = true;
  await page.route('**/api/anhaenge/*', (route) => {
    if (route.request().method() === 'PUT' && erster) {
      erster = false;
      return route.abort();
    }
    return route.continue();
  });

  await waehlen(page, [berichtPdf()]);
  await knopf(page).click();
  await expect(erneut(page, 'bericht.pdf')).toBeVisible();
  await erneut(page, 'bericht.pdf').click();

  await expect(abbrechen(page, 'bericht.pdf')).toBeFocused();
  await expect.poll(() => anfragen.filter((anfrage) => anfrage.methode === 'PUT').length).toBe(2);
  const puts = anfragen.filter((anfrage) => anfrage.methode === 'PUT').map((anfrage) => anfrage.pfad);
  expect(puts[1]).toBe(puts[0]);
  expect(anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge')).toHaveLength(1);
  await expect.poll(() => anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).map((anfrage) => anfrage.koerper), { timeout: 20000 })
    .toEqual([{ text: '', anhaenge: [puts[0].split('/').at(-1)] }]);
});

test('Abgelaufen (Desktop): der PUT antwortet 404 `abgelaufen` → der Browser meldet neu an und lädt hoch', async ({ browser }) => {
  const { page, anfragen, fehler } = await seiteAuf(browser, 'desktop', { bytesJeSekunde: -1 });
  let erster = true;
  await page.route('**/api/anhaenge/*', (route) => {
    if (route.request().method() === 'PUT' && erster) {
      erster = false;
      return route.fulfill({ status: 404, contentType: 'application/json', body: JSON.stringify({ fehler: 'Diese Anmeldung ist abgelaufen — neu anmelden.', grund: 'abgelaufen' }) });
    }
    return route.continue();
  });

  await waehlen(page, [fotoPng()]);
  await knopf(page).click();

  await expect.poll(() => anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge').length).toBe(2);
  await expect.poll(() => anfragen.filter((anfrage) => anfrage.methode === 'PUT').length).toBe(2);
  const puts = anfragen.filter((anfrage) => anfrage.methode === 'PUT').map((anfrage) => anfrage.pfad);
  expect(puts[1]).not.toBe(puts[0]);
  await expect(eintrag(page).locator('.lc-hochladung-grund')).toBeHidden();
  expect(fehler.filter((zeile) => !zeile.includes('/api/anhaenge/'))).toEqual([]);
});
