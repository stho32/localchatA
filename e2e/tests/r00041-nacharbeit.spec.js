'use strict';
// R00041 Nacharbeit #68 (code-reviewer B1, B2, A2, A3; ux-reviewer S3): das Nachholen frischt das ganze Fenster auf
// (F0134), das offene Feld steht bei der Rückkehr in den Channel wieder im Verlauf (F0142), die Listener der Ansicht
// hängen an ihrem Signal, Löschen sendet höchstens einen Aufruf zugleich, eigenes Löschen sagt nicht „inzwischen“.
const { test, expect } = require('./rahmen');
const { netzAus, netzAn, leiste } = require('./r00012-hilfen');
const {
  sendenStarten, imBrowserBeitreten, beitreten, senden, kontextOeffnen, eingabe,
} = require('./senden-hilfen');
const { channelAnlegen } = require('./hilfen');
const {
  betaVerlauf, betaSendet, bearbeitenPerApi, loeschenPerApi, browserOeffnen, imBrowserSenden, nachrichtMitZeit,
  ansagenMitschreiben,
} = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const feld = (page) => page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' });
const ohneNetzfehler = (fehler) => fehler.filter((zeile) => !/ERR_INTERNET_DISCONNECTED/.test(zeile));

const auswahl = (page) => feld(page).evaluate((textfeld) => [textfeld.selectionStart, textfeld.selectionEnd]);

async function channelWaehlen(page, name) {
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name, exact: true }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', `Verlauf # ${name}`);
}

function bearbeitungenZaehlen(page) {
  const liste = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten/bearbeiten')) {
      liste.push(anfrage.postDataJSON());
    }
  });
  return liste;
}

/// `allgemein` mit `anzahl` Nachrichten von `shoff@127.0.0.1` (eine je Sekunde ab 14:00) — A tritt im Browser als
/// `shoff` von 127.0.0.1 bei und kann sie bearbeiten; → { zeiten, vorbereiten }.
function eigenerVerlauf(anzahl) {
  const zeiten = Array.from({ length: anzahl }, (_, nummer) => {
    const minute = String(Math.floor(nummer / 60)).padStart(2, '0');
    const sekunde = String(nummer % 60).padStart(2, '0');
    return `2026-10-28T14:${minute}:${sekunde}.000+01:00`;
  });
  return {
    zeiten,
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein', {
      '2026-10-28': zeiten.map((zeit, nummer) => [zeit, 'shoff@127.0.0.1', `Nachricht ${nummer + 1}`]),
    }),
  };
}

/// Vier Agenten von 127.0.0.2 senden je `jeAgent` Nachrichten nach `allgemein` (unter dem Vorrat des Schreibkontos).
async function agentenSenden(adresse, jeAgent) {
  for (const name of ['agent-a', 'agent-b', 'agent-c', 'agent-d']) {
    const token = await beitreten(adresse, name, { herkunft: '127.0.0.2' });
    for (let nummer = 0; nummer < jeAgent; nummer += 1) {
      expect((await senden(adresse, token, 'allgemein', `${name} ${nummer}`, '127.0.0.2')).status).toBe(201);
    }
  }
}

/// Das Feld ist sichtbar im Verlauf, fokussiert, mit Text und Schreibmarke.
async function feldStehtWieder(page, { text, marke }) {
  await expect(feld(page)).toHaveCount(1);
  await expect(feld(page)).toBeFocused();
  await expect(feld(page)).toBeInViewport();
  await expect(feld(page)).toHaveValue(text);
  expect(await auswahl(page)).toEqual(marke);
}

async function feldOeffnen(page, zeit) {
  await nachrichtMitZeit(page, zeit).hover();
  await nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).click();
  await expect(feld(page)).toBeFocused();
}

/// Scrollt den Verlauf an den Anfang, bis die Nachricht `zeit` im Verlauf steht.
async function nachObenBis(page, zeit) {
  const log = page.getByRole('log');
  for (let schritt = 0; schritt < 80 && await nachrichtMitZeit(page, zeit).count() === 0; schritt += 1) {
    await log.evaluate((verlauf) => { verlauf.scrollTop = 0; });
    await page.waitForTimeout(100);
  }
  await expect(nachrichtMitZeit(page, zeit)).toHaveCount(1);
}

