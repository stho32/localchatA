'use strict';
// R00019 F0083 B0389 „Leerzeichen abgewiesen“ (ADR-0033 Ergänzung Karte #27 B0389): das Feld „Aufbewahrungsfrist“ ist ein
// Textfeld, damit der Rohtext zu sehen ist; getippt wird mit der Tastatur, nicht mit fill (fill umgeht die Eingabe).
const fs = require('fs');
const path = require('path');
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln } = require('./hilfen');

const JETZT = '2026-10-28T12:00:00.000+01:00';
const ROUTE = '/api/channels/allgemein/frist';
// Ab dem zweiten Fehlversuch hängt „ (n. Versuch)“ an (R00019 B0541).
const FEHLERZEILE = /^⚠ Frist: ganze Zahl von 1 bis 3650 Tagen\.( \(\d+\. Versuch\))?$/;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const dialog = (page) => page.getByRole('dialog');
const feld = (page) => page.getByRole('textbox', { name: 'Aufbewahrungsfrist' });

async function dialogOeffnen(browser) {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }),
  });
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await kontext.newPage();
  const fehler = fehlerSammeln(page);
  const abrufe = [];
  page.on('request', (anfrage) => {
    if (anfrage.url().endsWith(ROUTE)) {
      abrufe.push(anfrage.method());
    }
  });
  await page.goto(`${instanz.adresse}#/c/allgemein`);
  await page.getByLabel('Name').fill('abcdefghijklm');
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText('abcdefghijklm@');
  await page.getByRole('banner').getByRole('button', { name: '# allgemein — Channel-Einstellungen' }).click();
  await expect(dialog(page)).toBeVisible();
  return { page, fehler, abrufe };
}

// Feld leeren und den Text Taste für Taste tippen.
async function tippen(page, text) {
  await feld(page).focus();
  await page.keyboard.press('Control+A');
  await page.keyboard.press('Delete');
  await page.keyboard.type(text);
}

test('Leerzeichen abgewiesen (B0389): Feldart Textfeld; „ 30“, „30 “, „3 0“ → Fehlerzeile, aria-invalid, kein Abruf, Rohtext bleibt; „30“ → 200', async ({ browser }) => {
  const a = await dialogOeffnen(browser);

  await expect(feld(a.page)).toHaveAttribute('type', 'text');
  await expect(feld(a.page)).toHaveAttribute('inputmode', 'numeric');
  await expect(feld(a.page)).toHaveAttribute('autocomplete', 'off');
  for (const name of ['min', 'max', 'step', 'pattern']) {
    await expect(feld(a.page)).not.toHaveAttribute(name, /.*/);
  }
  await expect(dialog(a.page).getByRole('spinbutton')).toHaveCount(0);

  for (const eingabe of [' 30', '30 ', '3 0']) {
    await tippen(a.page, eingabe);
    await feld(a.page).press('Enter');
    await expect(dialog(a.page).getByRole('alert'), JSON.stringify(eingabe)).toHaveText(FEHLERZEILE);
    await expect(feld(a.page)).toHaveAttribute('aria-invalid', 'true');
    await expect(feld(a.page)).toHaveValue(eingabe);
    expect(a.abrufe, JSON.stringify(eingabe)).toEqual([]);
  }

  await tippen(a.page, '30');
  await expect(feld(a.page)).not.toHaveAttribute('aria-invalid', /.*/);
  await expect(dialog(a.page).getByRole('alert')).toBeHidden();
  const antwort = a.page.waitForResponse((r) => r.url().endsWith(ROUTE));
  await feld(a.page).press('Enter');
  expect((await antwort).status()).toBe(200);
  await expect(a.page.locator('.lc-statuszeile')).toHaveText('Frist für # allgemein: 30 Tage.');
  expect(a.fehler).toEqual([]);
});
