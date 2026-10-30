'use strict';
// R00041 F0141 Nachrichten im Verlauf fokussieren (UX M1, PO 129): zugänglicher Name je Nachricht (B0632), Rollfokus
// im Verlauf (B0631), Pfeil hoch in der leeren Eingabe (B0633), Fokus beim Ersetzen (B0634). Konsole streng.
const { test, expect } = require('./rahmen');
const { netzAus, netzAn } = require('./r00012-hilfen');
const {
  sendenStarten, beitreten, senden, anfragen, kontextOeffnen, imBrowserBeitreten, eingabe, nachrichten,
} = require('./senden-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const BETA = '127.0.0.2';

/// Eine fremde Nachricht von `beta` per API, bevor A den Channel öffnet.
async function betaSendet(texte) {
  const token = await beitreten(instanz.adresse, 'beta', { herkunft: BETA });
  for (const text of texte) {
    const antwort = await senden(instanz.adresse, token, 'allgemein', text, BETA);
    expect(antwort.status, JSON.stringify(antwort.daten)).toBe(201);
  }
  return token;
}

test('Zugänglicher Name (B0632): jede bestätigte Nachricht heißt „<Kennung>, <HH:MM>: <Text>“', async ({ browser }) => {
  instanz = await sendenStarten();
  await betaSendet(['Hallo\nzusammen']);
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');

  const nachricht = nachrichten(a.page);
  await expect(nachricht).toHaveCount(1);
  await expect(a.page.getByRole('article', { name: /^beta@127\.0\.0\.2, \d\d:\d\d: Hallo zusammen$/ })).toHaveCount(1);
  expect(a.fehler).toEqual([]);
});

/// Der Fokus steht auf einer Nachricht: ihre `data-zeit` (null, wenn nicht).
const fokussierteZeit = (page) => page.evaluate(() => {
  const aktiv = document.activeElement;
  if (aktiv === null || !aktiv.matches('.lc-msg')) {
    return null;
  }
  return aktiv.dataset.zeit;
});

/// Der Fokus steht sichtbar (Fokusring) auf einer Nachricht, die ganz im Verlauf liegt, nicht unter dem klebenden Tagestrenner.
const fokusSichtbarImVerlauf = (page) => page.evaluate(() => {
  const aktiv = document.activeElement;
  const log = document.querySelector('[role="log"]').getBoundingClientRect();
  const lage = aktiv.getBoundingClientRect();
  const trenner = document.querySelector('.lc-day');
  let oben = log.top;
  if (trenner !== null) {
    oben = Math.max(oben, trenner.getBoundingClientRect().bottom);
  }
  return {
    nachricht: aktiv.matches('.lc-msg'),
    fokusSichtbar: aktiv.matches(':focus-visible'),
    ring: getComputedStyle(aktiv).outlineStyle,
    ganz: lage.top >= oben - 1 && lage.bottom <= log.bottom + 1,
  };
});

/// Was im Verlauf per Tab erreichbar ist (tabindex 0 oder Knopf ohne tabindex -1): die Nachrichten, in denen es liegt.
const tabErreichbarIn = (page) => page.evaluate(() => {
  const erreichbar = [...document.querySelectorAll('.lc-log .lc-msg, .lc-log .lc-msg button')]
    .filter((knoten) => knoten.getAttribute('tabindex') !== '-1' && (knoten.tabIndex >= 0));
  return [...new Set(erreichbar.map((knoten) => knoten.closest('.lc-msg').dataset.zeit))];
});

/// Drückt Tab, bis die Eingabe den Fokus hat (höchstens `grenze` Mal) und liefert, wo der Fokus unterwegs war.
async function tabBisEingabe(page, grenze = 4) {
  const besucht = [];
  for (let schritt = 0; schritt < grenze; schritt += 1) {
    await page.keyboard.press('Tab');
    const stelle = await page.evaluate(() => {
      const aktiv = document.activeElement;
      const nachricht = aktiv.closest('.lc-msg');
      return { eingabe: aktiv.matches('.lc-comp-feld'), nachricht: nachricht === null ? null : nachricht.dataset.zeit };
    });
    besucht.push(stelle.nachricht);
    if (stelle.eingabe) {
      return besucht;
    }
  }
  throw new Error(`Eingabe nach ${grenze} Tab nicht erreicht: ${JSON.stringify(besucht)}`);
}

test('Rollfokus (B0631): genau eine Nachricht ist Tab-Stopp; Tab führt vom Rand in sie und von ihr in die Eingabe', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 20 });
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichten(a.page)).toHaveCount(20);

  await expect(a.page.locator('.lc-log .lc-msg[tabindex="0"]')).toHaveCount(1);
  await expect(a.page.locator('.lc-log .lc-msg[tabindex="-1"]')).toHaveCount(19);
  await expect(a.page.getByRole('log')).toHaveAttribute('tabindex', '-1');
  const neueste = await nachrichten(a.page).last().getAttribute('data-zeit');
  expect(await tabErreichbarIn(a.page)).toEqual([neueste]);

  await a.page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'technik', exact: true }).focus();
  await a.page.keyboard.press('Tab');
  expect(await fokussierteZeit(a.page)).toBe(neueste);
  const unterwegs = await tabBisEingabe(a.page);
  expect(unterwegs.filter((zeit) => zeit !== null && zeit !== neueste)).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Rollfokus (B0631): ↑ und ↓ wechseln die Nachricht, sichtbar und in Sicht; der Tab-Stopp folgt dem Fokus', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 60 });
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichten(a.page)).toHaveCount(60);
  const zeiten = await nachrichten(a.page).evaluateAll((alle) => alle.map((einzeln) => einzeln.dataset.zeit));

  await nachrichten(a.page).last().focus();
  for (let schritt = 1; schritt <= 40; schritt += 1) {
    await a.page.keyboard.press('ArrowUp');
    expect(await fokussierteZeit(a.page)).toBe(zeiten[59 - schritt]);
    const lage = await fokusSichtbarImVerlauf(a.page);
    expect(lage, `Schritt ${schritt}`).toEqual({ nachricht: true, fokusSichtbar: true, ring: 'solid', ganz: true });
  }
  await a.page.keyboard.press('ArrowDown');
  expect(await fokussierteZeit(a.page)).toBe(zeiten[20]);
  expect(await fokusSichtbarImVerlauf(a.page)).toEqual({ nachricht: true, fokusSichtbar: true, ring: 'solid', ganz: true });

  await expect(a.page.locator('.lc-log .lc-msg[tabindex="0"]')).toHaveCount(1);
  expect(await a.page.locator('.lc-log .lc-msg[tabindex="0"]').getAttribute('data-zeit')).toBe(zeiten[20]);
  expect(await tabErreichbarIn(a.page)).toEqual([zeiten[20]]);
  // Weit oben steht die Pille „Zum Neuesten“ zwischen Verlauf und Eingabe: ein Stopp ohne Nachricht.
  const unterwegs = await tabBisEingabe(a.page);
  expect(unterwegs.filter((zeit) => zeit !== null && zeit !== zeiten[20])).toEqual([]);
  // Zurück führt in dieselbe Nachricht (höchstens über ihre Leiste „Ganze Nachricht anzeigen“).
  for (let schritt = 0; schritt < unterwegs.length; schritt += 1) {
    await a.page.keyboard.press('Shift+Tab');
  }
  expect(await a.page.evaluate(() => document.activeElement.closest('.lc-msg')?.dataset.zeit ?? null)).toBe(zeiten[20]);
  expect(a.fehler).toEqual([]);
});

