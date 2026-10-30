'use strict';
// R00041 F0135 auf Touch-Geräten: Menü „⋯ Aktionen“ (B0628, UX M6, PO 125) und das Feld am Telefon (B0629, UX M7);
// Geräte wie R00027 (Telefon 375 × 812, Tablet 768 × 1024, je mit Touch), nur durch Tippen bedient.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const { betaSendet, imBrowserSenden, nachrichtMitZeit } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const schalter = (page, zeit) => nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Aktionen', exact: true });
const imMenue = (page, zeit, name) => nachrichtMitZeit(page, zeit).getByRole('button', { name, exact: true });
const feld = (page) => page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' });

function bearbeitungenZaehlen(page) {
  const liste = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten/bearbeiten')) {
      liste.push(anfrage.postDataJSON());
    }
  });
  return liste;
}

for (const geraet of ['telefon', 'tablet']) {
  test(`${geraet} (B0628): „Aktionen“ öffnet das Menü mit „Bearbeiten“ und „Löschen“; „Bearbeiten“ → Feld; Enter macht eine Zeile; „Speichern“ speichert`, async ({ browser }) => {
    instanz = await sendenStarten();
    const a = await geraetOeffnen(browser, geraet, { streng: true });
    const bearbeitungen = bearbeitungenZaehlen(a.page);
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const zeit = await imBrowserSenden(a.page, 'Hallo');
    await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'false');
    await expect(imMenue(a.page, zeit, 'Bearbeiten')).toBeHidden();

    await schalter(a.page, zeit).tap();
    await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'true');
    await expect(imMenue(a.page, zeit, 'Bearbeiten')).toBeVisible();
    await expect(imMenue(a.page, zeit, 'Löschen')).toBeVisible();
    await imMenue(a.page, zeit, 'Bearbeiten').tap();
    await expect(feld(a.page)).toBeFocused();
    await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'false');
    await a.page.keyboard.press('Enter');
    await a.page.keyboard.type('Welt');
    await expect(feld(a.page)).toHaveValue('Hallo\nWelt');
    expect(bearbeitungen).toEqual([]);
    await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' }).tap();
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo\nWelt');
    expect(bearbeitungen).toHaveLength(1);
    expect(a.fehler).toEqual([]);
  });
}

test('Schalter „⋯“ (B0628, UX M6): an drei gruppierten Einzeilern je ≥ 44 × 44 px und an Mitte und Ecken selbst getroffen; Menüpunkte ≥ 44 × 44 px mit 8 px Abstand; höchstens ein Menü offen; an fremden keiner', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await geraetOeffnen(browser, 'telefon', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeiten = [];
  for (const text of ['Eins', 'Zwei', 'Drei']) {
    zeiten.push(await imBrowserSenden(a.page, text));
  }
  const beta = await betaSendet(instanz.adresse, ['Fremd']);
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0])).toHaveCount(1);
  await expect(schalter(a.page, beta.zeiten[0])).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeiten[1])).not.toHaveClass(/\bfirst\b/);

  for (const zeit of zeiten) {
    const treffer = await schalter(a.page, zeit).evaluate((knopf) => {
      const rand = knopf.getBoundingClientRect();
      const punkte = [
        [rand.left + rand.width / 2, rand.top + rand.height / 2], [rand.left + 1, rand.top + 1], [rand.right - 1, rand.top + 1],
        [rand.left + 1, rand.bottom - 1], [rand.right - 1, rand.bottom - 1],
      ];
      return {
        breite: rand.width,
        hoehe: rand.height,
        getroffen: punkte.map(([x, y]) => { const ziel = document.elementFromPoint(x, y); return ziel !== null && knopf.contains(ziel); }),
      };
    });
    expect(treffer.breite).toBeGreaterThanOrEqual(44);
    expect(treffer.hoehe).toBeGreaterThanOrEqual(44);
    expect(treffer.getroffen).toEqual([true, true, true, true, true]);
  }

  await schalter(a.page, zeiten[0]).tap();
  const bearbeiten = await imMenue(a.page, zeiten[0], 'Bearbeiten').boundingBox();
  const loeschen = await imMenue(a.page, zeiten[0], 'Löschen').boundingBox();
  for (const box of [bearbeiten, loeschen]) {
    expect(box.width).toBeGreaterThanOrEqual(44);
    expect(box.height).toBeGreaterThanOrEqual(44);
  }
  const abstand = Math.max(loeschen.y - (bearbeiten.y + bearbeiten.height), loeschen.x - (bearbeiten.x + bearbeiten.width));
  expect(abstand).toBeGreaterThanOrEqual(8);

  await schalter(a.page, zeiten[1]).tap();
  await expect(a.page.getByRole('log').locator('[aria-expanded="true"]')).toHaveCount(1);
  await expect(schalter(a.page, zeiten[1])).toHaveAttribute('aria-expanded', 'true');
  await schalter(a.page, zeiten[1]).tap();
  await expect(a.page.getByRole('log').locator('.lc-aktionen-schalter[aria-expanded="true"]')).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});
