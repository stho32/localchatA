'use strict';
// R00036 F0127 „Tabs auf dem Beitreten-Bildschirm folgen dem Beitritt eines Nachbar-Tabs“ (ADR-0012 Ergänzung Karte #62):
// Abnahme-Probe wörtlich, gegen das echte Programm auf freiem Port. Alle Tabs gehören zu einem Browser-Kontext. Läufe:
// SharedWorker vorhanden · per Init-Skript entfernt, je mit 2 und mit 3 Tabs; dazu ein Lauf ohne Web Locks wie
// r00003-tabs.spec.js B1. `POST /api/beitreten` wird über `context.on('request')` gezählt.
const { test, expect } = require('./rahmen');
const { instanzStarten, freierPort, channelAnlegen, fehlerSammeln, beitretenPerApi } = require('./hilfen');
const { leiste, vorlaeufige, bestaetigt, saetzeMit } = require('./r00012-hilfen');
const { JETZT, eingabe } = require('./senden-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const SITZUNG = 'localchatA.sitzung';
const NACHRICHTEN = '/api/channels/allgemein/nachrichten';
const GRENZE_MS = 1000;
const GRUND = '⚠ Nicht gesendet — der Beitritt war abgelaufen.';

const WORKER = [
  { titel: 'mit SharedWorker', ohneWorker: false },
  { titel: 'ohne SharedWorker', ohneWorker: true },
];

async function umgebungEinrichten(context, { ohneWorker = false, ohneWebLocks = false } = {}) {
  if (ohneWorker) {
    await context.addInitScript(() => { delete window.SharedWorker; });
  }
  if (ohneWebLocks) {
    await context.addInitScript(() => {
      Object.defineProperty(Navigator.prototype, 'locks', { get: () => undefined });
    });
  }
}

/// Zählt jeden POST /api/beitreten des Kontexts und merkt den Augenblick der ersten Antwort 201 danach.
function beitritteMitschreiben(context) {
  const stand = { posts: 0, angenommenUm: null };
  context.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/api/beitreten')) {
      stand.posts += 1;
    }
  });
  context.on('response', (antwort) => {
    const anfrage = antwort.request();
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/api/beitreten') && antwort.status() === 201) {
      stand.angenommenUm = Date.now();
    }
  });
  return stand;
}

function nachrichtenPostsMitschreiben(context) {
  const stand = { posts: 0 };
  context.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith(NACHRICHTEN)) {
      stand.posts += 1;
    }
  });
  return stand;
}

function ichAnfragenMitschreiben(context) {
  const stand = { anfragen: 0 };
  context.on('request', (anfrage) => {
    if (anfrage.url().endsWith('/api/ich')) {
      stand.anfragen += 1;
    }
  });
  return stand;
}

/// Kennung der Kopfzeile und Zahl der Listenzeilen mit dieser Kennung, je Tab.
function stand(tabs, kennung) {
  return Promise.all(tabs.map((tab) => tab.evaluate((gesucht) => {
    const ich = document.querySelector('.lc-me');
    const zeilen = [...document.querySelectorAll('#lc-teilnehmer .lc-person .lc-id')]
      .filter((id) => id.textContent.includes(gesucht)).length;
    return { kennung: ich === null ? null : ich.textContent, zeilen };
  }, kennung)));
}

/// Wartet ereignisgenau (25 ms), bis jeder Tab Kopfzeile und Liste des Nachbarn zeigt; liefert die Dauer ab `ab`.
async function alleFolgenNach(tabs, kennung, ab) {
  await expect.poll(async () => {
    const staende = await stand(tabs, kennung);
    return staende.every((s) => s.kennung === kennung && s.zeilen === 1);
  }, { intervals: [25], timeout: 10000, message: `alle ${tabs.length} Tabs zeigen ${kennung} und die Liste` }).toBe(true);
  return Date.now() - ab;
}

async function tabsAufDemBildschirm(context, anzahl) {
  const tabs = [];
  const fehler = [];
  for (let nummer = 0; nummer < anzahl; nummer += 1) {
    const tab = await context.newPage();
    fehler.push(fehlerSammeln(tab, { streng: true }));
    await tab.goto(instanz.adresse);
    await expect(tab.getByLabel('Name')).toBeVisible();
    tabs.push(tab);
  }
  return { tabs, fehler };
}

