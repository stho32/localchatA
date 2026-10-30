'use strict';
// R00030 F0119 B0519 — Neuladen während des Hochladens (UX M4, V8; product-owner Zeile 163; Gate #9 R1): solange ein
// Eintrag „wird hochgeladen …“ offen ist, fragt der Browser vor dem Neuladen nach; danach steht der gesicherte Text im
// Feld (bei nicht leerem Feld davor, durch eine Leerzeile getrennt), Hinweiszeile und Ansage-Region sagen „Hochladen
// unterbrochen — <namen> bitte erneut wählen.“, nur im Channel der Nachricht, bis zum Anhängen oder Senden. Der Eintrag
// im Tab-Speicher hält nur Text und Namen. Weicht die Hauptansicht dem Beitreten-Bildschirm (401, Beitritt scheitert),
// gilt dasselbe wie beim Neuladen.
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { anhangStarten, anfragenMitschreiben, waehlen, fotoPng, berichtPdf, textDatei } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const SCHLUESSEL = 'localchatA.unterbrochen.allgemein';
const NACHRICHTEN = '/api/channels/allgemein/nachrichten';
const BEITRETEN = '/api/beitreten';
const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const hinweiszeile = (page) => page.locator('#lc-comp-hinweis');
const seitenleiste = (page) => page.getByRole('navigation', { name: 'Channels' });

/// Desktop, gedrosselt auf `bytesJeSekunde` beim Hochladen. Vor jedem Laden der Seite merkt sich ein Init-Skript den
/// Tab-Speicher (`speicherBeimLaden`) und schreibt die Ansagen mit (`ansagenNachDemLaden`); `dialoge` sammelt die Art
/// jeder Rückfrage des Browsers und bestätigt sie.
async function seiteAuf(browser, { bytesJeSekunde = 100 * 1024, zusaetzlich = {} } = {}) {
  instanz = await anhangStarten();
  const seite = await geraetOeffnen(browser, 'desktop', { zusaetzlich });
  await seite.page.addInitScript(() => {
    window.speicherBeimLaden = Object.fromEntries(Object.entries(window.sessionStorage));
    window.ansagenNachDemLaden = [];
    new MutationObserver(() => {
      const region = document.querySelector('.lc-ansage');
      const neu = region !== null && region.textContent !== '' && window.ansagenNachDemLaden.at(-1) !== region.textContent;
      if (neu) {
        window.ansagenNachDemLaden.push(region.textContent);
      }
    }).observe(document, { childList: true, characterData: true, subtree: true });
  });
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  const cdp = await seite.page.context().newCDPSession(seite.page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: bytesJeSekunde });
  const dialoge = [];
  seite.page.on('dialog', async (dialog) => {
    dialoge.push(dialog.type());
    await dialog.accept();
  });
  return { ...seite, dialoge, anfragen: anfragenMitschreiben(seite.page) };
}

/// Sendet `text` mit `dateien` und wartet, bis der Eintrag „wird hochgeladen …“ steht.
async function sendenMitDateien(page, text, dateien) {
  await eingabe(page).fill(text);
  await waehlen(page, dateien);
  await knopf(page).click();
  await expect(eintrag(page)).toContainText('wird hochgeladen …');
}

async function neuLaden(page) {
  await page.reload();
  await expect(eingabe(page)).toBeVisible();
}

const ansagen = (page) => page.evaluate(() => window.ansagenNachDemLaden);

test('Neuladen während des Hochladens: Rückfrage, danach Text im Feld, Hinweis und Ansage; der Tab-Speicher hält nur Text und Name', async ({ browser }) => {
  const { page, fehler, dialoge } = await seiteAuf(browser);
  await sendenMitDateien(page, 'Bericht anbei', [berichtPdf()]);
  const sitzungsToken = await page.evaluate(() => window.localStorage.getItem('localchatA.sitzung'));

  await neuLaden(page);

  expect(dialoge).toEqual(['beforeunload']);
  const HINWEIS = 'Hochladen unterbrochen — bericht.pdf bitte erneut wählen.';
  await expect(eingabe(page)).toHaveValue('Bericht anbei');
  await expect(hinweiszeile(page)).toHaveText(HINWEIS);
  await expect.poll(() => ansagen(page)).toContainEqual(expect.stringContaining(HINWEIS));
  await expect(eintrag(page)).toHaveCount(0);
  const gemerkt = (await page.evaluate(() => window.speicherBeimLaden))[SCHLUESSEL];
  expect(gemerkt).toContain('bericht.pdf');
  expect(gemerkt.length).toBeLessThan(1024);
  expect(gemerkt).not.toMatch(/[a-z2-7]{26}/);
  expect(gemerkt).not.toContain(sitzungsToken);
  expect(await page.evaluate((schluessel) => window.sessionStorage.getItem(schluessel), SCHLUESSEL)).toBeNull();

  await eingabe(page).press('Enter');
  await expect(eingabe(page)).toHaveValue('');
  await expect(hinweiszeile(page)).toHaveText('');

  await neuLaden(page);
  expect(dialoge).toEqual(['beforeunload']);
  await expect(hinweiszeile(page)).toHaveText('');
  expect(fehler).toEqual([]);
});

