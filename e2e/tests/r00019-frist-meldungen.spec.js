'use strict';
// R00019 F0066 Frist-Meldungen deutsch mit Ausweg (US-6, US-13, US-14; ADR-0033 Ergänzung Karte #27): der Dialog
// „# name — Einstellungen“ zeigt Serverfehler ohne Systemtext und markiert das Feld nur bei Prüffehlern.
// Heute ist der 28.10.2026. Kriterien mit chmod laufen nicht als root.
const fs = require('fs');
const path = require('path');
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln } = require('./hilfen');

const JETZT = '2026-10-28T12:00:00.000+01:00';
const ROUTE = '/api/channels/allgemein/frist';
const SCHREIBFEHLER = '⚠ Frist nicht gesetzt — channels/allgemein/einstellungen.toml ließ sich nicht schreiben. '
  + 'Wer localchatA betreibt, muss das Schreibrecht des Ordners prüfen; danach erneut versuchen.';

let instanz;
let schreibgeschuetzt = null;
test.afterEach(async () => {
  if (schreibgeschuetzt) {
    fs.chmodSync(schreibgeschuetzt, 0o755);
    schreibgeschuetzt = null;
  }
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function starten() {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }),
  });
}

// US-6: der Ordner des Channels ist schreibgeschützt, die Einstellungsdatei lässt sich nicht anlegen.
function ordnerSchreibschuetzen() {
  schreibgeschuetzt = path.join(instanz.ordner, 'channels', 'allgemein');
  fs.chmodSync(schreibgeschuetzt, 0o555);
}

async function dialogOeffnen(browser, { zusaetzlich = { [ROUTE]: [500] } } = {}) {
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await kontext.newPage();
  const fehler = fehlerSammeln(page, { zusaetzlich });
  await page.goto(`${instanz.adresse}#/c/allgemein`);
  await page.getByLabel('Name').fill('abcdefghijklm');
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText('abcdefghijklm@');
  await page.getByRole('banner').getByRole('button', { name: '# allgemein — Channel-Einstellungen' }).click();
  await expect(dialog(page)).toBeVisible();
  return { page, fehler };
}

const dialog = (page) => page.getByRole('dialog');
const feld = (page) => dialog(page).getByLabel('Aufbewahrungsfrist');
const fehlerzeile = (page) => dialog(page).getByRole('alert');

async function speichern(page, tage) {
  await feld(page).fill(String(tage));
  const antwort = page.waitForResponse((r) => r.url().endsWith(ROUTE));
  await feld(page).press('Enter');
  return (await antwort).status();
}

async function feldUnmarkiertMitEingabe(page, eingabe) {
  await expect(feld(page)).not.toHaveAttribute('aria-invalid', /.*/);
  await expect(feld(page)).toHaveValue(eingabe);
  await expect(feld(page)).toBeFocused();
  await expect(dialog(page)).toBeVisible();
}

test('Serverfehler im Dialog (US-6, B0325): ⚠ Satz mit Ausweg ohne os error, Feld ohne aria-invalid; Gegenprobe 0 → aria-invalid', async ({ browser }) => {
  await starten();
  ordnerSchreibschuetzen();
  const a = await dialogOeffnen(browser);

  expect(await speichern(a.page, 30)).toBe(500);

  await expect(fehlerzeile(a.page)).toHaveText(SCHREIBFEHLER);
  await expect(fehlerzeile(a.page)).not.toContainText('os error');
  await expect(fehlerzeile(a.page)).not.toContainText('Permission');
  await feldUnmarkiertMitEingabe(a.page, '30');
  await feld(a.page).fill('0');
  await expect(feld(a.page)).toHaveAttribute('aria-invalid', 'true');
  expect(a.fehler).toEqual([]);
});

// Kriterium nach Nutzen: 502 ohne Körper und der Abbruch sind über die Instanz nicht herstellbar.
for (const [art, stellen, erwartet] of [
  ['502 ohne Körper', (route) => route.fulfill({ status: 502, body: '' }), '⚠ Nicht gespeichert (Status 502) — später erneut versuchen.'],
  ['404', (route) => route.fulfill({ status: 404, contentType: 'application/json', body: JSON.stringify({ fehler: '# allgemein gibt es nicht.' }) }), '⚠ # allgemein gibt es nicht.'],
  ['409 einstellungen-defekt', (route) => route.fulfill({
    status: 409,
    contentType: 'application/json',
    body: JSON.stringify({ regel: 'einstellungen-defekt', fehler: 'channels/allgemein/einstellungen.toml ist defekt; von Hand reparieren und neu starten.' }),
  }), '⚠ channels/allgemein/einstellungen.toml ist defekt; von Hand reparieren und neu starten.'],
  ['Abbruch', (route) => route.abort(), '⚠ Nicht gespeichert — Verbindung prüfen.'],
]) {
  test(`Fehlerarten im Dialog (B0325): ${art} → „${erwartet}“, Feld ohne aria-invalid, Eingabe und Fokus bleiben`, async ({ browser }) => {
    await starten();
    const a = await dialogOeffnen(browser, { zusaetzlich: { [ROUTE]: [502, 404, 409] } });
    await a.page.route(`**${ROUTE}`, stellen);

    await feld(a.page).fill('30');
    await feld(a.page).press('Enter');

    await expect(fehlerzeile(a.page)).toHaveText(erwartet);
    await feldUnmarkiertMitEingabe(a.page, '30');
    expect(a.fehler.filter((zeile) => !zeile.includes('net::ERR_FAILED'))).toEqual([]);
  });
}
