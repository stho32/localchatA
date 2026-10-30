'use strict';
// R00030 F0101 — Kriterien über die Bubbles hinweg: „Fokus sichtbar“ (UX M3) an „Abbrechen“, „Erneut hochladen“ und
// „Verwerfen“, „Kontrast“ (Mutante „opacity: .6 am Eintrag“), „Zielfläche und Ort“ der Schalter am Eintrag und Feldbreite
// mit drei Dateien und einem Eintrag „wird hochgeladen …“, „Bildschirmleser“ (keine zusätzliche Live-Region), „Abgelegte
// Datei“.
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf, box } = require('./r00027-hilfen');
const {
  anhangStarten, dateizeilen, waehlen, berichtPdf, textDatei, archivZip, ringGegenFlaeche, liveRegionen,
} = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const schalterAmEintrag = (page, name) => eintrag(page).getByRole('button', { name, exact: true });

/// A auf `geraet`, Hochladen auf 50 KiB/s gedrosselt; `fehlschlagen`: jeder PUT von `kaputt.bin` bricht ab.
async function seiteAuf(browser, geraet, { thema = 'light', fenster, fehlschlagen = false } = {}) {
  instanz = await anhangStarten();
  const seite = await geraetOeffnen(browser, geraet, { thema, fenster });
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  const cdp = await seite.page.context().newCDPSession(seite.page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: 50 * 1024 });
  if (fehlschlagen) {
    const kaputt = new Set();
    await seite.page.route('**/api/anhaenge', async (route) => {
      const antwort = await route.fetch();
      const daten = await antwort.json();
      if (antwort.status() === 201 && daten.anhang.name === 'kaputt.bin') {
        kaputt.add(daten.anhang.id);
      }
      await route.fulfill({ response: antwort, json: daten });
    });
    await seite.page.route('**/api/anhaenge/*', (route) => {
      const id = new URL(route.request().url()).pathname.split('/').at(-1);
      if (route.request().method() === 'PUT' && kaputt.has(id)) {
        return route.abort();
      }
      return route.continue();
    });
  }
  return seite;
}

/// Ein Eintrag „wird hochgeladen …“: `warten` — `bericht.pdf` und `log.txt` laden, `archiv.zip` wartet (höchstens zwei
/// zugleich), `gross.bin` ist bei der Anmeldung abgewiesen (zu groß); `fehlschlag` — `kaputt.bin` (PUT bricht ab) und
/// `bericht.pdf` laden zugleich.
async function eintragMitZustaenden(page, geraet, variante = 'warten') {
  let dateien = [berichtPdf(), textDatei('log.txt', 400 * 1024), archivZip('archiv.zip', 100 * 1024), archivZip('gross.bin', 10485761)];
  if (variante === 'fehlschlag') {
    dateien = [archivZip('kaputt.bin', 1024), berichtPdf()];
  }
  await waehlen(page, dateien, { geste: geraet === 'desktop' ? 'klick' : 'tippen' });
  await knopf(page).click();
  await expect(eintrag(page)).toHaveCount(1);
}

for (const thema of ['light', 'dark']) {
  test(`Fokus sichtbar (Desktop, ${thema}, UX M3): „Abbrechen“, „Erneut hochladen“ und „Verwerfen“ mit eigenem Ring`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, 'desktop', { thema, fehlschlagen: true });
    await eintragMitZustaenden(page, 'desktop', 'fehlschlag');
    await expect(schalterAmEintrag(page, 'kaputt.bin erneut hochladen')).toBeVisible();

    for (const name of ['bericht.pdf: Hochladen abbrechen', 'kaputt.bin erneut hochladen', 'Verwerfen']) {
      await eingabe(page).press('Shift+Tab');
      await schalterAmEintrag(page, name).focus();
      const ring = await schalterAmEintrag(page, name).evaluate(ringGegenFlaeche);
      expect(ring.sichtbar, name).toBe(true);
      expect(ring.stil, name).not.toBe('none');
      expect(ring.breite, name).toBeGreaterThanOrEqual(2);
      expect(ring.versatz, name).toBeGreaterThanOrEqual(0);
      expect(ring.kontrast, name).toBeGreaterThanOrEqual(3);
    }
  });
}

for (const thema of ['light', 'dark']) {
  test(`Kontrast (Desktop, ${thema}): „wird hochgeladen …“, Name, Größe, „wartet“, Prozent und Fehlertext ≥ 4,5 : 1 mit opacity; Balken ≥ 3 : 1 gegen die Spur`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, 'desktop', { thema });
    await eintragMitZustaenden(page, 'desktop');
    await expect(eintrag(page).locator('.lc-hochladung-zustand', { hasText: /^wartet$/ }).first()).toBeVisible();
    await expect(eintrag(page).locator('.lc-hochladung-grund:not([hidden])')).toBeVisible();
    await expect(eintrag(page).locator('.lc-hochladung-fortschritt:not([hidden]) .lc-balken-prozent').first()).toBeVisible();

    const teile = {
      'wird hochgeladen …': eintrag(page).locator('.lc-send'),
      Name: eintrag(page).locator('.lc-datei-name').first(),
      Größe: eintrag(page).locator('.lc-datei-groesse').first(),
      wartet: eintrag(page).locator('.lc-hochladung-zustand', { hasText: /^wartet$/ }).first(),
      Prozent: eintrag(page).locator('.lc-hochladung-fortschritt:not([hidden]) .lc-balken-prozent').first(),
      Fehlertext: eintrag(page).locator('.lc-hochladung-grund:not([hidden])'),
    };
    for (const [name, teil] of Object.entries(teile)) {
      await eintrag(page).hover();
      expect(await teil.evaluate(schriftKontrast), name).toBeGreaterThanOrEqual(4.5);
    }
    const balken = await eintrag(page).locator('.lc-hochladung-fortschritt:not([hidden]) .lc-balken').first().evaluate(balkenKontrast);
    expect(balken).toBeGreaterThanOrEqual(3);
  });
}

