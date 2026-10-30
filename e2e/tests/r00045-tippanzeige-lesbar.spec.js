'use strict';
// R00045 F0162 B0690 (Karte #77, ux-reviewer A, B, D): Wortlaut mit mehreren Namen und beiden Zuständen, Platz in der
// vorhandenen 20-px-Hinweisreihe, Vorrang des Hinweises, eine Zeile mit Ellipse, Kontrast. `ben` sieht im Browser zu;
// die anderen setzen ihren Zustand über die API (dieselbe Route wie der Browser, mit langer Gültigkeit).
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, beitreten, anfragen, eingabe } = require('./senden-hilfen');
const { browserOeffnen, schriftKontrast } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const BREITEN = [
  { name: 'Desktop 1280 × 800', viewport: { width: 1280, height: 800 } },
  { name: 'Telefon 375 × 667', viewport: { width: 375, height: 667 } },
  { name: 'Kleines Telefon 320 × 568', viewport: { width: 320, height: 568 } },
];
const tippzeile = (page) => page.locator('.lc-comp .lc-comp-tipp');

async function benSiehtZu(browser, { geraet = BREITEN[0], farbschema = 'light' } = {}) {
  instanz = await sendenStarten({ anzahl: 30 });
  const ben = await browserOeffnen(browser, { geraet: { viewport: geraet.viewport, colorScheme: farbschema }, streng: true });
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');
  return ben;
}

/// Tritt per API bei und setzt `zustand` in `allgemein` für 60 s; → Token.
async function setztPerApi(name, zustand) {
  const token = await beitreten(instanz.adresse, name);
  await zustandSetzen(token, zustand);
  return token;
}

async function zustandSetzen(token, zustand) {
  const antwort = await anfragen(instanz.adresse, {
    methode: 'PUT', pfad: 'api/channels/allgemein/aktivitaet', koerper: { zustand, gueltig_s: 60 }, token,
  });
  expect(antwort.status, JSON.stringify(antwort.daten)).toBe(200);
}

async function zustandLoeschen(token) {
  const antwort = await anfragen(instanz.adresse, { methode: 'DELETE', pfad: 'api/channels/allgemein/aktivitaet', token });
  expect(antwort.status).toBe(204);
}

/// Unterkante der letzten Nachricht, Höhe von Tippzeile und Eingabe (px).
function lageMessen(page) {
  return page.evaluate(() => {
    const nachrichten = document.querySelectorAll('[role="log"] .lc-msg');
    return {
      letzteUnten: nachrichten[nachrichten.length - 1].getBoundingClientRect().bottom,
      tipp: document.querySelector('.lc-comp .lc-comp-tipp').getBoundingClientRect().height,
      eingabe: document.querySelector('.lc-comp').getBoundingClientRect().height,
    };
  });
}

test('Mehrere Namen (US-6, A): ein Name, zwei, ab drei gezählt, beide Zustände; Namen halbfett, keine Animation', async ({ browser }) => {
  const ben = await benSiehtZu(browser);

  await setztPerApi('anna', 'tippt');
  await expect(tippzeile(ben.page)).toHaveText('anna tippt …');
  await setztPerApi('cora', 'tippt');
  await expect(tippzeile(ben.page)).toHaveText('anna und cora tippen …');
  await setztPerApi('claude', 'denkt_nach');
  await expect(tippzeile(ben.page)).toHaveText('anna und cora tippen · claude denkt nach …');
  await setztPerApi('dora', 'tippt');
  await expect(tippzeile(ben.page)).toHaveText('3 Teilnehmer tippen · claude denkt nach …');

  const stil = await tippzeile(ben.page).evaluate((zeile) => {
    const berechnet = getComputedStyle(zeile);
    return {
      animation: berechnet.animationName,
      uebergang: berechnet.transitionDuration,
      namen: [...zeile.querySelectorAll('.lc-comp-tipp-name')].map((name) => [name.textContent, getComputedStyle(name).fontWeight]),
      schrift: [berechnet.fontSize, berechnet.lineHeight, berechnet.fontFamily.split(',')[0]],
    };
  });
  expect(stil).toEqual({
    animation: 'none', uebergang: '0s', namen: [['claude', '600']], schrift: ['12px', '20px', '"IBM Plex Sans"'],
  });
  expect(ben.fehler).toEqual([]);
});

for (const geraet of BREITEN) {
  test(`Platz reserviert (US-6, B) — ${geraet.name}: Tippzeile 20 px, Eingabe 82 px, die letzte Nachricht steht vor, während und nach der Anzeige auf derselben Pixelzeile`, async ({ browser }) => {
    const ben = await benSiehtZu(browser, { geraet });
    const vorher = await lageMessen(ben.page);

    const anna = await setztPerApi('anna', 'tippt');
    await expect(tippzeile(ben.page)).toHaveText('anna tippt …');
    const waehrend = await lageMessen(ben.page);
    await zustandLoeschen(anna);
    await expect(tippzeile(ben.page)).toHaveText('');
    const nachher = await lageMessen(ben.page);

    expect(waehrend).toEqual({ letzteUnten: vorher.letzteUnten, tipp: 20, eingabe: 82 });
    expect(nachher).toEqual(vorher);
    expect(ben.fehler).toEqual([]);
  });
}

