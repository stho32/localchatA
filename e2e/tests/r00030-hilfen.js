'use strict';
// Hilfen für R00030 F0101 (Anhänge im Browser hochladen): der Schalter „Anhang“ und die Liste der gewählten Dateien in
// der Eingabe, „Wählen“ wie in den Akzeptanzkriterien (Schalter auslösen, `filechooser` abwarten, `setFiles`) und die
// Testdateien, die im Test entstehen.
const crypto = require('crypto');
const http = require('http');
const zlib = require('zlib');
const { instanzStarten, channelAnlegen } = require('./hilfen');
const { JETZT } = require('./senden-hilfen');

/// Instanz mit `allgemein` und `technik`, Quoten wie R00029 (Platz für Anhänge 10.485.760 Bytes).
function anhangStarten({ argumente = ['--plattenquote', '10MiB', '--anhangquote', '10MiB'] } = {}) {
  return instanzStarten({
    argumente,
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      channelAnlegen(ordner, 'allgemein');
      channelAnlegen(ordner, 'technik');
    },
  });
}

/// Eine Anfrage je Verbindung (`agent: false`) von `herkunft`; → { status, daten, kopf }.
function perApi(adresse, { methode, pfad, token, json, roh, herkunft = '127.0.0.2' }) {
  const ziel = new URL(pfad, adresse);
  const kopf = {};
  let koerper = Buffer.alloc(0);
  if (token) {
    kopf['X-LocalchatA-Sitzung'] = token;
  }
  if (json !== undefined) {
    kopf['Content-Type'] = 'application/json';
    koerper = Buffer.from(JSON.stringify(json));
  }
  if (roh !== undefined) {
    kopf['Content-Type'] = 'application/octet-stream';
    koerper = roh;
  }
  kopf['Content-Length'] = koerper.length;
  return new Promise((fertig, gescheitert) => {
    const anfrage = http.request({
      host: ziel.hostname, port: ziel.port, path: ziel.pathname, method: methode, agent: false, localAddress: herkunft, headers: kopf,
    }, (antwort) => {
      const teile = [];
      antwort.on('data', (teil) => teile.push(teil));
      antwort.on('end', () => {
        const text = Buffer.concat(teile).toString('utf8');
        fertig({ status: antwort.statusCode, daten: text.startsWith('{') ? JSON.parse(text) : text, kopf: antwort.headers });
      });
    });
    anfrage.once('error', (fehler) => gescheitert(new Error(`${methode} ${pfad}: ${fehler.message}`)));
    anfrage.end(koerper);
  });
}

/// `beta` (von 127.0.0.2) tritt bei; → Token.
async function betaBeitreten(adresse) {
  const antwort = await perApi(adresse, { methode: 'POST', pfad: 'api/beitreten', json: { name: 'beta' } });
  if (antwort.status !== 201) {
    throw new Error(`Beitritt beta: ${antwort.status}`);
  }
  return antwort.daten.token;
}

/// Meldet einen Anhang an; → ID (wirft bei einer anderen Antwort als 201).
async function perApiAnmelden(adresse, token, name, bytes) {
  const antwort = await perApi(adresse, { methode: 'POST', pfad: 'api/anhaenge', token, json: { name, bytes } });
  if (antwort.status !== 201) {
    throw new Error(`Anmelden ${name}: ${antwort.status} ${JSON.stringify(antwort.daten)}`);
  }
  return antwort.daten.anhang.id;
}

/// Meldet je Größe einen Anhang an, lädt Zufallsbytes hoch und sendet alle in einer Nachricht nach `allgemein`.
async function perApiAblegen(adresse, token, groessen) {
  const ids = [];
  for (const [stelle, bytes] of groessen.entries()) {
    const id = await perApiAnmelden(adresse, token, `teil-${stelle + 1}.bin`, bytes);
    const hochgeladen = await perApi(adresse, { methode: 'PUT', pfad: `api/anhaenge/${id}`, token, roh: crypto.randomBytes(bytes) });
    if (hochgeladen.status !== 200) {
      throw new Error(`Hochladen: ${hochgeladen.status} ${JSON.stringify(hochgeladen.daten)}`);
    }
    ids.push(id);
  }
  const gesendet = await perApi(adresse, { methode: 'POST', pfad: 'api/channels/allgemein/nachrichten', token, json: { text: '', anhaenge: ids } });
  if (gesendet.status !== 201) {
    throw new Error(`Senden: ${gesendet.status} ${JSON.stringify(gesendet.daten)}`);
  }
  return ids;
}

/// Schreibt die Anfragen der Seite an Anhänge und Nachrichten mit: [{ methode, pfad, koerper }].
function anfragenMitschreiben(page) {
  const anfragen = [];
  page.on('request', (anfrage) => {
    const pfad = new URL(anfrage.url()).pathname;
    const zaehlt = pfad.startsWith('/api/anhaenge') || (pfad.endsWith('/nachrichten') && anfrage.method() === 'POST');
    if (zaehlt) {
      let koerper = null;
      if (anfrage.method() === 'POST') {
        koerper = anfrage.postDataJSON();
      }
      anfragen.push({ methode: anfrage.method(), pfad, koerper });
    }
  });
  return anfragen;
}

