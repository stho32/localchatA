'use strict';
// R00011 F0043 Eingabe und eigene Nachricht im Browser (US-11 bis US-16, Z3): gegen das echte Programm, Chromium
// 1280 × 800, Uhr ab 2026-10-28 15:00 (+01:00). Browser A ist `shoff` (Mensch).
const { test, expect } = require('@playwright/test');
const {
  sendenStarten, beitreten, verlaufLesen, kontextOeffnen, imBrowserBeitreten, eingabe, nachrichten,
} = require('./senden-hilfen');

const ROUTE = '**/api/channels/*/nachrichten';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function mitA(browser, starten = {}, sammeln = {}) {
  instanz = await sendenStarten(starten);
  const a = await kontextOeffnen(browser, sammeln);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  return a;
}

async function apiVerlauf(channel = 'allgemein') {
  const token = await beitreten(instanz.adresse, 'leser', { herkunft: '127.0.0.9' });
  return verlaufLesen(instanz.adresse, token, channel, '127.0.0.9');
}

const vorlaeufige = (page) => page.getByRole('log').locator('.lc-msg.lc-vorlaeufig');

// „Senden“ (US-11): Feld mit Namen und Platzhalter; Hallo, Shift Enter, Welt, Enter → leer, eine Zeile hoch, Fokus
// im Feld; im Verlauf von shoff@127.0.0.1 mit zwei Zeilen; die API liefert "Hallo\nWelt" und "art":"mensch".
test('US-11: schreiben und senden', async ({ browser }) => {
  const { page, fehler } = await mitA(browser);
  const feld = eingabe(page);

  await expect(feld).toHaveAttribute('placeholder', 'Nachricht an # allgemein');
  await expect(feld).toHaveAccessibleName('Nachricht an # allgemein');
  const hoeheLeer = await feld.evaluate((element) => element.getBoundingClientRect().height);
  await feld.click();
  await page.keyboard.type('Hallo');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.type('Welt');
  await page.keyboard.press('Enter');

  await expect(feld).toHaveValue('');
  expect(await feld.evaluate((element) => element.getBoundingClientRect().height)).toBe(hoeheLeer);
  expect(await feld.evaluate((element) => document.activeElement === element)).toBe(true);
  const gesendet = nachrichten(page).filter({ hasText: 'Welt' });
  await expect(gesendet).toHaveCount(1);
  await expect(gesendet.locator('.lc-body')).toHaveText('Hallo\nWelt');
  await expect(gesendet.locator('.lc-id')).toHaveText('shoff@127.0.0.1');
  await expect(vorlaeufige(page)).toHaveCount(0);
  const liste = await apiVerlauf();
  expect(liste[liste.length - 1]).toMatchObject({ von: 'shoff@127.0.0.1', art: 'mensch', text: 'Hallo\nWelt' });
  expect(fehler).toEqual([]);
});

// „Vorläufig“ (US-12, Z3): die Antwort um 2 s verzögert → „wird gesendet …“ mit `eins`, solange sie aussteht; danach
// `eins` genau einmal und keine vorläufige mehr.
test('US-12 (Z3): sofort sichtbar, auch wenn der Server langsam antwortet', async ({ browser }) => {
  const { page, fehler } = await mitA(browser);
  let beantwortet = false;
  await page.route(ROUTE, async (route) => {
    if (route.request().method() !== 'POST') {
      await route.continue();
      return;
    }
    const antwort = await route.fetch();
    await new Promise((weiter) => setTimeout(weiter, 2000));
    beantwortet = true;
    await route.fulfill({ response: antwort });
  });

  await eingabe(page).fill('eins');
  await eingabe(page).press('Enter');

  await expect(vorlaeufige(page)).toHaveCount(1);
  await expect(vorlaeufige(page).locator('.lc-body')).toHaveText('eins');
  await expect(vorlaeufige(page).locator('.lc-send')).toContainText('wird gesendet …');
  expect(beantwortet).toBe(false);
  await expect(vorlaeufige(page)).toHaveCount(0, { timeout: 10000 });
  await expect(nachrichten(page).filter({ hasText: 'eins' })).toHaveCount(1);
  expect(fehler).toEqual([]);
});

