'use strict';
// R00050 F0178 B0785 (Karte #13): das Menü der Teilnehmerliste beginnt ein Direktgespräch — ein Tab-Stopp mit
// Roving-Tabindex, je andere Zeile ein `button` mit aria-haspopup=menu und aria-expanded, `role=menu` mit `menuitem`
// „Direktnachricht“, ↑/↓ Home/Ende, Escape, Tab; keine Zeile und kein Menü am eigenen Teilnehmer; keine Eingabe einer
// freien Kennung; die Auswahl öffnet `#/d/<kennung>`, schließt das Menü und unter 1100 px das Panel, der Fokus steht in
// der Eingabe (V5, V6, US-20); der Sekundentakt zerstört Fokus und Menü nicht, eine Zeile, die bei offenem Menü geht,
// schließt es ohne Konsolenfehler. Mutanten, die rot werden müssen: Menüzeile am eigenen Teilnehmer; Kennung frei
// eintippbar; Sekundentakt baut die Zeile neu; zwei Tab-Stopps je Zeile. Gegen das echte Programm.
const { test, expect } = require('./rahmen');
const { instanzStarten, channelAnlegen, stromOeffnen } = require('./hilfen');
const { beitreten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');
const { GERAETE } = require('./r00027-hilfen');
const { BEN, eingabeAn, gespraechOffen } = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const CLAUDE = 'claude@127.0.0.1';
const region = (page) => page.getByRole('complementary', { name: 'Teilnehmer' });
const zeileVon = (page, kennung) => region(page).locator('.lc-person').filter({ has: page.locator('.lc-id', { hasText: kennung }) });
const knopfVon = (page, kennung) => zeileVon(page, kennung).getByRole('button');
const menue = (page) => page.getByRole('menu');
const direktnachricht = (page) => page.getByRole('menuitem', { name: 'Direktnachricht' });
const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));

