'use strict';
// R00030 F0102 „Anhänge im Verlauf zeigen“ (Desktop 1280 × 800, Gegeben über die API): Datei-Zeile mit Kürzel, Name,
// Größe, „Öffnen“ und „Laden“ (B0457), Vorschau (B0458), Bild im Dialog (B0459), Anhänge an der Nachricht (B0461).
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { anzeigeStarten, dateiZeile, testdateien } = require('./r00030-anzeige-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Zählt die Anfragen an `/anhaenge/` im Kontext der Seite.
function anhangAnfragenZaehlen(page) {
  const anfragen = [];
  page.context().on('request', (anfrage) => {
    if (new URL(anfrage.url()).pathname.startsWith('/anhaenge/')) {
      anfragen.push(anfrage.url());
    }
  });
  return anfragen;
}

test('Datei-Zeile, Öffnen und Laden (B0457): bericht.pdf mit „PDF“, Name, „2,3 MiB“, „Öffnen“ im neuen Tab, „Laden“; archiv.zip nur „Laden“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['bericht.pdf', 'archiv.zip'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  const [pdfId] = lauf.ids[0];
  const pdf = dateiZeile(knoten, 'bericht.pdf');
  await expect(pdf.locator('.lc-datei-kuerzel')).toHaveText('PDF');
  await expect(pdf.locator('.lc-datei-kuerzel')).toHaveAttribute('aria-hidden', 'true');
  await expect(pdf.locator('.lc-datei-name')).toHaveText('bericht.pdf');
  await expect(pdf.locator('.lc-datei-groesse')).toHaveText('2,3 MiB');
  const oeffnen = pdf.getByRole('link', { name: 'bericht.pdf öffnen', exact: true });
  await expect(oeffnen).toHaveText('Öffnen');
  await expect(oeffnen).toHaveAttribute('target', '_blank');
  await expect(oeffnen).toHaveAttribute('rel', 'noopener');
  await expect(oeffnen).toHaveAttribute('href', `/anhaenge/${pdfId}`);
  const laden = pdf.getByRole('link', { name: 'bericht.pdf laden', exact: true });
  await expect(laden).toHaveText('Laden');
  await expect(laden).toHaveAttribute('download', 'bericht.pdf');

  const pdfAntwort = page.context().waitForEvent('response', (antwort) => antwort.url().endsWith(`/anhaenge/${pdfId}`));
  await oeffnen.click();
  expect((await pdfAntwort).headers()['content-type']).toBe('application/pdf');

  const zip = dateiZeile(knoten, 'archiv.zip');
  await expect(zip.getByRole('link')).toHaveCount(1);
  await expect(zip.getByRole('link', { name: 'archiv.zip öffnen' })).toHaveCount(0);
  const herunter = page.waitForEvent('download');
  await zip.getByRole('link', { name: 'archiv.zip laden', exact: true }).click();
  expect((await herunter).suggestedFilename()).toBe('archiv.zip');
  expect(lauf.a.fehler).toEqual([]);
});

test('Ogg-Video nur laden (B0457, PO 158; Sollte S3): film.ogv → „OGV“, Name, Größe, nur „Laden“ direkt hinter der Größe; bis zum Klick keine Anfrage an /anhaenge/', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['film.ogv'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const anfragen = anhangAnfragenZaehlen(page);
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  const zeile = dateiZeile(knoten, 'film.ogv');
  await expect(zeile.locator('.lc-datei-kuerzel')).toHaveText('OGV');
  await expect(zeile.getByRole('link')).toHaveCount(1);
  await expect(knoten.locator('video, audio')).toHaveCount(0);
  const laden = zeile.getByRole('link', { name: 'film.ogv laden', exact: true });
  const groesseRechts = await zeile.locator('.lc-datei-groesse').evaluate((element) => element.getBoundingClientRect().right);
  const ladenLinks = await laden.evaluate((element) => element.getBoundingClientRect().left);
  expect(ladenLinks - groesseRechts).toBeLessThanOrEqual(8 + 1);
  await page.waitForTimeout(300);
  expect(anfragen).toEqual([]);
  const herunter = page.waitForEvent('download');
  await laden.click();
  expect((await herunter).suggestedFilename()).toBe('film.ogv');
  expect(lauf.a.fehler).toEqual([]);
});

