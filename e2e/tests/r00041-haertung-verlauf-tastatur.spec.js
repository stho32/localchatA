'use strict';
// Karte #82 Härtung Verlauf-Tastatur (ux-reviewer #69): „Ganze Nachricht anzeigen“ — nach Enter liegt der Knopf in Sicht,
// ist ≥ 24 px hoch (Touch 44 px); nach Alt ↑/↓ (Channelwechsel) liegt der Fokus in der Eingabe, nicht auf body.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, box } = require('./r00027-hilfen');
const { betaSendet, nachrichtMitZeit } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const LANG = Array.from({ length: 30 }, (_, zeile) => `Zeile ${zeile + 1}`).join('\n');
const verlauf = (page) => page.getByRole('log');
const leiste = (page, zeit, name) => nachrichtMitZeit(page, zeit).getByRole('button', { name });

/// Zehn kurze Nachrichten, dann die lange — sie steht am unteren Rand des Verlaufs.
async function langeAmEnde() {
  const kurz = Array.from({ length: 10 }, (_, nummer) => `kurz ${nummer + 1}`);
  const beta = await betaSendet(instanz.adresse, [...kurz, LANG]);
  return beta.zeiten[beta.zeiten.length - 1];
}

test('Ganze Nachricht anzeigen: nach Enter liegt „Einklappen“ ganz im Verlauf, hat den Fokus und ist ≥ 24 px hoch; zurück ebenso', async ({ browser }) => {
  instanz = await sendenStarten();
  const zeit = await langeAmEnde();
  const a = await geraetOeffnen(browser, 'desktop', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');

  const anzeigen = leiste(a.page, zeit, /^Ganze Nachricht anzeigen/);
  await anzeigen.focus();
  expect((await box(anzeigen)).height).toBeGreaterThanOrEqual(24);
  await a.page.keyboard.press('Enter');

  const einklappen = leiste(a.page, zeit, 'Einklappen');
  await expect(einklappen).toBeFocused();
  const rahmen = await box(verlauf(a.page));
  const knopf = await box(einklappen);
  expect(knopf.top, 'oben im Verlauf').toBeGreaterThanOrEqual(rahmen.top);
  expect(knopf.bottom, 'unten im Verlauf').toBeLessThanOrEqual(rahmen.bottom);
  expect(knopf.height).toBeGreaterThanOrEqual(24);

  await a.page.keyboard.press('Enter');
  await expect(anzeigen).toBeFocused();
  const zurueck = await box(anzeigen);
  expect(zurueck.top).toBeGreaterThanOrEqual((await box(verlauf(a.page))).top);
  expect(zurueck.bottom).toBeLessThanOrEqual((await box(verlauf(a.page))).bottom);
  expect(a.fehler).toEqual([]);
});

test('Ganze Nachricht anzeigen auf Touch: Zielfläche ≥ 44 px hoch', async ({ browser }) => {
  instanz = await sendenStarten();
  const zeit = await langeAmEnde();
  const a = await geraetOeffnen(browser, 'telefon', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const knopf = leiste(a.page, zeit, /^Ganze Nachricht anzeigen/);
  await knopf.scrollIntoViewIfNeeded();
  expect((await box(knopf)).height).toBeGreaterThanOrEqual(44);
  expect(a.fehler).toEqual([]);
});

test('Channelwechsel mit Alt ↓ / Alt ↑ aus einer Nachricht: der Fokus liegt in der Eingabe, nicht auf body', async ({ browser }) => {
  instanz = await sendenStarten();
  await betaSendet(instanz.adresse, ['Hallo']);
  const a = await geraetOeffnen(browser, 'desktop', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const nachricht = verlauf(a.page).locator('.lc-msg').first();
  await nachricht.focus();
  await expect(nachricht).toBeFocused();

  await a.page.keyboard.press('Alt+ArrowDown');
  await expect(a.page).toHaveURL(/#\/c\/technik$/);
  await expect(verlauf(a.page)).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(eingabe(a.page)).toBeFocused();

  await a.page.keyboard.press('Alt+ArrowUp');
  await expect(a.page).toHaveURL(/#\/c\/allgemein$/);
  await expect(verlauf(a.page)).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await expect(eingabe(a.page)).toBeFocused();
  expect(a.fehler).toEqual([]);
});