const schalter = (page) => page.locator('.lc-comp').getByRole('button', { name: 'Anhang', exact: true });
const dateiliste = (page) => page.getByRole('list', { name: 'Angehängte Dateien' });
const dateizeilen = (page) => dateiliste(page).getByRole('listitem');

/// Löst den Schalter aus (`geste`: 'klick' | 'tippen') und wählt `dateien` ({ name, mimeType, buffer }).
async function waehlen(page, dateien, { geste = 'klick' } = {}) {
  const auswahl = page.waitForEvent('filechooser');
  if (geste === 'tippen') {
    await schalter(page).tap();
  } else {
    await schalter(page).click();
  }
  const chooser = await auswahl;
  await chooser.setFiles(dateien);
}

/// Ein PNG 1280 × 800 (schwarz), aufgefüllt hinter IEND auf genau `bytes` (Standard 307.200 = „300 KiB“).
function fotoPng(name = 'foto.png', bytes = 307200) {
  const BREITE = 1280;
  const HOEHE = 800;
  const kopf = Buffer.alloc(13);
  kopf.writeUInt32BE(BREITE, 0);
  kopf.writeUInt32BE(HOEHE, 4);
  kopf.set([8, 2, 0, 0, 0], 8);
  const zeilen = Buffer.alloc(HOEHE * (1 + BREITE * 3));
  const png = Buffer.concat([
    Buffer.from('89504e470d0a1a0a', 'hex'),
    pngBlock('IHDR', kopf),
    pngBlock('IDAT', zlib.deflateSync(zeilen)),
    pngBlock('IEND', Buffer.alloc(0)),
  ]);
  return { name, mimeType: 'image/png', buffer: Buffer.concat([png, Buffer.alloc(bytes - png.length)]) };
}

function pngBlock(art, daten) {
  const laenge = Buffer.alloc(4);
  laenge.writeUInt32BE(daten.length);
  const typ = Buffer.from(art, 'ascii');
  const pruefsumme = Buffer.alloc(4);
  pruefsumme.writeUInt32BE(zlib.crc32(Buffer.concat([typ, daten])));
  return Buffer.concat([laenge, typ, daten, pruefsumme]);
}

/// `bericht.pdf`: 2.457.600 Bytes, beginnt mit `%PDF-1.7`.
function berichtPdf(name = 'bericht.pdf') {
  const inhalt = Buffer.alloc(2457600, 0x20);
  inhalt.write('%PDF-1.7\n');
  return { name, mimeType: 'application/pdf', buffer: inhalt };
}

/// Text aus `bytes` Zeichen (Standard 40 KiB).
function textDatei(name = 'log.txt', bytes = 40 * 1024) {
  return { name, mimeType: 'text/plain', buffer: Buffer.alloc(bytes, 'a') };
}

/// Zufallsbytes (Standard 1 MiB), Name `archiv.zip`.
function archivZip(name = 'archiv.zip', bytes = 1024 * 1024) {
  return { name, mimeType: 'application/zip', buffer: crypto.randomBytes(bytes) };
}

/// Ring des Elements und sein Kontrast gegen kd-surface (Fläche der Eingabe .lc-comp).
function ringGegenFlaeche(element) {
  function leuchtdichte(farbe) {
    const [r, g, b] = farbe.match(/[\d.]+/g).slice(0, 3).map(Number).map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }
  const stil = getComputedStyle(element);
  const ring = leuchtdichte(stil.outlineColor);
  const flaeche = leuchtdichte(getComputedStyle(element.closest('.lc-comp')).backgroundColor);
  return {
    sichtbar: element.matches(':focus-visible'),
    stil: stil.outlineStyle,
    breite: parseFloat(stil.outlineWidth),
    versatz: parseFloat(stil.outlineOffset),
    kontrast: (Math.max(ring, flaeche) + 0.05) / (Math.min(ring, flaeche) + 0.05),
  };
}

/// Schreibt jede Ansage der einen Ansage-Region `.lc-ansage` mit; liefert eine Abfrage der Liste.
async function ansagenMitschreiben(page) {
  await page.evaluate(() => {
    window.ansagenR00030 = [];
    const region = document.querySelector('.lc-ansage');
    new MutationObserver(() => {
      if (region.textContent !== '') {
        window.ansagenR00030.push(region.textContent);
      }
    }).observe(region, { childList: true, characterData: true, subtree: true });
  });
  return () => page.evaluate(() => window.ansagenR00030);
}

/// Die Live-Regionen der Seite (aria-live polite oder assertive, role status oder alert).
const liveRegionen = (page) => page.evaluate(() => document.querySelectorAll(
  '[aria-live="polite"], [aria-live="assertive"], [role="status"], [role="alert"]',
).length);

module.exports = {
  anhangStarten, perApi, betaBeitreten, perApiAnmelden, perApiAblegen, anfragenMitschreiben, ansagenMitschreiben, liveRegionen, ringGegenFlaeche, schalter, dateiliste, dateizeilen, waehlen, fotoPng, berichtPdf, textDatei, archivZip };
