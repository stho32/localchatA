'use strict';
// Messhilfen für den Verlauf (R00007 B0157 Öffnungszeit, B0167 Frames beim Scrollen): Gate L2 und Messwerte M1/M2
// für den Entwicklertest. Gemessen wird im Browser, ereignisgenau (requestAnimationFrame).
const http = require('http');
const { beitretenPerApi } = require('./hilfen');

/// Die Zeit der jüngsten Nachricht eines Channels — über die API, als Agent von 127.0.0.9.
async function juengsteZeit(adresse, channel) {
  const beitritt = await beitretenPerApi(adresse, 'messung', '127.0.0.9');
  const ziel = new URL(`api/channels/${channel}/nachrichten?n=1`, adresse);
  const block = await new Promise((fertig, gescheitert) => {
    http.get({
      host: ziel.hostname, port: ziel.port, path: `${ziel.pathname}${ziel.search}`, localAddress: '127.0.0.9',
      headers: { 'X-LocalchatA-Sitzung': beitritt.daten.token },
    }, (antwort) => {
      let text = '';
      antwort.on('data', (teil) => { text += teil; });
      antwort.on('end', () => fertig(JSON.parse(text)));
    }).once('error', gescheitert);
  });
  return block.nachrichten[0].zeit;
}

/// Klickt `# name` in der Seitenleiste und liefert die Millisekunden vom Zeitstempel des click bis zum ersten Bild,
/// in dem die Nachricht mit `zeit` samt Absender und Uhrzeit ganz im Verlauf sichtbar ist (Uhr des Browsers).
async function neuesteSichtbarNachKlick(page, name, zeit) {
  const verweis = page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true });
  await verweis.waitFor();
  await page.evaluate((channel) => {
    window.lcKlickZeit = null;
    const verweis = document.querySelector(`nav[aria-label="Channels"] a[href="#/c/${channel}"]`);
    verweis.addEventListener('click', (ereignis) => { window.lcKlickZeit = ereignis.timeStamp; }, { once: true, capture: true });
  }, name);
  await verweis.click();
  const ergebnis = await page.waitForFunction((gesucht) => {
    const nachricht = document.querySelector(`[role="log"] .lc-msg[data-zeit="${gesucht}"]`);
    if (window.lcKlickZeit === null || nachricht === null) {
      return false;
    }
    const log = nachricht.closest('[role="log"]').getBoundingClientRect();
    const lage = nachricht.getBoundingClientRect();
    const ganz = lage.top >= log.top - 1 && lage.bottom <= log.bottom + 1;
    const mitKopf = nachricht.querySelector('.lc-id') !== null || nachricht.querySelector('time') !== null;
    if (!ganz || !mitKopf) {
      return false;
    }
    return performance.now() - window.lcKlickZeit;
  }, zeit, { polling: 'raf', timeout: 10000 });
  return Math.round(await ergebnis.jsonValue());
}

/// Die Zeiten eines Channels blockweise rückwärts über die API (Blöcke zu 100, der neueste zuerst), je Block
/// älteste zuerst — die Erwartung für Fenster und Lückenlosigkeit.
async function bloeckeRueckwaerts(adresse, channel, anzahl) {
  const beitritt = await beitretenPerApi(adresse, 'zaehler', '127.0.0.8');
  const bloecke = [];
  let cursor = '';
  for (let nummer = 0; nummer < anzahl; nummer += 1) {
    const block = await apiBlock(adresse, channel, cursor, beitritt.daten.token);
    const zeiten = block.nachrichten.map((nachricht) => nachricht.zeit);
    bloecke.push(zeiten);
    cursor = `vor=${encodeURIComponent(zeiten[0])}&`;
  }
  return bloecke;
}

function apiBlock(adresse, channel, cursor, token) {
  const ziel = new URL(`api/channels/${channel}/nachrichten?${cursor}n=100`, adresse);
  return new Promise((fertig, gescheitert) => {
    http.get({
      host: ziel.hostname, port: ziel.port, path: `${ziel.pathname}${ziel.search}`, localAddress: '127.0.0.8',
      headers: { 'X-LocalchatA-Sitzung': token },
    }, (antwort) => {
      let text = '';
      antwort.on('data', (teil) => { text += teil; });
      antwort.on('end', () => fertig(JSON.parse(text)));
    }).once('error', gescheitert);
  });
}

