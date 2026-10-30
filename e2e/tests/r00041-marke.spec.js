'use strict';
// R00041 F0134 B0626 Marke „bearbeitet“ als Schalter (UX M8, PO 123, 128): ein Knopf mit aria-expanded, der Datum und
// Uhrzeit der letzten Änderung ein- und ausblendet — mit Maus, Tastatur, Touch und Bildschirmleser, in beiden Themen
// lesbar, groß genug, auch an eingeklappten und gruppierten Nachrichten.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const { betaSendet, bearbeitenPerApi, loeschenPerApi, nachrichtMitZeit, schriftKontrast } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const marke = (page, zeit) => nachrichtMitZeit(page, zeit).getByRole('button', { name: 'bearbeitet', exact: true });

/// „TT.MM.JJJJ, HH:MM“ aus einer Zeit der API (RFC 3339 mit Versatz, Ortszeit des Versatzes).
function zuletzt(geaendert) {
  const [, jahr, monat, tag, stunde, minute] = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(geaendert);
  return `Zuletzt bearbeitet: ${tag}.${monat}.${jahr}, ${stunde}:${minute}`;
}

/// beta sendet `texte`, bearbeitet die erste zu `Hallo zusammen`; → { token, zeiten, geaendert }.
async function bearbeiteteNachricht(texte = ['Hallo']) {
  const beta = await betaSendet(instanz.adresse, texte);
  const antwort = await bearbeitenPerApi(instanz.adresse, beta.token, beta.zeiten[0], 'Hallo zusammen');
  expect(antwort.status, JSON.stringify(antwort.daten)).toBe(200);
  return { ...beta, geaendert: antwort.daten.nachricht.geaendert };
}

