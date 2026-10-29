'use strict';
// R00027 F0091 — die Eingabe mit dem Daumen und der Tastatur bedienbar (UX-Vorprüfung #41): Fokusring des Knopfs (M1,
// B0480), Schrift des Felds ohne Zoom auf Touch-Geräten (M2, B0481), Schalter an der fehlgeschlagenen Nachricht (S7,
// B0482). Geräte als emulierte Kontexte (r00027-hilfen.js).
const { test, expect } = require('@playwright/test');
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');

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

// „Fokus sichtbar“ (M1, wörtlich): „Knopf nach Tab: outline nicht none, ≥ 2 px, offset ≥ 0, Abstand zur Innenkante der
// Box ≥ offset+width, Ringfarbe gegen kd-surface ≥ 3:1, auch „nicht verfügbar“; Mutante „ohne eigene Fokusregel“ rot.“
for (const thema of ['light', 'dark']) {
  for (const [geraet, text, zustand] of [['desktop', 'x', 'bereit'], ['desktop', '', 'nicht verfügbar'], ['tablet', 'x', 'bereit']]) {
    test(`Fokus sichtbar (${geraet}, ${thema}, ${zustand}): eigener Ring außen, ≥ 2 px, ganz in der Box, ≥ 3 : 1`, async ({ browser }) => {
      const { page } = await seiteAuf(browser, geraet, { thema });

      await eingabe(page).focus();
      await page.keyboard.type(text);
      await expect(page.locator('.lc-comp-box')).toHaveCSS('outline-style', 'solid');
      await page.keyboard.press('Tab');
      await expect(knopf(page)).toBeFocused();
      await expect(page.locator('.lc-comp-box')).toHaveCSS('outline-style', 'none');
      const ring = await knopf(page).evaluate(fokusringVermessen);

      expect(ring.sichtbar).toBe(true);
      expect(ring.stil).not.toBe('none');
      expect(ring.breite).toBeGreaterThanOrEqual(2);
      expect(ring.versatz).toBeGreaterThanOrEqual(0);
      for (const abstand of ring.abstaendeZurInnenkante) {
        expect(abstand).toBeGreaterThanOrEqual(ring.versatz + ring.breite);
      }
      expect(ring.kontrastGegenFlaeche).toBeGreaterThanOrEqual(3);
    });
  }
}

/// Ring des fokussierten Knopfs: Stil, Breite, Versatz, Abstände zur Innenkante von .lc-comp-box (links, oben,
/// rechts, unten) und Kontrast der Ringfarbe gegen die Fläche der Box (kd-surface).
function fokusringVermessen(knopfElement) {
  function leuchtdichte(farbe) {
    const [r, g, b] = farbe.match(/[\d.]+/g).slice(0, 3).map(Number).map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }
  const stil = getComputedStyle(knopfElement);
  const kasten = knopfElement.closest('.lc-comp-box');
  const aussen = kasten.getBoundingClientRect();
  const innenLinks = aussen.left + kasten.clientLeft;
  const innenOben = aussen.top + kasten.clientTop;
  const rand = knopfElement.getBoundingClientRect();
  const ring = leuchtdichte(stil.outlineColor);
  const flaeche = leuchtdichte(getComputedStyle(kasten).backgroundColor);
  return {
    sichtbar: knopfElement.matches(':focus-visible'),
    stil: stil.outlineStyle,
    breite: parseFloat(stil.outlineWidth),
    versatz: parseFloat(stil.outlineOffset),
    abstaendeZurInnenkante: [
      rand.left - innenLinks,
      rand.top - innenOben,
      innenLinks + kasten.clientWidth - rand.right,
      innenOben + kasten.clientHeight - rand.bottom,
    ],
    kontrastGegenFlaeche: (Math.max(ring, flaeche) + 0.05) / (Math.min(ring, flaeche) + 0.05),
  };
}
