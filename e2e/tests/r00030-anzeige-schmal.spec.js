'use strict';
// R00030 F0102 „Schmal“ und „Breiten“ (UX V1) auf dem Telefon (320 und 375 px, Touch): die Datei-Zeile gibt dem Namen
// mindestens 96 px und bricht dafür auf zwei Zeilen um, „Öffnen“ und „Laden“ sind ganz sichtbar, je mindestens
// 44 × 44 px, liegen im Artikel und überschneiden den Schalter „⋯ Aktionen“ nicht; kein Querscrollen.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { anzeigeStarten, testdateien, dateiZeile } = require('./r00030-anzeige-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const LANGER_NAME = 'bericht mit einem sehr langen Namen aus dem dritten Quartal.pdf';
const telefon = (breite) => ({ viewport: { width: breite, height: 812 }, hasTouch: true, isMobile: true, deviceScaleFactor: 3 });
const rechteck = (locator) => locator.evaluate((element) => {
  const { left, right, top, bottom, width, height } = element.getBoundingClientRect();
  return { left, right, top, bottom, width, height };
});
const ueberschneiden = (eins, zwei) => eins.left < zwei.right && zwei.left < eins.right && eins.top < zwei.bottom && zwei.top < eins.bottom;

for (const breite of [320, 375]) {
  test(`Schmal (B0457, V1) bei ${breite} px: Name der Datei-Zeile ≥ 96 px, „Öffnen“ und „Laden“ in Zeile 2, je ≥ 44 × 44 im Artikel, keine Überschneidung mit „⋯“, kein Querscrollen`, async ({ browser }) => {
    const lauf = await anzeigeStarten(browser, [{ namen: [{ name: LANGER_NAME, inhalt: testdateien()['bericht.pdf'] }] }], { geraet: telefon(breite) });
    instanz = lauf.instanz;
    const { page } = lauf.a;
    const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
    const zeile = dateiZeile(knoten, 'bericht mit');
    await expect(zeile).toHaveCount(1);
    const artikel = await rechteck(knoten);
    const schalter = await rechteck(knoten.getByRole('button', { name: 'Aktionen', exact: true }));
    const name = await rechteck(zeile.locator('.lc-datei-name'));
    expect(name.width).toBeGreaterThanOrEqual(96);
    const kuerzel = await rechteck(zeile.locator('.lc-datei-kuerzel'));
    const groesse = await rechteck(zeile.locator('.lc-datei-groesse'));
    expect(Math.abs(name.top - kuerzel.top)).toBeLessThanOrEqual(4);
    expect(Math.abs(groesse.top - name.top)).toBeLessThanOrEqual(4);
    const oeffnen = await rechteck(zeile.getByRole('link', { name: `${LANGER_NAME} öffnen`, exact: true }));
    const laden = await rechteck(zeile.getByRole('link', { name: `${LANGER_NAME} laden`, exact: true }));
    expect(oeffnen.top).toBeGreaterThanOrEqual(name.bottom - 1);
    expect(Math.abs(oeffnen.left - kuerzel.left)).toBeLessThanOrEqual(1);
    expect(laden.left - oeffnen.right).toBeGreaterThanOrEqual(8 - 0.5);
    expect(laden.left - oeffnen.right).toBeLessThanOrEqual(8 + 0.5);
    for (const link of [oeffnen, laden]) {
      expect(link.width).toBeGreaterThanOrEqual(44);
      expect(link.height).toBeGreaterThanOrEqual(44);
      expect(link.left).toBeGreaterThanOrEqual(artikel.left);
      expect(link.right).toBeLessThanOrEqual(artikel.right);
      expect(link.bottom).toBeLessThanOrEqual(artikel.bottom);
      expect(ueberschneiden(link, schalter)).toBe(false);
    }
    expect(ueberschneiden(await rechteck(zeile), schalter)).toBe(false);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.getByRole('log').evaluate((log) => log.scrollWidth <= log.clientWidth)).toBe(true);
    expect(lauf.a.fehler).toEqual([]);
  });
}

test('Eine Zeile, wenn der Name 96 px behält (B0457, V1): bericht.pdf am Desktop → Kürzel, Name, Größe, „Öffnen“, „Laden“ auf einer Höhe', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['bericht.pdf'] }]);
  instanz = lauf.instanz;
  const zeile = dateiZeile(nachrichtMitZeit(lauf.a.page, lauf.zeiten[0]), 'bericht.pdf');
  const name = await rechteck(zeile.locator('.lc-datei-name'));
  const laden = await rechteck(zeile.getByRole('link', { name: 'bericht.pdf laden', exact: true }));
  expect(laden.top).toBeLessThan(name.bottom);
  expect(laden.left).toBeGreaterThan(name.right);
  expect(lauf.a.fehler).toEqual([]);
});
