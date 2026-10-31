'use strict';
// R00030 F0183 / B0811 „Fokus bleibt, wenn ein anderer Anhang gelöscht wird“ — Abnahme-Probe wörtlich (ux-reviewer #85
// Vorprüfung, Optional #43 F0118 O2): Fokus bleibt auf „bericht.pdf laden“, wenn foto.png gelöscht wird, keine Ansage. Aufbau
// (planer): Nachricht mit foto.png und bericht.pdf, Fokus auf „bericht.pdf laden“, `beta` löscht foto.png über die API.
// Gegenprobe (F0118, gilt weiter): wird das Element mit dem Fokus selbst ersetzt, liegt der Fokus auf der Nachricht und die
// Ansage „Anhang gelöscht: <name>“ kommt einmal (r00030-geloescht-live.spec.js).
const { test, expect } = require('./rahmen');
const { nachrichtMitZeit, ansagenMitschreiben } = require('./r00041-hilfen');
const { anzeigeStarten, anhaengeLoeschen } = require('./r00030-anzeige-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ELEMENTE = [
  ['Link „bericht.pdf laden“', ['foto.png', 'bericht.pdf'], (knoten) => knoten.getByRole('link', { name: 'bericht.pdf laden', exact: true }), 'bericht.pdf laden'],
  ['Link „bericht.pdf öffnen“', ['foto.png', 'bericht.pdf'], (knoten) => knoten.getByRole('link', { name: 'bericht.pdf öffnen', exact: true }), 'bericht.pdf öffnen'],
  ['Vorschau von foto.png', ['bericht.pdf', 'foto.png'], (knoten) => knoten.getByRole('button', { name: 'foto.png groß anzeigen', exact: true }), 'foto.png groß anzeigen'],
];

for (const [ort, namen, finden, erwartet] of ELEMENTE) {
  test(`Fokus bleibt (${ort}): ein anderer Anhang wird gelöscht → derselbe Fokus, keine Ansage`, async ({ browser }) => {
    const lauf = await anzeigeStarten(browser, [{ namen }]);
    instanz = lauf.instanz;
    const { page } = lauf.a;
    const knoten = nachrichtMitZeit(page, lauf.zeiten[0]);
    await finden(knoten).focus();
    const ansagen = await ansagenMitschreiben(page);
    const zuLoeschen = namen.indexOf(ort.includes('foto.png') ? 'bericht.pdf' : 'foto.png');

    await anhaengeLoeschen(instanz.adresse, lauf.beta.token, [lauf.ids[0][zuLoeschen]]);

    await expect(knoten.locator('.lc-datei-geloescht')).toHaveCount(1);
    await page.waitForTimeout(500);
    expect(await page.evaluate(() => document.activeElement.getAttribute('aria-label'))).toBe(erwartet);
    expect((await ansagen()).filter((einzeln) => einzeln.includes('Anhang gelöscht'))).toEqual([]);
    expect(lauf.a.fehler).toEqual([]);
  });
}
