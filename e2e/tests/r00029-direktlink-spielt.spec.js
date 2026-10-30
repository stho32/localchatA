'use strict';
// R00029 F0100 Karte #71 (ADR-0014 Ergänzung „Karte #71“, Proben M-1, M-2, M-3; Kriterien „Direktlink spielt“, „CORP
// beim Direktlink“, „Kein Skript beim Direktlink“; US-39), Chromium gegen das echte Programm: ein Video oder Ton spielt
// über /anhaenge/<id> im Medien-Dokument des Browsers, kein Skript eines Anhangs läuft, und eine fremde Seite liest die
// Bytes eines Mediums nicht. Im sandboxed Medien-Dokument laufen keine Seiten-Timer und `load` kommt erst, wenn das
// Medium geladen ist — darum Navigation bis `commit`, im Dokument nur synchrone Abfragen, gewartet wird in Node.
const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');
const { execFileSync } = require('child_process');
const { test, expect } = require('./rahmen');
const { instanzStarten } = require('./hilfen');
const { beitreten, kontextOeffnen, imBrowserBeitreten } = require('./senden-hilfen');

const ANHANG_CSP = "sandbox; default-src 'none'";
const ANHANG_CSP_MEDIUM = "sandbox; default-src 'none'; media-src 'self'";

let instanz;
let werkstatt;
let fremdserver;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
  if (werkstatt) {
    fs.rmSync(werkstatt, { recursive: true, force: true });
    werkstatt = null;
  }
  if (fremdserver) {
    await new Promise((fertig) => fremdserver.close(fertig));
    fremdserver = null;
  }
});

function mitAllgemein() {
  return instanzStarten({ vorbereiten: (ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }) });
}

/// Je Anfrage eine eigene Verbindung (`agent: false`): ffmpeg läuft synchron zwischen den Anfragen, und ein Socket aus
/// dem Keep-Alive-Vorrat, den die Instanz nach 10 s Leerlauf geschlossen hat, endete sonst in `socket hang up`.
function anfrage(adresse, { methode, pfad, kopf = {}, koerper = Buffer.alloc(0) }) {
  const ziel = new URL(pfad, adresse);
  return new Promise((fertig, gescheitert) => {
    const laufend = http.request({
      host: ziel.hostname,
      port: ziel.port,
      path: ziel.pathname,
      method: methode,
      agent: false,
      localAddress: '127.0.0.1',
      headers: { ...kopf, 'Content-Length': koerper.length },
    }, (antwort) => {
      const teile = [];
      antwort.on('data', (teil) => teile.push(teil));
      antwort.on('end', () => {
        const text = Buffer.concat(teile).toString('utf8');
        fertig({ status: antwort.statusCode, daten: text.startsWith('{') ? JSON.parse(text) : text });
      });
    });
    laufend.once('error', (fehler) => gescheitert(new Error(`${methode} ${pfad}: ${fehler.message}`)));
    laufend.end(koerper);
  });
}

/// Anmelden, hochladen und als Nachricht ohne Text senden; liefert die ID.
async function ablegen(adresse, token, name, inhalt) {
  const json = { 'Content-Type': 'application/json', 'X-LocalchatA-Sitzung': token };
  const angemeldet = await anfrage(adresse, {
    methode: 'POST', pfad: 'api/anhaenge', kopf: json, koerper: Buffer.from(JSON.stringify({ name, bytes: inhalt.length })),
  });
  expect(angemeldet.status, JSON.stringify(angemeldet.daten)).toBe(201);
  const { id } = angemeldet.daten.anhang;
  const hochgeladen = await anfrage(adresse, {
    methode: 'PUT',
    pfad: `api/anhaenge/${id}`,
    kopf: { 'Content-Type': 'application/octet-stream', 'X-LocalchatA-Sitzung': token },
    koerper: inhalt,
  });
  expect(hochgeladen.status, JSON.stringify(hochgeladen.daten)).toBe(200);
  const gesendet = await anfrage(adresse, {
    methode: 'POST',
    pfad: 'api/channels/allgemein/nachrichten',
    kopf: json,
    koerper: Buffer.from(JSON.stringify({ text: '', anhaenge: [id] })),
  });
  expect(gesendet.status, JSON.stringify(gesendet.daten)).toBe(201);
  return id;
}

/// Eine Datei aus ffmpeg, 20 s Testbild und Ton (oder nur Ton): bei 100 kB/s Netz größer als das, was in einer
/// Sekunde ankommt — so muss der Sprung ans Ende einen eigenen Teilabruf stellen.
function medium(name, kodierung) {
  const pfad = path.join(werkstatt, name);
  const bild = name.endsWith('.mp3') ? [] : ['-f', 'lavfi', '-i', 'testsrc=d=20:s=320x240:r=25'];
  execFileSync('ffmpeg', ['-v', 'error', '-y', ...bild, '-f', 'lavfi', '-i', 'sine=d=20', '-shortest', ...kodierung, pfad]);
  return fs.readFileSync(pfad);
}

