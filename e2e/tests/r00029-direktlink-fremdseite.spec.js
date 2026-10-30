'use strict';
// R00029 F0100 Karte #71 (ADR-0014 Ergänzung „Karte #71 (Chrome 154)“, Bedingung C5, Probe S4; Kriterium „CORP beim
// Direktlink“; US-39), in Playwright-Chromium und Google Chrome stable: eine fremde Seite (`localhost`, andere Site als
// `127.0.0.1`) kann ein Medium der Instanz weder einbetten noch lesen. Dieselbe Probe gegen einen Testserver mit den
// Mutanten (CORP `cross-origin`, `Access-Control-Allow-Origin` `*`, `null`, gespiegelt) schlägt je an — sie erkennt
// also, was sie ausschließen soll.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { test, expect } = require('./rahmen');
const { beitreten, kontextOeffnen } = require('./senden-hilfen');
const { ANHANG_CSP_MEDIUM, mitAllgemein, ablegen, medium, testserverStarten, testserverStoppen } = require('./r00029-direktlink-hilfen');

let instanz;
let werkstatt;
let server = [];
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
  if (werkstatt) {
    fs.rmSync(werkstatt, { recursive: true, force: true });
    werkstatt = null;
  }
  await Promise.all(server.map(testserverStoppen));
  server = [];
});

const SICHER = {
  eingebettet: 'nicht geladen',
  cors: 'TypeError',
  noCors: 'TypeError',
  ausNullIframe: 'TypeError',
  iframe: 'nicht geladen',
  fenster: 'nicht lesbar',
};

/// Die fremde Seite auf `localhost`: bettet das Medium als `<video>` und als `<iframe>` ein und trägt einen
/// `<iframe sandbox="allow-scripts">` (Ursprung `null`), der das Medium per `fetch` liest und das Ergebnis meldet.
async function fremdeSeiteStarten(quelle) {
  const nullIframe = `<script>fetch(${JSON.stringify(quelle)}).then((a) => a.arrayBuffer()).then((b) => parent.postMessage('gelesen ' + b.byteLength, '*'), (f) => parent.postMessage(f.name, '*'));</script>`;
  const seite = `<html><body>
<video src="${quelle}" muted></video>
<iframe id="direkt" src="${quelle}"></iframe>
<iframe sandbox="allow-scripts" srcdoc="${nullIframe.replace(/"/g, '&quot;')}"></iframe>
<script>window.ausNullIframe = new Promise((fertig) => addEventListener('message', (ereignis) => fertig(ereignis.data)));</script>
</body></html>`;
  const fremd = await testserverStarten('localhost', (_anfrage, antwort) => {
    antwort.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    antwort.end(seite);
  });
  server.push(fremd);
  return `http://localhost:${fremd.address().port}/`;
}

/// Was die fremde Seite vom Medium hat: eingebettet (nach 2,5 s `readyState` ≥ 1), gelesen per `fetch` `cors`,
/// `no-cors` (Bytes) und aus dem `null`-iframe, das `<iframe>` und ein Fenster per `window.open`.
async function wasDieFremdeSeiteHat(page, fremdeSeite, quelle) {
  await page.goto(fremdeSeite);
  await page.waitForTimeout(2500);
  const imDokument = await page.evaluate(async (ziel) => {
    const ergebnis = {};
    ergebnis.eingebettet = 'nicht geladen';
    if (document.querySelector('video').readyState >= 1) {
      ergebnis.eingebettet = 'geladen';
    }
    try {
      const antwort = await fetch(ziel, { mode: 'cors' });
      ergebnis.cors = `gelesen ${(await antwort.arrayBuffer()).byteLength}`;
    } catch (fehler) {
      ergebnis.cors = fehler.name;
    }
    try {
      const undurchsichtig = await fetch(ziel, { mode: 'no-cors' });
      ergebnis.noCors = `${undurchsichtig.type} ${(await undurchsichtig.arrayBuffer()).byteLength}`;
    } catch (fehler) {
      ergebnis.noCors = fehler.name;
    }
    ergebnis.ausNullIframe = await window.ausNullIframe;
    ergebnis.fenster = 'nicht lesbar';
    const fenster = window.open(ziel);
    await new Promise((fertig) => setTimeout(fertig, 1000));
    try {
      if (fenster && fenster.document.querySelector('video, audio')) {
        ergebnis.fenster = 'lesbar';
      }
    } catch (fehler) {
      ergebnis.fenster = 'nicht lesbar';
    }
    if (fenster) {
      fenster.close();
    }
    return ergebnis;
  }, quelle);
  const direkt = page.frames().find((rahmen) => rahmen.url() === quelle);
  let iframe = 'nicht geladen';
  if (direkt && await direkt.evaluate(() => document.querySelector('video, audio') !== null).catch(() => false)) {
    iframe = 'geladen';
  }
  return { ...imDokument, iframe };
}

