'use strict';
// R00030 F0101 Nacharbeit UX-Review #43 M1 (Probe wörtlich): Touch (hasTouch, isMobile) 320×568, 375×667, 768×1024 und
// Desktop 1280×800; Dateinamen 20, 30, 70, 120 Zeichen ohne Leerzeichen, einzeln und zusammen mit foto.png:
// (a) getBoundingClientRect().right von „Entfernen“ ≤ rechte Kante der Liste und ≥ 0, Größe sichtbar;
// (b) .lc-datei-name.scrollWidth > clientWidth und clientWidth ≥ 96 px bei langen Namen;
// (c) documentElement.scrollWidth ≤ innerWidth, .lc-dateien.scrollWidth ≤ clientWidth;
// (d) dasselbe im Eintrag „wird hochgeladen“.
// „Bei langen Namen“ heißt hier: der Name passt nicht ganz hinein (scrollWidth > clientWidth) — dann ist er
// mindestens 96 px breit; ein Name von 120 Zeichen passt auf Touch nie ganz.
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { anhangStarten, waehlen, fotoPng, textDatei } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const FENSTER = [
  { name: 'Touch 320×568', geraet: 'telefon', fenster: { width: 320, height: 568 }, touch: true },
  { name: 'Touch 375×667', geraet: 'telefon', fenster: { width: 375, height: 667 }, touch: true },
  { name: 'Touch 768×1024', geraet: 'tablet', fenster: { width: 768, height: 1024 }, touch: true },
  { name: 'Desktop 1280×800', geraet: 'desktop', fenster: { width: 1280, height: 800 }, touch: false },
];
const LAENGEN = [20, 30, 70, 120];

const langerName = (zeichen) => `${'b'.repeat(zeichen - 4)}.txt`;

/// Misst je Zeile der Liste `liste` (Selektor) Name, Größe und Schalter gegen die Liste und das Fenster.
function vermessen(page, liste, zeile) {
  return page.evaluate(({ liste: listenSelektor, zeile: zeilenSelektor }) => {
    const listeElement = document.querySelector(listenSelektor);
    const rand = listeElement.getBoundingClientRect();
    return {
      seiteBreit: document.documentElement.scrollWidth,
      fensterBreit: window.innerWidth,
      listeScroll: listeElement.scrollWidth,
      listeClient: listeElement.clientWidth,
      zeilen: [...listeElement.querySelectorAll(zeilenSelektor)].map((zeileElement) => {
        const name = zeileElement.querySelector('.lc-datei-name');
        const groesse = zeileElement.querySelector('.lc-datei-groesse').getBoundingClientRect();
        return {
          text: name.textContent,
          nameScroll: name.scrollWidth,
          nameClient: name.clientWidth,
          groesseRechts: groesse.right,
          groesseBreit: groesse.width,
          schalter: [...zeileElement.querySelectorAll('button')].map((schalter) => {
            const box = schalter.getBoundingClientRect();
            return { links: box.left, rechts: box.right };
          }),
        };
      }),
      links: rand.left,
      rechts: rand.right,
    };
  }, { liste, zeile });
}

function pruefen(mass, { touch, zeichen, wo }) {
  const ort = `${wo}, ${zeichen} Zeichen`;
  expect(mass.seiteBreit, `${ort}: Seite`).toBeLessThanOrEqual(mass.fensterBreit);
  expect(mass.listeScroll, `${ort}: Liste`).toBeLessThanOrEqual(mass.listeClient);
  for (const zeile of mass.zeilen) {
    expect(zeile.schalter.length, `${ort}: ${zeile.text} Schalter`).toBeGreaterThan(0);
    for (const schalter of zeile.schalter) {
      expect(schalter.rechts, `${ort}: ${zeile.text} Schalter rechts`).toBeLessThanOrEqual(mass.rechts + 0.5);
      expect(schalter.links, `${ort}: ${zeile.text} Schalter links`).toBeGreaterThanOrEqual(0);
    }
    expect(zeile.groesseBreit, `${ort}: ${zeile.text} Größe`).toBeGreaterThan(0);
    expect(zeile.groesseRechts, `${ort}: ${zeile.text} Größe rechts`).toBeLessThanOrEqual(mass.rechts + 0.5);
    const gekuerzt = zeile.nameScroll > zeile.nameClient;
    if (gekuerzt) {
      expect(zeile.nameClient, `${ort}: ${zeile.text} Namensbreite`).toBeGreaterThanOrEqual(96);
    }
    if (touch && zeile.text.length === 120) {
      expect(gekuerzt, `${ort}: 120 Zeichen gekürzt`).toBe(true);
    }
  }
}

