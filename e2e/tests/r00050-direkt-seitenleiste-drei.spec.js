'use strict';
// R00050 F0180 B0792 (Karte #13, ADR-0055 D12 Teil Seitenleiste, US-25, US-26): drei Browserkontexte gegen das echte
// Programm. anna schreibt ben → ben sieht den Eintrag in der Seitenleiste live und hört die Ansage binnen 2 s; cora sieht
// weder Eintrag noch Ansage, auch nach dem Neuladen und unter der Adresse des fremden Paars; ben auf dem Telefon (375 px,
// Touch) sieht drei Einträge mit Statuspunkt und Kürzel, ≥ 44 px hoch, mit vollem Namen — alphabetisch, nicht in der
// Reihenfolge der Nachrichten. Mutanten, die rot werden müssen: Seitenleiste listet fremde Gespräche; Hash eines fremden
// Paars öffnet Inhalt; Sortierung nach Aktivität.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, beitreten } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');
const { GERAETE } = require('./r00027-hilfen');
const { direktSenden, direktOeffnen, eingabeAn, gespraechOffen, verlaufMit } = require('./r00050-direkt-hilfen');

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
const GEHEIM = 'Nur für ben';
const direkt = (page) => page.getByRole('navigation', { name: 'Direktnachrichten' });
const eintraege = (page) => direkt(page).getByRole('link');

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

test('Drei Kontexte (B0792, D12): ben sieht den Eintrag live und hört die Ansage, cora nichts — auch nach Neuladen; ben am Telefon alphabetisch mit Kürzel', async ({ browser }) => {
  test.setTimeout(45000);
  instanz = await sendenStarten();
  const anna = await browserOeffnen(browser);
  const ben = await browserOeffnen(browser, { geraet: GERAETE.telefon });
  const cora = await browserOeffnen(browser);
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');
  await imBrowserBeitreten(cora.page, instanz.adresse, 'cora');
  const emil = await beitreten(instanz.adresse, 'emil', { art: 'mensch', herkunft: '127.0.0.5' });
  const dora = await beitreten(instanz.adresse, 'dora', { art: 'mensch', herkunft: '127.0.0.4' });
  const gehoert = { ben: await ansagenMitschreiben(ben.page), cora: await ansagenMitschreiben(cora.page) };
  for (const seite of [ben.page, cora.page]) {
    await expect(direkt(seite).locator('.lc-side-leer')).toHaveCount(2);
  }

  await direktOeffnen(anna.page, BEN_B);
  await gespraechOffen(anna.page, BEN_B);
  const gesendet = anna.page.waitForResponse((antwort) => antwort.request().method() === 'POST' && antwort.url().endsWith('/nachrichten'));
  await eingabeAn(anna.page, BEN_B).fill(GEHEIM);
  await eingabeAn(anna.page, BEN_B).press('Enter');
  expect((await gesendet).status()).toBe(201);
  const ab = Date.now();
  await expect(eintraege(ben.page)).toHaveCount(1, { timeout: ANSAGE_MS });
  await expect(eintraege(ben.page).first()).toHaveAttribute('aria-label', `Direktnachricht mit ${ANNA_B}, online`);
  await expect.poll(async () => (await gehoert.ben()).map(({ text }) => text), { intervals: [25], timeout: 5000 })
    .toContain(`Neue Direktnachricht von ${ANNA_B}`);
  const ansage = (await gehoert.ben()).find(({ text }) => text.includes('Neue Direktnachricht'));
  expect(ansage.um - ab, 'Ansage bei ben ab der 201').toBeLessThanOrEqual(ANSAGE_MS);
  await expect(eintraege(anna.page)).toHaveCount(1);

  expect((await direktSenden(instanz.adresse, emil, BEN_B, 'von emil', '127.0.0.5')).status).toBe(201);
  expect((await direktSenden(instanz.adresse, dora, BEN_B, 'von dora', '127.0.0.4')).status).toBe(201);
  expect((await direktSenden(instanz.adresse, dora, ANNA_B, 'an anna', '127.0.0.4')).status).toBe(201);
  await expect(eintraege(ben.page)).toHaveCount(3);
  const amTelefon = await eintraege(ben.page).evaluateAll((links) => links.map((link) => ({
    kuerzel: link.querySelector('.lc-direkt-kuerzel').textContent,
    kuerzelSichtbar: link.querySelector('.lc-direkt-kuerzel').getClientRects().length > 0,
    punktSichtbar: link.querySelector('.lc-dot').getClientRects().length > 0,
    hoehe: link.getBoundingClientRect().height,
    name: link.getAttribute('aria-label').replace(/, [^,]*$/, '').replace(/, offline$/, ''),
  })));
  expect(amTelefon.map(({ kuerzel }) => kuerzel)).toEqual(['an', 'do', 'em']);
  expect(amTelefon.map(({ name }) => name)).toEqual([ANNA_B, 'dora@127.0.0.4', 'emil@127.0.0.5'].map((kennung) => `Direktnachricht mit ${kennung}`));
  for (const eintrag of amTelefon) {
    expect(eintrag.kuerzelSichtbar && eintrag.punktSichtbar).toBe(true);
    expect(eintrag.hoehe).toBeGreaterThanOrEqual(44);
  }

  await expect(eintraege(cora.page)).toHaveCount(0);
  await expect(direkt(cora.page).locator('.lc-side-leer').first()).toHaveText('Noch keine Direktnachrichten.');
  await cora.page.reload();
  await expect(direkt(cora.page).locator('.lc-side-leer').first()).toHaveText('Noch keine Direktnachrichten.');
  await expect(eintraege(cora.page)).toHaveCount(0);
  await direktOeffnen(cora.page, ANNA_B);
  await gespraechOffen(cora.page, ANNA_B);
  await expect(verlaufMit(cora.page, ANNA_B).locator('.lc-msg')).toHaveCount(0);
  await expect(cora.page.getByText(GEHEIM)).toHaveCount(0);
  await expect(eintraege(cora.page)).toHaveCount(0);
  const coraHoert = await ansagenMitschreiben(cora.page);
  expect((await direktSenden(instanz.adresse, emil, BEN_B, 'noch einmal', '127.0.0.5')).status).toBe(201);
  await expect.poll(async () => (await gehoert.ben()).length, { timeout: 12000 }).toBeGreaterThan(1);
  expect([...(await gehoert.cora()), ...(await coraHoert())].filter(({ text }) => text.includes('Direktnachricht'))).toEqual([]);
  expect([...anna.fehler, ...ben.fehler, ...cora.fehler]).toEqual([]);
});
