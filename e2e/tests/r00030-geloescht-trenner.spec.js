'use strict';
// R00030 F0183 / B0810 „Trenner der gelöschten Zeile steht am Namen“ — Abnahme-Probe wörtlich (ux-reviewer #85 Vorprüfung,
// Optional #43 F0118 O1): Trenner ≤ 8 px am Namen, gleiche Zeile. Messung (planer): Abstand zwischen Namensende und Trenner
// ≤ 8 px, Trenner-top und Name-top gleich (±2 px). Heute steht der Trenner am Desktop 200+ px vor „300 KiB · Gelöscht“.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, pngMitBytes } = require('./r00041-hilfen');
const { anzeigeStarten, anhaengeLoeschen } = require('./r00030-anzeige-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const GERAETE = [
  { name: 'Desktop 1280×800', optionen: { viewport: { width: 1280, height: 800 } } },
  { name: 'Touch 375×667', optionen: { viewport: { width: 375, height: 667 }, hasTouch: true, isMobile: true } },
];
const NAMEN = ['foto.png', 'bericht mit einem sehr langen Namen aus dem dritten Quartal.png'];

for (const geraet of GERAETE) {
  test(`Trenner am Namen (${geraet.name}): höchstens 8 px hinter dem Namensende, in der Zeile des Namens`, async ({ browser }) => {
    const lauf = await anzeigeStarten(browser, [{ namen: NAMEN.map((name) => ({ name, inhalt: pngMitBytes(30000) })) }], { geraet: geraet.optionen });
    instanz = lauf.instanz;
    const { page } = lauf.a;
    await anhaengeLoeschen(instanz.adresse, lauf.beta.token, lauf.ids[0]);
    await page.reload();
    const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
    await knoten.waitFor();
    const zeilen = knoten.locator('.lc-datei-geloescht');
    await expect(zeilen).toHaveCount(2);

    for (const stelle of [0, 1]) {
      const lage = await zeilen.nth(stelle).evaluate((zeile) => {
        const rechteck = (auswahl) => zeile.querySelector(auswahl).getBoundingClientRect();
        const trenner = rechteck('.lc-datei-trenner');
        const ende = rechteck('.lc-datei-name-ende');
        return { abstand: trenner.left - ende.right, trennerOben: trenner.top, nameOben: rechteck('.lc-datei-name').top };
      });
      expect(lage.abstand, `Abstand Namensende–Trenner, Name ${stelle}`).toBeLessThanOrEqual(8);
      expect(lage.abstand, `Trenner nach dem Namen, Name ${stelle}`).toBeGreaterThanOrEqual(-1);
      expect(Math.abs(lage.trennerOben - lage.nameOben), `gleiche Zeile, Name ${stelle}`).toBeLessThanOrEqual(2);
    }
  });
}
