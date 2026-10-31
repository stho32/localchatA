'use strict';
// Hilfen für die Specs zum Bearbeiten-Feld aus Karte #85 (F0186): das Feld, Öffnen per Tastatur und Menü, POSTs zählen.
const { expect } = require('@playwright/test');
const { nachrichtMitZeit } = require('./r00041-hilfen');

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

module.exports = { feld, feldPerTaste, feldPerMenue, bearbeitungenMitschreiben };
