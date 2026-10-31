'use strict';
// R00030 F0184 / B0797 „Selbstblockade-Hinweis nennt den Platz, „Entfernen“ in eigener Zeile“ — Abnahme-Probe wörtlich
// (ux-reviewer #85 Vorprüfung, korrigiert): Touch 320×568, 375×667 und Desktop; Aufbau --anhangquote 10MiB, anhang_belegt
// 10.280.960, 4 Dateien zu 1 KiB. (a) Größe in Worten + „bleiben“/„frei“; (b) „Entfernen“ unter dem Text, scrollWidth ok,
// ≥ 44 px bei grobem Zeiger; (c) der Hinweis enthält „mindestens 64 KiB“; (d) der Knopf trifft elementFromPoint auch nach
// 500 ms, der Verlauf steht am Ende. (a), (b), (d) sind Wächter, (c) ist die rote Probe.
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { anhangStarten, waehlen, perApi, betaBeitreten, perApiAblegen } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const MIB = 1024 * 1024;
const FENSTER = [
  { name: 'Touch 320×568', geraet: 'telefon', fenster: { width: 320, height: 568 }, touch: true },
  { name: 'Touch 375×667', geraet: 'telefon', fenster: { width: 375, height: 667 }, touch: true },
  { name: 'Desktop 1280×800', geraet: 'desktop', fenster: { width: 1280, height: 800 }, touch: false },
];
const kleineDatei = (name) => ({ name, mimeType: 'application/octet-stream', buffer: Buffer.alloc(1024, 7) });

for (const fenster of FENSTER) {
  test(`Selbstblockade-Hinweis (${fenster.name}): Platz in Worten, Mindestverbrauch, „Entfernen“ unter dem Text und treffbar`, async ({ browser }) => {
    test.setTimeout(150000);
    instanz = await anhangStarten();
    const token = await betaBeitreten(instanz.adresse);
    await perApiAblegen(instanz.adresse, token, [...Array.from({ length: 9 }, () => MIB), 843776]);
    expect((await perApi(instanz.adresse, { methode: 'GET', pfad: 'api/gesundheit' })).daten.anhang_belegt).toBe(10280960);
    const { page, fehler } = await geraetOeffnen(browser, fenster.geraet, { fenster: fenster.fenster });
    await imBrowserBeitreten(page, instanz.adresse, 'shoff');

    await page.getByRole('textbox', { name: /^Nachricht an # / }).fill('viele');
    await waehlen(page, ['k1.bin', 'k2.bin', 'k3.bin', 'k4.bin'].map(kleineDatei));
    await knopf(page).click();
    const abgewiesen = page.getByRole('log').locator('.lc-hochladung-datei.lc-fehlgeschlagen');
    await expect(abgewiesen).toHaveCount(1, { timeout: 90000 });
    const grund = abgewiesen.locator('.lc-hochladung-grund');
    await page.waitForTimeout(500);

    const text = await grund.textContent();
    expect(text, '(a) Größe in Worten').toMatch(/\d+(,\d)? (KiB|MiB)/);
    expect(text, '(a) bleiben').toMatch(/bleiben|frei/);
    expect(text, '(c) Mindestverbrauch').toContain('mindestens 64 KiB');
    const entfernen = abgewiesen.getByRole('button', { name: 'k4.bin entfernen' });
    const lage = await page.evaluate(() => {
      const grundBox = document.querySelector('.lc-hochladung-datei.lc-fehlgeschlagen .lc-hochladung-grund').getBoundingClientRect();
      const schalter = document.querySelector('.lc-hochladung-datei.lc-fehlgeschlagen .lc-hochladung-schalter button');
      const box = schalter.getBoundingClientRect();
      const treffer = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
      const verlauf = document.querySelector('[role="log"]');
      return {
        grundUnten: grundBox.bottom, schalterOben: box.top, hoehe: box.height, breite: box.width,
        seite: document.documentElement.scrollWidth, fenster: window.innerWidth,
        trifft: treffer === schalter || schalter.contains(treffer),
        abstandUnten: verlauf.scrollHeight - verlauf.scrollTop - verlauf.clientHeight,
      };
    });
    expect(lage.schalterOben, '(b) Entfernen unter dem Text').toBeGreaterThanOrEqual(lage.grundUnten - 1);
    expect(lage.seite, '(b) kein Querscrollen').toBeLessThanOrEqual(lage.fenster);
    if (fenster.touch) {
      expect(lage.hoehe, '(b) Ziel Höhe').toBeGreaterThanOrEqual(44);
      expect(lage.breite, '(b) Ziel Breite').toBeGreaterThanOrEqual(44);
    }
    expect(lage.trifft, '(d) trifft elementFromPoint').toBe(true);
    expect(lage.abstandUnten, '(d) Verlauf am Ende').toBeLessThanOrEqual(2);
    await expect(entfernen).toBeVisible();
    expect(fehler.filter((zeile) => !/507/.test(zeile))).toEqual([]);
  });
}
