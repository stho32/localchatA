'use strict';
// R00041 F0186 B0805 (Karte #85, Entwicklertester #68): Esc bei Fokus auf „Abbrechen“ oder „Speichern“ im Bearbeiten-Feld
// schließt das Feld wie Esc im Feld — kein POST, genau eine Ansage „Bearbeiten abgebrochen.“, Fokus auf der Nachricht; ein
// zweites Esc auf der Nachricht wirkt wie im Verlauf (zurück in die Eingabe). „Abbrechen“ per Klick, Enter und Leertaste
// führt ebenfalls auf die Nachricht. Mutante, die rot werden muss: Esc-Behandlung auf das Feld beschränkt.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { browserOeffnen, imBrowserSenden, nachrichtMitZeit, ansagenMitschreiben } = require('./r00041-hilfen');
const { feld, feldPerTaste, bearbeitungenMitschreiben } = require('./r00041-feld-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const knopfAm = (page, zeit, name) => nachrichtMitZeit(page, zeit).getByRole('button', { name, exact: true });

async function vorbereiten(browser) {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await a.page.waitForTimeout(300);
  const angesagt = await ansagenMitschreiben(a.page);
  await feldPerTaste(a.page, zeit);
  return { a, zeit, bearbeitungen, angesagt };
}

for (const name of ['Abbrechen', 'Speichern']) {
  test(`Esc auf „${name}“ (B0805): Feld zu, kein POST, „Bearbeiten abgebrochen.“ genau einmal, Fokus auf der Nachricht; zweites Esc in die Eingabe`, async ({ browser }) => {
    const { a, zeit, bearbeitungen, angesagt } = await vorbereiten(browser);
    await a.page.keyboard.type(' geändert');
    await a.page.keyboard.press('Tab');
    await expect(knopfAm(a.page, zeit, 'Abbrechen')).toBeFocused();
    if (name === 'Speichern') {
      await a.page.keyboard.press('Tab');
      await expect(knopfAm(a.page, zeit, 'Speichern')).toBeFocused();
    }

    await a.page.keyboard.press('Escape');
    await expect(feld(a.page)).toHaveCount(0);
    await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo');
    await expect.poll(angesagt).toEqual(['Bearbeiten abgebrochen.']);
    await a.page.waitForTimeout(300);
    expect(await angesagt()).toEqual(['Bearbeiten abgebrochen.']);
    expect(bearbeitungen).toEqual([]);

    await a.page.keyboard.press('Escape');
    await expect(eingabe(a.page)).toBeFocused();
    expect(a.fehler).toEqual([]);
  });
}

for (const [wie, ausloesen] of [
  ['Klick', (page, zeit) => knopfAm(page, zeit, 'Abbrechen').click()],
  ['Enter', (page) => page.keyboard.press('Enter')],
  ['Leertaste', (page) => page.keyboard.press('Space')],
]) {
  test(`„Abbrechen“ per ${wie} (B0805): Feld zu, Fokus auf der Nachricht, genau eine Ansage`, async ({ browser }) => {
    const { a, zeit, bearbeitungen, angesagt } = await vorbereiten(browser);
    await a.page.keyboard.press('Tab');
    await expect(knopfAm(a.page, zeit, 'Abbrechen')).toBeFocused();
    await ausloesen(a.page, zeit);
    await expect(feld(a.page)).toHaveCount(0);
    await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
    await expect.poll(angesagt).toEqual(['Bearbeiten abgebrochen.']);
    expect(bearbeitungen).toEqual([]);
    expect(a.fehler).toEqual([]);
  });
}
