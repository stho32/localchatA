'use strict';
// Hilfen der Specs zu R00030 F0102 „Anhänge im Verlauf zeigen“: Testdateien aus R00030 („Testdateien entstehen im Test“),
// einmal je Worker in einem eigenen Wegwerf-Ordner erzeugt, und Anhänge, die `beta` über die API sendet (Gegeben über
// die API, R00029). Video und Ton sind 6 s lang: lang genug, dass eine Probe während der Wiedergabe handeln kann (F0165).
const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');
const { execFileSync } = require('child_process');
const { betaSendet, browserOeffnen, nachrichtMitZeit } = require('./r00041-hilfen');
const { sendenStarten, imBrowserBeitreten, anfragen } = require('./senden-hilfen');
const { expect } = require('./rahmen');

let vorrat = null;

/// Die Testdateien nach Namen: { 'foto.png': Buffer, … }.
function testdateien() {
  if (vorrat === null) {
    vorrat = dateienErzeugen();
  }
  return vorrat;
}

function dateienErzeugen() {
  const ordner = fs.mkdtempSync(path.join(os.tmpdir(), 'localchata-e2e-r00030-'));
  const ffmpeg = (name, argumente) => {
    const pfad = path.join(ordner, name);
    execFileSync('ffmpeg', ['-v', 'error', '-y', ...argumente, pfad]);
    return fs.readFileSync(pfad);
  };
  try {
    return {
      'foto.png': ffmpeg('foto.png', ['-f', 'lavfi', '-i', 'testsrc=s=1280x800', '-frames:v', '1']),
      'bericht.pdf': pdfMitBytes(2457600),
      'log.txt': Buffer.from('Zeile im Protokoll\n'.repeat(Math.ceil(40960 / 19))).subarray(0, 40960),
      'film.webm': ffmpeg('film.webm', ['-f', 'lavfi', '-i', 'testsrc=d=6:s=64x48:r=10', '-f', 'lavfi', '-i', 'sine=d=6', '-c:v', 'libvpx', '-c:a', 'libopus']),
      'ton.ogg': ffmpeg('ton.ogg', ['-f', 'lavfi', '-i', 'sine=d=6', '-c:a', 'libopus']),
      'film.ogv': ffmpeg('film.ogv', ['-f', 'lavfi', '-i', 'testsrc=d=1:s=64x48:r=10', '-c:v', 'libtheora']),
      'archiv.zip': Buffer.from(Array.from({ length: 1048576 }, (_, stelle) => (stelle * 7919 + 13) % 251)),
      'hochkant.jpg': mitOrientierung6(ffmpeg('hochkant-roh.jpg', ['-f', 'lavfi', '-i', 'testsrc=s=4000x3000', '-frames:v', '1'])),
      'foto.avif': fs.readFileSync(path.resolve(__dirname, '../../tests/gemeinsam/foto-4032x3024.avif')),
      'foto.heic': heic(),
    };
  } finally {
    fs.rmSync(ordner, { recursive: true, force: true });
  }
}

function pdfMitBytes(bytes) {
  const kopf = Buffer.from('%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\n%');
  const fuss = Buffer.from('\ntrailer << /Root 1 0 R >>\n%%EOF\n');
  return Buffer.concat([kopf, Buffer.alloc(bytes - kopf.length - fuss.length, 0x41), fuss]);
}

/// Fügt hinter SOI ein APP1 mit EXIF (`Orientation` 6, big endian) ein: das Foto ist 4000 × 3000 aufgenommen und steht
/// gedreht hochkant.
function mitOrientierung6(jpeg) {
  const tiff = Buffer.from([
    0x4d, 0x4d, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x01,
    0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x06, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
  ]);
  const nutzlast = Buffer.concat([Buffer.from('Exif\0\0', 'latin1'), tiff]);
  const laenge = Buffer.alloc(2);
  laenge.writeUInt16BE(nutzlast.length + 2);
  return Buffer.concat([jpeg.subarray(0, 2), Buffer.from([0xff, 0xe1]), laenge, nutzlast, jpeg.subarray(2)]);
}

/// ISOBMFF mit Hauptmarke `heic` (wie `tests/gemeinsam/dateien.rs` mp4): der Server erkennt `image/heic` ohne Maße.
function heic() {
  const boxe = (typ, inhalt) => {
    const laenge = Buffer.alloc(4);
    laenge.writeUInt32BE(8 + inhalt.length);
    return Buffer.concat([laenge, Buffer.from(typ, 'latin1'), inhalt]);
  };
  const ftyp = Buffer.concat([Buffer.from('heic', 'latin1'), Buffer.from([0, 0, 2, 0]), Buffer.from('mif1isom', 'latin1')]);
  return Buffer.concat([boxe('ftyp', ftyp), boxe('moov', boxe('mvhd', Buffer.alloc(100))), boxe('mdat', Buffer.from('MEDIENDATEN'))]);
}