// „Reihenfolge“ (US-13): 1, 2, 3 direkt nacheinander → je einmal in dieser Folge, API gleich mit steigenden Zeiten;
// kein POST beginnt, bevor der vorige beantwortet ist.
test('US-13: schnell hintereinander bleibt in der Reihenfolge, ein Auftrag zugleich', async ({ browser }) => {
  const { page, fehler } = await mitA(browser);
  const ablauf = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten')) {
      ablauf.push(['beginn', anfrage.postDataJSON().text]);
    }
  });
  page.on('requestfinished', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten')) {
      ablauf.push(['ende', anfrage.postDataJSON().text]);
    }
  });

  for (const text of ['1', '2', '3']) {
    await eingabe(page).fill(text);
    await eingabe(page).press('Enter');
  }

  await expect(vorlaeufige(page)).toHaveCount(0);
  await expect(nachrichten(page).locator('.lc-body')).toHaveText(['1', '2', '3']);
  const liste = await apiVerlauf();
  expect(liste.map((nachricht) => nachricht.text)).toEqual(['1', '2', '3']);
  const zeiten = liste.map((nachricht) => Date.parse(nachricht.zeit));
  expect(zeiten[0] < zeiten[1] && zeiten[1] < zeiten[2]).toBe(true);
  expect(ablauf).toEqual([['beginn', '1'], ['ende', '1'], ['beginn', '2'], ['ende', '2'], ['beginn', '3'], ['ende', '3']]);
  expect(fehler).toEqual([]);
});

// „Leerraum“ (US-16): drei Leerzeichen und Enter → kein POST, kein Fehler, der Text bleibt.
test('US-16: Leerraum sendet nichts und bleibt im Feld', async ({ browser }) => {
  const { page, fehler } = await mitA(browser);
  let gepostet = 0;
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten')) {
      gepostet += 1;
    }
  });

  await eingabe(page).fill('   ');
  await eingabe(page).press('Enter');
  await page.waitForTimeout(300);

  expect(gepostet).toBe(0);
  await expect(eingabe(page)).toHaveValue('   ');
  await expect(vorlaeufige(page)).toHaveCount(0);
  expect(fehler).toEqual([]);
});

// „Senden springt ans Ende“ (US-14): 1.000 Nachrichten; nach oben, bis der neueste Block abgeworfen ist; Fokus ins Feld,
// `hier` senden → Verlauf am Ende, `hier` sichtbar, Fokus im Feld.
test('US-14: Senden holt mich ans Ende', async ({ browser }) => {
  const { page, fehler } = await mitA(browser, { anzahl: 1000 });
  const log = page.getByRole('log');
  await expect(nachrichten(page)).not.toHaveCount(0);
  const letzteZeit = await nachrichten(page).last().getAttribute('data-zeit');
  await expect(async () => {
    await log.evaluate((element) => { element.scrollTop = 0; });
    await expect(log.locator(`.lc-msg[data-zeit="${letzteZeit}"]`)).toHaveCount(0, { timeout: 200 });
  }).toPass({ timeout: 20000 });

  await eingabe(page).click();
  await eingabe(page).fill('hier');
  await eingabe(page).press('Enter');

  const hier = nachrichten(page).filter({ hasText: /^hier$/ });
  await expect(hier).toHaveCount(1, { timeout: 10000 });
  await expect(hier).toBeInViewport();
  await expect.poll(() => log.evaluate((element) => element.scrollHeight - element.scrollTop - element.clientHeight)).toBeLessThanOrEqual(1);
  expect(await eingabe(page).evaluate((element) => document.activeElement === element)).toBe(true);
  expect(fehler).toEqual([]);
});

// „Fehler an der Nachricht“ (US-15): gestellte Antworten 422, 429, 500 und ein Abbruch → vier fehlgeschlagene in
// Sendereihenfolge mit ⚠ und Grund in kd-danger; danach `fünf` bestätigt.
test('US-15: abgewiesen — der Grund steht an der Nachricht, die Schlange läuft weiter', async ({ browser }) => {
  const { page, fehler } = await mitA(browser, {}, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [422, 429, 500] } });
  const gestellt = [
    (route) => route.fulfill({ status: 422, contentType: 'application/json', body: JSON.stringify({ fehler: 'Die Nachricht ist leer.', regel: 'leer' }) }),
    (route) => route.fulfill({
      status: 429, contentType: 'application/json', headers: { 'Retry-After': '3' },
      body: JSON.stringify({ fehler: 'Zu viele Nachrichten von shoff@127.0.0.1 — wieder möglich in 3 s.', grenze: 'kennung', wieder_in_ms: 2500 }),
    }),
    (route) => route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ fehler: '…' }) }),
    (route) => route.abort(),
  ];
  await page.route(ROUTE, async (route) => {
    const naechste = gestellt.shift();
    if (route.request().method() !== 'POST' || naechste === undefined) {
      await route.continue();
      return;
    }
    await naechste(route);
  });

  for (const text of ['eins', 'zwei', 'drei', 'vier']) {
    await eingabe(page).fill(text);
    await eingabe(page).press('Enter');
  }
  await expect(page.getByRole('log').locator('.lc-send.lc-fehlgeschlagen')).toHaveCount(4);
  await eingabe(page).fill('fünf');
  await eingabe(page).press('Enter');

  const fehlgeschlagen = vorlaeufige(page);
  await expect(fehlgeschlagen.locator('.lc-body')).toHaveText(['eins', 'zwei', 'drei', 'vier']);
  await expect(fehlgeschlagen.locator('.lc-send')).toHaveText([
    '⚠ Die Nachricht ist leer.',
    '⚠ Zu viele Nachrichten von shoff@127.0.0.1 — wieder möglich in 3 s.',
    '⚠ Nicht gespeichert — der Server konnte nicht schreiben.',
    '⚠ Keine Antwort vom Server.',
  ]);
  const farben = await fehlgeschlagen.locator('.lc-send').evaluateAll((zeilen) => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--kd-danger)';
    document.body.append(probe);
    const danger = getComputedStyle(probe).color;
    probe.remove();
    return zeilen.map((zeile) => getComputedStyle(zeile).color === danger);
  });
  expect(farben).toEqual([true, true, true, true]);
  await expect(nachrichten(page).filter({ hasText: 'fünf' }).and(page.locator(':not(.lc-vorlaeufig)'))).toHaveCount(1);
  const unerwartet = fehler.filter((zeile) => !(zeile.includes('Failed to load resource: net::ERR_FAILED') && zeile.includes('/nachrichten')));
  expect(unerwartet).toEqual([]);
});