test('Rollfokus (B0631): am Rand bleibt der Fokus; Umschalter und Alt ↑ lassen die Nachricht stehen', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 3 });
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichten(a.page)).toHaveCount(3);
  const zeiten = await nachrichten(a.page).evaluateAll((alle) => alle.map((einzeln) => einzeln.dataset.zeit));

  await nachrichten(a.page).last().focus();
  await a.page.keyboard.press('ArrowDown');
  expect(await fokussierteZeit(a.page)).toBe(zeiten[2]);
  await a.page.keyboard.press('Shift+ArrowUp');
  await a.page.keyboard.press('Control+ArrowUp');
  expect(await fokussierteZeit(a.page)).toBe(zeiten[2]);
  await a.page.keyboard.press('ArrowUp');
  await a.page.keyboard.press('ArrowUp');
  await a.page.keyboard.press('ArrowUp');
  expect(await fokussierteZeit(a.page)).toBe(zeiten[0]);
  await a.page.keyboard.press('Escape');
  await expect(eingabe(a.page)).toBeFocused();
  expect(a.fehler).toEqual([]);
});

test('Rollfokus (B0631): über die Blockgrenze nach oben — der nächste Pfeil trifft die nachgeladene Nachricht', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 250 });
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichten(a.page)).toHaveCount(100);
  const ersteZeit = await nachrichten(a.page).first().getAttribute('data-zeit');

  await nachrichten(a.page).last().focus();
  for (let schritt = 0; schritt < 99; schritt += 1) {
    await a.page.keyboard.press('ArrowUp');
  }
  expect(await fokussierteZeit(a.page)).toBe(ersteZeit);
  await expect.poll(() => nachrichten(a.page).count()).toBeGreaterThan(100);

  await a.page.keyboard.press('ArrowUp');
  const jetzt = await fokussierteZeit(a.page);
  expect(Date.parse(jetzt)).toBeLessThan(Date.parse(ersteZeit));
  expect(await fokusSichtbarImVerlauf(a.page)).toEqual({ nachricht: true, fokusSichtbar: true, ring: 'solid', ganz: true });
  expect(a.fehler).toEqual([]);
});