test('Vorrang und Zugänglichkeit (US-6, B): Text im Hinweis → Tippanzeige visibility hidden, Platz bleibt; nie in aria-describedby, nicht fokussierbar', async ({ browser }) => {
  const ben = await benSiehtZu(browser);
  await setztPerApi('anna', 'tippt');
  await expect(tippzeile(ben.page)).toHaveText('anna tippt …');
  await expect(tippzeile(ben.page)).toBeVisible();

  await eingabe(ben.page).fill('a\u0007');
  await expect(ben.page.locator('.lc-comp-hinweis')).toHaveText(/^⚠ Steuerzeichen/);
  await expect(tippzeile(ben.page)).toHaveCSS('visibility', 'hidden');
  expect((await lageMessen(ben.page)).tipp).toBe(20);
  await eingabe(ben.page).fill('a');
  await expect(ben.page.locator('.lc-comp-hinweis')).toHaveText('');
  await expect(tippzeile(ben.page)).toHaveCSS('visibility', 'visible');

  const zugang = await ben.page.evaluate(() => {
    const zeile = document.querySelector('.lc-comp .lc-comp-tipp');
    const beschrieben = [...document.querySelectorAll('[aria-describedby]')]
      .flatMap((element) => element.getAttribute('aria-describedby').split(/\s+/))
      .map((id) => document.getElementById(id));
    return {
      inBeschreibung: beschrieben.some((element) => element !== null && (element === zeile || element.contains(zeile))),
      imHinweis: zeile.closest('.lc-comp-hinweis') !== null,
      tabindex: zeile.getAttribute('tabindex'),
      ariaLive: zeile.getAttribute('aria-live'),
    };
  });
  expect(zugang).toEqual({ inBeschreibung: false, imHinweis: false, tabindex: null, ariaLive: null });
  await tippzeile(ben.page).evaluate((zeile) => zeile.focus());
  await expect(tippzeile(ben.page)).not.toBeFocused();
  expect(ben.fehler).toEqual([]);
});

for (const geraet of BREITEN.slice(1)) {
  test(`Eine Zeile (US-6, D) — ${geraet.name}: längster Text (zwei Gruppen, je zwei 32-stellige Namen) bleibt 20 px hoch, gekürzt mit Ellipse, ohne waagrechtes Scrollen`, async ({ browser }) => {
    const ben = await benSiehtZu(browser, { geraet });
    const namen = ['a', 'b', 'c', 'd'].map((buchstabe) => `${buchstabe.repeat(31)}1`);

    await setztPerApi(namen[0], 'tippt');
    await setztPerApi(namen[1], 'tippt');
    await setztPerApi(namen[2], 'denkt_nach');
    await setztPerApi(namen[3], 'denkt_nach');
    await expect(tippzeile(ben.page)).toHaveText(`${namen[0]} und ${namen[1]} tippen · ${namen[2]} und ${namen[3]} denken nach …`);

    const lage = await tippzeile(ben.page).evaluate((zeile) => {
      const berechnet = getComputedStyle(zeile);
      return {
        hoehe: zeile.getBoundingClientRect().height,
        gekuerzt: zeile.scrollWidth > zeile.clientWidth,
        stil: [berechnet.whiteSpace, berechnet.overflowX, berechnet.textOverflow],
        seiteScrolltQuer: document.documentElement.scrollWidth > document.documentElement.clientWidth,
      };
    });
    expect(lage).toEqual({ hoehe: 20, gekuerzt: true, stil: ['nowrap', 'hidden', 'ellipsis'], seiteScrolltQuer: false });
    expect((await lageMessen(ben.page)).eingabe).toBe(82);
    expect(ben.fehler).toEqual([]);
  });
}

for (const farbschema of ['light', 'dark']) {
  test(`Kontrast (US-6, B) — Thema ${farbschema}: Schrift und Namen mindestens 4,5:1 gegen die Fläche`, async ({ browser }) => {
    const ben = await benSiehtZu(browser, { farbschema });
    await setztPerApi('anna', 'tippt');
    await expect(tippzeile(ben.page)).toHaveText('anna tippt …');
    await expect(ben.page.locator('html')).toHaveAttribute('data-theme', farbschema);

    expect(await schriftKontrast(tippzeile(ben.page))).toBeGreaterThanOrEqual(4.5);
    expect(await schriftKontrast(tippzeile(ben.page).locator('.lc-comp-tipp-name'))).toBeGreaterThanOrEqual(4.5);
    expect(ben.fehler).toEqual([]);
  });
}
