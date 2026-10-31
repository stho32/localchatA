'use strict';
// R00041 F0186 B0809 (Karte #85, ux-reviewer-Vorprüfung, Optional #68 O4): der Schalter „Aktionen“ (⋯) jeder Nachricht nennt
// über aria-describedby den Namen seiner Nachricht, damit mehrere Schalter nicht gleich klingen. Probe: sechs Nachrichten,
// Touch; je Schalter zeigt aria-describedby auf ein Element, dessen Text der zugängliche Name der eigenen Nachricht ist; die
// sechs Texte sind paarweise verschieden; nach einem Bearbeiten trägt der Schalter den neuen Namen. Mutante, die rot werden
// muss: aria-describedby entfernt.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { betaVerlauf, betaSendet, browserOeffnen, nachrichtMitZeit, bearbeitenPerApi } = require('./r00041-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Je Nachricht: der Name der Nachricht und der Text des Elements, auf das der Schalter „Aktionen“ zeigt.
function schalterBeschreibungen() {
  return [...document.querySelectorAll('.lc-log .lc-msg[data-zeit]')].map((artikel) => {
    const schalter = artikel.querySelector('.lc-aktionen-schalter');
    const ziele = (schalter.getAttribute('aria-describedby') || '').split(/\s+/).filter((id) => id !== '');
    return {
      zeit: artikel.dataset.zeit,
      name: artikel.getAttribute('aria-label'),
      ziele: ziele.map((id) => {
        const ziel = document.getElementById(id);
        return ziel === null ? null : ziel.textContent;
      }),
    };
  });
}

test('Touch (B0809): sechs Schalter „Aktionen“ nennen je den Namen ihrer Nachricht, paarweise verschieden', async ({ browser }) => {
  const verlauf = betaVerlauf(6);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await geraetOeffnen(browser, 'telefon', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(a.page.getByRole('log').locator('.lc-msg[data-zeit]')).toHaveCount(6);

  const beschreibungen = await a.page.evaluate(schalterBeschreibungen);
  expect(beschreibungen).toHaveLength(6);
  for (const { name, ziele } of beschreibungen) {
    expect(name).toMatch(/^beta@127\.0\.0\.2, 14:00: Nachricht \d$/);
    expect(ziele).toEqual([name]);
  }
  expect(new Set(beschreibungen.map((einzeln) => einzeln.ziele[0])).size).toBe(6);
  for (const zeit of verlauf.zeiten) {
    await expect(nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Aktionen', exact: true }))
      .toHaveAccessibleDescription(new RegExp(`^beta@127\\.0\\.0\\.2, 14:00: Nachricht ${verlauf.zeiten.indexOf(zeit) + 1}$`));
  }
  expect(a.fehler).toEqual([]);
});

test('Touch (B0809): eine bearbeitete Nachricht liefert dem Schalter ihren neuen Namen', async ({ browser }) => {
  const verlauf = betaVerlauf(2);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await geraetOeffnen(browser, 'telefon', { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(a.page.getByRole('log').locator('.lc-msg[data-zeit]')).toHaveCount(2);
  const beta = await betaSendet(instanz.adresse, ['Neu']);
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0])).toHaveCount(1);
  const antwort = await bearbeitenPerApi(instanz.adresse, beta.token, beta.zeiten[0], 'Geändert');
  expect(antwort.status).toBe(200);
  const schalter = nachrichtMitZeit(a.page, beta.zeiten[0]).getByRole('button', { name: 'Aktionen', exact: true });
  await expect(schalter).toHaveAccessibleDescription(/, bearbeitet: Geändert$/);
  expect(a.fehler).toEqual([]);
});
