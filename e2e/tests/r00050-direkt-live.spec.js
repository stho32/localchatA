'use strict';
// R00050 F0178 B0787 (Karte #13): live im Direktgespräch — zwei Browser `anna` und `ben` im Gespräch miteinander:
// Nachricht binnen 1 s ab der 201 (K2), Tippen nur beim Gegenüber, Bearbeiten, Löschen, Antworten mit Zitat und ein
// Anhang mit Player wie im Channel, Nachladen und das Tastenkürzel ↑; Senden an ein Gegenüber, das offline ist, geht
// ohne Zustellzusage, 404 `empfaenger`, 429 und 507 bleiben als ⚠ mit dem Servertext ohne Ordnernamen (V14, V15).
// Mutanten, die rot werden müssen: Ereignisse mit `direkt` werden im Verteiler verworfen; Tippzeile zeigt das eigene
// Tippen; 404 `empfaenger` verschwindet; Servertext der 429/507 nennt den Ordnernamen. Gegen das echte Programm.
const path = require('path');
const { execFileSync } = require('child_process');
const { test, expect } = require('./rahmen');
const { instanzStarten, channelAnlegen, sichtbarNach } = require('./hilfen');
const { JETZT, sendenStarten, beitreten, imBrowserBeitreten, mitText } = require('./senden-hilfen');
const { browserOeffnen, nachrichtMitZeit, zeigtBearbeitet, zeigtGeloescht, webm } = require('./r00041-hilfen');
const { zeigtTipp } = require('./r00045-hilfen');
const {
  BEN, tippenOhneGespraech, direktMitAnhaengenSenden, direktLesen, direktOeffnen, verlaufMit, eingabeAn, gespraechOffen,
} = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ANNA_B = 'anna@127.0.0.1';
const BEN_B = 'ben@127.0.0.1';
const ZUSTELLUNG_MS = 1000;
const QUOTE_ORDNER = path.resolve(__dirname, '../../target/debug/examples/quote-ordner');
const vorlaeufige = (page) => verlaufMit(page, BEN).locator('.lc-msg.lc-vorlaeufig');
const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));

/// Zwei Browser im Gespräch miteinander: anna in #/d/ben@127.0.0.1, ben in #/d/anna@127.0.0.1. → { anna, ben }
async function zweiImGespraech(browser) {
  instanz = await sendenStarten();
  const anna = await browserOeffnen(browser, { zusaetzlich: tippenOhneGespraech(BEN_B) });
  const ben = await browserOeffnen(browser, { zusaetzlich: tippenOhneGespraech(ANNA_B) });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');
  await direktOeffnen(anna.page, BEN_B);
  await gespraechOffen(anna.page, BEN_B);
  await direktOeffnen(ben.page, ANNA_B);
  await gespraechOffen(ben.page, ANNA_B);
  return { anna, ben };
}

/// Sendet `text` im Browser mit Enter; → { zeit, ab } — ab: Ankunft der 201 im Testprozess.
async function imGespraechSenden(page, gegenueber, text) {
  const antwort = page.waitForResponse((empfangen) => empfangen.request().method() === 'POST'
    && new URL(empfangen.url()).pathname === `/api/direkt/${encodeURIComponent(gegenueber)}/nachrichten`);
  await eingabeAn(page, gegenueber).fill(text);
  await eingabeAn(page, gegenueber).press('Enter');
  const empfangen = await antwort;
  const ab = Date.now();
  expect(empfangen.status()).toBe(201);
  return { zeit: (await empfangen.json()).nachricht.zeit, ab };
}

function zeigtText({ text }) {
  return [...document.querySelectorAll('.lc-log .lc-msg:not(.lc-vorlaeufig) .lc-body')].some((body) => body.textContent === text);
}

