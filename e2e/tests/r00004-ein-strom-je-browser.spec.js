'use strict';
// R00004 F0014 Ein Strom je Browser (US-20 bis US-24, ADR-0015): alle Tabs eines Browser-Kontexts teilen einen
// Strom über einen SharedWorker; ohne SharedWorker hält jeder Tab seinen eigenen.
const { test, expect } = require('@playwright/test');
const { instanzStarten, freierPort, fehlerSammeln } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const region = (page) => page.getByRole('complementary', { name: 'Teilnehmer' });
const zeileVon = (page, kennung) => region(page).locator('.lc-person').filter({ has: page.locator('.lc-id', { hasText: kennung }) });

async function gesundheit(request) {
  return (await request.get(instanz.adresse + 'api/gesundheit')).json();
}

async function zeigtSichSelbst(page, kennung = 'shoff@127.0.0.1', timeout = 5000) {
  const eigene = zeileVon(page, kennung);
  await expect(eigene.locator('.lc-dot')).toHaveClass(/\bon\b/, { timeout });
  await expect(eigene.locator('.lc-du')).toHaveText('(du)');
}

// Ein Kontext mit beigetretenem erstem Tab; weitere Tabs öffnen dieselbe Adresse und teilen die Sitzung.
async function kontextMitTabs(browser, name, anzahl, { ohneSharedWorker = false } = {}) {
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  if (ohneSharedWorker) {
    await kontext.addInitScript(() => { delete window.SharedWorker; });
  }
  const tabs = [];
  const fehler = [];
  for (let nummer = 0; nummer < anzahl; nummer += 1) {
    const tab = await kontext.newPage();
    fehler.push(fehlerSammeln(tab));
    await tab.goto(instanz.adresse);
    if (nummer === 0) {
      await tab.getByLabel('Name').fill(name);
      await tab.getByLabel('Name').press('Enter');
    }
    await zeigtSichSelbst(tab, `${name}@127.0.0.1`);
    tabs.push(tab);
  }
  return { kontext, tabs, fehler };
}

function ohneFehler(fehlerJeTab) {
  for (const gesammelt of fehlerJeTab) {
    expect(gesammelt).toEqual([]);
  }
}

test('US-20: sieben Tabs, ein Strom — alle zeigen die Liste, halter 1, Tab 7 antwortet in < 1 s', async ({ browser, request }) => {
  instanz = await instanzStarten();

  const a = await kontextMitTabs(browser, 'shoff', 7);

  const zeit = await a.tabs[6].evaluate(async () => {
    const anfang = performance.now();
    const antwort = await fetch('/api/gesundheit');
    await antwort.json();
    return performance.now() - anfang;
  });
  expect(zeit).toBeLessThan(1000);
  await expect.poll(async () => (await gesundheit(request)).halter).toBe(1);
  ohneFehler(a.fehler);
});

test('US-21: ein neuer Tab zeigt die Liste in < 2 s, ohne dass sich etwas ändert', async ({ browser }) => {
  instanz = await instanzStarten();
  const b = await kontextMitTabs(browser, 'mia', 1);
  const a = await kontextMitTabs(browser, 'shoff', 1);
  await expect(region(a.tabs[0]).locator('.lc-id')).toHaveText(['mia@127.0.0.1', 'shoff@127.0.0.1']);

  const geoeffnet = Date.now();
  const zweiter = await a.kontext.newPage();
  const fehler = fehlerSammeln(zweiter);
  await zweiter.goto(instanz.adresse);
  await expect(region(zweiter).locator('.lc-id')).toHaveText(['mia@127.0.0.1', 'shoff@127.0.0.1'], { timeout: 2000 });

  expect(Date.now() - geoeffnet).toBeLessThan(2000);
  expect(fehler).toEqual([]);
  ohneFehler([...a.fehler, ...b.fehler]);
});