async function beitretenKlicken(tab, name) {
  await tab.getByLabel('Name').fill(name);
  await tab.getByRole('button', { name: 'Beitreten' }).click();
}

async function frischeInstanz(umgebung = {}) {
  instanz = await instanzStarten({
    port: await freierPort(),
    umgebung: { LOCALCHATA_JETZT: JETZT, ...umgebung },
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein'),
  });
}

// Erstbeitritt: alle Tabs auf dem Beitreten-Bildschirm (kein Token, kein Name); in B steht `abc` im Namensfeld.
const ERSTBEITRITT = [
  ...WORKER.flatMap((worker) => [2, 3].map((tabs) => ({ ...worker, tabs, titel: `${worker.titel}, ${tabs} Tabs` }))),
  { titel: 'mit SharedWorker, ohne Web Locks, 2 Tabs', tabs: 2, ohneWorker: false, ohneWebLocks: true },
];

for (const variante of ERSTBEITRITT) {
  test(`Erstbeitritt ${variante.titel}: Tab A tritt bei, alle folgen binnen 1 s mit einem einzigen POST`, async ({ context }) => {
    await umgebungEinrichten(context, variante);
    await frischeInstanz();
    const beitritte = beitritteMitschreiben(context);
    const { tabs, fehler } = await tabsAufDemBildschirm(context, variante.tabs);
    await tabs[1].getByLabel('Name').fill('abc');

    await beitretenKlicken(tabs[0], 'shoff');
    await expect.poll(() => beitritte.angenommenUm, { intervals: [25], timeout: 10000 }).not.toBeNull();
    const dauer = await alleFolgenNach(tabs, 'shoff@127.0.0.1', beitritte.angenommenUm);

    expect(dauer, 'binnen 1 s ab der Antwort 201').toBeLessThan(GRENZE_MS);
    await tabs[0].waitForTimeout(500);
    expect(beitritte.posts, 'genau ein POST /api/beitreten im Kontext').toBe(1);
    for (const stands of await stand(tabs, 'shoff@127.0.0.1')) {
      expect(stands).toEqual({ kennung: 'shoff@127.0.0.1', zeilen: 1 });
    }
    for (const tab of tabs) {
      await expect(tab.getByLabel('Name')).toHaveCount(0);
    }
    expect((await beitretenPerApi(instanz.adresse, 'shoff', '127.0.0.2')).status, 'der Name ist einmal belegt').toBe(409);
    for (const gesammelt of fehler) {
      expect(gesammelt).toEqual([]);
    }
  });
}

/// A und B (und C) sind als `shoff` in der Hauptansicht, die Instanz startet neu, `shoff` ist per API belegt, B sendet
/// `fünf` und hat `noch offen` im Feld; der stille Wiederbeitritt scheitert mit 409 → alle Tabs auf „Name vergeben“.
async function nameVergeben(context, anzahl) {
  await frischeInstanz();
  const tabs = [];
  const fehler = [];
  for (let nummer = 0; nummer < anzahl; nummer += 1) {
    const tab = await context.newPage();
    fehler.push(fehlerSammeln(tab, { streng: true }));
    await tab.goto(`${instanz.adresse}#/c/allgemein`);
    if (nummer === 0) {
      await beitretenKlicken(tab, 'shoff');
    }
    await expect(tab.getByRole('banner').locator('.lc-me')).toHaveText('shoff@127.0.0.1');
    tabs.push(tab);
  }
  const [a, b] = tabs;
  const totesToken = await a.evaluate((schluessel) => localStorage.getItem(schluessel), SITZUNG);
  await instanz.beenden();
  await expect(leiste(b)).toHaveText(/^⚠ Verbindung verloren/);
  await instanz.starten();
  expect((await beitretenPerApi(instanz.adresse, 'shoff')).status).toBe(201);

  let beitrittFreigeben = null;
  const gehalten = new Promise((weiter) => { beitrittFreigeben = weiter; });
  await b.route('**/api/beitreten', async (route) => {
    await gehalten;
    await route.continue();
  });
  await eingabe(b).fill('fünf');
  await eingabe(b).press('Enter');
  await eingabe(b).fill('noch offen');
  beitrittFreigeben();
  for (const tab of tabs) {
    await expect(tab.getByLabel('Name')).toBeVisible({ timeout: 20000 });
    await expect(tab.locator('#lc-meldung')).toContainText('vergeben');
  }
  await b.unroute('**/api/beitreten');
  await expect(b.locator('.lc-wartende')).toHaveText('1 ungesendete Nachricht wartet auf den Beitritt — Neuladen verwirft sie.');
  return { tabs, fehler, totesToken };
}

