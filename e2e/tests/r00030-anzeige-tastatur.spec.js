'use strict';
// R00030 F0102 „Tastatur und Bildschirmleser“, „Tab-Folge“ (UX V2; B0732) und „Tasten gehören dem Element im Fokus“
// (UX V2; B0752), Desktop 1280 × 800, Gegeben über die API. Ein Video ist ab F0165 ein Player: seine inneren
// Bedienelemente melden das Element selbst als Fokus — sie zählen als ein Stopp „Player“.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, bearbeitenPerApi, zeigtBearbeitet } = require('./r00041-hilfen');
const { sichtbarNach } = require('./hilfen');
const { eingabe } = require('./senden-hilfen');
const { anzeigeStarten } = require('./r00030-anzeige-hilfen');
const { aeltereNachrichten } = require('./r00030-player-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Was den Fokus hat, in Worten: Zitat, Eingabe, sonst der zugängliche Name bzw. Text des Elements.
function imFokus(page) {
  return page.evaluate(() => {
    const element = document.activeElement;
    if (element.classList.contains('lc-zitat')) {
      return 'Zitat';
    }
    if (element.matches('.lc-comp textarea')) {
      return 'Eingabe';
    }
    if (element.classList.contains('lc-msg')) {
      return `Nachricht ${element.dataset.zeit}`;
    }
    return element.getAttribute('aria-label') || element.textContent.trim();
  });
}

/// Die Zeit der Rollfokus-Nachricht (tabindex 0).
const rollfokus = (page) => page.getByRole('log').locator('.lc-msg[tabindex="0"]').getAttribute('data-zeit');

/// Die Zeit der Nachricht, in deren Anhängen der Fokus liegt, sonst null.
const fokusInAnhaengenVon = (page) => page.evaluate(() => {
  if (document.activeElement.closest('.lc-anhaenge') === null) {
    return null;
  }
  return document.activeElement.closest('.lc-msg').dataset.zeit;
});

test('Tastatur und Bildschirmleser (B0732): fokussierte Nachricht mit foto.png und bericht.pdf → Tab durch Vorschau, Öffnen, Laden in Sendefolge, Aktionen, Eingabe; an anderen Nachrichten tabindex -1', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png', 'bericht.pdf'] }, { namen: ['foto.png', 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const fokussiert = nachrichtMitZeit(page, lauf.zeiten[1]);
  await fokussiert.focus();
  const folge = [];
  for (let schritt = 0; schritt < 7; schritt += 1) {
    await page.keyboard.press('Tab');
    folge.push(await imFokus(page));
  }
  expect(folge).toEqual([
    'foto.png groß anzeigen', 'foto.png öffnen', 'foto.png laden', 'bericht.pdf öffnen', 'bericht.pdf laden', 'Antworten', 'Eingabe',
  ]);
  const andere = nachrichtMitZeit(page, lauf.zeiten[0]);
  for (const bedienelement of await andere.locator('.lc-anhaenge button, .lc-anhaenge a').all()) {
    await expect(bedienelement).toHaveAttribute('tabindex', '-1');
  }
  for (const bedienelement of await fokussiert.locator('.lc-anhaenge button, .lc-anhaenge a').all()) {
    await expect(bedienelement).toHaveAttribute('tabindex', '0');
  }
  await expect(page.getByRole('log').locator('.lc-msg[tabindex="0"]')).toHaveCount(1);
  expect(lauf.a.fehler).toEqual([]);
});