// B1 (F0134, ADR-0048 Regel 11): das offene Feld an der neuesten Nachricht schützt das untere Ende, das Fenster wächst
// beim Scrollen an den Anfang auf acht Blöcke (751 Nachrichten). Offline bearbeitet beta die älteste, löscht die
// zweitälteste und bearbeitet eine aus der Mitte; nach „Wieder verbunden“ zeigt A alle drei Änderungen, ohne Neuladen,
// und das Feld steht mit dem getippten Text.
test('B1 Nachholen frischt das ganze Fenster auf: acht Blöcke, älteste bearbeitet, zweitälteste gelöscht, Mitte bearbeitet', async ({ browser }) => {
  test.setTimeout(90000);
  const verlauf = betaVerlauf(750);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const beta = (await betaSendet(instanz.adresse, [])).token;
  const a = await browserOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const meine = await imBrowserSenden(a.page, 'Meine');
  await feldOeffnen(a.page, meine);
  await a.page.keyboard.type(' getippt');
  const [aelteste, zweite, mitte] = [verlauf.zeiten[0], verlauf.zeiten[1], verlauf.zeiten[150]];
  await nachObenBis(a.page, aelteste);
  await expect(nachrichtMitZeit(a.page, meine)).toHaveCount(1);
  await a.page.evaluate(() => { window.nichtNeuGeladen = true; });

  await netzAus(a);
  expect((await bearbeitenPerApi(instanz.adresse, beta, aelteste, 'Älteste neu')).status).toBe(200);
  expect((await loeschenPerApi(instanz.adresse, beta, zweite)).status).toBe(200);
  expect((await bearbeitenPerApi(instanz.adresse, beta, mitte, 'Mitte neu')).status).toBe(200);
  await netzAn(a);
  await expect(leiste(a.page)).toHaveText(/Wieder verbunden/);

  await expect(nachrichtMitZeit(a.page, aelteste).locator('.lc-body')).toHaveText('Älteste neu');
  await expect(nachrichtMitZeit(a.page, aelteste).locator('.lc-marke', { hasText: 'bearbeitet' })).toHaveCount(1);
  await expect(nachrichtMitZeit(a.page, zweite).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await expect(nachrichtMitZeit(a.page, mitte).locator('.lc-body')).toHaveText('Mitte neu');
  await expect(feld(a.page)).toHaveValue('Meine getippt');
  expect(await a.page.evaluate(() => window.nichtNeuGeladen)).toBe(true);
  expect(ohneNetzfehler(a.fehler)).toEqual([]);
});

// B2 (F0142, PO 124) — Rückkehr in den Channel: (a) die Nachricht des Felds liegt in einem älteren Block.
test('B2 (a) Rückkehr, Feld in einem älteren Block: sichtbar und fokussiert mit Text und Schreibmarke, kein POST', async ({ browser }) => {
  test.setTimeout(60000);
  const verlauf = eigenerVerlauf(300);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenZaehlen(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const alt = verlauf.zeiten[50];
  await nachObenBis(a.page, alt);
  await feldOeffnen(a.page, alt);
  await a.page.keyboard.type(' getippt');
  await feld(a.page).evaluate((textfeld) => textfeld.setSelectionRange(3, 3));

  await channelWaehlen(a.page, 'technik');
  await channelWaehlen(a.page, 'allgemein');

  await feldStehtWieder(a.page, { text: 'Nachricht 51 getippt', marke: [3, 3] });
  await expect(nachrichtMitZeit(a.page, alt).getByRole('textbox')).toHaveCount(1);
  expect(bearbeitungen).toEqual([]);
  expect(a.fehler).toEqual([]);
});

// B2 (b): während A in # technik ist, kommen mehr als 100 neue Nachrichten — die eigene fällt aus dem neuesten Block.
test('B2 (b) Rückkehr nach 108 neuen Nachrichten: Feld sichtbar und fokussiert mit Text und Schreibmarke, kein POST', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenZaehlen(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const meine = await imBrowserSenden(a.page, 'Meine');
  await feldOeffnen(a.page, meine);
  await a.page.keyboard.type(' getippt');
  await feld(a.page).evaluate((textfeld) => textfeld.setSelectionRange(2, 4));
  await channelWaehlen(a.page, 'technik');
  await agentenSenden(instanz.adresse, 27);

  await channelWaehlen(a.page, 'allgemein');

  await feldStehtWieder(a.page, { text: 'Meine getippt', marke: [2, 4] });
  expect(bearbeitungen).toEqual([]);
  expect(a.fehler).toEqual([]);
});

// B2 (c): nach der Rückkehr fokussiert ein zweites Bearbeiten das offene Feld, es entsteht kein zweites.
test('B2 (c) zweites Bearbeiten nach der Rückkehr: kein zweites Feld, Fokus ins offene', async ({ browser }) => {
  test.setTimeout(60000);
  const verlauf = eigenerVerlauf(300);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const alt = verlauf.zeiten[50];
  await nachObenBis(a.page, alt);
  await feldOeffnen(a.page, alt);
  await a.page.keyboard.type(' getippt');
  await channelWaehlen(a.page, 'technik');
  await channelWaehlen(a.page, 'allgemein');
  await expect(feld(a.page)).toBeFocused();

  await nachrichtMitZeit(a.page, verlauf.zeiten[52]).focus();
  await a.page.keyboard.press('e');

  await expect(feld(a.page)).toHaveCount(1);
  await expect(nachrichtMitZeit(a.page, alt).getByRole('textbox')).toBeFocused();
  await expect(feld(a.page)).toHaveValue('Nachricht 51 getippt');
  expect(a.fehler).toEqual([]);
});

// B2 (d): dieselbe Klasse ohne Channelwechsel — „Ende“ holt den neuesten Block, das Feld (Nachricht 51) steht nicht mehr
// im Verlauf; ein zweites Bearbeiten an der neuesten eigenen holt es zurück und fokussiert es.
test('B2 (d) zweites Bearbeiten, während das Feld nicht im Verlauf steht: das Feld kommt zurück und hat den Fokus', async ({ browser }) => {
  test.setTimeout(60000);
  const verlauf = eigenerVerlauf(300);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenZaehlen(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const alt = verlauf.zeiten[50];
  await nachObenBis(a.page, alt);
  await feldOeffnen(a.page, alt);
  await a.page.keyboard.type(' getippt');
  await a.page.getByRole('log').locator('.lc-msg').first().focus();
  await a.page.keyboard.press('End');
  await expect(nachrichtMitZeit(a.page, alt)).toHaveCount(0);

  await nachrichtMitZeit(a.page, verlauf.zeiten[299]).focus();
  await a.page.keyboard.press('e');

  await feldStehtWieder(a.page, { text: 'Nachricht 51 getippt', marke: [20, 20] });
  await expect(nachrichtMitZeit(a.page, alt).getByRole('textbox')).toHaveCount(1);
  expect(bearbeitungen).toEqual([]);
  expect(a.fehler).toEqual([]);
});

// Gegenprobe zu B2: die alte Nachricht wird gelöscht, während A in # technik ist → bei der Rückkehr schließt das Feld mit
// Hinweis (PO 124), nichts bleibt hängen.
test('B2 Gegenprobe: alte Nachricht gelöscht, während A weg ist → Feld zu, „Nachricht gelöscht“, Hinweiszeile', async ({ browser }) => {
  test.setTimeout(60000);
  const verlauf = eigenerVerlauf(300);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const alt = verlauf.zeiten[50];
  await nachObenBis(a.page, alt);
  await feldOeffnen(a.page, alt);
  await a.page.keyboard.type(' getippt');
  await channelWaehlen(a.page, 'technik');
  const token = await a.page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  expect((await loeschenPerApi(instanz.adresse, token, alt, '127.0.0.1')).status).toBe(200);

  await channelWaehlen(a.page, 'allgemein');

  await expect(a.page.locator('.lc-statuszeile')).toHaveText('Die Nachricht wurde inzwischen gelöscht.');
  await expect(feld(a.page)).toHaveCount(0);
  await nachObenBis(a.page, alt);
  await expect(nachrichtMitZeit(a.page, alt).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect(a.fehler).toEqual([]);
});

// A2: verlaufAnlegen und bearbeitenAnlegen hängen ihre Listener an document (pointerdown) und window (resize) an das
// Signal der Ansicht — nach zwei neuen Hauptansichten (401 beim Senden, 409 beim stillen Beitritt, Beitritt unter
// anderem Namen) sind es so viele wie nach der ersten. Gezählt über das DevTools-Protokoll.
async function lauscherZahl(page, ziel, art) {
  const cdp = await page.context().newCDPSession(page);
  const { result } = await cdp.send('Runtime.evaluate', { expression: ziel });
  const { listeners } = await cdp.send('DOMDebugger.getEventListeners', { objectId: result.objectId });
  await cdp.detach();
  return listeners.filter((lauscher) => lauscher.type === art).length;
}

async function lauscherStand(page) {
  return { pointerdown: await lauscherZahl(page, 'document', 'pointerdown'), resize: await lauscherZahl(page, 'window', 'resize') };
}

async function hauptansichtNeuZeichnen(page, name) {
  let erste = true;
  await page.route('**/api/channels/allgemein/nachrichten', async (route) => {
    if (route.request().method() !== 'POST' || !erste) {
      await route.continue();
      return;
    }
    erste = false;
    await route.fulfill({ status: 401, contentType: 'application/json', body: JSON.stringify({ fehler: 'Sitzung unbekannt.' }) });
  });
  await page.route('**/api/beitreten', (route) => route.fulfill({
    status: 409,
    contentType: 'application/json',
    body: JSON.stringify({ fehler: 'Der Name ist vergeben.', belegt_von: '127.0.0.9', seit: '2026-10-28T15:00:00+01:00', vorschlag: name }),
  }), { times: 1 });
  await eingabe(page).fill('eins');
  await eingabe(page).press('Enter');
  await expect(page.getByLabel('Name')).toBeVisible();
  await page.unroute('**/api/channels/allgemein/nachrichten');
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(eingabe(page)).toBeVisible();
}

test('A2 Listener der Ansicht am Signal: nach zwei neuen Hauptansichten so viele pointerdown- und resize-Listener wie nach der ersten', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [401], '/api/beitreten': [409] } });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const vorher = await lauscherStand(a.page);

  await hauptansichtNeuZeichnen(a.page, 'shoff-2');
  await hauptansichtNeuZeichnen(a.page, 'shoff-3');

  expect(await lauscherStand(a.page)).toEqual(vorher);
  expect(a.fehler).toEqual([]);
});