const MEDIEN = [
  ['film.webm', ['-c:v', 'libvpx', '-c:a', 'libopus']],
  ['film.mp4', ['-c:v', 'libvpx-vp9', '-deadline', 'realtime', '-cpu-used', '8', '-c:a', 'libopus']],
  ['ton.mp3', ['-b:a', '320k']],
];

/// Was das Medien-Dokument gerade zeigt — synchron, denn im sandboxed Dokument laufen keine Timer.
function zustand(page) {
  return page.evaluate(() => {
    const element = document.querySelector('video, audio');
    if (!element) {
      return { element: false };
    }
    return {
      element: true,
      bereit: element.readyState,
      zeit: element.currentTime,
      dauer: element.duration,
      fehler: element.error && element.error.code,
      zuEnde: element.ended,
    };
  });
}

/// Die Teilabrufe (206) nach einem Zeitpunkt, deren Bereich nicht am Anfang der Datei beginnt.
function teilabrufeAbMitte(antworten, ab) {
  return antworten.slice(ab).filter(({ status, bereich }) => status === 206 && !/^bytes=0-/.test(bereich));
}

test('Video und Ton spielen über den Direktlink, ein Sprung ans Ende lädt nur den Rest', async ({ browser }) => {
  test.setTimeout(120000);
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-direktlink-'));
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');

  for (const [name, kodierung] of MEDIEN) {
    const id = await ablegen(instanz.adresse, token, name, medium(name, kodierung));
    const { kontext, page } = await kontextOeffnen(browser);
    const konsole = [];
    page.on('console', (meldung) => konsole.push(meldung.text()));
    const antworten = [];
    page.on('response', (antwort) => antworten.push({ status: antwort.status(), bereich: antwort.request().headers().range || '' }));
    const netz = await kontext.newCDPSession(page);
    await netz.send('Network.enable');
    await netz.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: 100000, uploadThroughput: -1 });

    const navigation = await page.goto(`${instanz.adresse}anhaenge/${id}`, { waitUntil: 'commit' });
    expect(navigation.headers()['content-security-policy'], name).toBe(ANHANG_CSP_MEDIUM);
    await expect.poll(async () => (await zustand(page)).bereit, { message: `${name}: canplay`, timeout: 15000 }).toBeGreaterThanOrEqual(3);
    await page.evaluate(() => {
      const element = document.querySelector('video, audio');
      element.muted = true;
      element.play();
    });
    await expect.poll(async () => (await zustand(page)).zeit, { message: `${name}: Zeit rückt vor`, timeout: 15000 }).toBeGreaterThan(0.5);
    expect((await zustand(page)).fehler, name).toBeNull();

    const vorDemSprung = antworten.length;
    await page.evaluate(() => {
      const element = document.querySelector('video, audio');
      element.currentTime = element.duration - 0.1;
    });
    await expect.poll(async () => (await zustand(page)).zuEnde, { message: `${name}: ended`, timeout: 20000 }).toBe(true);
    expect(teilabrufeAbMitte(antworten, vorDemSprung).length, `${name}: ${JSON.stringify(antworten)}`).toBeGreaterThan(0);
    expect((await zustand(page)).fehler, name).toBeNull();
    expect(konsole.filter((zeile) => /Content Security Policy|ERR_BLOCKED/.test(zeile)), name).toEqual([]);
    await kontext.close();
  }
});

const SKRIPT = "window.__x = 1; document.title = 'ausgefuehrt'; try { localStorage.setItem('fremd', '1'); } catch (e) {}";
const HTML = `<html><head><title>leer</title></head><body><script>${SKRIPT}</script>Hallo</body></html>`;

/// Ogg-Seite mit `OpusHead`: die Instanz erkennt daran `audio/ogg`.
function oggOpusKopf() {
  const paket = Buffer.concat([Buffer.from('OpusHead'), Buffer.from([1, 2, 0x38, 1, 0x80, 0xbb, 0, 0, 0, 0, 0])]);
  return Buffer.concat([Buffer.from('OggS'), Buffer.from([0, 2]), Buffer.alloc(20), Buffer.from([1, paket.length]), paket]);
}

