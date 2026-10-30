'use strict';
// R00030 F0119 B0520 „Ohne Verbindung“ (E2E, Desktop, gedrosselt): bricht die Verbindung während Anmelden oder Hochladen
// ab (`context.setOffline(true)`), zeigt die Datei „wartet auf Verbindung“ — kein ⚠, kein „Erneut hochladen“. Nach
// „Wieder verbunden“ meldet der Browser selbst neu an bzw. lädt dieselbe Anmeldung neu (nach `abgelaufen` neue
// Anmeldung) und sendet danach, genau einmal bei A und B.
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, kontextOeffnen, eingabe } = require('./senden-hilfen');
const { knopf } = require('./r00027-hilfen');
const { saetzeMit } = require('./r00012-hilfen');
const { anhangStarten, anfragenMitschreiben, waehlen, berichtPdf } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const datei = (page, name) => eintrag(page).locator('.lc-hochladung-datei', { hasText: name });
const leiste = (page) => page.locator('.lc-leiste');
/// Konsolenzeilen außer denen, die der Test vorschreibt: Anfragen, solange die Seite offline ist, und `auch`.
const unerwartet = (fehler, auch = []) => fehler.filter((zeile) => ![
  'net::ERR_INTERNET_DISCONNECTED', ...auch].some((vorgeschrieben) => zeile.includes(vorgeschrieben)));
const bestaetigt = (page, text) => page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)').filter({ has: page.locator('.lc-body', { hasText: text }) });

/// A (`shoff`) und B (`anna`) im Channel; A beim Hochladen auf `bytesJeSekunde` gedrosselt.
async function zweiAuf(browser, { bytesJeSekunde = 200 * 1024, zusaetzlich = {} } = {}) {
  instanz = await anhangStarten();
  const a = await kontextOeffnen(browser, { zusaetzlich });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const cdp = await a.page.context().newCDPSession(a.page);
  await cdp.send('Network.enable');
  const netz = (offline) => cdp.send('Network.emulateNetworkConditions', { offline, latency: 0, downloadThroughput: -1, uploadThroughput: bytesJeSekunde });
  await netz(false);
  // Offline über dieselbe Emulation wie die Drosselung: `context.setOffline` hebt die Drosselung auf, und der Rest des
  // laufenden PUT ginge sofort hinaus.
  a.offline = async () => {
    await a.kontext.setOffline(true);
    await netz(true);
  };
  a.online = async () => {
    await netz(false);
    await a.kontext.setOffline(false);
  };
  return { a, b, anfragen: anfragenMitschreiben(a.page) };
}

async function wartetAufVerbindung(page, name) {
  await expect(datei(page, name).locator('.lc-hochladung-zustand')).toHaveText('wartet auf Verbindung');
  await expect(datei(page, name)).not.toContainText('⚠');
  await expect(datei(page, name).getByRole('button', { name: `${name} erneut hochladen` })).toHaveCount(0);
}

async function genauEinmalGesendet({ a, b }, text) {
  await expect(bestaetigt(a.page, text)).toHaveCount(1, { timeout: 30000 });
  await expect(bestaetigt(b.page, text)).toHaveCount(1);
  await expect(eintrag(a.page)).toHaveCount(0);
  expect(saetzeMit(instanz.ordner, 'allgemein', text)).toBe(1);
}

test('Offline während des Hochladens: „wartet auf Verbindung“ ohne ⚠ und ohne „Erneut“; nach „Wieder verbunden“ PUT auf dieselbe ID, gesendet genau einmal', async ({ browser }) => {
  test.setTimeout(60000);
  const seiten = await zweiAuf(browser);
  const { a, anfragen } = seiten;
  await eingabe(a.page).fill('Bericht anbei');
  await waehlen(a.page, [berichtPdf()]);
  await knopf(a.page).click();
  await expect(datei(a.page, 'bericht.pdf').getByRole('progressbar')).toBeVisible();

  await a.offline();
  await wartetAufVerbindung(a.page, 'bericht.pdf');
  const putsVorher = anfragen.filter((anfrage) => anfrage.methode === 'PUT').map((anfrage) => anfrage.pfad);
  await a.online();
  await expect(leiste(a.page)).toContainText('Wieder verbunden', { timeout: 15000 });

  await genauEinmalGesendet(seiten, 'Bericht anbei');
  const puts = anfragen.filter((anfrage) => anfrage.methode === 'PUT').map((anfrage) => anfrage.pfad);
  expect(puts.length).toBeGreaterThan(putsVorher.length);
  expect(new Set(puts).size).toBe(1);
  expect(anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge')).toHaveLength(1);
  expect(unerwartet(a.fehler)).toEqual([]);
});

test('Offline beim Anmelden: „wartet auf Verbindung“, nach „Wieder verbunden“ meldet der Browser selbst an, lädt hoch und sendet', async ({ browser }) => {
  test.setTimeout(60000);
  const seiten = await zweiAuf(browser);
  const { a } = seiten;
  await eingabe(a.page).fill('Später angemeldet');
  await waehlen(a.page, [berichtPdf()]);
  await a.offline();
  await knopf(a.page).click();

  await wartetAufVerbindung(a.page, 'bericht.pdf');
  await a.online();
  await expect(leiste(a.page)).toContainText('Wieder verbunden', { timeout: 15000 });

  await genauEinmalGesendet(seiten, 'Später angemeldet');
  expect(unerwartet(a.fehler)).toEqual([]);
});

test('Nach „Wieder verbunden“ ist die Anmeldung abgelaufen (404): neue Anmeldung, Hochladen, gesendet genau einmal', async ({ browser }) => {
  test.setTimeout(60000);
  const seiten = await zweiAuf(browser);
  const { a, anfragen } = seiten;
  await eingabe(a.page).fill('Neu angemeldet');
  await waehlen(a.page, [berichtPdf()]);
  await knopf(a.page).click();
  await expect(datei(a.page, 'bericht.pdf').getByRole('progressbar')).toBeVisible();

  await a.offline();
  await wartetAufVerbindung(a.page, 'bericht.pdf');
  let ersterNachDerTrennung = true;
  await a.page.route('**/api/anhaenge/*', (route) => {
    if (route.request().method() === 'PUT' && ersterNachDerTrennung) {
      ersterNachDerTrennung = false;
      return route.fulfill({
        status: 404, contentType: 'application/json',
        body: JSON.stringify({ fehler: 'Diese Anmeldung ist abgelaufen — neu anmelden.', grund: 'abgelaufen' }),
      });
    }
    return route.continue();
  });
  await a.online();

  await genauEinmalGesendet(seiten, 'Neu angemeldet');
  expect(anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge')).toHaveLength(2);
  expect(unerwartet(a.fehler, ['status of 404 (Not Found)'])).toEqual([]);
});
