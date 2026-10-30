'use strict';
// R00047 I0044 F0159: die Antwortzeile über dem Eingabefeld (B0714, ADR-0053 Regel 7) — Zeile, Abbrechen und Esc,
// Channelwechsel, das Original bleibt sichtbar (Z2, mit R00048), schmal (Z3).
const { test, expect } = require('./rahmen');
const { channelAnlegen } = require('./hilfen');
const { sendenStarten, eingabe, imBrowserBeitreten } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { annaSchreibt, benOeffnen, antwortzeile, ueberHoverAntworten, sendeaufrufeZaehlen } = require('./r00047-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ANNA_KENNUNG = 'anna@127.0.0.2';

test('Zeile: „Antwort an <Kennung>: <Textanfang>“ mit „Abbrechen“ über dem Feld, role group mit Namen, ohne aria-describedby; eine zweite Wahl ersetzt sie (B0714)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?', 'Noch eine']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[1])).toHaveCount(1);
  await expect(antwortzeile(b.page)).toHaveCount(0);

  await ueberHoverAntworten(b.page, anna.zeiten[0]);

  const zeile = antwortzeile(b.page);
  await expect(zeile).toHaveCount(1);
  await expect(zeile).toHaveAccessibleName(`Antwort an ${ANNA_KENNUNG}`);
  await expect(zeile).toContainText(`Antwort an ${ANNA_KENNUNG}:`);
  await expect(zeile.locator('.lc-antwort-text')).toHaveText('Frage?');
  await expect(zeile.getByRole('button', { name: 'Abbrechen', exact: true })).toBeVisible();
  expect(await zeile.getAttribute('aria-describedby')).toBeNull();
  expect(await eingabe(b.page).getAttribute('aria-describedby')).not.toContain('lc-antwort');
  await expect(eingabe(b.page)).toBeFocused();
  const reihenfolge = await b.page.locator('.lc-comp').evaluate((rahmen) => {
    const zeileOben = rahmen.querySelector('.lc-antwortzeile').getBoundingClientRect().bottom;
    const feldOben = rahmen.querySelector('.lc-comp-feld').getBoundingClientRect().top;
    return zeileOben <= feldOben;
  });
  expect(reihenfolge, 'die Zeile steht über dem Feld').toBe(true);

  await ueberHoverAntworten(b.page, anna.zeiten[1]);
  await expect(antwortzeile(b.page)).toHaveCount(1);
  await expect(antwortzeile(b.page).locator('.lc-antwort-text')).toHaveText('Noch eine');
  expect(b.fehler).toEqual([]);
});

test('Abbrechen und Esc: die Zeile geht, der getippte Text bleibt, der Fokus steht im Feld, kein Netzaufruf (B0714)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  const aufrufe = sendeaufrufeZaehlen(b.page);

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await b.page.keyboard.type('Ja');
  await antwortzeile(b.page).getByRole('button', { name: 'Abbrechen', exact: true }).click();
  await expect(antwortzeile(b.page)).toHaveCount(0);
  await expect(eingabe(b.page)).toBeFocused();
  await expect(eingabe(b.page)).toHaveValue('Ja');

  await nachrichtMitZeit(b.page, anna.zeiten[0]).focus();
  await b.page.keyboard.press('r');
  await expect(antwortzeile(b.page)).toHaveCount(1);
  await b.page.keyboard.press('Escape');
  await expect(antwortzeile(b.page)).toHaveCount(0);
  await expect(eingabe(b.page)).toBeFocused();
  await expect(eingabe(b.page)).toHaveValue('Ja');
  expect(aufrufe).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('Zeile und Channelwechsel: nach # technik und zurück ist die Zeile weg, der Entwurf bleibt, die Hinweiszeile sagt, dass die Antwort verworfen wurde (B0714, Mutant Z15)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);

  await ueberHoverAntworten(b.page, anna.zeiten[0]);
  await b.page.keyboard.type('Ja');
  await b.page.getByRole('link', { name: 'technik' }).click();
  await expect(b.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(antwortzeile(b.page)).toHaveCount(0);
  await expect(b.page.locator('.lc-statuszeile')).toHaveText(`Die Antwort an ${ANNA_KENNUNG} wurde beim Wechsel verworfen.`);
  await b.page.getByRole('link', { name: 'allgemein' }).click();
  await expect(b.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  await expect(eingabe(b.page)).toHaveValue('Ja');
  await expect(antwortzeile(b.page)).toHaveCount(0);

  await b.page.reload();
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0])).toHaveCount(1);
  await expect(antwortzeile(b.page)).toHaveCount(0);
  expect(b.fehler).toEqual([]);
});

