'use strict';
// R00030 F0165 „Video und Audio im Player“ (B0460; Desktop 1280 × 800 und Telefon 320 px, Gegeben über die API):
// Fehlerzustand, billig beim Öffnen, Schmal (V1), Rollfokus und Tasten im Player (V2). Dass der Player in beiden Browsern
// spielt, prüft r00030-player-spielt.spec.js.
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, mitAnhaengenSenden } = require('./r00041-hilfen');
const { eingabe } = require('./senden-hilfen');
const { anzeigeStarten, testdateien } = require('./r00030-anzeige-hilfen');
const { player, abspielen, wiedergabe, anhangAnfragenMitschreiben } = require('./r00030-player-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const rechteck = (locator) => locator.evaluate((element) => {
  const { left, right, top, bottom, width, height } = element.getBoundingClientRect();
  return { left, right, top, bottom, width, height };
});
const ueberschneiden = (eins, zwei) => eins.left < zwei.right && zwei.left < eins.right && eins.top < zwei.bottom && zwei.top < eins.bottom;
const FEHLERTEXT = 'Kann im Browser nicht abgespielt werden — „Laden“ nutzen.';

test('Fehlerzustand (B0460): kann der Browser das Video nicht abspielen (error), steht „Kann im Browser nicht abgespielt werden — „Laden“ nutzen.“ unter dem Bild; die Höhe der Nachricht bleibt', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: ['film.webm'] }], {
    vorSeite: (page) => page.context().route('**/anhaenge/**', (route) => route.fulfill({ status: 200, contentType: 'video/webm', body: 'kein Video' })),
  });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  const video = player(knoten, 'film.webm');
  await expect(knoten.getByText(FEHLERTEXT, { exact: true })).toBeHidden();
  const hoeheVorher = (await rechteck(knoten)).height;
  await video.evaluate((element) => {
    element.muted = true;
    element.play().catch(() => {});
  });
  await expect.poll(async () => (await wiedergabe(video)).fehler, { timeout: 10000 }).not.toBeNull();
  const meldung = knoten.getByText(FEHLERTEXT, { exact: true });
  await expect(meldung).toBeVisible();
  const flaeche = await rechteck(knoten.locator('.lc-player'));
  const text = await rechteck(meldung);
  expect(text.top).toBeGreaterThanOrEqual(flaeche.top);
  expect(text.bottom).toBeLessThanOrEqual(flaeche.bottom + 0.5);
  expect(Math.abs((await rechteck(knoten)).height - hoeheVorher)).toBeLessThanOrEqual(0.5);
  await expect(knoten.getByText('Gelöscht', { exact: true })).toHaveCount(0);
  expect(lauf.a.fehler).toEqual([]);
});

test('Billig beim Öffnen (B0460, QS5): Channel mit 20 Video- und Audionachrichten → keine Anfrage an /anhaenge/ beim Öffnen und Scrollen', async ({ browser }) => {
  test.setTimeout(90000);
  const nachrichten = Array.from({ length: 20 }, (_, nummer) => ({ text: `Medium ${nummer + 1}`, namen: [['film.webm', 'ton.ogg'][nummer % 2]] }));
  let anfragen = null;
  const lauf = await anzeigeStarten(browser, nachrichten, {
    vorSeite: (page) => {
      anfragen = anhangAnfragenMitschreiben(page);
    },
  });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  await expect(page.getByRole('log').locator('video, audio')).toHaveCount(20);
  const log = page.getByRole('log');
  for (let schritt = 0; schritt < 30; schritt += 1) {
    await log.evaluate((element) => element.scrollBy(0, -400));
    await page.waitForTimeout(50);
  }
  await log.evaluate((element) => { element.scrollTop = element.scrollHeight; });
  await page.waitForTimeout(500);
  expect(anfragen).toEqual([]);
  expect(lauf.a.fehler).toEqual([]);
});