const LOESCHEN = '/api/channels/allgemein/nachrichten/loeschen';
const imDialog = (page, name) => page.getByRole('dialog').getByRole('button', { name, exact: true });

function loeschungenMitschreiben(page) {
  const liste = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && new URL(anfrage.url()).pathname === LOESCHEN) {
      liste.push(anfrage.postData());
    }
  });
  return liste;
}

/// Hält jeden POST …/loeschen, bis `freigeben()` gerufen wird, und reicht ihn dann an die Instanz.
async function loeschenHalten(page) {
  let freigeben = null;
  const freigegeben = new Promise((weiter) => { freigeben = weiter; });
  await page.route(`**${LOESCHEN}`, async (route) => {
    await freigegeben;
    await route.continue();
  });
  return () => freigeben();
}

// A3: die Sperre „höchstens ein Löschen zugleich“ — solange der POST unterwegs ist, öffnet ein zweites Entf die Rückfrage
// erneut, „Löschen“ darin sendet aber keinen zweiten POST.
test('A3 Sperre beim Löschen: POST gehalten, zweites Entf und „Löschen“ → genau ein POST; danach gelöscht', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const loeschungen = loeschungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  const freigeben = await loeschenHalten(a.page);
  await nachrichtMitZeit(a.page, zeit).focus();
  await a.page.keyboard.press('Delete');
  await imDialog(a.page, 'Löschen').click();
  await expect.poll(() => loeschungen.length).toBe(1);

  await nachrichtMitZeit(a.page, zeit).focus();
  await a.page.keyboard.press('Delete');
  await imDialog(a.page, 'Löschen').click();
  await a.page.waitForTimeout(300);
  freigeben();

  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await a.page.waitForTimeout(300);
  expect(loeschungen).toEqual([JSON.stringify({ zeit })]);
  expect(a.fehler).toEqual([]);
});

