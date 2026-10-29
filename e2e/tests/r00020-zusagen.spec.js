'use strict';
// R00020 F0071 Zusagen aus R00011 mit Tests belegt (US-7, US-8, „Folgen knapp über dem Ende“): gegen das echte
// Programm, Chromium 1280 × 800, Uhr ab 2026-10-28 15:00 (+01:00). A ist `shoff`, B ist `anna`; `beta` sendet per API.
const { test, expect } = require('./rahmen');
const {
  sendenStarten, beitreten, senden, kontextOeffnen, imBrowserBeitreten, eingabe, nachrichten, mitText,
} = require('./senden-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const pille = (page) => page.locator('.lc-pillen .lc-pill');

function abstandZumEnde(log) {
  return log.evaluate((element) => element.scrollHeight - element.scrollTop - element.clientHeight);
}

/// B in `allgemein` mit `anzahl` Nachrichten am Ende; liefert Seite, Verlauf, Fehler und das Token von `beta`.
async function bAmEnde(browser, anzahl) {
  instanz = await sendenStarten({ anzahl });
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const log = b.page.getByRole('log');
  await expect(nachrichten(b.page)).not.toHaveCount(0);
  await expect.poll(() => abstandZumEnde(log)).toBeLessThanOrEqual(1);
  const beta = await beitreten(instanz.adresse, 'beta');
  return { page: b.page, log, fehler: b.fehler, beta };
}

/// Stellt den Blick `ueberDemEnde(zeile)` px über das Ende und wartet, bis er dort steht.
async function ueberDasEnde(log, ueberDemEnde) {
  const zeile = await log.evaluate((element) => parseFloat(getComputedStyle(element).lineHeight));
  const abstand = ueberDemEnde(zeile);
  await log.evaluate((element, px) => { element.scrollTop = element.scrollHeight - element.clientHeight - px; }, abstand);
  await expect.poll(async () => Math.abs(await abstandZumEnde(log) - abstand)).toBeLessThanOrEqual(1);
  return abstand;
}

// „Folgen knapp über dem Ende“ (B0339, Code-Review #8 A2): eine halbe Zeile über dem Ende folgt der Blick einer neuen
// Nachricht; eine Zeile + 5 px darüber bleibt er stehen, und die Pille zählt.
test('Folgen: eine halbe Zeile über dem Ende folgt der Blick der neuen Nachricht', async ({ browser }) => {
  const { page, log, fehler, beta } = await bAmEnde(browser, 300);
  await ueberDasEnde(log, (zeile) => zeile / 2);

  await senden(instanz.adresse, beta, 'allgemein', 'knapp darunter');

  await expect(mitText(page, /^knapp darunter$/)).toBeInViewport();
  await expect.poll(() => abstandZumEnde(log)).toBeLessThanOrEqual(1);
  await expect(pille(page)).toBeHidden();
  expect(fehler).toEqual([]);
});

test('Folgen, Gegenprobe: eine Zeile + 5 px über dem Ende bleibt der Blick stehen, die Pille zählt', async ({ browser }) => {
  const { page, log, fehler, beta } = await bAmEnde(browser, 300);
  await ueberDasEnde(log, (zeile) => zeile + 5);
  const vorher = await log.evaluate((element) => element.scrollTop);

  await senden(instanz.adresse, beta, 'allgemein', 'weiter unten');

  await expect(page.getByRole('button', { name: '1 neue Nachricht', exact: true })).toBeVisible();
  await expect(mitText(page, /^weiter unten$/)).toHaveCount(1);
  expect(Math.abs(await log.evaluate((element) => element.scrollTop) - vorher)).toBeLessThanOrEqual(1);
  expect(fehler).toEqual([]);
});

// US-7, „Markup bleibt Text“ (B0340, Security-Review #8 Optional 5): HTML im Text erscheint wörtlich; kein Element aus
// dem Text entsteht, kein Skript läuft.
test('US-7: HTML im Nachrichtentext bleibt Text', async ({ browser }) => {
  const { page, fehler, beta } = await bAmEnde(browser, 5);
  const markup = '<img src=x onerror="window.__xss=1"><b>fett</b><script>window.__xss=2</script>';

  await senden(instanz.adresse, beta, 'allgemein', markup);

  const nachricht = nachrichten(page).last();
  await expect(nachricht.locator('.lc-body')).toHaveText(markup);
  await expect(nachricht.locator('img')).toHaveCount(0);
  await expect(nachricht.locator('b')).toHaveCount(0);
  await expect(nachricht.locator('script')).toHaveCount(0);
  expect(await page.evaluate(() => typeof window.__xss)).toBe('undefined');
  expect(fehler).toEqual([]);
});

// US-8, „Fehlgeschlagene unter allen bestätigten“ (B0350; Abnahme-Probe aus ADR-0037 Ergänzung Karte #31, wörtlich):
// „A sendet „x“, Route antwortet 500; danach sendet `beta` 2 → die beiden stehen über „x“, „x“ ist das letzte Element
// des Verlaufs und zeigt ⚠.“ Die 500 kommt als gestellte Antwort; erlaubt ist dazu genau ihre Konsolenzeile.
test('US-8: die fehlgeschlagene Nachricht bleibt unter allen bestätigten', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 5 });
  const a = await kontextOeffnen(browser, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [500] } });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichten(a.page)).toHaveCount(5);
  await a.page.route('**/api/channels/allgemein/nachrichten', (route) => {
    if (route.request().method() !== 'POST') {
      return route.continue();
    }
    return route.fulfill({ status: 500, contentType: 'application/json', body: '{"fehler":"Interner Fehler"}' });
  });
  await eingabe(a.page).fill('x');
  await eingabe(a.page).press('Enter');
  const x = mitText(a.page, /^x$/);
  await expect(x).toContainText('⚠');
  const beta = await beitreten(instanz.adresse, 'beta');

  await senden(instanz.adresse, beta, 'allgemein', 'eins');
  await senden(instanz.adresse, beta, 'allgemein', 'zwei');

  await expect(mitText(a.page, /^zwei$/)).toHaveCount(1);
  const texte = await nachrichten(a.page).evaluateAll((liste) => liste.map((nachricht) => nachricht.querySelector('.lc-body').textContent));
  expect(texte.slice(-3)).toEqual(['eins', 'zwei', 'x']);
  const letzte = nachrichten(a.page).last();
  await expect(letzte.locator('.lc-body')).toHaveText('x');
  await expect(letzte).toHaveClass(/lc-fehlgeschlagen/);
  await expect(letzte).toContainText('⚠');
  expect(a.fehler).toEqual([]);
});
