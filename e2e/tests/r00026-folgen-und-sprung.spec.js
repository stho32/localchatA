'use strict';
// R00026 F0089 Folgen und Sprung mit Tests belegt: gegen das echte Programm, Chromium 1280 × 800, Uhr ab 2026-10-28
// 15:00 (+01:00). A ist `shoff`, B ist `anna`; `alpha` und `beta` senden per API.
const { test, expect } = require('./rahmen');
const {
  sendenStarten, beitreten, senden, kontextOeffnen, imBrowserBeitreten, nachrichten, mitText,
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

/// Wartet, bis mindestens ein Frame vergangen ist, in dem Scrollereignisse der Seite zugestellt wurden.
function frameAbwarten(page) {
  return page.evaluate(() => new Promise((fertig) => {
    requestAnimationFrame(() => requestAnimationFrame(() => fertig()));
  }));
}

/// B in `allgemein` mit `anzahl` Nachrichten am Ende; liefert Seite, Verlauf, Fehler und die Token von `alpha`, `beta`.
async function bAmEnde(browser, anzahl) {
  instanz = await sendenStarten({ anzahl });
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const log = b.page.getByRole('log');
  await expect(nachrichten(b.page)).not.toHaveCount(0);
  await expect.poll(() => abstandZumEnde(log)).toBeLessThanOrEqual(1);
  const alpha = await beitreten(instanz.adresse, 'alpha');
  const beta = await beitreten(instanz.adresse, 'beta');
  return { page: b.page, log, fehler: b.fehler, alpha, beta };
}

/// Sendet `text`, wartet, bis die Nachricht in B gezeichnet ist und danach mindestens ein Frame vergangen ist.
async function sendenUndAbwarten(page, token, text) {
  await senden(instanz.adresse, token, 'allgemein', text);
  await expect(mitText(page, new RegExp(`^${text}$`))).toHaveCount(1);
  await frameAbwarten(page);
}

// „Folgen ohne erste neue“ (B0577; Abnahme-Probe aus dem Code-Review #30, Sollte A1, wörtlich): „B am Ende, beta „a“,
// dann „b“, „c“ in getrennten Frames < 1 min → „b“, „c“ ohne .lc-meta.“ Die Mutante „Wächter `lage === 'zaehlen'`
// entfernt“ gibt `b` die Marke der ersten neuen (und damit den Kopf), sobald das Scrollereignis nach `a` den Stand
// `ersteNeue === null` hergestellt hat — deshalb jede Nachricht erst nach einem vergangenen Frame.
test('Folgen: am Ende gefolgte Nachrichten desselben Absenders haben keinen eigenen Kopf, keine Pille', async ({ browser }) => {
  const { page, log, fehler, alpha, beta } = await bAmEnde(browser, 300);
  await sendenUndAbwarten(page, alpha, 'x');

  await sendenUndAbwarten(page, beta, 'a');
  await sendenUndAbwarten(page, beta, 'b');
  await sendenUndAbwarten(page, beta, 'c');

  await expect(mitText(page, /^a$/).locator('.lc-meta .lc-id')).toHaveText('beta@127.0.0.1');
  await expect(mitText(page, /^b$/).locator('.lc-meta')).toHaveCount(0);
  await expect(mitText(page, /^c$/).locator('.lc-meta')).toHaveCount(0);
  await expect.poll(() => abstandZumEnde(log)).toBeLessThanOrEqual(1);
  await expect(pille(page)).toBeHidden();
  expect(fehler).toEqual([]);
});
