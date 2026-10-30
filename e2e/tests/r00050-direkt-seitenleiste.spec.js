'use strict';
// R00050 F0180 B0786 (Karte #13, V8, V5, V7, V13, US-25): die Seitenleiste zeigt nach „Channels“ den Abschnitt
// „Direktnachrichten“ (h2, ohne „+“), immer da; ohne Gespräch die zwei Sätze des Leerzustands; je Gespräch ein Link
// `#/d/<kennung>` mit Statuspunkt, voller Kennung und offline/„gleich weg“/weg als Wort, alphabetisch nach Kennung (nie
// nach Aktivität), aria-current beim offenen, Name „Direktnachricht mit <kennung>, <status>“; geladen aus GET
// /api/direkt, neue Einträge live aus dem Strom, der Fokus bleibt beim Einfügen; auf dem Telefon Statuspunkt und
// Kürzel, ≥ 44 px hoch, bei feinem Zeiger 28 px; kein Zähler, kein @n, kein Marker. Mutanten, die rot werden müssen:
// Sortierung nach Aktivität; fremde Gespräche. Gegen das echte Programm.
const { test, expect } = require('./rahmen');
const { stromOeffnen } = require('./hilfen');
const { beitreten } = require('./senden-hilfen');
const { GERAETE } = require('./r00027-hilfen');
const {
  ANNA, BEN, CORA, direktStarten, direktSenden, gespraechOffen, kontrastUnterschritten,
} = require('./r00050-direkt-hilfen');

let instanz;
let stroeme = [];
test.afterEach(async () => {
  stroeme.forEach((strom) => strom.schliessen());
  stroeme = [];
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const DORA = 'dora@127.0.0.4';
const ANTON = 'anton@127.0.0.6';
const channels = (page) => page.getByRole('navigation', { name: 'Channels' });
const direkt = (page) => page.getByRole('navigation', { name: 'Direktnachrichten' });
const eintraege = (page) => direkt(page).getByRole('link');
const eintragVon = (page, kennung) => direkt(page).locator(`a[data-kennung="${kennung}"]`);

async function starten(browser, optionen) {
  const gestartet = await direktStarten(browser, optionen);
  instanz = gestartet.instanz;
  stroeme.push(await stromOeffnen(instanz.adresse, gestartet.token.ben, '127.0.0.3'));
  return gestartet;
}

async function dazu(name, herkunft) {
  return beitreten(instanz.adresse, name, { art: 'mensch', herkunft });
}

test('Leerzustand und Einträge (V13, V8): zwei Sätze ohne Gespräch, danach ben und cora alphabetisch — live, nach dem Neuladen gleich', async ({ browser }) => {
  const { anna, token } = await starten(browser);
  await expect(direkt(anna.page).getByRole('heading', { level: 2 })).toHaveText('Direktnachrichten');
  await expect(direkt(anna.page).locator('.lc-side-leer')).toHaveText(['Noch keine Direktnachrichten.', 'Ein Gespräch beginnt in der Teilnehmerliste.']);
  await expect(direkt(anna.page).getByRole('button')).toHaveCount(0);
  const reihenfolge = await anna.page.evaluate(() => {
    const [erste, zweite] = [...document.querySelectorAll('nav')];
    return [erste.getAttribute('aria-label'), zweite.getAttribute('aria-label'), erste.compareDocumentPosition(zweite) & Node.DOCUMENT_POSITION_FOLLOWING];
  });
  expect(reihenfolge).toEqual(['Channels', 'Direktnachrichten', 4]);

  expect((await direktSenden(instanz.adresse, token.cora, ANNA, 'zuerst cora', '127.0.0.9')).status).toBe(201);
  expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'dann ben', '127.0.0.3')).status).toBe(201);
  expect((await direktSenden(instanz.adresse, token.ben, CORA, 'fremd', '127.0.0.3')).status).toBe(201);

  await expect(eintraege(anna.page)).toHaveCount(2);
  await expect(eintraege(anna.page).locator('.lc-id')).toHaveText([BEN, CORA]);
  await expect(eintragVon(anna.page, BEN)).toHaveAttribute('aria-label', `Direktnachricht mit ${BEN}, online`);
  await expect(eintragVon(anna.page, BEN)).toHaveAttribute('href', `#/d/${encodeURIComponent(BEN)}`);
  await expect(eintragVon(anna.page, CORA)).toHaveAttribute('aria-label', new RegExp(`^Direktnachricht mit ${CORA}, (online|offline)$`));
  await expect(direkt(anna.page).locator('.lc-side-leer')).toHaveCount(0);
  // Kein Zähler, kein @n, kein Marker: jeder Eintrag besteht genau aus Punkt, Kennung, Kürzel und höchstens dem Statuswort.
  await expect(eintraege(anna.page)).toHaveText([/^ben@127\.0\.0\.3be$/, /^cora@127\.0\.0\.9co(offline|gleich weg)?$/]);

  await anna.page.reload();
  await expect(eintraege(anna.page).locator('.lc-id')).toHaveText([BEN, CORA]);
  expect(anna.fehler).toEqual([]);
});

