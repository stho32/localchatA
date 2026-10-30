'use strict';
// R00030 F0102 „Anhänge im Verlauf zeigen“ (Desktop 1280 × 800, Gegeben über die API): Datei-Zeile mit Kürzel, Name,
// Größe, „Öffnen“ und „Laden“ (B0457), Vorschau (B0458), Bild im Dialog (B0459), Anhänge an der Nachricht (B0461).
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const { anzeigeStarten, dateiZeile } = require('./r00030-anzeige-hilfen');

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
