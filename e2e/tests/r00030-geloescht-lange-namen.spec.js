'use strict';
// R00030 F0183 / B0794 „Lange gelöschte Namen kürzen wie die heile Zeile“ — Abnahme-Probe wörtlich (ux-reviewer #43 F0118,
// Sollte S1): Touch 320×568, 375×667, 768×1024 und Desktop 1280×800, hell und dunkel; beta sendet „bericht mit einem sehr
// langen Namen aus dem dritten Quartal.pdf“ und „Screenshot 2026-10-30 um 14.02.11.png“ (je ≥ 29 KiB) und löscht sie. Je
// gelöschte Zeile: (a) scrollWidth ≤ clientWidth; (b) .lc-datei-name-ende rechts ≤ Zeilen-Rechtskante + 1; (c) Kürzel und
// Name oben gleich (±2 px); (d) „Gelöscht“ sichtbar innerhalb der Zeile; (e) höchstens 2 Textzeilen hoch. Der kurze Name
// „foto.png“ bleibt unverändert (Zeilenhöhe 40/40/28/28 px).
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, pngMitBytes } = require('./r00041-hilfen');
const { anzeigeStarten, anhaengeLoeschen, testdateien } = require('./r00030-anzeige-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const GERAETE = [
  { breite: 320, hoehe: 568, zeilenhoehe: 40, touch: true },
  { breite: 375, hoehe: 667, zeilenhoehe: 40, touch: true },
  { breite: 768, hoehe: 1024, zeilenhoehe: 28, touch: false },
  { breite: 1280, hoehe: 800, zeilenhoehe: 28, touch: false },
];
const LANG_PDF = 'bericht mit einem sehr langen Namen aus dem dritten Quartal.pdf';
const LANG_PNG = 'Screenshot 2026-10-30 um 14.02.11.png';

/// Die Maße einer gelöschten Zeile nach der Probe.
const messen = (zeile) => zeile.evaluate((element) => {
  const rahmen = element.getBoundingClientRect();
  const oben = (auswahl) => element.querySelector(auswahl).getBoundingClientRect();
  const marke = oben('.lc-datei-geloescht-marke');
  return {
    ueberlauf: element.scrollWidth - element.clientWidth,
    endeRechts: oben('.lc-datei-name-ende').right - rahmen.right,
    kuerzelOben: oben('.lc-datei-kuerzel').top,
    nameOben: oben('.lc-datei-name').top,
    markeLinks: marke.left - rahmen.left,
    markeRechts: marke.right - rahmen.right,
    markeSichtbar: marke.width > 0 && marke.height > 0,
    hoehe: rahmen.height,
    textzeile: parseFloat(getComputedStyle(element).lineHeight) || 20,
  };
});

for (const thema of ['light', 'dark']) {
  for (const geraet of GERAETE) {
    test(`Lange gelöschte Namen (${geraet.breite}×${geraet.hoehe}, ${thema}): (a)–(e), „foto.png“ unverändert hoch`, async ({ browser }) => {
      const viewport = { width: geraet.breite, height: geraet.hoehe };
      const optionen = geraet.touch ? { viewport, hasTouch: true, isMobile: true } : { viewport };
      const lauf = await anzeigeStarten(browser, [
        { namen: [{ name: 'foto.png', inhalt: pngMitBytes(30000) }, { name: LANG_PDF, inhalt: testdateien()['bericht.pdf'] }, { name: LANG_PNG, inhalt: pngMitBytes(30000) }] },
      ], { geraet: optionen });
      instanz = lauf.instanz;
      const { page } = lauf.a;
      await anhaengeLoeschen(instanz.adresse, lauf.beta.token, lauf.ids[0]);
      await page.emulateMedia({ colorScheme: thema });
      await page.reload();
      const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
      await knoten.waitFor();
      await expect(page.locator('html')).toHaveAttribute('data-theme', thema);

      const zeilen = knoten.locator('.lc-datei-geloescht');
      await expect(zeilen).toHaveCount(3);
      const kurz = await messen(zeilen.nth(0));
      expect(kurz.hoehe).toBe(geraet.zeilenhoehe);
      for (const stelle of [1, 2]) {
        const lage = await messen(zeilen.nth(stelle));
        expect(lage.ueberlauf, `(a) Überlauf Zeile ${stelle}`).toBeLessThanOrEqual(0);
        expect(lage.endeRechts, `(b) Namensende Zeile ${stelle}`).toBeLessThanOrEqual(1);
        expect(Math.abs(lage.kuerzelOben - lage.nameOben), `(c) Kürzel und Name Zeile ${stelle}`).toBeLessThanOrEqual(2);
        expect(lage.markeSichtbar, `(d) sichtbar Zeile ${stelle}`).toBe(true);
        expect(lage.markeLinks, `(d) links Zeile ${stelle}`).toBeGreaterThanOrEqual(-1);
        expect(lage.markeRechts, `(d) rechts Zeile ${stelle}`).toBeLessThanOrEqual(1);
        expect(lage.hoehe, `(e) Höhe Zeile ${stelle}`).toBeLessThanOrEqual(Math.max(2 * lage.textzeile, geraet.zeilenhoehe) + 1);
      }
      expect(lauf.a.fehler).toEqual([]);
    });
  }
}