/// anna im Browser, ben (Mensch, 127.0.0.3) und claude (Agent) per API; claude hält einen Strom offen, den
/// `claudeGeht()` schließt. → { anna, claudeGeht }
async function dreiTeilnehmer(browser, { geraet, umgebung = {} } = {}) {
  instanz = await instanzStarten({ umgebung, vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein') });
  const anna = await browserOeffnen(browser, { geraet });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await beitreten(instanz.adresse, 'ben', { art: 'mensch', herkunft: '127.0.0.3' });
  const strom = await stromOeffnen(instanz.adresse, await beitreten(instanz.adresse, 'claude'));
  return { anna, claudeGeht: strom.schliessen };
}

async function listeOffen(page) {
  if (await region(page).isHidden()) {
    await page.getByRole('button', { name: /^Teilnehmer/ }).click();
  }
  await expect(knopfVon(page, BEN)).toBeVisible();
  await expect(knopfVon(page, CLAUDE)).toBeVisible();
}

/// Die Tab-Stopps in der Region (tabindex ≥ 0, sichtbar), je mit der Kennung ihrer Zeile — die Region selbst als „Region“.
function tabStoppsIn(selektor) {
  const liste = document.querySelector(selektor);
  const stopps = [...liste.querySelectorAll('button, a, input, textarea, [tabindex]'), liste]
    .filter((element) => element.tabIndex >= 0 && !element.disabled && element.getClientRects().length > 0);
  return stopps.map((element) => element.closest('.lc-person')?.dataset.kennung ?? 'Region');
}

test('Tastatur (B0785, V6 b, c): ein Tab-Stopp, Pfeile und Home/Ende, Enter und Leertaste öffnen, Escape und Tab schließen', async ({ browser }) => {
  const { anna } = await dreiTeilnehmer(browser);
  await listeOffen(anna.page);

  expect(await anna.page.evaluate(tabStoppsIn, '#lc-teilnehmer')).toEqual([BEN]);
  await expect(zeileVon(anna.page, 'anna@127.0.0.1').getByRole('button')).toHaveCount(0);
  await expect(zeileVon(anna.page, 'anna@127.0.0.1').locator('[aria-haspopup]')).toHaveCount(0);
  for (const kennung of [BEN, CLAUDE]) {
    await expect(knopfVon(anna.page, kennung)).toHaveAttribute('aria-haspopup', 'menu');
    await expect(knopfVon(anna.page, kennung)).toHaveAttribute('aria-expanded', 'false');
  }

  await knopfVon(anna.page, BEN).focus();
  await anna.page.keyboard.press('ArrowDown');
  await expect(knopfVon(anna.page, CLAUDE)).toBeFocused();
  await anna.page.keyboard.press('Home');
  await expect(knopfVon(anna.page, BEN)).toBeFocused();
  await anna.page.keyboard.press('End');
  await expect(knopfVon(anna.page, CLAUDE)).toBeFocused();
  expect(await anna.page.evaluate(tabStoppsIn, '#lc-teilnehmer')).toEqual([CLAUDE]);
  await anna.page.keyboard.press('ArrowUp');
  await expect(knopfVon(anna.page, BEN)).toBeFocused();

  await anna.page.keyboard.press('Enter');
  await expect(menue(anna.page)).toBeVisible();
  await expect(direktnachricht(anna.page)).toBeFocused();
  await expect(knopfVon(anna.page, BEN)).toHaveAttribute('aria-expanded', 'true');
  await anna.page.keyboard.press('ArrowDown');
  await expect(direktnachricht(anna.page)).toBeFocused();
  await anna.page.keyboard.press('Escape');
  await expect(menue(anna.page)).toHaveCount(0);
  await expect(knopfVon(anna.page, BEN)).toBeFocused();
  await expect(knopfVon(anna.page, BEN)).toHaveAttribute('aria-expanded', 'false');

  await anna.page.keyboard.press(' ');
  await expect(direktnachricht(anna.page)).toBeFocused();
  await anna.page.keyboard.press('Tab');
  await expect(menue(anna.page)).toHaveCount(0);
  const imListe = await region(anna.page).evaluate((liste) => liste.contains(document.activeElement));
  expect(imListe, 'Tab verlässt die Liste').toBe(false);
  await anna.page.keyboard.press('Shift+Tab');
  await expect(knopfVon(anna.page, BEN)).toBeFocused();

  await anna.page.keyboard.press('Enter');
  await anna.page.keyboard.press('Enter');
  await expect(anna.page).toHaveURL(new RegExp(`#/d/${encodeURIComponent(BEN)}$`));
  await gespraechOffen(anna.page, BEN);
  await expect(eingabeAn(anna.page, BEN)).toBeFocused();
  await expect(menue(anna.page)).toHaveCount(0);
  await expect(anna.page.getByRole('textbox')).toHaveCount(1);
  expect(anna.fehler).toEqual([]);
});

test('Maus (B0785, US-20): Klick auf den Agenten öffnet das Menü, die eigene Zeile nicht; Auswahl öffnet #/d/claude…', async ({ browser }) => {
  const { anna } = await dreiTeilnehmer(browser);
  await listeOffen(anna.page);

  await zeileVon(anna.page, 'anna@127.0.0.1').click();
  await expect(menue(anna.page)).toHaveCount(0);
  await knopfVon(anna.page, CLAUDE).click();
  await direktnachricht(anna.page).click();

  await expect(anna.page).toHaveURL(new RegExp(`#/d/${encodeURIComponent(CLAUDE)}$`));
  await gespraechOffen(anna.page, CLAUDE);
  await expect(eingabeAn(anna.page, CLAUDE)).toBeFocused();
  await expect(region(anna.page)).toBeVisible();
  expect(anna.fehler).toEqual([]);
});

test('Telefon (B0785, V5, V6 d): Touch-Ziele ≥ 44 px; die Auswahl schließt das Panel, Verlauf sichtbar, Fokus in der Eingabe', async ({ browser }) => {
  const { anna } = await dreiTeilnehmer(browser, { geraet: GERAETE.telefon });
  await listeOffen(anna.page);

  expect((await knopfVon(anna.page, BEN).boundingBox()).height).toBeGreaterThanOrEqual(44);
  await knopfVon(anna.page, BEN).tap();
  expect((await direktnachricht(anna.page).boundingBox()).height).toBeGreaterThanOrEqual(44);
  await direktnachricht(anna.page).tap();

  await gespraechOffen(anna.page, BEN);
  await expect(region(anna.page)).toBeHidden();
  await expect(eingabeAn(anna.page, BEN)).toBeFocused();
  expect(anna.fehler).toEqual([]);
});

test('Feiner Zeiger (B0785, V5): Zeilenknopf und Menüzeile bleiben 28 px hoch', async ({ browser }) => {
  const { anna } = await dreiTeilnehmer(browser);
  await listeOffen(anna.page);

  expect((await knopfVon(anna.page, BEN).boundingBox()).height).toBe(28);
  await knopfVon(anna.page, BEN).click();
  expect((await direktnachricht(anna.page).boundingBox()).height).toBe(28);
  expect(anna.fehler).toEqual([]);
});

test('Sekundentakt (B0785, V6 a, e): Fokus und Menü an der Zeile „weg in …“ bleiben 3 s; geht die Zeile, schließt das Menü ohne Fehler', async ({ browser }) => {
  test.setTimeout(45000);
  const { anna, claudeGeht } = await dreiTeilnehmer(browser, { umgebung: { LOCALCHATA_GNADENFRIST_MS: '8000' } });
  await listeOffen(anna.page);
  claudeGeht();
  await expect(zeileVon(anna.page, CLAUDE).locator('.lc-status')).toHaveText(/^weg in \d:\d\d$/, { timeout: 5000 });

  await knopfVon(anna.page, CLAUDE).focus();
  const fokussiert = await anna.page.evaluateHandle(() => document.activeElement);
  await warten(3000);
  expect(await fokussiert.evaluate((element) => element === document.activeElement && element.isConnected)).toBe(true);

  await anna.page.keyboard.press('Enter');
  await expect(direktnachricht(anna.page)).toBeFocused();
  await warten(2000);
  await expect(direktnachricht(anna.page)).toBeFocused();

  await expect(zeileVon(anna.page, CLAUDE)).toHaveCount(0, { timeout: 10000 });
  await expect(menue(anna.page)).toHaveCount(0);
  expect(anna.fehler).toEqual([]);
});

// Code-Review #13 F0178 S4 (Abnahme-Probe wörtlich): ein neuer Teilnehmer zeichnet die Liste ganz neu — der Fokus auf
// dem Knopf von ben bleibt, genau ein Tab-Stopp; ein offenes Menü bleibt offen an seiner Zeile, der Fokus im Menü, und
// Escape führt auf den Knopf von ben zurück.
test('Volles Neuzeichnen (S4): dora tritt bei → Fokus auf ben bleibt; emil tritt bei offenem Menü bei → Menü bleibt', async ({ browser }) => {
  const { anna } = await dreiTeilnehmer(browser);
  await listeOffen(anna.page);
  await knopfVon(anna.page, BEN).focus();

  await beitreten(instanz.adresse, 'dora', { art: 'mensch', herkunft: '127.0.0.4' });
  await expect(zeileVon(anna.page, 'dora@127.0.0.4')).toHaveCount(1);
  await expect(knopfVon(anna.page, BEN)).toBeFocused();
  expect(await anna.page.evaluate(tabStoppsIn, '#lc-teilnehmer')).toEqual([BEN]);

  await anna.page.keyboard.press('Enter');
  await expect(direktnachricht(anna.page)).toBeFocused();
  await beitreten(instanz.adresse, 'emil', { art: 'mensch', herkunft: '127.0.0.5' });
  await expect(zeileVon(anna.page, 'emil@127.0.0.5')).toHaveCount(1);

  await expect(menue(anna.page)).toBeVisible();
  await expect(knopfVon(anna.page, BEN)).toHaveAttribute('aria-expanded', 'true');
  const menueOben = (await menue(anna.page).boundingBox()).y;
  const zeileUnten = await zeileVon(anna.page, BEN).boundingBox().then((kasten) => kasten.y + kasten.height);
  expect(Math.abs(menueOben - zeileUnten)).toBeLessThanOrEqual(3);
  await expect(direktnachricht(anna.page)).toBeFocused();
  await anna.page.keyboard.press('Escape');
  await expect(menue(anna.page)).toHaveCount(0);
  await expect(knopfVon(anna.page, BEN)).toBeFocused();
  expect(anna.fehler).toEqual([]);
});

// ux-reviewer #13 F0178 Optional 3 (Bestand, mit F0180): der Schalter „Teilnehmer n“ ist mit grobem Zeiger ein Ziel von
// mindestens 44 × 44 px — auf dem Telefon und dem Tablet; mit feinem Zeiger bleibt er 28 px hoch.
for (const geraet of ['telefonSchmal', 'telefon', 'tablet', 'desktop']) {
  test(`Schalter „Teilnehmer“ (Optional 3) — ${geraet}: Touch ≥ 44 × 44 px, fein 28 px`, async ({ browser }) => {
    const { anna } = await dreiTeilnehmer(browser, { geraet: GERAETE[geraet] });
    const kasten = await anna.page.getByRole('button', { name: /^Teilnehmer/ }).boundingBox();
    if (geraet === 'desktop') {
      expect(kasten.height).toBe(28);
    } else {
      expect(kasten.height).toBeGreaterThanOrEqual(44);
      expect(kasten.width).toBeGreaterThanOrEqual(44);
    }
    expect(await anna.page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(anna.fehler).toEqual([]);
  });
}
