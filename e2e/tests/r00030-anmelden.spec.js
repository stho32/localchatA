'use strict';
// R00030 F0101 B0450 — Anmelden beim Senden: Absenden mit Dateien legt am Ende des eigenen Verlaufs den Eintrag
// „wird hochgeladen …“ an, Feld und Liste werden leer, jede Datei wird angemeldet (`POST /api/anhaenge`). Eine Abweisung
// (507 `voll`, 422 `zu_gross`) steht an der Datei mit ⚠ und dem Text der Instanz, wird einmal angesagt, und nichts lädt;
// 507 `hochladungen` heißt warten und nach `Retry-After` selbst neu anmelden. Quoten wie R00029 (10 MiB).
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const {
  anhangStarten, perApi, betaBeitreten, perApiAnmelden, perApiAblegen, anfragenMitschreiben, ansagenMitschreiben,
  dateizeilen, waehlen, fotoPng, textDatei, archivZip,
} = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ERLAUBT = { '/api/anhaenge': [422, 507] };
const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const dateiImEintrag = (page, name) => eintrag(page).locator('.lc-hochladung-datei').filter({ hasText: name });
const VOLL = 'Nicht hochgeladen: der Platz für Anhänge ist voll — frei sind 0,2 MiB, die Datei braucht 0,3 MiB. Platz entsteht, '
  + 'wenn Anhänge gelöscht werden oder ältere Tage entfallen; Text kann weiter gesendet werden.';

async function seiteAuf(browser, geraet = 'desktop') {
  const seite = await geraetOeffnen(browser, geraet, { zusaetzlich: ERLAUBT });
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  return seite;
}

/// Hält jeden `PUT` an, bis der Test endet — der Eintrag bleibt „wird hochgeladen …“.
async function hochladenAnhalten(page) {
  await page.route('**/api/anhaenge/*', (route) => {
    if (route.request().method() !== 'PUT') {
      return route.continue();
    }
    return undefined;
  });
}

test('Absenden mit Dateien: Eintrag „wird hochgeladen …“ am Ende des eigenen Verlaufs, Feld und Liste leer, je Datei eine Anmeldung', async ({ browser }) => {
  instanz = await anhangStarten();
  const { page, fehler } = await seiteAuf(browser);
  const anfragen = anfragenMitschreiben(page);
  await hochladenAnhalten(page);

  await waehlen(page, [textDatei(), archivZip()]);
  await eingabe(page).fill('zwei Dateien');
  await eingabe(page).press('Enter');

  await expect(eintrag(page)).toHaveCount(1);
  await expect(eintrag(page)).toContainText('wird hochgeladen …');
  await expect(eintrag(page).locator('.lc-body')).toHaveText('zwei Dateien');
  await expect(eintrag(page).locator('.lc-hochladung-datei')).toHaveCount(2);
  await expect(dateiImEintrag(page, 'log.txt')).toContainText('40 KiB');
  await expect(dateiImEintrag(page, 'archiv.zip')).toContainText('1,0 MiB');
  await expect(eingabe(page)).toHaveValue('');
  await expect(dateizeilen(page)).toHaveCount(0);
  await expect(eingabe(page)).toBeFocused();
  const letzte = await page.getByRole('log').evaluate((log) => [...log.querySelectorAll('.lc-msg')].at(-1).classList.contains('lc-hochladung'));
  expect(letzte).toBe(true);
  await expect.poll(() => anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge').map((anfrage) => anfrage.koerper))
    .toEqual([{ name: 'log.txt', bytes: 40960 }, { name: 'archiv.zip', bytes: 1048576 }]);
  expect(anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten'))).toEqual([]);
  expect(fehler).toEqual([]);
});

test('Kein Platz (Betreiber F1′): ⚠ mit dem Text der Instanz und „Entfernen“, kein PUT, kein Senden, einmal angesagt; Entfernen ohne Text → Eintrag weg', async ({ browser }) => {
  instanz = await anhangStarten();
  const beta = await betaBeitreten(instanz.adresse);
  await perApiAblegen(instanz.adresse, beta, [...Array(9).fill(1048576), 843776]);
  const { page, fehler } = await seiteAuf(browser);
  const anfragen = anfragenMitschreiben(page);
  const angesagt = await ansagenMitschreiben(page);

  await waehlen(page, [fotoPng()]);
  await knopf(page).click();

  const datei = dateiImEintrag(page, 'foto.png');
  await expect(datei.locator('.lc-hochladung-grund')).toHaveText(`⚠ ${VOLL}`);
  await expect(datei.getByRole('button', { name: 'foto.png entfernen', exact: true })).toBeVisible();
  await expect.poll(angesagt).toContain(`Nicht hochgeladen: foto.png — ${VOLL.replace('Nicht hochgeladen: ', '')}`);
  expect((await angesagt()).filter((text) => text.includes('Nicht hochgeladen'))).toHaveLength(1);
  expect(anfragen.filter((anfrage) => anfrage.methode === 'PUT' || anfrage.pfad.endsWith('/nachrichten'))).toEqual([]);

  await datei.getByRole('button', { name: 'foto.png entfernen', exact: true }).click();
  await expect(eintrag(page)).toHaveCount(0);
  await expect(eingabe(page)).toBeFocused();
  expect(anfragen.filter((anfrage) => anfrage.methode === 'PUT' || anfrage.pfad.endsWith('/nachrichten'))).toEqual([]);
  expect(fehler).toEqual([]);
});

test('Kein Platz mit Text: „Entfernen“ an der abgewiesenen Datei sendet den Text `trotzdem` als Nachricht', async ({ browser }) => {
  instanz = await anhangStarten();
  const beta = await betaBeitreten(instanz.adresse);
  await perApiAblegen(instanz.adresse, beta, [...Array(9).fill(1048576), 843776]);
  const { page, fehler } = await seiteAuf(browser);
  const anfragen = anfragenMitschreiben(page);

  await waehlen(page, [fotoPng()]);
  await eingabe(page).fill('trotzdem');
  await eingabe(page).press('Enter');
  const datei = dateiImEintrag(page, 'foto.png');
  await expect(datei.locator('.lc-hochladung-grund')).toHaveText(`⚠ ${VOLL}`);
  await datei.getByRole('button', { name: 'foto.png entfernen', exact: true }).click();

  await expect(eintrag(page)).toHaveCount(0);
  const bestaetigt = page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)').filter({ hasText: 'trotzdem' });
  await expect(bestaetigt).toHaveCount(1);
  const gesendet = anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).map((anfrage) => anfrage.koerper);
  expect(gesendet).toEqual([{ text: 'trotzdem' }]);
  expect(fehler).toEqual([]);
});

