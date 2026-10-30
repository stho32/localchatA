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

test('Schalter „⋯“ (B0628, UX M6): an drei gruppierten Einzeilern je ≥ 44 × 44 px und an Mitte und Ecken selbst getroffen; Menüpunkte ≥ 44 × 44 px mit 8 px Abstand; höchstens ein Menü offen; an fremden nur Kopieren', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await geraetOeffnen(browser, 'telefon', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeiten = [];
  for (const text of ['Eins', 'Zwei', 'Drei']) {
    zeiten.push(await imBrowserSenden(a.page, text));
  }
  const beta = await betaSendet(instanz.adresse, ['Fremd']);
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0])).toHaveCount(1);
  await expect(schalter(a.page, beta.zeiten[0])).toHaveCount(1);
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0]).locator('.lc-aktion')).toHaveCount(1);
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0]).getByRole('button', { name: /Bearbeiten|Löschen/ })).toHaveCount(0);
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

/// Öffnet auf dem Touch-Gerät das Feld an `zeit` über „Aktionen“ → „Bearbeiten“.
async function feldPerMenue(page, zeit) {
  await schalter(page, zeit).tap();
  await imMenue(page, zeit, 'Bearbeiten').tap();
  await expect(feld(page)).toBeFocused();
}

test('Feld am Telefon (B0629, UX M7): Schrift ≥ 16 px; bei 375 × 500 liegen Feld, „Abbrechen“ und „Speichern“ ganz im Fenster und werden selbst getroffen; ab 10 Zeilen scrollt das Feld', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 30 });
  const a = await geraetOeffnen(browser, 'telefon', { fenster: { width: 375, height: 500 }, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldPerMenue(a.page, zeit);
  expect(await feld(a.page).evaluate((textfeld) => parseFloat(getComputedStyle(textfeld).fontSize))).toBeGreaterThanOrEqual(16);
  const knoepfe = [nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Abbrechen' }), nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' })];
  for (const ziel of [feld(a.page), ...knoepfe]) {
    const lage = await ziel.evaluate((element) => {
      const rand = element.getBoundingClientRect();
      const mitte = document.elementFromPoint(rand.left + rand.width / 2, rand.top + rand.height / 2);
      return {
        ganz: rand.top >= 0 && rand.left >= 0 && rand.bottom <= window.innerHeight && rand.right <= window.innerWidth,
        selbst: mitte !== null && element.contains(mitte),
        hoehe: rand.height,
      };
    });
    expect(lage.ganz).toBe(true);
    expect(lage.selbst).toBe(true);
  }
  for (const ziel of knoepfe) {
    expect((await ziel.boundingBox()).height).toBeGreaterThanOrEqual(44);
  }

  await feld(a.page).fill(Array.from({ length: 10 }, (_, zeile) => `Zeile ${zeile + 1}`).join('\n'));
  const hoeheBei10 = await feld(a.page).evaluate((textfeld) => textfeld.getBoundingClientRect().height);
  await feld(a.page).fill(Array.from({ length: 15 }, (_, zeile) => `Zeile ${zeile + 1}`).join('\n'));
  const bei15 = await feld(a.page).evaluate((textfeld) => ({ hoehe: textfeld.getBoundingClientRect().height, scrollt: textfeld.scrollHeight > textfeld.clientHeight }));
  expect(bei15.hoehe).toBe(hoeheBei10);
  expect(bei15.scrollt).toBe(true);
  expect(a.fehler).toEqual([]);
});

for (const { geraet, fenster } of [
  { geraet: 'telefon' }, { geraet: 'telefon', fenster: { width: 320, height: 640 } }, { geraet: 'tablet' }, { geraet: 'desktop' },
]) {
  const titel = fenster ? `${geraet} bei 320 px` : geraet;
  test(`Zielfläche und Schmal (B0629, ${titel}): mit offenem Feld kein Querscrollen; Knöpfe groß genug`, async ({ browser }) => {
    instanz = await sendenStarten();
    const a = await geraetOeffnen(browser, geraet, { fenster, streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const zeit = await imBrowserSenden(a.page, 'Hallo mit einem etwas längeren Text, der umbrechen muss, damit es eng wird');
    const mindestens = geraet === 'desktop' ? 24 : 44;
    if (geraet === 'desktop') {
      await nachrichtMitZeit(a.page, zeit).hover();
      await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Bearbeiten' }).click();
    } else {
      await feldPerMenue(a.page, zeit);
    }
    for (const name of ['Abbrechen', 'Speichern']) {
      const box = await nachrichtMitZeit(a.page, zeit).getByRole('button', { name }).boundingBox();
      expect(box.width).toBeGreaterThanOrEqual(mindestens);
      expect(box.height).toBeGreaterThanOrEqual(mindestens);
    }
    expect(await a.page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(a.fehler).toEqual([]);
  });
}
