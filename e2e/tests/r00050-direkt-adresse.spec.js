'use strict';
// R00050 F0178 B0784 (Karte #13): die Adresse `#/d/<kennung>` öffnet die Hauptansicht im Privat-Modus (`lc-privat-bg`)
// mit Verlauf und Eingabe gegen `/api/direkt/<kennung>/…` (V7, V9, V10); die eigene und jede ungültige Adresse führen
// zu keinem Gespräch (V14); die Adresse eines fremden Paars öffnet nie dessen Inhalt (R3; Mutante „Hash eines
// fremden Paars öffnet Inhalt“). Gegen das echte Programm.
const { test, expect } = require('./rahmen');
const { mitText } = require('./senden-hilfen');
const {
  ANNA, BEN, CORA, KEIN_GESPRAECH, tippenOhneGespraech, direktStarten, direktSenden, direktLesen, paarordner, direktOeffnen, verlaufMit, eingabeAn,
  gespraechOffen, kontrastUnterschritten,
} = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function starten(browser, optionen) {
  const gestartet = await direktStarten(browser, optionen);
  instanz = gestartet.instanz;
  return gestartet;
}

const mitte = (page) => page.locator('main.lc-center');
const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));

test('Adresse (B0784, V7, V9): #/d/ben%40127.0.0.3 zeigt Verlauf und Eingabe des Paars im Privat-Modus; Senden geht an ben', async ({ browser }) => {
  const { anna, token } = await starten(browser);

  await direktOeffnen(anna.page, BEN);

  await gespraechOffen(anna.page, BEN);
  await expect(mitte(anna.page)).toHaveClass(/\blc-privat\b/);
  const grund = await mitte(anna.page).evaluate((element) => ({
    gezeichnet: getComputedStyle(element).backgroundColor,
    token: getComputedStyle(document.documentElement).getPropertyValue('--lc-privat-bg').trim(),
  }));
  expect(grund).toEqual({ gezeichnet: 'rgb(233, 237, 242)', token: '#e9edf2' });
  await expect(mitte(anna.page).locator('.lc-mitte-hinweis').first()).toHaveText(`Noch keine Nachrichten mit ${BEN}.`);
  await expect(mitte(anna.page).locator('.lc-leer-api')).toHaveText(`Agenten schreiben hier mit POST /api/direkt/${BEN}/nachrichten.`);

  await eingabeAn(anna.page, BEN).fill('Hallo ben');
  await eingabeAn(anna.page, BEN).press('Enter');

  await expect(mitText(anna.page, /^Hallo ben$/).and(anna.page.locator('.lc-msg:not(.lc-vorlaeufig)'))).toHaveCount(1);
  expect(await direktLesen(instanz.adresse, token.ben, ANNA, '127.0.0.3')).toEqual(['Hallo ben']);
  await anna.page.evaluate(() => { window.location.hash = '#/c/allgemein'; });
  await expect(anna.page.getByRole('log', { name: 'Verlauf # allgemein' })).toBeVisible();
  await expect(mitte(anna.page)).not.toHaveClass(/\blc-privat\b/);
  await expect(mitText(anna.page, /Hallo ben/)).toHaveCount(0);
  expect(anna.fehler).toEqual([]);
});

