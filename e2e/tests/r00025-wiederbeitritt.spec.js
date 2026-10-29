'use strict';
// R00025 F0088 „Wiederbeitritt beim Senden auf dem Weg des Stroms“: gegen das echte Programm, Chromium 1280 × 800. A ist
// `shoff`; die erste Sendung beantwortet die Route mit 401, den stillen Beitritt mit 409 (Name vergeben). Ein Lauf und
// ein Weg prüfen die Node-Tests (e2e/einheit/wiederbeitritt.test.mjs, senden.test.mjs).
const { test, expect } = require('@playwright/test');
const { JETZT, sendenStarten, beitreten, kontextOeffnen, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { ROUTE, leiste, vorlaeufige, bestaetigt, saetzeMit, sendungenMitschreiben } = require('./r00012-hilfen');
const { instanzStarten, freierPort, channelAnlegen } = require('./hilfen');

let instanz;
let kontexte = [];
test.afterEach(async () => {
  await Promise.all(kontexte.map((kontext) => kontext.close()));
  kontexte = [];
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

// Nacharbeit #36, ux-reviewer M1 (Abnahme-Probe wörtlich): „Varianten (a) Strom meldet 401 zuerst (Instanz neu starten,
// während die Leiste mit Countdown steht, dann senden) und (b) Senden meldet sie zuerst; je Variante Name nach dem
// Neustart per API belegen, im Tab unter anderem Namen beitreten → „Erneut senden“ binnen 2 s genau ein POST
// …/nachrichten mit dem gespeicherten Schlüssel, „fünf“ einmal im Log; danach neue Nachricht → 201 binnen 2 s.“
// Echte 401 und echte 409 des Programms. In (b) liest der Tab Strom und Verlauf nach dem Neustart mit dem Token eines
// Helfers (Route tauscht nur GET-Köpfe, Strom im Tab statt im SharedWorker; bis der Helfer da ist, gilt der Server
// als weg) — so ist er verbunden, und erst das Senden trifft auf das unbekannte Token.
const STROM = '/api/ereignisse';
const GRENZE_MS = 2000;

async function neustartLassen(browser, { sendenZuerst }) {
  instanz = await instanzStarten({
    port: await freierPort(),
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein'),
  });
  const erwartet = { [BEITRETEN]: [409] };
  if (sendenZuerst) {
    erwartet[NACHRICHTEN] = [401];
  } else {
    erwartet[STROM] = [401];
  }
  const a = await kontextOeffnen(browser, { serverWegErwartet: true, zusaetzlich: erwartet });
  kontexte.push(a.kontext);
  let helferToken = null;
  if (sendenZuerst) {
    await a.kontext.addInitScript(() => { delete window.SharedWorker; });
  }
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const sendungen = sendungenMitschreiben(a.page);
  if (sendenZuerst) {
    await a.page.route('**/api/**', async (route) => {
      const anfrage = route.request();
      if (anfrage.method() !== 'GET') {
        await route.continue();
        return;
      }
      if (helferToken === null) {
        await route.abort('connectionrefused');
        return;
      }
      await route.continue({ headers: { ...anfrage.headers(), 'x-localchata-sitzung': helferToken } });
    });
  }

  await instanz.beenden();
  await expect(leiste(a.page)).toHaveText(/^⚠ Verbindung verloren/);
  await instanz.starten();
  await beitreten(instanz.adresse, 'shoff', { art: 'mensch', herkunft: '127.0.0.2' });
  if (sendenZuerst) {
    helferToken = await beitreten(instanz.adresse, 'helfer', { art: 'mensch' });
  }
  await eingabe(a.page).fill('fünf');
  await eingabe(a.page).press('Enter');

  await expect(a.page.getByLabel('Name')).toBeVisible({ timeout: 20000 });
  if (sendenZuerst) {
    await a.page.unroute('**/api/**');
  }
  return { a, sendungen };
}

for (const variante of [{ titel: '(a) Strom meldet 401 zuerst', sendenZuerst: false }, { titel: '(b) Senden meldet 401 zuerst', sendenZuerst: true }]) {
  test(`Scheitern nach Neustart ${variante.titel}: „Erneut senden“ und neue Nachricht gehen hinaus`, async ({ browser }) => {
    test.setTimeout(60000);
    const { a, sendungen } = await neustartLassen(browser, variante);
    const vorDemBeitritt = sendungen.length;

    await alsBeitreten(a.page, 'shoff-anders');
    await expect(vorlaeufige(a.page).locator('.lc-body')).toHaveText(['fünf']);
    const schluessel = await vorlaeufige(a.page).getAttribute('data-schluessel');
    const klick = Date.now();
    await vorlaeufige(a.page).getByRole('button', { name: 'Erneut senden' }).click();

    await expect.poll(() => sendungen.length, { intervals: [25], timeout: 10000 }).toBe(vorDemBeitritt + 1);
    expect(sendungen[vorDemBeitritt].beginn - klick).toBeLessThan(GRENZE_MS);
    expect(sendungen[vorDemBeitritt]).toMatchObject({ text: 'fünf', schluessel });
    await expect(bestaetigt(a.page, 'fünf')).toHaveCount(1);
    await a.page.waitForTimeout(300);
    expect(sendungen.length).toBe(vorDemBeitritt + 1);
    expect(saetzeMit(instanz.ordner, 'allgemein', 'fünf')).toBe(1);

    const antwort = a.page.waitForResponse((r) => r.request().method() === 'POST' && r.url().endsWith(NACHRICHTEN), { timeout: 10000 });
    const abgeschickt = Date.now();
    await eingabe(a.page).fill('sechs');
    await eingabe(a.page).press('Enter');
    expect((await antwort).status()).toBe(201);
    expect(Date.now() - abgeschickt).toBeLessThan(GRENZE_MS);
    await expect(bestaetigt(a.page, 'sechs')).toHaveCount(1);
    expect(a.fehler).toEqual([]);
  });
}
