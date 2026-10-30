'use strict';
// R00030 F0101 B0753 — die Eingabe wächst im sichtbaren Bereich (UX-Vorprüfung #43, V6): Antwortzeile → Liste
// „Angehängte Dateien“ → Feld mit „Anhang“ und „Senden“ → Hinweis- und Tippzeile, oben nach unten wie in der Tab-Folge;
// Tab und `/` aus dem Verlauf landen im Feld; die Liste ist min(3½ Zeilen, 25 % von visualViewport.height), mindestens
// 1½ Zeilen hoch; alles zusammen höchstens 60 %, der Verlauf behält 88 px, die letzte Nachricht bleibt ganz sichtbar
// (R00048). Bildschirmtastatur genähert über ein niedrigeres Fenster (Telefon 375 × 500, Tablet quer 1024 × 400).
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe, nachrichten } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { annaSchreibt } = require('./r00047-hilfen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { schalter, dateiliste, dateizeilen, waehlen, textDatei } = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const zehn = () => Array.from({ length: 10 }, (_, stelle) => textDatei(`bericht-${stelle + 1}.txt`, 100));
const antwortzeile = (page) => page.locator('.lc-comp').getByRole('group', { name: /^Antwort an / });

/// A (`shoff`) auf `geraet` mit `fenster`; B (`anna`) hat zuletzt `Hallo` geschrieben. → { page, fehler, hallo }
async function aufbauen(browser, geraet, fenster) {
  instanz = await sendenStarten({ anzahl: 20 });
  const anna = await annaSchreibt(instanz.adresse, ['Hallo']);
  const seite = await geraetOeffnen(browser, geraet, { fenster, streng: true });
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  await expect(nachrichten(seite.page)).toHaveCount(21);
  return { ...seite, hallo: anna.zeiten[0] };
}

async function antwortenAuf(page, zeit) {
  const nachricht = nachrichtMitZeit(page, zeit);
  await nachricht.getByRole('button', { name: 'Aktionen', exact: true }).tap();
  await nachricht.getByRole('button', { name: 'Antworten', exact: true }).tap();
  await expect(antwortzeile(page)).toBeVisible();
}

/// Lage der Bausteine der Eingabe, des Verlaufs und der letzten Nachricht gegen visualViewport.
function lageMessen(page) {
  return page.evaluate(() => {
    const rechteck = (auswahl) => document.querySelector(auswahl).getBoundingClientRect();
    const sicht = window.visualViewport;
    const teile = {
      antwort: rechteck('.lc-antwortzeile'),
      liste: rechteck('.lc-dateien'),
      kasten: rechteck('.lc-comp-box'),
      hinweis: rechteck('.lc-comp-hinweisreihe'),
    };
    const imSichtbaren = Object.values(teile).every((rand) => rand.top >= sicht.offsetTop - 0.5
      && rand.bottom <= sicht.offsetTop + sicht.height + 0.5 && rand.left >= -0.5 && rand.right <= sicht.width + 0.5);
    const log = document.querySelector('[role="log"]');
    const alle = log.querySelectorAll('.lc-msg');
    const letzte = alle[alle.length - 1].getBoundingClientRect();
    const logRand = log.getBoundingClientRect();
    const listeElement = document.querySelector('.lc-dateien');
    const zeilen = listeElement.querySelectorAll('li');
    const zeilenAbstand = zeilen[1].getBoundingClientRect().top - zeilen[0].getBoundingClientRect().top;
    return {
      sichthoehe: sicht.height,
      imSichtbaren,
      reihenfolge: teile.antwort.bottom <= teile.liste.top + 0.5 && teile.liste.bottom <= teile.kasten.top + 0.5
        && teile.kasten.bottom <= teile.hinweis.top + 0.5,
      summe: teile.hinweis.bottom - teile.antwort.top,
      verlauf: log.clientHeight,
      letzteGanzSichtbar: letzte.top >= logRand.top - 0.5 && letzte.bottom <= logRand.bottom + 0.5,
      liste: { hoehe: listeElement.clientHeight, zeile: zeilen[0].getBoundingClientRect().height, zeilenAbstand, scrollt: listeElement.scrollHeight > listeElement.clientHeight },
      antwortZeilen: [...document.querySelector('.lc-antwortzeile').children].map((kind) => kind.getBoundingClientRect())
        .map((rand) => ({ top: rand.top, bottom: rand.bottom })),
    };
  });
}

/// Die letzte Nachricht steht ganz im Verlauf.
function letzteGanzSichtbar(page) {
  return page.evaluate(() => {
    const log = document.querySelector('[role="log"]');
    const alle = log.querySelectorAll('.lc-msg');
    const letzte = alle[alle.length - 1].getBoundingClientRect();
    const logRand = log.getBoundingClientRect();
    return letzte.top >= logRand.top - 0.5 && letzte.bottom <= logRand.bottom + 0.5;
  });
}

for (const [geraet, fenster] of [['telefon', { width: 375, height: 500 }], ['tabletQuer', { width: 1024, height: 400 }]]) {
  test(`${geraet} ${fenster.width} × ${fenster.height}: Antwortzeile, 10 Dateien, Feld und Hinweiszeile im sichtbaren Bereich, Liste begrenzt, letzte Nachricht bleibt sichtbar`, async ({ browser }) => {
    const { page, fehler, hallo } = await aufbauen(browser, geraet, fenster);
    await expect.poll(() => letzteGanzSichtbar(page)).toBe(true);

    await antwortenAuf(page, hallo);
    await expect.poll(() => letzteGanzSichtbar(page), { intervals: [25], timeout: 3000 }).toBe(true);
    await waehlen(page, zehn(), { geste: 'tippen' });
    await expect(dateizeilen(page)).toHaveCount(10);
    await expect.poll(() => letzteGanzSichtbar(page), { intervals: [25], timeout: 3000 }).toBe(true);

    const lage = await lageMessen(page);
    expect(lage.imSichtbaren, JSON.stringify(lage)).toBe(true);
    expect(lage.reihenfolge, JSON.stringify(lage)).toBe(true);
    expect(lage.summe).toBeLessThanOrEqual(0.6 * lage.sichthoehe);
    expect(lage.verlauf).toBeGreaterThanOrEqual(88);
    expect(lage.liste.zeile).toBe(44);
    const zeilenHoch = lage.liste.hoehe / lage.liste.zeilenAbstand;
    expect(lage.liste.hoehe).toBeLessThanOrEqual(Math.min(3.5 * lage.liste.zeilenAbstand, 0.25 * lage.sichthoehe) + 1);
    expect(zeilenHoch).toBeGreaterThanOrEqual(1.5);
    expect(lage.liste.scrollt).toBe(true);
    if (lage.sichthoehe < 500) {
      const unten = Math.min(...lage.antwortZeilen.map((rand) => rand.bottom));
      expect(lage.antwortZeilen.every((rand) => rand.top < unten), 'Antwortzeile einzeilig').toBe(true);
    }

    await antwortzeile(page).getByRole('button', { name: 'Abbrechen' }).tap();
    await expect(antwortzeile(page)).toBeHidden();
    await expect.poll(() => letzteGanzSichtbar(page), { intervals: [25], timeout: 3000 }).toBe(true);
    expect(fehler).toEqual([]);
  });
}

test('Tab-Folge (Telefon 375 × 500): Antwortzeile → Entfernen ×10 → Feld → Anhang → Senden, wie oben nach unten', async ({ browser }) => {
  const { page, hallo } = await aufbauen(browser, 'telefon', { width: 375, height: 500 });
  await antwortenAuf(page, hallo);
  await waehlen(page, zehn(), { geste: 'tippen' });

  await antwortzeile(page).getByRole('button', { name: 'Abbrechen' }).focus();
  const folge = [];
  for (let schritt = 0; schritt < 13; schritt += 1) {
    await page.keyboard.press('Tab');
    folge.push(await page.evaluate(() => document.activeElement.getAttribute('aria-label') || document.activeElement.textContent));
  }
  const entfernen = Array.from({ length: 10 }, (_, stelle) => `bericht-${stelle + 1}.txt entfernen`);
  expect(folge).toEqual([...entfernen, 'Nachricht an # allgemein', 'Anhang', 'Senden']);
});

test('Tab und / aus dem Verlauf landen im Feld, nicht im ersten „Entfernen“ (Desktop)', async ({ browser }) => {
  const { page, hallo } = await aufbauen(browser, 'desktop');
  await waehlen(page, zehn());
  await expect(dateizeilen(page)).toHaveCount(10);

  await nachrichtMitZeit(page, hallo).focus();
  let ausDemVerlauf = false;
  for (let schritt = 0; schritt < 10 && !ausDemVerlauf; schritt += 1) {
    await page.keyboard.press('Tab');
    ausDemVerlauf = await page.evaluate(() => document.activeElement.closest('.lc-verlauf') === null);
  }
  await expect(eingabe(page)).toBeFocused();

  await nachrichtMitZeit(page, hallo).focus();
  await page.keyboard.press('/');
  await expect(eingabe(page)).toBeFocused();
  await expect(eingabe(page)).toHaveValue('');

  await eingabe(page).press('Shift+Tab');
  await expect(dateiliste(page).getByRole('button', { name: 'bericht-10.txt entfernen' })).toBeFocused();
  await dateiliste(page).getByRole('button', { name: 'bericht-1.txt entfernen' }).click();
  await expect(dateizeilen(page)).toHaveCount(9);
  await expect(schalter(page)).toBeVisible();
  await expect(knopf(page)).toBeVisible();
});

test('Hinweiszeile höchstens 2 Zeilen, dann Ellipse; Tippanzeige in der 20-px-Reihe; der Wechsel 1 ↔ 2 Zeilen verdeckt nichts (Telefon 375 × 500)', async ({ browser }) => {
  const { page } = await aufbauen(browser, 'telefon', { width: 375, height: 500 });
  await expect.poll(() => letzteGanzSichtbar(page)).toBe(true);
  const zeile = page.locator('#lc-comp-hinweis');

  for (const text of ['Eine kurze Zeile.', 'Lang '.repeat(80), 'Eine kurze Zeile.']) {
    await zeile.evaluate((element, inhalt) => { element.textContent = inhalt; }, text);
    await expect.poll(() => letzteGanzSichtbar(page), { intervals: [25], timeout: 3000 }).toBe(true);
    const masse = await zeile.evaluate((element) => ({ hoehe: element.getBoundingClientRect().height, zeilen: getComputedStyle(element).webkitLineClamp }));
    expect(masse.hoehe).toBeLessThanOrEqual(40.5);
    expect(masse.zeilen).toBe('2');
  }
  const tipp = await page.locator('.lc-comp-tipp').evaluate((element) => element.getBoundingClientRect().height);
  expect(tipp).toBe(20);
});
