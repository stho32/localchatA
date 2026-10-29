'use strict';
// R00029 F0100 „Kein Skript im Ursprung“ (QS1, M1, M2) und „Skript-Gadget“ (M15), Chromium gegen das echte Programm:
// Anhänge werden über die API abgelegt und gesendet; der Browser A ist über die Oberfläche beigetreten und hält sein
// Token in localStorage. Kein Anhang führt ein Skript aus oder wendet einen Stil an — weder als eigenes Dokument
// unter /anhaenge/<id> (CSP sandbox, Text statt HTML) noch eingebunden in eine Seite der App (nosniff). Die Polyglotte
// sind ein vollständiges PNG bzw. ein WebM-Kopf mit angehängtem HTML (hinter IEND — für den Tilger Daten, kein Block).
const fs = require('fs');
const http = require('http');
const path = require('path');
const { test, expect } = require('./rahmen');
const { instanzStarten } = require('./hilfen');
const { beitreten, kontextOeffnen, imBrowserBeitreten } = require('./senden-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

function mitAllgemein() {
  return instanzStarten({ vorbereiten: (ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }) });
}

function anfrage(adresse, { methode, pfad, kopf = {}, koerper = Buffer.alloc(0) }) {
  const ziel = new URL(pfad, adresse);
  return new Promise((fertig, gescheitert) => {
    const laufend = http.request({
      host: ziel.hostname,
      port: ziel.port,
      path: ziel.pathname,
      method: methode,
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
    laufend.once('error', gescheitert);
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

const SKRIPT = "document.title = 'ausgefuehrt'; try { localStorage.setItem('fremd', '1'); } catch (e) {}";
const PNG_KOPF = Buffer.from('89504e470d0a1a0a0000000d4948445200000001000000010802000000907753de0000000c49444154789c63606060000000040001f61738550000000049454e44ae426082', 'hex');
const WEBM_KOPF = Buffer.from('1a45dfa39f4286810142f7810142f2810442f381084282847765626d42878102428581021853806701ffffffffffffff', 'hex');

test('kein Skript eines Anhangs läuft unter /anhaenge/<id>, das Token der App bleibt', async ({ browser }) => {
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');
  const html = `<html><head><title>leer</title></head><body><script>${SKRIPT}</script>Hallo</body></html>`;
  const dateien = [
    ['x.html', Buffer.from(html)],
    ['bild.svg', Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg"><script>${SKRIPT}</script></svg>`)],
    ['polyglott.png', Buffer.concat([PNG_KOPF, Buffer.from(html)])],
    ['polyglott.webm', Buffer.concat([WEBM_KOPF, Buffer.from(html)])],
  ];
  const ids = [];
  for (const [name, inhalt] of dateien) {
    ids.push(await ablegen(instanz.adresse, token, name, inhalt));
  }
  const { page } = await kontextOeffnen(browser);
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  const speicherVorher = await page.evaluate(() => JSON.stringify({ ...localStorage }));

  for (const id of ids) {
    await page.goto(`${instanz.adresse}anhaenge/${id}`);
    await page.waitForTimeout(200);
    expect(await page.title(), id).not.toBe('ausgefuehrt');
  }
  await page.goto(`${instanz.adresse}anhaenge/${ids[0]}`);
  await expect(page.locator('body')).toContainText('<script>');
  await page.goto(`${instanz.adresse}anhaenge/${ids[1]}`);
  await expect(page.locator('body')).toContainText('<svg');

  await page.goto(instanz.adresse);
  expect(await page.evaluate(() => JSON.stringify({ ...localStorage }))).toBe(speicherVorher);
});

test('ein Text-Anhang wird in der App nie als Skript, Modul, Worker oder Stil wirksam', async ({ browser }) => {
  instanz = await mitAllgemein();
  const token = await beitreten(instanz.adresse, 'beta');
  const js = await ablegen(instanz.adresse, token, 'gadget.js', Buffer.from(
    "self.__gadget = (self.__gadget || 0) + 1; if (typeof postMessage === 'function' && typeof window === 'undefined') { postMessage('lief'); }",
  ));
  const css = await ablegen(instanz.adresse, token, 'stil.css', Buffer.from('body { background-color: rgb(1, 2, 3) !important; }'));
  const { page } = await kontextOeffnen(browser);
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');

  const ergebnis = await page.evaluate(async ({ jsUrl, cssUrl }) => {
    const gemeldet = [];
    const warten = (ms) => new Promise((fertig) => setTimeout(fertig, ms));
    const skript = document.createElement('script');
    skript.src = jsUrl;
    document.head.append(skript);
    const modul = document.createElement('script');
    modul.type = 'module';
    modul.src = `${jsUrl}?modul`;
    document.head.append(modul);
    try {
      await import(`${jsUrl}?import`);
      gemeldet.push('import');
    } catch (e) { /* abgewiesen */ }
    try {
      const arbeiter = new Worker(jsUrl);
      arbeiter.onmessage = () => gemeldet.push('worker');
    } catch (e) { /* abgewiesen */ }
    try {
      const geteilt = new SharedWorker(`${jsUrl}?geteilt`);
      geteilt.port.onmessage = () => gemeldet.push('sharedworker');
      geteilt.port.start();
    } catch (e) { /* abgewiesen */ }
    try {
      await navigator.serviceWorker.register(jsUrl, { scope: '/anhaenge/' });
      gemeldet.push('serviceworker');
    } catch (e) { /* abgewiesen */ }
    const stil = document.createElement('link');
    stil.rel = 'stylesheet';
    stil.href = cssUrl;
    document.head.append(stil);
    await warten(1000);
    return {
      gadget: window.__gadget ?? 0,
      gemeldet,
      hintergrund: getComputedStyle(document.body).backgroundColor,
    };
  }, { jsUrl: `/anhaenge/${js}`, cssUrl: `/anhaenge/${css}` });

  expect(ergebnis.gadget).toBe(0);
  expect(ergebnis.gemeldet).toEqual([]);
  expect(ergebnis.hintergrund).not.toBe('rgb(1, 2, 3)');
});
