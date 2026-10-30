'use strict';
// R00007 F0029 Verlauf öffnen (US-9 bis US-13, L2): gegen das echte Programm, Chromium 1280 × 800, Uhr
// 2026-10-28 15:00 (+01:00). Nachrichten liegen vor dem Start als Tagesdateien; `gross` (100.000 über 7 Tage) kommt
// vom Testdaten-Erzeuger. Die Grenze L2 ist die der Anforderung (500 ms), gemessen im Browser vom Zeitstempel des
// click bis zum Bild, in dem die neueste Nachricht ganz sichtbar ist (requestAnimationFrame).
const { test, expect } = require('./rahmen');
const {
  instanzStarten, fehlerSammeln, beitretenPerApi, channelAnlegen, grossAnlegen,
} = require('./hilfen');
const { neuesteSichtbarNachKlick, juengsteZeit } = require('./verlauf-hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const GRENZE_L2_MS = 500;
const ALPHA = 'alpha@127.0.0.2';
const BETA = 'beta@127.0.0.3';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function starten(vorbereiten) {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_JETZT: JETZT }, vorbereiten });
}

const US9 = {
  '2026-10-28': [
    ['2026-10-28T14:00:00.000+01:00', ALPHA, 'eins'],
    ['2026-10-28T14:20:00.000+01:00', BETA, 'zwei\ndrei'],
    ['2026-10-28T14:40:00.000+01:00', ALPHA, '<b>vier</b>'],
  ],
};

const link = (page, name) => page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true });
const verlauf = (page) => page.getByRole('log');
const nachrichten = (page) => verlauf(page).locator('.lc-msg');

async function beitreten(page, name = 'shoff', url = instanz.adresse) {
  await page.goto(url);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
}

async function seiteMitFehlern(browser, optionen = {}) {
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 }, ...optionen });
  const page = await kontext.newPage();
  return { kontext, page, fehler: fehlerSammeln(page) };
}

// Ganz sichtbar heißt: innerhalb der sichtbaren Fläche des Verlaufs (± 1 px).
function ganzSichtbar(element) {
  return element.evaluate((nachricht) => {
    const log = nachricht.closest('[role="log"]').getBoundingClientRect();
    const lage = nachricht.getBoundingClientRect();
    return lage.top >= log.top - 1 && lage.bottom <= log.bottom + 1;
  });
}

test('Anzeige (US-9): drei Nachrichten mit Kennung in Mono und Uhrzeit des Servers, Text als Text, am Ende', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'allgemein', US9));
  const a = await seiteMitFehlern(browser, { timezoneId: 'Asia/Tokyo' });
  await beitreten(a.page);

  await link(a.page, 'allgemein').click();

  await expect(nachrichten(a.page)).toHaveCount(3);
  await expect(nachrichten(a.page).locator('.lc-id')).toHaveText([ALPHA, BETA, ALPHA]);
  await expect(nachrichten(a.page).locator('.lc-time')).toHaveText(['14:00', '14:20', '14:40']);
  await expect(nachrichten(a.page).locator('.lc-body')).toHaveText(['eins', 'zwei\ndrei', '<b>vier</b>']);
  const mono = await nachrichten(a.page).first().locator('.lc-id').evaluate((element) => getComputedStyle(element).fontFamily);
  expect(mono).toContain('IBM Plex Mono');
  await expect(nachrichten(a.page).first().locator('time')).toHaveAttribute('title', '28.10.2026, 14:00:00');
  await expect(nachrichten(a.page).first().locator('time')).toHaveAttribute('datetime', '2026-10-28T14:00:00.000+01:00');
  const zweiZeilen = await nachrichten(a.page).nth(1).locator('.lc-body').evaluate((element) => element.getClientRects()[0].height);
  const eineZeile = await nachrichten(a.page).first().locator('.lc-body').evaluate((element) => element.getClientRects()[0].height);
  expect(zweiZeilen).toBeGreaterThan(eineZeile * 1.5);
  await expect(verlauf(a.page).locator('b')).toHaveCount(0);
  expect(await ganzSichtbar(nachrichten(a.page).last())).toBe(true);
  expect(a.fehler).toEqual([]);
});