for (const variante of ERSTBEITRITT.slice(0, 4)) {
  test(`Name vergeben ${variante.titel}: Tab A tritt als shoff2 bei, B folgt, fünf steht fehlgeschlagen und geht erst auf Klick`, async ({ context }) => {
    test.setTimeout(90000);
    await umgebungEinrichten(context, variante);
    const { tabs } = await nameVergeben(context, variante.tabs);
    const [a, b] = tabs;
    const beitritte = beitritteMitschreiben(context);
    const nachrichten = nachrichtenPostsMitschreiben(context);
    const fehlerAb = tabs.map((tab) => fehlerSammeln(tab, { streng: true }));

    await beitretenKlicken(a, 'shoff2');
    await expect.poll(() => beitritte.angenommenUm, { intervals: [25], timeout: 10000 }).not.toBeNull();
    const dauer = await alleFolgenNach(tabs, 'shoff2@127.0.0.1', beitritte.angenommenUm);

    expect(dauer, 'binnen 1 s ab der Antwort 201').toBeLessThan(GRENZE_MS);
    await expect(vorlaeufige(b).locator('.lc-body')).toHaveText(['fünf']);
    await expect(vorlaeufige(b).locator('.lc-send')).toHaveText([GRUND]);
    await expect(vorlaeufige(b).getByRole('button', { name: 'Erneut senden' })).toHaveCount(1);
    await expect(vorlaeufige(b).getByRole('button', { name: 'Verwerfen' })).toHaveCount(1);
    await expect(eingabe(b)).toHaveValue('noch offen');
    await b.waitForTimeout(500);
    expect(beitritte.posts, 'seit dem Klick in A genau ein POST /api/beitreten').toBe(1);
    expect(nachrichten.posts, 'kein POST …/nachrichten ohne Klick').toBe(0);

    await vorlaeufige(b).getByRole('button', { name: 'Erneut senden' }).click();

    await expect(bestaetigt(b, 'fünf')).toHaveCount(1);
    await expect(bestaetigt(b, 'fünf').locator('.lc-id')).toContainText('shoff2');
    expect(saetzeMit(instanz.ordner, 'allgemein', 'fünf')).toBe(1);
    expect(nachrichten.posts).toBe(1);
    for (const gesammelt of fehlerAb) {
      expect(gesammelt, 'Konsole ab dem Beitritt in A').toEqual([]);
    }
  });
}

// Kein Fehlalarm (US-4): das abgelehnte Token, erneut geschrieben, löst weder eine Anfrage noch einen Beitritt aus.
test('Kein Fehlalarm: das abgelehnte Token, im Speicher erneut geschrieben, ändert nichts am Bildschirm', async ({ context }) => {
  test.setTimeout(90000);
  const { tabs, totesToken } = await nameVergeben(context, 2);
  const [a, b] = tabs;
  const beitritte = beitritteMitschreiben(context);
  const ich = ichAnfragenMitschreiben(context);

  await a.evaluate(([schluessel, token]) => localStorage.setItem(schluessel, token), [SITZUNG, totesToken]);
  await b.waitForTimeout(700);
  await a.evaluate((schluessel) => localStorage.removeItem(schluessel), SITZUNG);
  await b.waitForTimeout(300);

  expect(ich.anfragen, 'keine Anfrage GET /api/ich').toBe(0);
  expect(beitritte.posts).toBe(0);
  await expect(b.locator('#lc-meldung')).toContainText('vergeben');
  await expect(b.getByLabel('Name')).toBeEnabled();
});

