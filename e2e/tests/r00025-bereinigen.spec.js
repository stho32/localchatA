'use strict';
// R00025 F0086 „Hinweiszeile hilft beim Bereinigen“ (B0404): der Schalter „Steuerzeichen entfernen“ neben der
// Hinweiszeile — nimmt erst auf Klick die verbotenen Zeichen aus dem Feld, sagt an, ist auf Touch-Geräten 44 × 44 px
// und lässt den Fokus im Feld. Gegen das echte Programm; die Regel selbst prüft e2e/einheit/textregel.test.mjs.
const { test, expect } = require('@playwright/test');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { ansageRegion, sendungenMitschreiben } = require('./r00012-hilfen');
const { GERAETE, geraetOeffnen, knopf, box } = require('./r00027-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const schalter = (page) => page.locator('.lc-comp').getByRole('button', { name: 'Steuerzeichen entfernen' });
const hinweiszeile = (page) => page.locator('#lc-comp-hinweis');
const ROT = 'rot \u001b[31m';

function antwort201(page) {
  return page.waitForResponse((antwort) => antwort.request().method() === 'POST'
    && antwort.url().endsWith('/nachrichten') && antwort.status() === 201);
}

test('Schalter: Hinweis mit Schalter, Enter formt nichts um, Klick entfernt, danach sendet Enter (201)', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const sendungen = sendungenMitschreiben(a.page);

  await eingabe(a.page).fill(ROT);
  await expect(hinweiszeile(a.page)).toHaveText('⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.');
  await expect(schalter(a.page)).toBeVisible();
  await eingabe(a.page).press('Enter');
  await a.page.waitForTimeout(300);
  expect(sendungen).toEqual([]);
  await expect(eingabe(a.page)).toHaveValue(ROT);

  await schalter(a.page).click();

  await expect(eingabe(a.page)).toHaveValue('rot [31m');
  await expect(hinweiszeile(a.page)).toHaveText('');
  await expect(eingabe(a.page)).not.toHaveAttribute('aria-invalid', 'true');
  await expect(eingabe(a.page)).toBeFocused();
  expect(await eingabe(a.page).evaluate((feld) => [feld.selectionStart, feld.selectionEnd])).toEqual([8, 8]);
  await expect(schalter(a.page)).toBeHidden();
  const gesendet = antwort201(a.page);
  await eingabe(a.page).press('Enter');
  await gesendet;
  expect(a.fehler).toEqual([]);
});

test('Ohne Steuerzeichen kein Schalter: weder bei „Zu lang“ noch bei „noch n Zeichen“', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');

  await eingabe(a.page).fill('a'.repeat(16385));
  await expect(hinweiszeile(a.page)).toHaveText('⚠ Zu lang: 16.385 von höchstens 16.384 Byte — bitte teilen.');
  await expect(schalter(a.page)).toBeHidden();
  await eingabe(a.page).fill('a'.repeat(15000));
  await expect(hinweiszeile(a.page)).toHaveText('noch 1.384 Zeichen');
  await expect(schalter(a.page)).toBeHidden();
  expect(a.fehler).toEqual([]);
});

test('Entfernen angesagt (V4): „2 Steuerzeichen entfernt“, mit einem „1 Steuerzeichen entfernt“', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');

  await eingabe(a.page).fill(`${ROT}\u001b`);
  await schalter(a.page).click();
  await expect(ansageRegion(a.page)).toHaveText('2 Steuerzeichen entfernt');
  await eingabe(a.page).fill(ROT);
  await schalter(a.page).click();
  await expect(ansageRegion(a.page)).toHaveText('1 Steuerzeichen entfernt');
  expect(a.fehler).toEqual([]);
});