/// Sendet im Browser `text` und wartet, bis er als bestätigte (nicht mehr vorläufige) Nachricht im Verlauf steht.
async function imBrowserSenden(page, text) {
  await eingabe(page).fill(text);
  await eingabe(page).press('Enter');
  await expect(page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)')
    .filter({ has: page.locator('.lc-body', { hasText: text }) })).toHaveCount(1);
}

const textDerFokussierten = (page) => page.evaluate(() => {
  const aktiv = document.activeElement;
  if (aktiv === null || !aktiv.matches('.lc-msg')) {
    return null;
  }
  return aktiv.querySelector('.lc-body').textContent;
});

test('Pfeil hoch (B0633, US-13): ↑ in der leeren Eingabe trifft die eigene letzte, nicht die neueste; ↑/↓ gehen von dort weiter', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserSenden(a.page, 'Eins');
  await imBrowserSenden(a.page, 'Zwei');
  const betaToken = await beitreten(instanz.adresse, 'beta', { herkunft: BETA });
  expect((await senden(instanz.adresse, betaToken, 'allgemein', 'Drei', BETA)).status).toBe(201);
  await expect(nachrichten(a.page)).toHaveCount(3);

  await eingabe(a.page).focus();
  await a.page.keyboard.press('ArrowUp');
  expect(await textDerFokussierten(a.page)).toBe('Zwei');
  expect(await fokusSichtbarImVerlauf(a.page)).toEqual({ nachricht: true, fokusSichtbar: true, ring: 'solid', ganz: true });
  await a.page.keyboard.press('ArrowUp');
  expect(await textDerFokussierten(a.page)).toBe('Eins');
  await a.page.keyboard.press('ArrowDown');
  expect(await textDerFokussierten(a.page)).toBe('Zwei');
  await a.page.keyboard.press('ArrowDown');
  expect(await textDerFokussierten(a.page)).toBe('Drei');
  expect(a.fehler).toEqual([]);
});

test('Pfeil hoch (B0633): mit Text in der Eingabe bewegt ↑ nur die Schreibmarke; mit Umschalter oder Alt geschieht nichts', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserSenden(a.page, 'Eins');

  await eingabe(a.page).focus();
  await a.page.keyboard.type('Entwurf');
  await a.page.keyboard.press('ArrowUp');
  await expect(eingabe(a.page)).toBeFocused();
  expect(await eingabe(a.page).evaluate((feld) => feld.selectionStart)).toBe(0);

  await eingabe(a.page).fill('');
  await a.page.keyboard.press('Shift+ArrowUp');
  await a.page.keyboard.press('Control+ArrowUp');
  await a.page.keyboard.press('Alt+ArrowUp');
  await expect(eingabe(a.page)).toBeFocused();
  expect(a.fehler).toEqual([]);
});

