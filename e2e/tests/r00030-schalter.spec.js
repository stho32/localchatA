'use strict';
// R00030 F0101 B0448 — der Schalter „Anhang“ in der Eingabe: Ort vor „Senden“, Dateiauswahl über Klick, Tippen, Enter,
// Leertaste und Strg Shift U, danach Fokus im Feld; Zielfläche, ⎘ unter 480 px, Feldbreite bei 375 und 320 px, Fokus
// sichtbar (UX M1, M3), Bildschirmleser. Geräte als emulierte Kontexte (r00027-hilfen.js).
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf, box } = require('./r00027-hilfen');
const { schalter, textDatei, ringGegenFlaeche } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function seiteAuf(browser, geraet, optionen = {}) {
  instanz = await sendenStarten();
  const seite = await geraetOeffnen(browser, geraet, optionen);
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  return seite;
}

/// Erwartet, dass `ausloesen` die Dateiauswahl des Systems öffnet, und wählt darin `log.txt`.
async function oeffnetAuswahl(page, ausloesen) {
  const auswahl = page.waitForEvent('filechooser', { timeout: 3000 });
  await ausloesen();
  const chooser = await auswahl;
  expect(chooser.isMultiple()).toBe(true);
  await chooser.setFiles([textDatei()]);
}

test('Desktop mit Maus und Tastatur: Klick, Strg Shift U, Enter und Leertaste öffnen die Auswahl; Tab-Folge Feld → Anhang → Senden → Teilnehmer', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'desktop');
  const feld = eingabe(page);

  await oeffnetAuswahl(page, () => schalter(page).click());
  await expect(feld).toBeFocused();
  await oeffnetAuswahl(page, () => feld.press('Control+Shift+U'));
  await expect(feld).toBeFocused();
  await feld.press('Tab');
  await expect(schalter(page)).toBeFocused();
  await oeffnetAuswahl(page, () => page.keyboard.press('Enter'));
  await expect(feld).toBeFocused();
  await feld.press('Tab');
  await oeffnetAuswahl(page, () => page.keyboard.press(' '));
  await expect(feld).toBeFocused();

  await feld.press('Tab');
  await expect(schalter(page)).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(knopf(page)).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('complementary', { name: 'Teilnehmer' })).toBeFocused();
  expect(fehler).toEqual([]);
});

test('Telefon: Tippen auf den Schalter öffnet die Auswahl mit Mehrfachauswahl, danach liegt der Fokus im Feld', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'telefon');

  await oeffnetAuswahl(page, () => schalter(page).tap());
  await expect(eingabe(page)).toBeFocused();
});

test('Bildschirmleser: genau ein Schalter „Anhang“ in der Eingabe, mit aria-keyshortcuts, ohne accept', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'desktop');

  await expect(page.getByRole('button', { name: 'Anhang', exact: true })).toHaveCount(1);
  await expect(schalter(page)).toHaveAttribute('aria-keyshortcuts', 'Control+Shift+U');
  await expect(schalter(page)).toHaveAttribute('type', 'button');
  const auswahl = page.locator('.lc-comp input[type="file"]');
  await expect(auswahl).toHaveCount(1);
  await expect(auswahl).not.toHaveAttribute('accept', /.*/);
});

for (const [geraet, mindestens] of [['telefon', 44], ['tablet', 44], ['tabletQuer', 44], ['desktop', 24]]) {
  test(`Zielfläche und Ort (${geraet}): mindestens ${mindestens} × ${mindestens} px, rechts vom Feld, links von „Senden“, eine Zeile`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, geraet);

    const anhang = await box(schalter(page));
    const senden = await box(knopf(page));
    const feld = await box(eingabe(page));
    expect(anhang.width).toBeGreaterThanOrEqual(mindestens);
    expect(anhang.height).toBeGreaterThanOrEqual(mindestens);
    expect(anhang.left).toBeGreaterThanOrEqual(feld.right);
    expect(anhang.right).toBeLessThanOrEqual(senden.left);
    expect(Math.abs(anhang.bottom - senden.bottom)).toBeLessThanOrEqual(1);
    await expect(schalter(page)).toHaveCSS('touch-action', 'manipulation');
  });
}

for (const [breite, sichtbar] of [[479, '⎘'], [480, 'Anhang']]) {
  test(`Bei ${breite} px zeigt der Schalter „${sichtbar}“, der zugängliche Name bleibt „Anhang“`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, 'telefon', { fenster: { width: breite, height: 812 } });

    expect(await schalter(page).evaluate((element) => element.innerText.trim())).toBe(sichtbar);
    await expect(schalter(page)).toHaveAccessibleName('Anhang');
  });
}

for (const [breite, feldMindestens] of [[375, 160], [320, 100]]) {
  test(`Schmal (Telefon ${breite} px): das Feld ist mindestens ${feldMindestens} px breit, kein Querscrollen`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, 'telefon', { fenster: { width: breite, height: 812 } });

    expect((await box(eingabe(page))).width).toBeGreaterThanOrEqual(feldMindestens);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
}

// „Fokus sichtbar“ (UX M3) am Schalter: outline nicht none, ≥ 2 px, Abstand ≥ 0, ≥ 3 : 1 gegen kd-surface. Die
// allgemeine Regel (`outline-offset: -2px`) allein wird rot.
for (const thema of ['light', 'dark']) {
  test(`Fokus sichtbar (Desktop, ${thema}): eigener Ring am Schalter „Anhang“`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, 'desktop', { thema });

    await eingabe(page).focus();
    await page.keyboard.press('Tab');
    await expect(schalter(page)).toBeFocused();
    const ring = await schalter(page).evaluate(ringGegenFlaeche);

    expect(ring.sichtbar).toBe(true);
    expect(ring.stil).not.toBe('none');
    expect(ring.breite).toBeGreaterThanOrEqual(2);
    expect(ring.versatz).toBeGreaterThanOrEqual(0);
    expect(ring.kontrast).toBeGreaterThanOrEqual(3);
  });
}
