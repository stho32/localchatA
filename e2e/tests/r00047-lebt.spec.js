'use strict';
// R00047 I0044 F0160: das Zitat lebt mit dem Original (ADR-0053 Regel 5, P12, Mutant Z10) — in allen offenen Tabs
// (B0721), in der offenen Antwortzeile (B0722).
const { test, expect } = require('./rahmen');
const { sichtbarNach, fristPerApi } = require('./hilfen');
const { sendenStarten, beitreten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { browserOeffnen, nachrichtMitZeit, bearbeitenPerApi, loeschenPerApi } = require('./r00041-hilfen');
const {
  ANNA, CLAUDE, annaSchreibt, claudeSchreibt, mitBezugSenden, benOeffnen, zitatVon, antwortzeile, ueberHoverAntworten,
} = require('./r00047-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Prüfung für sichtbarNach: das Zitat der Antwort `antwort` zeigt `text`; liefert den Text.
function zitatZeigt({ antwort, text }) {
  const zitat = document.querySelector(`.lc-log .lc-msg[data-zeit="${antwort}"] .lc-zitat-text`);
  return zitat !== null && zitat.textContent === text && zitat.textContent;
}

test('Zitat lebt in zwei Tabs (P12, B0721): bearbeitet → binnen 1 s Frage!; gelöscht → binnen 1 s „Nachricht gelöscht“ ohne Text; die Lesestelle bleibt', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Ja', anna.zeiten[0], CLAUDE);
  const b = await benOeffnen(browser, instanz.adresse);
  const zweiterTab = await b.kontext.newPage();
  await zweiterTab.goto(`${instanz.adresse}#/c/allgemein`);
  const tabs = [b.page, zweiterTab];
  for (const tab of tabs) {
    await expect(zitatVon(tab, antwort).locator('.lc-zitat-text')).toHaveText('Frage?');
  }

  const lage = (tab) => nachrichtMitZeit(tab, anna.zeiten[0]).evaluate((knoten) => Math.round(knoten.getBoundingClientRect().top));
  const lagenVorher = [await lage(b.page), await lage(zweiterTab)];

  const bearbeitet = await bearbeitenPerApi(instanz.adresse, anna.token, anna.zeiten[0], 'Frage!', ANNA);
  expect(bearbeitet.status, JSON.stringify(bearbeitet.daten)).toBe(200);
  for (const [stelle, tab] of tabs.entries()) {
    const ms = await sichtbarNach(tab, zitatZeigt, { argument: { antwort, text: 'Frage!' }, ab: bearbeitet.empfangen, timeout: 10000 });
    console.log(`Messwert Zitat bearbeitet, Tab ${stelle + 1}: ${ms} ms`);
    expect(ms, 'bearbeitet, ms').toBeLessThanOrEqual(1000);
    expect(await lage(tab), 'Lesestelle').toBe(lagenVorher[stelle]);
  }

  const geloescht = await loeschenPerApi(instanz.adresse, anna.token, anna.zeiten[0], ANNA);
  expect(geloescht.status).toBe(200);
  for (const tab of tabs) {
    const ms = await sichtbarNach(tab, zitatZeigt, { argument: { antwort, text: 'Nachricht gelöscht' }, ab: geloescht.empfangen, timeout: 10000 });
    console.log(`Messwert Zitat gelöscht: ${ms} ms`);
    expect(ms, 'gelöscht, ms').toBeLessThanOrEqual(1000);
    await expect(zitatVon(tab, antwort)).not.toContainText('Frage');
    await expect(zitatVon(tab, antwort)).toHaveAttribute('aria-label', /Nachricht gelöscht$/);
  }
  expect(b.fehler).toEqual([]);
});

test('Ein aenderung vor dem Ende des Ladens landet nach dem Block im Zitat (P12, B0721)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Ja', anna.zeiten[0], CLAUDE);
  const b = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'ben', 'technik');
  let bearbeitet = null;
  await b.page.route(/\/api\/channels\/allgemein\/nachrichten\?n=100$/, async (route) => {
    const alteAntwort = await route.fetch();
    bearbeitet = await bearbeitenPerApi(instanz.adresse, anna.token, anna.zeiten[0], 'Frage!', ANNA);
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.fulfill({ response: alteAntwort });
  });

  await b.page.getByRole('link', { name: 'allgemein' }).click();

  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-text')).toHaveText('Frage!');
  expect(bearbeitet.status).toBe(200);
  expect(b.fehler).toEqual([]);
});

test('Entfallen (B0721): nach dem Verkürzen der Frist zeigt das Zitat beim nächsten Laden „Ältere Nachricht, nicht mehr vorhanden“', async ({ browser }) => {
  const original = '2026-10-25T10:00:00.000+01:00';
  instanz = await sendenStarten({
    vorbereiten: (ordner) => require('./hilfen').channelAnlegen(ordner, 'allgemein', { '2026-10-25': [[original, 'anna@127.0.0.2', 'Frage?']] }),
  });
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Ja', original, CLAUDE);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-text')).toHaveText('Frage?');

  const pfleger = await beitreten(instanz.adresse, 'pfleger');
  const frage = await fristPerApi(instanz.adresse, pfleger, 'allgemein', { tage: 1 });
  if (frage.status !== 200) {
    const bestaetigt = await fristPerApi(instanz.adresse, pfleger, 'allgemein', { tage: 1, bestaetigt: true });
    expect(bestaetigt.status, JSON.stringify(bestaetigt.daten)).toBe(200);
  }
  await b.page.reload();
  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-text')).toHaveText('Ältere Nachricht, nicht mehr vorhanden');
  await expect(nachrichtMitZeit(b.page, original)).toHaveCount(0);
  expect(b.fehler).toEqual([]);
});

test('Offene Zeile lebt (B0722): die Zeile „Antwort an anna: Frage?“ zeigt nach dem Bearbeiten Frage!, nach dem Löschen „Nachricht gelöscht“; der Text im Feld bleibt', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await eingabe(b.page).fill('Ja');
  const zeilentext = antwortzeile(b.page).locator('.lc-antwort-text');
  await expect(zeilentext).toHaveText('Frage?');

  expect((await bearbeitenPerApi(instanz.adresse, anna.token, anna.zeiten[0], 'Frage!', ANNA)).status).toBe(200);
  await expect(zeilentext).toHaveText('Frage!');
  expect((await loeschenPerApi(instanz.adresse, anna.token, anna.zeiten[0], ANNA)).status).toBe(200);
  await expect(zeilentext).toHaveText('Nachricht gelöscht');
  await expect(antwortzeile(b.page)).not.toContainText('Frage');
  await expect(eingabe(b.page)).toHaveValue('Ja');
  expect(b.fehler).toEqual([]);
});