test('Auswahl und Fokus (V8): Klick öffnet #/d/<kennung> mit aria-current; ein neuer Eintrag davor nimmt den Fokus nicht', async ({ browser }) => {
  const { anna, token } = await starten(browser);
  expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'hallo', '127.0.0.3')).status).toBe(201);
  await eintragVon(anna.page, BEN).click();
  await gespraechOffen(anna.page, BEN);
  await expect(eintragVon(anna.page, BEN)).toHaveAttribute('aria-current', 'page');
  await expect(channels(anna.page).locator('[aria-current]')).toHaveCount(0);

  await eintragVon(anna.page, BEN).focus();
  const anton = await dazu('anton', '127.0.0.6');
  expect((await direktSenden(instanz.adresse, anton, ANNA, 'ich bin neu', '127.0.0.6')).status).toBe(201);
  await expect(eintraege(anna.page).locator('.lc-id')).toHaveText([ANTON, BEN]);
  await expect(eintragVon(anna.page, BEN)).toBeFocused();

  await channels(anna.page).getByRole('link', { name: 'technik', exact: true }).focus();
  const dora = await dazu('dora', '127.0.0.4');
  expect((await direktSenden(instanz.adresse, dora, ANNA, 'und ich', '127.0.0.4')).status).toBe(201);
  await expect(eintraege(anna.page)).toHaveCount(3);
  await expect(channels(anna.page).getByRole('link', { name: 'technik', exact: true })).toBeFocused();
  expect(anna.fehler).toEqual([]);
});

test('Statuswort (V8): offline und weg stehen als Wort am Eintrag, online nicht', async ({ browser }) => {
  const { anna, token } = await starten(browser);
  expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'hallo', '127.0.0.3')).status).toBe(201);
  await expect(eintragVon(anna.page, BEN).locator('.lc-status')).toHaveCount(0);
  stroeme.forEach((strom) => strom.schliessen());
  stroeme = [];
  await expect(eintragVon(anna.page, BEN)).toHaveAttribute('aria-label', new RegExp(`^Direktnachricht mit ${BEN}, offline`), { timeout: 15000 });
  await expect(eintragVon(anna.page, BEN).locator('.lc-status')).toHaveText(/^(offline|gleich weg)$/);
  expect(anna.fehler).toEqual([]);
});

// Nacharbeit F0180 (code-reviewer #13, Sollte 2): ein fremder Beitritt ändert keinen Eintrag — der Abschnitt wird nicht
// angefasst (0 Mutationen, Marke und Fokus bleiben); eine echte Änderung (ben online → offline) zeichnet neu.
// Mutante: jedes Teilnehmer-Ereignis ersetzt alle Einträge → Mutationen > 0, Marke weg, rot.
test('Neu zeichnen nur bei Änderung (Nacharbeit): fremder Beitritt = 0 Mutationen, Marke und Fokus bleiben; ben online → offline zeichnet neu', async ({ browser }) => {
  const { anna, token } = await starten(browser);
  expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'hallo', '127.0.0.3')).status).toBe(201);
  await expect(eintragVon(anna.page, BEN)).toHaveAttribute('aria-label', `Direktnachricht mit ${BEN}, online`);
  await eintragVon(anna.page, BEN).focus();
  await eintragVon(anna.page, BEN).evaluate((knoten) => {
    knoten.dataset.marke = '1';
    window.mutationen = 0;
    new MutationObserver((liste) => {
      window.mutationen += liste.length;
    }).observe(document.querySelector('nav[aria-label="Direktnachrichten"]'), { childList: true, subtree: true, attributes: true, characterData: true });
  });

  await dazu('dora', '127.0.0.4');
  await expect(anna.page.getByText('dora', { exact: false }).first()).toBeVisible();
  await anna.page.waitForTimeout(1500);
  expect(await anna.page.evaluate(() => window.mutationen), 'Mutationen im Abschnitt nach fremdem Beitritt').toBe(0);
  expect(await anna.page.locator('nav[aria-label="Direktnachrichten"] [data-marke="1"]').count()).toBe(1);
  await expect(eintragVon(anna.page, BEN)).toBeFocused();

  stroeme.forEach((strom) => strom.schliessen());
  stroeme = [];
  await expect(eintragVon(anna.page, BEN)).toHaveAttribute('aria-label', new RegExp(`^Direktnachricht mit ${BEN}, offline`), { timeout: 15000 });
  await expect(eintragVon(anna.page, BEN).locator('.lc-status')).toHaveText(/^(offline|gleich weg)$/);
  expect(await anna.page.evaluate(() => window.mutationen)).toBeGreaterThan(0);
  expect(anna.fehler).toEqual([]);
});