test('Leer (US-10): ein Channel ohne Tagesdatei sagt „Noch keine Nachrichten in # leer.“', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'leer'));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);

  await link(a.page, 'leer').click();

  await expect(verlauf(a.page).locator('p').first()).toHaveText('Noch keine Nachrichten in # leer.');
  await expect(verlauf(a.page)).toHaveText('Noch keine Nachrichten in # leer.Agenten schreiben hier mit POST /api/channels/leer/nachrichten.');
  await expect(nachrichten(a.page)).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

test('L2 (US-11): 100.000 Nachrichten — die neueste ist innerhalb der Grenze L2 nach dem Klick sichtbar, ein Abruf ohne Cursor', async ({ browser }, testInfo) => {
  await starten((ordner) => grossAnlegen(ordner));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  const abrufe = [];
  a.page.on('request', (anfrage) => {
    if (anfrage.url().includes('/nachrichten')) {
      abrufe.push(new URL(anfrage.url()).search);
    }
  });
  const juengste = await juengsteZeit(instanz.adresse, 'gross');

  const ms = await neuesteSichtbarNachKlick(a.page, 'gross', juengste);
  testInfo.annotations.push({ type: 'messung', description: `l2_ms=${ms}` });
  console.log(`messung l2_ms=${ms}`);

  expect(ms).toBeLessThan(GRENZE_L2_MS);
  await expect(nachrichten(a.page)).toHaveCount(100);
  await expect(nachrichten(a.page).last()).toHaveAttribute('data-zeit', juengste);
  await expect(nachrichten(a.page).last().locator('.lc-id')).toHaveText(/^claude@10\.0\.0\.\d$/);
  expect(abrufe).toEqual(['?n=100']);
  expect(a.fehler).toEqual([]);
});

test('Wechsel mit verzögerter Antwort (US-12): nach dem Ende des Abrufs für gross zeigt der Verlauf genau allgemein', async ({ browser }) => {
  await starten((ordner) => {
    grossAnlegen(ordner);
    channelAnlegen(ordner, 'allgemein', US9);
  });
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await a.page.route('**/api/channels/gross/nachrichten**', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.continue().catch(() => {});
  });
  const grossFertig = new Promise((fertig) => {
    const beendet = (anfrage) => {
      if (anfrage.url().includes('/api/channels/gross/nachrichten')) {
        fertig();
      }
    };
    a.page.on('requestfinished', beendet);
    a.page.on('requestfailed', beendet);
  });

  await link(a.page, 'gross').click();
  await link(a.page, 'allgemein').click();
  await grossFertig;
  await a.page.waitForTimeout(100);

  await expect(nachrichten(a.page)).toHaveCount(3);
  await expect(nachrichten(a.page).locator('.lc-body')).toHaveText(['eins', 'zwei\ndrei', '<b>vier</b>']);
  await expect(a.page.getByRole('banner').locator('.lc-head-chan')).toHaveText('# allgemein');
  const fehler = a.fehler.filter((zeile) => !(zeile.includes('Failed to load resource') && zeile.includes('/api/channels/gross/nachrichten')));
  expect(fehler).toEqual([]);
});

test('Am Ende trotz später Schrift: lädt die Schrift 1 s später und ändert die Höhen, bleibt die neueste Nachricht ganz sichtbar', async ({ browser }) => {
  await starten((ordner) => grossAnlegen(ordner));
  // Schmaler als 1280 px, damit die Zeilen umbrechen: die Zeilenhöhe ist fest, nur der Umbruch hängt an der Schrift.
  const a = await seiteMitFehlern(browser, { bypassCSP: true, viewport: { width: 800, height: 800 } });
  await a.page.route('**/web/test-schrift.ttf', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.fulfill({ path: '/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf', contentType: 'font/ttf' });
  });
  await beitreten(a.page);
  const juengste = await juengsteZeit(instanz.adresse, 'gross');
  await expect(link(a.page, 'gross')).toBeVisible();

  await a.page.addStyleTag({ content: '@font-face { font-family: "IBM Plex Sans"; src: url("/web/test-schrift.ttf"); }' });
  await link(a.page, 'gross').click();
  await expect(nachrichten(a.page).last()).toHaveAttribute('data-zeit', juengste);
  const vorher = await verlauf(a.page).evaluate((log) => ({ hoehe: log.scrollHeight, schrift: document.fonts.status }));
  await a.page.evaluate(() => document.fonts.ready);
  await a.page.evaluate(() => new Promise((weiter) => requestAnimationFrame(() => requestAnimationFrame(weiter))));

  const hoeheNachher = await verlauf(a.page).evaluate((log) => log.scrollHeight);
  expect(vorher.schrift).toBe('loading');
  expect(hoeheNachher).not.toBe(vorher.hoehe);
  expect(await ganzSichtbar(nachrichten(a.page).last())).toBe(true);
  expect(a.fehler).toEqual([]);
});