test('eine fremde Seite bettet ein Medium der Instanz nicht ein und liest nichts', async ({ browser }) => {
  test.setTimeout(60000);
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-fremdseite-'));
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');
  const id = await ablegen(instanz.adresse, token, 'film.webm', medium(werkstatt, 'film.webm', ['-t', '2', '-c:v', 'libvpx', '-c:a', 'libopus']));
  const quelle = `${instanz.adresse}anhaenge/${id}`;
  const fremdeSeite = await fremdeSeiteStarten(quelle);
  const { page } = await kontextOeffnen(browser);

  expect(await wasDieFremdeSeiteHat(page, fremdeSeite, quelle)).toEqual(SICHER);
});

/// Die Mutanten der Kopfzeilen einer Medienantwort; je Mutante die Zeile der Probe, die anschlagen muss. `fenster`
/// hat keine Mutante: was `window.open` öffnet, liest eine fremde Seite nach der Same-Origin-Policy nie.
const MUTANTEN = [
  ['X-Frame-Options fehlt', (_anfrage) => ({ 'X-Frame-Options': undefined }), 'iframe'],
  ['CORP cross-origin', (_anfrage) => ({ 'Cross-Origin-Resource-Policy': 'cross-origin' }), 'eingebettet'],
  ['ACAO *', (_anfrage) => ({ 'Access-Control-Allow-Origin': '*' }), 'cors'],
  ['ACAO null', (_anfrage) => ({ 'Access-Control-Allow-Origin': 'null' }), 'ausNullIframe'],
  ['ACAO gespiegelt', (anfrage) => ({ 'Access-Control-Allow-Origin': anfrage.headers.origin || '*' }), 'cors'],
];

test('die Probe der fremden Seite schlägt bei jeder Mutante der Kopfzeilen an', async ({ browser }) => {
  test.setTimeout(120000);
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-fremdseite-'));
  const bytes = medium(werkstatt, 'film.webm', ['-t', '2', '-c:v', 'libvpx', '-c:a', 'libopus']);
  const { page } = await kontextOeffnen(browser);

  const ohneMutante = await wasDieFremdeSeiteHat(page, ...await medienserverMitFremderSeite(bytes, () => ({})));
  expect(ohneMutante, 'Testserver ohne Mutante wie das Programm').toEqual(SICHER);
  for (const [mutante, zusatz, anschlagen] of MUTANTEN) {
    const ergebnis = await wasDieFremdeSeiteHat(page, ...await medienserverMitFremderSeite(bytes, zusatz));
    expect(ergebnis[anschlagen], `${mutante}: ${JSON.stringify(ergebnis)}`).not.toBe(SICHER[anschlagen]);
  }
});

/// Ein Testserver auf `127.0.0.1`, der das Medium mit den Kopfzeilen des Programms (V3) und dem `zusatz` der Mutante
/// ausliefert (`undefined` entfernt eine Kopfzeile), dazu die fremde Seite; liefert fremde Seite und Quelle.
async function medienserverMitFremderSeite(bytes, zusatz) {
  const medienserver = await testserverStarten('127.0.0.1', (anfrage, antwort) => {
    const kopf = {
      'Content-Type': 'video/webm',
      'Content-Security-Policy': ANHANG_CSP_MEDIUM,
      'Cross-Origin-Resource-Policy': 'same-origin',
      'X-Content-Type-Options': 'nosniff',
      'X-Frame-Options': 'DENY',
      ...zusatz(anfrage),
    };
    const gesetzt = Object.entries(kopf).filter(([, wert]) => wert !== undefined);
    antwort.writeHead(200, Object.fromEntries(gesetzt));
    antwort.end(bytes);
  });
  server.push(medienserver);
  const quelle = `http://127.0.0.1:${medienserver.address().port}/film.webm`;
  return [await fremdeSeiteStarten(quelle), quelle];
}
