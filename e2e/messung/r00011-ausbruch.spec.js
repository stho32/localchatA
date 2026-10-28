'use strict';
// R00011 Z9 (B0229) — Messwert im Entwicklertest, kein Gate: B steht am Ende von `# allgemein`; 4 Agenten einer
// Herkunft senden je 30 kleine Nachrichten so schnell wie möglich (120 = Vorrat der Herkunft). Gemessen in B: längste
// Aufgabe (longtask) und Anteil der Frames über 50 ms. Grenze laut Anforderung: keine Aufgabe über 250 ms, höchstens
// 2 % der Frames über 50 ms — darüber ein Befund an ADR-0037, kein roter Test. Vorher: ruhiger Rechner (uptime < 2).
// npx playwright test --config messung.config.js r00011-ausbruch
const { test, expect } = require('@playwright/test');
const { sendenStarten, beitreten, senden, imBrowserBeitreten, nachrichten } = require('../tests/senden-hilfen');

let instanz;
test.afterAll(async () => instanz && instanz.stoppen());

test('Z9: Ausbruch von 120 Nachrichten zeichnen', async ({ page }) => {
  instanz = await sendenStarten({ anzahl: 100 });
  await imBrowserBeitreten(page, instanz.adresse, 'anna');
  await expect(nachrichten(page)).toHaveCount(100);
  await page.evaluate(() => {
    window.lcMessung = { aufgaben: [], frames: [], laeuft: true };
    new PerformanceObserver((liste) => {
      liste.getEntries().forEach((eintrag) => window.lcMessung.aufgaben.push(eintrag.duration));
    }).observe({ type: 'longtask', buffered: false });
    let vorher = performance.now();
    const bild = (jetzt) => {
      window.lcMessung.frames.push(jetzt - vorher);
      vorher = jetzt;
      if (window.lcMessung.laeuft) {
        requestAnimationFrame(bild);
      }
    };
    requestAnimationFrame(bild);
  });
  const agenten = [];
  for (let nummer = 0; nummer < 4; nummer += 1) {
    agenten.push(await beitreten(instanz.adresse, `ausbruch${nummer}`, { herkunft: '127.0.0.4' }));
  }

  const status = await Promise.all(agenten.map(async (token, nummer) => {
    const ergebnisse = [];
    for (let folge = 0; folge < 30; folge += 1) {
      ergebnisse.push((await senden(instanz.adresse, token, 'allgemein', `a${nummer}-${folge}`, '127.0.0.4')).status);
    }
    return ergebnisse;
  }));
  const angekommen = status.flat().filter((wert) => wert === 201).length;
  await expect(nachrichten(page)).toHaveCount(100 + angekommen, { timeout: 30000 });
  await page.waitForTimeout(500);
  const werte = await page.evaluate(() => {
    window.lcMessung.laeuft = false;
    return window.lcMessung;
  });

  const ueber50 = werte.frames.filter((dauer) => dauer > 50).length;
  const laengste = Math.max(0, ...werte.aufgaben);
  console.log(`messung z9: ${angekommen} angekommen, ${werte.frames.length} Frames, über 50 ms ${ueber50} (${((ueber50 / werte.frames.length) * 100).toFixed(2)} %), lange Aufgaben ${werte.aufgaben.length}, längste ${laengste.toFixed(0)} ms (Grenzen: 2 %, 250 ms)`);
});
