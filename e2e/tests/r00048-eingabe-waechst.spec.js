'use strict';
// R00048 F0164 (Karte #80): wächst oder schrumpft das Feld der Eingabe, hält der Verlauf seine Lesestelle — die
// Nachricht, die vorher ganz sichtbar war, bleibt es, und beim Schrumpfen springt nichts. Gegen das echte Programm,
// `allgemein` mit 30 Nachrichten; `anna` liest im Telefon (375 × 667, 320 × 568, hasTouch, isMobile).
const { test, expect } = require('./rahmen');
const { fehlerSammeln } = require('./hilfen');
const { sendenStarten, imBrowserBeitreten, eingabe, nachrichten } = require('./senden-hilfen');

const TOLERANZ_PX = 1;
const TELEFONE = [
  { name: 'Telefon 375 × 667', viewport: { width: 375, height: 667 } },
  { name: 'Kleines Telefon 320 × 568', viewport: { width: 320, height: 568 } },
];
/// Wo `anna` vor dem Wachsen liest: Abstand der Lesestelle zum Ende des Inhalts als Anteil des Wegs (`anteil`) plus
/// feste px (`px`), vom Ende aus gerechnet.
const LESESTELLEN = [
  { name: 'am Ende', anteil: 0, px: 0 },
  { name: 'eine Zeilenhöhe über dem Ende', anteil: 0, px: 20 },
  { name: 'in der Mitte', anteil: 0.5, px: 0 },
];
const ZEILEN = [3, 5];

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Wie die Probe der Karte: Stelle der Nachricht Nr. `index` im Verlauf — Unterkante sichtbar (über der Eingabe), Abstand
/// ihrer Unterkante zur Unterkante des Verlaufs, Stand von `scrollTop` und der Pille.
function lageMessen(page, index) {
  return page.evaluate((nr) => {
    const log = document.querySelector('[role="log"]');
    const feld = document.querySelector('textarea');
    const nachricht = log.querySelectorAll('.lc-msg')[nr].getBoundingClientRect();
    const sicht = log.getBoundingClientRect();
    const pille = document.querySelector('.lc-pillen .lc-pill');
    return {
      unterkanteSichtbar: nachricht.bottom <= feld.getBoundingClientRect().top + 0.5 && nachricht.bottom <= sicht.bottom + 0.5,
      abstand: sicht.bottom - nachricht.bottom,
      kanten: [nachricht.top, nachricht.bottom, sicht.top, sicht.bottom, feld.getBoundingClientRect().top],
      scrollTop: log.scrollTop,
      sichthoehe: log.clientHeight,
      pille: pille.hidden ? '' : pille.textContent,
    };
  }, index);
}

/// Index der letzten Nachricht, deren Unterkante im Verlauf steht (hohe Nachrichten ragen oben über den Rand; wo
/// der Verlauf kürzer wird, verlangt die Probe nur die Unterkante: ≤ Oberkante der Eingabe).
function letzteGanzSichtbare(page) {
  return page.evaluate(() => {
    const log = document.querySelector('[role="log"]');
    const sicht = log.getBoundingClientRect();
    const liste = [...log.querySelectorAll('.lc-msg')];
    return liste.findLastIndex((nachricht) => {
      const lage = nachricht.getBoundingClientRect();
      return lage.bottom > sicht.top && lage.bottom <= sicht.bottom;
    });
  });
}

async function startenBei(browser, telefon, lesestelle) {
  instanz = await sendenStarten({ anzahl: 30 });
  const kontext = await browser.newContext({ viewport: telefon.viewport, hasTouch: true, isMobile: true });
  const page = await kontext.newPage();
  const fehler = fehlerSammeln(page, { streng: true });
  await imBrowserBeitreten(page, instanz.adresse, 'anna');
  await expect(nachrichten(page)).toHaveCount(30);
  const log = page.getByRole('log');
  await log.evaluate((element, { anteil, px }) => {
    const weg = element.scrollHeight - element.clientHeight;
    element.scrollTop = weg - anteil * weg - px;
  }, lesestelle);
  await expect.poll(() => letzteGanzSichtbare(page)).toBeGreaterThan(0);
  return { page, fehler };
}

/// Schreibt `zeilen` Zeilen ins Feld (Shift Enter zwischen den Zeilen).
async function zeilenSchreiben(page, zeilen) {
  const feld = eingabe(page);
  await feld.fill('');
  await feld.focus();
  for (let nr = 1; nr <= zeilen; nr += 1) {
    await page.keyboard.type(`zeile ${nr}`);
    if (nr < zeilen) {
      await page.keyboard.press('Shift+Enter');
    }
  }
}

// Ursache zu F0164: beim Messen setzt das Feld seine Höhe kurz zurück und liest `scrollHeight`; der Verlauf wird dabei
// kurz höher, der Browser klemmt `scrollTop` und schiebt die Lesestelle — auch wenn die Höhe des Felds am Ende
// dieselbe ist. Steht `anna` höchstens so weit vom Ende, wie das Feld über einer Zeile hoch ist, kippt sie so aus
// dem Ende, ohne dass sich eine Größe geändert hätte.
for (const abstandVomEnde of [0, 20]) {
  test(`Telefon 375 × 667: ein Zeichen im dreizeiligen Feld verschiebt den Verlauf nicht (${abstandVomEnde} px vor dem Ende)`, async ({ browser }) => {
    const { page, fehler } = await startenBei(browser, TELEFONE[0], LESESTELLEN[0]);
    await zeilenSchreiben(page, 3);
    const log = page.getByRole('log');
    await log.evaluate((element, abstand) => { element.scrollTop = element.scrollHeight - element.clientHeight - abstand; }, abstandVomEnde);
    await expect.poll(() => log.evaluate((element) => element.scrollHeight - element.clientHeight - element.scrollTop), { intervals: [25] })
      .toBeCloseTo(abstandVomEnde, 0);
    const vorher = await lageMessen(page, 0);

    await page.keyboard.type('x');
    await page.evaluate(() => new Promise((fertig) => requestAnimationFrame(() => requestAnimationFrame(fertig))));

    const nachher = await lageMessen(page, 0);
    expect(nachher.sichthoehe).toBe(vorher.sichthoehe);
    expect(Math.abs(nachher.scrollTop - vorher.scrollTop)).toBeLessThanOrEqual(TOLERANZ_PX);
    expect(fehler).toEqual([]);
  });
}
