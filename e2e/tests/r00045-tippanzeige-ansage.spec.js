'use strict';
// R00045 F0162 B0691 (Karte #77, ux-reviewer C): eigene Ansage-Region `.lc-tipp-ansage` für Bildschirmleser ohne
// Dauerrauschen — `anna` tippt 12 s ohne Pause im Browser, `ben` hört zu: genau zwei Änderungen an der Region
// ('' → 'anna tippt' → ''), keine an der gemeinsamen `.lc-ansage` (Mutanten M13, M14). Bei gekürzter Zeile trägt die
// Region den vollen Text. Die 30-s-Regel prüft der Unit-Test mit gestellter Uhr (e2e/einheit/aktivitaetsanzeige.test.mjs).
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe, beitreten, anfragen } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const tippzeile = (page) => page.locator('.lc-comp .lc-comp-tipp');
const tippAnsage = (page) => page.locator('.lc-tipp-ansage');

/// Schreibt jede Änderung am Text der beiden Regionen mit (auch das Leeren).
function regionenBelauschen(page) {
  return page.evaluate(() => {
    window.geaendert = { tipp: [], gemeinsam: [] };
    const beobachten = (auswahl, liste) => {
      const region = document.querySelector(auswahl);
      new MutationObserver(() => liste.push(region.textContent))
        .observe(region, { childList: true, characterData: true, subtree: true });
    };
    beobachten('.lc-tipp-ansage', window.geaendert.tipp);
    beobachten('.lc-ansage', window.geaendert.gemeinsam);
  });
}

test('Bildschirmleser (US-7, C): anna tippt 12 s ohne Pause → genau zwei Änderungen an .lc-tipp-ansage, keine an .lc-ansage', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await sendenStarten();
  const anna = await browserOeffnen(browser, { streng: true });
  const ben = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');
  await regionenBelauschen(ben.page);

  await eingabe(anna.page).click();
  const beginn = Date.now();
  await eingabe(anna.page).pressSequentially('x'.repeat(60), { delay: 200 });
  expect(Date.now() - beginn).toBeGreaterThanOrEqual(12000);
  await expect(tippzeile(ben.page)).toHaveText('anna tippt …');
  await expect(tippzeile(ben.page)).toHaveText('', { timeout: 10000 });

  const geaendert = await ben.page.evaluate(() => window.geaendert);
  expect(geaendert).toEqual({ tipp: ['anna tippt', ''], gemeinsam: [] });
  expect([...anna.fehler, ...ben.fehler]).toEqual([]);
});

test('Region (US-7, C): eigene verborgene Statusregion, nicht .lc-ansage; die sichtbare Zeile hat kein aria-live', async ({ browser }) => {
  instanz = await sendenStarten();
  const ben = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');

  const region = await tippAnsage(ben.page).evaluate((element) => ({
    rolle: element.getAttribute('role'),
    live: element.getAttribute('aria-live'),
    atomar: element.getAttribute('aria-atomic'),
    unsichtbar: element.classList.contains('lc-unsichtbar'),
    gemeinsam: element.classList.contains('lc-ansage'),
    imVerlauf: element.closest('[role="log"]') !== null,
    hoehe: element.getBoundingClientRect().height,
  }));
  expect(region).toEqual({ rolle: 'status', live: 'polite', atomar: 'true', unsichtbar: true, gemeinsam: false, imVerlauf: false, hoehe: 1 });
  await expect(tippzeile(ben.page)).not.toHaveAttribute('aria-live');
  await expect(tippAnsage(ben.page)).toHaveCount(1);
  expect(ben.fehler).toEqual([]);
});

test('Gekürzte Zeile (US-6, D): bei 320 px trägt .lc-tipp-ansage den vollen Text ohne „ …“', async ({ browser }) => {
  instanz = await sendenStarten();
  const ben = await browserOeffnen(browser, { geraet: { viewport: { width: 320, height: 568 } }, streng: true });
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben', 'technik');
  const namen = ['a', 'b', 'c', 'd'].map((buchstabe) => `${buchstabe.repeat(31)}1`);
  const zustaende = ['tippt', 'tippt', 'denkt_nach', 'denkt_nach'];
  for (const [stelle, name] of namen.entries()) {
    const token = await beitreten(instanz.adresse, name);
    const antwort = await anfragen(instanz.adresse, {
      methode: 'PUT', pfad: 'api/channels/allgemein/aktivitaet', koerper: { zustand: zustaende[stelle], gueltig_s: 60 }, token,
    });
    expect(antwort.status).toBe(200);
  }
  const voll = `${namen[0]} und ${namen[1]} tippen · ${namen[2]} und ${namen[3]} denken nach`;
  await expect(tippzeile(ben.page)).toHaveText('');

  await ben.page.evaluate(() => { window.location.hash = '#/c/allgemein'; });

  await expect(tippzeile(ben.page)).toHaveText(`${voll} …`);
  expect(await tippzeile(ben.page).evaluate((zeile) => zeile.scrollWidth > zeile.clientWidth)).toBe(true);
  await expect(tippAnsage(ben.page)).toHaveText(voll);
  expect(ben.fehler).toEqual([]);
});