test('Tab-Folge (B0732, V2): Zitat → „bearbeitet“ → Leiste → foto.png (groß anzeigen, öffnen, laden) → Player film.webm → film.webm laden → film.ogv laden → Aktionen → Eingabe; Shift Tab rückwärts; 30 × Shift Tab nie in fremden Anhängen', async ({ browser }) => {
  const lang = Array.from({ length: 41 }, (_, nummer) => `Zeile ${nummer + 1}`).join('\n');
  const lauf = await anzeigeStarten(browser, [
    { text: 'Frage?', namen: ['foto.png', 'bericht.pdf'] },
    { text: lang, namen: ['foto.png', 'film.webm', 'film.ogv'], antwortAuf: 0 },
  ]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const zeit = lauf.zeiten[1];
  const geaendert = await bearbeitenPerApi(instanz.adresse, lauf.beta.token, zeit, `${lang}\nneu`);
  expect(geaendert.status, JSON.stringify(geaendert.daten)).toBe(200);
  await sichtbarNach(page, zeigtBearbeitet, { argument: { zeit, text: `${lang}\nneu` }, ab: geaendert.empfangen, timeout: 10000 });
  const knoten = nachrichtMitZeit(page, zeit);
  await expect(knoten.locator('.lc-foldbar button')).toHaveCount(1);
  await knoten.focus();
  const erwartet = [
    'Zitat', 'bearbeitet', await knoten.locator('.lc-foldbar button').textContent(),
    'foto.png groß anzeigen', 'foto.png öffnen', 'foto.png laden', 'film.webm', 'film.webm laden', 'film.ogv laden', 'Antworten', 'Kopieren', 'Eingabe',
  ];
  const vorwaerts = await stoppsBis(page, 'Tab', erwartet.length);
  expect(vorwaerts).toEqual(erwartet);
  // Zurück aus der Eingabe liegt die Leiste der Aktionen nicht im Weg: sie steht nur, solange die Nachricht den Fokus
  // hat (R00041/R00046, r00041-fokussieren.spec.js) — rückwärts folgen die Anhänge direkt auf die Eingabe.
  const ohneAktionen = erwartet.filter((stopp) => !['Antworten', 'Kopieren'].includes(stopp));
  const rueckwaerts = await stoppsBis(page, 'Shift+Tab', ohneAktionen.length);
  expect(rueckwaerts).toEqual([...ohneAktionen.slice(0, -1).reverse(), `Nachricht ${zeit}`]);

  await eingabe(page).focus();
  for (let schritt = 0; schritt < 30; schritt += 1) {
    const stoppVorher = await rollfokus(page);
    await page.keyboard.press('Shift+Tab');
    const inAnhaengen = await fokusInAnhaengenVon(page);
    if (inAnhaengen !== null) {
      expect(inAnhaengen, `Schritt ${schritt + 1}: ${await imFokus(page)}`).toBe(stoppVorher);
    }
  }
  expect(lauf.a.fehler).toEqual([]);
});

/// Drückt `taste`, bis `anzahl` verschiedene Stopps nacheinander erreicht sind (höchstens 40 Mal); aufeinander folgende
/// gleiche Stopps (die Bedienelemente eines Players) zählen einmal. Wer im Player hängen bleibt, liefert zu wenige.
async function stoppsBis(page, taste, anzahl) {
  const stopps = [];
  for (let druck = 0; druck < 40 && stopps.length < anzahl; druck += 1) {
    await page.keyboard.press(taste);
    const stopp = await imFokus(page);
    if (stopps[stopps.length - 1] !== stopp) {
      stopps.push(stopp);
    }
  }
  return stopps;
}

/// Merkt sich je keydown, ob es nach dem Verlauf noch unverhindert war (Lauscher am Fenster, nach dem Verlauf).
const verhinderteMitschreiben = (page) => page.evaluate(() => {
  window.verhindert = [];
  window.addEventListener('keydown', (ereignis) => window.verhindert.push(`${ereignis.key}:${ereignis.defaultPrevented}`));
});

test('Tasten gehören dem Element im Fokus (B0752, V2): auf „foto.png öffnen“, „foto.png groß anzeigen“ und im Bild-Dialog nimmt der Verlauf Ende, Pos1, Leertaste und Pfeile nicht — kein preventDefault, der Fokus bleibt am Element', async ({ browser }) => {
  // Der Verlauf ist höher als das Fenster (60 ältere Nachrichten, code-reviewer #43 F0102 S5): nähme er Ende, öffnete er
  // neu oder holte den Fokus. Auf Link und Vorschau darf der Browser selbst scrollen (ux-reviewer #43 F0102, Antwort 3);
  // geprüft wird, dass der Verlauf die Taste nicht nimmt. Im Player wirken die Tasten am Element (r00030-player.spec.js).
  const lauf = await anzeigeStarten(browser, [{ text: 'Bild', namen: ['foto.png'] }], { vorbereiten: aeltereNachrichten(60) });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const log = page.getByRole('log');
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  await knoten.focus();
  await verhinderteMitschreiben(page);
  const ziele = [
    knoten.getByRole('link', { name: 'foto.png öffnen', exact: true }),
    knoten.getByRole('button', { name: 'foto.png groß anzeigen', exact: true }),
  ];
  for (const ziel of ziele) {
    for (const taste of ['End', 'Home', 'Space', 'ArrowDown', 'ArrowUp']) {
      await ziel.focus();
      const name = await ziel.getAttribute('aria-label');
      await ziel.evaluate((element) => { window.zielImFokus = element; });
      await page.keyboard.press(taste);
      await page.waitForTimeout(100);
      expect(await page.evaluate(() => window.verhindert.at(-1)), `${taste} auf ${name}`).toMatch(/:false$/);
      if (taste !== 'Space') {
        expect(await page.evaluate(() => window.zielImFokus.isConnected && document.activeElement === window.zielImFokus), `${taste} auf ${name}`).toBe(true);
      }
      if (await page.locator('dialog[open]').count() > 0) {
        await page.keyboard.press('Escape');
      }
    }
  }
  await ziele[1].click();
  const schliessen = page.getByRole('dialog', { name: 'foto.png' }).getByRole('button', { name: 'Schließen', exact: true });
  await expect(schliessen).toBeFocused();
  const stand = await log.evaluate((element) => element.scrollTop);
  for (const taste of ['End', 'Home', 'ArrowDown', 'ArrowUp']) {
    await page.keyboard.press(taste);
    expect(await log.evaluate((element) => element.scrollTop), `${taste} im Dialog`).toBe(stand);
    expect(await page.evaluate(() => window.verhindert.at(-1)), `${taste} im Dialog`).toMatch(/:false$/);
  }
  await expect(schliessen).toBeFocused();
  expect(lauf.a.fehler).toEqual([]);
});