test('Live (B0787, K2, US-22, US-23): Nachricht binnen 1 s, Tippen nur bei ben, Bearbeiten, Antworten mit Zitat, Löschen und ↑ wie im Channel', async ({ browser }) => {
  const { anna, ben } = await zweiImGespraech(browser);

  const hallo = await imGespraechSenden(anna.page, BEN_B, 'Hallo');
  const ms = await sichtbarNach(ben.page, zeigtText, { argument: { text: 'Hallo' }, ab: hallo.ab });
  expect(ms, 'Zustellung bei ben ab der 201').toBeLessThanOrEqual(ZUSTELLUNG_MS);

  const tippenAb = Date.now();
  await eingabeAn(anna.page, BEN_B).pressSequentially('x');
  const tippt = await sichtbarNach(ben.page, zeigtTipp, { argument: 'anna tippt …', ab: tippenAb });
  expect(tippt, 'Tippen bei ben').toBeLessThanOrEqual(ZUSTELLUNG_MS);
  await expect(anna.page.locator('.lc-comp .lc-comp-tipp')).toHaveText('');
  await eingabeAn(anna.page, BEN_B).fill('');

  await nachrichtMitZeit(anna.page, hallo.zeit).hover();
  await nachrichtMitZeit(anna.page, hallo.zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).click();
  await nachrichtMitZeit(anna.page, hallo.zeit).getByRole('textbox', { name: 'Nachricht bearbeiten' }).fill('Hallo ben');
  await nachrichtMitZeit(anna.page, hallo.zeit).getByRole('textbox', { name: 'Nachricht bearbeiten' }).press('Enter');
  await sichtbarNach(ben.page, zeigtBearbeitet, { argument: { zeit: hallo.zeit, text: 'Hallo ben' }, ab: Date.now() });

  await nachrichtMitZeit(ben.page, hallo.zeit).hover();
  await nachrichtMitZeit(ben.page, hallo.zeit).getByRole('button', { name: 'Antworten', exact: true }).click();
  const antwort = await imGespraechSenden(ben.page, ANNA_B, 'Hallo anna');
  await expect(nachrichtMitZeit(anna.page, antwort.zeit).locator('.lc-zitat')).toContainText('Hallo ben');

  await eingabeAn(anna.page, BEN_B).focus();
  await anna.page.keyboard.press('ArrowUp');
  await expect(nachrichtMitZeit(anna.page, hallo.zeit)).toBeFocused();

  await nachrichtMitZeit(anna.page, hallo.zeit).hover();
  await nachrichtMitZeit(anna.page, hallo.zeit).getByRole('button', { name: 'Löschen', exact: true }).click();
  await anna.page.getByRole('dialog').getByRole('button', { name: 'Löschen', exact: true }).click();
  await sichtbarNach(ben.page, zeigtGeloescht, { argument: { zeit: hallo.zeit }, ab: Date.now() });

  await ben.page.reload();
  await gespraechOffen(ben.page, ANNA_B);
  await expect(mitText(ben.page, /^Hallo anna$/)).toHaveCount(1);
  await expect(nachrichtMitZeit(ben.page, hallo.zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Anhang mit Player (B0787, US-22): ben schickt anna ein Video ins Direktgespräch — anna sieht den Player, auch nach Neuladen', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await browserOeffnen(browser);
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  const benToken = await beitreten(instanz.adresse, 'ben', { art: 'mensch', herkunft: '127.0.0.3' });
  await direktOeffnen(anna.page, BEN);
  await gespraechOffen(anna.page, BEN);

  const zeit = await direktMitAnhaengenSenden(instanz.adresse, benToken, ANNA_B, 'Film', [{ name: 'film.webm', inhalt: webm() }], '127.0.0.3');

  const video = nachrichtMitZeit(anna.page, zeit).locator('video[aria-label="film.webm"]');
  await expect(video).toHaveCount(1, { timeout: 2000 });
  await anna.page.reload();
  await gespraechOffen(anna.page, BEN);
  await expect(video).toHaveCount(1);
  expect(anna.fehler).toEqual([]);
});

test('Offline und unbekannt (B0787, V14, US-24): an ben ohne Strom geht es ohne Zustellzusage; 404 empfaenger bleibt als ⚠ mit dem Servertext', async ({ browser }) => {
  instanz = await sendenStarten();
  const zed = 'zed@127.0.0.7';
  const anna = await browserOeffnen(browser, {
    zusaetzlich: { ...tippenOhneGespraech(BEN, zed), [`/api/direkt/${encodeURIComponent(zed)}/nachrichten`]: [404] },
  });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  const benToken = await beitreten(instanz.adresse, 'ben', { art: 'mensch', herkunft: '127.0.0.3' });

  await direktOeffnen(anna.page, BEN);
  await gespraechOffen(anna.page, BEN);
  await imGespraechSenden(anna.page, BEN, 'Bist du da?');
  await expect(verlaufMit(anna.page, BEN).locator('.lc-msg', { hasText: 'Bist du da?' })).not.toContainText(/zugestellt|gelesen/i);
  expect(await direktLesen(instanz.adresse, benToken, ANNA_B, '127.0.0.3')).toEqual(['Bist du da?']);

  await direktOeffnen(anna.page, zed);
  await gespraechOffen(anna.page, zed);
  await eingabeAn(anna.page, zed).fill('Hallo zed');
  await eingabeAn(anna.page, zed).press('Enter');
  const fehlgeschlagen = verlaufMit(anna.page, zed).locator('.lc-msg.lc-vorlaeufig');
  const text = 'Diesen Teilnehmer gibt es gerade nicht — ein neues Gespräch beginnt nur mit jemandem aus der Teilnehmerliste.';
  await expect(fehlgeschlagen.locator('.lc-send')).toHaveText(`⚠ ${text}`);
  await warten(3000);
  await expect(fehlgeschlagen.locator('.lc-send')).toHaveText(`⚠ ${text}`);
  await expect(fehlgeschlagen.getByRole('button', { name: 'Erneut senden' })).toBeVisible();
  expect(anna.fehler).toEqual([]);
});

test('Quote (B0787, V15): 507 an der Plattenquote steht als ⚠ mit dem Servertext ohne Ordnernamen an der Nachricht', async ({ browser }) => {
  instanz = await instanzStarten({
    argumente: ['--plattenquote', '1MiB'],
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      channelAnlegen(ordner, 'allgemein');
      execFileSync(QUOTE_ORDNER, [ordner, 'allgemein', '1048576'], {
        stdio: 'ignore', env: { ...process.env, TZ: 'Europe/Berlin', LOCALCHATA_JETZT: JETZT },
      });
    },
  });
  const anna = await browserOeffnen(browser, {
    zusaetzlich: { ...tippenOhneGespraech(BEN), [`/api/direkt/${encodeURIComponent(BEN)}/nachrichten`]: [507] },
  });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await beitreten(instanz.adresse, 'ben', { art: 'mensch', herkunft: '127.0.0.3' });
  await direktOeffnen(anna.page, BEN);
  await gespraechOffen(anna.page, BEN);

  await eingabeAn(anna.page, BEN).fill('Hallo ben');
  await eingabeAn(anna.page, BEN).press('Enter');

  const grund = vorlaeufige(anna.page).locator('.lc-send');
  await expect(grund).toHaveText(/^⚠ Nicht gesendet: Die Gespräche belegen .* --plattenquote ein\.$/);
  await expect(grund).not.toContainText(/direkt\/|\+|127\.0\.0\.3|ben@/);
  expect(anna.fehler).toEqual([]);
});

test('Grenze neuer Gespräche (B0787, V15): eine 429 `direktgespraeche` steht als ⚠ mit dem Servertext an der Nachricht', async ({ browser }) => {
  instanz = await sendenStarten();
  const pfad = `/api/direkt/${encodeURIComponent(BEN)}/nachrichten`;
  const anna = await browserOeffnen(browser, { zusaetzlich: { ...tippenOhneGespraech(BEN), [pfad]: [429] } });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await beitreten(instanz.adresse, 'ben', { art: 'mensch', herkunft: '127.0.0.3' });
  const text = 'Zu viele neue Direktgespräche: höchstens 4096 je Rechner — bestehende Gespräche nutzen; Platz entsteht, wenn '
    + 'ein Gespräch nach seiner Frist entfällt.';
  await anna.page.route(`**${pfad}`, (anfrage) => anfrage.fulfill({
    status: 429, contentType: 'application/json',
    body: JSON.stringify({ fehler: text, grenze: 'direktgespraeche', art: 'herkunft', hoechstens: 4096 }),
  }));
  await direktOeffnen(anna.page, BEN);
  await gespraechOffen(anna.page, BEN);

  await eingabeAn(anna.page, BEN).fill('Hallo ben');
  await eingabeAn(anna.page, BEN).press('Enter');

  await expect(vorlaeufige(anna.page).locator('.lc-send')).toHaveText(`⚠ ${text}`);
  expect(anna.fehler).toEqual([]);
});
