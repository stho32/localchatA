'use strict';
// Hilfen für R00046 (Nachricht kopieren im Browser): Zwischenablage mit Wächtertext vorbelegen und über ein Prüffeld
// der Seite lesen (Strg+V geht auch ohne sicheren Kontext), Instanz im LAN, Prüfsumme der Channel-Dateien.
// Die Zwischenablage gehört dem Browser-Prozess, nicht dem Kontext (ADR-0052): wer sie benutzt, läuft seriell.
const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const { instanzStarten, channelAnlegen, lanAdresse } = require('./hilfen');
const { JETZT } = require('./senden-hilfen');

const WAECHTER = 'WAECHTER-78';

/// Instanz auf 0.0.0.0 mit `allgemein`; → { instanz, api (127.0.0.1), lan (http://<LAN-IP>:<port>/) }.
async function lanStarten() {
  const adresseDesRechners = await lanAdresse();
  const instanz = await instanzStarten({
    host: '0.0.0.0',
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein'),
  });
  const lan = instanz.adresse.replace(/\/\/[^:/]+:/, `//${adresseDesRechners}:`);
  return { instanz, api: instanz.adresse, lan };
}

/// Ein Textfeld der Seite außerhalb des Verlaufs, in dem die Zwischenablage gelesen und mit dem Wächter belegt wird.
async function pruefFeldAnlegen(page) {
  await page.evaluate(() => {
    const feld = document.createElement('textarea');
    feld.id = 'lc-pruefen';
    feld.style.cssText = 'position:fixed;left:-1000px;top:0;width:300px;height:80px';
    document.body.append(feld);
  });
}

/// Belegt die Zwischenablage mit `text` (Auswählen und Strg+C im Prüffeld); danach steht der Fokus im Prüffeld.
async function ablageBelegen(page, text = WAECHTER) {
  const feld = page.locator('#lc-pruefen');
  await feld.fill(text);
  await feld.focus();
  await page.keyboard.press('Control+A');
  await page.keyboard.press('Control+C');
}

/// Liest die Zwischenablage durch Einfügen ins Prüffeld; verändert den Fokus.
async function ablageLesen(page) {
  const feld = page.locator('#lc-pruefen');
  await feld.fill('');
  await feld.focus();
  await page.keyboard.press('Control+V');
  return feld.inputValue();
}

/// Prüfsumme über alle Dateien unter `channels/` der Instanz.
function kanalPruefsumme(ordner) {
  const hash = crypto.createHash('sha256');
  const dateien = [];
  const sammeln = (verzeichnis) => {
    for (const eintrag of fs.readdirSync(verzeichnis, { withFileTypes: true })) {
      const pfad = path.join(verzeichnis, eintrag.name);
      if (eintrag.isDirectory()) {
        sammeln(pfad);
      } else {
        dateien.push(pfad);
      }
    }
  };
  sammeln(path.join(ordner, 'channels'));
  for (const datei of dateien.sort()) {
    hash.update(datei);
    hash.update(fs.readFileSync(datei));
  }
  return hash.digest('hex');
}

/// Der Text aus QK1 (ADR-0052): vier Zeilen mit **fett**, Leerzeile, Codezaun, am Ende Zeile ohne Umbruch.
const QUELLTEXT = 'Ein **fett** und ein Wort\n\n```sh\nls -la\n```\nEnde';

module.exports = { WAECHTER, QUELLTEXT, lanStarten, pruefFeldAnlegen, ablageBelegen, ablageLesen, kanalPruefsumme };
