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

module.exports = { juengsteZeit, neuesteSichtbarNachKlick };