test('Ohne offenen Eintrag fragt der Browser beim Neuladen nicht nach', async ({ browser }) => {
  const { page, fehler, dialoge } = await seiteAuf(browser);
  await eingabe(page).fill('nur getippt');

  await neuLaden(page);

  expect(dialoge).toEqual([]);
  await expect(hinweiszeile(page)).toHaveText('');
  expect(fehler).toEqual([]);
});

test('Neuladen mit nicht leerem Feld: gesicherter Text, Leerzeile, Entwurf; der Hinweis endet mit „Text steht oben im Feld.“', async ({ browser }) => {
  const { page, fehler, dialoge } = await seiteAuf(browser);
  await sendenMitDateien(page, 'Bericht anbei', [berichtPdf()]);
  await eingabe(page).fill('Neuer Entwurf');

  await neuLaden(page);

  expect(dialoge).toEqual(['beforeunload']);
  const HINWEIS = 'Hochladen unterbrochen — bericht.pdf bitte erneut wählen. Text steht oben im Feld.';
  await expect(eingabe(page)).toHaveValue('Bericht anbei\n\nNeuer Entwurf');
  await expect(hinweiszeile(page)).toHaveText(HINWEIS);
  await expect.poll(() => ansagen(page)).toContainEqual(expect.stringContaining(HINWEIS));
  expect(fehler).toEqual([]);
});

test('Namen im Hinweis: 5 Dateien → die ersten drei und „und 2 weitere“', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser);
  const MIB = 1024 * 1024;
  await sendenMitDateien(page, '', ['a.pdf', 'b.png', 'c.txt', 'd.txt', 'e.txt'].map((name) => textDatei(name, MIB)));

  await neuLaden(page);

  const HINWEIS = 'Hochladen unterbrochen — a.pdf, b.png, c.txt und 2 weitere bitte erneut wählen.';
  await expect(hinweiszeile(page)).toHaveText(HINWEIS);
  await expect.poll(() => ansagen(page)).toContainEqual(expect.stringContaining(HINWEIS));
  await expect(eingabe(page)).toHaveValue('');
  expect(fehler).toEqual([]);
});

test('Ein Name mit Markup erscheint in Hinweis und Ansage nur als Text', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, { zusaetzlich: { '/api/anhaenge': [422] } });
  const NAME = '<img src=x onerror=xs=1>.png';
  await sendenMitDateien(page, 'Bild', [fotoPng(NAME)]);

  await neuLaden(page);

  await expect(hinweiszeile(page)).toHaveText(`Hochladen unterbrochen — ${NAME} bitte erneut wählen.`);
  await expect.poll(() => ansagen(page)).toContainEqual(expect.stringContaining(NAME));
  await expect(hinweiszeile(page).locator('img')).toHaveCount(0);
  await expect(page.locator('.lc-ansage img')).toHaveCount(0);
  expect(await page.evaluate(() => window.xs)).toBeUndefined();
  expect(fehler).toEqual([]);
});

