'use strict';
// R00027 F0091 Senden-Knopf neben dem Feld — Senden über den Knopf auf demselben Weg wie Enter (B0415, K1, K4):
// Tippen, Klick, Tastatur, Leerraum, Textregel, Doppeltipp, Komposition, Warteschlange, Fehlschlag, ohne Verbindung,
// die vorläufige Nachricht mit genäherter Bildschirmtastatur. Uhr ab 2026-10-28 15:00 (+01:00); A ist `shoff`.
const { test, expect } = require('@playwright/test');
const { sendenStarten, imBrowserBeitreten, eingabe, nachrichten } = require('./senden-hilfen');
const {
  ROUTE, vorlaeufige, bestaetigt, ansageRegion, netzAus, netzAn, sendungenMitschreiben,
} = require('./r00012-hilfen');
const { geraetOeffnen, knopf, ganzImSichtbaren } = require('./r00027-hilfen');

const NACHRICHTEN = '/api/channels/allgemein/nachrichten';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function seiteAuf(browser, geraet, optionen = {}) {
  instanz = await sendenStarten();
  const seite = await geraetOeffnen(browser, geraet, optionen);
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  return seite;
}

/// Hält jede Antwort auf ein POST `ms` zurück, bevor sie weitergeleitet wird (wie R00011 US-12).
async function postVerzoegern(page, ms) {
  await page.route(ROUTE, async (route) => {
    if (route.request().method() === 'POST') {
      await new Promise((weiter) => setTimeout(weiter, ms));
    }
    await route.continue();
  });
}

const feldHatFokus = (page) => eingabe(page).evaluate((feld) => document.activeElement === feld);

/// „Tippen“ auf Touch-Geräten: aufs Feld tippen und den Text über die Tastatur eingeben.
async function tippen(page, text) {
  await eingabe(page).tap();
  await page.keyboard.type(text);
}

for (const geraet of ['telefon', 'tablet']) {
  test(`Tippen sendet (${geraet}): ein POST, erst vorläufig, dann bestätigt, Feld leer mit Fokus, der nie wich, Entwurf gelöscht`, async ({ browser }) => {
    const { page, fehler } = await seiteAuf(browser, geraet);
    const sendungen = sendungenMitschreiben(page);
    await postVerzoegern(page, 1000);
    const hoeheLeer = await eingabe(page).evaluate((feld) => feld.getBoundingClientRect().height);

    await tippen(page, 'Hallo');
    await eingabe(page).evaluate((feld) => {
      window.lcFeldVerlassen = 0;
      feld.addEventListener('blur', () => {
        window.lcFeldVerlassen += 1;
      });
    });
    await knopf(page).tap();

    await expect(vorlaeufige(page).locator('.lc-send')).toHaveText('◌ wird gesendet …');
    expect(await page.evaluate(() => window.lcFeldVerlassen)).toBe(0);
    await expect(bestaetigt(page, 'Hallo')).toHaveCount(1, { timeout: 10000 });
    await expect(vorlaeufige(page)).toHaveCount(0);
    await expect(bestaetigt(page, 'Hallo').locator('.lc-id')).toHaveText('shoff@127.0.0.1');
    expect(sendungen.map((sendung) => sendung.text)).toEqual(['Hallo']);
    await expect(eingabe(page)).toHaveValue('');
    expect(await eingabe(page).evaluate((feld) => feld.getBoundingClientRect().height)).toBe(hoeheLeer);
    expect(await feldHatFokus(page)).toBe(true);
    expect(await page.evaluate(() => sessionStorage.getItem('localchatA.entwurf.allgemein'))).toBeNull();
    expect(fehler).toEqual([]);
  });
}

test('Klick sendet (Desktop): ein POST, bestätigt, der Fokus liegt danach im Feld, nicht auf dem Knopf', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'desktop');
  const sendungen = sendungenMitschreiben(page);

  await eingabe(page).fill('Build ist grün');
  await knopf(page).click();

  await expect(bestaetigt(page, 'Build ist grün')).toHaveCount(1);
  await expect(bestaetigt(page, 'Build ist grün').locator('.lc-id')).toHaveText('shoff@127.0.0.1');
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['Build ist grün']);
  await expect(eingabe(page)).toHaveValue('');
  expect(await feldHatFokus(page)).toBe(true);
  expect(await page.evaluate(() => sessionStorage.getItem('localchatA.entwurf.allgemein'))).toBeNull();
  expect(fehler).toEqual([]);
});

test('Tastatur (Desktop): Tab auf den Knopf, Enter und Leertaste senden, der Fokus kehrt ins Feld; Tab weiter in die Teilnehmerliste', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'desktop');
  const sendungen = sendungenMitschreiben(page);
  await eingabe(page).focus();

  await page.keyboard.type('eins');
  await page.keyboard.press('Tab');
  await expect(knopf(page)).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(bestaetigt(page, 'eins')).toHaveCount(1);
  expect(await feldHatFokus(page)).toBe(true);

  await page.keyboard.type('zwei');
  await page.keyboard.press('Tab');
  await expect(knopf(page)).toBeFocused();
  await page.keyboard.press(' ');
  await expect(bestaetigt(page, 'zwei')).toHaveCount(1);
  expect(await feldHatFokus(page)).toBe(true);

  await page.keyboard.press('Tab');
  await expect(knopf(page)).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(page.locator('#lc-teilnehmer')).toBeFocused();
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['eins', 'zwei']);
  expect(fehler).toEqual([]);
});

