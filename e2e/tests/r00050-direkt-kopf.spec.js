'use strict';
// R00050 F0178 B0789 (Karte #13): der Kopf des Direktgesprächs — volle Kennung des Gegenübers in der Kopfzeile statt
// `# name ▾` (kein Schalter), „· 7 Tage“ ab 720 px, Tab-Titel „Direktnachricht mit <kennung> — localchatA“ (V3) und der
// Hinweis wörtlich als Streifen `p.lc-privat-hinweis` unter der Kopfzeile (V1), bei jeder Breite sichtbar (V2), nicht
// nur Farbe (V7). Mutanten, die rot werden müssen: Hinweis aria-hidden; Hinweis bei 320 px abgeschnitten; Wortlaut weicht
// ab; Wort „geheim“ im Hinweis. Gegen das echte Programm.
const { test, expect } = require('./rahmen');
const {
  BEN, HINWEIS, direktStarten, direktOeffnen, gespraechOffen, verlaufMit, eingabeAn, kontrastUnterschritten,
} = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function annaImGespraech(browser, kennung = BEN, breite = 1280, hoehe = 800) {
  const gestartet = await direktStarten(browser, { geraet: { viewport: { width: breite, height: hoehe } } });
  instanz = gestartet.instanz;
  await direktOeffnen(gestartet.anna.page, kennung);
  await gespraechOffen(gestartet.anna.page, kennung);
  return gestartet.anna;
}

const hinweis = (page) => page.locator('p.lc-privat-hinweis');
const ort = (page) => page.getByRole('banner').locator('.lc-head-ort');

/// Für den Hinweis: der Wortlaut, ob er oder ein Vorfahr aria-hidden ist, ob er quer überläuft, ob die Seite quer
/// scrollt und ob seine Mitte von ihm selbst getroffen wird.
function hinweisVermessen() {
  const element = document.querySelector('p.lc-privat-hinweis');
  const rechteck = element.getBoundingClientRect();
  const getroffen = document.elementFromPoint(rechteck.left + rechteck.width / 2, rechteck.top + rechteck.height / 2);
  return {
    text: element.textContent,
    versteckt: element.closest('[aria-hidden="true"]') !== null,
    ueberlaeuft: element.scrollWidth > element.clientWidth,
    seiteQuer: document.documentElement.scrollWidth > document.documentElement.clientWidth,
    getroffen: getroffen !== null && element.contains(getroffen),
  };
}

const SICHTBAR = { text: HINWEIS, versteckt: false, ueberlaeuft: false, seiteQuer: false, getroffen: true };

test('Hinweis und Kopf (B0789, V1, V3): Wortlaut wörtlich in einem p, volle Kennung ohne Schalter, · 7 Tage, Tab-Titel', async ({ browser }) => {
  const anna = await annaImGespraech(browser);

  await expect(hinweis(anna.page)).toHaveCount(1);
  expect(await anna.page.evaluate(hinweisVermessen)).toEqual(SICHTBAR);
  expect(HINWEIS).not.toMatch(/verschl|geheim|niemand sonst|sicher/i);
  expect(await hinweis(anna.page).textContent()).not.toMatch(/verschl|geheim|niemand sonst|sicher/i);
  await expect(ort(anna.page).locator('.lc-head-gegenueber')).toHaveText(BEN);
  await expect(ort(anna.page).locator('.lc-head-frist')).toHaveText('· 7 Tage');
  await expect(ort(anna.page).locator('.lc-head-frist')).toBeVisible();
  await expect(ort(anna.page).getByRole('button')).toHaveCount(0);
  await expect(anna.page).toHaveTitle(`Direktnachricht mit ${BEN} — localchatA`);

  await anna.page.evaluate(() => { window.location.hash = '#/c/allgemein'; });
  await expect(anna.page.getByRole('log', { name: 'Verlauf # allgemein' })).toBeVisible();
  await expect(hinweis(anna.page)).toBeHidden();
  await expect(ort(anna.page).locator('.lc-head-gegenueber')).toHaveCount(0);
  await expect(anna.page).toHaveTitle('# allgemein — localchatA');
  expect(anna.fehler).toEqual([]);
});

