'use strict';
// R00041 F0186 B0801 (Karte #85, ux-reviewer-Vorprüfung): ein zweites „Bearbeiten“ (Taste E oder Knopf) bei offenem Feld öffnet
// kein zweites Feld, nennt aber sichtbar den Ort des offenen Felds in der Hinweiszeile der Seite (#lc-comp-hinweis):
// „Erst speichern oder abbrechen: offenes Feld bei der Nachricht von 14:07.“ bzw. „… offenes Feld in # technik, Nachricht von
// 14:07.“ — innerhalb 1 s da, mindestens 5 s sichtbar, nicht abgeschnitten, von der Zeile selbst getroffen; die Ansage „Erst
// speichern oder abbrechen.“ bleibt genau einmal, die Hinweiszeile hat weder aria-live noch role; Channelnamen über 16 Zeichen
// stehen mit „…“. Desktop, Touch 375 × 667 und 320 × 568. Mutanten, die rot werden müssen: Hinweis ohne Ort; Hinweis über die
// feste Statuszeile; Ansage doppelt.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen, nachrichtMitZeit, ansagenMitschreiben } = require('./r00041-hilfen');
const { channelAnlegen } = require('./hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const { feld, feldPerTaste, feldPerMenue } = require('./r00041-feld-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const EINS = '2026-10-28T14:07:00.000+01:00';
const ZWEI = '2026-10-28T14:09:00.000+01:00';
const LANG = 'sehr-langer-channel-name';
const ICH = 'shoff@127.0.0.1';

function vorbereiten(ordner) {
  channelAnlegen(ordner, 'allgemein', { '2026-10-28': [[EINS, ICH, 'Eins'], [ZWEI, ICH, 'Zwei']] });
  channelAnlegen(ordner, 'technik', { '2026-10-28': [[EINS, ICH, 'Technik eins']] });
  channelAnlegen(ordner, LANG, { '2026-10-28': [[EINS, ICH, 'Lang eins']] });
}

const GERAETE = [
  ['Desktop', async (browser) => browserOeffnen(browser, { streng: true }), false],
  ['Touch 375 × 667', async (browser) => geraetOeffnen(browser, 'telefon', { streng: true, fenster: { width: 375, height: 667 } }), true],
  ['Touch 320 × 568', async (browser) => geraetOeffnen(browser, 'telefonSchmal', { streng: true }), true],
];

const hinweisZeile = (page) => page.locator('#lc-comp-hinweis');

/// Öffnet das Feld an `zeit`: am Desktop mit der Taste E, auf Touch über „Aktionen“ → „Bearbeiten“.
async function feldOeffnen(page, zeit, mitMenue) {
  if (mitMenue) {
    await feldPerMenue(page, zeit);
    return;
  }
  await feldPerTaste(page, zeit);
}

/// Das zweite „Bearbeiten“: am Desktop die Taste E auf der Nachricht, auf Touch Tippen auf „Bearbeiten“ im Menü.
async function zweitesBearbeiten(page, zeit, mitMenue) {
  if (mitMenue) {
    await nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Aktionen', exact: true }).tap();
    await nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).tap();
    return;
  }
  await nachrichtMitZeit(page, zeit).focus();
  await page.keyboard.press('e');
}

function zeileVermessen() {
  const zeile = document.querySelector('#lc-comp-hinweis');
  const rand = zeile.getBoundingClientRect();
  const ziel = document.elementFromPoint(rand.left + rand.width / 2, rand.top + rand.height / 2);
  return {
    abgeschnitten: zeile.scrollHeight > zeile.clientHeight,
    getroffen: ziel !== null && zeile.contains(ziel),
    live: zeile.getAttribute('aria-live'),
    rolle: zeile.getAttribute('role'),
    statuszeileSichtbar: document.querySelector('.lc-statuszeile') !== null && !document.querySelector('.lc-statuszeile').hidden,
  };
}

