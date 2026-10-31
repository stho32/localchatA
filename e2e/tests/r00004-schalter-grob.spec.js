'use strict';
// R00004 F0188 B0806 (Karte #85): der Schalter „Teilnehmer n“ ist bei grobem Zeiger mindestens 36 px hoch, ohne dass die
// Kopfzeile höher wird (320, 375, 414 px: 80 px, 768 px: 44 px) — im Channel wie im Direktgespräch. Mutante, die rot
// werden muss: Regel `@media (pointer: coarse) .lc-hbtn` entfernt. Wächter: mit feinem Zeiger bleibt der Schalter 28 px.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');
const { BEN, direktStarten, direktOeffnen, gespraechOffen } = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const BREITEN = [[320, 80], [375, 80], [414, 80], [768, 44]];

const grob = (breite) => ({ viewport: { width: breite, height: 700 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 });

/// Höhe des Schalters, der Kopfzeile, Querscrollen und ob die Mitte des Schalters ihn selbst trifft.
function schalterVermessen() {
  const schalter = document.querySelector('.lc-hbtn');
  const rand = schalter.getBoundingClientRect();
  const ziel = document.elementFromPoint(rand.left + rand.width / 2, rand.top + rand.height / 2);
  return {
    schalterHoehe: rand.height,
    kopfzeileHoehe: document.querySelector('.lc-head').getBoundingClientRect().height,
    quer: document.documentElement.scrollWidth > document.documentElement.clientWidth,
    getroffen: ziel !== null && schalter.contains(ziel),
  };
}

for (const [breite, kopfHoehe] of BREITEN) {
  test(`grober Zeiger ${breite} px im Channel (B0806): Schalter „Teilnehmer“ ≥ 36 px, Kopfzeile ${kopfHoehe} px, kein Querscrollen`, async ({ browser }) => {
    instanz = await sendenStarten();
    const a = await browserOeffnen(browser, { geraet: grob(breite) });
    await imBrowserBeitreten(a.page, instanz.adresse, 'anna');
    await expect(a.page.getByRole('banner').getByRole('button', { name: /^Teilnehmer/ })).toBeVisible();
    const gemessen = await a.page.evaluate(schalterVermessen);
    expect(gemessen.schalterHoehe).toBeGreaterThanOrEqual(36);
    expect(gemessen.kopfzeileHoehe).toBe(kopfHoehe);
    expect(gemessen.quer).toBe(false);
    expect(gemessen.getroffen).toBe(true);
  });

  test(`grober Zeiger ${breite} px im Direktgespräch (B0806): Schalter „Teilnehmer“ ≥ 36 px, Kopfzeile ${kopfHoehe} px`, async ({ browser }) => {
    const gestartet = await direktStarten(browser, { geraet: grob(breite) });
    instanz = gestartet.instanz;
    await direktOeffnen(gestartet.anna.page, BEN);
    await gespraechOffen(gestartet.anna.page, BEN);
    await expect(gestartet.anna.page.getByRole('banner').getByRole('button', { name: /^Teilnehmer/ })).toBeVisible();
    const gemessen = await gestartet.anna.page.evaluate(schalterVermessen);
    expect(gemessen.schalterHoehe).toBeGreaterThanOrEqual(36);
    expect(gemessen.kopfzeileHoehe).toBe(kopfHoehe);
    expect(gemessen.quer).toBe(false);
    expect(gemessen.getroffen).toBe(true);
  });
}

test('feiner Zeiger 1280 px (Wächter B0806): der Schalter bleibt 28 px hoch', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'anna');
  await expect(a.page.getByRole('banner').getByRole('button', { name: /^Teilnehmer/ })).toBeVisible();
  expect((await a.page.evaluate(schalterVermessen)).schalterHoehe).toBe(28);
});
