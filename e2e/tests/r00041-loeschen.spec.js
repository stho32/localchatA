'use strict';
const http = require('http');
// R00041 F0139 Eigene Nachricht im Browser löschen: Aktion und Rückfrage-Dialog (B0623, UX M5, PO 126), Löschen über
// die API (B0624), Ausnahme zu R00030 „Kein Lösch-Knopf“.
const { test, expect } = require('./rahmen');
const { sichtbarNach } = require('./hilfen');
const { sendenStarten, imBrowserBeitreten, beitreten, anfragen } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const {
  BETA, betaSendet, browserOeffnen, imBrowserSenden, nachrichtMitZeit, zeigtGeloescht, ansagenMitschreiben, mitAnhaengenSenden,
  PNG, pngMitBytes,
} = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const LOESCHEN = '/api/channels/allgemein/nachrichten/loeschen';
const dialog = (page) => page.getByRole('dialog');
const imDialog = (page, name) => dialog(page).getByRole('button', { name, exact: true });

function loeschungenMitschreiben(page) {
  const liste = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && new URL(anfrage.url()).pathname === LOESCHEN) {
      liste.push(anfrage.postData());
    }
  });
  return liste;
}

test('Rückfrage am Desktop (B0623): Hover → „Löschen“ → modaler Dialog „Diese Nachricht löschen?“ mit „Abbrechen“ · „Löschen“; „Abbrechen“ schließt ohne POST', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const loeschungen = loeschungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).hover();
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Löschen', exact: true }).click();

  await expect(dialog(a.page)).toBeVisible();
  expect(await a.page.evaluate(() => document.querySelector('dialog[open]').matches(':modal'))).toBe(true);
  await expect(dialog(a.page)).toHaveAccessibleName('Diese Nachricht löschen?');
  await expect(dialog(a.page)).toContainText('Das lässt sich nicht rückgängig machen.');
  await expect(dialog(a.page)).toHaveAccessibleDescription(/Das lässt sich nicht rückgängig machen\./);
  await expect(dialog(a.page)).toContainText(/\d\d:\d\d: Hallo/);
  await expect(dialog(a.page).getByRole('button')).toHaveText(['Abbrechen', 'Löschen']);
  await imDialog(a.page, 'Abbrechen').click();
  await expect(dialog(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
  expect(loeschungen).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Tastatur (B0623, UX M5): Entf öffnet, Fokus auf „Abbrechen“ und bleibt im Dialog; Enter sofort und Esc schließen ohne POST, Fokus zurück; Strg Entf und Entf an fremden öffnen nichts', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const loeschungen = loeschungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).focus();

  await a.page.keyboard.press('Delete');
  await expect(imDialog(a.page, 'Abbrechen')).toBeFocused();
  for (const taste of ['Tab', 'Tab', 'Tab', 'Shift+Tab', 'Shift+Tab', 'Shift+Tab']) {
    await a.page.keyboard.press(taste);
    expect(await a.page.evaluate(() => document.querySelector('dialog[open]').contains(document.activeElement))).toBe(true);
  }
  await imDialog(a.page, 'Abbrechen').focus();
  await a.page.keyboard.press('Enter');
  await expect(dialog(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();

  await a.page.keyboard.press('Delete');
  await expect(dialog(a.page)).toBeVisible();
  await a.page.keyboard.press('Escape');
  await expect(dialog(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();

  await a.page.keyboard.press('Control+Delete');
  await a.page.waitForTimeout(200);
  await expect(dialog(a.page)).toHaveCount(0);
  const beta = await betaSendet(instanz.adresse, ['Fremd']);
  await nachrichtMitZeit(a.page, beta.zeiten[0]).focus();
  await a.page.keyboard.press('Delete');
  await a.page.waitForTimeout(200);
  await expect(dialog(a.page)).toHaveCount(0);
  expect(loeschungen).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Löschen am Desktop (B0624): Doppelklick auf „Löschen“ im Dialog → genau ein POST mit {"zeit"}; A und B zeigen „Nachricht gelöscht“, B unter 1 s; Fokus bei A auf der Nachricht; genau eine Ansage', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const b = await browserOeffnen(browser, { streng: true });
  const loeschungen = loeschungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await expect(nachrichtMitZeit(b.page, zeit)).toHaveCount(1);
  await a.page.waitForTimeout(300);
  const angesagt = await ansagenMitschreiben(a.page);
  await nachrichtMitZeit(a.page, zeit).hover();
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Löschen', exact: true }).click();

  const antwort = a.page.waitForResponse((ankunft) => new URL(ankunft.url()).pathname === LOESCHEN);
  await imDialog(a.page, 'Löschen').dblclick();
  expect((await antwort).status()).toBe(200);
  const bisB = await sichtbarNach(b.page, zeigtGeloescht, { argument: { zeit }, ab: Date.now(), timeout: 10000 });
  expect(bisB).toBeLessThan(1000);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
  await expect.poll(angesagt).toEqual(['Nachricht gelöscht.']);
  await a.page.waitForTimeout(300);
  expect(await angesagt()).toHaveLength(1);
  expect(loeschungen).toEqual([JSON.stringify({ zeit })]);
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('Löschen per Tastatur (B0624): Entf, Tab, Enter → genau ein POST, gelöscht', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const loeschungen = loeschungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).focus();
  await a.page.keyboard.press('Delete');
  await a.page.keyboard.press('Tab');
  await expect(imDialog(a.page, 'Löschen')).toBeFocused();
  await a.page.keyboard.press('Enter');
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect(loeschungen).toHaveLength(1);
  expect(a.fehler).toEqual([]);
});

test('Telefon (B0624): „Aktionen“ → „Löschen“ → Dialog → „Löschen“ → gelöscht; der Dialog passt in 375 × 812 und bei 320 px ohne Querscrollen', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await geraetOeffnen(browser, 'telefon', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Aktionen' }).tap();
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Löschen', exact: true }).tap();
  await expect(dialog(a.page)).toBeVisible();
  const passt = () => a.page.evaluate(() => {
    const rand = document.querySelector('dialog[open]').getBoundingClientRect();
    return rand.left >= 0 && rand.top >= 0 && rand.right <= window.innerWidth && rand.bottom <= window.innerHeight
      && document.documentElement.scrollWidth <= window.innerWidth;
  });
  expect(await passt()).toBe(true);
  await a.page.setViewportSize({ width: 320, height: 640 });
  expect(await passt()).toBe(true);
  for (const name of ['Abbrechen', 'Löschen']) {
    const box = await imDialog(a.page, name).boundingBox();
    expect(box.height).toBeGreaterThanOrEqual(44);
  }
  await imDialog(a.page, 'Löschen').tap();
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect(a.fehler).toEqual([]);
});

test('Mit Anhängen (B0624): der Dialog nennt „mit 2 Anhängen (2,6 MiB)“; danach antworten beide Abrufe 410 und anhang_belegt ist 0', async ({ browser }) => {
  instanz = await sendenStarten();
  const token = await beitreten(instanz.adresse, 'shoff', { art: 'mensch' });
  const ids = [];
  const zeit = await mitAnhaengenSenden(instanz.adresse, token, 'Bericht', [
    { name: 'foto.png', inhalt: pngMitBytes(307200) }, { name: 'bericht.pdf', inhalt: pdfMitBytes(2457600) },
  ], '127.0.0.1', ids);
  expect((await anfragen(instanz.adresse, { methode: 'POST', pfad: 'api/verlassen', token })).status).toBeLessThan(300);
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await nachrichtMitZeit(a.page, zeit).hover();
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Löschen', exact: true }).click();
  await expect(dialog(a.page)).toHaveAccessibleName('Diese Nachricht mit 2 Anhängen (2,6 MiB) löschen? Die Anhänge werden mitgelöscht.');
  await imDialog(a.page, 'Löschen').click();
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  for (const id of ids) {
    expect(await statusVon(instanz.adresse, `anhaenge/${id}`)).toBe(410);
  }
  const gesundheit = await anfragen(instanz.adresse, { methode: 'GET', pfad: 'api/gesundheit' });
  expect(gesundheit.daten.anhang_belegt).toBe(0);
  expect(a.fehler).toEqual([]);
});

test('Fehlschlag beim Löschen (B0624, S2): abgebrochener POST → Hinweiszeile „⚠ Keine Antwort vom Server beim Löschen.“, die Nachricht bleibt; ein zweites Löschen gelingt', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await a.page.route(`**${LOESCHEN}`, (route) => route.abort(), { times: 1 });
  await nachrichtMitZeit(a.page, zeit).focus();
  await a.page.keyboard.press('Delete');
  await imDialog(a.page, 'Löschen').click();
  await expect(a.page.locator('.lc-statuszeile')).toHaveText('⚠ Keine Antwort vom Server beim Löschen.');
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo');

  await nachrichtMitZeit(a.page, zeit).focus();
  await a.page.keyboard.press('Delete');
  await imDialog(a.page, 'Löschen').click();
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect(a.fehler.filter((zeile) => !/ERR_FAILED/.test(zeile))).toEqual([]);
});

test('Nur eigene; Ausnahme zu R00030 (B0624): an Nachrichten von anna und beta, auch mit Anhängen, kein Element mit „lösch“ im Namen; an der eigenen genau eines', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const b = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const eigene = await imBrowserSenden(a.page, 'Eigene');
  const vonAnna = await imBrowserSenden(b.page, 'Von Anna');
  const beta = await betaSendet(instanz.adresse, []);
  const mitDatei = await mitAnhaengenSenden(instanz.adresse, beta.token, 'Mit Datei', [{ name: 'foto.png', inhalt: PNG }], BETA);
  for (const zeit of [vonAnna, mitDatei]) {
    await nachrichtMitZeit(a.page, zeit).hover();
    await nachrichtMitZeit(a.page, zeit).focus();
    await expect(nachrichtMitZeit(a.page, zeit).getByRole('button', { name: /lösch/i })).toHaveCount(0);
    await expect(nachrichtMitZeit(a.page, zeit).getByRole('link', { name: /lösch/i })).toHaveCount(0);
  }
  await nachrichtMitZeit(a.page, eigene).hover();
  await expect(nachrichtMitZeit(a.page, eigene).getByRole('button', { name: /lösch/i })).toHaveCount(1);
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

/// Der Status eines GET ohne Sitzung (der Körper zählt nicht).
function statusVon(adresse, pfad) {
  return new Promise((fertig, gescheitert) => {
    http.get(new URL(pfad, adresse), (antwort) => {
      antwort.resume();
      fertig(antwort.statusCode);
    }).once('error', gescheitert);
  });
}

/// Ein PDF mit genau `bytes` Bytes (Kommentarzeile als Füllung).
function pdfMitBytes(bytes) {
  const kopf = '%PDF-1.4\n1 0 obj<<>>endobj\n';
  const ende = '%%EOF\n';
  return Buffer.from(`${kopf}%${'a'.repeat(bytes - kopf.length - ende.length - 2)}\n${ende}`);
}
