'use strict';
// R00041 F0141 Nachrichten im Verlauf fokussieren (UX M1, PO 129): zugänglicher Name je Nachricht (B0632), Rollfokus
// im Verlauf (B0631), Pfeil hoch in der leeren Eingabe (B0633), Fokus beim Ersetzen (B0634). Konsole streng.
const { test, expect } = require('./rahmen');
const {
  sendenStarten, beitreten, senden, kontextOeffnen, imBrowserBeitreten, eingabe, nachrichten,
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
