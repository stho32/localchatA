'use strict';
// R00050 F0180 B0790 (Karte #13, V11, US-26): eine fremde Direktnachricht in einem nicht offenen Gespräch sagt die
// bestehende Live-Region binnen 2 s an — „Neue Direktnachricht von anna@127.0.0.1“ —, im offenen Gespräch ebenso ohne
// „#“; die eigene Nachricht nie; ein Dritter hört nichts; es bleiben genau zwei Live-Regionen. Mutanten, die rot werden
// müssen: eine dritte Live-Region; Ansage der eigenen Nachricht; „#“ im Text; cora hört die Ansage. Gegen das echte
// Programm.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');
const { ansageRegion } = require('./r00012-hilfen');
const { direktOeffnen, eingabeAn, gespraechOffen } = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ANNA_B = 'anna@127.0.0.1';
const BEN_B = 'ben@127.0.0.1';
const ANSAGE_MS = 2000;
const NACH_DEM_TAKT_MS = 12000;

/// Merkt im Tab jeden nicht leeren Text der Ansage-Region mit dem Zeitpunkt (Date.now()) — auch einen, der nur kurz steht.
async function ansagenMitschreiben(page) {
  await page.evaluate(() => {
    window.ansagen = [];
    const region = document.querySelector('[role="status"][aria-live="polite"]:not(.lc-tipp-ansage)');
    new MutationObserver(() => {
      if (region.textContent !== '') {
        window.ansagen.push({ text: region.textContent, um: Date.now() });
      }
    }).observe(region, { childList: true, characterData: true, subtree: true });
  });
  return () => page.evaluate(() => window.ansagen);
}

/// Live-Regionen, die etwas ansagen können: aria-live außer „off“, außerhalb geschlossener Dialoge (deren Hinweise sprechen
/// erst, wenn der Dialog offen ist).
const liveRegionen = (page) => ({
  count: () => page.evaluate(() => [...document.querySelectorAll('[aria-live]:not([aria-live="off"])')]
    .filter((region) => region.closest('dialog:not([open])') === null).length),
});

async function sendenAn(page, kennung, text) {
  const gesendet = page.waitForResponse((antwort) => antwort.request().method() === 'POST' && antwort.url().endsWith('/nachrichten'));
  await eingabeAn(page, kennung).fill(text);
  await eingabeAn(page, kennung).press('Enter');
  expect((await gesendet).status()).toBe(201);
  return Date.now();
}

test('Ansage (B0790, V11): ben in # allgemein hört binnen 2 s „Neue Direktnachricht von anna@…“, offen ohne „#“; cora und anna hören nichts', async ({ browser }) => {
  test.setTimeout(45000);
  instanz = await sendenStarten();
  const [anna, ben, cora] = [await browserOeffnen(browser), await browserOeffnen(browser), await browserOeffnen(browser)];
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');
  await imBrowserBeitreten(cora.page, instanz.adresse, 'cora');
  const regionenVorher = await Promise.all([anna, ben, cora].map(({ page }) => liveRegionen(page).count()));
  expect(regionenVorher).toEqual([2, 2, 2]);
  // Ein zweiter Tab von anna bekommt ihre Nachricht aus dem Strom wie eine fremde — angesagt werden darf sie dort nicht.
  const annaZwei = await anna.kontext.newPage();
  await annaZwei.goto(`${instanz.adresse}#/c/allgemein`);
  await expect(annaZwei.getByRole('log', { name: 'Verlauf # allgemein' })).toBeVisible();
  const gehoert = {
    anna: await ansagenMitschreiben(anna.page),
    annaZwei: await ansagenMitschreiben(annaZwei),
    ben: await ansagenMitschreiben(ben.page),
    cora: await ansagenMitschreiben(cora.page),
  };
  await direktOeffnen(anna.page, BEN_B);
  await gespraechOffen(anna.page, BEN_B);

  const ab = await sendenAn(anna.page, BEN_B, 'Hallo ben');
  await expect.poll(async () => (await gehoert.ben()).map(({ text }) => text), { intervals: [25], timeout: 5000 })
    .toContain(`Neue Direktnachricht von ${ANNA_B}`);
  const erste = (await gehoert.ben()).find(({ text }) => text.includes('Neue Direktnachricht'));
  expect(erste.um - ab, 'Ansage ab der 201').toBeLessThanOrEqual(ANSAGE_MS);

  await direktOeffnen(ben.page, ANNA_B);
  await gespraechOffen(ben.page, ANNA_B);
  await sendenAn(anna.page, BEN_B, 'Noch eine');
  await expect.poll(async () => (await gehoert.ben()).length, { intervals: [100], timeout: NACH_DEM_TAKT_MS }).toBeGreaterThan(1);
  const imOffenen = (await gehoert.ben()).slice(1).map(({ text }) => text);
  expect(imOffenen.join(' ')).toMatch(/Direktnachricht/);
  expect(imOffenen.join(' ')).not.toContain('#');

  expect((await gehoert.cora()).map(({ text }) => text).filter((text) => text.includes('Direktnachricht'))).toEqual([]);
  for (const eigene of [gehoert.anna, gehoert.annaZwei]) {
    expect((await eigene()).map(({ text }) => text).filter((text) => text.includes('Direktnachricht'))).toEqual([]);
  }
  const regionenNachher = await Promise.all([anna, ben, cora].map(({ page }) => liveRegionen(page).count()));
  expect(regionenNachher).toEqual([2, 2, 2]);
  await expect(ansageRegion(ben.page)).toHaveCount(1);
  expect([...anna.fehler, ...ben.fehler, ...cora.fehler]).toEqual([]);
});

// Nacharbeit F0180 (code-reviewer #13, Sollte 1): ben hört „Neue Direktnachricht von anna“, bekommt binnen 10 s eine
// zweite (gesammelt) und öffnet das Gespräch binnen 3 s — die gesammelte Ansage entfällt, er hat die Nachricht vor Augen.
// Mutante: „gewechselt“ trägt den Kanal nicht → nach 10 s eine zweite Ansage, rot.
test('Ansage (V11, Nacharbeit): eine gesammelte Ansage entfällt, wenn ben das Gespräch binnen 3 s öffnet', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await sendenStarten();
  const [anna, ben] = [await browserOeffnen(browser), await browserOeffnen(browser)];
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');
  const gehoert = await ansagenMitschreiben(ben.page);
  await direktOeffnen(anna.page, BEN_B);
  await gespraechOffen(anna.page, BEN_B);

  const ab = await sendenAn(anna.page, BEN_B, 'Erste');
  await expect.poll(async () => (await gehoert()).length, { intervals: [25], timeout: 5000 }).toBe(1);
  await sendenAn(anna.page, BEN_B, 'Zweite');
  await expect(ben.page.getByRole('navigation', { name: 'Direktnachrichten' }).getByText('anna')).toBeVisible();
  await direktOeffnen(ben.page, ANNA_B);
  await gespraechOffen(ben.page, ANNA_B);
  expect(Date.now() - ab, 'geöffnet binnen 10 s nach der ersten Ansage').toBeLessThan(10000);

  await ben.page.waitForTimeout(ab + 14000 - Date.now());
  const direktAnsagen = (await gehoert()).map(({ text }) => text).filter((text) => text.includes('Direktnachricht von anna'));
  expect(direktAnsagen).toEqual([`Neue Direktnachricht von ${ANNA_B}`]);
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});
