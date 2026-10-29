'use strict';
// R00025 F0084 „Eine Verbindungswarnung mit Countdown, angesagt“: gegen das echte Programm, Chromium 1280 × 800. A ist
// `shoff`, B ist `anna`; „Strom getrennt“ ist `context.setOffline(true)`. Countdown-Text, Zeitpunkt und Quelle prüfen
// die Node-Tests mit gestellter Zeit (e2e/einheit/verbindungsleiste.test.mjs, live.test.mjs, verteiler.test.mjs).
const { test, expect } = require('@playwright/test');
const { sendenStarten, kontextOeffnen, imBrowserBeitreten, beitreten, senden, eingabe } = require('./senden-hilfen');
const { leiste, netzAus, netzAn } = require('./r00012-hilfen');
const { GERAETE, geraetOeffnen } = require('./r00027-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

// „Countdown im Tab“: B getrennt → die Warnung nennt „neuer Versuch in“ mit einer Zahl von 1 bis 10 und zählt herunter
// (zwei Lesungen im Abstand von 1 s unterscheiden sich). Lesart: getrennt wird hier der Strom selbst — das Programm
// endet; `setOffline` erreicht den Strom im SharedWorker nicht, dann gilt „Netz aus ohne Meldung des Stroms“ (ohne
// Countdown, Node-Test „geltender Zeitpunkt“).
test('Countdown im Tab: getrennt zählt die Warnung bis zum nächsten Versuch herunter', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser, { serverWegErwartet: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');

  await instanz.stoppen();
  instanz = null;
  await expect(leiste(b.page)).toHaveText(/^⚠ Verbindung verloren · neuer Versuch in ([1-9]|10) s$/, { timeout: 20000 });
  const erste = await leiste(b.page).textContent();
  await b.page.waitForTimeout(1000);
  const zweite = await leiste(b.page).textContent();

  expect(zweite).not.toBe(erste);
  expect(b.fehler).toEqual([]);
});

/// Konsolenfehler außer „Failed to load resource“ zu Anfragen, die bei getrenntem Netz scheitern.
const unerwartet = (fehler) => fehler.filter((zeile) => !zeile.includes('net::ERR_INTERNET_DISCONNECTED'));

/// Sichtbare Elemente (nicht visuell verborgen), deren eigener Text „Verbindung verloren“ oder „Verbindung
/// unterbrochen“ enthält — das innerste je Fundstelle.
function sichtbareWarnungen(page) {
  return page.evaluate(() => [...document.querySelectorAll('body *')].filter((element) => {
    const text = element.textContent;
    const traegt = text.includes('Verbindung verloren') || text.includes('Verbindung unterbrochen');
    const innerstes = ![...element.children].some((kind) => kind.textContent.includes('Verbindung'));
    const rahmen = element.getBoundingClientRect();
    return traegt && innerstes && rahmen.width > 1 && rahmen.height > 1 && getComputedStyle(element).visibility !== 'hidden';
  }).length);
}

const oberkanteDesVerlaufs = (page) => page.locator('.lc-log').evaluate((log) => log.getBoundingClientRect().top);

const EINE_LIVE_REGION = `document.querySelectorAll('[aria-live]:not([aria-live="off"]), [role="status"], [role="alert"], [role="log"]:not([aria-live="off"])').length`;

// „Eine Warnung ohne Sprung“ (Abnahme-Probe UX-Review #22 F1, wörtlich: „genau ein Element, Oberkante .lc-log
// unverändert vor/während/nach der Trennung“): B in # allgemein, Strom getrennt, dann wieder — genau ein sichtbares
// Element mit „Verbindung verloren“ oder „Verbindung unterbrochen“, die Oberkante von .lc-log ändert sich um höchstens
// 1 px. Vor der Härtung rot (zwei Elemente, 37 px).
test('Eine Warnung ohne Sprung: genau ein Element, Oberkante des Verlaufs fest vor, während und nach der Trennung', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const vorher = await oberkanteDesVerlaufs(b.page);

  await netzAus(b);
  const waehrend = await oberkanteDesVerlaufs(b.page);
  const warnungen = await sichtbareWarnungen(b.page);
  await netzAn(b);
  await expect(b.page.locator('.lc-leiste-fehler')).toHaveCount(0, { timeout: 10000 });
  const nachher = await oberkanteDesVerlaufs(b.page);

  expect(warnungen).toBe(1);
  expect(Math.abs(waehrend - vorher)).toBeLessThanOrEqual(1);
  expect(Math.abs(nachher - vorher)).toBeLessThanOrEqual(1);
  expect(unerwartet(b.fehler)).toEqual([]);
});

