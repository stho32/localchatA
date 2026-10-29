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
// Nachricht ist der Schalter „Ganze Nachricht anzeigen“; Tab aus dem Verlauf führt auf den der obersten Nachricht. Der
// Schalter liegt vorher im Scrollbereich, aber unter der Leiste — liegt er ganz außerhalb, scrollt Chromium ihn ohnehin
// mittig, und die Probe sähe `scroll-padding-top` nicht (überlebender Mutant). Eigene Variante dazu: der fokussierte
// Schalter liegt ganz unter der Leiste, nicht nur teilweise.
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

  await b.page.locator('.lc-log').evaluate((log) => {
    const schalter = log.querySelector('.lc-foldbar button');
    const versatz = schalter.getBoundingClientRect().top - log.getBoundingClientRect().top;
    log.scrollTop += versatz - 5;
  });
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

/// Z14 aus R00025: Netz von B wieder an → Ansage-Region enthält „Wieder verbunden“, Grenze < 2 s.
const Z14_MS = 2000;

/// Schreibt jeden nicht leeren Inhalt der Ansage-Region mit.
async function ansagenMitschreiben(page) {
  await page.evaluate(() => {
    window.ansagen = [];
    const region = document.querySelector('[role="status"][aria-live="polite"]');
    new MutationObserver(() => {
      if (region.textContent !== '') {
        window.ansagen.push(region.textContent);
      }
    }).observe(region, { childList: true, characterData: true, subtree: true });
  });
  return () => page.evaluate(() => window.ansagen);
}

// „Ansage“ (Z14; Abnahme-Probe UX-Review #22 F2, wörtlich: „Statusregion enthält „Wieder verbunden“ binnen 2 s“): B
// getrennt → die Region enthält „Verbindung verloren“; `beta` sendet 3; Strom wieder → binnen Z14 „Wieder verbunden · 3
// nachgeladen“. Die Uhr startet beim Wiedereinschalten des Netzes; gemessen wird ereignisgenau.
test('Ansage (Z14): „Verbindung verloren“, nach dem Wiederverbinden binnen 2 s „Wieder verbunden · 3 nachgeladen“', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 5 });
  const a = await kontextOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(b.page.getByRole('log').locator('.lc-msg')).toHaveCount(5);
  const angesagt = await ansagenMitschreiben(b.page);
  const token = await beitreten(instanz.adresse, 'beta');

  await netzAus(b);
  await expect.poll(angesagt, { intervals: [25] }).toContain('Verbindung verloren');
  for (const text of ['g1', 'g2', 'g3']) {
    expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
  }
  // Die Ereignisse zu g1–g3 sind verteilt (A zeigt g3), bevor B wieder online geht — sonst käme g3 live an und zählte
  // nach V9 richtig nicht als nachgeladen.
  await expect(a.page.locator('.lc-log-inhalt .lc-msg .lc-body', { hasText: /^g3$/ })).toHaveCount(1);
  const beginn = Date.now();
  await b.kontext.setOffline(false);
  await expect.poll(angesagt, { intervals: [25], timeout: 10000 }).toContain('Wieder verbunden · 3 nachgeladen');
  const dauer = Date.now() - beginn;
  test.info().annotations.push({ type: 'Z14 ms', description: String(dauer) });

  expect(dauer).toBeLessThan(Z14_MS);
  expect((await angesagt()).filter((text) => text === 'Verbindung verloren')).toHaveLength(1);
  expect(unerwartet(b.fehler)).toEqual([]);
});

// „Gescheitertes Nachholen“ (wie R00012 „Gescheitertes Nachholen“): solange das Nachholen scheitert, enthält die
// Ansage-Region kein „Wieder verbunden“; die Warnung zählt dann bis zum nächsten Nachhol-Versuch herunter.
test('Gescheitertes Nachholen: keine Ansage „Wieder verbunden“, solange der Abruf scheitert', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 5 });
  const b = await kontextOeffnen(browser, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [500] } });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(b.page.getByRole('log').locator('.lc-msg')).toHaveCount(5);
  const angesagt = await ansagenMitschreiben(b.page);
  await netzAus(b);
  await b.page.route((adresse) => adresse.searchParams.has('nach'), (route) => route.fulfill({ status: 500, body: '' }));

  await netzAn(b);
  await expect(leiste(b.page)).toHaveText(/^⚠ Verbindung verloren · (neuer Versuch in \d+ s|wird verbunden …)$/, { timeout: 5000 });
  await b.page.waitForTimeout(3500);

  expect((await angesagt()).some((text) => text.includes('Wieder verbunden'))).toBe(false);
  expect(unerwartet(b.fehler)).toEqual([]);
});

