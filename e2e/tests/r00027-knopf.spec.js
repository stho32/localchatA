'use strict';
// R00027 F0091 Senden-Knopf neben dem Feld — wie der Knopf aussieht und wo er steht (B0414): Gruppe, Zielfläche, Ort
// und Schmal, Kontrast, Grund am Knopf, Bildschirmleser. Geräte als emulierte Kontexte (r00027-hilfen.js).
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { ROUTE } = require('./r00012-hilfen');
const { geraetOeffnen, knopf, box, ganzImSichtbaren } = require('./r00027-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function seiteAuf(browser, geraet, optionen = {}) {
  const seite = await geraetOeffnen(browser, geraet, optionen);
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  return seite;
}

const ZU_LANG = 'a'.repeat(16385);
const STEUERZEICHEN = 'rot \u001b[31m';

/// Füllt das Feld wie ein Einfügen (ein input-Ereignis), ohne zu senden.
async function einfuegen(page, text) {
  await eingabe(page).fill(text);
}

for (const geraet of ['telefon', 'desktop']) {
  // Mit R00030 („Bestand bleibt grün“): „Senden“ ist das letzte Kind, „Anhang“ der einzige weitere Schalter.
  test(`Gruppe (${geraet}): „Senden“ ist der letzte Schalter rechts vom Feld, „Anhang“ der einzige weitere, Abstand ≥ 8 px`, async ({ browser }) => {
    instanz = await sendenStarten();
    const { page, fehler } = await seiteAuf(browser, geraet);

    const gruppe = page.locator('.lc-comp-box > .lc-comp-aktionen');
    await expect(gruppe).toHaveCount(1);
    const aufbau = await gruppe.evaluate((element) => ({
      letztesIstKnopf: element.lastElementChild.textContent === 'Senden',
      schalter: [...element.querySelectorAll('button, [role="button"]')].map((schalter) => schalter.getAttribute('aria-label') || schalter.textContent),
      abstand: parseFloat(getComputedStyle(element).columnGap),
    }));
    expect(aufbau).toEqual({ letztesIstKnopf: true, schalter: ['Anhang', 'Senden'], abstand: expect.any(Number) });
    expect(aufbau.abstand).toBeGreaterThanOrEqual(8);
    expect(fehler).toEqual([]);
  });
}

for (const [geraet, mindestens] of [['telefon', 44], ['tablet', 44], ['tabletQuer', 44], ['desktop', 24]]) {
  test(`Zielfläche (${geraet}): der Knopf misst mindestens ${mindestens} × ${mindestens} px`, async ({ browser }) => {
    instanz = await sendenStarten();
    const { page } = await seiteAuf(browser, geraet);

    const masse = await box(knopf(page));
    expect(masse.width).toBeGreaterThanOrEqual(mindestens);
    expect(masse.height).toBeGreaterThanOrEqual(mindestens);
    await expect(knopf(page)).toHaveCSS('touch-action', 'manipulation');
  });
}

for (const geraet of ['telefon', 'tablet', 'tabletQuer', 'desktop']) {
  test(`Ort (${geraet}): rechts neben dem Feld, unten bündig — auch bei 200 px Feldhöhe`, async ({ browser }) => {
    instanz = await sendenStarten();
    const { page } = await seiteAuf(browser, geraet);
    const feld = eingabe(page);

    await expect(knopf(page)).toBeVisible();
    await liegtRechtsUntenNeben(feld, knopf(page));
    await feld.click();
    for (let zeile = 1; zeile <= 20; zeile += 1) {
      await feld.press('Shift+Enter');
    }
    expect((await box(feld)).height).toBeGreaterThan(190);
    await liegtRechtsUntenNeben(feld, knopf(page));
  });
}

async function liegtRechtsUntenNeben(feld, schalter) {
  const feldBox = await box(feld);
  const knopfBox = await box(schalter);
  expect(knopfBox.left).toBeGreaterThanOrEqual(feldBox.right);
  expect(Math.abs(knopfBox.bottom - feldBox.bottom)).toBeLessThanOrEqual(1);
}

test('Schmal (Telefon-Kontext, 320 px): kein Querscrollen mit Feld, Knopf und einer fehlgeschlagenen Nachricht', async ({ browser }) => {
  instanz = await sendenStarten();
  const { page } = await seiteAuf(browser, 'telefon', { fenster: { width: 320, height: 812 } });
  await page.route(ROUTE, (route) => route.abort());

  await eingabe(page).fill('kaputt');
  await eingabe(page).press('Control+Enter');
  await expect(page.getByRole('button', { name: 'Erneut senden' })).toBeVisible();
  await expect(knopf(page)).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});