// „Ohne offenen Channel“: B hat keinen Channel gewählt; getrennt → genau ein sichtbares Element mit „Verbindung
// verloren“; ein Channelwechsel während der Trennung ändert die Zahl nicht (Mutante „Leiste je Channel neu angelegt“).
test('Ohne offenen Channel: eine Warnung, auch nach einem Channelwechsel während der Trennung', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser);
  await b.page.goto(instanz.adresse);
  await b.page.getByLabel('Name').fill('anna');
  await b.page.getByLabel('Name').press('Enter');
  await expect(b.page.locator('.lc-mitte-hinweis')).toHaveText('Links einen Channel wählen.');

  await b.kontext.setOffline(true);
  await expect(leiste(b.page)).toHaveText(/^⚠ Verbindung verloren/);
  const ohneChannel = await sichtbareWarnungen(b.page);
  await b.page.getByRole('navigation').getByRole('link', { name: /allgemein/ }).click();
  await expect(b.page.locator('.lc-log')).toBeVisible();
  const nachWechsel = await sichtbareWarnungen(b.page);

  expect(ohneChannel).toBe(1);
  expect(nachWechsel).toBe(1);
  await expect(b.page.locator('.lc-leiste')).toHaveCount(1);
  expect(unerwartet(b.fehler)).toEqual([]);
});

// „Eine Live-Region“ (UX-Vorprüfung #36 Muss V1, wörtlich: „In der Hauptansicht ohne offenen Dialog gilt vor, während
// und nach einer Trennung, bei sichtbarer Hinweiszeile … `document.querySelectorAll('[aria-live]:not([aria-live="off"]),
// [role="status"], [role="alert"], [role="log"]:not([aria-live="off"])').length === 1`. Der Verlauf hat `role="log"` und
// `aria-live="off"`.“) — den Zustand „Verworfen · Rückgängig“ prüft F0085 („Keine eigene Region“).
test('Eine Live-Region vor, während und nach der Trennung, auch mit sichtbarer Hinweiszeile', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const zaehlen = () => b.page.evaluate(EINE_LIVE_REGION);

  const vorher = await zaehlen();
  await eingabe(b.page).fill('rot \u001b[31m');
  const mitHinweis = await zaehlen();
  await netzAus(b);
  const waehrend = await zaehlen();
  await netzAn(b);
  await expect(b.page.locator('.lc-leiste-fehler')).toHaveCount(0, { timeout: 10000 });
  const nachher = await zaehlen();

  expect([vorher, mitHinweis, waehrend, nachher]).toEqual([1, 1, 1, 1]);
  await expect(b.page.getByRole('log')).toHaveAttribute('aria-live', 'off');
  expect(unerwartet(b.fehler)).toEqual([]);
});

// „Gleiche Höhe“ (V6, wörtlich: „Bei 320, 375 und 414 px hat die Leiste für ‚neuer Versuch in 10 s‘, ‚in 1 s‘,
// ‚verbinde …‘ und ‚⚠ Verbindung verloren‘ dieselbe Höhe (± 1 px); die Ladezeile bewegt sich nicht.“) — die Texte
// erzeugt warnungText aus dem Modul der Seite mit gestellter Zeit; die Ladezeile wird für die Messung eingeblendet.
for (const breite of [320, 375, 414]) {
  test(`Gleiche Höhe (${breite} px): die Leiste ist in allen Texten gleich hoch, die Ladezeile liegt fest darunter`, async ({ browser }) => {
    instanz = await sendenStarten();
    const b = await geraetOeffnen(browser, 'telefon', { fenster: { width: breite, height: 812 } });
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    await netzAus(b);

    const messungen = await b.page.evaluate(async () => {
      const { warnungText } = await import('/web/verbindungsleiste.js');
      const jetzt = 1000000;
      const texte = [warnungText(jetzt + 10000, jetzt), warnungText(jetzt + 1000, jetzt), warnungText(jetzt, jetzt), warnungText(null, jetzt)];
      const leisteElement = document.querySelector('.lc-center > .lc-leiste');
      const ladezeile = document.querySelector('.lc-loadmore');
      ladezeile.hidden = false;
      return texte.map((text) => {
        leisteElement.textContent = text;
        return { text, hoehe: leisteElement.getBoundingClientRect().height, unten: leisteElement.getBoundingClientRect().bottom, ladezeileOben: ladezeile.getBoundingClientRect().top };
      });
    });

    expect(messungen.map((messung) => messung.text)).toEqual([
      '⚠ Verbindung verloren · neuer Versuch in 10 s', '⚠ Verbindung verloren · neuer Versuch in 1 s',
      '⚠ Verbindung verloren · wird verbunden …', '⚠ Verbindung verloren',
    ]);
    const hoehen = messungen.map((messung) => messung.hoehe);
    expect(Math.max(...hoehen) - Math.min(...hoehen)).toBeLessThanOrEqual(1);
    const ladezeile = messungen.map((messung) => messung.ladezeileOben);
    expect(Math.max(...ladezeile) - Math.min(...ladezeile)).toBeLessThanOrEqual(1);
    expect(Math.abs(messungen[0].ladezeileOben - messungen[0].unten)).toBeLessThanOrEqual(1);
    expect(unerwartet(b.fehler)).toEqual([]);
  });
}