test('Telefon 375 px mit Touch (V8, V5): drei Einträge mit Statuspunkt und Kürzel, ≥ 44 px hoch, voller Name', async ({ browser }) => {
  const { anna, token } = await starten(browser, { geraet: GERAETE.telefon });
  const dora = await dazu('dora', '127.0.0.4');
  for (const [tokenVon, herkunft] of [[token.ben, '127.0.0.3'], [token.cora, '127.0.0.9'], [dora, '127.0.0.4']]) {
    expect((await direktSenden(instanz.adresse, tokenVon, ANNA, 'hallo', herkunft)).status).toBe(201);
  }
  await expect(eintraege(anna.page)).toHaveCount(3);
  const gemessen = await eintraege(anna.page).evaluateAll((links) => links.map((link) => ({
    hoehe: link.getBoundingClientRect().height,
    breite: link.getBoundingClientRect().right,
    sichtbar: [...link.querySelectorAll('*')].filter((teil) => teil.getClientRects().length > 0 && teil.textContent.trim() !== '')
      .map((teil) => teil.textContent.trim()),
    punkt: link.querySelector('.lc-dot').getClientRects().length > 0,
    name: link.getAttribute('aria-label'),
  })));
  expect(gemessen.map(({ sichtbar }) => sichtbar)).toEqual([['be'], ['co'], ['do']]);
  for (const eintrag of gemessen) {
    expect(eintrag.hoehe).toBeGreaterThanOrEqual(44);
    expect(eintrag.breite).toBeLessThanOrEqual(48);
    expect(eintrag.punkt).toBe(true);
  }
  expect(gemessen.map(({ name }) => name.replace(/, .*$/, ''))).toEqual([BEN, CORA, DORA].map((kennung) => `Direktnachricht mit ${kennung}`));
  expect(await anna.page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  expect(anna.fehler).toEqual([]);
});

test('Feiner Zeiger (V5): Einträge 28 px hoch, Kennung in voller Länge', async ({ browser }) => {
  const { anna, token } = await starten(browser);
  expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'hallo', '127.0.0.3')).status).toBe(201);
  await expect(eintragVon(anna.page, BEN).locator('.lc-id')).toHaveText(BEN);
  await expect(eintragVon(anna.page, BEN).locator('.lc-id')).toBeVisible();
  expect((await eintragVon(anna.page, BEN).boundingBox()).height).toBe(28);
  expect(anna.fehler).toEqual([]);
});

for (const thema of ['light', 'dark']) {
  test(`Nicht nur Farbe (V7) — ${thema}: Kontrast unterschritten: [] in der Seitenleiste, Statuspunkt auch unter forced-colors`, async ({ browser }) => {
    const { anna, token } = await starten(browser);
    await anna.page.emulateMedia({ colorScheme: thema });
    expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'hallo', '127.0.0.3')).status).toBe(201);
    await expect(eintraege(anna.page)).toHaveCount(1);
    expect(await anna.page.evaluate(kontrastUnterschritten, '.lc-side')).toEqual([]);

    await anna.page.emulateMedia({ colorScheme: thema, forcedColors: 'active' });
    const punkt = await eintragVon(anna.page, BEN).locator('.lc-dot').evaluate((element) => {
      const flaecheUnter = (knoten) => {
        for (let vorfahr = knoten.parentElement; vorfahr !== null; vorfahr = vorfahr.parentElement) {
          const farbe = getComputedStyle(vorfahr).backgroundColor;
          if (farbe !== 'rgba(0, 0, 0, 0)') {
            return farbe;
          }
        }
        return 'rgb(255, 255, 255)';
      };
      return { punkt: getComputedStyle(element).backgroundColor, darunter: flaecheUnter(element), breite: element.getBoundingClientRect().width };
    });
    expect(punkt.breite).toBeGreaterThan(0);
    expect(punkt.punkt, 'der volle Punkt hebt sich unter forced-colors von der Fläche ab').not.toBe(punkt.darunter);
    expect(punkt.punkt).not.toBe('rgba(0, 0, 0, 0)');
    expect(anna.fehler).toEqual([]);
  });
}
