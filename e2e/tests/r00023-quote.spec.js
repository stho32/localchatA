'use strict';
// R00023 F0075 „Im Browser“ (US-4): gegen das echte Programm mit `--plattenquote 1MiB` und dem Quote-Ordner aus der
// Testhilfe (gestern 1.048.276 Bytes); die Quote ist über die API erreicht (drei Sätze zu 81 Bytes, die vierte
// Sendung 507). A ist `shoff`, schreibt „Hallo zusammen“ — an der Quote sind 57 Bytes frei, der Satz von A ist
// länger — und liest den Text der 507 an seiner Nachricht, „Erneut senden“ bleibt.
const path = require('path');
const { execFileSync } = require('child_process');
const { test, expect } = require('./rahmen');
const { instanzStarten } = require('./hilfen');
const {
  JETZT, beitreten, senden, verlaufLesen, kontextOeffnen, imBrowserBeitreten, eingabe,
} = require('./senden-hilfen');
const { vorlaeufige } = require('./r00012-hilfen');

const QUOTE_ORDNER = path.resolve(__dirname, '../../target/debug/examples/quote-ordner');
/// Text zu 17 Zeichen: ein Satz von `beta@127.0.0.1` (Agent) mit 81 Bytes.
const TEXT = 'Quote-Probe 17 Z.';
const TEXT_507 = 'Nicht gesendet: Die Gespräche belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB. Platz entsteht, '
  + 'wenn alte Tage nach ihrer Aufbewahrungsfrist entfallen (beim nächsten Tageswechsel) oder eine Frist verkürzt wird; '
  + 'die Quote stellt der Betreiber beim Start mit --plattenquote ein.';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function anDerQuote() {
  instanz = await instanzStarten({
    argumente: ['--plattenquote', '1MiB'],
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      execFileSync(QUOTE_ORDNER, [ordner, 'allgemein', '1048276'], {
        stdio: 'ignore',
        env: { ...process.env, TZ: 'Europe/Berlin', LOCALCHATA_JETZT: JETZT },
      });
    },
  });
  const beta = await beitreten(instanz.adresse, 'beta');
  const status = [];
  for (let nummer = 0; nummer < 4; nummer += 1) {
    status.push((await senden(instanz.adresse, beta, 'allgemein', TEXT)).status);
  }
  expect(status).toEqual([201, 201, 201, 507]);
  return beta;
}

test('US-4: an der Quote steht der Grund an der Nachricht, „Erneut senden“ bleibt', async ({ browser }) => {
  const beta = await anDerQuote();
  const vorher = await verlaufLesen(instanz.adresse, beta, 'allgemein');
  const a = await kontextOeffnen(browser, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [507] } });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');

  await eingabe(a.page).fill('Hallo zusammen');
  await eingabe(a.page).press('Enter');

  const fehlgeschlagen = vorlaeufige(a.page);
  await expect(fehlgeschlagen).toHaveCount(1);
  await expect(fehlgeschlagen.locator('.lc-body')).toHaveText('Hallo zusammen');
  await expect(fehlgeschlagen.locator('.lc-send')).toHaveText(`⚠ ${TEXT_507}`);
  await expect(fehlgeschlagen.getByRole('button', { name: 'Erneut senden' })).toBeVisible();
  await expect(a.page.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)')
    .filter({ hasText: 'Hallo zusammen' })).toHaveCount(0);
  expect(await verlaufLesen(instanz.adresse, beta, 'allgemein')).toEqual(vorher);
  expect(a.fehler).toEqual([]);
});