test('Video und Audio vor dem Player (B0457): film.webm und ton.ogg → je eine Datei-Zeile mit „Laden“, ohne <video>/<audio>, ohne „Öffnen“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['film.webm', 'ton.ogg'] }]);
  instanz = lauf.instanz;
  const knoten = nachrichtMitZeit(lauf.a.page, lauf.zeiten[0]);
  for (const name of ['film.webm', 'ton.ogg']) {
    const zeile = dateiZeile(knoten, name);
    await expect(zeile.getByRole('link', { name: `${name} laden`, exact: true })).toHaveCount(1);
    await expect(zeile.getByRole('link')).toHaveCount(1);
  }
  await expect(knoten.locator('video, audio')).toHaveCount(0);
  expect(lauf.a.fehler).toEqual([]);
});

test('HEIC (B0457, „AVIF und HEIC“): foto.heic → nur Datei-Zeile mit „HEIC“ und „Laden“, ohne „Öffnen“, ohne <img> und <video>', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.heic'] }]);
  instanz = lauf.instanz;
  const knoten = nachrichtMitZeit(lauf.a.page, lauf.zeiten[0]);
  const zeile = dateiZeile(knoten, 'foto.heic');
  await expect(zeile.locator('.lc-datei-kuerzel')).toHaveText('HEIC');
  await expect(zeile.getByRole('link')).toHaveCount(1);
  await expect(zeile.getByRole('link', { name: 'foto.heic laden', exact: true })).toHaveCount(1);
  await expect(knoten.locator('img, video, audio')).toHaveCount(0);
  expect(lauf.a.fehler).toEqual([]);
});

test('Kein Skript beim Öffnen (B0457, QS1): boese.html mit Skript → Datei-Zeile mit „Öffnen“; der neue Tab zeigt Text, kein Skript läuft, localStorage der App bleibt', async ({ browser }) => {
  const SKRIPT = "<html><head><title>leer</title></head><body><script>document.title = 'ausgefuehrt'; localStorage.setItem('fremd', '1');</script></body></html>";
  const lauf = await anzeigeStarten(browser, [{ namen: [{ name: 'boese.html', inhalt: Buffer.from(SKRIPT) }] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const vorher = await page.evaluate(() => JSON.stringify(localStorage));
  const zeile = dateiZeile(nachrichtMitZeit(page, lauf.zeiten[0]), 'boese.html');
  const neu = page.context().waitForEvent('page');
  await zeile.getByRole('link', { name: 'boese.html öffnen', exact: true }).click();
  const tab = await neu;
  await tab.waitForLoadState();
  expect(await tab.title()).not.toBe('ausgefuehrt');
  expect(await page.evaluate(() => JSON.stringify(localStorage))).toBe(vorher);
  expect(lauf.a.fehler).toEqual([]);
});

test('Fokus sichtbar (UX M3) an „Öffnen“ und „Laden“: outline nicht none, ≥ 2 px, Abstand ≥ 0', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['bericht.pdf'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  await knoten.focus();
  for (const name of ['bericht.pdf öffnen', 'bericht.pdf laden']) {
    await page.keyboard.press('Tab');
    const link = knoten.getByRole('link', { name, exact: true });
    await expect(link).toBeFocused();
    const rahmen = await link.evaluate((element) => {
      const stil = getComputedStyle(element);
      return { art: stil.outlineStyle, breite: parseFloat(stil.outlineWidth), abstand: parseFloat(stil.outlineOffset) };
    });
    expect(rahmen.art).not.toBe('none');
    expect(rahmen.breite).toBeGreaterThanOrEqual(2);
    expect(rahmen.abstand).toBeGreaterThanOrEqual(0);
  }
  expect(lauf.a.fehler).toEqual([]);
});

/// Hält jede Anfrage an `/anhaenge/` `ms` zurück (verzögerter Fake: zwischen Zeichnen und Bild liegt Zeit).
async function bilderVerzoegern(page, ms) {
  await page.context().route('**/anhaenge/**', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, ms));
    await route.continue();
  });
}

