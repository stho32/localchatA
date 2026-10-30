'use strict';
// R00029 F0115 „Zeitfelder“ (ADR-0047 Ergänzung „Karte #9 Security-Review B2“ Regel 5g, Probe P10, B0647), Chromium
// gegen das echte Programm: Filme mit `creation_time` werden hochgeladen und gesendet; der Tilger nullt Erstell- und
// Änderungszeit in `mvhd`, `tkhd`, `mdhd` — der Player des Browsers spielt die unter /anhaenge/<id> ausgelieferten
// Bytes trotzdem: `canplay`, `currentTime` rückt vor, kein `error`. Gespielt wird aus einer lokalen Seite ohne CSP:
// das Medien-Dokument unter /anhaenge/<id> lädt wegen `sandbox; default-src 'none'` nichts, und der Player im
// Verlauf (App-CSP mit `media-src`) ist noch nicht gebaut — beides unabhängig vom Tilger.
const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');
const { execFileSync } = require('child_process');
const { test, expect } = require('./rahmen');
const { instanzStarten } = require('./hilfen');
const { beitreten } = require('./senden-hilfen');

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

/// `GET /anhaenge/<id>` roh; liefert die Bytes.
function ausgeliefert(adresse, id) {
  return new Promise((fertig, gescheitert) => {
    http.get(new URL(`anhaenge/${id}`, adresse), { localAddress: '127.0.0.1' }, (antwort) => {
      const teile = [];
      antwort.on('data', (teil) => teile.push(teil));
      antwort.on('end', () => fertig({ status: antwort.statusCode, bytes: Buffer.concat(teile) }));
    }).once('error', gescheitert);
  });
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

/// Ein Film aus ffmpeg: Testbild und Ton, 2 s, `creation_time` 2026-10-29T12:34:56Z.
function film(name, optionen = []) {
  const pfad = path.join(werkstatt, name);
  execFileSync('ffmpeg', [
    '-v', 'error', '-y', '-f', 'lavfi', '-i', 'testsrc=d=2:s=64x48:r=10', '-f', 'lavfi', '-i', 'sine=d=2', '-shortest',
    '-metadata', 'creation_time=2026-10-29T12:34:56Z', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-c:a', 'aac',
    ...optionen, pfad,
  ]);
  return fs.readFileSync(pfad);
}

/// Die Boxen in [anfang, ende): Typ, Anfang des Inhalts, Ende.
function boxen(bytes, anfang, ende) {
  const liste = [];
  for (let pos = anfang; pos + 8 <= ende;) {
    const groesse = bytes.readUInt32BE(pos);
    const bis = groesse === 0 ? ende : pos + groesse;
    liste.push({ typ: bytes.toString('latin1', pos + 4, pos + 8), inhalt: pos + 8, bis });
    pos = bis;
  }
  return liste;
}

/// `mvhd`, `tkhd`, `mdhd` in Version 1 (64 Bit); `moov` steht hinter `mdat`, die Versätze in `stco` bleiben gültig.
function version1(bytes) {
  const umbauen = (anfang, ende) => Buffer.concat(boxen(bytes, anfang, ende).map(({ typ, inhalt, bis }) => {
    let teil = bytes.subarray(inhalt, bis);
    if (['moov', 'trak', 'mdia'].includes(typ)) {
      teil = umbauen(inhalt, bis);
    } else if (['mvhd', 'tkhd', 'mdhd'].includes(typ) && teil[0] === 0) {
      const dauerAb = typ === 'tkhd' ? 20 : 16;
      const lang = (stelle) => { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(teil.readUInt32BE(stelle))); return b; };
      teil = Buffer.concat([
        Buffer.from([1, teil[1], teil[2], teil[3]]), lang(4), lang(8), teil.subarray(12, dauerAb), lang(dauerAb),
        teil.subarray(dauerAb + 4),
      ]);
    }
    const kopf = Buffer.alloc(8);
    kopf.writeUInt32BE(teil.length + 8);
    kopf.write(typ, 4, 'latin1');
    return Buffer.concat([kopf, teil]);
  }));
  return umbauen(0, bytes.length);
}

test('ein Film mit genullten Zeitfeldern spielt im Browser', async ({ page }) => {
  werkstatt = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-film-'));
  instanz = await instanzStarten({ vorbereiten: (ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }) });
  const token = await beitreten(instanz.adresse, 'beta');
  const android = film('android.mp4');
  const filme = [
    ['android.mp4', android],
    ['fragmentiert.mp4', film('fragmentiert.mp4', ['-movflags', 'frag_keyframe+empty_moov+default_base_moof'])],
    ['version1.mp4', version1(android)],
  ];
  for (const [name, inhalt] of filme) {
    const id = await ablegen(instanz.adresse, token, name, inhalt);
    const geholt = await ausgeliefert(instanz.adresse, id);
    expect(geholt.status, name).toBe(200);
    expect(geholt.bytes.length, name).toBe(inhalt.length);
    expect(geholt.bytes.equals(inhalt), `${name}: getilgt`).toBe(false);
    fs.writeFileSync(path.join(werkstatt, `geliefert-${name}`), geholt.bytes);
    fs.writeFileSync(path.join(werkstatt, 'spielen.html'), `<video src="geliefert-${name}"></video>`);

    await page.goto(`file://${path.join(werkstatt, 'spielen.html')}`);
    const gespielt = await page.evaluate(() => new Promise((fertig) => {
      const video = document.querySelector('video');
      if (!video) {
        fertig({ fehler: 'kein Video' });
        return;
      }
      video.muted = true;
      const pruefen = () => fertig({ bereit: video.readyState, zeit: video.currentTime, fehler: video.error && video.error.code });
      video.addEventListener('error', pruefen);
      const spielen = () => {
        video.play().catch(() => {});
        setTimeout(pruefen, 1000);
      };
      if (video.readyState >= 3) {
        spielen();
      } else {
        video.addEventListener('canplay', spielen, { once: true });
      }
    }));

    expect(gespielt, name).toEqual(expect.objectContaining({ fehler: null }));
    expect(gespielt.bereit, name).toBeGreaterThanOrEqual(3);
    expect(gespielt.zeit, name).toBeGreaterThan(0);
  }
});