test('Zu groß: 10.485.761 Bytes → ⚠ mit dem Text der 422 `zu_gross`, kein PUT', async ({ browser }) => {
  instanz = await anhangStarten();
  const { page, fehler } = await seiteAuf(browser);
  const anfragen = anfragenMitschreiben(page);

  await waehlen(page, [archivZip('gross.bin', 10485761)]);
  await knopf(page).click();

  await expect(dateiImEintrag(page, 'gross.bin').locator('.lc-hochladung-grund'))
    .toHaveText('⚠ Nicht hochgeladen: die Datei ist größer als der ganze Platz für Anhänge dieser Instanz (10,0 MiB).');
  expect(anfragen.filter((anfrage) => anfrage.methode === 'PUT')).toEqual([]);
  expect(fehler).toEqual([]);
});

test('Wartet: 507 `hochladungen` → „wartet · neuer Versuch in 30 s“, kein ⚠; nach Retry-After meldet der Browser selbst neu an', async ({ browser }) => {
  test.setTimeout(90000);
  instanz = await anhangStarten();
  const beta = await betaBeitreten(instanz.adresse);
  const gehalten = await perApiAnmelden(instanz.adresse, beta, 'gross.bin', 10485760);
  const { page, fehler } = await seiteAuf(browser);
  const anfragen = anfragenMitschreiben(page);
  await hochladenAnhalten(page);

  await waehlen(page, [fotoPng()]);
  await knopf(page).click();
  const datei = dateiImEintrag(page, 'foto.png');
  await expect(datei).toContainText('wartet · neuer Versuch in 30 s');
  const ersteAbweisung = Date.now();
  await expect(datei).not.toContainText('⚠');
  const verworfen = await perApi(instanz.adresse, { methode: 'DELETE', pfad: `api/anhaenge/${gehalten}`, token: beta });
  expect(verworfen.status).toBeLessThan(300);

  const anmeldungen = () => anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge').length;
  await expect.poll(anmeldungen, { timeout: 45000 }).toBe(2);
  await expect(datei).not.toContainText('wartet', { timeout: 5000 });
  expect(Date.now() - ersteAbweisung).toBeLessThanOrEqual(40000);
  expect(fehler).toEqual([]);
});