/// Beobachtet im Browser den Anker (ADR-0026 Regel 3): die oberste sichtbare Nachricht wird bei jedem Scrollen
/// gemerkt; bei jeder Änderung im Verlauf und bei jedem Ein- und Ausblenden der Ladezeile wird ihre Verschiebung
/// festgehalten (window.lcSpruenge), dazu jede gesehene Zeit (window.lcGesehen) und ob die Zeiten im DOM je
/// streng steigend waren (window.lcOrdnungVerletzt).
async function ankerBeobachten(page) {
  await page.evaluate(() => {
    const log = document.querySelector('[role="log"]');
    const inhalt = log.querySelector('.lc-log-inhalt');
    function oberste() {
      const rand = log.getBoundingClientRect().top;
      const nachricht = [...inhalt.querySelectorAll('.lc-msg')].find((element) => element.getBoundingClientRect().bottom > rand);
      if (nachricht === undefined) {
        return null;
      }
      return { element: nachricht, top: nachricht.getBoundingClientRect().top };
    }
    function zeitenPruefen() {
      const zeiten = [...inhalt.querySelectorAll('.lc-msg')].map((element) => element.dataset.zeit);
      zeiten.forEach((zeit) => window.lcGesehen.add(zeit));
      for (let index = 1; index < zeiten.length; index += 1) {
        if (!(Date.parse(zeiten[index - 1]) < Date.parse(zeiten[index]))) {
          window.lcOrdnungVerletzt = true;
        }
      }
    }
    window.lcSpruenge = [];
    window.lcGesehen = new Set();
    window.lcOrdnungVerletzt = false;
    let letzte = oberste();
    zeitenPruefen();
    // Im Einfangen, also vor den Zuhörern des Verlaufs selbst: sonst sähe eine Änderung, die das Scrollen auslöst
    // (Ladezeile), noch die Lage vor dem Scrollen.
    document.addEventListener('scroll', (ereignis) => {
      if (ereignis.target === log) {
        letzte = oberste();
      }
    }, { passive: true, capture: true });
    const messen = () => {
      if (letzte !== null && letzte.element.isConnected) {
        window.lcSpruenge.push(letzte.element.getBoundingClientRect().top - letzte.top);
      }
      zeitenPruefen();
      letzte = oberste();
    };
    new MutationObserver(messen).observe(inhalt, { childList: true, subtree: true });
    const ladezeile = document.querySelector('.lc-loadmore');
    new MutationObserver(messen).observe(ladezeile, { attributes: true });
  });
}

/// Wartet, bis die Seite ausgeladen hat: keine laufende Anfrage auf /nachrichten und zwei Bilder Ruhe.
async function ruhig(page) {
  await page.waitForFunction(() => document.querySelector('.lc-loadmore').hidden && !document.querySelector('[role="log"]').dataset.laedt,
    null, { polling: 'raf', timeout: 10000 });
  await page.evaluate(() => new Promise((weiter) => requestAnimationFrame(() => requestAnimationFrame(weiter))));
}

/// Messwert M1 (B0167): Mausrad nach oben über `anzahl` Nachrichten (1.000 px je 16 ms wie die Probe der ADR-0026);
/// gemessen werden die Abstände der requestAnimationFrame-Takte und die langen Aufgaben (longtask).
/// Braucht ankerBeobachten (zählt die gesehenen Nachrichten). → { frames, ueber50, anteilUeber50, laengsteAufgabe }
async function framesBeimScrollen(page, anzahl) {
  await page.evaluate(() => {
    window.lcFrames = [];
    window.lcAufgaben = [];
    window.lcMessen = true;
    new PerformanceObserver((liste) => {
      for (const eintrag of liste.getEntries()) {
        window.lcAufgaben.push(eintrag.duration);
      }
    }).observe({ type: 'longtask' });
    let vorher = performance.now();
    const takt = (jetzt) => {
      window.lcFrames.push(jetzt - vorher);
      vorher = jetzt;
      if (window.lcMessen) {
        requestAnimationFrame(takt);
      }
    };
    requestAnimationFrame(takt);
  });
  const ziel = await page.evaluate(() => window.lcGesehen.size);
  const log = await page.getByRole('log').boundingBox();
  await page.mouse.move(log.x + log.width / 2, log.y + log.height / 2);
  for (let runde = 0; runde < 20000; runde += 1) {
    if (await page.evaluate((grenze) => window.lcGesehen.size >= grenze, ziel + anzahl)) {
      break;
    }
    await page.mouse.wheel(0, -1000);
    await page.waitForTimeout(16);
  }
  return page.evaluate(() => {
    window.lcMessen = false;
    const frames = window.lcFrames.slice(1);
    const ueber50 = frames.filter((dauer) => dauer > 50).length;
    return {
      frames: frames.length,
      ueber50,
      anteilUeber50: ueber50 / frames.length,
      laengsteAufgabe: Math.max(0, ...window.lcAufgaben),
      aufgaben: window.lcAufgaben.length,
    };
  });
}

module.exports = { juengsteZeit, neuesteSichtbarNachKlick, bloeckeRueckwaerts, ankerBeobachten, ruhig, framesBeimScrollen };
