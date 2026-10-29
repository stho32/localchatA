'use strict';
// Hilfen für R00027 (Senden-Knopf und Enter auf Touch-Geräten): Geräte als emulierte Browser-Kontexte, nicht als
// verkleinerte Fenster — Telefon, Tablet, Tablet quer mit Touch, Desktop ohne; der Knopf „Senden“ in der Eingabe.
const { fehlerSammeln } = require('./hilfen');

const GERAETE = {
  telefon: { viewport: { width: 375, height: 812 }, hasTouch: true, isMobile: true, deviceScaleFactor: 3 },
  tablet: { viewport: { width: 768, height: 1024 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 },
  tabletQuer: { viewport: { width: 1024, height: 768 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 },
  desktop: { viewport: { width: 1280, height: 800 } },
};

/// Ein Kontext für `geraet` (Schlüssel aus GERAETE); `fenster` ersetzt die Fenstergröße (genäherte
/// Bildschirmtastatur), `thema` ist 'light' oder 'dark'; `streng` und `zusaetzlich` wie fehlerSammeln.
async function geraetOeffnen(browser, geraet, { fenster, thema = 'light', ...sammeln } = {}) {
  const einstellungen = { ...GERAETE[geraet], colorScheme: thema };
  if (fenster) {
    einstellungen.viewport = fenster;
  }
  const kontext = await browser.newContext(einstellungen);
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page, sammeln) };
}

const knopf = (page) => page.locator('.lc-comp').getByRole('button', { name: 'Senden', exact: true });

/// Die Box eines Elements im Fenster (getBoundingClientRect).
const box = (locator) => locator.evaluate((element) => element.getBoundingClientRect().toJSON());

/// Liegt das Element ganz im sichtbaren Ausschnitt (visualViewport) — mit genäherter Bildschirmtastatur?
function ganzImSichtbaren(locator) {
  return locator.evaluate((element) => {
    const rand = element.getBoundingClientRect();
    const sicht = window.visualViewport;
    return rand.top >= sicht.offsetTop && rand.left >= sicht.offsetLeft
      && rand.bottom <= sicht.offsetTop + sicht.height && rand.right <= sicht.offsetLeft + sicht.width;
  });
}

module.exports = { GERAETE, geraetOeffnen, knopf, box, ganzImSichtbaren };