// Seit O4 (Code-Review #13 F0178) meldet ein leeres Direktgespräch kein Tippen — darum hat das Gespräch hier eine Nachricht.
test('Entwurf und Tippen (B0784, V10): der Entwurf liegt unter localchatA.entwurf.direkt:<kennung>, Tippen geht an das offene Gespräch', async ({ browser }) => {
  const { instanz: gestartet, anna, token } = await starten(browser);
  expect((await direktSenden(gestartet.adresse, token.ben, ANNA, 'Erste von ben', '127.0.0.3')).status).toBe(201);
  const aktivitaet = [];
  anna.page.on('request', (anfrage) => {
    const pfad = new URL(anfrage.url()).pathname;
    if (pfad.endsWith('/aktivitaet')) {
      aktivitaet.push(`${anfrage.method()} ${pfad}`);
    }
  });

  await direktOeffnen(anna.page, BEN);
  await gespraechOffen(anna.page, BEN);
  await expect(verlaufMit(anna.page, BEN).locator('.lc-msg')).toHaveCount(1);
  await eingabeAn(anna.page, BEN).pressSequentially('Entwurf an ben');
  await warten(700);
  await anna.page.evaluate(() => { window.location.hash = '#/c/allgemein'; });
  await expect(anna.page.getByRole('textbox', { name: 'Nachricht an # allgemein' })).toHaveValue('');

  const gemerkt = await anna.page.evaluate(() => Object.fromEntries(Object.keys(sessionStorage)
    .filter((schluessel) => schluessel.startsWith('localchatA.entwurf.')).map((schluessel) => [schluessel, sessionStorage.getItem(schluessel)])));
  expect(gemerkt).toEqual({ [`localchatA.entwurf.direkt:${BEN}`]: 'Entwurf an ben' });
  expect(aktivitaet.length).toBeGreaterThan(0);
  expect(aktivitaet.filter((aufruf) => !aufruf.endsWith(`/api/direkt/${encodeURIComponent(BEN)}/aktivitaet`))).toEqual([]);

  await direktOeffnen(anna.page, BEN);
  await expect(eingabeAn(anna.page, BEN)).toHaveValue('Entwurf an ben');
  expect(anna.fehler).toEqual([]);
});

for (const [fall, kennung] of [
  ['die eigene Kennung', ANNA], ['ohne Herkunft', 'xyz'], ['eine unmögliche Adresse', 'ben@999.1.1.1'], ['Großbuchstaben', 'BEN@127.0.0.3'],
  ['IPv4-gemappt (Code-Review S1)', 'ben@::ffff:7f00:1'],
]) {
  test(`Keine Adresse eines Gesprächs (B0784, V14): ${fall} → „${KEIN_GESPRAECH}“ mit Rückweg, Eingabe disabled`, async ({ browser }) => {
    const { anna } = await starten(browser);
    const direktAufrufe = [];
    anna.page.on('request', (anfrage) => {
      if (new URL(anfrage.url()).pathname.startsWith('/api/direkt/')) {
        direktAufrufe.push(anfrage.url());
      }
    });

    await direktOeffnen(anna.page, kennung);

    await expect(mitte(anna.page).getByText(KEIN_GESPRAECH, { exact: true })).toBeVisible();
    await expect(mitte(anna.page).getByRole('textbox')).toBeDisabled();
    await expect(anna.page.getByRole('log')).toHaveCount(0);
    expect(direktAufrufe).toEqual([]);
    await mitte(anna.page).getByRole('link', { name: 'Zurück zu # allgemein' }).click();
    await expect(anna.page.getByRole('log', { name: 'Verlauf # allgemein' })).toBeVisible();
    await expect(anna.page.getByRole('textbox', { name: 'Nachricht an # allgemein' })).toBeEnabled();
    expect(anna.fehler).toEqual([]);
  });
}

test('Fremdes Paar (B0784, R3): cora↔ben haben ein Gespräch — anna sieht unter #/d/cora… und #/d/ben… nur leere eigene Gespräche', async ({ browser }) => {
  const { anna, token } = await starten(browser);
  expect((await direktSenden(instanz.adresse, token.cora, BEN, 'Nur für ben', '127.0.0.9')).status).toBe(201);
  const vorher = paarordner(instanz.ordner);
  expect(vorher).toHaveLength(1);

  for (const kennung of [CORA, BEN]) {
    await direktOeffnen(anna.page, kennung);
    await gespraechOffen(anna.page, kennung);
    await expect(mitte(anna.page).locator('.lc-mitte-hinweis').first()).toHaveText(`Noch keine Nachrichten mit ${kennung}.`);
    await expect(verlaufMit(anna.page, kennung).locator('.lc-msg')).toHaveCount(0);
  }

  await expect(anna.page.getByText('Nur für ben')).toHaveCount(0);
  expect(paarordner(instanz.ordner)).toEqual(vorher);
  expect(anna.fehler).toEqual([]);
});

