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

test('Bild auf dem Telefon (B0459): Tippen auf die Vorschau → Dialog, das Bild passt in 375 × 812 ohne Querscrollen; Tippen auf „Schließen“ → zu', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png'] }], { geraet: telefon(375) });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  await nachrichtMitZeit(page, lauf.zeiten[0]).getByRole('button', { name: 'foto.png groß anzeigen', exact: true }).tap();
  const dialog = page.getByRole('dialog', { name: 'foto.png', exact: true });
  await expect(dialog).toBeVisible();
  const bild = dialog.getByRole('img', { name: 'foto.png', exact: true });
  await expect.poll(() => bild.evaluate((element) => element.complete && element.naturalWidth > 0)).toBe(true);
  const rahmen = await rechteck(bild);
  expect(rahmen.left).toBeGreaterThanOrEqual(0);
  expect(rahmen.right).toBeLessThanOrEqual(375);
  expect(rahmen.top).toBeGreaterThanOrEqual(0);
  expect(rahmen.bottom).toBeLessThanOrEqual(812);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  const schliessen = dialog.getByRole('button', { name: 'Schließen', exact: true });
  const flaeche = await rechteck(schliessen);
  expect(flaeche.width).toBeGreaterThanOrEqual(44);
  expect(flaeche.height).toBeGreaterThanOrEqual(44);
  await schliessen.tap();
  await expect(dialog).toHaveCount(0);
  expect(lauf.a.fehler).toEqual([]);
});

const NAME_120 = `${'f'.repeat(116)}.png`;

for (const breite of [320, 375]) {
  test(`Schmal mit Bild (B0461, PO 157) bei ${breite} px: Vorschau und Datei-Zeilen (Name mit 120 Zeichen) liegen im Artikel links von „⋯ Aktionen“, kein Querscrollen`, async ({ browser }) => {
    const lauf = await anzeigeStarten(browser, [{ namen: [{ name: NAME_120, inhalt: testdateien()['foto.png'] }, 'bericht.pdf'] }], { geraet: telefon(breite) });
    instanz = lauf.instanz;
    const { page } = lauf.a;
    const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
    const artikel = await rechteck(knoten);
    const schalter = await rechteck(knoten.getByRole('button', { name: 'Aktionen', exact: true }));
    const teile = [knoten.locator('.lc-vorschau'), dateiZeile(knoten, 'ffff'), dateiZeile(knoten, 'bericht.pdf')];
    for (const teil of teile) {
      const flaeche = await rechteck(teil);
      expect(ueberschneiden(flaeche, schalter)).toBe(false);
      expect(flaeche.right).toBeLessThanOrEqual(schalter.left);
      expect(flaeche.left).toBeGreaterThanOrEqual(artikel.left);
      expect(flaeche.bottom).toBeLessThanOrEqual(artikel.bottom);
    }
    for (const link of await knoten.getByRole('link').all()) {
      const flaeche = await rechteck(link);
      expect(flaeche.width).toBeGreaterThanOrEqual(44);
      expect(flaeche.height).toBeGreaterThanOrEqual(44);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.getByRole('log').evaluate((log) => log.scrollWidth <= log.clientWidth)).toBe(true);
    expect(lauf.a.fehler).toEqual([]);
  });
}

