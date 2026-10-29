'use strict';
// R00012 F0047 Lücken nach Wiederverbinden (US-6 bis US-8, Z6): gegen das echte Programm, Chromium 1280 × 800, Uhr ab
// 2026-10-28 15:00 (+01:00). A ist `shoff`, B ist `anna`; Agenten senden per API.
const { test, expect } = require('./rahmen');
const { sendenStarten, beitreten, senden, kontextOeffnen, imBrowserBeitreten, eingabe, nachrichten } = require('./senden-hilfen');
const { leiste, bestaetigt, netzAus, netzAn } = require('./r00012-hilfen');

/// Z6 aus R00012: ab Wiederverbinden bis Meldung und Nachrichten sichtbar.
const Z6_MS = 2000;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

function ohneGescheiterteAnfragen(fehler) {
  return fehler.filter((zeile) => !/Failed to load resource: net::ERR_INTERNET_DISCONNECTED/.test(zeile));
}

// „Lücke“ (US-6, Z6): B getrennt, `beta` sendet 3 über die API, Strom wieder → innerhalb Z6 „✓ Wieder verbunden · 3
// nachgeladen“ und die 3 genau einmal in der Folge ihrer Zeiten; die Meldung geht nach 4 s. Die Uhr startet im Browser
// beim Ereignis `online` (Wiederverbinden des Tabs) und endet am ersten Bild mit Meldung und allen drei Nachrichten.
test('US-6 (Z6): Verpasstes wird nachgeholt', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 5 });
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(5);
  const token = await beitreten(instanz.adresse, 'beta');
  await netzAus(b);
  for (const text of ['l1', 'l2', 'l3']) {
    expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
  }
  await b.page.evaluate(() => {
    window.addEventListener('online', () => { window.onlineUm = Date.now(); }, { once: true });
  });

  await netzAn(b);
  const sichtbarUm = await b.page.waitForFunction(() => {
    const meldung = document.querySelector('.lc-leiste');
    const texte = [...document.querySelectorAll('[role="log"] .lc-msg .lc-body')].map((body) => body.textContent);
    const alle = ['l1', 'l2', 'l3'].every((text) => texte.includes(text));
    if (meldung === null || meldung.hidden || meldung.textContent !== '✓ Wieder verbunden · 3 nachgeladen' || !alle) {
      return false;
    }
    return Date.now();
  }, null, { polling: 'raf', timeout: 10000 }).then((ergebnis) => ergebnis.jsonValue());
  const dauer = sichtbarUm - await b.page.evaluate(() => window.onlineUm);

  console.log(`Z6 gemessen: ${dauer} ms (Grenze ${Z6_MS})`);
  expect(dauer).toBeLessThan(Z6_MS);
  await expect(bestaetigt(b.page, 'l[123]').locator('.lc-body')).toHaveText(['l1', 'l2', 'l3']);
  await expect(nachrichten(b.page)).toHaveCount(8);
  await expect(leiste(b.page)).toBeHidden({ timeout: 10000 });
  expect(ohneGescheiterteAnfragen(b.fehler)).toEqual([]);
});

// „Zu viel verpasst“ (US-7): während der Trennung senden 44 Agenten auf 127.0.0.2 bis 127.0.0.12 je 25 (1 100 > 5 × 200;
// je Kennung 25, je Herkunft 100 — kein 429) → B hat neu geöffnet, die neueste ist sichtbar, „· neueste geladen“.
test('US-7: zu viel verpasst — dann die neuesten', async ({ browser }) => {
  test.setTimeout(90000);
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser);
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const agenten = [];
  for (let nummer = 0; nummer < 44; nummer += 1) {
    const herkunft = `127.0.0.${2 + Math.floor(nummer / 4)}`;
    agenten.push({ herkunft, token: await beitreten(instanz.adresse, `a${nummer}`, { herkunft }) });
  }
  await netzAus(b);

  const status = await Promise.all(agenten.map(async ({ herkunft, token }, nummer) => {
    const ergebnisse = [];
    for (let folge = 0; folge < 25; folge += 1) {
      ergebnisse.push((await senden(instanz.adresse, token, 'allgemein', `a${nummer}-${folge}`, herkunft)).status);
    }
    return ergebnisse;
  }));
  const letzte = await senden(instanz.adresse, agenten[0].token, 'allgemein', 'die neueste', agenten[0].herkunft);
  await netzAn(b);

  expect(status.flat().filter((wert) => wert !== 201)).toEqual([]);
  expect(letzte.status).toBe(201);
  await expect(leiste(b.page)).toHaveText('✓ Wieder verbunden · neueste geladen', { timeout: 10000 });
  await expect(bestaetigt(b.page, 'die neueste')).toBeVisible();
  expect(ohneGescheiterteAnfragen(b.fehler)).toEqual([]);
});

// „Fremde vor eigenen“ (US-8; ADR-0037 Pre-Mortem 2): A getrennt schreibt `eigen`, `beta` sendet `f1` und `f2`; Strom
// wieder → A zeigt `f1`, `f2`, `eigen` je einmal in dieser Folge; der POST von `eigen` beginnt erst nach dem Ende des
// Abrufs `nach=`.
test('US-8: Fremdes aus der Trennung steht vor dem Eigenen', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await kontextOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const token = await beitreten(instanz.adresse, 'beta');
  const ablauf = [];
  a.page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten')) {
      ablauf.push('POST beginnt');
    }
  });
  a.page.on('requestfinished', (anfrage) => {
    if (anfrage.method() === 'GET' && anfrage.url().includes('nach=')) {
      ablauf.push('nach= fertig');
    }
  });
  await netzAus(a);

  await eingabe(a.page).fill('eigen');
  await eingabe(a.page).press('Enter');
  for (const text of ['f1', 'f2']) {
    expect((await senden(instanz.adresse, token, 'allgemein', text)).status).toBe(201);
  }
  await netzAn(a);

  await expect(bestaetigt(a.page, '(f1|f2|eigen)').locator('.lc-body')).toHaveText(['f1', 'f2', 'eigen'], { timeout: 10000 });
  await expect(a.page.getByRole('log').locator('.lc-vorlaeufig')).toHaveCount(0);
  expect(ablauf).toEqual(['nach= fertig', 'POST beginnt']);
  expect(ohneGescheiterteAnfragen(a.fehler)).toEqual([]);
});