for (const [breite, hoehe] of [[320, 800], [375, 800], [414, 800], [768, 800], [375, 500]]) {
  test(`Bei jeder Breite sichtbar (B0789, V2) — ${breite} × ${hoehe}: Hinweis ganz zu sehen, nichts quer, Verlauf da`, async ({ browser }) => {
    const anna = await annaImGespraech(browser, BEN, breite, hoehe);

    expect(await anna.page.evaluate(hinweisVermessen)).toEqual(SICHTBAR);
    const verlaufHoehe = await verlaufMit(anna.page, BEN).evaluate((log) => log.getBoundingClientRect().height);
    expect(verlaufHoehe).toBeGreaterThanOrEqual(120);
    await expect(ort(anna.page).locator('.lc-head-gegenueber')).toHaveText(BEN);
    if (breite < 720) {
      await expect(ort(anna.page).locator('.lc-head-frist')).toBeHidden();
    }
    expect(anna.fehler).toEqual([]);
  });
}

test('Lange Kennung (B0789, V3, V4): die längste mögliche Kennung (72 Zeichen) macht die Kopfzeile bei 320 px höchstens 140 px hoch, ungekürzt', async ({ browser }) => {
  const lang = 'abcdefghijklmnopqrstuvwxyz012345@fd00:1234:5678:9abc:def0:1234:5678:9abc';
  expect(lang).toHaveLength(72);
  const anna = await annaImGespraech(browser, lang, 320, 800);

  const kopf = await anna.page.getByRole('banner').evaluate((element) => {
    const gegenueber = element.querySelector('.lc-head-gegenueber');
    return {
      hoehe: element.getBoundingClientRect().height,
      ganz: gegenueber.scrollWidth <= gegenueber.clientWidth && gegenueber.textContent === gegenueber.innerText.replace(/\n/g, ''),
    };
  });
  expect(kopf.hoehe).toBeLessThanOrEqual(140);
  expect(kopf.ganz).toBe(true);
  expect(await anna.page.evaluate(hinweisVermessen)).toEqual(SICHTBAR);
  const reihenfolge = await anna.page.evaluate(() => {
    const folgt = (vorn, hinten) => Boolean(vorn.compareDocumentPosition(hinten) & Node.DOCUMENT_POSITION_FOLLOWING);
    const [kopfzeile, streifen, log, feld] = ['header', 'p.lc-privat-hinweis', '[role="log"]', 'textarea'].map((wahl) => document.querySelector(wahl));
    return folgt(kopfzeile, streifen) && folgt(streifen, log) && folgt(log, feld);
  });
  expect(reihenfolge, 'Kopfzeile → Hinweis → Verlauf → Eingabe').toBe(true);
  expect(anna.fehler).toEqual([]);
});

for (const thema of ['light', 'dark']) {
  test(`Nicht nur Farbe (B0789, V7) — ${thema}: Kontraste in Kopf und Hinweis, unter forced-colors bleibt das Wort`, async ({ browser }) => {
    const anna = await annaImGespraech(browser);
    await anna.page.emulateMedia({ colorScheme: thema });

    expect(await anna.page.evaluate(kontrastUnterschritten, 'p.lc-privat-hinweis'), 'Kontrast unterschritten').toEqual([]);
    expect(await anna.page.evaluate(kontrastUnterschritten, '.lc-head'), 'Kontrast unterschritten').toEqual([]);
    await anna.page.emulateMedia({ colorScheme: thema, forcedColors: 'active' });
    expect(await anna.page.evaluate(hinweisVermessen)).toEqual(SICHTBAR);
    await expect(anna.page).toHaveTitle(/^Direktnachricht mit /);
    await expect(eingabeAn(anna.page, BEN)).toBeVisible();
    expect(anna.fehler).toEqual([]);
  });
}