for (const [geraet, fenster] of [['telefon', { width: 375, height: 500 }], ['tabletQuer', { width: 1024, height: 400 }]]) {
  test(`Schmal mit Bildschirmtastatur (${geraet} ${fenster.width} × ${fenster.height}): Feld, Knopf und Hinweiszeile ganz sichtbar`, async ({ browser }) => {
    instanz = await sendenStarten();
    const { page } = await seiteAuf(browser, geraet, { fenster });
    const teile = [eingabe(page), knopf(page), page.locator('#lc-comp-hinweis')];

    for (const teil of teile) {
      expect(await ganzImSichtbaren(teil)).toBe(true);
    }
    await einfuegen(page, ZU_LANG);
    await expect(page.locator('#lc-comp-hinweis')).toContainText('⚠ Zu lang');
    for (const teil of teile) {
      expect(await ganzImSichtbaren(teil)).toBe(true);
    }
  });
}

for (const geraet of ['desktop', 'telefon']) {
  for (const thema of ['light', 'dark']) {
    test(`Kontrast (${geraet}, ${thema}): Text gegen Fläche ≥ 4,5 : 1, bereit und nicht verfügbar, mit opacity`, async ({ browser }) => {
      instanz = await sendenStarten();
      const { page } = await seiteAuf(browser, geraet, { thema });

      await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
      expect(await knopf(page).evaluate(gezeichneterKontrast)).toBeGreaterThanOrEqual(4.5);
      await einfuegen(page, 'x');
      await expect(knopf(page)).not.toHaveAttribute('aria-disabled', 'true');
      expect(await knopf(page).evaluate(gezeichneterKontrast)).toBeGreaterThanOrEqual(4.5);
    });
  }
}

/// Kontrast der Farben, die der Browser zeichnet: Text und Fläche des Knopfs, je mit der opacity des Knopfs und seiner
/// Vorfahren über die Fläche dahinter gemischt (WCAG 2.x).
function gezeichneterKontrast(element) {
  function anteile(farbe) {
    const werte = farbe.match(/[\d.]+/g).map(Number);
    if (werte.length === 3) {
      werte.push(1);
    }
    return werte;
  }
  function mischen(vorne, hinten, deckung) {
    return [0, 1, 2].map((kanal) => vorne[kanal] * deckung + hinten[kanal] * (1 - deckung));
  }
  function leuchtdichte(rgb) {
    const [r, g, b] = rgb.map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }
  function flaecheHinter(knoten) {
    for (let vorfahr = knoten.parentElement; vorfahr !== null; vorfahr = vorfahr.parentElement) {
      const farbe = anteile(getComputedStyle(vorfahr).backgroundColor);
      if (farbe[3] > 0) {
        return farbe;
      }
    }
    return [255, 255, 255, 1];
  }
  let deckung = 1;
  for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
    deckung *= Number(getComputedStyle(knoten).opacity);
  }
  const stil = getComputedStyle(element);
  const hinten = flaecheHinter(element);
  const eigeneFlaeche = anteile(stil.backgroundColor);
  const flaeche = mischen(mischen(eigeneFlaeche, hinten, eigeneFlaeche[3]), hinten, deckung);
  const schrift = anteile(stil.color);
  const text = mischen(mischen(schrift, flaeche, schrift[3]), hinten, deckung);
  const hell = Math.max(leuchtdichte(text), leuchtdichte(flaeche));
  const dunkel = Math.min(leuchtdichte(text), leuchtdichte(flaeche));
  return (hell + 0.05) / (dunkel + 0.05);
}

test('Grund am Knopf (Desktop): die Beschreibung des Knopfs ist die Hinweiszeile', async ({ browser }) => {
  instanz = await sendenStarten();
  const { page } = await seiteAuf(browser, 'desktop');

  await expect(knopf(page)).toHaveAttribute('aria-describedby', 'lc-comp-hinweis');
  await expect(knopf(page)).toHaveAccessibleDescription('');
  await einfuegen(page, ZU_LANG);
  await expect(knopf(page)).toHaveAccessibleDescription(/^⚠ Zu lang: /);
  await einfuegen(page, '');
  await expect(knopf(page)).toHaveAccessibleDescription('');
});

test('Bildschirmleser (Desktop): ein Schalter „Senden“ ohne aria-label und ohne disabled, nicht bereit per Tab erreichbar', async ({ browser }) => {
  instanz = await sendenStarten();
  const { page, fehler } = await seiteAuf(browser, 'desktop');

  await expect(page.locator('.lc-comp').getByRole('button', { name: 'Senden' })).toHaveCount(1);
  await expect(knopf(page)).not.toHaveAttribute('aria-label', /.*/);
  await expect(knopf(page)).not.toHaveAttribute('disabled', /.*/);
  await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
  await eingabe(page).focus();
  await page.keyboard.press('Tab');
  await expect(page.locator('.lc-comp').getByRole('button', { name: 'Anhang', exact: true })).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(knopf(page)).toBeFocused();
  await einfuegen(page, STEUERZEICHEN);
  await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
  await einfuegen(page, 'Danke');
  await expect(knopf(page)).not.toHaveAttribute('aria-disabled', /.*/);
  const imKnopfBereich = await page.locator('.lc-comp-aktionen')
    .evaluate((gruppe) => gruppe.querySelectorAll('[aria-live], [role="status"], [role="alert"]').length
      + Number(gruppe.matches('[aria-live], [role="status"], [role="alert"]')));
  expect(imKnopfBereich).toBe(0);
  expect(fehler).toEqual([]);
});