test('Desktop (B0626): Knopf mit aria-expanded und Beschreibung; Klick, Enter, Leertaste blenden die Zeit ein und aus; title wie die Zeit', async ({ browser }) => {
  instanz = await sendenStarten();
  const { zeiten, geaendert } = await bearbeiteteNachricht();
  const a = await geraetOeffnen(browser, 'desktop', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const knopf = marke(a.page, zeiten[0]);
  const zeit = nachrichtMitZeit(a.page, zeiten[0]).getByText(zuletzt(geaendert), { exact: true });

  await expect(knopf).toHaveAttribute('aria-expanded', 'false');
  await expect(knopf).toHaveAccessibleDescription(zuletzt(geaendert));
  await expect(knopf).toHaveAttribute('title', zuletzt(geaendert));
  await expect(zeit).toBeHidden();
  await knopf.click();
  await expect(knopf).toHaveAttribute('aria-expanded', 'true');
  await expect(zeit).toBeVisible();
  await knopf.click();
  await expect(knopf).toHaveAttribute('aria-expanded', 'false');
  await expect(zeit).toBeHidden();
  await knopf.focus();
  await a.page.keyboard.press('Enter');
  await expect(knopf).toHaveAttribute('aria-expanded', 'true');
  await expect(zeit).toBeVisible();
  await a.page.keyboard.press(' ');
  await expect(knopf).toHaveAttribute('aria-expanded', 'false');
  await expect(zeit).toBeHidden();
  const box = await knopf.boundingBox();
  expect(box.width).toBeGreaterThanOrEqual(24);
  expect(box.height).toBeGreaterThanOrEqual(24);
  expect(a.fehler).toEqual([]);
});

for (const geraet of ['telefon', 'tablet']) {
  test(`${geraet} (B0626): Antippen zeigt Datum und Uhrzeit der letzten Änderung; nach einer zweiten Änderung die neue Zeit; Trefferfläche ≥ 44 px`, async ({ browser }) => {
    instanz = await sendenStarten();
    const beta = await bearbeiteteNachricht();
    const a = await geraetOeffnen(browser, geraet, { streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const knopf = marke(a.page, beta.zeiten[0]);
    await knopf.tap();
    await expect(knopf).toHaveAttribute('aria-expanded', 'true');
    await expect(nachrichtMitZeit(a.page, beta.zeiten[0]).getByText(zuletzt(beta.geaendert), { exact: true })).toBeVisible();
    const box = await knopf.boundingBox();
    expect(box.width).toBeGreaterThanOrEqual(44);
    expect(box.height).toBeGreaterThanOrEqual(44);

    const zweite = await bearbeitenPerApi(instanz.adresse, beta.token, beta.zeiten[0], 'Hallo noch einmal');
    expect(zweite.status).toBe(200);
    expect(zweite.daten.nachricht.geaendert).not.toBe(beta.geaendert);
    await expect(nachrichtMitZeit(a.page, beta.zeiten[0]).locator('.lc-body')).toHaveText('Hallo noch einmal');
    await marke(a.page, beta.zeiten[0]).tap();
    await expect(nachrichtMitZeit(a.page, beta.zeiten[0]).getByText(zuletzt(zweite.daten.nachricht.geaendert), { exact: true })).toBeVisible();
    await expect(marke(a.page, beta.zeiten[0])).toHaveAccessibleDescription(zuletzt(zweite.daten.nachricht.geaendert));
    expect(a.fehler).toEqual([]);
  });
}

for (const thema of ['light', 'dark']) {
  test(`Kontrast (B0626, ${thema}): die Marke ≥ 4,5 : 1, ruhend und mit Hover`, async ({ browser }) => {
    instanz = await sendenStarten();
    const { zeiten } = await bearbeiteteNachricht();
    const a = await geraetOeffnen(browser, 'desktop', { thema, streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await a.page.mouse.move(0, 0);
    expect(await schriftKontrast(marke(a.page, zeiten[0]))).toBeGreaterThanOrEqual(4.5);
    await nachrichtMitZeit(a.page, zeiten[0]).hover();
    expect(await schriftKontrast(marke(a.page, zeiten[0]))).toBeGreaterThanOrEqual(4.5);
    expect(a.fehler).toEqual([]);
  });
}

test('Eingeklappt und gruppiert (B0626): die Marke ist an einer langen eingeklappten und an einer gruppierten Nachricht sichtbar und bedienbar', async ({ browser }) => {
  instanz = await sendenStarten();
  const lang = Array.from({ length: 40 }, (_, zeile) => `Zeile ${zeile + 1}`).join('\n');
  const beta = await betaSendet(instanz.adresse, ['Erste', 'Zweite']);
  expect((await bearbeitenPerApi(instanz.adresse, beta.token, beta.zeiten[0], lang)).status).toBe(200);
  expect((await bearbeitenPerApi(instanz.adresse, beta.token, beta.zeiten[1], 'Gruppiert')).status).toBe(200);
  const a = await geraetOeffnen(browser, 'desktop', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0]).getByRole('button', { name: /^Ganze Nachricht anzeigen/ })).toBeVisible();
  await expect(nachrichtMitZeit(a.page, beta.zeiten[1])).not.toHaveClass(/\bfirst\b/);
  for (const zeit of beta.zeiten) {
    const knopf = marke(a.page, zeit);
    await knopf.scrollIntoViewIfNeeded();
    await expect(knopf).toBeVisible();
    await knopf.click();
    await expect(knopf).toHaveAttribute('aria-expanded', 'true');
  }
  expect(a.fehler).toEqual([]);
});

for (const thema of ['light', 'dark']) {
  test(`Kontrast „Nachricht gelöscht“ (F0138, ${thema}): ≥ 4,5 : 1, ruhend und mit Hover`, async ({ browser }) => {
    instanz = await sendenStarten();
    const beta = await betaSendet(instanz.adresse, ['Hallo']);
    expect((await loeschenPerApi(instanz.adresse, beta.token, beta.zeiten[0])).status).toBe(200);
    const a = await geraetOeffnen(browser, 'desktop', { thema, streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const marke = nachrichtMitZeit(a.page, beta.zeiten[0]).locator('.lc-body');
    await expect(marke).toHaveText('Nachricht gelöscht');
    await a.page.mouse.move(0, 0);
    expect(await schriftKontrast(marke)).toBeGreaterThanOrEqual(4.5);
    await nachrichtMitZeit(a.page, beta.zeiten[0]).hover();
    expect(await schriftKontrast(marke)).toBeGreaterThanOrEqual(4.5);
    expect(a.fehler).toEqual([]);
  });
}
