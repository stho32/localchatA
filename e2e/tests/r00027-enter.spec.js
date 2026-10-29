'use strict';
// R00027 F0092 Auf Touch-Geräten macht Enter eine neue Zeile (B0484, ADR-0038 Ergänzung Karte #41 K5, K6): Telefon,
// Tablet mit Tastatur, Desktop unverändert, Wechsel der Zeigerart ohne Neuladen (CDP), kein Sende-Hinweis. Konsole
// streng: keine einzige Fehlerzeile.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { bestaetigt, sendungenMitschreiben } = require('./r00012-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function seiteAuf(browser, geraet) {
  instanz = await sendenStarten();
  const seite = await geraetOeffnen(browser, geraet, { streng: true });
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  return seite;
}

const feldHatFokus = (page) => eingabe(page).evaluate((feld) => document.activeElement === feld);

/// Hallo, Enter, Welt auf einem Touch-Gerät → nichts gesendet, zwei Zeilen im Feld; Tippen auf den Knopf sendet beide.
async function zweiZeilenUndKnopf(page, sendungen) {
  await eingabe(page).tap();
  await page.keyboard.type('Hallo');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Welt');
  await page.waitForTimeout(300);
  expect(sendungen).toEqual([]);
  await expect(eingabe(page)).toHaveValue('Hallo\nWelt');

  await knopf(page).tap();
  await expect(bestaetigt(page, 'Hallo\nWelt')).toHaveCount(1);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['Hallo\nWelt']);
}

test('Telefon: Enter macht eine Zeile, der Knopf sendet beide Zeilen', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'telefon');
  const sendungen = sendungenMitschreiben(page);

  await zweiZeilenUndKnopf(page, sendungen);
  expect(fehler).toEqual([]);
});

test('Tablet mit Tastatur: Enter macht eine Zeile, Strg Enter sendet, der Fokus bleibt im Feld', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'tablet');
  const sendungen = sendungenMitschreiben(page);

  await zweiZeilenUndKnopf(page, sendungen);
  await page.keyboard.type('kurz');
  await page.keyboard.press('Control+Enter');

  await expect(bestaetigt(page, 'kurz')).toHaveCount(1);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['Hallo\nWelt', 'kurz']);
  expect(await feldHatFokus(page)).toBe(true);
  expect(fehler).toEqual([]);
});

test('Desktop unverändert: Enter sendet', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'desktop');
  const sendungen = sendungenMitschreiben(page);

  await eingabe(page).focus();
  await page.keyboard.type('Hallo');
  await page.keyboard.press('Enter');

  await expect(bestaetigt(page, 'Hallo')).toHaveCount(1);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['Hallo']);
  await expect(eingabe(page)).toHaveValue('');
  expect(fehler).toEqual([]);
});

// Geprüft wird das Verhalten, nicht die Medienabfrage: nach dem Ausschalten ist auch (pointer: fine) falsch (Zeiger
// „keiner“) — er gilt wie am Desktop (ADR-0038 Ergänzung Karte #41, Probe P2).
test('Wechsel der Zeigerart (Desktop-Kontext): Touch ein ohne Neuladen → Enter macht eine Zeile; aus → Enter sendet', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'desktop');
  const sendungen = sendungenMitschreiben(page);
  const cdp = await page.context().newCDPSession(page);

  await eingabe(page).focus();
  await page.keyboard.type('eins');
  await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });
  await page.keyboard.press('Enter');
  await page.keyboard.type('zwei');
  await page.waitForTimeout(300);
  expect(sendungen).toEqual([]);
  await expect(eingabe(page)).toHaveValue('eins\nzwei');

  await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: false });
  await page.keyboard.press('Enter');
  await expect(bestaetigt(page, 'eins\nzwei')).toHaveCount(1);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['eins\nzwei']);
  expect(fehler).toEqual([]);
});

test('Kein Sende-Hinweis (Telefon): das Feld trägt kein enterkeyhint="send"', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'telefon');

  await expect(eingabe(page)).toBeVisible();
  expect(await eingabe(page).getAttribute('enterkeyhint')).not.toBe('send');
  expect(fehler).toEqual([]);
});