// „Abgleich gegen alle bekannten Zeiten“ (Abnahme-Probe Code-Review #22 A-a, wörtlich: „400 Nachrichten, erster Abruf
// hängt, hochscrollen bis der neueste Block abfällt, zurück, Durchlass → jede Zeit genau einmal“). Lesart: `alpha` hat
// 400 Nachrichten; B ist am Ende, Strom getrennt, `beta` sendet 3; Strom wieder, der erste `nach=`-Abruf wird gehalten;
// B scrollt hoch, bis der neueste Block aus dem Fenster fällt (ADR-0026), und zurück zum Ende; dann Durchlass → je
// `data-zeit` genau eine Zeile, g1–g3 je einmal (Mutante „Abgleich nur gegen den jüngsten Block“ wird rot). Mit Blöcken
// zu 100 und Abwurf erst über 5 Blöcken fällt bei 400 Nachrichten kein Block — die Probe nutzt 1.000. Damit die
// Blockgrenze beim Zurückscrollen zwischen g1–g3 fällt, kommen vor der Trennung 98 Nachrichten live an.
test('Abgleich gegen alle bekannten Zeiten: hochgescrollt während des Nachholens, danach jede Zeit genau einmal', async ({ browser }) => {
  test.setTimeout(90000);
  instanz = await sendenStarten({ anzahl: 1000 });
  const b = await kontextOeffnen(browser, { breite: 1280 });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const log = b.page.locator('.lc-log');
  await expect(b.page.getByRole('log').locator('.lc-msg').last()).toBeVisible();
  for (let kennung = 0; kennung < 4; kennung += 1) {
    const live = await beitreten(instanz.adresse, `live${kennung}`);
    for (let nummer = 0; nummer < 25 && kennung * 25 + nummer < 98; nummer += 1) {
      expect((await senden(instanz.adresse, live, 'allgemein', `l${kennung * 25 + nummer}`)).status).toBe(201);
    }
  }
  await expect(b.page.locator('.lc-log-inhalt .lc-msg .lc-body', { hasText: /^l97$/ })).toHaveCount(1);
  const neuesteZeit = await b.page.getByRole('log').locator('.lc-msg').last().getAttribute('data-zeit');
  const token = await beitreten(instanz.adresse, 'beta');
  await netzAus(b);
  for (const text of ['g1', 'g2', 'g3']) {
    expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
  }
  let durchlassen = null;
  const gehalten = new Promise((fertig) => { durchlassen = fertig; });
  let ersterNachholAbruf = true;
  await b.page.route((adresse) => adresse.searchParams.has('nach'), async (route) => {
    if (ersterNachholAbruf) {
      ersterNachholAbruf = false;
      await gehalten;
    }
    await route.continue();
  });

  await netzAn(b);
  const istGeladen = () => log.evaluate((element) => element.dataset.laedt === undefined);
  for (let schritt = 0; schritt < 40; schritt += 1) {
    await log.evaluate((element) => { element.scrollTop = 0; });
    await b.page.waitForTimeout(100);
    await expect.poll(istGeladen).toBe(true);
    if (await b.page.locator(`.lc-msg[data-zeit="${neuesteZeit}"]`).count() === 0) {
      break;
    }
  }
  await expect(b.page.locator(`.lc-msg[data-zeit="${neuesteZeit}"]`)).toHaveCount(0);
  for (let schritt = 0; schritt < 40; schritt += 1) {
    await log.evaluate((element) => { element.scrollTop = element.scrollHeight; });
    await b.page.waitForTimeout(100);
    await expect.poll(istGeladen).toBe(true);
    if (await b.page.locator(`.lc-msg[data-zeit="${neuesteZeit}"]`).count() > 0) {
      await log.evaluate((element) => { element.scrollTop = element.scrollHeight; });
      break;
    }
  }
  await expect(b.page.locator(`.lc-msg[data-zeit="${neuesteZeit}"]`)).toHaveCount(1);
  durchlassen();
  await expect(b.page.locator('.lc-leiste-fehler')).toHaveCount(0, { timeout: 15000 });
  await b.page.waitForTimeout(500);

  const zeiten = await b.page.locator('.lc-log-inhalt .lc-msg[data-zeit]').evaluateAll((zeilen) => zeilen.map((zeile) => zeile.dataset.zeit));
  const doppelt = zeiten.filter((zeit, stelle) => zeiten.indexOf(zeit) !== stelle);
  expect(doppelt).toEqual([]);
  for (const text of ['g1', 'g2', 'g3']) {
    await expect(b.page.locator('.lc-log-inhalt .lc-msg .lc-body', { hasText: new RegExp(`^${text}$`) })).toHaveCount(1);
  }
  expect(unerwartet(b.fehler)).toEqual([]);
});
