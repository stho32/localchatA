'use strict';
// R00029 F0100 Karte #71 (ADR-0014 Ergänzung „Karte #71 (Chrome 154)“, Bedingungen C2–C4, Proben S2, S3; Kriterium
// „Kein Skript beim Direktlink“; US-39), in Playwright-Chromium und Google Chrome stable: das Medien-Dokument läuft im
// Ursprung der Instanz, aber ohne Skript; Nicht-Medien bleiben in ihrem undurchsichtigen Ursprung. Der Sentinel
// schreibt nur Titel, `window.__x` und `localStorage` — ein `fetch` oder `<img>` sperrte `default-src 'none'` auch
// bei laufendem Skript. Das Kontrollpaar über den Testserver beweist, dass er anschlägt, wenn Skript läuft.
// `allow-scripts` in der Medien-CSP erreicht diese Spec nur über den Testserver; die Wache dafür ist der
// Wortlaut-Test `tests/anhaenge_medien_csp.rs` (C1).
const fs = require('fs');
const os = require('os');
const path = require('path');
const { test, expect } = require('./rahmen');
const { beitreten, kontextOeffnen, imBrowserBeitreten } = require('./senden-hilfen');
const {
  ANHANG_CSP, ANHANG_CSP_MEDIUM, mitAllgemein, ablegen, medium, testserverStarten, testserverStoppen,
} = require('./r00029-direktlink-hilfen');

let instanz;
let werkstatt;
let testserver;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
  if (werkstatt) {
    fs.rmSync(werkstatt, { recursive: true, force: true });
    werkstatt = null;
  }
  if (testserver) {
    await testserverStoppen(testserver);
    testserver = null;
  }
});

const SKRIPT = "window.__x = 1; document.title = 'ausgefuehrt'; try { localStorage.setItem('fremd', '1'); } catch (e) {}";
const HTML = `<html><head><title>leer</title></head><body><script>${SKRIPT}</script>Hallo</body></html>`;
const ANGESCHLAGEN = { x: 1, titel: 'ausgefuehrt', fremd: '1' };
const STILL = { x: null, titel: expect.not.stringMatching(/ausgefuehrt/), fremd: null };

/// Die sechs Signaturen (C3): je eine kurze Datei aus ffmpeg; die Art bestimmt der Server aus dem Inhalt.
const SIGNATUREN = [
  ['film.mp4', ['-t', '1', '-c:v', 'libvpx-vp9', '-deadline', 'realtime', '-cpu-used', '8', '-c:a', 'libopus'], 'medium'],
  ['ton.m4a', ['-t', '1', '-c:a', 'aac'], 'medium'],
  ['film.webm', ['-t', '1', '-c:v', 'libvpx', '-c:a', 'libopus'], 'medium'],
  ['ton.ogg', ['-t', '1', '-c:a', 'libopus'], 'medium'],
  ['film.ogv', ['-t', '1', '-c:v', 'libtheora', '-c:a', 'libopus'], 'laden'],
  ['ton.mp3', ['-t', '1'], 'medium'],
];

/// Ogg-Seite mit `OpusHead`: die Instanz erkennt daran `audio/ogg`.
function oggOpusKopf() {
  const paket = Buffer.concat([Buffer.from('OpusHead'), Buffer.from([1, 2, 0x38, 1, 0x80, 0xbb, 0, 0, 0, 0, 0])]);
  return Buffer.concat([Buffer.from('OggS'), Buffer.from([0, 2]), Buffer.alloc(20), Buffer.from([1, paket.length]), paket]);
}

/// Das Skript-HTML als Anhang an Medien-Bytes; bei MP4 und M4A in einer `free`-Box, denn eine lose angehängte
/// Folge verwirft schon die Prüfung beim Hochladen (Längen widersprechen sich).
function mitSkriptHtml(name, bytes) {
  const html = Buffer.from(HTML);
  if (!/\.(mp4|m4a)$/.test(name)) {
    return Buffer.concat([bytes, html]);
  }
  const kopf = Buffer.alloc(8);
  kopf.writeUInt32BE(8 + html.length, 0);
  kopf.write('free', 4, 'latin1');
  return Buffer.concat([bytes, kopf, html]);
}

/// Medien-Bytes mit Skript-HTML, je Signatur, dazu die beiden Präfix-Polyglotten.
function polyglotten() {
  const ausFfmpeg = SIGNATUREN.map(([name, kodierung, erwartet]) => [
    `polyglott-${name}`, mitSkriptHtml(name, medium(werkstatt, name, kodierung)), erwartet,
  ]);
  return [
    ...ausFfmpeg,
    ['id3.html', Buffer.concat([Buffer.from([0x49, 0x44, 0x33, 3, 0, 0, 0, 0, 0, 0]), Buffer.from(HTML)]), 'medium'],
    ['oggs.html', Buffer.concat([oggOpusKopf(), Buffer.from(HTML)]), 'medium'],
  ];
}

/// Was ein Skript hinterlassen hätte — synchron, denn im sandboxed Dokument laufen keine Timer.
function spuren(page) {
  return page.evaluate(() => {
    let fremd;
    try {
      fremd = localStorage.getItem('fremd');
    } catch (fehler) {
      fremd = fehler.name;
    }
    let x = null;
    if (window.__x !== undefined) {
      x = window.__x;
    }
    return { x, titel: document.title, fremd, ursprung: self.origin };
  });
}

function mediumGebaut(page) {
  return page.evaluate(() => document.querySelector('video, audio') !== null);
}

