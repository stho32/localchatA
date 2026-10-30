'use strict';
// R00030 F0118 „Gelöschte Anhänge im Verlauf“, Teil B0517 (Gelöschter Anhang als Zeile; UX V4, V5), Desktop 1280 × 800,
// Gegeben über die API: `beta` sendet Anhänge und löscht sie (R00029 F0116), A lädt den Verlauf neu und sieht die Zeile
// „foto.png · 300 KiB · Gelöscht“ — ohne Links, Vorschau, Player und reservierten Platz.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, pngMitBytes, schriftKontrast } = require('./r00041-hilfen');
const { anzeigeStarten, anhaengeLoeschen, testdateien } = require('./r00030-anzeige-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const foto = () => ({ name: 'foto.png', inhalt: pngMitBytes(307200) });

/// A lädt den Verlauf neu: „Gelöscht“ steht dann aus dem Zustand der API da, nicht aus einem Ereignis (B0518 kommt später).
async function neuLaden(page, zeit) {
  await page.reload();
  const knoten = nachrichtMitZeit(page, zeit);
  await knoten.waitFor();
  return knoten;
}

/// Der sichtbare Text der Zeile ohne das Typkürzel.
const sichtbarerText = (zeile) => zeile.locator('.lc-datei-kopf').evaluate(
  (kopf) => [...kopf.children].filter((kind) => !kind.classList.contains('lc-datei-kuerzel')).map((kind) => kind.textContent).join(''),
);

test('Wortlaut (UX V4): die Zeile lautet „foto.png · 300 KiB · Gelöscht“, das Kürzel ist aria-hidden, der Name der Zeile ist „foto.png, 300 KiB, Gelöscht“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: [foto(), 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);
  const knoten = await neuLaden(lauf.a.page, lauf.zeiten[0]);

  const zeile = knoten.getByRole('group', { name: 'foto.png, 300 KiB, Gelöscht', exact: true });
  await expect(zeile).toBeVisible();
  await expect(zeile).toHaveAccessibleName('foto.png, 300 KiB, Gelöscht');
  expect(await sichtbarerText(zeile)).toBe('foto.png · 300 KiB · Gelöscht');
  await expect(zeile.locator('.lc-datei-kuerzel')).toHaveText('PNG');
  await expect(zeile.locator('.lc-datei-kuerzel')).toHaveAttribute('aria-hidden', 'true');
  expect(lauf.a.fehler).toEqual([]);
});

test('Zeile ohne Bedienelemente (UX V4): weder Link noch Knopf noch fokussierbar, ohne Vorschau und Bildplatz; kein Tab-Stopp, „bericht.pdf“ bleibt mit „Öffnen“ und „Laden“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: [foto(), 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);
  const knoten = await neuLaden(page, lauf.zeiten[0]);

  const zeile = knoten.getByRole('group', { name: 'foto.png, 300 KiB, Gelöscht', exact: true });
  await expect(zeile.locator('a, button, [tabindex], input, img, video, audio')).toHaveCount(0);
  await expect(knoten.locator('.lc-vorschau, .lc-vorschau-platz, img')).toHaveCount(0);
  await zeile.evaluate((element) => element.focus());
  expect(await zeile.evaluate((element) => element.contains(document.activeElement))).toBe(false);
  await knoten.focus();
  const erreicht = [];
  for (let schritt = 0; schritt < 4; schritt += 1) {
    await page.keyboard.press('Tab');
    erreicht.push(await page.evaluate(() => document.activeElement.getAttribute('aria-label')));
  }
  expect(erreicht.slice(0, 2)).toEqual(['bericht.pdf öffnen', 'bericht.pdf laden']);
  expect(erreicht).not.toContain('foto.png öffnen');
  expect(await zeile.evaluate((element) => element.contains(document.activeElement))).toBe(false);
  const pdf = knoten.locator('.lc-datei', { has: page.locator('.lc-datei-name', { hasText: 'bericht.pdf' }) });
  await expect(pdf.getByRole('link')).toHaveCount(2);
  expect(lauf.a.fehler).toEqual([]);
});