for (const geraet of ['telefon', 'tablet']) {
  test(`Zielfläche (${geraet}): „Abbrechen“, „Erneut hochladen“ und „Verwerfen“ je ≥ 44 × 44 px, ≥ 8 px auseinander`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, geraet, { fehlschlagen: true });
    await eintragMitZustaenden(page, geraet, 'fehlschlag');
    await expect(schalterAmEintrag(page, 'kaputt.bin erneut hochladen')).toBeVisible();

    const boxen = [];
    for (const name of ['kaputt.bin erneut hochladen', 'kaputt.bin entfernen', 'bericht.pdf: Hochladen abbrechen', 'Verwerfen']) {
      const masse = await box(schalterAmEintrag(page, name));
      expect(masse.width, name).toBeGreaterThanOrEqual(44);
      expect(masse.height, name).toBeGreaterThanOrEqual(44);
      boxen.push(masse);
    }
    for (let i = 0; i < boxen.length; i += 1) {
      for (let j = i + 1; j < boxen.length; j += 1) {
        const waagerecht = Math.max(boxen[j].left - boxen[i].right, boxen[i].left - boxen[j].right);
        const senkrecht = Math.max(boxen[j].top - boxen[i].bottom, boxen[i].top - boxen[j].bottom);
        expect(Math.max(waagerecht, senkrecht), `Abstand ${i} ↔ ${j}`).toBeGreaterThanOrEqual(8);
      }
    }
  });
}

for (const [breite, feldMindestens] of [[375, 160], [320, 100]]) {
  test(`Schmal (Telefon ${breite} px): mit drei gewählten Dateien und einem Eintrag „wird hochgeladen …“ ist das Feld ≥ ${feldMindestens} px, kein Querscrollen`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, 'telefon', { fenster: { width: breite, height: 812 } });
    await eintragMitZustaenden(page, 'telefon');
    await waehlen(page, [textDatei('a.txt'), textDatei('b mit einem recht langen Namen für die schmale Zeile.txt'), textDatei('c.txt')], { geste: 'tippen' });
    await expect(dateizeilen(page)).toHaveCount(3);

    expect((await box(eingabe(page))).width).toBeGreaterThanOrEqual(feldMindestens);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
}

test('Bildschirmleser (Desktop): mit Eintrag, Balken und Schaltern keine zusätzliche Live-Region und kein role="alert"', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'desktop');
  const vorher = await liveRegionen(page);

  await eintragMitZustaenden(page, 'desktop');
  await expect(page.getByRole('progressbar', { name: 'bericht.pdf wird hochgeladen' })).toHaveAttribute('aria-valuenow', /^\d+$/);
  await expect(schalterAmEintrag(page, 'bericht.pdf: Hochladen abbrechen')).toBeVisible();

  expect(await liveRegionen(page)).toBe(vorher);
  await expect(page.locator('[role="alert"]')).toHaveCount(0);
});

test('Abgelegte Datei (Desktop): dragover und drop auf den Verlauf → Standardaktion verhindert, Adresse unverändert, keine Datei in der Liste', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'desktop');
  const adresse = page.url();

  const verhindert = await page.getByRole('log').evaluate((log) => {
    const daten = new DataTransfer();
    daten.items.add(new File(['inhalt'], 'abgelegt.txt', { type: 'text/plain' }));
    return ['dragover', 'drop'].map((art) => {
      const ereignis = new DragEvent(art, { bubbles: true, cancelable: true, dataTransfer: daten });
      log.dispatchEvent(ereignis);
      return ereignis.defaultPrevented;
    });
  });

  expect(verhindert).toEqual([true, true]);
  expect(page.url()).toBe(adresse);
  await expect(dateizeilen(page)).toHaveCount(0);
});

/// Kontrast der Schrift gegen die erste deckende Fläche darunter, mit der opacity aller Vorfahren.
function schriftKontrast(element) {
  function anteile(farbe) {
    return farbe.match(/[\d.]+/g).map(Number);
  }
  function leuchtdichte([r, g, b]) {
    const [lr, lg, lb] = [r, g, b].map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
  }
  let deckkraft = 1;
  for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
    deckkraft *= parseFloat(getComputedStyle(knoten).opacity);
  }
  let flaeche = [255, 255, 255];
  for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
    const farbe = anteile(getComputedStyle(knoten).backgroundColor);
    if (farbe.length === 3 || farbe[3] === 1) {
      flaeche = farbe.slice(0, 3);
      break;
    }
  }
  const schrift = anteile(getComputedStyle(element).color).slice(0, 3).map((wert, stelle) => wert * deckkraft + flaeche[stelle] * (1 - deckkraft));
  const hell = Math.max(leuchtdichte(schrift), leuchtdichte(flaeche));
  const dunkel = Math.min(leuchtdichte(schrift), leuchtdichte(flaeche));
  return (hell + 0.05) / (dunkel + 0.05);
}

/// Kontrast der Füllung des Balkens gegen seine Spur.
function balkenKontrast(balken) {
  function leuchtdichte(farbe) {
    const [r, g, b] = farbe.match(/[\d.]+/g).slice(0, 3).map(Number).map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }
  const fuellung = leuchtdichte(getComputedStyle(balken.firstElementChild).backgroundColor);
  const spur = leuchtdichte(getComputedStyle(balken).backgroundColor);
  return (Math.max(fuellung, spur) + 0.05) / (Math.min(fuellung, spur) + 0.05);
}
