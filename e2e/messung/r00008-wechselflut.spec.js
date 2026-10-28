'use strict';
// R00008 M3 Wechsel-Flut (Messwert im Entwicklertest, kein Gate; B0192): ein Agent setzt 10 s lang über eine
// Verbindung abwechselnd 7 und 3650; ein Zuschauer-Tab mit Seitenleiste misst wirksame Wechsel je Sekunde (aus
// `stand`), die längste Aufgabe (`longtask`) und den Anteil der Frames über 50 ms. Grenze: längste Aufgabe ≤ 250 ms.
// Vorher: ruhiger Rechner (uptime unter 2). Die Werte stehen in der Ausgabe.
const http = require('http');
const { test } = require('@playwright/test');
const { instanzStarten, beitretenPerApi, channelAnlegen } = require('../tests/hilfen');

const DAUER_MS = 10000;

let instanz;
test.beforeAll(async () => {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: '2026-10-28T12:00:00.000+01:00' },
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein'),
  });
});
test.afterAll(async () => instanz.stoppen());

// Eine Verbindung (keep-alive), Anfragen nacheinander; zählt die 200.
async function flut(adresse, token) {
  const agent = new http.Agent({ keepAlive: true, maxSockets: 1 });
  const ziel = new URL('api/channels/allgemein/frist', adresse);
  const ende = Date.now() + DAUER_MS;
  let gesetzt = 0;
  for (let runde = 0; Date.now() < ende; runde += 1) {
    let tage = 7;
    if (runde % 2 === 0) {
      tage = 3650;
    }
    const text = JSON.stringify({ tage });
    const status = await new Promise((fertig, gescheitert) => {
      const anfrage = http.request({
        host: ziel.hostname, port: ziel.port, path: ziel.pathname, method: 'PUT', agent, localAddress: '127.0.0.1',
        headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(text), 'X-LocalchatA-Sitzung': token },
      }, (antwort) => {
        antwort.resume();
        antwort.on('end', () => fertig(antwort.statusCode));
      });
      anfrage.once('error', gescheitert);
      anfrage.end(text);
    });
    if (status === 200) {
      gesetzt += 1;
    }
  }
  agent.destroy();
  return gesetzt;
}

test('M3: Wechsel-Flut 7 ↔ 3650 über 10 s', async ({ page }) => {
  await page.goto(`${instanz.adresse}#/c/allgemein`);
  await page.getByLabel('Name').fill('zuschauer');
  await page.getByLabel('Name').press('Enter');
  await page.locator('.lc-head-frist').waitFor();
  await page.evaluate(() => {
    window.messung = { aufgaben: [], frames: [], letzter: performance.now() };
    new PerformanceObserver((liste) => {
      for (const eintrag of liste.getEntries()) {
        window.messung.aufgaben.push(eintrag.duration);
      }
    }).observe({ type: 'longtask', buffered: false });
    const frame = (jetzt) => {
      window.messung.frames.push(jetzt - window.messung.letzter);
      window.messung.letzter = jetzt;
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  });
  const agent = await beitretenPerApi(instanz.adresse, 'flut');

  const gesetzt = await flut(instanz.adresse, agent.daten.token);

  const werte = await page.evaluate(() => window.messung);
  const ueber50 = werte.frames.filter((dauer) => dauer > 50).length;
  const laengste = Math.max(0, ...werte.aufgaben);
  console.log(`messung m3: ${(gesetzt / (DAUER_MS / 1000)).toFixed(1)} wirksame Wechsel/s, längste Aufgabe ${laengste.toFixed(0)} ms, `
    + `Frames über 50 ms ${ueber50} von ${werte.frames.length} (${((ueber50 / Math.max(1, werte.frames.length)) * 100).toFixed(2)} %)`);
});