test('Ein kaputter oder übergroßer Eintrag im Tab-Speicher wird ohne Fehler übergangen und entfernt', async ({ browser }) => {
  const { page, fehler, dialoge } = await seiteAuf(browser);
  await page.evaluate(() => {
    window.sessionStorage.setItem('localchatA.unterbrochen.allgemein', '{kaputt');
    window.sessionStorage.setItem('localchatA.unterbrochen.technik', JSON.stringify({ text: 'x'.repeat(100 * 1024), namen: ['a.pdf'] }));
  });

  await neuLaden(page);

  expect(dialoge).toEqual([]);
  await expect(eingabe(page)).toHaveValue('');
  await expect(hinweiszeile(page)).toHaveText('');
  await seitenleiste(page).getByRole('link', { name: 'technik', exact: true }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(eingabe(page)).toHaveValue('');
  await expect(hinweiszeile(page)).toHaveText('');
  const uebrig = await page.evaluate(() => Object.keys(window.sessionStorage).filter((schluessel) => schluessel.includes('unterbrochen')));
  expect(uebrig).toEqual([]);
  expect(fehler).toEqual([]);
});

test('Nur im Channel der Nachricht: in # technik kein Hinweis, zurück in # allgemein steht er, ohne den Fokus zu bewegen, bis zum Anhängen', async ({ browser }) => {
  const { page, fehler, dialoge } = await seiteAuf(browser);
  await sendenMitDateien(page, 'Bericht anbei', [berichtPdf()]);
  await seitenleiste(page).getByRole('link', { name: 'technik', exact: true }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');

  await neuLaden(page);

  expect(dialoge).toEqual(['beforeunload']);
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(eingabe(page)).toHaveValue('');
  await expect(hinweiszeile(page)).toHaveText('');
  const allgemein = seitenleiste(page).getByRole('link', { name: 'allgemein', exact: true });
  await allgemein.click();
  const HINWEIS = 'Hochladen unterbrochen — bericht.pdf bitte erneut wählen.';
  await expect(hinweiszeile(page)).toHaveText(HINWEIS);
  await expect(allgemein).toBeFocused();
  await expect(eingabe(page)).toHaveValue('Bericht anbei');

  await seitenleiste(page).getByRole('link', { name: 'technik', exact: true }).click();
  await expect(hinweiszeile(page)).toHaveText('');
  await allgemein.click();
  await expect(hinweiszeile(page)).toHaveText(HINWEIS);

  await waehlen(page, [fotoPng()]);
  await expect(hinweiszeile(page)).not.toContainText('Hochladen unterbrochen');
  expect(fehler).toEqual([]);
});

// Der 401-Weg (Gate #9, Karte #43 F0101 „401 verliert laufende Hochladungen“): eine Sendung bekommt 401, der stille
// Beitritt 409 → der Beitreten-Bildschirm; die Hochladung hält an und wird gemerkt wie beim Neuladen. Nach dem Beitritt
// unter anderem Namen steht der Text im Feld und der Hinweis da; die Nachricht mit dem Anhang geht nie hinaus.
test('Hauptansicht weicht dem Beitreten-Bildschirm: die Hochladung hält an, nach dem Beitritt Text und Hinweis wie nach dem Neuladen', async ({ browser }) => {
  // Rund um den Beitreten-Bildschirm kann eine Meldung `aktivitaet` mit 401 enden (beobachtet in 1 von 40 Läufen unter
  // Last, Ursache nicht untersucht; kein Gegenstand dieses Tests).
  const zusaetzlich = { [NACHRICHTEN]: [401], [BEITRETEN]: [409], '/api/channels/allgemein/aktivitaet': [401] };
  const { page, fehler, anfragen } = await seiteAuf(browser, { zusaetzlich });
  await sendenMitDateien(page, 'Bericht anbei', [berichtPdf()]);
  await page.route(`**${NACHRICHTEN}`, (route) => route.fulfill({
    status: 401, contentType: 'application/json', body: JSON.stringify({ fehler: 'Sitzung unbekannt.' }),
  }));
  await page.route(`**${BEITRETEN}`, (route) => route.fulfill({
    status: 409,
    contentType: 'application/json',
    body: JSON.stringify({ fehler: 'Der Name shoff ist vergeben.', belegt_von: '127.0.0.9', seit: '2026-10-28T15:00:00+01:00', vorschlag: 'shoff-2' }),
  }));
  await eingabe(page).fill('eins');
  await eingabe(page).press('Enter');
  await expect(page.getByLabel('Name')).toBeVisible();
  await page.unroute(`**${BEITRETEN}`);
  await page.unroute(`**${NACHRICHTEN}`);

  await page.getByLabel('Name').fill('shoff-2');
  await page.getByLabel('Name').press('Enter');

  await expect(eingabe(page)).toHaveValue('Bericht anbei');
  await expect(hinweiszeile(page)).toHaveText('Hochladen unterbrochen — bericht.pdf bitte erneut wählen.');
  await expect(eintrag(page)).toHaveCount(0);
  await page.waitForTimeout(500);
  const mitAnhang = anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten') && anfrage.koerper.anhaenge !== undefined);
  expect(mitAnhang).toEqual([]);
  expect(fehler).toEqual([]);
});
