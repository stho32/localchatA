'use strict';
// R00047 I0044 F0159: eine Antwort im Browser senden (B0715, ADR-0053 Regel 7) — mit Bezug, Abweisung durch die Instanz,
// Fehlschlag und „Erneut senden“ mit demselben Bezug.
const { test, expect } = require('./rahmen');
const { sendenStarten, eingabe } = require('./senden-hilfen');
const { nachrichtMitZeit, loeschenPerApi } = require('./r00041-hilfen');
const {
  ANNA, annaSchreibt, benOeffnen, antwortzeile, zitatVon, ueberHoverAntworten, tippenUndSenden, perApiLesen,
} = require('./r00047-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ANNA_KENNUNG = 'anna@127.0.0.2';
const hhmm = (zeit) => zeit.slice(11, 16);

test('Antworten per Maus: Zeile, Ja tippen, senden → die Antwort steht am Ende mit dem Zitat anna, Uhrzeit, Frage?; die Zeile ist weg; die Instanz hat den Bezug (B0715)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?', 'Später', 'Noch später']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[2])).toHaveCount(1);

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await expect(antwortzeile(b.page)).toContainText(`Antwort an ${ANNA_KENNUNG}:`);
  await expect(antwortzeile(b.page).locator('.lc-antwort-text')).toHaveText('Frage?');
  const antwort = await tippenUndSenden(b.page, 'Ja');

  await expect(antwortzeile(b.page)).toHaveCount(0);
  await expect(b.page.getByRole('log').locator('.lc-msg').last()).toHaveAttribute('data-zeit', antwort);
  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-kopf')).toHaveText(`↩ ${ANNA_KENNUNG} ${hhmm(anna.zeiten[0])}`);
  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-text')).toHaveText('Frage?');
  const gelesen = await perApiLesen(instanz.adresse, anna.token, antwort, ANNA);
  expect(gelesen.antwort_auf).toMatchObject({ von: ANNA_KENNUNG, zustand: 'da', textanfang: 'Frage?' });
  expect(Date.parse(gelesen.antwort_auf.zeit)).toBe(Date.parse(anna.zeiten[0]));
  expect(b.fehler).toEqual([]);
});

test('Abweisung: das Original wird gelöscht, bevor ben sendet → der Text der Instanz steht am Feld, die Zeile ist weg, Ja steht im Feld, nichts ist gespeichert (B0715, US-16)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [422] } });
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await eingabe(b.page).fill('Ja');
  expect((await loeschenPerApi(instanz.adresse, anna.token, anna.zeiten[0], ANNA)).status).toBe(200);
  await eingabe(b.page).press('Enter');

  const hinweis = b.page.locator('#lc-comp-hinweis');
  await expect(hinweis).toHaveText(/^⚠ \S.+/);
  await expect(hinweis).toHaveClass(/lc-fehler/);
  await expect(antwortzeile(b.page)).toHaveCount(0);
  await expect(eingabe(b.page)).toHaveValue('Ja');
  await expect(b.page.getByRole('log').locator('.lc-vorlaeufig')).toHaveCount(0);
  const verlauf = await perApiLesen(instanz.adresse, anna.token, anna.zeiten[0], ANNA);
  expect(verlauf.zustand).toBe('geloescht');
  const alle = (await require('./senden-hilfen').verlaufLesen(instanz.adresse, anna.token, 'allgemein', ANNA)).map((nachricht) => nachricht.text);
  expect(alle).not.toContain('Ja');
  expect(b.fehler).toEqual([]);
});

test('Fehlschlag und Wiederholen: „wird gesendet …“ und fehlgeschlagen zeigen das Zitat; „Erneut senden“ sendet mit demselben Bezug (B0715, US-17, verzögerter Fake)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse, { serverWegErwartet: true });
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  const koerper = [];
  let scheitern = true;
  await b.page.route('**/api/channels/allgemein/nachrichten', async (route) => {
    if (route.request().method() !== 'POST') {
      await route.continue();
      return;
    }
    koerper.push(route.request().postDataJSON());
    if (scheitern) {
      await new Promise((weiter) => setTimeout(weiter, 1000));
      await route.abort('connectionrefused');
      return;
    }
    await route.continue();
  });

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await eingabe(b.page).fill('Ja');
  await eingabe(b.page).press('Enter');
  const vorlaeufig = b.page.getByRole('log').locator('.lc-vorlaeufig');
  await expect(vorlaeufig.locator('.lc-send')).toContainText('wird gesendet');
  await expect(vorlaeufig.locator('.lc-zitat-text')).toHaveText('Frage?');
  await expect(vorlaeufig.locator('.lc-send.lc-fehlgeschlagen')).toHaveCount(1, { timeout: 10000 });
  await expect(vorlaeufig.locator('.lc-zitat-kopf')).toHaveText(`↩ ${ANNA_KENNUNG} ${hhmm(anna.zeiten[0])}`);
  await expect(vorlaeufig.locator('.lc-zitat-text')).toHaveText('Frage?');

  scheitern = false;
  await vorlaeufig.getByRole('button', { name: 'Erneut senden' }).click();
  const bestaetigt = b.page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)').filter({ has: b.page.locator('.lc-body', { hasText: /^Ja$/ }) });
  await expect(bestaetigt).toHaveCount(1);
  await expect(bestaetigt.locator('.lc-zitat-text')).toHaveText('Frage?');
  expect(koerper.length).toBe(2);
  expect(koerper[1]).toEqual(koerper[0]);
  expect(Date.parse(koerper[1].antwort_auf)).toBe(Date.parse(anna.zeiten[0]));
  expect(b.fehler.filter((fehler) => !fehler.includes('/api/channels/allgemein/nachrichten'))).toEqual([]);
});
