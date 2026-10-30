'use strict';
// R00047 I0044 Nacharbeit Karte #79 (Browserteil): Abnahme-Proben aus ux-reviewer und code-reviewer.
const { test, expect } = require('./rahmen');
const { sendenStarten, eingabe } = require('./senden-hilfen');
const { nachrichtMitZeit, ansagenMitschreiben, loeschenPerApi, bearbeitenPerApi } = require('./r00041-hilfen');
const { netzAus, netzAn } = require('./r00012-hilfen');
const { ANNA, annaSchreibt, mitBezugSenden, benOeffnen, antwortzeile, ueberHoverAntworten, tippenUndSenden, zitatVon } = require('./r00047-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ANNA_KENNUNG = 'anna@127.0.0.2';

/// Schneiden sich zwei Rechtecke (getBoundingClientRect) mit Fläche?
const schneiden = (a, b) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

for (const fenster of [{ width: 375, height: 667 }, { width: 320, height: 568 }, { width: 1280, height: 800 }]) {
  test(`UX S1 bei ${fenster.width} × ${fenster.height}: „Die Antwort an … wurde beim Wechsel verworfen.“ steht in der Hinweiszeile unter dem Feld, verdeckt weder Feld noch „Senden“ und wird einmal angesagt`, async ({ browser }) => {
    instanz = await sendenStarten();
    const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
    const b = await benOeffnen(browser, instanz.adresse, { geraet: { viewport: fenster }, streng: true });
    await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
    await nachrichtMitZeit(b.page, anna.zeiten[0]).focus();
    await b.page.keyboard.press('r');
    await expect(antwortzeile(b.page)).toHaveCount(1);
    const angesagt = await ansagenMitschreiben(b.page);
    const verworfen = `Die Antwort an ${ANNA_KENNUNG} wurde beim Wechsel verworfen.`;

    await b.page.evaluate(() => {
      window.location.hash = '#/c/technik';
    });
    await expect(b.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');

    const lage = await b.page.evaluate((text) => {
      const knopf = document.querySelector('.lc-comp-senden');
      const knopfRand = knopf.getBoundingClientRect();
      const mitte = document.elementFromPoint(knopfRand.left + knopfRand.width / 2, knopfRand.top + knopfRand.height / 2);
      const traeger = [...document.querySelectorAll('body *')]
        .filter((element) => element.textContent === text && element.children.length === 0 && element.closest('[aria-live]') === null)
        .filter((element) => element.closest('[hidden]') === null);
      return {
        mitteIstSenden: mitte === knopf,
        meldung: traeger.map((element) => element.getBoundingClientRect().toJSON()),
        feld: document.querySelector('.lc-comp-feld').getBoundingClientRect().toJSON(),
        senden: knopfRand.toJSON(),
      };
    }, verworfen);
    expect(lage.mitteIstSenden, JSON.stringify(lage)).toBe(true);
    await expect(b.page.locator('#lc-comp-hinweis')).toHaveText(verworfen);
    await expect(b.page.locator('#lc-comp-hinweis')).toBeInViewport({ ratio: 1 });
    expect(lage.meldung.length, JSON.stringify(lage)).toBe(1);
    expect(schneiden(lage.meldung[0], lage.feld), JSON.stringify(lage)).toBe(false);
    expect(schneiden(lage.meldung[0], lage.senden), JSON.stringify(lage)).toBe(false);
    await b.page.waitForTimeout(500);
    expect((await angesagt()).filter((text) => text.includes(verworfen)).length, JSON.stringify(await angesagt())).toBe(1);
    expect((await angesagt()).join(' ').split(verworfen).length - 1).toBe(1);
    await expect(eingabe(b.page)).toHaveValue('');
    expect(b.fehler).toEqual([]);
  });
}

test('UX S1: die Meldung „… wurde beim Wechsel verworfen.“ geht nach 6 s von selbst und weicht sofort der nächsten Eingabe', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  const hinweisZeile = b.page.locator('#lc-comp-hinweis');
  const verworfen = `Die Antwort an ${ANNA_KENNUNG} wurde beim Wechsel verworfen.`;

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await b.page.getByRole('link', { name: 'technik' }).click();
  await expect(hinweisZeile).toHaveText(verworfen);
  const gezeigtAb = Date.now();
  await expect(hinweisZeile).toHaveText('', { timeout: 10000 });
  expect(Date.now() - gezeigtAb).toBeGreaterThan(5000);

  await b.page.getByRole('link', { name: 'allgemein' }).click();
  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await b.page.getByRole('link', { name: 'technik' }).click();
  await expect(hinweisZeile).toHaveText(verworfen);
  await eingabe(b.page).press('x');
  await expect(hinweisZeile).toHaveText('');
  await expect(hinweisZeile).not.toHaveClass(/lc-fehler/);
  expect(b.fehler).toEqual([]);
});

const BEARBEITEN = '/api/channels/allgemein/nachrichten/bearbeiten';
const bearbeitenFeld = (page) => page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' });

/// ben antwortet anna im Browser mit „Ja“ und öffnet das Bearbeiten-Feld dieser Antwort; → { anna, antwort }.
async function eigeneAntwortBearbeiten(page) {
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  await expect(nachrichtMitZeit(page, anna.zeiten[0])).toHaveCount(1);
  await ueberHoverAntworten(page, anna.zeiten[0]);
  const antwort = await tippenUndSenden(page, 'Ja');
  await expect(zitatVon(page, antwort)).toHaveCount(1);
  await nachrichtMitZeit(page, antwort).hover();
  await nachrichtMitZeit(page, antwort).getByRole('button', { name: 'Bearbeiten', exact: true }).click();
  await expect(bearbeitenFeld(page)).toBeFocused();
  await bearbeitenFeld(page).fill('Doch nein');
  return { anna, antwort };
}

test('code-reviewer S3: Speichern einer Antwort mit 409 (Route) → kein Zitat über „Nachricht gelöscht“', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await benOeffnen(browser, instanz.adresse, { streng: false, zusaetzlich: { [BEARBEITEN]: [409] } });
  await b.page.route(`**${BEARBEITEN}`, (route) => route.fulfill({
    status: 409, contentType: 'application/json', body: JSON.stringify({ fehler: 'Nachricht gelöscht' }),
  }));
  const { antwort } = await eigeneAntwortBearbeiten(b.page);

  await nachrichtMitZeit(b.page, antwort).getByRole('button', { name: 'Speichern' }).click();
  await expect(bearbeitenFeld(b.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(b.page, antwort).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await expect(nachrichtMitZeit(b.page, antwort).locator('.lc-zitat')).toHaveCount(0);
  expect(b.fehler).toEqual([]);
});

test('code-reviewer S3, Ereignisweg: die Antwort wird während des Bearbeitens per API gelöscht → das Feld schließt, „Nachricht gelöscht“ ohne Zitat', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await benOeffnen(browser, instanz.adresse);
  const { antwort } = await eigeneAntwortBearbeiten(b.page);
  const token = await b.page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  expect((await loeschenPerApi(instanz.adresse, token, antwort, '127.0.0.1')).status).toBe(200);

  await expect(bearbeitenFeld(b.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(b.page, antwort).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await expect(nachrichtMitZeit(b.page, antwort).locator('.lc-zitat')).toHaveCount(0);
  expect(b.fehler).toEqual([]);
});

test('UX S2 (b): Antwortzeile offen, das Original wird per API gelöscht → binnen 1 s sagt die Region einmal „Nachricht gelöscht“; die Zeile zeigt es, der Text im Feld bleibt', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await b.page.keyboard.type('Ja');
  const angesagt = await ansagenMitschreiben(b.page);
  const ansage = `Antwort an ${ANNA_KENNUNG}: Nachricht gelöscht.`;

  const geloeschtUm = Date.now();
  expect((await loeschenPerApi(instanz.adresse, anna.token, anna.zeiten[0], '127.0.0.2')).status).toBe(200);
  await expect.poll(angesagt, { intervals: [25], timeout: 10000 }).toEqual([ansage]);
  expect(Date.now() - geloeschtUm).toBeLessThanOrEqual(1000);
  await expect(antwortzeile(b.page).locator('.lc-antwort-text')).toHaveText('Nachricht gelöscht');
  await b.page.waitForTimeout(500);
  expect(await angesagt()).toEqual([ansage]);
  await expect(eingabe(b.page)).toHaveValue('Ja');
  expect(b.fehler).toEqual([]);
});

const ABGEWIESEN = 'Die Nachricht, auf die du antwortest, ist gelöscht.';
const SENDEN = '/api/channels/allgemein/nachrichten';

/// Hält jedes POST an `…/allgemein/nachrichten` `ms` lang auf, dann antwortet die Instanz.
const sendenVerzoegern = (page, ms) => page.route(`**${SENDEN}`, async (route) => {
  if (route.request().method() === 'POST') {
    await new Promise((weiter) => setTimeout(weiter, ms));
  }
  await route.continue();
});

/// ben beginnt eine Antwort auf annas „Frage?“, anna löscht sie, ben tippt „Ja“ und sendet (verzögert); → { anna }.
async function antwortAufGeloeschtesSenden(page) {
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  await expect(nachrichtMitZeit(page, anna.zeiten[0])).toHaveCount(1);
  await ueberHoverAntworten(page, anna.zeiten[0]);
  await eingabe(page).fill('Ja');
  expect((await loeschenPerApi(instanz.adresse, anna.token, anna.zeiten[0], '127.0.0.2')).status).toBe(200);
  await sendenVerzoegern(page, 1500);
  await eingabe(page).press('Enter');
  await expect(page.getByRole('log').locator('.lc-vorlaeufig')).toHaveCount(1);
  return { anna };
}

test('code-reviewer S1: 422 der Instanz → am Feld „⚠ Die Nachricht, auf die du antwortest, ist gelöscht.“, der zurückgegebene Text vor dem inzwischen getippten, einmal angesagt', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await benOeffnen(browser, instanz.adresse, { zusaetzlich: { [SENDEN]: [422] } });
  await antwortAufGeloeschtesSenden(b.page);
  const angesagt = await ansagenMitschreiben(b.page);
  await eingabe(b.page).fill('Inzwischen');

  const hinweisZeile = b.page.locator('#lc-comp-hinweis');
  await expect(hinweisZeile).toHaveText(`⚠ ${ABGEWIESEN}`, { timeout: 10000 });
  await expect(hinweisZeile).toHaveClass(/lc-fehler/);
  await expect(eingabe(b.page)).toHaveValue('Ja\nInzwischen');
  await expect(b.page.getByRole('log').locator('.lc-vorlaeufig')).toHaveCount(0);
  await expect.poll(angesagt).toEqual([ABGEWIESEN]);
  await b.page.waitForTimeout(500);
  expect(await angesagt()).toEqual([ABGEWIESEN]);
  expect(b.fehler).toEqual([]);
});

test('code-reviewer S1: Channelwechsel zwischen Senden und Abweisen → der Text geht in den Entwurf von # allgemein, # technik bleibt unberührt und ohne Hinweis', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await benOeffnen(browser, instanz.adresse, { zusaetzlich: { [SENDEN]: [422] } });
  const abgewiesen = b.page.waitForResponse((antwort) => antwort.url().endsWith(SENDEN) && antwort.status() === 422);
  await antwortAufGeloeschtesSenden(b.page);
  await b.page.getByRole('link', { name: 'technik' }).click();
  await expect(b.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await eingabe(b.page).fill('Hallo technik');

  await abgewiesen;
  await b.page.waitForTimeout(300);
  await expect(eingabe(b.page)).toHaveValue('Hallo technik');
  await expect(b.page.locator('#lc-comp-hinweis')).not.toContainText(ABGEWIESEN);
  await b.page.getByRole('link', { name: 'allgemein' }).click();
  await expect(b.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await expect(eingabe(b.page)).toHaveValue('Ja');
  await b.page.getByRole('link', { name: 'technik' }).click();
  await expect(eingabe(b.page)).toHaveValue('Hallo technik');
  expect(b.fehler).toEqual([]);
});

test('code-reviewer S2 (c): Strom unterbrochen, das Original wird bearbeitet, Strom wieder da → das Zitat zeigt den neuen Text', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const antwort = await mitBezugSenden(instanz.adresse, anna.token, 'Selbst geantwortet', anna.zeiten[0], ANNA);
  const b = await benOeffnen(browser, instanz.adresse, { ohneWorker: true });
  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-text')).toHaveText('Frage?');

  await netzAus(b);
  expect((await bearbeitenPerApi(instanz.adresse, anna.token, anna.zeiten[0], 'Frage!', ANNA)).status).toBe(200);
  await b.page.waitForTimeout(300);
  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-text')).toHaveText('Frage?');
  await netzAn(b);

  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-text')).toHaveText('Frage!', { timeout: 10000 });
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0]).locator('.lc-body')).toHaveText('Frage!');
  expect(b.fehler.filter((zeile) => !/ERR_INTERNET_DISCONNECTED/.test(zeile))).toEqual([]);
});
