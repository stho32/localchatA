'use strict';
// R00025 F0088 „Wiederbeitritt beim Senden auf dem Weg des Stroms“: gegen das echte Programm, Chromium 1280 × 800. A ist
// `shoff`; die erste Sendung beantwortet die Route mit 401, den stillen Beitritt mit 409 (Name vergeben). Ein Lauf und
// ein Weg prüfen die Node-Tests (e2e/einheit/wiederbeitritt.test.mjs, senden.test.mjs).
const { test, expect } = require('@playwright/test');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { ROUTE, vorlaeufige, bestaetigt, saetzeMit, sendungenMitschreiben } = require('./r00012-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const NACHRICHTEN = '/api/channels/allgemein/nachrichten';
const BEITRETEN = '/api/beitreten';
const GRUND = '⚠ Nicht gesendet — der Beitritt war abgelaufen.';

/// A sendet `eins`; die Sendung bekommt 401, der stille Beitritt wird gehalten, bis `halb` im Feld steht, und dann
/// mit 409 beantwortet → der Beitreten-Bildschirm steht. Liefert Seite und mitgeschriebene Sendungen.
async function scheiternLassen(browser, weitereTexte = []) {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { zusaetzlich: { [NACHRICHTEN]: [401], [BEITRETEN]: [409] } });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const sendungen = sendungenMitschreiben(a.page);
  let ersteSendung = true;
  await a.page.route(ROUTE, async (route) => {
    if (route.request().method() !== 'POST' || !ersteSendung) {
      await route.continue();
      return;
    }
    ersteSendung = false;
    await route.fulfill({ status: 401, contentType: 'application/json', body: JSON.stringify({ fehler: 'Sitzung unbekannt.' }) });
  });
  let beitrittBeantworten = null;
  const beitrittGehalten = new Promise((weiter) => { beitrittBeantworten = weiter; });
  await a.page.route(`**${BEITRETEN}`, async (route) => {
    await beitrittGehalten;
    await route.fulfill({
      status: 409,
      contentType: 'application/json',
      body: JSON.stringify({ fehler: 'Der Name shoff ist vergeben.', belegt_von: '127.0.0.9', seit: '2026-10-28T15:00:00+01:00', vorschlag: 'shoff-2' }),
    });
  });

  await eingabe(a.page).fill('eins');
  await eingabe(a.page).press('Enter');
  for (const text of weitereTexte) {
    await eingabe(a.page).fill(text);
    await eingabe(a.page).press('Enter');
  }
  await eingabe(a.page).fill('halb');
  await expect.poll(() => sendungen.length).toBe(1);
  beitrittBeantworten();
  await expect(a.page.getByLabel('Name')).toBeVisible();
  await a.page.unroute(`**${BEITRETEN}`);
  return { a, sendungen };
}

async function alsBeitreten(page, name) {
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(eingabe(page)).toBeVisible();
}

// „Scheitern“ (ADR-0012 Ergänzung Karte #36, erster Test, wörtlich: „401 auf das Senden, Beitritt scheitert →
// Beitreten-Bildschirm wie bei einer 401 im Strom; danach Beitritt unter anderem Namen → die Nachricht steht
// fehlgeschlagen mit „Erneut senden“; Klick → gesendet unter dem neuen Namen, genau ein Satz. Entwurf im Feld wie
// vorher.“) Vor dem Klick kein POST …/nachrichten (Mutante „Zustellung mit der Hauptansicht verworfen“ wird rot).
test('Scheitern: Beitreten-Bildschirm, danach steht die Nachricht fehlgeschlagen da und geht erst auf Klick', async ({ browser }) => {
  const { a, sendungen } = await scheiternLassen(browser);

  await alsBeitreten(a.page, 'shoff-2');

  await expect(vorlaeufige(a.page).locator('.lc-body')).toHaveText(['eins']);
  await expect(vorlaeufige(a.page).locator('.lc-send')).toHaveText([GRUND]);
  await expect(vorlaeufige(a.page).getByRole('button', { name: 'Erneut senden' })).toHaveCount(1);
  await expect(vorlaeufige(a.page).getByRole('button', { name: 'Verwerfen' })).toHaveCount(1);
  await expect(eingabe(a.page)).toHaveValue('halb');
  await a.page.waitForTimeout(300);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['eins']);

  await vorlaeufige(a.page).getByRole('button', { name: 'Erneut senden' }).click();

  await expect(bestaetigt(a.page, 'eins')).toHaveCount(1);
  await expect(vorlaeufige(a.page)).toHaveCount(0);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['eins', 'eins']);
  expect(sendungen[1].schluessel).toBe(sendungen[0].schluessel);
  expect(saetzeMit(instanz.ordner, 'allgemein', 'eins')).toBe(1);
  await expect(bestaetigt(a.page, 'eins').locator('.lc-id')).toContainText('shoff-2');
  expect(a.fehler).toEqual([]);
});

// „Neuladen auf dem Beitreten-Bildschirm“: wie „Scheitern“, aber vor dem Beitritt neu geladen → nach dem Beitritt steht
// keine der ungesendeten Nachrichten da, der Entwurf liegt im Feld.
test('Neuladen auf dem Beitreten-Bildschirm: ungesendete sind weg, der Entwurf bleibt', async ({ browser }) => {
  const { a } = await scheiternLassen(browser);

  await a.page.reload();
  await alsBeitreten(a.page, 'shoff-2');

  await expect(eingabe(a.page)).toHaveValue('halb');
  await a.page.waitForTimeout(300);
  await expect(vorlaeufige(a.page)).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});


// „Wartende Nachrichten genannt“ (V12): mit einer ungesendeten Nachricht nennt der Beitreten-Bildschirm sie, mit zweien
// die Mehrzahl; nach dem Beitritt ist die Zeile weg; ohne ungesendete Nachricht keine Zeile (Mutante „ohne Zeile“ rot).
test('Wartende genannt: eine ungesendete Nachricht, nach dem Beitritt keine Zeile mehr', async ({ browser }) => {
  const { a } = await scheiternLassen(browser);

  await expect(a.page.locator('.lc-wartende')).toHaveText('1 ungesendete Nachricht wartet auf den Beitritt — Neuladen verwirft sie.');
  await alsBeitreten(a.page, 'shoff-2');
  await expect(a.page.locator('.lc-wartende')).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

test('Wartende genannt: zwei ungesendete Nachrichten in der Mehrzahl', async ({ browser }) => {
  const { a } = await scheiternLassen(browser, ['zwei']);

  await expect(a.page.locator('.lc-wartende')).toHaveText('2 ungesendete Nachrichten warten auf den Beitritt — Neuladen verwirft sie.');
  expect(a.fehler).toEqual([]);
});

test('Wartende genannt: ohne ungesendete Nachricht keine Zeile', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await a.page.goto(instanz.adresse);

  await expect(a.page.getByLabel('Name')).toBeVisible();
  await expect(a.page.locator('.lc-wartende')).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});
