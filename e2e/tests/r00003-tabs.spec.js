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

function beitritteZaehlen(context) {
  const beitritte = [];
  context.on('request', (anfrage) => {
    if (anfrage.url().endsWith('/api/beitreten')) {
      beitritte.push(anfrage.url());
    }
  });
  return beitritte;
}

async function gleichzeitigNeuLaden(tabs) {
  await Promise.all(tabs.map((tab) => tab.goto(instanz.adresse)));
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
// Seit R00004 hält jeder offene Tab die Sitzung über seinen Ereignisstrom am Leben (Halter, ADR-0012): die
// Gnadenfrist läuft erst ab, wenn keine Seite der Instanz mehr offen ist.
const GNADENFRIST = async (tabs) => {
  for (const tab of tabs) {
    await tab.goto('about:blank');
  }
  await tabs[0].waitForTimeout(1500);
};

const VARIANTEN = [
  { name: '(a) Neustart, zwei Tabs', tabs: 2, umgebung: {}, wegnehmen: NEUSTART },
  { name: '(b) Gnadenfrist 1000 ms abgelaufen, zwei Tabs', tabs: 2, umgebung: { LOCALCHATA_GNADENFRIST_MS: '1000' }, wegnehmen: GNADENFRIST },
  { name: '(c) Neustart, drei Tabs', tabs: 3, umgebung: {}, wegnehmen: NEUSTART },
  { name: '(d) Neustart, zwei Tabs, Browser ohne Web Locks', tabs: 2, umgebung: {}, wegnehmen: NEUSTART, ohneWebLocks: true },
  { name: '(e) Neustart, zwei Tabs, 201 kommt verzögert an', tabs: 2, umgebung: {}, wegnehmen: NEUSTART, beitrittVerzoegert: true },
  {
    name: '(f) Neustart, zwei Tabs, ohne Web Locks, 201 kommt verzögert an',
    tabs: 2, umgebung: {}, wegnehmen: NEUSTART, ohneWebLocks: true, beitrittVerzoegert: true,
  },
];

// Der gewonnene Beitritt erreicht seinen Tab erst, nachdem der andere Tab sein 409 hat — so liegt das Fenster
// zwischen 409 und dem Speichern des Tokens fest offen, statt vom Zufall der Reihenfolge abzuhängen.
async function gewonnenenBeitrittVerzoegern(context) {
  await context.route('**/api/beitreten', async (route) => {
    const antwort = await route.fetch();
    if (antwort.status() === 201) {
      await new Promise((weiter) => setTimeout(weiter, 500));
    }
    await route.fulfill({ response: antwort });
  });
}

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
    if (variante.beitrittVerzoegert) {
      await gewonnenenBeitrittVerzoegern(context);
    }
    const stilleBeitritte = beitritteZaehlen(context);
    await gleichzeitigNeuLaden(tabs);

    await alleTeilenEineSitzung(tabs, fehler, request);
    if (!variante.ohneWebLocks) {
      expect(stilleBeitritte, 'mit Web Locks tritt genau ein Tab still bei').toHaveLength(1);
    }
  });
}
