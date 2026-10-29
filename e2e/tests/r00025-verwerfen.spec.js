'use strict';
// R00025 F0085 „Verwerfen lässt sich rückgängig machen“: gegen das echte Programm. A ist `shoff`; die Route bricht den
// ersten POST ab → `drei` steht fehlgeschlagen mit „Erneut senden“ und „Verwerfen“. Die Regel (5 s, Fokus, Channel-
// wechsel) prüft e2e/einheit/zustellung.test.mjs mit gestellter Zeit.
const { test, expect } = require('@playwright/test');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { ROUTE, vorlaeufige, saetzeMit, sendungenMitschreiben } = require('./r00012-hilfen');
const { GERAETE, geraetOeffnen, box, knopf } = require('./r00027-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Konsolenfehler außer dem abgebrochenen ersten POST.
const unerwartet = (fehler) => fehler.filter((zeile) => !(zeile.includes('Failed to load resource: net::ERR_FAILED') && zeile.includes('/nachrichten')));

const verworfen = (page) => page.getByRole('log').locator('.lc-verworfen');
const rueckgaengig = (page) => verworfen(page).getByRole('button', { name: 'Rückgängig' });

async function erstenPostAbbrechen(page) {
  let erste = true;
  await page.route(ROUTE, async (route) => {
    if (route.request().method() !== 'POST' || !erste) {
      await route.continue();
      return;
    }
    erste = false;
    await route.abort();
  });
}

/// `drei` fehlgeschlagen, dann „Verwerfen“ geklickt → die Zeile „Verworfen“ mit „Rückgängig“ steht.
async function dreiVerwerfen(page) {
  await erstenPostAbbrechen(page);
  await eingabe(page).fill('drei');
  await knopf(page).click();
  await expect(vorlaeufige(page).locator('.lc-send')).toHaveText('⚠ Keine Antwort vom Server.');
  await vorlaeufige(page).getByRole('button', { name: 'Verwerfen' }).click();
  await expect(rueckgaengig(page)).toBeVisible();
}

const EINE_LIVE_REGION = `document.querySelectorAll('[aria-live]:not([aria-live="off"]), [role="status"], [role="alert"], [role="log"]:not([aria-live="off"])').length`;

// Zielgröße (V3, wörtlich: „‚Rückgängig‘ und ‚Steuerzeichen entfernen‘ sind `<button type="button">`, im Stil von
// `.lc-send-schalter`, mit `touch-action: manipulation`. Desktop: ≥ 24 × 24 px. Telefon (…, 375 × 812) und Tablet
// 768 × 1024: ≥ 44 × 44 px, Abstand ≥ 8 px. Mutante ‚ohne `(pointer: coarse)`-Regel‘ rot.“) — hier „Rückgängig“,
// Abstand zum Nachbarn in der Zeile („Verworfen“).
for (const [geraet, mindestens] of [['desktop', 24], ['telefon', 44], ['tablet', 44]]) {
  test(`Zielgröße (${geraet}): „Rückgängig“ ≥ ${mindestens} × ${mindestens} px, Abstand ≥ 8 px, Stil und Typ`, async ({ browser }) => {
    instanz = await sendenStarten();
    const a = await geraetOeffnen(browser, geraet);
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await dreiVerwerfen(a.page);

    const rahmen = await box(rueckgaengig(a.page));
    const nachbar = await box(verworfen(a.page).locator('.lc-verworfen-text'));
    const art = await rueckgaengig(a.page).evaluate((element) => ({
      typ: element.getAttribute('type'),
      imStil: element.closest('.lc-send-schalter') !== null,
      touch: getComputedStyle(element).touchAction,
    }));

    expect(rahmen.width).toBeGreaterThanOrEqual(mindestens);
    expect(rahmen.height).toBeGreaterThanOrEqual(mindestens);
    expect(Math.max(rahmen.left - nachbar.right, rahmen.top - nachbar.bottom)).toBeGreaterThanOrEqual(8);
    expect(art).toEqual({ typ: 'button', imStil: true, touch: 'manipulation' });
    expect(unerwartet(a.fehler)).toEqual([]);
  });
}

// „Keine eigene Region“ (zu V1, relativ): die Zahl aus dem Ausdruck von „Eine Live-Region“ ist mit „Verworfen ·
// Rückgängig“ gleich der ohne. Dazu V14: „Verworfen“ in kd-ink-muted auf kd-surface; die Zeile ist nicht höher als die
// Nachricht war.
test('Keine eigene Region, Farben, nicht höher als die Nachricht', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const ohne = await a.page.evaluate(EINE_LIVE_REGION);
  await erstenPostAbbrechen(a.page);
  await eingabe(a.page).fill('drei');
  await eingabe(a.page).press('Enter');
  await expect(vorlaeufige(a.page).locator('.lc-send')).toHaveText('⚠ Keine Antwort vom Server.');
  const nachrichtHoehe = (await box(vorlaeufige(a.page))).height;

  await vorlaeufige(a.page).getByRole('button', { name: 'Verwerfen' }).click();
  await expect(rueckgaengig(a.page)).toBeVisible();

  expect(await a.page.evaluate(EINE_LIVE_REGION)).toBe(ohne);
  const farben = await verworfen(a.page).evaluate((zeile) => {
    const probe = document.createElement('span');
    zeile.append(probe);
    probe.style.color = 'var(--kd-ink-muted)';
    probe.style.backgroundColor = 'var(--kd-surface)';
    const erwartet = { text: getComputedStyle(probe).color, flaeche: getComputedStyle(probe).backgroundColor };
    probe.remove();
    const text = zeile.querySelector('.lc-verworfen-text');
    return { erwartet, text: getComputedStyle(text).color, flaeche: getComputedStyle(zeile).backgroundColor };
  });
  expect(farben.text).toBe(farben.erwartet.text);
  expect(farben.flaeche).toBe(farben.erwartet.flaeche);
  expect((await box(verworfen(a.page))).height).toBeLessThanOrEqual(nachrichtHoehe);
  expect(unerwartet(a.fehler)).toEqual([]);
});