// Kein Fehlalarm (US-4): ist das gemeldete Token dem Server unbekannt (401), bleibt der Bildschirm mit seiner Eingabe,
// das Formular ist wieder frei, und das Token gehört weiter dem Nachbarn — ein dritter Tab, der danach lädt, folgt.
test('Kein Fehlalarm: ich 401 beim Folgen — Bildschirm bleibt, Token bleibt im Speicher, ein später geladener Tab tritt nicht neu bei', async ({ context }) => {
  await frischeInstanz();
  const beitritte = beitritteMitschreiben(context);
  const { tabs } = await tabsAufDemBildschirm(context, 2);
  const [a, b] = tabs;
  await b.route('**/api/ich', (route) => route.fulfill({
    status: 401, contentType: 'application/json', body: JSON.stringify({ fehler: 'Sitzung unbekannt.' }),
  }));
  await b.getByLabel('Name').fill('abc');
  const ichAnfrage = b.waitForRequest((anfrage) => anfrage.url().endsWith('/api/ich'), { timeout: 10000 });

  await beitretenKlicken(a, 'shoff');
  await ichAnfrage;

  await expect(b.getByLabel('Name')).toBeEnabled();
  await expect(b.getByLabel('Name')).toHaveValue('abc');
  await expect(b.locator('form')).not.toHaveAttribute('aria-busy', 'true');
  const token = await a.evaluate((schluessel) => localStorage.getItem(schluessel), SITZUNG);
  expect(await b.evaluate((schluessel) => localStorage.getItem(schluessel), SITZUNG), 'das Token des Nachbarn bleibt').toBe(token);

  const dritter = await context.newPage();
  const fehlerDritter = fehlerSammeln(dritter, { streng: true });
  await dritter.goto(instanz.adresse);
  await expect(dritter.getByRole('banner').locator('.lc-me')).toHaveText('shoff@127.0.0.1');
  expect(beitritte.posts, 'der dritte Tab tritt nicht neu bei').toBe(1);
  expect(fehlerDritter).toEqual([]);
});

// Klick im Wettlauf (mit Web Locks): beide Tabs klicken im selben Augenblick „Beitreten“ — eine Sitzung, ein POST.
test('Klick im Wettlauf: zwei Tabs klicken zugleich, beide zeigen dieselbe Kennung, ein POST, eine Sitzung', async ({ context }) => {
  await frischeInstanz();
  const beitritte = beitritteMitschreiben(context);
  const { tabs, fehler } = await tabsAufDemBildschirm(context, 2);
  const [a, b] = tabs;
  await a.getByLabel('Name').fill('shoff');
  await b.getByLabel('Name').fill('bert');

  // Der Beitritt wird gehalten, bis beide Klicks angekommen sind (beide Formulare gesperrt): so liegen die Klicks sicher
  // vor jedem Beitritt, ohne Wettlauf um die Zeit zwischen ihnen.
  let beitrittFreigeben = null;
  const gehalten = new Promise((weiter) => { beitrittFreigeben = weiter; });
  await context.route('**/api/beitreten', async (route) => {
    await gehalten;
    await route.continue();
  });
  await a.getByRole('button', { name: 'Beitreten' }).click();
  await b.getByRole('button', { name: 'Beitreten' }).click();
  await expect(a.locator('form')).toHaveAttribute('aria-busy', 'true');
  await expect(b.locator('form')).toHaveAttribute('aria-busy', 'true');
  beitrittFreigeben();

  await expect(a.getByRole('banner').locator('.lc-me')).toContainText('@127.0.0.1');
  await expect(b.getByRole('banner').locator('.lc-me')).toContainText('@127.0.0.1');
  const kennung = await a.getByRole('banner').locator('.lc-me').textContent();
  await expect(b.getByRole('banner').locator('.lc-me')).toHaveText(kennung);
  await a.waitForTimeout(500);
  expect(beitritte.posts, 'genau ein POST /api/beitreten').toBe(1);
  const unterlegener = kennung.startsWith('shoff') ? 'bert' : 'shoff';
  expect((await beitretenPerApi(instanz.adresse, unterlegener, '127.0.0.2')).status, 'der andere Name wurde nie belegt').toBe(201);
  for (const gesammelt of fehler) {
    expect(gesammelt).toEqual([]);
  }
});