test('US-22: offline erst, wenn der letzte Tab zu ist', async ({ browser }) => {
  instanz = await instanzStarten();
  const b = await kontextMitTabs(browser, 'mia', 3);
  const a = await kontextMitTabs(browser, 'shoff', 1);
  const mia = zeileVon(a.tabs[0], 'mia@127.0.0.1');
  await expect(mia.locator('.lc-dot')).toHaveClass(/\bon\b/);

  await b.tabs[0].close();
  await b.tabs[1].close();
  await a.tabs[0].waitForTimeout(2500);
  const nachZweien = await mia.locator('.lc-status').count();
  const geschlossen = Date.now();
  await b.tabs[2].close();
  await expect(mia.locator('.lc-status')).toHaveText('offline', { timeout: 2000 });

  expect(nachZweien).toBe(0);
  expect(Date.now() - geschlossen).toBeLessThan(2000);
  ohneFehler(a.fehler);
});

test('US-23: ohne SharedWorker hält jeder Tab seinen Strom — die Liste stimmt, drei Halter mehr', async ({ browser, request }) => {
  instanz = await instanzStarten();
  const a = await kontextMitTabs(browser, 'shoff', 1);
  await expect.poll(async () => (await gesundheit(request)).halter).toBe(1);

  const c = await kontextMitTabs(browser, 'mia', 3, { ohneSharedWorker: true });

  for (const tab of c.tabs) {
    await expect(region(tab).locator('.lc-id')).toHaveText(['mia@127.0.0.1', 'shoff@127.0.0.1']);
  }
  await expect.poll(async () => (await gesundheit(request)).halter).toBe(4);
  ohneFehler([...a.fehler, ...c.fehler]);
});

test('die Oberfläche legt den Worker unter /web/strom.js?v=<version> aus der Gesundheit an', async ({ browser, request }) => {
  instanz = await instanzStarten();
  const { version } = await gesundheit(request);
  const kontext = await browser.newContext();
  const angefragt = [];
  kontext.on('request', (anfrage) => angefragt.push(new URL(anfrage.url()).pathname + new URL(anfrage.url()).search));
  const tab = await kontext.newPage();
  await tab.goto(instanz.adresse);
  await tab.getByLabel('Name').fill('shoff');
  await tab.getByLabel('Name').press('Enter');
  await zeigtSichSelbst(tab);

  expect(angefragt).toContain(`/web/strom.js?v=${version}`);
});

// US-24 (ADR-0015 Regel 4): der Worker lässt genau einen Tab still beitreten; mit und ohne Web Locks (über
// eine LAN-Adresse gibt es keine), damit die Zuständigkeit des Workers und nicht die Sperre den Ausschlag gibt.
for (const { ohneWebLocks, titel } of [{ ohneWebLocks: false, titel: '' }, { ohneWebLocks: true, titel: ' — ohne Web Locks' }]) {
  test(`US-24: Neustart mit drei Tabs tritt genau einmal wieder bei${titel}`, async ({ browser, request }) => {
    instanz = await instanzStarten({ port: await freierPort() });
    const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    if (ohneWebLocks) {
      await kontext.addInitScript(() => {
        Object.defineProperty(Navigator.prototype, 'locks', { get: () => undefined });
      });
    }
    const tabs = [];
    const fehler = [];
    for (let nummer = 0; nummer < 3; nummer += 1) {
      const tab = await kontext.newPage();
      fehler.push(fehlerSammeln(tab, { streng: true }));
      await tab.goto(instanz.adresse);
      if (nummer === 0) {
        await tab.getByLabel('Name').fill('shoff');
        await tab.getByLabel('Name').press('Enter');
      }
      await zeigtSichSelbst(tab);
      tabs.push(tab);
    }
    const beitritte = [];
    kontext.on('request', (anfrage) => {
      if (anfrage.method() === 'POST' && anfrage.url().endsWith('/api/beitreten')) {
        beitritte.push(anfrage.url());
      }
    });

    await instanz.beenden();
    await expect(tabs[0].locator('#lc-verbindung')).toBeVisible();
    await instanz.starten();

    for (const tab of tabs) {
      await expect(tab.locator('#lc-verbindung')).toBeHidden({ timeout: 15000 });
      await zeigtSichSelbst(tab, 'shoff@127.0.0.1', 15000);
    }
    await tabs[0].waitForTimeout(1000);
    expect(beitritte).toHaveLength(1);
    expect((await request.post(instanz.adresse + 'api/beitreten', { data: { name: 'shoff' } })).status()).toBe(409);
    ohneFehler(fehler);
  });
}