const unterkante = (knoten) => knoten.evaluate((element) => element.getBoundingClientRect().bottom);
const bildGeladen = (bild) => bild.evaluate((element) => element.complete && element.naturalWidth > 0);

/// Misst die Unterkante der Nachricht vor dem Laden ihres Bilds und danach; → { vorher, nachher, bild }.
async function sprungBeimLaden(page, zeit) {
  const knoten = nachrichtMitZeit(page, zeit);
  const bild = knoten.locator('.lc-vorschau img');
  await expect(bild).toHaveCount(1);
  expect(await bildGeladen(bild)).toBe(false);
  const vorher = await unterkante(knoten);
  await expect.poll(() => bildGeladen(bild), { timeout: 10000 }).toBe(true);
  return { vorher, nachher: await unterkante(knoten), bild };
}

/// Startet mit verzögerten Bildern: der Browser tritt erst bei, wenn die Verzögerung steht.
async function mitVerzoegertenBildern(browser, nachrichten) {
  const lauf = await anzeigeStarten(browser, nachrichten, {
    vorSeite: (page) => bilderVerzoegern(page, 1000),
  });
  instanz = lauf.instanz;
  return lauf;
}

test('Bild (B0458): foto.png → <img width="360" height="225" loading="lazy" alt="foto.png">; Platz mit Name und Maßen vor dem Laden; Unterkante springt ≤ 1 px', async ({ browser }) => {
  const lauf = await mitVerzoegertenBildern(browser, [{ namen: ['foto.png'] }]);
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  const bild = knoten.locator('.lc-vorschau img');
  await expect(bild).toHaveAttribute('width', '360');
  await expect(bild).toHaveAttribute('height', '225');
  await expect(bild).toHaveAttribute('loading', 'lazy');
  await expect(bild).toHaveAttribute('decoding', 'async');
  await expect(bild).toHaveAttribute('alt', 'foto.png');
  await expect(knoten.locator('.lc-vorschau-platz')).toHaveText('foto.png · 1280 × 800');
  const { vorher, nachher } = await sprungBeimLaden(page, lauf.zeiten[0]);
  expect(Math.abs(nachher - vorher)).toBeLessThanOrEqual(1);
  const rahmen = await knoten.locator('.lc-vorschau').evaluate((element) => element.getBoundingClientRect());
  expect(Math.abs(rahmen.width - 360)).toBeLessThanOrEqual(1);
  expect(Math.abs(rahmen.height - 225)).toBeLessThanOrEqual(1);
  await expect(knoten.locator('.lc-vorschau-platz')).toBeHidden();
  await expect(dateiZeile(knoten, 'foto.png').getByRole('link', { name: 'foto.png öffnen', exact: true })).toHaveCount(1);
  expect(lauf.a.fehler).toEqual([]);
});

test('Hochformatfoto (B0458, F4): hochkant.jpg (4000 × 3000, Orientation 6) → Platz 180 × 240 vor dem ersten Byte, gezeichnet 3 : 4 ± 1 % mit object-fit: contain, Höhe springt ≤ 1 px', async ({ browser }) => {
  const lauf = await mitVerzoegertenBildern(browser, [{ namen: ['hochkant.jpg'] }]);
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  const bild = knoten.locator('.lc-vorschau img');
  await expect(bild).toHaveAttribute('width', '180');
  await expect(bild).toHaveAttribute('height', '240');
  const platz = await bild.evaluate((element) => element.getBoundingClientRect());
  expect(Math.abs(platz.width / platz.height - 3 / 4)).toBeLessThanOrEqual(0.0075);
  const { vorher, nachher } = await sprungBeimLaden(page, lauf.zeiten[0]);
  expect(Math.abs(nachher - vorher)).toBeLessThanOrEqual(1);
  const gezeichnet = await bild.evaluate((element) => ({ breite: element.naturalWidth, hoehe: element.naturalHeight, passen: getComputedStyle(element).objectFit }));
  expect(gezeichnet.passen).toBe('contain');
  expect(Math.abs(gezeichnet.breite / gezeichnet.hoehe - 3 / 4)).toBeLessThanOrEqual(0.0075);
  expect(lauf.a.fehler).toEqual([]);
});