test('Pfeil hoch (B0633): ohne eigene Nachricht bleibt der Fokus in der Eingabe', async ({ browser }) => {
  instanz = await sendenStarten();
  await betaSendet(['Hallo', 'Welt']);
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichten(a.page)).toHaveCount(2);

  await eingabe(a.page).focus();
  await a.page.keyboard.press('ArrowUp');
  await a.page.waitForTimeout(200);
  await expect(eingabe(a.page)).toBeFocused();
  expect(await fokussierteZeit(a.page)).toBeNull();
  expect(a.fehler).toEqual([]);
});

test('Pfeil hoch (B0633): eine gelöschte eigene letzte Nachricht wird übersprungen', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserSenden(a.page, 'Eins');
  await imBrowserSenden(a.page, 'Zwei');
  const zweiZeit = await nachrichten(a.page).last().getAttribute('data-zeit');
  const token = await a.page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  const geloescht = await anfragen(instanz.adresse, {
    methode: 'POST', pfad: 'api/channels/allgemein/nachrichten/loeschen', koerper: { zeit: zweiZeit }, token,
  });
  expect(geloescht.status, JSON.stringify(geloescht.daten)).toBe(200);

  await a.page.reload();
  await expect(nachrichten(a.page)).toHaveCount(2);
  await eingabe(a.page).focus();
  await a.page.keyboard.press('ArrowUp');
  expect(await textDerFokussierten(a.page)).toBe('Eins');
  expect(a.fehler).toEqual([]);
});

/// Hält die Abrufe des Nachholens (`nach` und `n=200`) an, bis `freigeben()` gerufen wird.
async function nachholenAnhalten(page) {
  let freigeben;
  const tor = new Promise((offen) => { freigeben = offen; });
  await page.route((url) => url.pathname.endsWith('/nachrichten') && url.searchParams.has('nach') && url.searchParams.get('n') === '200',
    async (route) => {
      await tor;
      await route.continue();
    });
  return freigeben;
}

test('Fokus beim Ersetzen (B0634): das Nachholen zeichnet den jüngsten Block neu — der Fokus steht danach auf dem neuen Knoten derselben Zeit', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 3 });
  const a = await kontextOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichten(a.page)).toHaveCount(3);
  const token = await beitreten(instanz.adresse, 'beta', { herkunft: BETA });
  await netzAus(a);
  expect((await senden(instanz.adresse, token, 'allgemein', 'l1', BETA)).status).toBe(201);
  const freigeben = await nachholenAnhalten(a.page);
  await netzAn(a);
  expect((await senden(instanz.adresse, token, 'allgemein', 'l2', BETA)).status).toBe(201);
  await expect(mitTextLive(a.page, 'l2')).toHaveCount(1);

  await mitTextLive(a.page, 'l2').focus();
  const zeit = await fokussierteZeit(a.page);
  expect(zeit).not.toBeNull();
  // Der Fokus kommt mit den Pfeilen zurück auf l2 — der Rückzug des Fokus von l1 darf ihn nicht vergessen machen.
  await a.page.keyboard.press('ArrowUp');
  await a.page.keyboard.press('ArrowDown');
  expect(await fokussierteZeit(a.page)).toBe(zeit);
  await a.page.waitForTimeout(100);
  await a.page.evaluate(() => { window.alterKnoten = document.activeElement; });

  freigeben();
  await expect(mitTextLive(a.page, 'l1')).toHaveCount(1);
  await expect(nachrichten(a.page)).toHaveCount(5);
  const lage = await a.page.evaluate(() => ({
    altVerbunden: window.alterKnoten.isConnected,
    zeit: document.activeElement.dataset.zeit ?? null,
    neuerKnoten: document.activeElement !== window.alterKnoten,
  }));
  expect(lage).toEqual({ altVerbunden: false, zeit, neuerKnoten: true });
  expect(a.fehler.filter((zeile) => !/ERR_INTERNET_DISCONNECTED/.test(zeile))).toEqual([]);
});

const mitTextLive = (page, text) => nachrichten(page).filter({ has: page.locator('.lc-body', { hasText: new RegExp(`^${text}$`) }) });
