'use strict';
// R00030 F0101 Nacharbeit UX-Review #43 S2 und S3 (Proben wörtlich aus dem Kommentar „ux-reviewer #43 F0101“):
// S2 „--anhangquote 10MiB, ≈ 3,5 MiB gebunden, drei Dateien à 2,9 MiB → nach 10 s Text höchstens 20 s und nennt
// „Platz“; nach 70 s Abweisung mit Text oder gesendet, nie derselbe Wartetext.“
// S3 „gross.bin + klein.bin senden → Kopfzeile ohne „wird hochgeladen“, nennt, dass es mit „Entfernen“ der
// abgewiesenen Datei weitergeht.“
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const {
  anhangStarten, betaBeitreten, perApiAnmelden, perApiAblegen, anfragenMitschreiben, waehlen, archivZip,
} = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const MIB = 1024 * 1024;
const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const zeile = (page, name) => eintrag(page).locator('.lc-hochladung-datei', { hasText: name });

// S2, Teil 1: belegt eine fremde Anmeldung den Platz, wartet die Datei, nennt den Platz und zählt herunter.
test('S2 Wartezustand zählt herunter und nennt den Platz', async ({ browser }) => {
  instanz = await anhangStarten();
  const beta = await betaBeitreten(instanz.adresse);
  await perApiAnmelden(instanz.adresse, beta, 'gehalten.bin', 10 * MIB);
  const { page, fehler } = await geraetOeffnen(browser, 'desktop', { zusaetzlich: { '/api/anhaenge': [507] } });
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');

  await waehlen(page, [archivZip('a.zip', 4 * 1024)]);
  await knopf(page).click();
  const zustand = zeile(page, 'a.zip').locator('.lc-hochladung-zustand');
  await expect(zustand).toHaveText('wartet · neuer Versuch in 30 s · Platz für Anhänge belegt');
  await expect(zustand).toHaveText(/^wartet · neuer Versuch in (2[0-7]) s · Platz für Anhänge belegt$/, { timeout: 10000 });
  await expect(zeile(page, 'a.zip').locator('.lc-hochladung-grund')).toBeHidden();
  expect(fehler).toEqual([]);
});

// S2, Teil 2 (Probe wörtlich, Lesart): halten die eigenen Anmeldungen der Nachricht den Platz, hilft Warten nicht — die
// Datei wird spätestens nach 70 s mit Text abgewiesen (in der Regel sofort, sobald die Anmeldungen der anderen Dateien
// bekannt sind), nie bleibt derselbe Wartetext stehen; „Entfernen“ sendet die übrigen.
test('S2 Platz von der eigenen Nachricht belegt: Abweisung mit Text statt endlosem Warten, „Entfernen“ sendet die übrigen', async ({ browser }) => {
  test.setTimeout(120000);
  instanz = await anhangStarten();
  await perApiAblegen(instanz.adresse, await betaBeitreten(instanz.adresse), [Math.round(3.5 * MIB)]);
  const { page, fehler } = await geraetOeffnen(browser, 'desktop', { zusaetzlich: { '/api/anhaenge': [507] } });
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  const anfragen = anfragenMitschreiben(page);
  const DATEI = Math.round(2.9 * MIB);

  await waehlen(page, ['a.zip', 'b.zip', 'c.zip'].map((name) => archivZip(name, DATEI)));
  await knopf(page).click();

  const abgewiesen = eintrag(page).locator('.lc-hochladung-datei').filter({ has: page.locator('.lc-hochladung-grund', { hasText: 'Platz für Anhänge' }) });
  await expect(abgewiesen).toHaveCount(1, { timeout: 70000 });
  const name = (await abgewiesen.locator('.lc-datei-name').textContent()).trim();
  await expect(abgewiesen.locator('.lc-hochladung-grund')).toContainText('„Entfernen“ sendet die übrigen');
  await expect(abgewiesen.locator('.lc-hochladung-zustand')).not.toContainText('wartet');
  await expect(eintrag(page).locator('.lc-hochladung-zustand', { hasText: 'wartet' })).toHaveCount(0);

  await abgewiesen.getByRole('button', { name: `${name} entfernen`, exact: true }).click();
  await expect.poll(() => anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).length, { timeout: 20000 }).toBe(1);
  expect(anfragen.find((anfrage) => anfrage.pfad.endsWith('/nachrichten')).koerper.anhaenge).toHaveLength(2);
  expect(fehler).toEqual([]);
});

test('S3 Nachricht hängt an einer abgewiesenen Datei: die Kopfzeile sagt nicht mehr „wird hochgeladen“, sondern nennt „Entfernen“', async ({ browser }) => {
  instanz = await anhangStarten();
  const { page, fehler } = await geraetOeffnen(browser, 'desktop', { zusaetzlich: { '/api/anhaenge': [422] } });
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  const anfragen = anfragenMitschreiben(page);

  await waehlen(page, [archivZip('gross.bin', 10 * MIB + 1), archivZip('klein.bin', 4 * 1024)]);
  await knopf(page).click();

  await expect(zeile(page, 'gross.bin').locator('.lc-hochladung-grund')).toContainText('⚠');
  await expect(zeile(page, 'klein.bin').locator('.lc-hochladung-zustand')).toHaveText('hochgeladen');
  const kopf = eintrag(page).locator('.lc-send').first();
  await expect(kopf).not.toContainText('wird hochgeladen');
  await expect(kopf).toContainText('nach „Entfernen“ der abgewiesenen Datei wird gesendet');

  await zeile(page, 'gross.bin').getByRole('button', { name: 'gross.bin entfernen', exact: true }).click();
  await expect.poll(() => anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).length).toBe(1);
  expect(fehler).toEqual([]);
});