test('Medien-Polyglotten laufen im Ursprung der Instanz ohne Skript, ein Theora-Polyglott wird geladen', async ({ browser }) => {
  test.setTimeout(120000);
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-kein-skript-'));
  instanz = await mitAllgemein();
  const ursprungDerInstanz = new URL(instanz.adresse).origin;
  const token = await beitreten(instanz.adresse, 'beta');
  const abgelegt = [];
  for (const [name, inhalt, erwartet] of polyglotten()) {
    abgelegt.push([name, await ablegen(instanz.adresse, token, name, inhalt), erwartet]);
  }
  const { page } = await kontextOeffnen(browser);
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  const speicherVorher = await page.evaluate(() => JSON.stringify({ ...localStorage }));

  for (const [name, id, erwartet] of abgelegt) {
    const quelle = `${instanz.adresse}anhaenge/${id}`;
    if (erwartet === 'laden') {
      const [download] = await Promise.all([page.waitForEvent('download'), page.goto(quelle).catch((fehler) => fehler)]);
      expect(download.suggestedFilename(), name).toBe(name);
      continue;
    }
    const navigation = await page.goto(quelle, { waitUntil: 'commit' });
    expect(navigation.headers()['content-security-policy'], name).toBe(ANHANG_CSP_MEDIUM);
    await expect.poll(() => mediumGebaut(page), { message: `${name}: Medien-Dokument`, timeout: 10000 }).toBe(true);
    expect(await spuren(page), name).toEqual({ ...STILL, ursprung: ursprungDerInstanz });
  }

  await page.goto(instanz.adresse);
  const speicherNachher = await page.evaluate(() => JSON.stringify({ ...localStorage }));
  expect(speicherNachher).toBe(speicherVorher);
  expect(JSON.parse(speicherNachher)).not.toHaveProperty('fremd');
});

test('Nicht-Medien erscheinen als Quelltext in ihrem undurchsichtigen Ursprung', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');
  const quelltexte = [
    ['x.html', Buffer.from(HTML)],
    ['bild.svg', Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg"><script>${SKRIPT}</script></svg>`)],
    ['seite.xhtml', Buffer.from(`<html xmlns="http://www.w3.org/1999/xhtml"><body><script>${SKRIPT}</script></body></html>`)],
    ['bild.png', Buffer.from(HTML)],
  ];
  const abgelegt = [];
  for (const [name, inhalt] of quelltexte) {
    abgelegt.push([name, await ablegen(instanz.adresse, token, name, inhalt)]);
  }
  const { page } = await kontextOeffnen(browser);

  for (const [name, id] of abgelegt) {
    const navigation = await page.goto(`${instanz.adresse}anhaenge/${id}`);
    expect(navigation.headers()['content-security-policy'], name).toBe(ANHANG_CSP);
    await expect(page.locator('body'), name).toContainText('<script>');
    expect(await spuren(page), name).toEqual({ ...STILL, fremd: 'SecurityError', ursprung: 'null' });
  }
});

/// Die Fehlurteile: Dokumente, die der Browser als Markup liest, mit einer CSP ausgeliefert.
function fehlurteile(polyglottBytes) {
  return [
    ...polyglottBytes.map(([name, inhalt]) => [name, inhalt, 'text/html']),
    ['x.html', Buffer.from(HTML), 'text/html'],
    ['bild.svg', Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg"><title>leer</title><script>${SKRIPT}</script></svg>`), 'image/svg+xml'],
    ['seite.xhtml', Buffer.from(`<html xmlns="http://www.w3.org/1999/xhtml"><head><title>leer</title></head><body><script>${SKRIPT}</script></body></html>`), 'application/xhtml+xml'],
  ];
}

test('Kontrollpaar: der Sentinel schlägt ohne sandbox an und bleibt unter der Medien-CSP des Programms still', async ({ browser }) => {
  test.setTimeout(120000);
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-kein-skript-'));
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');
  const bytes = polyglotten();
  const webm = bytes.find(([name]) => name === 'polyglott-film.webm');
  const id = await ablegen(instanz.adresse, token, 'film.webm', webm[1]);
  const { page } = await kontextOeffnen(browser);
  const echt = await page.goto(`${instanz.adresse}anhaenge/${id}`, { waitUntil: 'commit' });
  const medienCspDesProgramms = echt.headers()['content-security-policy'];
  expect(medienCspDesProgramms).toBe(ANHANG_CSP_MEDIUM);

  const faelle = fehlurteile(bytes);
  const gelockert = "media-src 'self'";
  testserver = await testserverStarten('127.0.0.1', (anfrage, antwort) => {
    const [, art, nummer] = anfrage.url.split('/');
    const fall = faelle[Number(nummer)];
    if (!fall) {
      antwort.writeHead(404);
      antwort.end();
      return;
    }
    const [, inhalt, typ] = fall;
    let csp = medienCspDesProgramms;
    if (art === 'gelockert') {
      csp = gelockert;
    }
    antwort.writeHead(200, { 'Content-Type': typ, 'Content-Security-Policy': csp, 'X-Content-Type-Options': 'nosniff' });
    antwort.end(inhalt);
  });
  const basis = `http://127.0.0.1:${testserver.address().port}`;

  for (const [nummer, [name]] of faelle.entries()) {
    await page.goto(`${basis}/gelockert/${nummer}`);
    const kontrolle = await spuren(page);
    expect({ x: kontrolle.x, titel: kontrolle.titel, fremd: kontrolle.fremd }, `Sentinel taugt nicht: ${name}`).toEqual(ANGESCHLAGEN);
    await page.evaluate(() => localStorage.removeItem('fremd'));

    await page.goto(`${basis}/medien-csp/${nummer}`);
    expect(await spuren(page), `${name} unter der Medien-CSP`).toEqual({ ...STILL, ursprung: basis });
  }
});
