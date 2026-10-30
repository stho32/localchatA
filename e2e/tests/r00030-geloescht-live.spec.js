'use strict';
// R00030 F0118 „Gelöschte Anhänge im Verlauf“, Teil B0518 (Löschen im Verlauf folgen; UX V4, S2 des code-reviewer #43),
// Desktop 1280 × 800, Gegeben über die API: `beta` sendet Anhänge und löscht sie; A (`shoff`) und B (`anna`) haben den
// Channel offen und sehen die Zeile „Gelöscht“ weniger als 1 s später — ohne Neuladen, mit Fokus auf der Nachricht.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, browserOeffnen, ansagenMitschreiben, bearbeitenPerApi } = require('./r00041-hilfen');
const { imBrowserBeitreten } = require('./senden-hilfen');
const { sichtbarNach } = require('./hilfen');
const { netzAus, netzAn } = require('./r00012-hilfen');
const { anzeigeStarten, anhaengeLoeschen } = require('./r00030-anzeige-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Prüfung für sichtbarNach: die Nachricht `zeit` zeigt `anzahl` Zeilen „Gelöscht“.
function zeigtGeloeschtZeilen({ zeit, anzahl }) {
  const knoten = document.querySelector(`.lc-log .lc-msg[data-zeit="${zeit}"]`);
  return knoten !== null && knoten.querySelectorAll('.lc-datei-geloescht').length === anzahl;
}

const zeileVon = (knoten, name) => knoten.getByRole('group', { name: new RegExp(`^${name.replace('.', '\\.')}, .*, Gelöscht$`) });

/// Der Fokus: die Zeit der fokussierten Nachricht, sonst das Tag des fokussierten Elements.
const fokusAn = (page) => page.evaluate(() => {
  const aktiv = document.activeElement;
  if (aktiv.classList.contains('lc-msg')) {
    return `Nachricht ${aktiv.dataset.zeit}`;
  }
  return aktiv.tagName;
});

async function mitB(browser, adresse, zeit) {
  const b = await browserOeffnen(browser);
  await imBrowserBeitreten(b.page, adresse, 'anna');
  await nachrichtMitZeit(b.page, zeit).waitFor();
  return b;
}

const text = (nummer) => ({ text: `Nachricht ${nummer}\nzweite Zeile\ndritte Zeile`, namen: [] });

test('Live gelöscht: foto.png und film.webm gelöscht → bei A und B binnen 1 s je eine Zeile statt Vorschau und Player, bericht.pdf bleibt; die Höhe ändert sich einmal, die Lesestelle bleibt, keine Anfrage an /anhaenge/ danach; nach dem Neuladen dieselbe Darstellung', async ({ browser }) => {
  const vorher = Array.from({ length: 10 }, (_, nummer) => text(nummer));
  const nachher = Array.from({ length: 8 }, (_, nummer) => text(20 + nummer));
  const lauf = await anzeigeStarten(browser, [...vorher, { namen: ['foto.png', 'film.webm', 'bericht.pdf'] }, ...nachher]);
  instanz = lauf.instanz;
  const zeit = lauf.zeiten[10];
  const [fotoId, filmId] = lauf.ids[10];
  const b = await mitB(browser, instanz.adresse, zeit);
  for (const seite of [lauf.a, b]) {
    const knoten = nachrichtMitZeit(seite.page, zeit);
    await knoten.scrollIntoViewIfNeeded();
    await expect(knoten.locator('.lc-vorschau[data-zustand="geladen"]')).toHaveCount(1);
    await seite.page.getByRole('log').evaluate((log) => { log.scrollTop -= 100; });
    seite.hoehen = await knoten.evaluate((element) => {
      window.hoehen = [element.getBoundingClientRect().height];
      new ResizeObserver(() => window.hoehen.push(element.getBoundingClientRect().height)).observe(element);
      return window.hoehen;
    });
    seite.oben = (await knoten.boundingBox()).y;
    seite.anhangAnfragen = [];
    seite.page.context().on('request', (anfrage) => {
      const pfad = new URL(anfrage.url()).pathname;
      if ([`/anhaenge/${fotoId}`, `/anhaenge/${filmId}`].includes(pfad)) {
        seite.anhangAnfragen.push(pfad);
      }
    });
  }

  const geloescht = await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [fotoId, filmId]);

  for (const seite of [lauf.a, b]) {
    const ms = await sichtbarNach(seite.page, zeigtGeloeschtZeilen, { argument: { zeit, anzahl: 2 }, ab: geloescht.empfangen, timeout: 10000 });
    expect(ms).toBeLessThan(1000);
    const knoten = nachrichtMitZeit(seite.page, zeit);
    await expect(zeileVon(knoten, 'foto.png')).toBeVisible();
    await expect(zeileVon(knoten, 'film.webm')).toBeVisible();
    await expect(knoten.locator('.lc-vorschau, img, video, audio')).toHaveCount(0);
    await expect(knoten.getByRole('link', { name: 'bericht.pdf öffnen' })).toBeVisible();
    await expect(knoten.getByRole('link', { name: 'bericht.pdf laden' })).toBeVisible();
    await expect(knoten.getByRole('link', { name: /foto\.png|film\.webm/ })).toHaveCount(0);
    const hoehen = await seite.page.evaluate(() => window.hoehen);
    const wechsel = hoehen.filter((wert, stelle) => stelle > 0 && Math.abs(wert - hoehen[stelle - 1]) > 0.5);
    expect(wechsel.length, `Höhenwechsel ${hoehen}`).toBe(1);
    expect(Math.abs((await knoten.boundingBox()).y - seite.oben), 'Lesestelle').toBeLessThanOrEqual(1);
    await seite.page.waitForTimeout(500);
    expect(seite.anhangAnfragen).toEqual([]);
    expect(seite.fehler).toEqual([]);
  }

  await lauf.a.page.reload();
  const neu = nachrichtMitZeit(lauf.a.page, zeit);
  await expect(zeileVon(neu, 'foto.png')).toBeVisible();
  await expect(zeileVon(neu, 'film.webm')).toBeVisible();
  await expect(neu.locator('.lc-vorschau, img, video, audio')).toHaveCount(0);
  await b.kontext.close();
});