// Nacharbeit 3, Code-Review B2: LAN-Rückfall ohne SharedWorker und ohne Web Locks — nach einem Neustart
// treten die Tabs gleichzeitig wieder bei; der 409 eines Nachbar-Tabs wird abgefangen und dessen Token
// übernommen. Kontrolle: dieselbe Probe mit Web Locks.
for (const { tabs: anzahl, ohneWebLocks } of [
  { tabs: 2, ohneWebLocks: true }, { tabs: 3, ohneWebLocks: true }, { tabs: 3, ohneWebLocks: false },
]) {
  let titel = `${anzahl} Tabs ohne SharedWorker`;
  if (ohneWebLocks) {
    titel += ' und ohne Web Locks';
  }
  test(`B2: Neustart im Rückfall, ${titel} — jeder Tab ist zurück, shoff genau einmal belegt`, async ({ browser, request }) => {
    instanz = await instanzStarten({ port: await freierPort() });
    const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    await kontext.addInitScript(() => { delete window.SharedWorker; });
    if (ohneWebLocks) {
      await kontext.addInitScript(() => {
        Object.defineProperty(Navigator.prototype, 'locks', { get: () => undefined });
      });
    }
    const tabs = [];
    for (let nummer = 0; nummer < anzahl; nummer += 1) {
      const tab = await kontext.newPage();
      fehlerSammeln(tab, { serverWegErwartet: true });
      await tab.goto(instanz.adresse);
      if (nummer === 0) {
        await tab.getByLabel('Name').fill('shoff');
        await tab.getByLabel('Name').press('Enter');
      }
      await zeigtSichSelbst(tab);
      tabs.push(tab);
    }

    await instanz.beenden();
    await expect(tabs[0].locator('#lc-verbindung')).toBeVisible();
    await instanz.starten();

    for (const tab of tabs) {
      await expect(tab.locator('#lc-verbindung')).toBeHidden({ timeout: 15000 });
      await zeigtSichSelbst(tab, 'shoff@127.0.0.1', 15000);
      await expect(tab.getByLabel('Name')).toHaveCount(0);
    }
    expect((await request.post(instanz.adresse + 'api/beitreten', { data: { name: 'shoff' } })).status()).toBe(409);
  });
}

// Nacharbeit 3, Code-Review A1: der Worker meldet das abgelehnte Token mit — ein Tab, dessen eigenes Token
// veraltet ist, erkennt so das Token aus dem Speicher als abgelehnt und tritt sofort bei.
test('A1: Neustart, Tab a tritt bei und geht, zweiter Neustart — Tab b ist in < 2 s zurück', async ({ browser }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const a = await kontext.newPage();
  await a.goto(instanz.adresse);
  await a.getByLabel('Name').fill('shoff');
  await a.getByLabel('Name').press('Enter');
  await zeigtSichSelbst(a);
  const b = await kontext.newPage();
  await b.goto(instanz.adresse);
  await zeigtSichSelbst(b);

  await instanz.beenden();
  await instanz.starten();
  await expect(a.locator('#lc-verbindung')).toBeHidden({ timeout: 15000 });
  await expect(b.locator('#lc-verbindung')).toBeHidden({ timeout: 15000 });
  await a.close();
  await instanz.beenden();
  await expect(b.locator('#lc-verbindung')).toBeVisible();
  await b.waitForTimeout(500);
  await instanz.starten();
  const neu = Date.now();
  await expect(b.locator('#lc-verbindung')).toBeHidden({ timeout: 15000 });
  const zurueckNach = Date.now() - neu;

  await zeigtSichSelbst(b);
  expect(zurueckNach).toBeLessThan(2000);
});
