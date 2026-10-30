'use strict';
// R00045 F0161 B0688 (Karte #77): der Browser meldet „tippt“ selbst — Netzbeobachtung der Aufrufe an
// /api/channels/{name}/aktivitaet gegen das echte Programm (`allgemein`, `technik`). Die Meldungen gehen vom Tab aus,
// nicht aus dem SharedWorker, und sind deshalb für Playwright sichtbar.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { browserOeffnen, imBrowserSenden, nachrichtMitZeit } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const RUHE_MS = 700;
const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));

/// Schreibt jeden Aufruf der Aktivität als „PUT allgemein {"zustand":"tippt"} → 200“ mit.
function meldungenMitschreiben(page) {
  const meldungen = [];
  page.on('response', (antwort) => {
    const anfrage = antwort.request();
    const treffer = new URL(anfrage.url()).pathname.match(/^\/api\/channels\/([^/]+)\/aktivitaet$/);
    if (treffer === null) {
      return;
    }
    let koerper = '';
    if (anfrage.postData() !== null) {
      koerper = ` ${anfrage.postData()}`;
    }
    meldungen.push(`${anfrage.method()} ${treffer[1]}${koerper} → ${antwort.status()}`);
  });
  return meldungen;
}

const SETZEN = (channel) => `PUT ${channel} {"zustand":"tippt"} → 200`;
const LOESCHEN = (channel) => `DELETE ${channel} → 204`;

async function annaImBrowser(browser) {
  const anna = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  anna.meldungen = meldungenMitschreiben(anna.page);
  return anna;
}

test('Takt (US-4): 10 s ohne Pause tippen → sofort eine Meldung, dann höchstens alle 3 s, zusammen höchstens 5', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaImBrowser(browser);

  await eingabe(anna.page).click();
  const beginn = Date.now();
  await eingabe(anna.page).pressSequentially('x'.repeat(50), { delay: 200 });
  const dauer = Date.now() - beginn;
  await warten(RUHE_MS);

  expect(dauer).toBeGreaterThanOrEqual(10000);
  expect(anna.meldungen.length, anna.meldungen.join('\n')).toBeLessThanOrEqual(5);
  expect(anna.meldungen.length, anna.meldungen.join('\n')).toBeGreaterThanOrEqual(3);
  expect(new Set(anna.meldungen)).toEqual(new Set([SETZEN('allgemein')]));
  expect(anna.fehler).toEqual([]);
});

test('Leeres Feld (US-4): Leerraum meldet nicht; wird das Feld leer, wird die Meldung gelöscht — einmal', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaImBrowser(browser);

  await eingabe(anna.page).click();
  await anna.page.keyboard.press('Space');
  await anna.page.keyboard.press('Shift+Enter');
  await warten(RUHE_MS);
  expect(anna.meldungen).toEqual([]);

  await anna.page.keyboard.type('a');
  await expect.poll(() => anna.meldungen).toEqual([SETZEN('allgemein')]);
  await eingabe(anna.page).press('Control+a');
  await eingabe(anna.page).press('Backspace');
  await expect.poll(() => anna.meldungen).toEqual([SETZEN('allgemein'), LOESCHEN('allgemein')]);
  await anna.page.keyboard.press('Space');
  await anna.page.keyboard.press('Backspace');
  await warten(RUHE_MS);
  expect(anna.meldungen).toEqual([SETZEN('allgemein'), LOESCHEN('allgemein')]);

  await anna.page.keyboard.type('b');
  await expect.poll(() => anna.meldungen).toEqual([SETZEN('allgemein'), LOESCHEN('allgemein'), SETZEN('allgemein')]);
  expect(anna.fehler).toEqual([]);
});

test('Senden (US-4): nach dem Senden kein Löschen, die nächste Taste meldet sofort wieder', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaImBrowser(browser);

  await eingabe(anna.page).pressSequentially('Hallo');
  await eingabe(anna.page).press('Enter');
  await expect(eingabe(anna.page)).toHaveValue('');
  await warten(RUHE_MS);
  expect(anna.meldungen).toEqual([SETZEN('allgemein')]);

  await anna.page.keyboard.type('n');
  await expect.poll(() => anna.meldungen).toEqual([SETZEN('allgemein'), SETZEN('allgemein')]);
  expect(anna.fehler).toEqual([]);
});

test('Channel-Wechsel (US-4): löscht im alten Channel; im neuen meldet die erste Taste sofort', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaImBrowser(browser);

  await eingabe(anna.page).pressSequentially('Entwurf');
  await expect.poll(() => anna.meldungen).toEqual([SETZEN('allgemein')]);
  await anna.page.evaluate(() => { window.location.hash = '#/c/technik'; });
  await expect(anna.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect.poll(() => anna.meldungen).toEqual([SETZEN('allgemein'), LOESCHEN('allgemein')]);

  await eingabe(anna.page).pressSequentially('t');
  await expect.poll(() => anna.meldungen).toEqual([SETZEN('allgemein'), LOESCHEN('allgemein'), SETZEN('technik')]);
  expect(anna.fehler).toEqual([]);
});

test('Kein Signal ohne Taste (ux-reviewer F): Entwurf wiederherstellen, Fokus, gesetzter Feldwert melden nicht; Einfügen ohne Taste meldet', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaImBrowser(browser);
  await eingabe(anna.page).pressSequentially('Entwurf');
  await expect.poll(() => anna.meldungen).toEqual([SETZEN('allgemein')]);
  await warten(RUHE_MS);

  await anna.page.reload();
  const nachDemLaden = meldungenMitschreiben(anna.page);
  await expect(eingabe(anna.page)).toHaveValue('Entwurf');
  await eingabe(anna.page).focus();
  await eingabe(anna.page).evaluate((feld) => { feld.value = 'gesetzt'; });
  await warten(RUHE_MS);
  expect(nachDemLaden).toEqual([]);

  await anna.page.keyboard.insertText(' diktiert');
  await expect.poll(() => nachDemLaden).toEqual([SETZEN('allgemein')]);
  expect(anna.fehler).toEqual([]);
});

test('Bearbeiten-Feld (US-4): Tippen beim Bearbeiten einer eigenen Nachricht meldet nichts', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaImBrowser(browser);
  const zeit = await imBrowserSenden(anna.page, 'Hallo');
  await warten(RUHE_MS);
  const vorher = [...anna.meldungen];

  await nachrichtMitZeit(anna.page, zeit).hover();
  await nachrichtMitZeit(anna.page, zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).click();
  const feld = anna.page.getByRole('textbox', { name: 'Nachricht bearbeiten' });
  await expect(feld).toBeFocused();
  await feld.pressSequentially(' und mehr', { delay: 50 });
  await warten(RUHE_MS);

  expect(anna.meldungen).toEqual(vorher);
  expect(anna.fehler).toEqual([]);
});
