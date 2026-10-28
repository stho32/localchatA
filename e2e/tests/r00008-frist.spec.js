'use strict';
// R00008 F0037 Channel-Einstellungen im Browser (US-15 bis US-20; ADR-0033): `# name ▾` in der Kopfzeile mit der
// Frist ab 720 px, der Dialog „# name — Einstellungen“, Verkürzen in zwei Schritten. Heute ist der 28.10.2026.
const fs = require('fs');
const http = require('http');
const path = require('path');
const { test, expect } = require('@playwright/test');
const { instanzStarten, fehlerSammeln, beitretenPerApi, channelAnlegen, sichtbarNach } = require('./hilfen');

const JETZT = '2026-10-28T12:00:00.000+01:00';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

function einstellungen(ordner, name, inhalt) {
  const pfad = path.join(ordner, 'channels', name);
  fs.mkdirSync(pfad, { recursive: true });
  fs.writeFileSync(path.join(pfad, 'einstellungen.toml'), inhalt);
}

// `allgemein` mit Frist 30 und den Tagen [tag, anzahl, versatz] → Vorbereitung für instanzStarten.
function fristDreissigMit(tageMitAnzahl) {
  return (ordner) => {
    const satz = (tag, nummer, versatz) => [`${tag}T09:0${nummer}:00.000${versatz}`, 'alpha@127.0.0.2', `${tag} n${nummer}`];
    const tage = {};
    for (const [tag, anzahl, versatz] of tageMitAnzahl) {
      tage[tag] = Array.from({ length: anzahl }, (_, nummer) => satz(tag, nummer, versatz));
    }
    channelAnlegen(ordner, 'allgemein', tage);
    einstellungen(ordner, 'allgemein', 'frist_tage = 30\n');
  };
}

// 20.(2), 21.(3), 22.(1), 28.(1) (US-17).
const verkuerzbar = fristDreissigMit([['2026-10-20', 2, '+02:00'], ['2026-10-21', 3, '+02:00'], ['2026-10-22', 1, '+02:00'], ['2026-10-28', 1, '+01:00']]);
// 21.(1), 22.(1), 28.(1) (US-25).
const einTagVerkuerzbar = fristDreissigMit([['2026-10-21', 1, '+02:00'], ['2026-10-22', 1, '+02:00'], ['2026-10-28', 1, '+01:00']]);

async function starten(vorbereiten) {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_JETZT: JETZT }, vorbereiten });
}

async function seiteMitFehlern(browser, { breite = 1280, zusaetzlich = {} } = {}) {
  const kontext = await browser.newContext({ viewport: { width: breite, height: 800 } });
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page, { zusaetzlich }) };
}

async function beitreten(page, name, channel = 'allgemein') {
  await page.goto(`${instanz.adresse}#/c/${channel}`);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
}

const kopf = (page) => page.getByRole('banner');
const schalter = (page, name = 'allgemein') => kopf(page).getByRole('button', { name: `# ${name} — Channel-Einstellungen` });
const fristText = (page) => kopf(page).locator('.lc-head-frist');
const dialog = (page) => page.getByRole('dialog');
const feld = (page) => dialog(page).getByLabel('Aufbewahrungsfrist');
const statuszeile = (page) => page.getByRole('status').and(page.locator('.lc-statuszeile'));

/// `PUT …/frist` per API von 127.0.0.1 (Agent).
function fristPerApi(adresse, token, name, koerper) {
  const text = JSON.stringify(koerper);
  const ziel = new URL(`api/channels/${name}/frist`, adresse);
  return new Promise((fertig, gescheitert) => {
    const anfrage = http.request({
      host: ziel.hostname, port: ziel.port, path: ziel.pathname, method: 'PUT', localAddress: '127.0.0.1',
      headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(text), 'X-LocalchatA-Sitzung': token },
    }, (antwort) => {
      let inhalt = '';
      antwort.on('data', (teil) => { inhalt += teil; });
      antwort.on('end', () => fertig({ status: antwort.statusCode, daten: JSON.parse(inhalt) }));
    });
    anfrage.once('error', gescheitert);
    anfrage.end(text);
  });
}

function tagDa(tag) {
  return fs.existsSync(path.join(instanz.ordner, 'channels', 'allgemein', `${tag}.log`));
}

test('Kopfzeile (US-15, B0196): # allgemein ▾ als Schalter, dahinter · 7 Tage vor der Beschreibung', async ({ browser }) => {
  await starten((ordner) => einstellungen(ordner, 'allgemein', 'beschreibung = "Absprachen"\n'));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');

  await expect(schalter(a.page)).toBeVisible();
  await expect(schalter(a.page)).toHaveAttribute('aria-haspopup', 'dialog');
  await expect(schalter(a.page)).toHaveText('# allgemein ▾');
  await expect(kopf(a.page).locator('.lc-head-chan')).toHaveText('# allgemein');
  await expect(fristText(a.page)).toHaveText('· 7 Tage');
  const reihenfolge = await kopf(a.page).locator('.lc-head-ort > *').evaluateAll((teile) => teile.map((teil) => teil.className));
  expect(reihenfolge).toEqual(['lc-head-inst', 'lc-head-schalter', 'lc-head-frist', 'lc-head-desc']);
  expect(a.fehler).toEqual([]);
});