const FOKUSORTE = [
  ['die Vorschau', (knoten) => knoten.getByRole('button', { name: 'foto.png groß anzeigen', exact: true })],
  ['„foto.png öffnen“', (knoten) => knoten.getByRole('link', { name: 'foto.png öffnen', exact: true })],
  ['„foto.png laden“', (knoten) => knoten.getByRole('link', { name: 'foto.png laden', exact: true })],
];
for (const [ort, finden] of FOKUSORTE) {
  test(`Fokus und Ansage (UX V4): Fokus auf ${ort}, foto.png wird gelöscht → Fokus auf der Nachricht, nie body; die Ansage sagt genau einmal „Anhang gelöscht: foto.png“`, async ({ browser }) => {
    const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png', 'bericht.pdf'] }]);
    instanz = lauf.instanz;
    const { page } = lauf.a;
    const zeit = lauf.zeiten[0];
    const knoten = nachrichtMitZeit(page, zeit);
    await finden(knoten).focus();
    const ansagen = await ansagenMitschreiben(page);

    await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);

    await expect(zeileVon(knoten, 'foto.png')).toBeVisible();
    expect(await fokusAn(page)).toBe(`Nachricht ${zeit}`);
    await expect.poll(ansagen, { intervals: [25] }).toContain('Anhang gelöscht: foto.png');
    await page.waitForTimeout(600);
    expect((await ansagen()).filter((einzeln) => einzeln.includes('Anhang gelöscht'))).toEqual(['Anhang gelöscht: foto.png']);
    expect(lauf.a.fehler).toEqual([]);
  });
}

test('Fokus und Ansage (UX V4): Fokus im Player von film.webm, der Anhang wird gelöscht → Fokus auf der Nachricht, einmal „Anhang gelöscht: film.webm“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['film.webm', 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const zeit = lauf.zeiten[0];
  const knoten = nachrichtMitZeit(page, zeit);
  await knoten.locator('video').focus();
  const ansagen = await ansagenMitschreiben(page);

  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);

  await expect(zeileVon(knoten, 'film.webm')).toBeVisible();
  expect(await fokusAn(page)).toBe(`Nachricht ${zeit}`);
  await expect.poll(ansagen, { intervals: [25] }).toEqual(['Anhang gelöscht: film.webm']);
  expect(lauf.a.fehler).toEqual([]);
});

test('Fokus woanders (UX V4): Fokus in der Eingabe → keine Ansage, der Fokus bleibt; Fokus auf „bericht.pdf öffnen“ derselben Nachricht → keine Ansage, der Fokus liegt danach nicht auf body', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png', 'bericht.pdf', 'archiv.zip'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const zeit = lauf.zeiten[0];
  const knoten = nachrichtMitZeit(page, zeit);
  const eingabe = page.getByRole('textbox', { name: /^Nachricht an # / });
  await eingabe.focus();
  const ansagen = await ansagenMitschreiben(page);

  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);
  await expect(zeileVon(knoten, 'foto.png')).toBeVisible();
  await page.waitForTimeout(500);
  await expect(eingabe).toBeFocused();
  expect((await ansagen()).filter((einzeln) => einzeln.includes('Anhang gelöscht'))).toEqual([]);

  await knoten.getByRole('link', { name: 'bericht.pdf öffnen', exact: true }).focus();
  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][2]]);
  await expect(zeileVon(knoten, 'archiv.zip')).toBeVisible();
  await page.waitForTimeout(500);
  expect(await fokusAn(page)).not.toBe('BODY');
  expect((await ansagen()).filter((einzeln) => einzeln.includes('Anhang gelöscht'))).toEqual([]);
  expect(lauf.a.fehler).toEqual([]);
});

