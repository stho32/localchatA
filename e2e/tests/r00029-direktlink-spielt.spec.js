'use strict';
// R00029 F0100 Karte #71 (ADR-0014 Ergänzung „Karte #71 (Chrome 154)“, Probe S6/M-1, Regel 5; Kriterien „Direktlink
// spielt“, „Theora nur laden“; US-39), in Playwright-Chromium und Google Chrome stable gegen das echte Programm: ein
// Video oder Ton spielt über /anhaenge/<id> im Medien-Dokument des Browsers; ein Theora-Video wird geladen. Im
// sandboxed Medien-Dokument laufen keine Seiten-Timer und `load` kommt erst, wenn das Medium geladen ist — darum
// Navigation bis `commit`, im Dokument nur synchrone Abfragen, gewartet wird in Node.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { test, expect } = require('./rahmen');
const { beitreten, kontextOeffnen } = require('./senden-hilfen');
const { ANHANG_CSP, ANHANG_CSP_MEDIUM, mitAllgemein, ablegen, medium } = require('./r00029-direktlink-hilfen');

let instanz;
let werkstatt;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
  if (werkstatt) {
    fs.rmSync(werkstatt, { recursive: true, force: true });
    werkstatt = null;
  }
});

/// Je 20 s Testbild und Ton (oder nur Ton): bei 100 kB/s Netz größer als das, was in einer Sekunde ankommt — so muss
/// das Ende über einen eigenen Teilabruf kommen. Ab wann er zählt: nach dem Sprung; bei Ogg schon ab dem Öffnen, denn
/// der Browser liest das Ende einer Ogg-Datei für die Dauer gleich beim Öffnen (beobachtet: `bytes=589824-` vor dem
/// Sprung, danach kein Abruf mehr).
const MEDIEN = [
  ['film.webm', ['-c:v', 'libvpx', '-c:a', 'libopus'], 'ab dem Sprung'],
  ['film.mp4', ['-c:v', 'libvpx-vp9', '-deadline', 'realtime', '-cpu-used', '8', '-c:a', 'libopus'], 'ab dem Sprung'],
  ['ton.mp3', ['-b:a', '320k'], 'ab dem Sprung'],
  ['ton.ogg', ['-c:a', 'libopus', '-b:a', '256k'], 'ab dem Öffnen'],
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

  for (const [name, kodierung, teilabrufZaehlt] of MEDIEN) {
    const id = await ablegen(instanz.adresse, token, name, medium(werkstatt, name, kodierung));
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

    let teilabrufeZaehlenAb = antworten.length;
    if (teilabrufZaehlt === 'ab dem Öffnen') {
      teilabrufeZaehlenAb = 0;
    }
    await page.evaluate(() => {
      const element = document.querySelector('video, audio');
      element.currentTime = element.duration - 0.1;
    });
    await expect.poll(async () => (await zustand(page)).zuEnde, { message: `${name}: ended`, timeout: 20000 }).toBe(true);
    expect(teilabrufeAbMitte(antworten, teilabrufeZaehlenAb).length, `${name}: ${JSON.stringify(antworten)}`).toBeGreaterThan(0);
    expect((await zustand(page)).fehler, name).toBeNull();
    expect(konsole.filter((zeile) => /CORS|Content Security Policy|ERR_BLOCKED/.test(zeile)), name).toEqual([]);
    await kontext.close();
  }
});

test('ein Theora-Video wird über den Direktlink geladen, nicht abgespielt', async ({ browser }) => {
  test.setTimeout(60000);
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-direktlink-'));
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');
  const id = await ablegen(instanz.adresse, token, 'film.ogv', medium(werkstatt, 'film.ogv', ['-t', '2', '-c:v', 'libtheora', '-c:a', 'libopus']));
  const { page } = await kontextOeffnen(browser);
  const konsole = [];
  page.on('console', (meldung) => konsole.push(meldung.text()));
  const quelle = `${instanz.adresse}anhaenge/${id}`;

  const antwort = page.waitForResponse(quelle);
  const [download] = await Promise.all([page.waitForEvent('download'), page.goto(quelle).catch((fehler) => fehler)]);
  const kopf = (await antwort).headers();

  expect(download.suggestedFilename()).toBe('film.ogv');
  expect(kopf['content-type']).toBe('video/ogg');
  expect(kopf['content-disposition']).toMatch(/^attachment; filename="film\.ogv"/);
  expect(kopf['content-security-policy']).toBe(ANHANG_CSP);
  expect(page.url()).toBe('about:blank');
  expect(konsole.filter((zeile) => /media-src|Content Security Policy/.test(zeile))).toEqual([]);
});
