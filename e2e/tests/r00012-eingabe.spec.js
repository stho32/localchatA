'use strict';
// R00012 F0048 Entwurf, Längenanzeige und Textprüfung im Browser (US-9 bis US-12, Z12): gegen das echte Programm,
// Chromium 1280 × 800, Uhr ab 2026-10-28 15:00 (+01:00). A ist `shoff`.
const { test, expect } = require('./rahmen');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { bestaetigt, sendungenMitschreiben } = require('./r00012-hilfen');

/// Z12 aus R00012: letzte Taste → Eintrag in sessionStorage.
const Z12_MS = 1000;
const ENTWURF = 'localchatA.entwurf.allgemein';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function mitA(browser, breite = 1280) {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { breite, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  return a;
}

const hinweiszeile = (page) => page.locator('.lc-comp-hinweis');

function farbeVon(page, variable) {
  return page.evaluate((name) => {
    const probe = document.createElement('span');
    probe.style.color = `var(${name})`;
    document.body.append(probe);
    const farbe = getComputedStyle(probe).color;
    probe.remove();
    return farbe;
  }, variable);
}

async function kanal(page, name) {
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', `Verlauf # ${name}`);
}

// „Zu lang“ (US-10): 16 385 × a → Fehler in kd-danger, aria-invalid; Enter sendet nicht, der Text bleibt; ein Zeichen
// gelöscht → „noch 0 Zeichen“, aria-invalid weg, Enter sendet (201).
test('US-10: zu lang — gesagt, bevor gesendet wird', async ({ browser }) => {
  const a = await mitA(browser);
  const sendungen = sendungenMitschreiben(a.page);
  const feld = eingabe(a.page);

  await feld.fill('a'.repeat(16385));
  await expect(hinweiszeile(a.page)).toHaveText('⚠ Zu lang: 16.385 von höchstens 16.384 Byte — bitte teilen.');
  expect(await hinweiszeile(a.page).evaluate((zeile) => getComputedStyle(zeile).color)).toBe(await farbeVon(a.page, '--kd-danger'));
  await expect(feld).toHaveAttribute('aria-invalid', 'true');
  await feld.press('Enter');
  await expect(feld).toHaveValue('a'.repeat(16385));
  expect(sendungen).toEqual([]);
  await feld.press('End');
  await feld.press('Backspace');

  await expect(hinweiszeile(a.page)).toHaveText('noch 0 Zeichen');
  await expect(feld).not.toHaveAttribute('aria-invalid', /.*/);
  const antwort = a.page.waitForResponse((antwort) => antwort.request().method() === 'POST' && antwort.url().endsWith('/nachrichten'));
  await feld.press('Enter');
  expect((await antwort).status()).toBe(201);
  expect(a.fehler).toEqual([]);
});

// „Steuerzeichen“ (US-11): `rot ` + ESC + `[31m` → Hinweis mit Stelle 5, Enter sendet nicht.
test('US-11: eingefügte Farbcodes', async ({ browser }) => {
  const a = await mitA(browser);
  const sendungen = sendungenMitschreiben(a.page);

  await eingabe(a.page).fill('rot \u001b[31m');
  await expect(hinweiszeile(a.page)).toHaveText('⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.');
  await eingabe(a.page).press('Enter');

  await expect(eingabe(a.page)).toHaveValue('rot \u001b[31m');
  expect(sendungen).toEqual([]);
  expect(a.fehler).toEqual([]);
});

// „Ansagen der Hinweiszeile“: von 14.990 auf 16.390 Bytes, je 10 Zeichen auf einmal → der angesagte Inhalt ändert sich
// genau zweimal: beim Überschreiten von 15.000 und beim Wechsel in „Zu lang“. Seit R00025 F0084 (ändert F0048) sagt
// die eine Ansage-Region an, ohne ⚠.
test('Ansagen der Hinweiszeile: nur an der Schwelle und beim Wechsel in einen Fehler', async ({ browser }) => {
  const a = await mitA(browser);
  const feld = eingabe(a.page);
  await feld.fill('a'.repeat(14990));
  await a.page.evaluate(() => {
    window.angesagt = [];
    const region = document.querySelector('[role="status"][aria-live="polite"]');
    new MutationObserver(() => {
      if (region.textContent !== '') {
        window.angesagt.push(region.textContent);
      }
    }).observe(region, { childList: true, characterData: true, subtree: true });
  });
  await feld.press('End');

  for (let bytes = 15000; bytes <= 16390; bytes += 10) {
    await a.page.keyboard.insertText('a'.repeat(10));
  }

  await expect(hinweiszeile(a.page)).toHaveText('⚠ Zu lang: 16.390 von höchstens 16.384 Byte — bitte teilen.');
  await expect.poll(() => a.page.evaluate(() => window.angesagt)).toEqual([
    'noch 1.384 Zeichen',
    'Zu lang: 16.390 von höchstens 16.384 Byte — bitte teilen.',
  ]);
  expect(a.fehler).toEqual([]);
});

// „Entwurf“ (US-9, Z12): `halb` steht spätestens Z12 nach der letzten Taste in sessionStorage; technik leer, zurück
// `halb`; neu laden `halb`; ein neu geöffneter Tab leer; senden und neu laden leer; `x` getippt und gelöscht → kein
// Eintrag; nie in localStorage.
test('US-9 (Z12): der halbe Text bleibt je Channel im Tab', async ({ browser }) => {
  const a = await mitA(browser);
  const feld = eingabe(a.page);
  await feld.click();
  await feld.evaluate((element) => {
    element.addEventListener('input', () => { window.letzteTasteUm = Date.now(); });
  });

  await a.page.keyboard.type('halb');
  const gespeichertUm = await a.page.waitForFunction(
    (schluessel) => window.sessionStorage.getItem(schluessel) === 'halb' && Date.now(),
    ENTWURF, { polling: 'raf', timeout: 10000 },
  ).then((ergebnis) => ergebnis.jsonValue());
  const dauer = gespeichertUm - await a.page.evaluate(() => window.letzteTasteUm);
  console.log(`Z12 gemessen: ${dauer} ms (Grenze ${Z12_MS})`);
  expect(dauer).toBeLessThanOrEqual(Z12_MS);

  await kanal(a.page, 'technik');
  await expect(feld).toHaveValue('');
  await kanal(a.page, 'allgemein');
  await expect(feld).toHaveValue('halb');
  await a.page.reload();
  await expect(eingabe(a.page)).toHaveValue('halb');
  const neuerTab = await a.kontext.newPage();
  await neuerTab.goto(`${instanz.adresse}#/c/allgemein`);
  await expect(neuerTab.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await expect(eingabe(neuerTab)).toHaveValue('');
  await neuerTab.close();

  await eingabe(a.page).press('Enter');
  await expect(bestaetigt(a.page, 'halb')).toHaveCount(1);
  await a.page.reload();
  await expect(eingabe(a.page)).toHaveValue('');
  await eingabe(a.page).click();
  await a.page.keyboard.type('x');
  await a.page.keyboard.press('Backspace');
  await a.page.waitForTimeout(700);
  expect(await a.page.evaluate((schluessel) => window.sessionStorage.getItem(schluessel), ENTWURF)).toBeNull();
  const imLocalStorage = await a.page.evaluate(() => Object.keys(window.localStorage).filter((schluessel) => schluessel.includes('entwurf')));
  expect(imLocalStorage).toEqual([]);
  expect(a.fehler).toEqual([]);
});

// „Fokus“ (US-12): Seitenleiste, `/` → Fokus im Feld, kein `/`; im Feld `/` → getippt; Verlauf, Esc → Feld; Fokus im
// Feld mit Entwurf `a` in allgemein, Alt ↓ nach technik mit Entwurf `b` → Fokus im Feld, Inhalt `b`.
test('US-12: schnell ins Feld', async ({ browser }) => {
  const a = await mitA(browser);
  const feld = eingabe(a.page);
  const seitenleiste = a.page.getByRole('navigation', { name: 'Channels' });

  await seitenleiste.getByRole('link', { name: 'technik', exact: true }).focus();
  await a.page.keyboard.press('/');
  await expect(feld).toBeFocused();
  await expect(feld).toHaveValue('');
  await a.page.keyboard.press('/');
  await expect(feld).toHaveValue('/');
  await feld.fill('');
  await a.page.getByRole('log').focus();
  await a.page.keyboard.press('Escape');
  await expect(feld).toBeFocused();

  await kanal(a.page, 'technik');
  await feld.fill('b');
  await kanal(a.page, 'allgemein');
  await feld.fill('a');
  await feld.focus();
  await a.page.keyboard.press('Alt+ArrowDown');
  await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(feld).toBeFocused();
  await expect(feld).toHaveValue('b');
  expect(a.fehler).toEqual([]);
});

// „Schmal“: bei 320 px mit Fehler in der Hinweiszeile kein Querscrollen.
test('Schmal: Fehler in der Hinweiszeile bei 320 px ohne Querscrollen', async ({ browser }) => {
  const a = await mitA(browser, 320);

  await eingabe(a.page).fill('a'.repeat(16385));
  await expect(hinweiszeile(a.page)).toHaveText(/^⚠ Zu lang/);

  const breiten = await a.page.evaluate(() => [document.documentElement.scrollWidth, window.innerWidth]);
  expect(breiten[0]).toBeLessThanOrEqual(breiten[1]);
  expect(a.fehler).toEqual([]);
});

// Merkt im Browser die Zeit der letzten Taste im Feld und jeden Channelwechsel (hashchange) mit seiner Zeit und dem
// Entwurf von `allgemein` in sessionStorage in diesem Augenblick (nach dem Wechsel der Seite, die zuerst lauscht).
async function tasteUndWechselMerken(page) {
  await page.evaluate((schluessel) => {
    window.wechsel = [];
    document.addEventListener('input', () => { window.letzteTasteUm = Date.now(); }, true);
    window.addEventListener('hashchange', () => {
      window.wechsel.push({ nach: window.location.hash, um: Date.now(), entwurf: window.sessionStorage.getItem(schluessel) });
    });
  }, ENTWURF);
}

async function wechselNachDerTaste(page) {
  return page.evaluate(() => window.wechsel.map(({ nach, um, entwurf }) => ({ nach, nachMs: um - window.letzteTasteUm, entwurf })));
}

// „Entwurf beim schnellen Wechsel“ (US-15; Abnahme-Probe Code-Review #22 B3 b, wörtlich): „A tippt x, wechselt < 200 ms
// nach # technik und zurück → Feld zeigt x; Variante Wechsel + Neuladen.“ Der Wechsel nach # technik liegt vor dem
// Schreiben der Drossel (500 ms), und in seinem Augenblick steht `x` schon in sessionStorage — so belegt der Test das
// Schreiben beim Wechsel (eingabe.js channelSetzen), unabhängig davon, wie schnell der Rückweg ist. Mutante „beim
// Wechsel nicht geschrieben“ wird rot.
const SCHNELL_MS = 200;

test('US-15: x getippt, in weniger als 200 ms nach # technik und zurück — das Feld zeigt x', async ({ browser }) => {
  const a = await mitA(browser);
  const feld = eingabe(a.page);
  await feld.click();
  await tasteUndWechselMerken(a.page);

  await a.page.keyboard.type('x');
  await a.page.keyboard.press('Alt+ArrowDown');
  await a.page.waitForFunction(() => window.wechsel.length === 1, null, { polling: 'raf' });
  await a.page.keyboard.press('Alt+ArrowUp');

  await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await expect(eingabe(a.page)).toHaveValue('x');
  const wechsel = await wechselNachDerTaste(a.page);
  console.log(`US-15 Wechsel nach der Taste: ${JSON.stringify(wechsel)}`);
  expect(wechsel.map(({ nach }) => nach)).toEqual(['#/c/technik', '#/c/allgemein']);
  expect(wechsel[0].nachMs).toBeLessThan(SCHNELL_MS);
  expect(wechsel[0].entwurf).toBe('x');
  expect(a.fehler).toEqual([]);
});

test('US-15: x getippt, in weniger als 200 ms nach # technik, Tab neu geladen, zurück — das Feld zeigt x', async ({ browser }) => {
  const a = await mitA(browser);
  const feld = eingabe(a.page);
  await feld.click();
  await tasteUndWechselMerken(a.page);

  await a.page.keyboard.type('x');
  await a.page.keyboard.press('Alt+ArrowDown');
  await a.page.waitForFunction(() => window.wechsel.length === 1, null, { polling: 'raf' });
  const wechsel = await wechselNachDerTaste(a.page);
  await a.page.reload();
  await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(eingabe(a.page)).toHaveValue('');
  await kanal(a.page, 'allgemein');

  await expect(eingabe(a.page)).toHaveValue('x');
  console.log(`US-15 Wechsel nach der Taste (vor dem Neuladen): ${JSON.stringify(wechsel)}`);
  expect(wechsel.map(({ nach }) => nach)).toEqual(['#/c/technik']);
  expect(wechsel[0].nachMs).toBeLessThan(SCHNELL_MS);
  expect(wechsel[0].entwurf).toBe('x');
  expect(a.fehler).toEqual([]);
});