// Zielgröße (V3, wörtlich: „‚Rückgängig‘ und ‚Steuerzeichen entfernen‘ sind `<button type="button">`, im Stil von
// `.lc-send-schalter`, mit `touch-action: manipulation`. Desktop: ≥ 24 × 24 px. Telefon (…, 375 × 812) und Tablet
// 768 × 1024: ≥ 44 × 44 px, Abstand ≥ 8 px. Mutante ‚ohne `(pointer: coarse)`-Regel‘ rot.“) — hier „Steuerzeichen
// entfernen“, Abstand zu „Senden“.
for (const [geraet, mindestens] of [['desktop', 24], ['telefon', 44], ['tablet', 44]]) {
  test(`Zielgröße (${geraet}): ≥ ${mindestens} × ${mindestens} px, Abstand zu Senden ≥ 8 px, Stil und Typ`, async ({ browser }) => {
    instanz = await sendenStarten();
    const a = await geraetOeffnen(browser, geraet, { streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await eingabe(a.page).fill(ROT);
    await expect(schalter(a.page)).toBeVisible();

    const rahmen = await box(schalter(a.page));
    const senden = await box(knopf(a.page));
    const art = await schalter(a.page).evaluate((element) => ({
      typ: element.getAttribute('type'),
      imStil: element.closest('.lc-send-schalter') !== null,
      touch: getComputedStyle(element).touchAction,
    }));

    expect(rahmen.width).toBeGreaterThanOrEqual(mindestens);
    expect(rahmen.height).toBeGreaterThanOrEqual(mindestens);
    const abstand = Math.max(rahmen.left - senden.right, senden.left - rahmen.right, rahmen.top - senden.bottom, senden.top - rahmen.bottom);
    expect(abstand).toBeGreaterThanOrEqual(8);
    expect(art).toEqual({ typ: 'button', imStil: true, touch: 'manipulation' });
    expect(a.fehler).toEqual([]);
  });
}

// Touch (V5, wörtlich: „`pointerdown` mit `preventDefault()` am Schalter, solange das Feld den Fokus hat (wie ADR-0038
// K4). `tap()` → kein `blur` am Feld, Fokus im Feld, ‚Senden‘ wechselt von `aria-disabled="true"` auf bereit; `tap()`
// auf ‚Senden‘ → 201.“)
test('Touch (Telefon): Tippen lässt den Fokus im Feld, danach sendet „Senden“ (201)', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await geraetOeffnen(browser, 'telefon', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await eingabe(a.page).fill(ROT);
  await eingabe(a.page).focus();
  await expect(knopf(a.page)).toHaveAttribute('aria-disabled', 'true');
  await eingabe(a.page).evaluate((feld) => {
    window.verlorenerFokus = 0;
    feld.addEventListener('blur', () => {
      window.verlorenerFokus += 1;
    });
  });

  await schalter(a.page).tap();

  await expect(eingabe(a.page)).toHaveValue('rot [31m');
  await expect(eingabe(a.page)).toBeFocused();
  expect(await a.page.evaluate(() => window.verlorenerFokus)).toBe(0);
  await expect(knopf(a.page)).not.toHaveAttribute('aria-disabled', 'true');
  const gesendet = antwort201(a.page);
  await knopf(a.page).tap();
  await gesendet;
  expect(a.fehler).toEqual([]);
});

// Reihenfolge und Beschreibung (V5, V17): Tab vom Feld → Senden → Steuerzeichen entfernen; der Schalter liegt in
// keinem Element, auf das aria-describedby von Feld oder Senden zeigt.
test('Reihenfolge: Feld → Senden → Steuerzeichen entfernen, nicht im Beschreibungsziel', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await eingabe(a.page).fill(ROT);

  await eingabe(a.page).focus();
  await a.page.keyboard.press('Tab');
  await expect(knopf(a.page)).toBeFocused();
  await a.page.keyboard.press('Tab');
  await expect(schalter(a.page)).toBeFocused();
  const imBeschreibungsziel = await schalter(a.page).evaluate((element) => {
    const ziele = ['.lc-comp-feld', '.lc-comp-senden']
      .flatMap((auswahl) => (document.querySelector(auswahl).getAttribute('aria-describedby') || '').split(' '))
      .map((id) => document.getElementById(id))
      .filter((ziel) => ziel !== null);
    return ziele.some((ziel) => ziel.contains(element));
  });
  expect(imBeschreibungsziel).toBe(false);
  expect(a.fehler).toEqual([]);
});

// Schmal (V13): mit Hinweis und Schalter im Telefon-Kontext bei 320, 375 und 414 px und auf dem Tablet hoch und quer —
// kein Querscrollen, keine Vergrößerung.
for (const [geraet, breite] of [['telefon', 320], ['telefon', 375], ['telefon', 414], ['tablet', 768], ['tabletQuer', 1024]]) {
  test(`Schmal (${geraet} ${breite} px): Hinweis mit Schalter ohne Querscrollen`, async ({ browser }) => {
    instanz = await sendenStarten();
    const fenster = { width: breite, height: GERAETE[geraet].viewport.height };
    const a = await geraetOeffnen(browser, geraet, { fenster, streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await eingabe(a.page).fill(ROT);
    await expect(schalter(a.page)).toBeVisible();

    const masse = await a.page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      innerWidth: window.innerWidth,
      clientWidth: document.documentElement.clientWidth,
      scale: window.visualViewport.scale,
    }));

    expect(masse.scrollWidth).toBeLessThanOrEqual(masse.innerWidth);
    expect(masse.clientWidth).toBe(breite);
    expect(masse.scale).toBe(1);
    expect(a.fehler).toEqual([]);
  });
}