// UX S3 / Code-Review A4: wer die eigene Nachricht mit offenem Feld selbst löscht (Umschalt+Tab auf die Nachricht, Entf), hört nur „Nachricht gelöscht.“ — keine
// Hinweiszeile „inzwischen gelöscht“, auch wenn das Ereignis des Stroms vor der Antwort des POST ankommt. Die Gegenprobe
// (gelöscht von anderswo) steht in r00041-erhalten.spec.js „Live gelöscht (B0636, UX M3)“.
for (const { titel, antwortVerzoegert } of [{ titel: 'Antwort sofort', antwortVerzoegert: false }, { titel: 'Ereignis vor der Antwort', antwortVerzoegert: true }]) {
  test(`S3 eigenes Löschen bei offenem Feld (${titel}): nur „Nachricht gelöscht.“, keine Hinweiszeile, Feld zu, Fokus auf der Nachricht`, async ({ browser }) => {
    instanz = await sendenStarten();
    const a = await browserOeffnen(browser, { streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const zeit = await imBrowserSenden(a.page, 'Hallo');
    await a.page.waitForTimeout(300);
    if (antwortVerzoegert) {
      await a.page.route(`**${LOESCHEN}`, async (route) => {
        const antwort = await route.fetch();
        await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
        await route.fulfill({ response: antwort });
      });
    }
    const angesagt = await ansagenMitschreiben(a.page);
    await feldOeffnen(a.page, zeit);
    await a.page.keyboard.type(' getippt');

    await a.page.keyboard.press('Shift+Tab');
    await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
    await expect(feld(a.page)).toHaveValue('Hallo getippt');
    await a.page.keyboard.press('Delete');
    await imDialog(a.page, 'Löschen').click();

    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
    await expect(feld(a.page)).toHaveCount(0);
    await expect.poll(angesagt).toEqual(['Nachricht gelöscht.']);
    await a.page.waitForTimeout(300);
    expect(await angesagt()).toEqual(['Nachricht gelöscht.']);
    await expect(a.page.locator('.lc-statuszeile')).not.toHaveText(/inzwischen/);
    await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
    expect(a.fehler).toEqual([]);
  });
}
