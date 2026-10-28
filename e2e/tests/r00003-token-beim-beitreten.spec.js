'use strict';
// R00003 F0010, Nacharbeit 2 (Code-Review A2): „Beitreten“ im Fehlerzustand und „Erneut versuchen“ schicken
// das gemerkte Token mit — lebt die Sitzung noch, kehrt sie wieder, statt den eigenen Namen als vergeben zu zeigen.
const { test, expect } = require('@playwright/test');
const { instanzStarten } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const kopfzeile = (page) => page.getByRole('banner').locator('.lc-me');
const gemerktesToken = (page) => page.evaluate(() => window.localStorage.getItem('localchatA.sitzung'));

async function beigetretenAlsX(page) {
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').fill('x');
  await page.getByLabel('Name').press('Enter');
  await expect(kopfzeile(page)).toHaveText('x@127.0.0.1');
  return gemerktesToken(page);
}

function beitrittsantwortenZaehlen(page) {
  const status = [];
  page.on('response', (antwort) => {
    if (antwort.url().endsWith('/api/beitreten')) {
      status.push(antwort.status());
    }
  });
  return status;
}

const faelle = [
  {
    zustand: 'Fehler (ich → 500)',
    stoeren: (route) => route.fulfill({ status: 500, json: { fehler: 'Probe: interner Fehler' } }),
    erwarteteMeldung: '⚠ Probe: interner Fehler',
    schalter: 'Beitreten',
  },
  {
    zustand: 'Server weg (ich unerreichbar)',
    stoeren: (route) => route.abort('connectionrefused'),
    erwarteteMeldung: '⚠ Server nicht erreichbar.',
    schalter: 'Erneut versuchen',
  },
];

for (const { zustand, stoeren, erwarteteMeldung, schalter } of faelle) {
  test(`A2: ${zustand}, Sitzung lebt — „${schalter}“ führt zur Kopfzeile mit demselben Token, kein 409`, async ({ page }) => {
    instanz = await instanzStarten();
    const token = await beigetretenAlsX(page);
    await page.route('**/api/ich', stoeren);
    await page.reload();
    await expect(page.locator('#lc-meldung')).toContainText(erwarteteMeldung);
    await page.unroute('**/api/ich');
    const beitritte = beitrittsantwortenZaehlen(page);

    await page.getByRole('button', { name: schalter }).click();

    await expect(kopfzeile(page)).toHaveText('x@127.0.0.1');
    expect(await gemerktesToken(page)).toBe(token);
    expect(beitritte).toEqual([200]);
  });
}