/// Lage der Nachricht im Verlauf: ganz sichtbar? Abstand ihrer Unterkante zur Unterkante des Verlaufs, scrollTop.
const lageImVerlauf = (page, zeit) => nachrichtMitZeit(page, zeit).evaluate((knoten) => {
  const log = knoten.closest('[role="log"]');
  const rahmen = log.getBoundingClientRect();
  const rand = knoten.getBoundingClientRect();
  return {
    ganz: rand.top >= rahmen.top - 0.5 && rand.bottom <= rahmen.bottom + 0.5,
    abstandUnten: Math.round(rahmen.bottom - rand.bottom),
    scrollTop: log.scrollTop,
  };
});

for (const fenster of [{ width: 375, height: 667 }, { width: 375, height: 367 }]) {
  for (const vonUnten of [1, 6]) {
    test(`Das Original bleibt sichtbar (Z2, R00048) bei ${fenster.width} × ${fenster.height}, Original ${vonUnten}. von unten: danach ganz im Bild; war es das schon, bleibt sein Abstand zur Unterkante (B0714)`, async ({ browser }) => {
      instanz = await sendenStarten();
      const texte = Array.from({ length: 20 }, (_, nummer) => `Nachricht ${nummer + 1}`);
      const anna = await annaSchreibt(instanz.adresse, texte);
      const b = await geraetOeffnen(browser, 'telefon', { fenster, streng: true });
      await imBrowserBeitreten(b.page, instanz.adresse, 'ben');
      const zeit = anna.zeiten[anna.zeiten.length - vonUnten];
      await expect(nachrichtMitZeit(b.page, anna.zeiten[19])).toHaveCount(1);
      await nachrichtMitZeit(b.page, zeit).evaluate((knoten) => knoten.focus({ preventScroll: true }));
      const vorher = await lageImVerlauf(b.page, zeit);

      await b.page.keyboard.press('r');
      await expect(antwortzeile(b.page)).toHaveCount(1);
      await b.page.evaluate(() => new Promise((fertig) => requestAnimationFrame(() => requestAnimationFrame(fertig))));

      const nachher = await lageImVerlauf(b.page, zeit);
      expect(nachher.ganz, JSON.stringify({ vorher, nachher })).toBe(true);
      if (vorher.ganz) {
        expect(nachher.abstandUnten, JSON.stringify({ vorher, nachher })).toBe(vorher.abstandUnten);
      }
      expect(b.fehler).toEqual([]);
    });
  }
}

// Die Kennung des Kriteriums `claude-entwickler@server.de` gibt es nicht (Herkunft ist immer eine IP-Adresse, Kennung
// in src/gespraeche/kennung.rs); geprüft wird mit derselben Namenslänge und einer längeren Herkunft (IPv6, 39 Zeichen).
test('Zeile bei 320 px (Z3): Antwort an claude-entwickler@<IPv6> bricht um ohne Querscroll; „Abbrechen“ mit Maus ≥ 24 × 24 px, auf Touch ≥ 44 × 44 px; danach Fokus im Feld, der Text bleibt (B0714)', async ({ browser }) => {
  const kennung = 'claude-entwickler@fd12:3456:789a:bcde:f012:3456:789a:bcde';
  const zeit = '2026-10-28T14:03:00.000+01:00';
  instanz = await sendenStarten({ vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein', { '2026-10-28': [[zeit, kennung, 'Wie lautet der Stand der Dinge bei der langen Kennung?']] }) });
  for (const [geraet, mindestens] of [['desktop', 24], ['telefon', 44]]) {
    const b = await geraetOeffnen(browser, geraet, { fenster: { width: 320, height: 568 }, streng: true });
    await imBrowserBeitreten(b.page, instanz.adresse, `ben${geraet}`);
    await expect(nachrichtMitZeit(b.page, zeit)).toHaveCount(1);
    await nachrichtMitZeit(b.page, zeit).focus();
    await b.page.keyboard.press('r');
    const zeile = antwortzeile(b.page);
    await expect(zeile).toContainText(`Antwort an ${kennung}:`);
    await b.page.keyboard.type('Ja');
    const breiten = await b.page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: document.documentElement.clientWidth }));
    expect(breiten.inhalt, geraet).toBe(breiten.fenster);
    const zeileRand = await zeile.evaluate((element) => element.getBoundingClientRect().toJSON());
    expect(zeileRand.right, geraet).toBeLessThanOrEqual(320);
    const abbrechen = zeile.getByRole('button', { name: 'Abbrechen', exact: true });
    const rand = await abbrechen.evaluate((knopf) => knopf.getBoundingClientRect().toJSON());
    expect(rand.width, geraet).toBeGreaterThanOrEqual(mindestens);
    expect(rand.height, geraet).toBeGreaterThanOrEqual(mindestens);
    expect(rand.right, geraet).toBeLessThanOrEqual(320);
    if (geraet === 'telefon') {
      await abbrechen.tap();
    } else {
      await abbrechen.click();
    }
    await expect(zeile).toHaveCount(0);
    await expect(eingabe(b.page)).toBeFocused();
    await expect(eingabe(b.page)).toHaveValue('Ja');
    expect(b.fehler).toEqual([]);
    await b.kontext.close();
  }
});