test('Schmal (B0460, V1) bei 320 px Touch: Video so breit wie der Inhalt neben der Reserve, 16 : 9 (floor), Audio höchstens so breit, ≥ 40 px hoch und vor wie nach dem Abspielen gleich hoch; kein Querscrollen; keine Überschneidung mit „⋯“ an der eigenen Nachricht', async ({ browser }) => {
  const telefon = { viewport: { width: 320, height: 812 }, hasTouch: true, isMobile: true, deviceScaleFactor: 3 };
  const lauf = await anzeigeStarten(browser, [{ namen: ['ton.ogg'] }], { geraet: telefon });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const token = await page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  const eigeneZeit = await mitAnhaengenSenden(instanz.adresse, token, '', [{ name: 'film.webm', inhalt: testdateien()['film.webm'] }], '127.0.0.1');
  const eigene = nachrichtMitZeit(page, eigeneZeit);
  const video = player(eigene, 'film.webm');
  await expect(video).toHaveCount(1);
  const inhaltsbreite = (await rechteck(eigene.locator('.lc-msg-inhalt'))).width;
  const rahmen = Math.min(360, inhaltsbreite);
  test.info().annotations.push({ type: 'Inhaltsbreite', description: `${inhaltsbreite} px, Rahmen ${rahmen} × ${Math.floor((rahmen * 9) / 16)} px` });
  const flaeche = await rechteck(video);
  expect(Math.abs(flaeche.width - rahmen)).toBeLessThanOrEqual(1);
  expect(Math.abs(flaeche.height - Math.floor((rahmen * 9) / 16))).toBeLessThanOrEqual(1);
  const schalter = await rechteck(eigene.getByRole('button', { name: 'Aktionen', exact: true }));
  expect(ueberschneiden(flaeche, schalter)).toBe(false);
  expect(flaeche.right).toBeLessThanOrEqual(schalter.left);

  const fremde = nachrichtMitZeit(page, lauf.zeiten[0]);
  const audio = player(fremde, 'ton.ogg');
  const audioInhalt = (await rechteck(fremde.locator('.lc-msg-inhalt'))).width;
  const zeileVorher = await rechteck(audio);
  expect(zeileVorher.width).toBeLessThanOrEqual(Math.min(360, audioInhalt) + 0.5);
  expect(zeileVorher.height).toBeGreaterThanOrEqual(40);
  await abspielen(audio);
  expect(Math.abs((await rechteck(audio)).height - zeileVorher.height)).toBeLessThanOrEqual(1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  expect(await page.getByRole('log').evaluate((log) => log.scrollWidth <= log.clientWidth)).toBe(true);
  expect(lauf.a.fehler).toEqual([]);
});

/// Steht der Fokus im Player (oder einem seiner Bedienelemente), dessen Name; sonst null.
const fokusImPlayer = (page) => page.evaluate(() => {
  const element = document.activeElement;
  if (element === null || !element.matches('video, audio')) {
    return null;
  }
  return `${element.closest('.lc-msg').dataset.zeit} ${element.getAttribute('aria-label')}`;
});

test('Rollfokus (B0460, B0732): der Player einer nicht fokussierten Nachricht ist kein Tab-Stopp — weder von der Eingabe rückwärts noch von der fokussierten vorwärts; an der fokussierten erreicht Tab den Player, danach „film.webm laden“', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'Erstes', namen: ['film.webm'] }, { text: 'Zweites', namen: ['film.webm'] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const [erste, zweite] = lauf.zeiten;
  await nachrichtMitZeit(page, erste).focus();
  const vorwaerts = [];
  for (let schritt = 0; schritt < 12; schritt += 1) {
    await page.keyboard.press('Tab');
    vorwaerts.push(await fokusImPlayer(page));
  }
  expect(vorwaerts).toContain(`${erste} film.webm`);
  expect(vorwaerts).not.toContain(`${zweite} film.webm`);
  const nachDemPlayer = vorwaerts.lastIndexOf(`${erste} film.webm`);
  await nachrichtMitZeit(page, erste).focus();
  for (let schritt = 0; schritt <= nachDemPlayer + 1; schritt += 1) {
    await page.keyboard.press('Tab');
  }
  expect(await page.evaluate(() => document.activeElement.getAttribute('aria-label'))).toBe('film.webm laden');

  await nachrichtMitZeit(page, erste).focus();
  await eingabe(page).focus();
  const rueckwaerts = [];
  for (let schritt = 0; schritt < 12; schritt += 1) {
    await page.keyboard.press('Shift+Tab');
    rueckwaerts.push(await fokusImPlayer(page));
  }
  expect(rueckwaerts).not.toContain(`${zweite} film.webm`);
  expect(lauf.a.fehler).toEqual([]);
});

