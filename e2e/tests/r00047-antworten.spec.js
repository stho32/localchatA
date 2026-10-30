'use strict';
// R00047 I0044 F0159 Im Browser mit Zitat antworten (ADR-0053 Regeln 6–8): die Aktion „Antworten“ in Leiste, Menü „⋯“,
// Tab-Weg und Taste R (B0713).
const { test, expect } = require('./rahmen');
const { sendenStarten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const { imBrowserBeitreten } = require('./senden-hilfen');
const { nachrichtMitZeit, loeschenPerApi, mitAnhaengenSenden, PNG } = require('./r00041-hilfen');
const {
  ANNA, annaSchreibt, claudeSchreibt, benOeffnen, antwortenKnopf, ueberHoverAntworten, tippenUndSenden,
} = require('./r00047-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const FENSTER_TELEFON = { width: 375, height: 667 };

/// Zählt die POST-Aufrufe der Seite an `…/nachrichten`.
function sendeaufrufeZaehlen(page) {
  const aufrufe = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().includes('/nachrichten')) {
      aufrufe.push(anfrage.url());
    }
  });
  return aufrufe;
}

test('Leiste: „Antworten“ steht zuerst — an der fremden Antworten · Kopieren, an der eigenen Antworten · Kopieren · Bearbeiten · Löschen; Klick setzt den Fokus ins Feld ohne Netzaufruf (B0713)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse);
  const eigene = await tippenUndSenden(b.page, 'Hallo');
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  const aufrufe = sendeaufrufeZaehlen(b.page);

  await nachrichtMitZeit(b.page, anna.zeiten[0]).hover();
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0]).locator('.lc-aktion')).toHaveText(['Antworten', 'Kopieren']);
  await nachrichtMitZeit(b.page, eigene).hover();
  await expect(nachrichtMitZeit(b.page, eigene).locator('.lc-aktion')).toHaveText(['Antworten', 'Kopieren', 'Bearbeiten', 'Löschen']);

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await expect(eingabe(b.page)).toBeFocused();
  expect(aufrufe).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('Tastatur: R auf der fokussierten Nachricht setzt den Fokus ins Feld; Tab von der Nachricht erreicht „Antworten“, Enter löst aus (B0713)', async ({ browser }) => {
  instanz = await sendenStarten();
  const claude = await claudeSchreibt(instanz.adresse, ['Ergebnis']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, claude.zeiten[0])).toHaveCount(1);

  for (const taste of ['r', 'R']) {
    await nachrichtMitZeit(b.page, claude.zeiten[0]).focus();
    await b.page.keyboard.press(taste);
    await expect(eingabe(b.page), `Taste ${taste}`).toBeFocused();
    await expect(eingabe(b.page)).toHaveValue('');
  }

  await nachrichtMitZeit(b.page, claude.zeiten[0]).focus();
  await b.page.keyboard.press('Tab');
  await expect(antwortenKnopf(b.page, claude.zeiten[0])).toBeFocused();
  await b.page.keyboard.press('Enter');
  await expect(eingabe(b.page)).toBeFocused();
  expect(b.fehler).toEqual([]);
});

test('Taste R wirkungslos mit Strg, Alt, Meta, Umschalt, bei offenem Dialog, auf dem Knopf und bei markiertem Text; im Feld tippt sie den Buchstaben (B0713)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Ein Wort steht hier']);
  const b = await benOeffnen(browser, instanz.adresse);
  const nachricht = nachrichtMitZeit(b.page, anna.zeiten[0]);
  await expect(nachricht).toHaveCount(1);

  for (const kombination of ['Control+r', 'Alt+r', 'Meta+r', 'Shift+R']) {
    await nachricht.focus();
    await b.page.keyboard.press(kombination);
    await expect(nachricht, kombination).toBeFocused();
  }

  await b.page.evaluate(() => {
    const dialog = document.createElement('dialog');
    dialog.id = 'lc-pruefen-dialog';
    dialog.setAttribute('open', '');
    document.body.append(dialog);
  });
  await nachricht.focus();
  await b.page.keyboard.press('r');
  await expect(nachricht, 'Dialog offen').toBeFocused();
  await b.page.evaluate(() => document.getElementById('lc-pruefen-dialog').remove());

  await nachricht.focus();
  await b.page.keyboard.press('Tab');
  await b.page.keyboard.press('Tab');
  await expect(nachricht.getByRole('button', { name: 'Kopieren', exact: true })).toBeFocused();
  await b.page.keyboard.press('r');
  await expect(nachricht.getByRole('button', { name: 'Kopieren', exact: true }), 'Fokus auf einem Knopf').toBeFocused();

  await nachricht.focus();
  await b.page.evaluate((zeit) => {
    const knoten = document.querySelector(`.lc-msg[data-zeit="${zeit}"] .lc-body`).firstChild;
    const bereich = document.createRange();
    bereich.setStart(knoten, 4);
    bereich.setEnd(knoten, 8);
    getSelection().removeAllRanges();
    getSelection().addRange(bereich);
  }, anna.zeiten[0]);
  await expect(nachricht).toBeFocused();
  await b.page.keyboard.press('r');
  await expect(nachricht, 'markierter Text').toBeFocused();

  await eingabe(b.page).focus();
  await b.page.keyboard.press('r');
  await expect(eingabe(b.page)).toHaveValue('r');
  expect(b.fehler).toEqual([]);
});