test('Bild-Dialog (UX V4): der Dialog von foto.png ist offen, foto.png wird gelöscht → er bleibt offen; nach Esc liegt der Fokus auf der Nachricht (die Zeile ist nicht fokussierbar), nie auf body', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png', 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const zeit = lauf.zeiten[0];
  const knoten = nachrichtMitZeit(page, zeit);
  await knoten.getByRole('button', { name: 'foto.png groß anzeigen', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'foto.png', exact: true });
  await expect(dialog).toBeVisible();
  const ansagen = await ansagenMitschreiben(page);

  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);

  await expect(zeileVon(knoten, 'foto.png')).toBeAttached();
  await page.waitForTimeout(500);
  await expect(dialog).toBeVisible();
  expect((await ansagen()).filter((einzeln) => einzeln.includes('Anhang gelöscht'))).toEqual([]);
  await page.keyboard.press('Escape');
  await expect(page.locator('dialog')).toHaveCount(0);
  expect(await fokusAn(page)).toBe(`Nachricht ${zeit}`);
  expect(lauf.a.fehler).toEqual([]);
});

test('Bild-Dialog (code-reviewer #43 F0102 S2): der Dialog ist offen, die Nachricht wird per API bearbeitet → Esc legt den Fokus auf die neue Vorschau derselben Nachricht, nie auf body', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'alt', namen: ['foto.png'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const zeit = lauf.zeiten[0];
  const knoten = nachrichtMitZeit(page, zeit);
  const vorschau = knoten.getByRole('button', { name: 'foto.png groß anzeigen', exact: true });
  await vorschau.click();
  await expect(page.getByRole('dialog', { name: 'foto.png', exact: true })).toBeVisible();
  await elementMerken(vorschau);

  const bearbeitet = await bearbeitenPerApi(instanz.adresse, lauf.beta.token, zeit, 'neu');
  expect(bearbeitet.status, JSON.stringify(bearbeitet.daten)).toBe(200);
  await expect(knoten.locator('.lc-body')).toHaveText('neu');
  expect(await vorschau.evaluate((element) => element === window.alteVorschau)).toBe(false);
  await page.keyboard.press('Escape');

  await expect(page.locator('dialog')).toHaveCount(0);
  await expect(vorschau).toBeFocused();
  expect(lauf.a.fehler).toEqual([]);
});

async function elementMerken(locator) {
  await locator.evaluate((element) => { window.alteVorschau = element; });
}

test('Beim Nachholen: B verliert die Verbindung, beta löscht foto.png, B ist wieder online → nach „Wieder verbunden“ zeigt B die Zeile „Gelöscht“ statt der Vorschau, ohne Neuladen', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png', 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  const zeit = lauf.zeiten[0];
  const b = await browserOeffnen(browser, { ohneWorker: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const knoten = nachrichtMitZeit(b.page, zeit);
  await expect(knoten.locator('.lc-vorschau[data-zustand="geladen"]')).toHaveCount(1);
  await netzAus(b);

  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);
  await b.page.waitForTimeout(500);
  await expect(knoten.locator('.lc-vorschau')).toHaveCount(1);
  await netzAn(b);

  await expect(zeileVon(knoten, 'foto.png')).toBeVisible({ timeout: 20000 });
  await expect(knoten.locator('.lc-vorschau, img')).toHaveCount(0);
  await b.kontext.close();
});

test('Abruf unterwegs: der Verlauf wird geladen, beta löscht foto.png, bevor die (alte) Antwort ankommt → nach der Antwort steht die Zeile „Gelöscht“ — der Tab hat das Ereignis vermerkt', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png', 'bericht.pdf'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const zeit = lauf.zeiten[0];
  let loslassen;
  const tor = new Promise((frei) => { loslassen = frei; });
  let gehalten = false;
  await page.route(/\/api\/channels\/allgemein\/nachrichten\?/, async (route) => {
    if (route.request().method() !== 'GET' || gehalten) {
      await route.continue();
      return;
    }
    gehalten = true;
    const antwort = await route.fetch();
    await tor;
    await route.fulfill({ response: antwort });
  });
  await page.reload();
  await expect.poll(() => gehalten).toBe(true);

  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][0]]);
  await page.waitForTimeout(1200);
  loslassen();

  await expect(zeileVon(nachrichtMitZeit(page, zeit), 'foto.png')).toBeVisible();
  expect(lauf.a.fehler).toEqual([]);
});

test('Nach dem Neuladen keine Ansage (UX S9): „Gelöscht“ steht da, die Region sagt nichts', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ namen: ['foto.png'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  await anhaengeLoeschen(instanz.adresse, lauf.beta.token, lauf.ids[0]);
  await page.reload();
  await page.locator('[role="status"][aria-live="polite"]').first().waitFor({ state: 'attached' });
  const ansagen = await ansagenMitschreiben(page);
  await expect(zeileVon(nachrichtMitZeit(page, lauf.zeiten[0]), 'foto.png')).toBeVisible();
  await page.waitForTimeout(600);
  expect((await ansagen()).filter((einzeln) => einzeln.includes('Anhang gelöscht'))).toEqual([]);
});