test('Ende im Player vor dem Beginn der Wiedergabe (PO 178, B0752): Fokus im Player eines nie gespielten Videos, Ende → nichts wirkt (Stelle 0, Verlauf scrollt nicht, Fokus bleibt) und es entsteht keine Anfrage an /anhaenge/', async ({ browser }) => {
  const danach = Array.from({ length: 30 }, (_, nummer) => ({ text: `Danach ${nummer + 1}`, namen: [] }));
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: ['film.webm'] }, ...danach]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const log = page.getByRole('log');
  const video = player(nachrichtMitZeit(page, lauf.zeiten[0]), 'film.webm');
  await video.scrollIntoViewIfNeeded();
  const anfragen = anhangAnfragenMitschreiben(page);
  await video.focus();
  const scrollVorher = await log.evaluate((element) => element.scrollTop);

  await page.keyboard.press('End');
  await page.waitForTimeout(500);

  expect((await wiedergabe(video)).zeit).toBe(0);
  expect(await video.evaluate((element) => element.readyState)).toBe(0);
  expect(anfragen).toEqual([]);
  expect(await log.evaluate((element) => element.scrollTop)).toBe(scrollVorher);
  expect(await page.evaluate(() => document.activeElement.getAttribute('aria-label'))).toBe('film.webm');
  expect(lauf.a.fehler).toEqual([]);
});

test('Tasten im Player (B0460, V2): Fokus im Player nach begonnener Wiedergabe, Ende → currentTime am Ende, Pos1 → Anfang, Leertaste → Wiedergabe an/aus, Pfeil; der Verlauf scrollt nicht und verhindert nichts', async ({ browser }) => {
  const danach = Array.from({ length: 30 }, (_, nummer) => ({ text: `Danach ${nummer + 1}`, namen: [] }));
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: ['film.webm'] }, ...danach]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const log = page.getByRole('log');
  const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
  const video = player(knoten, 'film.webm');
  await video.scrollIntoViewIfNeeded();
  await knoten.focus();
  await abspielen(video);
  await video.evaluate((element) => element.pause());
  await page.evaluate(() => {
    window.verhindert = [];
    window.addEventListener('keydown', (ereignis) => window.verhindert.push(`${ereignis.key}:${ereignis.defaultPrevented}`));
  });
  await video.focus();
  const scrollVorher = await log.evaluate((element) => element.scrollTop);

  await page.keyboard.press('End');
  await expect.poll(async () => {
    const { zeit, dauer } = await wiedergabe(video);
    return dauer - zeit;
  }, { timeout: 5000 }).toBeLessThanOrEqual(0.2);
  await page.keyboard.press('Home');
  await expect.poll(async () => (await wiedergabe(video)).zeit, { timeout: 5000 }).toBeLessThanOrEqual(0.1);
  await page.keyboard.press('Space');
  await expect.poll(async () => (await wiedergabe(video)).pausiert, { timeout: 5000 }).toBe(false);
  await page.keyboard.press('Space');
  await expect.poll(async () => (await wiedergabe(video)).pausiert, { timeout: 5000 }).toBe(true);
  await page.keyboard.press('ArrowRight');
  await page.waitForTimeout(200);

  expect(await log.evaluate((element) => element.scrollTop)).toBe(scrollVorher);
  expect(await page.evaluate(() => window.verhindert.filter((eintrag) => eintrag.endsWith(':true')).length)).toBe(0);
  expect(await page.evaluate(() => document.activeElement.getAttribute('aria-label'))).toBe('film.webm');
  expect(lauf.a.fehler).toEqual([]);
});