for (const thema of ['light', 'dark']) {
  test(`Nicht nur Farbe (B0784, V7) — ${thema}: Kontrast im Privat-Modus und das Wort in Verlauf und Eingabe, auch unter forced-colors`, async ({ browser }) => {
    const { anna, token } = await starten(browser);
    await anna.page.emulateMedia({ colorScheme: thema });
    expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'Hallo anna', '127.0.0.3')).status).toBe(201);

    await direktOeffnen(anna.page, BEN);
    await gespraechOffen(anna.page, BEN);
    await expect(mitText(anna.page, /^Hallo anna$/)).toHaveCount(1);

    expect(await anna.page.evaluate(kontrastUnterschritten, 'main.lc-center'), 'Kontrast unterschritten').toEqual([]);
    await anna.page.emulateMedia({ colorScheme: thema, forcedColors: 'active' });
    await gespraechOffen(anna.page, BEN);
    expect(anna.fehler).toEqual([]);
  });
}

// Code-Review #13 F0178 O1: im Direktgespräch sagt der Hinweis „Direktnachricht erneut wählen“, und die neue Wahl über
// das Menü der Teilnehmerliste versucht es — wie die Wahl des offenen Channels in der Seitenleiste (R00007 US-13).
test('Nicht geladen im Direktgespräch (O1): „Direktnachricht erneut wählen“, die Wahl über das Menü lädt erneut', async ({ browser }) => {
  const { instanz: gestartet, anna, token } = await starten(browser, {
    sammeln: { zusaetzlich: tippenOhneGespraech(BEN, CORA), serverWegErwartet: true },
  });
  expect((await direktSenden(gestartet.adresse, token.ben, ANNA, 'Erste von ben', '127.0.0.3')).status).toBe(201);
  const abruf = `**/api/direkt/${encodeURIComponent(BEN)}/nachrichten?n=100`;
  await anna.page.route(abruf, (route) => route.abort('connectionrefused'));
  const ueberDasMenue = async () => {
    await anna.page.getByRole('complementary', { name: 'Teilnehmer' }).locator('.lc-person')
      .filter({ has: anna.page.locator('.lc-id', { hasText: BEN }) }).getByRole('button').click();
    await anna.page.getByRole('menuitem', { name: 'Direktnachricht' }).click();
  };

  await ueberDasMenue();
  await expect(mitte(anna.page).locator('.lc-fehler'))
    .toHaveText('⚠ Verlauf nicht geladen — Direktnachricht erneut wählen, um es noch einmal zu versuchen.');
  await anna.page.unroute(abruf);
  await ueberDasMenue();

  await expect(verlaufMit(anna.page, BEN).locator('.lc-msg .lc-body')).toHaveText(['Erste von ben']);
  await expect(mitte(anna.page).locator('.lc-fehler')).toHaveCount(0);
  expect(anna.fehler).toEqual([]);
});

// ux-reviewer #13 F0178 Optional 4: der Merker des Tabs kennt das offene Direktgespräch — ein Laden ohne Adresse öffnet
// es wieder (vorher # allgemein); der Rückweg aus einer Adresse ohne Gespräch führt weiter zu einem Channel.
test('Neuladen ohne Adresse (Optional 4): das zuletzt offene Direktgespräch öffnet wieder; Rückweg zu einem Channel', async ({ browser }) => {
  const { instanz: gestartet, anna } = await starten(browser);
  await direktOeffnen(anna.page, BEN);
  await gespraechOffen(anna.page, BEN);

  await anna.page.goto(gestartet.adresse);
  await expect(anna.page).toHaveURL(new RegExp(`#/d/${encodeURIComponent(BEN)}$`));
  await gespraechOffen(anna.page, BEN);

  await direktOeffnen(anna.page, ANNA);
  await expect(mitte(anna.page).getByRole('link', { name: 'Zurück zu # allgemein' })).toBeVisible();
  expect(anna.fehler).toEqual([]);
});