test('Fehler (US-13): ist der Server weg, sagt die Mitte „Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.“ (R00037) mit ⚠ in kd-danger', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'allgemein', US9));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await expect(link(a.page, 'allgemein')).toBeVisible();

  await instanz.beenden();
  await link(a.page, 'allgemein').click();

  const meldung = a.page.locator('.lc-center .lc-fehler');
  await expect(meldung).toHaveText('⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.');
  await expect(a.page.locator('.lc-center')).not.toContainText('Noch keine Nachrichten');
  const farben = await meldung.evaluate((element) => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--kd-danger)';
    document.body.append(probe);
    const gefahr = getComputedStyle(probe).color;
    probe.remove();
    return { text: getComputedStyle(element).color, gefahr };
  });
  expect(farben.text).toBe(farben.gefahr);
});

// Abrufe der neuesten 100 (Öffnen) für einen Channel; Nachladen trägt vor= bzw. nach= und zählt nicht.
function oeffnungenZaehlen(page, channel) {
  const oeffnungen = [];
  page.on('request', (anfrage) => {
    const ziel = new URL(anfrage.url());
    if (ziel.pathname === `/api/channels/${channel}/nachrichten` && ziel.search === '?n=100') {
      oeffnungen.push(ziel.href);
    }
  });
  return oeffnungen;
}

// UX S4 (R00007: „eine neue Wahl versucht es erneut“): nach „Verlauf nicht geladen“ öffnet auch die Wahl des schon
// gewählten Channels neu — per Klick und per Enter auf dem Eintrag; Variante: erst ein zweiter Fehlschlag.
const klicken = (page) => link(page, 'allgemein').click();
const enterDruecken = async (page) => { await link(page, 'allgemein').focus(); await page.keyboard.press('Enter'); };
for (const [weg, waehlen, fehlschlaege] of [['Klick', klicken, 1], ['Enter', enterDruecken, 1], ['Klick, zweimal gescheitert', klicken, 2]]) {
  test(`Fehler (US-13, ${weg}): nach „Verlauf nicht geladen“ versucht eine neue Wahl desselben Channels es erneut`, async ({ browser }) => {
    await starten((ordner) => channelAnlegen(ordner, 'allgemein', US9));
    const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    const page = await kontext.newPage();
    const fehler = fehlerSammeln(page, { serverWegErwartet: true });
    await beitreten(page);
    const oeffnungen = oeffnungenZaehlen(page, 'allgemein');
    const meldung = page.locator('.lc-center .lc-fehler');
    await page.route('**/api/channels/allgemein/nachrichten?n=100', (route) => route.abort('connectionrefused'));

    for (let versuch = 1; versuch <= fehlschlaege; versuch += 1) {
      await waehlen(page);
      await expect.poll(() => oeffnungen.length).toBe(versuch);
      await expect(meldung).toHaveText('⚠ Verlauf nicht geladen — Channel erneut wählen, um es noch einmal zu versuchen.');
    }
    await page.unroute('**/api/channels/allgemein/nachrichten?n=100');
    await waehlen(page);

    await expect(nachrichten(page)).toHaveCount(3);
    await expect(meldung).toHaveCount(0);
    await waehlen(page);
    await page.waitForTimeout(300);
    expect(oeffnungen.length).toBe(fehlschlaege + 1);
    await expect(nachrichten(page)).toHaveCount(3);
    expect(fehler).toEqual([]);
  });
}

test('Kontrollpaar zu S4: ohne vorherigen Fehler löst die Wahl des gewählten Channels keinen zweiten Abruf aus', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'allgemein', US9));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  const oeffnungen = oeffnungenZaehlen(a.page, 'allgemein');

  await link(a.page, 'allgemein').click();
  await expect(nachrichten(a.page)).toHaveCount(3);
  await link(a.page, 'allgemein').click();
  await link(a.page, 'allgemein').press('Enter');
  await a.page.waitForTimeout(500);

  expect(oeffnungen.length).toBe(1);
  await expect(nachrichten(a.page)).toHaveCount(3);
  expect(a.fehler).toEqual([]);
});