test('AVIF (B0458): foto.avif (4032 × 3024) → <img width="320" height="240">, Unterkante springt ≤ 1 px; ein AVIF ohne lesbare Maße → nur Datei-Zeile mit „Öffnen“', async ({ browser }) => {
  const ohneMasse = { name: 'leer.avif', inhalt: avifOhneMasse() };
  const lauf = await mitVerzoegertenBildern(browser, [{ namen: ['foto.avif'] }, { namen: [ohneMasse] }]);
  const { page } = lauf.a;
  const bild = nachrichtMitZeit(page, lauf.zeiten[0]).locator('.lc-vorschau img');
  await expect(bild).toHaveAttribute('width', '320');
  await expect(bild).toHaveAttribute('height', '240');
  const { vorher, nachher } = await sprungBeimLaden(page, lauf.zeiten[0]);
  expect(Math.abs(nachher - vorher)).toBeLessThanOrEqual(1);
  const ohne = nachrichtMitZeit(page, lauf.zeiten[1]);
  await expect(ohne.locator('img, video, audio')).toHaveCount(0);
  await expect(dateiZeile(ohne, 'leer.avif').getByRole('link', { name: 'leer.avif öffnen', exact: true })).toHaveCount(1);
  expect(lauf.a.fehler).toEqual([]);
});

test('Fehlerzustand (B0458): scheitert die Bildanfrage, bleibt der Platz und zeigt „Bild nicht verfügbar.“ — nicht „Gelöscht“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png'] }], {
    vorSeite: (page) => page.context().route('**/anhaenge/**', (route) => route.abort('failed')),
  });
  instanz = lauf.instanz;
  const knoten = nachrichtMitZeit(lauf.a.page, lauf.zeiten[0]);
  const vorschau = knoten.locator('.lc-vorschau');
  await expect(vorschau.locator('.lc-vorschau-platz')).toHaveText('Bild nicht verfügbar.');
  await expect(vorschau.locator('.lc-vorschau-platz')).toBeVisible();
  const rahmen = await vorschau.evaluate((element) => element.getBoundingClientRect());
  expect(Math.abs(rahmen.width - 360)).toBeLessThanOrEqual(1);
  expect(Math.abs(rahmen.height - 225)).toBeLessThanOrEqual(1);
  await expect(knoten).not.toContainText('Gelöscht');
  expect(lauf.a.fehler.filter((zeile) => !/Failed to load resource/.test(zeile))).toEqual([]);
});

