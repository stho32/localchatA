'use strict';
// R00003 F0010, Nacharbeit Code-Review B1 (ADR-0012 „alle Tabs eines Browsers teilen die Sitzung“): mehrere
// Tabs eines Browser-Kontexts laden gleichzeitig neu, nachdem die Sitzung am Server weg ist.
const { test, expect } = require('@playwright/test');
const { instanzStarten, freierPort, fehlerSammeln } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const KENNUNG = 'shoff@127.0.0.1';

async function tabsMitShoff(context, anzahl) {
  const tabs = [];
  const fehler = [];
  for (let nummer = 0; nummer < anzahl; nummer += 1) {
    const tab = await context.newPage();
    fehler.push(fehlerSammeln(tab));
    await tab.goto(instanz.adresse);
    tabs.push(tab);
  }
  const erster = tabs[0];
  await erster.getByLabel('Name').fill('shoff');
  await erster.getByLabel('Name').press('Enter');
  for (const tab of tabs) {
    if (tab !== erster) {
      await tab.reload();
    }
    await expect(tab.getByRole('banner').locator('.lc-me')).toHaveText(KENNUNG);
  }
  return { tabs, fehler };
}

async function gleichzeitigNeuLaden(tabs) {
  await Promise.all(tabs.map((tab) => tab.reload()));
}

async function alleTeilenEineSitzung(tabs, fehler, request) {
  for (const tab of tabs) {
    await expect(tab.getByRole('banner').locator('.lc-me')).toHaveText(KENNUNG);
    await expect(tab.getByLabel('Name')).toHaveCount(0);
  }
  const tokens = await Promise.all(tabs.map((tab) => tab.evaluate(() => localStorage.getItem('localchatA.sitzung'))));
  expect(new Set(tokens).size).toBe(1);
  const ich = await request.get(instanz.adresse + 'api/ich', { headers: { 'X-LocalchatA-Sitzung': tokens[0] } });
  expect(ich.status()).toBe(200);
  expect((await ich.json()).name).toBe('shoff');
  expect((await request.post(instanz.adresse + 'api/beitreten', { data: { name: 'shoff' } })).status()).toBe(409);
  expect((await request.post(instanz.adresse + 'api/beitreten', { data: { name: 'shoff-2' } })).status()).toBe(201);
  for (const gesammelt of fehler) {
    expect(gesammelt).toEqual([]);
  }
}

const NEUSTART = async () => {
  await instanz.beenden();
  await instanz.starten();
};
const GNADENFRIST = async (tabs) => {
  await tabs[0].waitForTimeout(1500);
};

const VARIANTEN = [
  { name: '(a) Neustart, zwei Tabs', tabs: 2, umgebung: {}, wegnehmen: NEUSTART },
  { name: '(b) Gnadenfrist 1000 ms abgelaufen, zwei Tabs', tabs: 2, umgebung: { LOCALCHATA_GNADENFRIST_MS: '1000' }, wegnehmen: GNADENFRIST },
  { name: '(c) Neustart, drei Tabs', tabs: 3, umgebung: {}, wegnehmen: NEUSTART },
  { name: '(d) Neustart, zwei Tabs, Browser ohne Web Locks', tabs: 2, umgebung: {}, wegnehmen: NEUSTART, ohneWebLocks: true },
];

for (const variante of VARIANTEN) {
  test(`B1 ${variante.name}: gleichzeitiges Neuladen zeigt in jedem Tab die Kopfzeile mit einer Sitzung`, async ({ context, request }) => {
    if (variante.ohneWebLocks) {
      await context.addInitScript(() => {
        Object.defineProperty(Navigator.prototype, 'locks', { get: () => undefined });
      });
    }
    instanz = await instanzStarten({ port: await freierPort(), umgebung: variante.umgebung });
    const { tabs, fehler } = await tabsMitShoff(context, variante.tabs);

    await variante.wegnehmen(tabs);
    await gleichzeitigNeuLaden(tabs);

    await alleTeilenEineSitzung(tabs, fehler, request);
  });
}