test('Kopfzeile (US-19): kaputte Einstellungen zeigen · ⚠ Frist gesperrt; eine Frist von 1 heißt · 1 Tag', async ({ browser }) => {
  await starten((ordner) => {
    einstellungen(ordner, 'kaputt', 'frist_tage = "sieben"\n');
    einstellungen(ordner, 'kurz', 'frist_tage = 1\n');
  });
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff', 'kaputt');

  await expect(fristText(a.page)).toHaveText('· ⚠ Frist gesperrt');
  await a.page.goto(`${instanz.adresse}#/c/kurz`);
  await expect(fristText(a.page)).toHaveText('· 1 Tag');
  expect(a.fehler).toEqual([]);
});

test('Schmal (US-20): bei 719 px kein Frist-Text, bei 720 px sichtbar', async ({ browser }) => {
  await starten((ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }));
  const a = await seiteMitFehlern(browser, { breite: 719 });
  await beitreten(a.page, 'shoff');
  await expect(schalter(a.page)).toBeVisible();

  await expect(fristText(a.page)).toBeHidden();
  await a.page.setViewportSize({ width: 720, height: 800 });
  await expect(fristText(a.page)).toBeVisible();
  expect(a.fehler).toEqual([]);
});

test('Live (B0196): eine Änderung per API erreicht die Kopfzeile ohne Neuladen', async ({ browser }) => {
  await starten((ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await expect(fristText(a.page)).toHaveText('· 7 Tage');
  const beta = await beitretenPerApi(instanz.adresse, 'beta');

  const gesetzt = await fristPerApi(instanz.adresse, beta.daten.token, 'allgemein', { tage: 31 });

  expect(gesetzt.status).toBe(200);
  await expect(fristText(a.page)).toHaveText('· 31 Tage');
  expect(a.fehler).toEqual([]);
});

/// Z2 aus R00008: Antwort 200 in A → Kopfzeile in B, ereignisgenau (requestAnimationFrame).
const Z2_MS = 2000;

test('Setzen (US-15, B0197): Dialog mit 7, 30 + Enter → zu, Statuszeile, A und B zeigen · 30 Tage (Z2)', async ({ browser }) => {
  await starten((ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }));
  const a = await seiteMitFehlern(browser);
  const b = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await beitreten(b.page, 'anna');
  await expect(fristText(b.page)).toHaveText('· 7 Tage');

  await schalter(a.page).click();

  await expect(dialog(a.page)).toBeVisible();
  await expect(dialog(a.page).getByRole('heading')).toHaveText('# allgemein — Einstellungen');
  await expect(feld(a.page)).toHaveValue('7');
  await expect(feld(a.page)).toBeFocused();
  await expect(dialog(a.page)).toContainText('Ältere Tage entfallen. Heute zählt mit. Standard: 7 Tage.');
  await feld(a.page).fill('30');
  const antwort = a.page.waitForResponse((r) => r.url().endsWith('/api/channels/allgemein/frist'));
  await feld(a.page).press('Enter');
  const ab = (await antwort, Date.now());
  const ms = await sichtbarNach(b.page, () => document.querySelector('.lc-head-frist')?.textContent === '· 30 Tage', { ab, timeout: 10000 });

  console.log(`messung z2_ms: ${ms}`);
  expect(ms).toBeLessThanOrEqual(Z2_MS);
  await expect(dialog(a.page)).toHaveCount(0);
  await expect(statuszeile(a.page)).toHaveText('Frist für # allgemein: 30 Tage.');
  await expect(fristText(a.page)).toHaveText('· 30 Tage');
  await expect(schalter(a.page)).toBeFocused();
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('Ungültig (US-16): 0 → Fehlerzeile mit ⚠ und aria-invalid, Enter sendet nichts', async ({ browser }) => {
  await starten((ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  const abrufe = [];
  a.page.on('request', (anfrage) => {
    if (anfrage.url().endsWith('/frist')) {
      abrufe.push(anfrage.url());
    }
  });
  await schalter(a.page).click();

  await feld(a.page).fill('0');
  await feld(a.page).press('Enter');

  await expect(dialog(a.page).getByRole('alert')).toHaveText('⚠ Frist: ganze Zahl von 1 bis 3650 Tagen.');
  await expect(feld(a.page)).toHaveAttribute('aria-invalid', 'true');
  const farbe = await dialog(a.page).getByRole('alert').evaluate((zeile) => getComputedStyle(zeile).color);
  const gefahr = await a.page.evaluate(() => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--kd-danger)';
    document.body.append(probe);
    const wert = getComputedStyle(probe).color;
    probe.remove();
    return wert;
  });
  expect(farbe).toBe(gefahr);
  await expect(dialog(a.page)).toBeVisible();
  expect(abrufe).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Esc (US-18): schließt ohne zu speichern, der Fokus liegt auf # allgemein', async ({ browser }) => {
  await starten((ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  const abrufe = [];
  a.page.on('request', (anfrage) => {
    if (anfrage.url().endsWith('/frist')) {
      abrufe.push(anfrage.url());
    }
  });
  await schalter(a.page).focus();
  await a.page.keyboard.press('Enter');
  await expect(feld(a.page)).toBeFocused();
  await feld(a.page).fill('12');

  await a.page.keyboard.press('Escape');

  await expect(dialog(a.page)).toHaveCount(0);
  await expect(schalter(a.page)).toBeFocused();
  await expect(fristText(a.page)).toHaveText('· 7 Tage');
  expect(abrufe).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Fremde Änderung (ADR-0033 Regel 5): die Eingabe bleibt, der Hinweis nennt den neuen Wert', async ({ browser }) => {
  await starten((ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await schalter(a.page).click();
  await feld(a.page).fill('12');
  const beta = await beitretenPerApi(instanz.adresse, 'beta');

  expect((await fristPerApi(instanz.adresse, beta.daten.token, 'allgemein', { tage: 31 })).status).toBe(200);

  await expect(dialog(a.page)).toContainText('Inzwischen geändert: 31 Tage.');
  await expect(feld(a.page)).toHaveValue('12');
  expect(a.fehler).toEqual([]);
});

test('Gesperrt (US-19): Feld gesperrt, der Dialog nennt die Datei und „in diesem Channel entfällt nichts“', async ({ browser }) => {
  await starten((ordner) => einstellungen(ordner, 'kaputt', 'frist_tage = "sieben"\n'));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff', 'kaputt');

  await schalter(a.page, 'kaputt').click();

  await expect(feld(a.page)).toBeDisabled();
  await expect(dialog(a.page).locator('code')).toHaveText('channels/kaputt/einstellungen.toml');
  await expect(dialog(a.page)).toContainText('in diesem Channel entfällt nichts');
  await expect(dialog(a.page).getByRole('button', { name: 'Speichern' })).toBeDisabled();
  expect(a.fehler).toEqual([]);
});

test('Netzfehler (Kriterium nach Nutzen): Programm beendet, 30 speichern → Fehlerzeile, Dialog offen, Eingabe bleibt', async ({ browser }) => {
  await starten((ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff');
  await schalter(a.page).click();
  await feld(a.page).fill('30');
  await instanz.beenden();

  await feld(a.page).press('Enter');

  await expect(dialog(a.page).getByRole('alert')).toHaveText('⚠ Nicht gespeichert — Verbindung prüfen.');
  await expect(dialog(a.page)).toBeVisible();
  await expect(feld(a.page)).toHaveValue('30');
});

test('Schmal (US-20): bei 320 px mit und ohne offenen Dialog kein Querscrollen', async ({ browser }) => {
  await starten((ordner) => fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true }));
  const a = await seiteMitFehlern(browser, { breite: 320 });
  await beitreten(a.page, 'shoff');
  const querscrollen = () => a.page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);

  expect(await querscrollen()).toBe(false);
  await schalter(a.page).click();
  await expect(dialog(a.page)).toBeVisible();
  expect(await querscrollen()).toBe(false);
  await a.page.keyboard.press('Escape');
  await expect(dialog(a.page)).toHaveCount(0);
  expect(await querscrollen()).toBe(false);
  expect(a.fehler).toEqual([]);
});

test('Verkürzen in zwei Schritten (US-17, B0198): zweimal Enter löscht nichts, „Frist verkürzen“ löscht und öffnet neu', async ({ browser }) => {
  await starten(verkuerzbar);
  const a = await seiteMitFehlern(browser, { zusaetzlich: { '/api/channels/allgemein/frist': [409] } });
  await beitreten(a.page, 'shoff');
  await expect(a.page.locator('.lc-msg')).toHaveCount(7);
  await schalter(a.page).click();

  await feld(a.page).fill('7');
  await feld(a.page).press('Enter');

  const satz = 'Bei 7 Tagen entfallen sofort 2 Tage bis einschließlich 21. Oktober 2026 mit 5 Nachrichten. Das lässt sich nicht rückgängig machen.';
  await expect(dialog(a.page).locator('.lc-bestaetigung')).toHaveText(satz);
  await expect(feld(a.page)).toHaveJSProperty('readOnly', true);
  await expect(dialog(a.page).getByRole('button', { name: 'Zurück' })).toBeFocused();
  await a.page.keyboard.press('Enter');
  await expect(dialog(a.page).locator('.lc-bestaetigung')).toBeHidden();
  await expect(feld(a.page)).toHaveValue('7');
  await expect(feld(a.page)).toHaveJSProperty('readOnly', false);
  expect(['2026-10-20', '2026-10-21', '2026-10-22', '2026-10-28'].every(tagDa)).toBe(true);
  const beta = await beitretenPerApi(instanz.adresse, 'beta');
  const liste = await new Promise((fertig) => {
    http.get(new URL('api/channels', instanz.adresse), { headers: { 'X-LocalchatA-Sitzung': beta.daten.token }, localAddress: '127.0.0.1' }, (antwort) => {
      let text = '';
      antwort.on('data', (teil) => { text += teil; });
      antwort.on('end', () => fertig(JSON.parse(text)));
    });
  });
  expect(liste.liste.find((channel) => channel.name === 'allgemein').frist_tage).toBe(30);

  await feld(a.page).press('Enter');
  await dialog(a.page).getByRole('button', { name: 'Frist verkürzen' }).click();

  await expect(statuszeile(a.page)).toHaveText('Frist für # allgemein: 7 Tage — 2 Tage mit 5 Nachrichten entfallen.');
  await expect(dialog(a.page)).toHaveCount(0);
  await expect(a.page.locator('.lc-msg')).toHaveCount(2);
  await expect(a.page.locator('.lc-msg').first().locator('.lc-body')).toHaveText('2026-10-22 n0');
  expect(tagDa('2026-10-20') || tagDa('2026-10-21')).toBe(false);
  expect(a.fehler).toEqual([]);
});

// Abnahme-Probe UX-Review #12, Muss 1: der Bildschirmleser liest den Satz mit dem Fokus auf „Zurück“ vor. Statt des
// Musters /2 Tage .* 5 Nachrichten .* nicht rückgängig/ der ganze Satz aus US-26 — das Muster verlangt ein Leerzeichen
// nach „Nachrichten“, im Satz steht dort der Punkt.
test('Bildschirmleser (US-26, B0317): im zweiten Schritt trägt „Zurück“ den Satz mit 2 Tagen und 5 Nachrichten als Beschreibung', async ({ browser }) => {
  await starten(verkuerzbar);
  const a = await seiteMitFehlern(browser, { zusaetzlich: { '/api/channels/allgemein/frist': [409] } });
  await beitreten(a.page, 'shoff');
  await schalter(a.page).click();

  await feld(a.page).fill('7');
  await feld(a.page).press('Enter');

  const zurueck = a.page.getByRole('button', { name: 'Zurück' });
  await expect(zurueck).toBeFocused();
  await expect(zurueck).toHaveAccessibleDescription(
    'Bei 7 Tagen entfallen sofort 2 Tage bis einschließlich 21. Oktober 2026 mit 5 Nachrichten. Das lässt sich nicht rückgängig machen.',
  );
  expect(a.fehler).toEqual([]);
});

test('Bildschirmleser (US-25, US-26, B0317): ein einzelner Tag — „Zurück“ trägt „… entfällt sofort 1 Tag … mit 1 Nachricht“ als Beschreibung', async ({ browser }) => {
  await starten(einTagVerkuerzbar);
  const a = await seiteMitFehlern(browser, { zusaetzlich: { '/api/channels/allgemein/frist': [409] } });
  await beitreten(a.page, 'shoff');
  await schalter(a.page).click();

  await feld(a.page).fill('7');
  await feld(a.page).press('Enter');

  const satz = 'Bei 7 Tagen entfällt sofort 1 Tag bis einschließlich 21. Oktober 2026 mit 1 Nachricht. Das lässt sich nicht rückgängig machen.';
  await expect(dialog(a.page).locator('.lc-bestaetigung')).toHaveText(satz);
  const zurueck = a.page.getByRole('button', { name: 'Zurück' });
  await expect(zurueck).toBeFocused();
  await expect(zurueck).toHaveAccessibleDescription(satz);
  expect(a.fehler).toEqual([]);
});

test('Bildschirmleser (US-27, B0317): gesperrt — der Dialog trägt Datei und „in diesem Channel entfällt nichts“ als Beschreibung', async ({ browser }) => {
  await starten((ordner) => einstellungen(ordner, 'kaputt', 'frist_tage = "sieben"\n'));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page, 'shoff', 'kaputt');

  await schalter(a.page, 'kaputt').click();

  await expect(dialog(a.page)).toHaveAccessibleDescription(/channels\/kaputt\/einstellungen\.toml.*in diesem Channel entfällt nichts/);
  expect(a.fehler).toEqual([]);
});
