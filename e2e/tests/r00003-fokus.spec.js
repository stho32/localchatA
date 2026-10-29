'use strict';
// R00003 F0010, Nacharbeit UX-Review 1 (Muss): Fokus auf dem Primärschalter sichtbar, ≥ 3:1 gegen die
// angrenzende Fläche — für jeden .lc-btn.primary.
const { test, expect } = require('./rahmen');
const { instanzStarten } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const feld = (page) => page.getByLabel('Name');

// Kontrast nach WCAG 2.x aus zwei berechneten Farben `rgb(r, g, b)`.
function kontrastImBrowser() {
  function anteile(farbe) {
    return farbe.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number);
  }
  function leuchtdichte(farbe) {
    const [r, g, b] = anteile(farbe).map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }
  function flaecheUnter(element) {
    for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
      const farbe = getComputedStyle(knoten).backgroundColor;
      if (farbe !== 'rgba(0, 0, 0, 0)' && farbe !== 'transparent') {
        return farbe;
      }
    }
    return 'rgb(255, 255, 255)';
  }
  const schalter = document.activeElement;
  const stil = getComputedStyle(schalter);
  let angrenzend = flaecheUnter(schalter);
  if (parseFloat(stil.outlineOffset) > 0) {
    angrenzend = flaecheUnter(schalter.parentElement);
  }
  const hell = Math.max(leuchtdichte(stil.outlineColor), leuchtdichte(angrenzend));
  const dunkel = Math.min(leuchtdichte(stil.outlineColor), leuchtdichte(angrenzend));
  return {
    text: schalter.textContent,
    fokusSichtbar: schalter.matches(':focus-visible'),
    primaer: schalter.matches('.lc-btn.primary'),
    rahmen: `${stil.outlineStyle} ${stil.outlineWidth}`,
    kontrast: (hell + 0.05) / (dunkel + 0.05),
  };
}

for (const thema of ['light', 'dark']) {
  for (const breite of [480, 1280]) {
    test(`UX 1: Fokus auf Beitreten sichtbar mit Kontrast ≥ 3:1 — ${thema}, ${breite} px`, async ({ page }) => {
      instanz = await instanzStarten();
      await page.emulateMedia({ colorScheme: thema });
      await page.setViewportSize({ width: breite, height: 800 });
      await page.goto(instanz.adresse);
      await expect(feld(page)).toBeFocused();

      await page.keyboard.press('Tab');

      const fokus = await page.evaluate(kontrastImBrowser);
      expect(fokus.text).toBe('Beitreten');
      expect(fokus.fokusSichtbar).toBe(true);
      expect(fokus.primaer).toBe(true);
      expect(fokus.rahmen).toBe('solid 2px');
      expect(fokus.kontrast).toBeGreaterThanOrEqual(3);
    });
  }
}