test('Touch: „Aktionen“ öffnet das Menü mit „Antworten“ und „Kopieren“ in dieser Reihenfolge, je ≥ 44 × 44 px mit 8 px Abstand; Tippen auf „Antworten“ setzt den Fokus ins Feld (B0713)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Eins', 'Zwei']);
  const b = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'ben');
  const nachricht = nachrichtMitZeit(b.page, anna.zeiten[1]);
  await expect(nachricht).toHaveCount(1);

  await nachricht.getByRole('button', { name: 'Aktionen', exact: true }).tap();
  const eintraege = nachricht.locator('.lc-aktion');
  await expect(eintraege).toHaveText(['Antworten', 'Kopieren']);
  const boxen = await eintraege.evaluateAll((knoepfe) => knoepfe.map((knopf) => knopf.getBoundingClientRect().toJSON()));
  for (const box of boxen) {
    expect(box.width).toBeGreaterThanOrEqual(44);
    expect(box.height).toBeGreaterThanOrEqual(44);
  }
  expect(boxen[1].top - boxen[0].bottom).toBeGreaterThanOrEqual(8);

  await antwortenKnopf(b.page, anna.zeiten[1]).tap();
  await expect(eingabe(b.page)).toBeFocused();
  await expect(nachricht.getByRole('button', { name: 'Aktionen', exact: true })).toHaveAttribute('aria-expanded', 'false');
  expect(b.fehler).toEqual([]);
});

test('Nicht angeboten: gelöschte, „wird gesendet …“ und fehlgeschlagene Nachricht tragen kein „Antworten“, R tut dort nichts; eine Nachricht nur aus einem Anhang trägt es (B0713)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Wird gelöscht']);
  await loeschenPerApi(instanz.adresse, anna.token, anna.zeiten[0], ANNA);
  const nurAnhang = await mitAnhaengenSenden(instanz.adresse, anna.token, '', [{ name: 'bericht.png', inhalt: PNG }], ANNA);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, nurAnhang)).toHaveCount(1);

  await nachrichtMitZeit(b.page, anna.zeiten[0]).hover();
  await expect(antwortenKnopf(b.page, anna.zeiten[0])).toHaveCount(0);
  await nachrichtMitZeit(b.page, anna.zeiten[0]).focus();
  await b.page.keyboard.press('r');
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toBeFocused();
  await nachrichtMitZeit(b.page, nurAnhang).hover();
  await expect(antwortenKnopf(b.page, nurAnhang)).toHaveCount(1);

  let durchlassen = false;
  await b.page.route('**/api/channels/allgemein/nachrichten', async (route) => {
    if (route.request().method() !== 'POST' || durchlassen) {
      await route.continue();
      return;
    }
    await new Promise((weiter) => setTimeout(weiter, 1500));
    await route.abort();
  });
  await eingabe(b.page).fill('Unterwegs');
  await eingabe(b.page).press('Enter');
  const vorlaeufig = b.page.getByRole('log').locator('.lc-vorlaeufig');
  await expect(vorlaeufig).toHaveCount(1);
  await vorlaeufig.hover();
  await expect(vorlaeufig.getByRole('button', { name: /Antworten|Aktionen/ })).toHaveCount(0);
  await expect(vorlaeufig.locator('.lc-fehlgeschlagen')).toHaveCount(1, { timeout: 10000 });
  await vorlaeufig.hover();
  await expect(vorlaeufig.getByRole('button', { name: /Antworten|Aktionen/ })).toHaveCount(0);
  durchlassen = true;
  expect(b.fehler.filter((fehler) => !fehler.includes('/api/channels/allgemein/nachrichten'))).toEqual([]);
});

