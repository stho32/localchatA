'use strict';
// R00047 I0044 F0159: Ansagen beim Antworten, kein Thread, Mensch und Agent gleich (B0719).
const { test, expect } = require('./rahmen');
const { sendenStarten, eingabe } = require('./senden-hilfen');
const { nachrichtMitZeit, ansagenMitschreiben } = require('./r00041-hilfen');
const {
  ANNA, CLAUDE, annaSchreibt, claudeSchreibt, mitBezugSenden, benOeffnen, antwortzeile, zitatVon, ueberHoverAntworten, tippenUndSenden,
  perApiLesen,
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

test('Ansage: „Antworten“ sagt genau einmal „Antwort an <Kennung>, <HH:MM>.“; dieselbe Nachricht noch einmal sagt nichts; Abbrechen und Esc je einmal „Antwort abgebrochen.“; Senden wie bisher (B0719)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?', 'Zweite']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[1])).toHaveCount(1);
  const angesagt = await ansagenMitschreiben(b.page);
  const ersteAnsage = `Antwort an ${ANNA_KENNUNG}, ${hhmm(anna.zeiten[0])}.`;
  const zweiteAnsage = `Antwort an ${ANNA_KENNUNG}, ${hhmm(anna.zeiten[1])}.`;

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await expect.poll(angesagt).toEqual([ersteAnsage]);
  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await b.page.waitForTimeout(300);
  expect(await angesagt()).toEqual([ersteAnsage]);
  await ueberHoverAntworten(b.page, anna.zeiten[1]);
  await expect.poll(angesagt).toEqual([ersteAnsage, zweiteAnsage]);

  await antwortzeile(b.page).getByRole('button', { name: 'Abbrechen', exact: true }).click();
  await expect.poll(angesagt).toEqual([ersteAnsage, zweiteAnsage, 'Antwort abgebrochen.']);
  await nachrichtMitZeit(b.page, anna.zeiten[0]).focus();
  await b.page.keyboard.press('r');
  await expect.poll(angesagt).toEqual([ersteAnsage, zweiteAnsage, 'Antwort abgebrochen.', ersteAnsage]);
  await b.page.keyboard.press('Escape');
  await expect.poll(angesagt).toEqual([ersteAnsage, zweiteAnsage, 'Antwort abgebrochen.', ersteAnsage, 'Antwort abgebrochen.']);

  await nachrichtMitZeit(b.page, anna.zeiten[0]).focus();
  await b.page.keyboard.press('r');
  await tippenUndSenden(b.page, 'Ja');
  await b.page.waitForTimeout(300);
  expect(await angesagt()).toEqual([ersteAnsage, zweiteAnsage, 'Antwort abgebrochen.', ersteAnsage, 'Antwort abgebrochen.', ersteAnsage]);
  expect(b.fehler).toEqual([]);
});

test('Kein Thread (US-20): am Original kein Zähler, keine Ansicht „Antworten“; die Antwort steht in Zeitfolge und bleibt nach 10 weiteren Nachrichten, wo sie gesendet wurde (B0719)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  const antwort = await tippenUndSenden(b.page, 'Ja');
  const claude = await claudeSchreibt(instanz.adresse, []);
  for (let nummer = 1; nummer <= 10; nummer += 1) {
    await mitBezugSenden(instanz.adresse, claude.token, `Danach ${nummer}`, null, CLAUDE);
    await new Promise((weiter) => setTimeout(weiter, 120));
  }
  await expect(b.page.getByRole('log').locator('.lc-msg[data-zeit]').last().locator('.lc-body')).toHaveText('Danach 10');

  const zeiten = await b.page.getByRole('log').locator('.lc-msg[data-zeit]').evaluateAll((alle) => alle.map((knoten) => knoten.dataset.zeit));
  expect(zeiten.indexOf(antwort)).toBe(1);
  expect([...zeiten].sort((links, rechts) => Date.parse(links) - Date.parse(rechts))).toEqual(zeiten);
  const original = nachrichtMitZeit(b.page, anna.zeiten[0]);
  await original.hover();
  await expect(original.getByRole('button')).toHaveText(['Antworten', 'Kopieren']);
  await expect(original).not.toContainText(/\d+\s*Antwort/);
  await expect(b.page.getByRole('log')).toHaveCount(1);
  expect(b.fehler).toEqual([]);
});

test('Beide Wege gleich (US-21): die im Browser gesendete Antwort trägt per API denselben Bezug wie die per API gesendete, und beide zeigen im Browser dasselbe Zitat (B0719)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const claude = await claudeSchreibt(instanz.adresse, []);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  const imBrowser = await tippenUndSenden(b.page, 'Ja');
  const perApi = await mitBezugSenden(instanz.adresse, claude.token, 'Auch ja', anna.zeiten[0], CLAUDE);
  await expect(nachrichtMitZeit(b.page, perApi)).toHaveCount(1);

  const bezugBrowser = (await perApiLesen(instanz.adresse, anna.token, imBrowser, ANNA)).antwort_auf;
  const bezugApi = (await perApiLesen(instanz.adresse, anna.token, perApi, ANNA)).antwort_auf;
  expect(bezugBrowser).toEqual(bezugApi);
  expect(bezugBrowser).toMatchObject({ von: ANNA_KENNUNG, zustand: 'da', textanfang: 'Frage?' });
  for (const teil of ['.lc-zitat-kopf', '.lc-zitat-text']) {
    expect(await zitatVon(b.page, imBrowser).locator(teil).textContent()).toBe(await zitatVon(b.page, perApi).locator(teil).textContent());
  }
  expect(await zitatVon(b.page, imBrowser).getAttribute('aria-label')).toBe(await zitatVon(b.page, perApi).getAttribute('aria-label'));
  await expect(eingabe(b.page)).toHaveValue('');
  expect(b.fehler).toEqual([]);
});
