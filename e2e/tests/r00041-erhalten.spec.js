'use strict';
// R00041 F0142 Angefangenes Bearbeiten bleibt erhalten (UX M3, M4, PO 124): Feld je Channel und im Fenster (B0635) und
// Live-Änderung während des Bearbeitens (B0636). A ist `shoff`; gelöscht und bearbeitet wird „mit dem Sitzungs-Token von
// A“ über die API.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, nachrichten } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const {
  browserOeffnen, imBrowserSenden, nachrichtMitZeit, loeschenPerApi, ansagenMitschreiben,
} = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const feld = (page) => page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' });
const statuszeile = (page) => page.locator('.lc-statuszeile');
const tokenVon = (page) => page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
const auswahl = (page) => feld(page).evaluate((textfeld) => [textfeld.selectionStart, textfeld.selectionEnd]);

async function channelWaehlen(page, name) {
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', `Verlauf # ${name}`);
}

async function feldOeffnen(page, zeit) {
  await nachrichtMitZeit(page, zeit).hover();
  await nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).click();
  await expect(feld(page)).toBeFocused();
}

function bearbeitungenZaehlen(page) {
  const liste = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten/bearbeiten')) {
      liste.push(anfrage.postDataJSON());
    }
  });
  return liste;
}

test('Channelwechsel (B0635, PO 124): das Feld bleibt mit Text und Schreibmarke, bei der Rückkehr offen mit Fokus; kein POST; Neuladen verwirft es', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenZaehlen(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.type(' zusammen');
  await feld(a.page).evaluate((textfeld) => textfeld.setSelectionRange(3, 3));

  await channelWaehlen(a.page, 'technik');
  await channelWaehlen(a.page, 'allgemein');
  await expect(feld(a.page)).toHaveValue('Hallo zusammen');
  await expect(feld(a.page)).toBeFocused();
  expect(await auswahl(a.page)).toEqual([3, 3]);
  expect(bearbeitungen).toEqual([]);

  await a.page.reload();
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo');
  await expect(feld(a.page)).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

test('Nachricht weg bei der Rückkehr (B0635): gelöscht, während A in # technik ist → Feld zu, „Nachricht gelöscht“, Hinweiszeile', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.type(' zusammen');
  await channelWaehlen(a.page, 'technik');
  expect((await loeschenPerApi(instanz.adresse, await tokenVon(a.page), zeit, '127.0.0.1')).status).toBe(200);

  await channelWaehlen(a.page, 'allgemein');
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await expect(feld(a.page)).toHaveCount(0);
  await expect(statuszeile(a.page)).toHaveText('Die Nachricht wurde inzwischen gelöscht.');
  expect(a.fehler).toEqual([]);
});

test('Blockabwurf (B0635): 600 Nachrichten, Feld an der eigenen offen; an den Anfang und zurück → das Feld steht mit dem getippten Text', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await sendenStarten({ anzahl: 600 });
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Meine');
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.type(' getippt');
  await a.page.getByRole('log').evaluate((log) => { window.verlaufKnoten = log; });
  await feld(a.page).evaluate((textfeld) => { window.feldKnoten = textfeld; });

  for (let schritt = 0; schritt < 60; schritt += 1) {
    await a.page.evaluate(() => { window.verlaufKnoten.scrollTop = 0; });
    await a.page.waitForTimeout(100);
  }
  await expect(a.page.getByRole('log').locator('.lc-msg').first()).toHaveAttribute('data-zeit', /T00:/);
  expect(await a.page.evaluate(() => window.feldKnoten.isConnected)).toBe(true);
  for (let schritt = 0; schritt < 60 && await feld(a.page).count() === 1 && !(await feld(a.page).isVisible()); schritt += 1) {
    await a.page.evaluate(() => { window.verlaufKnoten.scrollTop = window.verlaufKnoten.scrollHeight; });
    await a.page.waitForTimeout(100);
  }
  await expect(feld(a.page)).toHaveValue('Meine getippt');
  expect(a.fehler).toEqual([]);
});

test('Zweites Bearbeiten (B0635): kein zweites Feld, Fokus ins offene, genau einmal „Erst speichern oder abbrechen.“; aus einem anderen Channel bleibt A, wo es ist', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const eins = await imBrowserSenden(a.page, 'Eins');
  const zwei = await imBrowserSenden(a.page, 'Zwei');
  await a.page.waitForTimeout(300);
  const angesagt = await ansagenMitschreiben(a.page);
  await feldOeffnen(a.page, eins);

  await nachrichtMitZeit(a.page, zwei).focus();
  await a.page.keyboard.press('e');
  await expect(feld(a.page)).toHaveCount(1);
  await expect(nachrichtMitZeit(a.page, eins).getByRole('textbox')).toBeFocused();
  await expect.poll(angesagt).toEqual(['Erst speichern oder abbrechen.']);

  await nachrichtMitZeit(a.page, zwei).hover();
  await nachrichtMitZeit(a.page, zwei).getByRole('button', { name: 'Bearbeiten', exact: true }).click();
  await expect(feld(a.page)).toHaveCount(1);
  await expect(nachrichtMitZeit(a.page, eins).getByRole('textbox')).toBeFocused();
  await expect.poll(angesagt).toEqual(['Erst speichern oder abbrechen.', 'Erst speichern oder abbrechen.']);

  await nachrichtMitZeit(a.page, eins).getByRole('textbox').press('Escape');
  await channelWaehlen(a.page, 'technik');
  const dort = await imBrowserSenden(a.page, 'Dort');
  await feldOeffnen(a.page, dort);
  await channelWaehlen(a.page, 'allgemein');
  await nachrichtMitZeit(a.page, zwei).hover();
  await nachrichtMitZeit(a.page, zwei).getByRole('button', { name: 'Bearbeiten', exact: true }).click();
  await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await expect(feld(a.page)).toHaveCount(0);
  const zweite = async () => (await angesagt()).filter((text) => text === 'Erst speichern oder abbrechen.').length;
  await expect.poll(zweite).toBe(3);
  expect(a.fehler).toEqual([]);
});

for (const geraet of ['desktop', 'telefon']) {
  test(`Tippen außerhalb (B0635, ${geraet}): ein Klick bzw. Tippen in den Verlauf außerhalb des Felds lässt es offen`, async ({ browser }) => {
    instanz = await sendenStarten({ anzahl: 5 });
    const a = await geraetOeffnen(browser, geraet, { streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const zeit = await imBrowserSenden(a.page, 'Hallo');
    if (geraet === 'desktop') {
      await feldOeffnen(a.page, zeit);
    } else {
      await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Aktionen' }).tap();
      await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).tap();
    }
    await feld(a.page).fill('Hallo getippt');
    const andere = nachrichten(a.page).first().locator('.lc-body');
    if (geraet === 'desktop') {
      await andere.click();
    } else {
      await andere.tap();
    }
    await expect(feld(a.page)).toHaveValue('Hallo getippt');
    expect(a.fehler).toEqual([]);
  });
}
