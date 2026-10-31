'use strict';
// R00030 F0184 / code-reviewer #85 B1 (Ursache des Wacklers in r00030-selbstblockade-hinweis): wächst der Verlauf am Ende
// ein zweites Mal, bevor das Scroll-Ereignis der ersten Nachführung verarbeitet ist, rechnete der Verlauf „nicht mehr am
// Ende“ und folgte nie wieder — der Knopf „Entfernen“ lag dauerhaft unter der Eingabe. Probe: am Ende des Verlaufs den
// Inhalt wachsen lassen und im ersten Scroll-Ereignis (Capture, vor dem Verlauf) ein zweites Mal → nach 500 ms steht der
// Verlauf wieder am Ende. Heute rot (Abstand bleibt bei der zweiten Höhe).
const { test, expect } = require('./rahmen');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten } = require('./senden-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

test('Verlauf am Ende folgt auch einem zweiten Wachstum vor dem Scroll-Ereignis der Nachführung', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 30 });
  const { page, fehler } = await kontextOeffnen(browser);
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  const log = page.getByRole('log');
  await expect.poll(() => log.evaluate((verlauf) => verlauf.scrollHeight - verlauf.scrollTop - verlauf.clientHeight), { intervals: [25] }).toBeLessThanOrEqual(1);

  await log.evaluate((verlauf) => {
    const platz = verlauf.querySelector('.lc-ausstehend');
    const wachsen = () => {
      const block = document.createElement('div');
      block.style.height = '200px';
      platz.append(block);
    };
    verlauf.addEventListener('scroll', wachsen, { capture: true, once: true });
    wachsen();
  });

  await page.waitForTimeout(500);
  const abstand = await log.evaluate((verlauf) => verlauf.scrollHeight - verlauf.scrollTop - verlauf.clientHeight);
  expect(abstand, 'Verlauf am Ende nach zwei Wachstumsschritten').toBeLessThanOrEqual(1);
  expect(fehler).toEqual([]);
});