test('Darstellung (F0118): Bild, Video, Ton und PDF gelöscht → je eine Zeile „Gelöscht“, ohne Links, Vorschau und Player', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: [foto(), 'film.webm', 'ton.ogg', 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, lauf.ids[0]);
  const knoten = await neuLaden(lauf.a.page, lauf.zeiten[0]);

  await expect(knoten.getByRole('group').filter({ hasText: 'Gelöscht' })).toHaveCount(4);
  await expect(knoten.locator('.lc-anhaenge').locator('a, button, video, audio, img, .lc-player, .lc-vorschau')).toHaveCount(0);
  await expect(knoten.getByRole('group', { name: /^film\.webm, .*, Gelöscht$/ })).toHaveCount(1);
  await expect(knoten.getByRole('group', { name: /^ton\.ogg, .*, Gelöscht$/ })).toHaveCount(1);
  await expect(knoten.getByRole('group', { name: /^bericht\.pdf, 2,3 MiB, Gelöscht$/ })).toHaveCount(1);
  expect(lauf.a.fehler).toEqual([]);
});

for (const thema of ['light', 'dark']) {
  test(`Lesbarkeit (UX V4, ${thema}): „Gelöscht“ in kd-ink-muted, nicht kd-accent, nicht nur durchgestrichen, Kontrast mindestens 4,5 : 1`, async ({ browser }) => {
    const lauf = await anzeigeStarten(browser, [{ namen: [foto()] }]);
    instanz = lauf.instanz;
    const { page } = lauf.a;
    await anhaengeLoeschen(instanz.adresse, lauf.beta.token, lauf.ids[0]);
    await page.emulateMedia({ colorScheme: thema });
    const knoten = await neuLaden(page, lauf.zeiten[0]);

    await expect(page.locator('html')).toHaveAttribute('data-theme', thema);
    const marke = knoten.getByRole('group').getByText('Gelöscht', { exact: true });
    await expect(marke).toBeVisible();
    const farben = await marke.evaluate((element) => {
      const gemessen = (wert) => {
        const probe = Object.assign(document.createElement('span'), { textContent: 'x' });
        probe.style.color = wert;
        element.parentElement.append(probe);
        const farbe = getComputedStyle(probe).color;
        probe.remove();
        return farbe;
      };
      const stil = getComputedStyle(element);
      return { ist: stil.color, muted: gemessen('var(--kd-ink-muted)'), akzent: gemessen('var(--kd-accent)'), linie: stil.textDecorationLine, opazitaet: stil.opacity };
    });
    expect(farben.ist).toBe(farben.muted);
    expect(farben.ist).not.toBe(farben.akzent);
    expect(farben.linie).toBe('none');
    expect(farben.opazitaet).toBe('1');
    expect(await schriftKontrast(marke)).toBeGreaterThanOrEqual(4.5);
    expect(lauf.a.fehler).toEqual([]);
  });
}

test('Zugänglicher Name der Nachricht (UX V5): ein gelöschter Anhang heißt „<name> (gelöscht)“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: [foto(), 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);
  const knoten = await neuLaden(lauf.a.page, lauf.zeiten[0]);

  await expect(knoten).toHaveAttribute('aria-label', /: 2 Anhänge: foto\.png \(gelöscht\), bericht\.pdf$/);
  expect(lauf.a.fehler).toEqual([]);
});

for (const breite of [320, 375]) {
  test(`Schmal (${breite} px, Touch): die Zeile bricht um statt zu überlaufen — Name mindestens 96 px, „Gelöscht“ ganz im Rahmen, kein Querscrollen`, async ({ browser }) => {
    const geraet = { viewport: { width: breite, height: 568 }, hasTouch: true, isMobile: true };
    const name = 'bericht mit einem sehr langen Namen aus dem dritten Quartal.pdf';
    const lauf = await anzeigeStarten(browser, [{ namen: [foto(), { name, inhalt: testdateien()['bericht.pdf'] }] }], { geraet });
    instanz = lauf.instanz;
    const { page } = lauf.a;
    await anhaengeLoeschen(instanz.adresse, lauf.beta.token, lauf.ids[0]);
    const knoten = await neuLaden(page, lauf.zeiten[0]);

    for (const zeile of await knoten.getByRole('group').all()) {
      const lage = await zeile.evaluate((element) => {
        const rahmen = element.getBoundingClientRect();
        const marke = element.querySelector('.lc-datei-geloescht-marke').getBoundingClientRect();
        const nameBreite = element.querySelector('.lc-datei-name').getBoundingClientRect().width;
        return { links: rahmen.left, rechts: rahmen.right, markeRechts: marke.right, markeLinks: marke.left, nameBreite };
      });
      expect(lage.nameBreite).toBeGreaterThanOrEqual(96);
      expect(lage.markeLinks).toBeGreaterThanOrEqual(lage.links - 1);
      expect(lage.markeRechts).toBeLessThanOrEqual(lage.rechts + 1);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(lauf.a.fehler).toEqual([]);
  });
}
