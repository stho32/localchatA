'use strict';
// R00041 F0135 Eigene Nachricht im Browser bearbeiten: Aktionen am Desktop und per Taste (B0609), Feld an Ort und
// Stelle (B0610), Speichern (B0611), Ansagen (B0612), Tasten im Feld (B0627), Menü „⋯“ auf Touch (B0628), Feld am
// Telefon (B0629).
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe, nachrichten } = require('./senden-hilfen');
const { betaSendet, browserOeffnen, imBrowserSenden, nachrichtMitZeit } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const knopf = (page, zeit, name) => nachrichtMitZeit(page, zeit).getByRole('button', { name, exact: true });

/// Drückt Tab, bis die Eingabe den Fokus hat, und liefert die zugänglichen Namen der Knöpfe unterwegs.
async function tabBisEingabe(page, grenze = 5) {
  const besucht = [];
  for (let schritt = 0; schritt < grenze; schritt += 1) {
    await page.keyboard.press('Tab');
    const stelle = await page.evaluate(() => {
      const aktiv = document.activeElement;
      return { eingabe: aktiv.matches('.lc-comp-feld'), knopf: aktiv.matches('button') ? aktiv.textContent : null };
    });
    if (stelle.eingabe) {
      return besucht;
    }
    besucht.push(stelle.knopf);
  }
  throw new Error(`Eingabe nach ${grenze} Tab nicht erreicht: ${JSON.stringify(besucht)}`);
}

test('Desktop (B0609): an der eigenen Nachricht erscheinen bei Hover „Bearbeiten“ und „Löschen“; kein „⋯“', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await eingabe(a.page).blur();
  await a.page.mouse.move(0, 0);
  await expect(knopf(a.page, zeit, 'Bearbeiten')).toBeHidden();

  await nachrichtMitZeit(a.page, zeit).hover();
  await expect(knopf(a.page, zeit, 'Bearbeiten')).toBeVisible();
  await expect(knopf(a.page, zeit, 'Löschen')).toBeVisible();
  await expect(a.page.getByRole('log').getByRole('button', { name: 'Aktionen' })).toHaveCount(0);
  for (const name of ['Bearbeiten', 'Löschen']) {
    const box = await knopf(a.page, zeit, name).boundingBox();
    expect(box.width).toBeGreaterThanOrEqual(24);
    expect(box.height).toBeGreaterThanOrEqual(24);
  }
  expect(a.fehler).toEqual([]);
});

test('Tab-Stopps (B0609): von der fokussierten eigenen Nachricht erreicht Tab „Bearbeiten“, „Löschen“, dann die Eingabe; von einer fremden direkt die Eingabe', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserSenden(a.page, 'Hallo');
  const beta = await betaSendet(instanz.adresse, ['Fremd']);
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0])).toHaveCount(1);

  await eingabe(a.page).focus();
  await a.page.keyboard.press('ArrowUp');
  expect(await tabBisEingabe(a.page)).toEqual(['Bearbeiten', 'Löschen']);
  await nachrichtMitZeit(a.page, beta.zeiten[0]).focus();
  expect(await tabBisEingabe(a.page)).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Nur eigene (B0609): an Nachrichten von anna und beta weder Aktionen noch Leiste; an „wird gesendet …“ und fehlgeschlagenen auch nicht', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [500] } });
  const b = await browserOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const vonAnna = await imBrowserSenden(b.page, 'Von Anna');
  const beta = await betaSendet(instanz.adresse, ['Von Beta']);
  await expect(nachrichten(a.page)).toHaveCount(2);
  for (const zeit of [vonAnna, beta.zeiten[0]]) {
    await nachrichtMitZeit(a.page, zeit).hover();
    await nachrichtMitZeit(a.page, zeit).focus();
    for (const name of ['Bearbeiten', 'Löschen', 'Aktionen']) {
      await expect(nachrichtMitZeit(a.page, zeit).getByRole('button', { name })).toHaveCount(0);
    }
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-aktionen, .lc-aktionen-schalter')).toHaveCount(0);
  }

  let freigeben;
  const tor = new Promise((offen) => { freigeben = offen; });
  await a.page.route('**/api/channels/allgemein/nachrichten', async (route) => {
    if (route.request().method() !== 'POST') {
      await route.continue();
      return;
    }
    await tor;
    await route.fulfill({ status: 500, contentType: 'application/json', body: '{"fehler":"Nicht gespeichert."}' });
  });
  await eingabe(a.page).fill('Unterwegs');
  await eingabe(a.page).press('Enter');
  const vorlaeufig = a.page.getByRole('log').locator('.lc-msg.lc-vorlaeufig');
  await expect(vorlaeufig).toHaveCount(1);
  await vorlaeufig.hover();
  await expect(vorlaeufig.getByRole('button', { name: /^(Bearbeiten|Löschen|Aktionen)$/ })).toHaveCount(0);
  freigeben();
  await expect(a.page.getByRole('log').locator('.lc-msg.lc-fehlgeschlagen')).toHaveCount(1);
  await expect(a.page.getByRole('log').locator('.lc-msg.lc-fehlgeschlagen').getByRole('button', { name: /^(Bearbeiten|Löschen|Aktionen)$/ })).toHaveCount(0);
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});
