'use strict';
// R00007 Messwerte für den Entwicklertest (kein Gate): M2 — Median aus 20 Öffnungen von `gross` ohne Drosselung
// (Grenze: Median ≤ 100 ms); M1 — Mausrad über 2.000 Nachrichten nach oben (Grenze: höchstens 2 % der Frames über
// 50 ms, keine Aufgabe über 250 ms). Vorher: ruhiger Rechner (uptime unter 2). Die Werte stehen in der Ausgabe.
const { test } = require('@playwright/test');
const { instanzStarten, grossAnlegen, channelAnlegen } = require('../tests/hilfen');
const { juengsteZeit, neuesteSichtbarNachKlick, ankerBeobachten, ruhig, framesBeimScrollen } = require('../tests/verlauf-hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';

let instanz;
test.beforeAll(async () => {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      grossAnlegen(ordner);
      channelAnlegen(ordner, 'leer');
    },
  });
});
test.afterAll(async () => instanz.stoppen());

async function beitreten(page, name) {
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'gross', exact: true }).waitFor();
}

test('M2: 20 Öffnungen von gross', async ({ page }) => {
  await beitreten(page, 'shoff');
  const juengste = await juengsteZeit(instanz.adresse, 'gross');
  const dauern = [];
  for (let runde = 0; runde < 20; runde += 1) {
    await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'leer', exact: true }).click();
    await page.getByRole('log').getByText('Noch keine Nachrichten in # leer.').waitFor();
    dauern.push(await neuesteSichtbarNachKlick(page, 'gross', juengste));
  }
  dauern.sort((links, rechts) => links - rechts);
  console.log(`messung m2_ms: min ${dauern[0]} median ${(dauern[9] + dauern[10]) / 2} max ${dauern[19]} — ${dauern.join(' ')}`);
});

test('M1: Mausrad über 2.000 Nachrichten nach oben', async ({ page }) => {
  await beitreten(page, 'mia');
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'gross', exact: true }).click();
  await page.getByRole('log').locator('.lc-msg').nth(99).waitFor();
  await ruhig(page);
  await ankerBeobachten(page);

  const werte = await framesBeimScrollen(page, 2000);

  console.log(`messung m1: ${werte.frames} Frames, über 50 ms ${werte.ueber50} (${(werte.anteilUeber50 * 100).toFixed(2)} %), lange Aufgaben ${werte.aufgaben}, längste ${werte.laengsteAufgabe.toFixed(0)} ms`);
});