// Ein nicht verfügbarer Knopf (aria-disabled) gilt Playwright als „nicht enabled“ und würde nie betätigt: Tippen und
// Klick darauf mit force — ausgelöst werden trotzdem die echten Zeiger-Ereignisse in seiner Mitte.
for (const geraet of ['telefon', 'desktop']) {
  test(`Leer und Leerraum (${geraet}): nicht verfügbar, ohne Wirkung; mit x bereit, leer wieder nicht`, async ({ browser }) => {
    const { page, fehler } = await seiteAuf(browser, geraet);
    const sendungen = sendungenMitschreiben(page);
    let betaetigen = () => knopf(page).click({ force: true });
    if (geraet === 'telefon') {
      betaetigen = () => knopf(page).tap({ force: true });
    }

    await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
    await betaetigen();
    await eingabe(page).focus();
    await page.keyboard.type('   ');
    await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
    await betaetigen();
    await page.waitForTimeout(300);
    expect(sendungen).toEqual([]);
    await expect(vorlaeufige(page)).toHaveCount(0);
    await expect(eingabe(page)).toHaveValue('   ');

    await eingabe(page).focus();
    await page.keyboard.type('x');
    await expect(knopf(page)).not.toHaveAttribute('aria-disabled', /.*/);
    await eingabe(page).fill('');
    await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
    expect(fehler).toEqual([]);
  });
}

for (const [art, text, korrigieren] of [
  ['Länge', 'a'.repeat(16385), ['End', 'Backspace']],
  ['Steuerzeichen', 'rot \u001b[31m', ['End', 'ArrowLeft', 'ArrowLeft', 'ArrowLeft', 'ArrowLeft', 'Backspace']],
]) {
  test(`Textregel (Desktop, ${art}): Fehler in der Hinweiszeile, Knopf nicht verfügbar, Klick ohne POST; korrigiert bereit`, async ({ browser }) => {
    const { page, fehler } = await seiteAuf(browser, 'desktop');
    const sendungen = sendungenMitschreiben(page);

    await eingabe(page).fill(text);
    await expect(page.locator('#lc-comp-hinweis')).toHaveClass(/lc-fehler/);
    await expect(page.locator('#lc-comp-hinweis')).toContainText('⚠');
    await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
    await knopf(page).click({ force: true });
    await page.waitForTimeout(300);
    expect(sendungen).toEqual([]);
    await expect(vorlaeufige(page)).toHaveCount(0);

    await eingabe(page).focus();
    for (const taste of korrigieren) {
      await page.keyboard.press(taste);
    }
    await expect(page.locator('#lc-comp-hinweis')).not.toHaveClass(/lc-fehler/);
    await expect(knopf(page)).not.toHaveAttribute('aria-disabled', /.*/);
    expect(fehler).toEqual([]);
  });
}

test('Doppeltipp (Telefon): zweimal schnell getippt, genau ein POST', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'telefon');
  const sendungen = sendungenMitschreiben(page);

  await tippen(page, 'einmal');
  await knopf(page).tap({ force: true });
  await knopf(page).tap({ force: true });

  await expect(bestaetigt(page, 'einmal')).toHaveCount(1);
  await page.waitForTimeout(300);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['einmal']);
  await expect(nachrichten(page).filter({ hasText: 'einmal' })).toHaveCount(1);
  expect(fehler).toEqual([]);
});

test('Komposition (Telefon): das sichtbare Wort wird gesendet, danach schreibt keine Tastatur es zurück', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'telefon');
  const sendungen = sendungenMitschreiben(page);
  const cdp = await page.context().newCDPSession(page);

  await tippen(page, 'Hallo ');
  await cdp.send('Input.imeSetComposition', { text: 'Wel', selectionStart: 3, selectionEnd: 3 });
  await expect(eingabe(page)).toHaveValue('Hallo Wel');
  await knopf(page).tap();

  await expect(bestaetigt(page, 'Hallo Wel')).toHaveCount(1);
  expect(sendungen.map((sendung) => sendung.text)).toEqual(['Hallo Wel']);
  await expect(eingabe(page)).toHaveValue('');
  expect(await feldHatFokus(page)).toBe(true);
  await cdp.send('Input.insertText', { text: 'x' });
  await expect(eingabe(page)).toHaveValue('x');
  expect(fehler).toEqual([]);
});