for (const fenster of FENSTER) {
  test(`M1 lange Dateinamen (${fenster.name}): „Entfernen“ und Größe bleiben sichtbar, der Name kürzt mit Ellipse — in der Liste und im Eintrag „wird hochgeladen“`, async ({ browser }) => {
    test.setTimeout(120000);
    instanz = await anhangStarten();
    const { page, fehler } = await geraetOeffnen(browser, fenster.geraet, { fenster: fenster.fenster });
    await imBrowserBeitreten(page, instanz.adresse, 'shoff');
    const cdp = await page.context().newCDPSession(page);
    await cdp.send('Network.enable');
    await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: 4 * 1024 });
    const eintrag = page.getByRole('log').locator('.lc-hochladung');

    for (const zeichen of LAENGEN) {
      for (const mitFoto of [false, true]) {
        const dateien = [textDatei(langerName(zeichen), 40 * 1024)];
        if (mitFoto) {
          dateien.push(fotoPng());
        }
        const wo = `${fenster.name}${mitFoto ? ' mit foto.png' : ''}`;
        await waehlen(page, dateien);
        await expect(page.locator('.lc-dateien .lc-datei')).toHaveCount(dateien.length);
        pruefen(await vermessen(page, '.lc-dateien', '.lc-datei'), { touch: fenster.touch, zeichen, wo: `Liste ${wo}` });

        await knopf(page).click();
        await expect(eintrag.locator('.lc-hochladung-datei')).toHaveCount(dateien.length);
        await expect(eintrag.getByRole('button', { name: `${langerName(zeichen)}: Hochladen abbrechen` })).toBeVisible();
        pruefen(await vermessen(page, '.lc-hochladung-dateien', '.lc-hochladung-datei'), { touch: fenster.touch, zeichen, wo: `Eintrag ${wo}` });
        await eintrag.getByRole('button', { name: 'Verwerfen', exact: true }).click();
        await expect(eintrag).toHaveCount(0);
      }
    }
    expect(fehler).toEqual([]);
  });
}

// UX-Review #43 F0101 S1 (Probe wörtlich): „leeres Feld bei 320 und 375 → Platzhalter einzeilig (nowrap + ellipsis oder
// kürzer), keine Zeile angeschnitten.“ Gemessen: das leere Feld scrollt nicht (scrollHeight ≤ clientHeight) — ein
// zweizeiliger Platzhalter macht es höher als die eine Zeile.
for (const fenster of FENSTER.filter((eintrag) => eintrag.fenster.width <= 375)) {
  test(`S1 Platzhalter einzeilig (${fenster.name})`, async ({ browser }) => {
    instanz = await anhangStarten();
    const { page, fehler } = await geraetOeffnen(browser, fenster.geraet, { fenster: fenster.fenster });
    await imBrowserBeitreten(page, instanz.adresse, 'shoff');

    const feld = page.getByRole('textbox', { name: 'Nachricht an # allgemein' });
    await expect(feld).toHaveValue('');
    const mass = await feld.evaluate((element) => ({
      scroll: element.scrollHeight,
      client: element.clientHeight,
      zeile: parseFloat(getComputedStyle(element).lineHeight),
      platzhalterUmbruch: getComputedStyle(element, '::placeholder').whiteSpace,
    }));
    expect(mass.scroll, JSON.stringify(mass)).toBeLessThanOrEqual(mass.client);
    expect(mass.client, JSON.stringify(mass)).toBeLessThan(2 * mass.zeile);
    expect(fehler).toEqual([]);
  });
}