test('kein Skript eines Anhangs läuft beim Direktlink, auch nicht in Medien-Polyglotten', async ({ browser }) => {
  test.setTimeout(90000);
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-direktlink-'));
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');
  const quelltexte = [
    ['x.html', Buffer.from(HTML)],
    ['bild.svg', Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg"><script>${SKRIPT}</script></svg>`)],
    ['seite.xhtml', Buffer.from(`<html xmlns="http://www.w3.org/1999/xhtml"><body><script>${SKRIPT}</script></body></html>`)],
    ['bild.png', Buffer.from(HTML)],
  ];
  const medienPolyglotte = [
    ['polyglott.webm', Buffer.concat([medium('klein.webm', ['-t', '1', '-c:v', 'libvpx', '-c:a', 'libopus']), Buffer.from(HTML)])],
    ['polyglott.mp3', Buffer.concat([medium('klein.mp3', ['-t', '1']), Buffer.from(HTML)])],
    ['id3.html', Buffer.concat([Buffer.from([0x49, 0x44, 0x33, 3, 0, 0, 0, 0, 0, 0]), Buffer.from(HTML)])],
    ['oggs.html', Buffer.concat([oggOpusKopf(), Buffer.from(HTML)])],
  ];
  const quelltextIds = [];
  for (const [name, inhalt] of quelltexte) {
    quelltextIds.push([name, await ablegen(instanz.adresse, token, name, inhalt)]);
  }
  const medienIds = [];
  for (const [name, inhalt] of medienPolyglotte) {
    medienIds.push([name, await ablegen(instanz.adresse, token, name, inhalt)]);
  }
  const { page } = await kontextOeffnen(browser);
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  const speicherVorher = await page.evaluate(() => JSON.stringify({ ...localStorage }));

  for (const [name, id] of [...quelltextIds, ...medienIds]) {
    const navigation = await page.goto(`${instanz.adresse}anhaenge/${id}`, { waitUntil: 'commit' });
    await page.waitForTimeout(300);
    const geprueft = await page.evaluate(() => {
      let speicher;
      try {
        speicher = `erreichbar ${localStorage.length}`;
      } catch (fehler) {
        speicher = fehler.name;
      }
      return { sentinel: window.__x === undefined ? 'ungesetzt' : 'gesetzt', titel: document.title, speicher };
    });
    expect(geprueft, name).toEqual({ sentinel: 'ungesetzt', titel: expect.not.stringMatching(/ausgefuehrt/), speicher: 'SecurityError' });
    if (quelltextIds.some(([quelltext]) => quelltext === name)) {
      expect(navigation.headers()['content-security-policy'], name).toBe(ANHANG_CSP);
      await expect(page.locator('body'), name).toContainText('<script>');
    } else {
      expect(navigation.headers()['content-security-policy'], `${name} wird Medienart`).toBe(ANHANG_CSP_MEDIUM);
    }
  }

  await page.goto(instanz.adresse);
  expect(await page.evaluate(() => JSON.stringify({ ...localStorage }))).toBe(speicherVorher);
});

/// Eine fremde Seite auf `localhost` (anderer Ursprung und andere Site als `127.0.0.1`), die das Medium einbettet.
function fremdeSeiteStarten(quelle) {
  const seite = `<html><body><video src="${quelle}" muted></video></body></html>`;
  return new Promise((fertig) => {
    const server = http.createServer((_anfrage, antwort) => {
      antwort.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      antwort.end(seite);
    });
    server.listen(0, 'localhost', () => fertig(server));
  });
}

test('eine fremde Seite liest die Bytes eines Mediums nicht', async ({ browser }) => {
  test.setTimeout(60000);
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-direktlink-'));
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');
  const id = await ablegen(instanz.adresse, token, 'film.webm', medium('film.webm', ['-t', '2', '-c:v', 'libvpx', '-c:a', 'libopus']));
  const quelle = `${instanz.adresse}anhaenge/${id}`;
  fremdserver = await fremdeSeiteStarten(quelle);
  const { page } = await kontextOeffnen(browser);

  await page.goto(`http://localhost:${fremdserver.address().port}/`);
  const gelesen = await page.evaluate(async (ziel) => {
    const ergebnis = {};
    try {
      const antwort = await fetch(ziel, { mode: 'cors' });
      ergebnis.cors = `gelesen ${(await antwort.arrayBuffer()).byteLength}`;
    } catch (fehler) {
      ergebnis.cors = fehler.name;
    }
    try {
      const undurchsichtig = await fetch(ziel, { mode: 'no-cors' });
      ergebnis.noCors = { typ: undurchsichtig.type, status: undurchsichtig.status, bytes: (await undurchsichtig.arrayBuffer()).byteLength };
    } catch (fehler) {
      ergebnis.noCors = fehler.name;
    }
    return ergebnis;
  }, quelle);

  expect(gelesen).toEqual({ cors: 'TypeError', noCors: { typ: 'opaque', status: 0, bytes: 0 } });
});