// „Schmal“ (V13): mit „Verworfen · Rückgängig“ im Telefon-Kontext bei 320, 375 und 414 px und auf dem Tablet hoch und
// quer — kein Querscrollen, keine Vergrößerung.
for (const [geraet, breite] of [['telefon', 320], ['telefon', 375], ['telefon', 414], ['tablet', 768], ['tabletQuer', 1024]]) {
  test(`Schmal (${geraet} ${breite} px): „Verworfen · Rückgängig“ ohne Querscrollen`, async ({ browser }) => {
    instanz = await sendenStarten();
    const fenster = { width: breite, height: GERAETE[geraet].viewport.height };
    const a = await geraetOeffnen(browser, geraet, { fenster });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await dreiVerwerfen(a.page);

    const masse = await a.page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      innerWidth: window.innerWidth,
      clientWidth: document.documentElement.clientWidth,
      scale: window.visualViewport.scale,
    }));

    expect(masse.scrollWidth).toBeLessThanOrEqual(masse.innerWidth);
    expect(masse.clientWidth).toBe(breite);
    expect(masse.scale).toBe(1);
    expect(unerwartet(a.fehler)).toEqual([]);
  });
}

/// Z15 aus R00025: Klick „Verwerfen“ → 4 s danach steht „Verworfen“ noch, spätestens 7 s danach ist die Zeile weg.
const Z15_NOCH_DA_MS = 4000;
const Z15_SPAETESTENS_MS = 7000;

// „Rückgängig“ (Abnahme-Probe UX-Review #22 F4: „5 s „Verworfen · Rückgängig““): Klick „Verwerfen“ → an der Stelle steht
// „Verworfen“ mit „Rückgängig“, der Fokus liegt im Feld; Klick „Rückgängig“ → `drei` zeigt wieder ⚠ „Keine Antwort vom
// Server.“ mit „Erneut senden“ und „Verwerfen“, laut Netzprotokoll kein POST, der Fokus liegt auf „Erneut senden“
// (Mutante „Rückgängig sendet“ wird rot).
test('Rückgängig: „Verworfen“ an der Stelle, Klick bringt drei zurück, ohne POST, Fokus auf „Erneut senden“', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const sendungen = sendungenMitschreiben(a.page);

  await dreiVerwerfen(a.page);
  await expect(verworfen(a.page).locator('.lc-verworfen-text')).toHaveText('Verworfen');
  await expect(eingabe(a.page)).toBeFocused();
  await rueckgaengig(a.page).click();

  await expect(vorlaeufige(a.page).locator('.lc-body')).toHaveText(['drei']);
  await expect(vorlaeufige(a.page).locator('.lc-send')).toHaveText('⚠ Keine Antwort vom Server.');
  await expect(vorlaeufige(a.page).getByRole('button')).toHaveText(['Erneut senden', 'Verwerfen']);
  await expect(vorlaeufige(a.page).getByRole('button', { name: 'Erneut senden' })).toBeFocused();
  await a.page.waitForTimeout(300);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['drei']);
  expect(unerwartet(a.fehler)).toEqual([]);
});

// „Verfallen“ (Z15): Klick „Verwerfen“ → 4 s danach steht „Verworfen“ noch, spätestens Z15 danach ist die Zeile weg; kein
// Satz `drei` im Log. „Rückgängig“ erhält dabei nie den Fokus. Die Uhr startet beim Klick; gemessen wird ereignisgenau.
test('Verfallen (Z15): nach 4 s noch da, spätestens 7 s nach dem Klick weg, kein Satz', async ({ browser }) => {
  test.setTimeout(30000);
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await erstenPostAbbrechen(a.page);
  await eingabe(a.page).fill('drei');
  await eingabe(a.page).press('Enter');
  await expect(vorlaeufige(a.page).locator('.lc-send')).toHaveText('⚠ Keine Antwort vom Server.');

  await vorlaeufige(a.page).getByRole('button', { name: 'Verwerfen' }).click();
  const beginn = Date.now();
  await a.page.waitForTimeout(Z15_NOCH_DA_MS - (Date.now() - beginn));
  const nachVierSekunden = await verworfen(a.page).count();
  await expect.poll(() => verworfen(a.page).count(), { intervals: [25], timeout: 10000 }).toBe(0);
  const dauer = Date.now() - beginn;
  test.info().annotations.push({ type: 'Z15 ms', description: String(dauer) });

  expect(nachVierSekunden).toBe(1);
  expect(dauer).toBeLessThanOrEqual(Z15_SPAETESTENS_MS);
  expect(saetzeMit(instanz.ordner, 'allgemein', 'drei')).toBe(0);
  expect(unerwartet(a.fehler)).toEqual([]);
});
