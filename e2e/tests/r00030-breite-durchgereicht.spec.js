'use strict';
// R00030 F0102/F0165 Nacharbeit (code-reviewer #43 F0165 S5): die gemessene Inhaltsbreite geht in die Rechnung der
// Darstellung — `width`/`height` des Bilds und des Videos sind die Maße, die das Layout ihnen gibt (UX V1), nicht die
// 360 px des Höchstwerts. Touch 320 × 568 und 375 × 667, Gegeben über die API.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { anzeigeStarten } = require('./r00030-anzeige-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

for (const [breite, hoehe] of [[320, 568], [375, 667]]) {
  test(`Echte Inhaltsbreite (code S5, ${breite} px, Touch): width/height von Bild und Video sind die gerenderten Maße`, async ({ browser }) => {
    const geraet = { viewport: { width: breite, height: hoehe }, hasTouch: true, isMobile: true };
    const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png', 'film.webm'] }], { geraet });
    instanz = lauf.instanz;
    const knoten = nachrichtMitZeit(lauf.a.page, lauf.zeiten[0]);
    const bild = knoten.locator('.lc-vorschau img');
    const video = knoten.locator('video');
    await expect(bild).toBeVisible();

    const masse = async (locator) => locator.evaluate((element) => {
      const lage = element.getBoundingClientRect();
      return { attributBreite: Number(element.getAttribute('width')), attributHoehe: Number(element.getAttribute('height')), breite: lage.width, hoehe: lage.height };
    });
    const b = await masse(bild);
    const v = await masse(video);
    expect(b.breite).toBeLessThan(360);
    expect(Math.abs(b.attributBreite - b.breite), `Bild width ${b.attributBreite} gegen ${b.breite}`).toBeLessThanOrEqual(1);
    expect(Math.abs(b.attributHoehe - b.hoehe), `Bild height ${b.attributHoehe} gegen ${b.hoehe}`).toBeLessThanOrEqual(1);
    expect(Math.abs(v.attributBreite - v.breite), `Video width ${v.attributBreite} gegen ${v.breite}`).toBeLessThanOrEqual(1);
    expect(Math.abs(v.attributHoehe - v.hoehe), `Video height ${v.attributHoehe} gegen ${v.hoehe}`).toBeLessThanOrEqual(1);
    expect(lauf.a.fehler).toEqual([]);
  });
}
