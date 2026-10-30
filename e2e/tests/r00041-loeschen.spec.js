'use strict';
// R00041 F0139 Eigene Nachricht im Browser löschen: Aktion und Rückfrage-Dialog (B0623, UX M5, PO 126), Löschen über
// die API (B0624), Ausnahme zu R00030 „Kein Lösch-Knopf“.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { betaSendet, browserOeffnen, imBrowserSenden, nachrichtMitZeit } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const LOESCHEN = '/api/channels/allgemein/nachrichten/loeschen';
const dialog = (page) => page.getByRole('dialog');
const imDialog = (page, name) => dialog(page).getByRole('button', { name, exact: true });

function loeschungenMitschreiben(page) {
  const liste = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && new URL(anfrage.url()).pathname === LOESCHEN) {
      liste.push(anfrage.postData());
    }
  });
  return liste;
}

test('Rückfrage am Desktop (B0623): Hover → „Löschen“ → modaler Dialog „Diese Nachricht löschen?“ mit „Abbrechen“ · „Löschen“; „Abbrechen“ schließt ohne POST', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const loeschungen = loeschungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).hover();
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Löschen', exact: true }).click();

  await expect(dialog(a.page)).toBeVisible();
  expect(await a.page.evaluate(() => document.querySelector('dialog[open]').matches(':modal'))).toBe(true);
  await expect(dialog(a.page)).toHaveAccessibleName('Diese Nachricht löschen?');
  await expect(dialog(a.page)).toContainText('Das lässt sich nicht rückgängig machen.');
  await expect(dialog(a.page)).toHaveAccessibleDescription(/Das lässt sich nicht rückgängig machen\./);
  await expect(dialog(a.page)).toContainText(/\d\d:\d\d: Hallo/);
  await expect(dialog(a.page).getByRole('button')).toHaveText(['Abbrechen', 'Löschen']);
  await imDialog(a.page, 'Abbrechen').click();
  await expect(dialog(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
  expect(loeschungen).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Tastatur (B0623, UX M5): Entf öffnet, Fokus auf „Abbrechen“ und bleibt im Dialog; Enter sofort und Esc schließen ohne POST, Fokus zurück; Strg Entf und Entf an fremden öffnen nichts', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const loeschungen = loeschungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).focus();

  await a.page.keyboard.press('Delete');
  await expect(imDialog(a.page, 'Abbrechen')).toBeFocused();
  for (const taste of ['Tab', 'Tab', 'Tab', 'Shift+Tab', 'Shift+Tab', 'Shift+Tab']) {
    await a.page.keyboard.press(taste);
    expect(await a.page.evaluate(() => document.querySelector('dialog[open]').contains(document.activeElement))).toBe(true);
  }
  await imDialog(a.page, 'Abbrechen').focus();
  await a.page.keyboard.press('Enter');
  await expect(dialog(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();

  await a.page.keyboard.press('Delete');
  await expect(dialog(a.page)).toBeVisible();
  await a.page.keyboard.press('Escape');
  await expect(dialog(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();

  await a.page.keyboard.press('Control+Delete');
  await a.page.waitForTimeout(200);
  await expect(dialog(a.page)).toHaveCount(0);
  const beta = await betaSendet(instanz.adresse, ['Fremd']);
  await nachrichtMitZeit(a.page, beta.zeiten[0]).focus();
  await a.page.keyboard.press('Delete');
  await a.page.waitForTimeout(200);
  await expect(dialog(a.page)).toHaveCount(0);
  expect(loeschungen).toEqual([]);
  expect(a.fehler).toEqual([]);
});