// „Channel-Wechsel“: eine fehlgeschlagene in `allgemein`; nach `technik` und zurück steht sie wieder dort, in
// `technik` nicht.
test('Channel-Wechsel: fehlgeschlagene gehören zu ihrem Channel', async ({ browser }) => {
  const { page, fehler } = await mitA(browser, {}, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [500] } });
  await page.route(ROUTE, (route) => {
    if (route.request().method() !== 'POST') {
      return route.continue();
    }
    return route.fulfill({ status: 500, contentType: 'application/json', body: '{"fehler":"…"}' });
  });
  await eingabe(page).fill('liegen geblieben');
  await eingabe(page).press('Enter');
  await expect(vorlaeufige(page).locator('.lc-send')).toHaveText('⚠ Nicht gespeichert — der Server konnte nicht schreiben.');

  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'technik', exact: true }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(vorlaeufige(page)).toHaveCount(0);
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'allgemein', exact: true }).click();

  await expect(page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await expect(vorlaeufige(page).locator('.lc-body')).toHaveText('liegen geblieben');
  expect(fehler).toEqual([]);
});

// „Leerer Channel“ (US-16): `technik` leer → beide Zeilen.
test('US-16: der leere Channel nennt, wie Agenten hier schreiben', async ({ browser }) => {
  instanz = await sendenStarten();
  const { page, fehler } = await kontextOeffnen(browser);
  await imBrowserBeitreten(page, instanz.adresse, 'shoff', 'technik');

  const log = page.getByRole('log');
  await expect(log.getByText('Noch keine Nachrichten in # technik.')).toBeVisible();
  await expect(log.getByText('Agenten schreiben hier mit POST /api/channels/technik/nachrichten.')).toBeVisible();
  await expect(eingabe(page)).toHaveAttribute('placeholder', 'Nachricht an # technik');
  expect(fehler).toEqual([]);
});

// „Höhe und Schmal“: 20 Zeilen → höchstens 200 px und scrollt in sich; bei 320 px mit Feld und einer fehlgeschlagenen
// Nachricht kein Querscrollen.
test('Höhe und Schmal: das Feld wächst bis 200 px, bei 320 px kein Querscrollen', async ({ browser }) => {
  const { page, fehler } = await mitA(browser, {}, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [500] } });
  const feld = eingabe(page);
  await feld.click();
  for (let zeile = 0; zeile < 20; zeile += 1) {
    await page.keyboard.type(`Zeile ${zeile}`);
    await page.keyboard.press('Shift+Enter');
  }

  const masse = await feld.evaluate((element) => ({ hoehe: element.getBoundingClientRect().height, innen: element.scrollHeight, sicht: element.clientHeight }));
  expect(masse.hoehe).toBeLessThanOrEqual(200);
  expect(masse.innen).toBeGreaterThan(masse.sicht);
  await page.route(ROUTE, (route) => {
    if (route.request().method() !== 'POST') {
      return route.continue();
    }
    return route.fulfill({ status: 500, contentType: 'application/json', body: '{"fehler":"…"}' });
  });
  await feld.fill('Eine sehr lange fehlgeschlagene Nachricht ohne Leerzeichen'.repeat(4));
  await feld.press('Enter');
  await expect(vorlaeufige(page)).toHaveCount(1);
  await page.setViewportSize({ width: 320, height: 800 });

  await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await expect(feld).toBeVisible();
  expect(fehler).toEqual([]);
});