const GERAETE = [
  { titel: 'Telefon 320', geraet: telefon(320) },
  { titel: 'Telefon 375', geraet: telefon(375) },
  { titel: 'Tablet 768', geraet: { viewport: { width: 768, height: 1024 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 } },
];

for (const { titel, geraet } of GERAETE) {
  test(`Breiten (B0461, V1) ${titel}: Vorschau und Datei-Zeile sind min(360, Inhaltsbreite) breit, die Vorschau proportional, ihr Platz steht vor dem Laden, Sprung ≤ 1 px`, async ({ browser }) => {
    const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png', 'bericht.pdf'] }], {
      geraet,
      vorSeite: (page) => page.context().route('**/anhaenge/**', async (route) => {
        await new Promise((weiter) => setTimeout(weiter, 1000));
        await route.continue();
      }),
    });
    instanz = lauf.instanz;
    const { page } = lauf.a;
    const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
    const inhaltsbreite = (await rechteck(knoten.locator('.lc-msg-inhalt'))).width;
    const rahmen = Math.min(360, inhaltsbreite);
    test.info().annotations.push({ type: 'Inhaltsbreite', description: `${titel}: ${inhaltsbreite} px, Rahmen ${rahmen} px` });
    const bild = knoten.locator('.lc-vorschau img');
    expect(await bild.evaluate((element) => element.complete && element.naturalWidth > 0)).toBe(false);
    const vorher = await rechteck(knoten.locator('.lc-vorschau'));
    const unterkanteVorher = (await rechteck(knoten)).bottom;
    expect(Math.abs(vorher.width - rahmen)).toBeLessThanOrEqual(1);
    expect(Math.abs(vorher.height - (rahmen * 800) / 1280)).toBeLessThanOrEqual(1);
    for (const name of ['foto.png', 'bericht.pdf']) {
      expect(Math.abs((await rechteck(dateiZeile(knoten, name))).width - rahmen)).toBeLessThanOrEqual(1);
    }
    await expect.poll(() => bild.evaluate((element) => element.complete && element.naturalWidth > 0), { timeout: 10000 }).toBe(true);
    expect(Math.abs((await rechteck(knoten)).bottom - unterkanteVorher)).toBeLessThanOrEqual(1);
    expect(lauf.a.fehler).toEqual([]);
  });
}

/// Hält jede Antwort unter /anhaenge/ `ms` zurück (geholt sofort, ausgeliefert verzögert).
const anhaengeVerzoegern = (ms) => (page) => page.context().route('**/anhaenge/**', async (route) => {
  const antwort = await route.fetch();
  await new Promise((weiter) => setTimeout(weiter, ms));
  await route.fulfill({ response: antwort });
});

test('Bild im Dialog vor dem Laden (Nacharbeit F0102, ux-reviewer S1): 375 × 667 Touch, Original 4 s verzögert → nach 500 ms steht der Dialog in der eingepassten Bildgröße (≥ 200 × 150), „Schließen“ springt nach dem Laden nicht (± 1 px)', async ({ browser }) => {
  test.setTimeout(60000);
  const geraet = { viewport: { width: 375, height: 667 }, hasTouch: true, isMobile: true, deviceScaleFactor: 3 };
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png'] }], { geraet, vorSeite: anhaengeVerzoegern(4000) });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  await nachrichtMitZeit(page, lauf.zeiten[0]).getByRole('button', { name: 'foto.png groß anzeigen', exact: true }).tap();
  const dialog = page.getByRole('dialog', { name: 'foto.png', exact: true });
  await expect(dialog).toBeVisible();
  await page.waitForTimeout(500);
  const bild = dialog.getByRole('img', { name: 'foto.png', exact: true });
  expect(await bild.evaluate((element) => element.complete && element.naturalWidth > 0)).toBe(false);
  const flaeche = await rechteck(bild);
  expect(flaeche.width).toBeGreaterThanOrEqual(200);
  expect(flaeche.height).toBeGreaterThanOrEqual(150);
  const schliessen = dialog.getByRole('button', { name: 'Schließen', exact: true });
  const vorher = await rechteck(schliessen);
  await expect.poll(() => bild.evaluate((element) => element.complete && element.naturalWidth > 0), { timeout: 15000 }).toBe(true);
  const nachher = await rechteck(schliessen);
  expect(Math.abs(nachher.left - vorher.left)).toBeLessThanOrEqual(1);
  expect(Math.abs(nachher.top - vorher.top)).toBeLessThanOrEqual(1);
  expect(lauf.a.fehler).toEqual([]);
});

test('Bild im Dialog bei Fehler (Nacharbeit F0102, ux-reviewer S1): Original 404 → im Dialog „Bild nicht verfügbar.“', async ({ browser }) => {
  const geraet = { viewport: { width: 375, height: 667 }, hasTouch: true, isMobile: true, deviceScaleFactor: 3 };
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png'] }], {
    geraet, vorSeite: (page) => page.context().route('**/anhaenge/**', (route) => route.fulfill({ status: 404, body: '' })),
  });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  await nachrichtMitZeit(page, lauf.zeiten[0]).getByRole('button', { name: 'foto.png groß anzeigen', exact: true }).tap();
  const dialog = page.getByRole('dialog', { name: 'foto.png', exact: true });
  await expect(dialog.getByText('Bild nicht verfügbar.', { exact: true })).toBeVisible();
  expect(lauf.a.fehler.filter((zeile) => !/Failed to load resource/.test(zeile))).toEqual([]);
});