test('Warteschlange (Desktop, Antwort 2 s verzögert): 1 per Knopf, 2 per Enter, 3 per Knopf — in Folge, ein Auftrag zugleich', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'desktop');
  const ablauf = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith(NACHRICHTEN)) {
      ablauf.push(['beginn', anfrage.postDataJSON().text]);
    }
  });
  page.on('requestfinished', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith(NACHRICHTEN)) {
      ablauf.push(['ende', anfrage.postDataJSON().text]);
    }
  });
  await postVerzoegern(page, 2000);

  await eingabe(page).fill('1');
  await knopf(page).click();
  await eingabe(page).fill('2');
  await expect(knopf(page)).not.toHaveAttribute('aria-disabled', /.*/);
  await eingabe(page).press('Enter');
  await eingabe(page).fill('3');
  await expect(knopf(page)).not.toHaveAttribute('aria-disabled', /.*/);
  await knopf(page).click();

  await expect(vorlaeufige(page)).toHaveCount(0, { timeout: 15000 });
  await expect(nachrichten(page).locator('.lc-body')).toHaveText(['1', '2', '3']);
  expect(ablauf).toEqual([['beginn', '1'], ['ende', '1'], ['beginn', '2'], ['ende', '2'], ['beginn', '3'], ['ende', '3']]);
  expect(fehler).toEqual([]);
});

test('Fehlschlag (Telefon): 500 → fehlgeschlagen mit Grund und Schaltern wie nach der Taste, einmal angesagt', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'telefon', { zusaetzlich: { [NACHRICHTEN]: [500] } });
  await page.route(ROUTE, async (route) => {
    if (route.request().method() !== 'POST') {
      await route.continue();
      return;
    }
    await route.fulfill({ status: 500, contentType: 'application/json', body: '{"fehler":"intern"}' });
  });
  const angesagt = await ansagenMitschreiben(page);

  await tippen(page, 'kaputt');
  await knopf(page).tap();

  const perKnopf = page.locator('.lc-msg.lc-fehlgeschlagen').filter({ hasText: 'kaputt' });
  await expect(perKnopf.locator('.lc-send')).toHaveText('⚠ Nicht gespeichert — der Server konnte nicht schreiben.');
  await expect(perKnopf.getByRole('button')).toHaveText(['Erneut senden', 'Verwerfen']);
  await expect(nachrichten(page).last()).toContainText('kaputt');
  await expect.poll(() => angesagt(), { timeout: 5000 }).toEqual(['Nachricht nicht gesendet: Nicht gespeichert — der Server konnte nicht schreiben.']);

  await tippen(page, 'taste');
  await page.keyboard.press('Control+Enter');
  const perTaste = page.locator('.lc-msg.lc-fehlgeschlagen').filter({ hasText: 'taste' });
  await expect(perTaste.locator('.lc-send')).toHaveText(await perKnopf.locator('.lc-send').textContent());
  await expect(perTaste.getByRole('button')).toHaveText(['Erneut senden', 'Verwerfen']);
  expect(fehler).toEqual([]);
});

/// Liefert eine Abfrage aller Texte, die die Ansage-Region bisher gesagt hat, die den Grund eines Fehlschlags tragen.
async function ansagenMitschreiben(page) {
  await ansageRegion(page).evaluate((region) => {
    window.lcGesagt = [];
    new MutationObserver(() => {
      if (region.textContent !== '') {
        window.lcGesagt.push(region.textContent);
      }
    }).observe(region, { childList: true, characterData: true, subtree: true });
  });
  return () => page.evaluate(() => window.lcGesagt.filter((text) => text.includes('Nicht gespeichert')));
}

test('Ohne Verbindung (Desktop): per Knopf gesendet wartet die Nachricht und wird nach dem Wiederverbinden bestätigt', async ({ browser }) => {
  const seite = await seiteAuf(browser, 'desktop');
  const { page, fehler } = seite;
  await netzAus(seite);

  await eingabe(page).fill('später');
  await knopf(page).click();
  await expect(vorlaeufige(page).locator('.lc-send')).toHaveText('◌ wird gesendet …');
  await netzAn(seite);

  await expect(bestaetigt(page, 'später')).toHaveCount(1, { timeout: 10000 });
  await expect(vorlaeufige(page)).toHaveCount(0);
  expect(fehler.filter((zeile) => !/net::ERR_(INTERNET_DISCONNECTED|FAILED)/.test(zeile))).toEqual([]);
});

for (const [geraet, fenster] of [['telefon', { width: 375, height: 500 }], ['tabletQuer', { width: 1024, height: 400 }]]) {
  test(`Schmal mit Bildschirmtastatur (${geraet} ${fenster.width} × ${fenster.height}): die eben gesendete vorläufige Nachricht ist ganz sichtbar`, async ({ browser }) => {
    const { page, fehler } = await seiteAuf(browser, geraet, { fenster });
    await postVerzoegern(page, 2000);

    await tippen(page, 'Passt');
    await knopf(page).tap();

    await expect(vorlaeufige(page).locator('.lc-send')).toHaveText('◌ wird gesendet …');
    expect(await ganzImSichtbaren(vorlaeufige(page))).toBe(true);
    expect(fehler).toEqual([]);
  });
}