// „Fokus unter der Leiste“ (V7, wörtlich: „Getrennt, Fokus per Tastatur auf die oberste Nachricht → ihr Rechteck liegt
// nicht vollständig unter der Leiste.“ Mutante „ohne `scroll-padding-top`“ rot). Lesart: fokussierbar in einer
// Nachricht ist der Schalter „Ganze Nachricht anzeigen“; Tab aus dem Verlauf führt auf den der obersten Nachricht.
// Eigene Variante dazu: der fokussierte Schalter liegt ganz unter der Leiste, nicht nur teilweise.
test('Fokus unter der Leiste: der per Tastatur fokussierte Schalter der obersten Nachricht liegt unter der Leiste', async ({ browser }) => {
  instanz = await sendenStarten();
  const token = await beitreten(instanz.adresse, 'beta');
  for (let nummer = 0; nummer < 12; nummer += 1) {
    await senden(instanz.adresse, token, 'allgemein', Array.from({ length: 30 }, (_, zeile) => `n${nummer} Zeile ${zeile}`).join('\n'));
  }
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(b.page.locator('.lc-foldbar button')).toHaveCount(12);
  await netzAus(b);

  await b.page.locator('.lc-log').focus();
  await b.page.keyboard.press('Tab');

  const lage = await b.page.evaluate(() => {
    const fokus = document.activeElement;
    const nachricht = fokus.closest('.lc-msg').getBoundingClientRect();
    const schalter = fokus.getBoundingClientRect();
    const leisteUnten = document.querySelector('.lc-center > .lc-leiste').getBoundingClientRect().bottom;
    return { inDerErsten: fokus.closest('.lc-msg').textContent.includes('n0 Zeile 0'), nachrichtUnten: nachricht.bottom, schalterOben: schalter.top, leisteUnten };
  });
  expect(lage.inDerErsten).toBe(true);
  expect(lage.nachrichtUnten).toBeGreaterThan(lage.leisteUnten);
  expect(lage.schalterOben).toBeGreaterThanOrEqual(lage.leisteUnten - 1);
  expect(unerwartet(b.fehler)).toEqual([]);
});

// „Viewport“ (ADR-0038 Ergänzung „Karte #36 UX-Vorprüfung“, erster Test, wörtlich: „E2E (Chromium, Gerät „Pixel 7“
// emuliert): `<meta name="viewport">` enthält `interactive-widget=resizes-content` (Regression; die Wirkung prüft nur das
// Gerät).“) — die übrigen Angaben bleiben.
test('Viewport: interactive-widget=resizes-content neben width=device-width, initial-scale=1 (Pixel 7)', async ({ playwright, browser }) => {
  instanz = await sendenStarten();
  const kontext = await browser.newContext({ ...playwright.devices['Pixel 7'] });
  const page = await kontext.newPage();
  await page.goto(instanz.adresse);

  const inhalt = await page.locator('meta[name="viewport"]').getAttribute('content');
  const angaben = inhalt.split(',').map((angabe) => angabe.trim());

  expect(angaben).toEqual(expect.arrayContaining(['width=device-width', 'initial-scale=1', 'interactive-widget=resizes-content']));
  await kontext.close();
});

// „Schmal“ (V13): mit sichtbarer Warnung im Telefon-Kontext bei 320, 375 und 414 px und auf dem Tablet hoch und quer —
// kein Querscrollen, keine Vergrößerung.
for (const [geraet, breite] of [['telefon', 320], ['telefon', 375], ['telefon', 414], ['tablet', 768], ['tabletQuer', 1024]]) {
  test(`Schmal (${geraet} ${breite} px): mit Warnung ohne Querscrollen`, async ({ browser }) => {
    instanz = await sendenStarten();
    const fenster = { width: breite, height: GERAETE[geraet].viewport.height };
    const b = await geraetOeffnen(browser, geraet, { fenster });
    await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
    await instanz.stoppen();
    instanz = null;
    await expect(leiste(b.page)).toHaveText(/^⚠ Verbindung verloren · /);

    const masse = await b.page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      innerWidth: window.innerWidth,
      clientWidth: document.documentElement.clientWidth,
      scale: window.visualViewport.scale,
    }));

    expect(masse.scrollWidth).toBeLessThanOrEqual(masse.innerWidth);
    expect(masse.clientWidth).toBe(breite);
    expect(masse.scale).toBe(1);
  });
}
