'use strict';
// Gemeinsames der R00050-Oberflächen-Specs (Karte #13, F0178): Instanz mit `allgemein` und `technik`, `anna` im
// Browser, weitere Teilnehmer per API von eigenen Herkünften (Linux: ganz 127.0.0.0/8 ist lokal), Senden und Lesen im
// Direktgespräch per API, die Adresse `#/d/<kennung>` und die Beschriftungen des Privat-Modus.
const fs = require('fs');
const path = require('path');
const { expect } = require('@playwright/test');
const { sendenStarten, beitreten, anfragen, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');

const ANNA = 'anna@127.0.0.1';
const BEN = 'ben@127.0.0.3';
const CORA = 'cora@127.0.0.9';
const HINWEIS = 'Direktnachricht — nur ihr beide im Chat; die Dateien liegen im Ordner der Instanz';
const KEIN_GESPRAECH = 'Diese Adresse führt zu keinem Gespräch.';

/// Vorgeschriebene Antwort (ADR-0055 Regel 3, Prüfreihenfolge): Tippen in einem Direktgespräch, das noch keine Nachricht
/// hat, beantwortet die Instanz mit 404 „wie nie da“ — Chromium schreibt dazu „Failed to load resource“ in die Konsole.
function tippenOhneGespraech(...kennungen) {
  return Object.fromEntries(kennungen.map((kennung) => [`/api/direkt/${encodeURIComponent(kennung)}/aktivitaet`, [404]]));
}

/// Startet eine Instanz und öffnet `anna` im Browser in `# allgemein`; `ben` und `cora` treten als Menschen per API
/// bei. → { instanz, anna: { kontext, page, fehler }, token: { ben, cora } }
async function direktStarten(browser, { sammeln = { zusaetzlich: tippenOhneGespraech(BEN, CORA) }, geraet } = {}) {
  const instanz = await sendenStarten();
  const anna = await browserOeffnen(browser, { geraet, ...sammeln });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  const token = {
    ben: await beitreten(instanz.adresse, 'ben', { art: 'mensch', herkunft: '127.0.0.3' }),
    cora: await beitreten(instanz.adresse, 'cora', { art: 'mensch', herkunft: '127.0.0.9' }),
  };
  return { instanz, anna, token };
}

/// POST /api/direkt/<gegenüber>/nachrichten als Inhaber von `token` von `herkunft`.
function direktSenden(adresse, token, gegenueber, text, herkunft) {
  const pfad = `api/direkt/${encodeURIComponent(gegenueber)}/nachrichten`;
  return anfragen(adresse, { methode: 'POST', pfad, koerper: { text }, token, herkunft });
}

async function direktLesen(adresse, token, gegenueber, herkunft) {
  const pfad = `api/direkt/${encodeURIComponent(gegenueber)}/nachrichten?n=200`;
  const antwort = await anfragen(adresse, { methode: 'GET', pfad, token, herkunft });
  return antwort.daten.nachrichten.map((nachricht) => nachricht.text);
}

/// Die Paarordner unter `direkt/` (leer, solange keiner existiert).
function paarordner(ordner) {
  const wurzel = path.join(ordner, 'direkt');
  if (!fs.existsSync(wurzel)) {
    return [];
  }
  return fs.readdirSync(wurzel).sort();
}

/// Öffnet die Adresse des Gesprächs mit `kennung` (prozentkodiert) im Tab.
async function direktOeffnen(page, kennung) {
  await page.evaluate((hash) => { window.location.hash = hash; }, `#/d/${encodeURIComponent(kennung)}`);
}

const verlaufMit = (page, kennung) => page.getByRole('log', { name: `Verlauf Direktnachricht mit ${kennung}` });
const eingabeAn = (page, kennung) => page.getByRole('textbox', { name: `Nachricht an ${kennung}`, exact: true });

/// Wartet, bis das Gespräch mit `kennung` offen ist: Verlauf und Eingabe tragen seinen Namen.
async function gespraechOffen(page, kennung) {
  await expect(verlaufMit(page, kennung)).toBeVisible();
  await expect(eingabeAn(page, kennung)).toBeVisible();
}

/// Texte in `wurzelSelektor`, deren Kontrast gegen die Fläche darunter unter 4,5:1 liegt (WCAG 2.x), mit eingerechneter
/// Deckkraft aller Vorfahren; im Browser gerechnet → [{ text, wert }].
function kontrastUnterschritten(wurzelSelektor) {
  function rgb(farbe) {
    return farbe.match(/[\d.]+/g).slice(0, 4).map(Number);
  }
  function leuchtdichte([r, g, b]) {
    const [lr, lg, lb] = [r, g, b].map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
  }
  function flaecheUnter(element) {
    for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
      const farbe = rgb(getComputedStyle(knoten).backgroundColor);
      if (farbe.length < 4 || farbe[3] > 0) {
        return farbe.slice(0, 3);
      }
    }
    return [255, 255, 255];
  }
  function deckkraft(element) {
    let produkt = 1;
    for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
      produkt *= Number(getComputedStyle(knoten).opacity);
    }
    return produkt;
  }
  function kontrast(vorn, hinten, alpha) {
    const gemischt = vorn.map((kanal, stelle) => kanal * alpha + hinten[stelle] * (1 - alpha));
    const [hell, dunkel] = [leuchtdichte(gemischt), leuchtdichte(hinten)].sort((a, b) => b - a);
    return (hell + 0.05) / (dunkel + 0.05);
  }
  const sichtbar = (element) => element.getClientRects().length > 0 && getComputedStyle(element).visibility !== 'hidden';
  return [...document.querySelector(wurzelSelektor).querySelectorAll('*')]
    .filter((element) => sichtbar(element)
      && [...element.childNodes].some((kind) => kind.nodeType === 3 && kind.textContent.trim() !== ''))
    .map((element) => ({
      text: element.textContent.trim().slice(0, 40),
      wert: kontrast(rgb(getComputedStyle(element).color).slice(0, 3), flaecheUnter(element), deckkraft(element)),
    }))
    .filter((eintrag) => eintrag.wert < 4.5);
}

module.exports = {
  ANNA, BEN, CORA, HINWEIS, KEIN_GESPRAECH, tippenOhneGespraech, direktStarten, direktSenden, direktLesen, paarordner, direktOeffnen, verlaufMit, eingabeAn,
  gespraechOffen, kontrastUnterschritten,
};
