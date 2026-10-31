'use strict';
// Hilfen für die Specs zum Bearbeiten-Feld aus Karte #85 (F0186): das Feld, Öffnen per Tastatur und Menü, POSTs zählen.
const { expect } = require('@playwright/test');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen, imBrowserSenden, nachrichtMitZeit } = require('./r00041-hilfen');

const feld = (page) => page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' });

/// Öffnet das Feld an der eigenen Nachricht `zeit` mit Fokus auf der Nachricht und der Taste E.
async function feldPerTaste(page, zeit) {
  await nachrichtMitZeit(page, zeit).focus();
  await page.keyboard.press('e');
  await expect(feld(page)).toBeFocused();
}

/// Öffnet das Feld über „Aktionen“ → „Bearbeiten“ (Touch).
async function feldPerMenue(page, zeit) {
  await nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Aktionen', exact: true }).tap();
  await nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).tap();
  await expect(feld(page)).toBeFocused();
}

/// Zählt die Anfragen `POST …/nachrichten/bearbeiten`.
function bearbeitungenMitschreiben(page) {
  const liste = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten/bearbeiten')) {
      liste.push(anfrage.postDataJSON());
    }
  });
  return liste;
}

const touch = (breite, hoehe) => ({ viewport: { width: breite, height: hoehe }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 });

/// Misst im Browser: Feld, Knöpfe, Verlauf, Eingabe, Querscrollen, Treffer in der Mitte der Knöpfe.
function vermessen() {
  const log = document.querySelector('[role="log"]').getBoundingClientRect();
  const textfeld = document.querySelector('textarea[aria-label="Nachricht bearbeiten"]');
  const feldRect = textfeld.getBoundingClientRect();
  const eingabe = document.querySelector('.lc-comp').getBoundingClientRect();
  const knoepfe = ['Abbrechen', 'Speichern'].map((name) => {
    const knopf = [...document.querySelectorAll('.lc-bearbeiten-knoepfe button')].find((einzeln) => einzeln.textContent === name);
    const rand = knopf.getBoundingClientRect();
    const ziel = document.elementFromPoint(rand.left + rand.width / 2, rand.top + rand.height / 2);
    return { name, unten: rand.bottom, getroffen: ziel !== null && knopf.contains(ziel) };
  });
  return {
    logOben: log.top, logUnten: log.bottom, logBreite: log.width,
    feldOben: feldRect.top, feldUnten: feldRect.bottom, feldBreite: feldRect.width, feldHoehe: feldRect.height,
    feldScrollt: textfeld.scrollHeight > textfeld.clientHeight,
    eingabeOben: eingabe.top, knoepfe,
    quer: document.documentElement.scrollWidth > window.innerWidth,
  };
}

/// Alles im Verlauf: Feld unten nicht über den Rand, Knöpfe nicht darunter und selbst getroffen, kein Querscrollen.
function imVerlauf(m) {
  expect(m.feldOben, 'Oberkante Feld ≥ Oberkante Verlauf').toBeGreaterThanOrEqual(m.logOben - 0.5);
  for (const knopf of m.knoepfe) {
    expect(knopf.unten, `Unterkante „${knopf.name}“ ≤ Unterkante Verlauf`).toBeLessThanOrEqual(m.logUnten + 0.5);
    expect(knopf.getroffen, `Mitte von „${knopf.name}“ trifft den Knopf`).toBe(true);
  }
  expect(m.quer, 'kein Querscrollen').toBe(false);
}

/// Startet eine Instanz mit 30 Nachrichten, öffnet ein Touch-Fenster, sendet `text` als letzte eigene Nachricht und öffnet das
/// Feld über „Aktionen“ → „Bearbeiten“; → { instanz, a }.
async function eigeneNachrichtBearbeiten(browser, breite, hoehe, text) {
  const instanz = await sendenStarten({ anzahl: 30 });
  const a = await browserOeffnen(browser, { geraet: touch(breite, hoehe), streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, text);
  await feldPerMenue(a.page, zeit);
  await a.page.waitForTimeout(200);
  return { instanz, a };
}


module.exports = {
  feld, feldPerTaste, feldPerMenue, bearbeitungenMitschreiben, touch, vermessen, imVerlauf, eigeneNachrichtBearbeiten,
};