test('Bild-Polyglot (B0458, QS1): PNG-Kopf mit angehängtem HTML als Vorschau → kein Skript läuft, localStorage der App bleibt', async ({ browser }) => {
  const polyglot = Buffer.concat([testdateien()['foto.png'], Buffer.from("<script>document.title='ausgefuehrt';localStorage.setItem('fremd','1')</script>")]);
  const lauf = await anzeigeStarten(browser, [{ namen: [{ name: 'polyglot.png', inhalt: polyglot }] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const bild = nachrichtMitZeit(page, lauf.zeiten[0]).locator('.lc-vorschau img');
  await expect.poll(() => bildGeladen(bild), { timeout: 10000 }).toBe(true);
  expect(await page.title()).not.toBe('ausgefuehrt');
  expect(await page.evaluate(() => localStorage.getItem('fremd'))).toBeNull();
  expect(lauf.a.fehler).toEqual([]);
});

/// ISOBMFF mit Hauptmarke `avif` ohne `ispe`: der Server erkennt `image/avif` ohne Maße.
function avifOhneMasse() {
  const heic = testdateien()['foto.heic'];
  return Buffer.concat([heic.subarray(0, 8), Buffer.from('avif', 'latin1'), heic.subarray(12)]);
}

const vorschauSchalter = (knoten, name) => knoten.getByRole('button', { name: `${name} groß anzeigen`, exact: true });

test('Bild im Dialog (B0459): Klick → modaler <dialog> „foto.png“ mit dem Bild, Fokus auf „Schließen“, Tab bleibt im Dialog; Esc → zu, Fokus auf der Vorschau', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  const vorschau = vorschauSchalter(knoten, 'foto.png');
  await vorschau.click();
  const dialog = page.getByRole('dialog', { name: 'foto.png', exact: true });
  await expect(dialog).toBeVisible();
  expect(await dialog.evaluate((element) => element.matches(':modal'))).toBe(true);
  await expect(dialog.getByRole('img', { name: 'foto.png', exact: true })).toHaveCount(1);
  const schliessen = dialog.getByRole('button', { name: 'Schließen', exact: true });
  await expect(schliessen).toBeFocused();
  for (const taste of ['Tab', 'Tab', 'Shift+Tab']) {
    await page.keyboard.press(taste);
    expect(await dialog.evaluate((element) => element.contains(document.activeElement))).toBe(true);
  }
  await page.keyboard.press('Escape');
  await expect(page.locator('dialog')).toHaveCount(0);
  await expect(vorschau).toBeFocused();
  expect(lauf.a.fehler).toEqual([]);
});

test('Bild im Dialog (B0459): Enter auf der Vorschau öffnet; „Schließen“ und Tippen daneben schließen, der Fokus kehrt auf die Vorschau zurück', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const vorschau = vorschauSchalter(nachrichtMitZeit(page, lauf.zeiten[0]), 'foto.png');
  const dialog = page.getByRole('dialog', { name: 'foto.png', exact: true });
  await vorschau.focus();
  await page.keyboard.press('Enter');
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Schließen', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(vorschau).toBeFocused();
  await vorschau.click();
  await expect(dialog).toBeVisible();
  await page.mouse.click(5, 5);
  await expect(dialog).toHaveCount(0);
  await expect(vorschau).toBeFocused();
  expect(lauf.a.fehler).toEqual([]);
});

test('Fokus sichtbar (UX M3) an der Vorschau (Umriss ≥ 2 px außerhalb) und an „Schließen“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  const umriss = (locator) => locator.evaluate((element) => {
    const stil = getComputedStyle(element);
    return { art: stil.outlineStyle, breite: parseFloat(stil.outlineWidth), abstand: parseFloat(stil.outlineOffset) };
  });
  await knoten.focus();
  await page.keyboard.press('Tab');
  const vorschau = vorschauSchalter(knoten, 'foto.png');
  await expect(vorschau).toBeFocused();
  const anVorschau = await umriss(vorschau);
  expect(anVorschau.art).not.toBe('none');
  expect(anVorschau.breite).toBeGreaterThanOrEqual(2);
  expect(anVorschau.abstand).toBeGreaterThanOrEqual(2);
  await page.keyboard.press('Enter');
  const schliessen = page.getByRole('dialog').getByRole('button', { name: 'Schließen', exact: true });
  await expect(schliessen).toBeFocused();
  await page.keyboard.press('Tab');
  const anSchliessen = await umriss(schliessen);
  expect(anSchliessen.art).not.toBe('none');
  expect(anSchliessen.breite).toBeGreaterThanOrEqual(2);
  expect(anSchliessen.abstand).toBeGreaterThanOrEqual(0);
  expect(lauf.a.fehler).toEqual([]);
});