test('Fokus: Tab von der Seitenleiste erreicht den Verlauf (role=log, aria-live=off) auf seiner Tab-Stopp-Nachricht (R00041 F0141), Bild ↑ scrollt ihn', async ({ browser }) => {
  await starten((ordner) => grossAnlegen(ordner));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await link(a.page, 'gross').click();
  await expect(nachrichten(a.page)).toHaveCount(100);
  await expect(verlauf(a.page)).toHaveAttribute('aria-live', 'off');

  await link(a.page, 'gross').focus();
  await a.page.keyboard.press('Tab');
  await expect(verlauf(a.page).locator(':focus')).toHaveCount(1);
  const vorher = await verlauf(a.page).evaluate((log) => log.scrollTop);
  await a.page.keyboard.press('PageUp');

  await expect.poll(() => verlauf(a.page).evaluate((log) => log.scrollTop)).toBeLessThan(vorher);
  const fokus = await a.page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);
  expect(fokus).toBe('solid');
  expect(a.fehler).toEqual([]);
});

// R00011 (ADR-0038 Regel 8) setzt die Eingabe zwischen Verlauf und Teilnehmerliste; R00027 (Ergänzung Karte #41 K7)
// den Knopf „Senden“ hinter das Feld.
test('Tab-Reihenfolge: Kopfzeile → Seitenleiste → Verlauf (die eine Tab-Stopp-Nachricht, R00041 F0141) → Eingabe → Senden → Teilnehmerliste', async ({ browser }) => {
  await starten((ordner) => channelAnlegen(ordner, 'allgemein', US9));
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await link(a.page, 'allgemein').click();
  await expect(nachrichten(a.page)).toHaveCount(3);
  await a.page.getByRole('button', { name: /^Teilnehmer/ }).focus();

  const folge = [];
  for (let schritt = 0; schritt < 6; schritt += 1) {
    await a.page.keyboard.press('Tab');
    folge.push(await a.page.evaluate(() => document.activeElement.getAttribute('aria-label') || document.activeElement.textContent));
  }

  expect(folge).toEqual(['Channel anlegen', 'allgemein', `${ALPHA}, 14:40: <b>vier</b>`, 'Nachricht an # allgemein', 'Senden', 'Teilnehmer']);
  expect(a.fehler).toEqual([]);
});

test('Schmal: bei 320 px mit geöffnetem Verlauf voller langer Nachrichten kein Querscrollen, die neueste ganz sichtbar', async ({ browser }) => {
  await starten((ordner) => grossAnlegen(ordner));
  const a = await seiteMitFehlern(browser, { viewport: { width: 320, height: 640 } });
  await beitreten(a.page, 'shoff', `${instanz.adresse}#/c/gross`);
  const juengste = await juengsteZeit(instanz.adresse, 'gross');

  await expect(nachrichten(a.page).last()).toHaveAttribute('data-zeit', juengste);

  const breiten = await a.page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth }));
  expect(breiten.inhalt).toBeLessThanOrEqual(breiten.fenster);
  expect(await ganzSichtbar(nachrichten(a.page).last())).toBe(true);
  expect(a.fehler).toEqual([]);
});

test('Wechsel bricht den laufenden Abruf des vorigen Channels ab (requestfailed, nicht beendet)', async ({ browser }) => {
  await starten((ordner) => {
    grossAnlegen(ordner);
    channelAnlegen(ordner, 'allgemein', US9);
  });
  const a = await seiteMitFehlern(browser);
  await beitreten(a.page);
  await a.page.route('**/api/channels/gross/nachrichten**', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.continue().catch(() => {});
  });
  const ausgang = new Promise((fertig) => {
    a.page.on('requestfinished', (anfrage) => anfrage.url().includes('/gross/') && fertig('beendet'));
    a.page.on('requestfailed', (anfrage) => anfrage.url().includes('/gross/') && fertig('abgebrochen'));
  });

  await link(a.page, 'gross').click();
  await link(a.page, 'allgemein').click();

  expect(await ausgang).toBe('abgebrochen');
  await expect(nachrichten(a.page)).toHaveCount(3);
});