for (const [name, oeffnen, mitMenue] of GERAETE) {
  test(`${name}, selber Channel (B0801): E an „Zwei“ bei offenem Feld an „Eins“ → Hinweiszeile nennt 14:07, 5 s sichtbar, ein Feld, eine Ansage`, async ({ browser }) => {
    instanz = await sendenStarten({ vorbereiten });
    const a = await oeffnen(browser);
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await expect(nachrichtMitZeit(a.page, ZWEI)).toHaveCount(1);
    await feldOeffnen(a.page, EINS, mitMenue);
    const angesagt = await ansagenMitschreiben(a.page);

    await zweitesBearbeiten(a.page, ZWEI, mitMenue);
    await expect(hinweisZeile(a.page)).toContainText('Erst speichern oder abbrechen', { timeout: 1000 });
    await expect(hinweisZeile(a.page)).toHaveText('Erst speichern oder abbrechen: offenes Feld bei der Nachricht von 14:07.');
    expect(await a.page.evaluate(zeileVermessen)).toEqual({
      abgeschnitten: false, getroffen: true, live: null, rolle: null, statuszeileSichtbar: false,
    });
    await expect(feld(a.page)).toHaveCount(1);
    await expect(feld(a.page)).toBeFocused();
    await a.page.waitForTimeout(5000);
    await expect(hinweisZeile(a.page)).toContainText('14:07');
    const ansagen = await angesagt();
    expect(ansagen.filter((text) => text.includes('Erst speichern'))).toHaveLength(1);
    expect(a.fehler).toEqual([]);
  });

  test(`${name}, anderer Channel (B0801): Feld in # technik offen, E in # allgemein → Hinweis nennt „# technik“, der Channel bleibt`, async ({ browser }) => {
    instanz = await sendenStarten({ vorbereiten });
    const a = await oeffnen(browser);
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff', 'technik');
    const technikZeit = EINS;
    await feldOeffnen(a.page, technikZeit, mitMenue);
    await a.page.evaluate(() => { window.location.hash = '#/c/allgemein'; });
    await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
    await expect(nachrichtMitZeit(a.page, ZWEI)).toHaveCount(1);
    const angesagt = await ansagenMitschreiben(a.page);

    await zweitesBearbeiten(a.page, ZWEI, mitMenue);
    await expect(hinweisZeile(a.page)).toHaveText('Erst speichern oder abbrechen: offenes Feld in # technik, Nachricht von 14:07.', { timeout: 1000 });
    expect(await a.page.evaluate(zeileVermessen)).toEqual({
      abgeschnitten: false, getroffen: true, live: null, rolle: null, statuszeileSichtbar: false,
    });
    await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
    await expect(feld(a.page)).toHaveCount(0);
    expect((await angesagt()).filter((text) => text.includes('Erst speichern'))).toHaveLength(1);
    expect(a.fehler).toEqual([]);
  });

  test(`${name}, langer Channelname (B0801): über 16 Zeichen steht er mit „…“, die Zeile bleibt unabgeschnitten`, async ({ browser }) => {
    instanz = await sendenStarten({ vorbereiten });
    const a = await oeffnen(browser);
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff', LANG);
    await feldOeffnen(a.page, EINS, mitMenue);
    await a.page.evaluate(() => { window.location.hash = '#/c/allgemein'; });
    await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
    await expect(nachrichtMitZeit(a.page, ZWEI)).toHaveCount(1);

    await zweitesBearbeiten(a.page, ZWEI, mitMenue);
    await expect(hinweisZeile(a.page)).toHaveText('Erst speichern oder abbrechen: offenes Feld in # sehr-langer-cha…, Nachricht von 14:07.', { timeout: 1000 });
    const gemessen = await a.page.evaluate(zeileVermessen);
    expect(gemessen.abgeschnitten).toBe(false);
    expect(gemessen.getroffen).toBe(true);
  });
}