/// Instanz mit `allgemein`; `beta` sendet je Eintrag von `nachrichten` ({ text, namen, antwortAuf? }) eine Nachricht mit diesen
/// Testdateien (oder eigenen { name, inhalt }); `vorSeite(page)` läuft vor dem Beitritt (Netz-Fakes); A (`shoff`) ist im
/// Browser beigetreten; `antwortAuf` ist die Stelle einer früheren Nachricht der Liste. → { instanz, a, beta, zeiten, ids }
/// Scheitert das Gegeben, stoppt die Instanz hier — der Test kennt sie noch nicht und könnte sie nicht stoppen.
async function anzeigeStarten(browser, nachrichten, { geraet, vorbereiten, vorSeite = async () => {} } = {}) {
  const instanz = await sendenStarten({ vorbereiten });
  try {
    return await anzeigeHerstellen(browser, instanz, nachrichten, { geraet, vorSeite });
  } catch (fehler) {
    await instanz.stoppen();
    throw fehler;
  }
}

async function anzeigeHerstellen(browser, instanz, nachrichten, { geraet, vorSeite }) {
  const beta = await betaSendet(instanz.adresse, []);
  const zeiten = [];
  const ids = [];
  for (const { text = '', namen, antwortAuf } of nachrichten) {
    const dateien = namen.map(dateiAus);
    const gesammelt = [];
    let bezug = null;
    if (antwortAuf !== undefined) {
      bezug = zeiten[antwortAuf];
    }
    zeiten.push(await nachrichtSenden(instanz.adresse, beta.token, { text, dateien, antwortAuf: bezug }, gesammelt));
    ids.push(gesammelt);
  }
  const optionen = {};
  if (geraet !== undefined) {
    optionen.geraet = geraet;
  }
  const a = await browserOeffnen(browser, optionen);
  await vorSeite(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  for (const zeit of zeiten) {
    await nachrichtMitZeit(a.page, zeit).waitFor();
  }
  return { instanz, a, beta, zeiten, ids };
}

/// Wie mitAnhaengenSenden (R00041), auch als Antwort (`antwort_auf`, R00047; `antwortAuf` null: keine); bremst die
/// Instanz (429), wartet es die genannte Zeit ab — so gehen auch 20 Nachrichten über die API. → die Zeit der Nachricht.
async function nachrichtSenden(adresse, token, { text, dateien, antwortAuf }, ids) {
  const herkunft = '127.0.0.2';
  for (const { name, inhalt } of dateien) {
    const angemeldet = await geduldigAnfragen(adresse, { methode: 'POST', pfad: 'api/anhaenge', koerper: { name, bytes: inhalt.length }, token, herkunft });
    expect(angemeldet.status, JSON.stringify(angemeldet.daten)).toBe(201);
    ids.push(angemeldet.daten.anhang.id);
    expect(await hochladen(adresse, `api/anhaenge/${angemeldet.daten.anhang.id}`, token, inhalt, herkunft), `Hochladen ${name}`).toBe(200);
  }
  const koerper = { text, anhaenge: ids };
  if (antwortAuf !== null) {
    koerper.antwort_auf = antwortAuf;
  }
  const gesendet = await geduldigAnfragen(adresse, { methode: 'POST', pfad: 'api/channels/allgemein/nachrichten', koerper, token, herkunft });
  expect(gesendet.status, JSON.stringify(gesendet.daten)).toBe(201);
  return gesendet.daten.nachricht.zeit;
}

async function geduldigAnfragen(adresse, anfrage) {
  for (let versuch = 0; versuch < 30; versuch += 1) {
    const antwort = await anfragen(adresse, anfrage);
    if (antwort.status !== 429) {
      return antwort;
    }
    await new Promise((weiter) => setTimeout(weiter, antwort.daten.wieder_in_ms + 50));
  }
  throw new Error(`immer noch gebremst: ${anfrage.pfad}`);
}

function hochladen(adresse, pfad, token, inhalt, herkunft) {
  const ziel = new URL(pfad, adresse);
  return new Promise((fertig, gescheitert) => {
    const anfrage = http.request({
      host: ziel.hostname, port: ziel.port, path: ziel.pathname, method: 'PUT', localAddress: herkunft, agent: false,
      headers: { 'Content-Type': 'application/octet-stream', 'Content-Length': inhalt.length, 'X-LocalchatA-Sitzung': token },
    }, (antwort) => {
      antwort.resume();
      antwort.on('end', () => fertig(antwort.statusCode));
    });
    anfrage.once('error', gescheitert);
    anfrage.end(inhalt);
  });
}

/// Ein Eintrag ist der Name einer Testdatei oder eine eigene Datei { name, inhalt }.
function dateiAus(eintrag) {
  if (typeof eintrag === 'string') {
    return { name: eintrag, inhalt: testdateien()[eintrag] };
  }
  return eintrag;
}

/// Die Datei-Zeile des Anhangs `name` in der Nachricht.
const dateiZeile = (knoten, name) => knoten.locator('.lc-datei').filter({ has: knoten.page().locator('.lc-datei-name', { hasText: name }) });

module.exports = { testdateien, anzeigeStarten, dateiZeile };
