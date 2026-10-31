'use strict';
// R00041 F0186 B0803 (Karte #85, ux-reviewer-Vorprüfung): Breite und Platz des Bearbeiten-Felds.
// Platzmangel: auf Touch bei 320 × 400 und 375 × 360 mit 124 Zeichen Text — Oberkante Feld ≥ Oberkante Verlauf, Unterkante
// beider Knöpfe ≤ Unterkante Verlauf, die Knöpfe treffen sich selbst, kein Querscrollen (das Feld darf schrumpfen und intern
// scrollen). Breite: bei 320 × 568 und 375 × 667 ist das Feld ≥ 80 % so breit wie der Verlauf (der 48-px-Rand rechts entfällt, es bleiben die 12 px der Nachricht).
// Wächter: 1 bis 3 Zeilen bei 320 × 568 und 375 × 667. Mutanten, die rot werden müssen: Rand rechts bleibt; Feldhöhe nicht an
// den Platz des Verlaufs gebunden.
const { test, expect } = require('./rahmen');
const { vermessen, imVerlauf, eigeneNachrichtBearbeiten } = require('./r00041-feld-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const TEXT_124 = 'Dies ist ein langer Text, der im Bearbeiten-Feld mehrere Zeilen braucht, damit der Platz im niedrigen Verlauf knapp wird. Ende.'.slice(0, 124);

async function bearbeiten(browser, breite, hoehe, text) {
  const gestartet = await eigeneNachrichtBearbeiten(browser, breite, hoehe, text);
  instanz = gestartet.instanz;
  return gestartet.a;
}

for (const [breite, hoehe] of [[320, 400], [375, 360]]) {
  test(`Platzmangel ${breite} × ${hoehe} (B0803): Feld und Knöpfe liegen ganz im niedrigen Verlauf, das Feld scrollt in sich`, async ({ browser }) => {
    const a = await bearbeiten(browser, breite, hoehe, TEXT_124);
    imVerlauf(await a.page.evaluate(vermessen));
    expect(a.fehler).toEqual([]);
  });
}

for (const [breite, hoehe] of [[320, 568], [375, 667]]) {
  test(`Breite ${breite} × ${hoehe} (B0803): das Feld ist mindestens 80 % so breit wie der Verlauf, ohne Rand rechts`, async ({ browser }) => {
    const a = await bearbeiten(browser, breite, hoehe, TEXT_124);
    const m = await a.page.evaluate(vermessen);
    imVerlauf(m);
    expect(m.feldBreite / m.logBreite).toBeGreaterThanOrEqual(0.8);
    const rand = await a.page.evaluate(() => getComputedStyle(document.querySelector('.lc-msg.lc-im-bearbeiten')).paddingRight);
    expect(rand).toBe('12px');
  });

  for (const zeilen of [1, 3]) {
    test(`Wächter ${breite} × ${hoehe} mit ${zeilen} Zeile${zeilen > 1 ? 'n' : ''} (B0803): alles im Verlauf`, async ({ browser }) => {
      const a = await bearbeiten(browser, breite, hoehe, Array.from({ length: zeilen }, (_, nummer) => `Zeile ${nummer + 1}`).join(' '));
      for (let weitere = 1; weitere < zeilen; weitere += 1) {
        await a.page.keyboard.press('Enter');
        await a.page.keyboard.type(`weiter ${weitere}`);
      }
      await a.page.waitForTimeout(200);
      imVerlauf(await a.page.evaluate(vermessen));
    });
  }
}
