'use strict';
// R00030 F0165 Player beim Ersetzen der Nachricht (B0734; Stand R00041; UX V3; US-22, US-26), Desktop 1280 × 800, Gegeben
// über die API: bearbeitet spielt derselbe Player weiter — auch während die eigene Nachricht im Feld steht; gelöscht
// hält er an und verschwindet, und das Folgen gilt wieder (B0733).
const { test, expect } = require('./rahmen');
const {
  nachrichtMitZeit, bearbeitenPerApi, loeschenPerApi, mitAnhaengenSenden, zeigtBearbeitet, zeigtGeloescht, ansagenMitschreiben,
} = require('./r00041-hilfen');
const { sichtbarNach } = require('./hilfen');
const { anzeigeStarten } = require('./r00030-anzeige-hilfen');
const {
  player, abspielen, wiedergabe, elementMerken, istDasselbe, anhangAnfragenMitschreiben, aeltereNachrichten, betaSchreibt, scrollStand, langesVideo,
} = require('./r00030-player-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Derselbe Player spielt weiter: gleiches Element, nicht pausiert, die Zeit nicht zurückgesprungen. → die Zeit jetzt
async function spieltWeiter(video, zeitVorher, wo) {
  expect(await istDasselbe(video, 'gemerkterPlayer'), `${wo}: dasselbe Element`).toBe(true);
  const { pausiert, zeit } = await wiedergabe(video);
  expect(pausiert, `${wo}: paused`).toBe(false);
  expect(zeit, `${wo}: currentTime`).toBeGreaterThanOrEqual(zeitVorher);
  return zeit;
}

test('Bearbeitet während der Wiedergabe (B0734, US-22): beta sendet Video mit film.webm, B spielt es; beta bearbeitet zu „Video neu“ → < 1 s nach der 200 „Video neu“ mit „bearbeitet“, derselbe Player, paused false, currentTime nicht zurück', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: [langesVideo()] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const zeit = lauf.zeiten[0];
  const video = player(nachrichtMitZeit(page, zeit), 'film.webm');
  await abspielen(video);
  await elementMerken(video, 'gemerkterPlayer');
  const zeitVorher = (await wiedergabe(video)).zeit;
  const geaendert = await bearbeitenPerApi(instanz.adresse, lauf.beta.token, zeit, 'Video neu');
  expect(geaendert.status, JSON.stringify(geaendert.daten)).toBe(200);
  const ms = await sichtbarNach(page, zeigtBearbeitet, { argument: { zeit, text: 'Video neu' }, ab: geaendert.empfangen, timeout: 10000 });
  expect(ms).toBeLessThan(1000);
  await spieltWeiter(video, zeitVorher, 'nach dem Bearbeiten');
  expect(lauf.a.fehler).toEqual([]);
});

test('Probe V3 (B0734, US-26): B bearbeitet die eigene Nachricht mit Player — beim Öffnen, Schließen und Speichern bleibt der Player sichtbar und bedienbar, dasselbe Element, paused false, currentTime monoton, keine Ansage zum Player', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'davor', namen: [] }]);
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const token = await page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  const zeit = await mitAnhaengenSenden(instanz.adresse, token, 'Video', [langesVideo()], '127.0.0.1');
  const knoten = nachrichtMitZeit(page, zeit);
  const video = player(knoten, 'film.webm');
  await abspielen(video);
  await elementMerken(video, 'gemerkterPlayer');
  const ansagen = await ansagenMitschreiben(page);
  let zuletzt = (await wiedergabe(video)).zeit;
  const feld = knoten.getByRole('textbox', { name: 'Nachricht bearbeiten' });
  const schritte = [
    ['Öffnen', async () => {
      await knoten.hover();
      await knoten.getByRole('button', { name: 'Bearbeiten', exact: true }).click();
      await expect(feld).toBeFocused();
    }],
    ['Schließen', async () => {
      await knoten.getByRole('button', { name: 'Abbrechen', exact: true }).click();
      await expect(feld).toHaveCount(0);
    }],
    ['wieder Öffnen', async () => {
      await knoten.hover();
      await knoten.getByRole('button', { name: 'Bearbeiten', exact: true }).click();
      await expect(feld).toBeFocused();
    }],
    ['Speichern', async () => {
      await feld.fill('Video neu');
      await knoten.getByRole('button', { name: 'Speichern', exact: true }).click();
      await sichtbarNach(page, zeigtBearbeitet, { argument: { zeit, text: 'Video neu' }, ab: Date.now(), timeout: 10000 });
    }],
  ];
  for (const [wo, schritt] of schritte) {
    await schritt();
    await expect(video, wo).toBeVisible();
    zuletzt = await spieltWeiter(video, zuletzt, wo);
  }
  await video.evaluate((element) => element.pause());
  expect((await wiedergabe(video)).pausiert).toBe(true);
  expect((await ansagen()).filter((text) => /film\.webm|Video abspielen|Player|Wiedergabe/.test(text))).toEqual([]);
  expect(lauf.a.fehler).toEqual([]);
});

test('Gelöscht während der Wiedergabe (B0734, US-22): B spielt Video ganz unten ab; beta löscht → „Nachricht gelöscht“, kein <video> im Knoten, keine weitere Anfrage an /anhaenge/, die nächste Nachricht wird wieder gefolgt', async ({ browser }) => {
  const lauf = await anzeigeStarten(browser, [{ text: 'Video', namen: [langesVideo()] }], { vorbereiten: aeltereNachrichten(60) });
  instanz = lauf.instanz;
  const { page } = lauf.a;
  const zeit = lauf.zeiten[0];
  const knoten = nachrichtMitZeit(page, zeit);
  const video = player(knoten, 'film.webm');
  await expect.poll(async () => (await scrollStand(page)).untenAbstand).toBeLessThanOrEqual(1);
  await abspielen(video);
  await elementMerken(video, 'gemerkterPlayer');
  const anfragen = anhangAnfragenMitschreiben(page);
  const geloescht = await loeschenPerApi(instanz.adresse, lauf.beta.token, zeit);
  expect(geloescht.status, JSON.stringify(geloescht.daten)).toBe(200);
  await sichtbarNach(page, zeigtGeloescht, { argument: { zeit }, ab: geloescht.empfangen, timeout: 10000 });
  await expect(knoten.locator('video, audio')).toHaveCount(0);
  expect(await page.evaluate(() => window.gemerkterPlayer.paused)).toBe(true);
  const nachLoeschen = anfragen.length;
  await page.waitForTimeout(1000);
  expect(anfragen.slice(nachLoeschen)).toEqual([]);

  const danach = await betaSchreibt(lauf, 'danach');
  await expect(nachrichtMitZeit(page, danach)).toHaveCount(1);
  await expect.poll(async () => (await scrollStand(page)).untenAbstand).toBeLessThanOrEqual(1);
  expect(lauf.a.fehler).toEqual([]);
});
