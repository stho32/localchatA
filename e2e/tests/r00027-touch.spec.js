'use strict';
// R00027 F0091 — die Eingabe mit dem Daumen und der Tastatur bedienbar (UX-Vorprüfung #41): Fokusring des Knopfs (M1,
// B0480), Schrift des Felds ohne Zoom auf Touch-Geräten (M2, B0481), Schalter an der fehlgeschlagenen Nachricht (S7,
// B0482). Geräte als emulierte Kontexte (r00027-hilfen.js).
const { test, expect } = require('@playwright/test');
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { ROUTE, bestaetigt, sendungenMitschreiben } = require('./r00012-hilfen');
const { geraetOeffnen, knopf, box } = require('./r00027-hilfen');

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

// „Kein Zoom beim Fokus“ (M2, wörtlich): „bei (pointer: coarse) Feld font-size ≥ 16 px, Desktop 13 px; Mutante „nur bei
// 375 px“ auf Tablet rot; Gerät: iPhone Safari scale = 1, Knopf ganz sichtbar; nicht über maximum-scale=1.“
for (const geraet of ['telefon', 'tablet', 'tabletQuer', 'desktop']) {
  test(`Kein Zoom beim Fokus (${geraet}): Schrift im Feld groß genug, kein Zoomverbot im Viewport`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, geraet);

    const schrift = parseFloat(await eingabe(page).evaluate((feld) => getComputedStyle(feld).fontSize));
    if (geraet === 'desktop') {
      expect(schrift).toBe(13);
    } else {
      expect(schrift).toBeGreaterThanOrEqual(16);
    }
    const viewport = await page.locator('meta[name="viewport"]').getAttribute('content');
    expect(viewport).not.toMatch(/maximum-scale|user-scalable/);
  });
}

// „Schalter der fehlgeschlagenen Nachricht“ (S7): Gegeben wie „Fehlschlag“ (die Route antwortet auf den ersten POST mit
// 500) → auf Touch-Geräten je ≥ 44 × 44 px und ≥ 8 px auseinander, am Desktop unverändert 28 px hoch; „Erneut senden“
// sendet genau einmal.
for (const geraet of ['telefon', 'tablet', 'desktop']) {
  test(`Schalter der fehlgeschlagenen Nachricht (${geraet}): Größe, Abstand, Erneut senden genau einmal`, async ({ browser }) => {
    const nachrichtenPfad = '/api/channels/allgemein/nachrichten';
    const { page, fehler } = await seiteAuf(browser, geraet, { zusaetzlich: { [nachrichtenPfad]: [500] } });
    const sendungen = sendungenMitschreiben(page);
    let erste = true;
    await page.route(ROUTE, async (route) => {
      if (route.request().method() !== 'POST' || !erste) {
        await route.continue();
        return;
      }
      erste = false;
      await route.fulfill({ status: 500, contentType: 'application/json', body: '{"fehler":"intern"}' });
    });

    await eingabe(page).fill('kaputt');
    await knopf(page).click();
    const erneut = page.getByRole('button', { name: 'Erneut senden' });
    const verwerfen = page.getByRole('button', { name: 'Verwerfen' });
    await expect(erneut).toBeVisible();
    const erneutBox = await box(erneut);
    const verwerfenBox = await box(verwerfen);
    if (geraet === 'desktop') {
      expect([erneutBox.height, verwerfenBox.height]).toEqual([28, 28]);
    } else {
      for (const schalter of [erneutBox, verwerfenBox]) {
        expect(schalter.width).toBeGreaterThanOrEqual(44);
        expect(schalter.height).toBeGreaterThanOrEqual(44);
      }
      const abstand = Math.max(verwerfenBox.left - erneutBox.right, verwerfenBox.top - erneutBox.bottom);
      expect(abstand).toBeGreaterThanOrEqual(8);
    }

    if (geraet === 'desktop') {
      await erneut.click();
    } else {
      await erneut.tap();
    }
    await expect(bestaetigt(page, 'kaputt')).toHaveCount(1);
    await page.waitForTimeout(300);
    expect(sendungen.map((sendung) => sendung.text)).toEqual(['kaputt', 'kaputt']);
    await expect(bestaetigt(page, 'kaputt')).toHaveCount(1);
    expect(fehler).toEqual([]);
  });
}
