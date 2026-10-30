'use strict';
// R00050 F0180 B0791 (Karte #13, V12, US-27): Alt ↓/↑ geht durch die Channels und danach durch die Direktnachrichten —
// allgemein → technik → ben@127.0.0.3 → cora@127.0.0.9, am Ende nicht weiter; Alt ↑ denselben Weg zurück (Rand wie
// F0032). Mutanten, die rot werden müssen: die Direktnachrichten werden übersprungen; Reihenfolge nach Aktivität.
// Gegen das echte Programm.
const { test, expect } = require('./rahmen');
const { ANNA, BEN, CORA, direktStarten, direktSenden, gespraechOffen } = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const adresseVon = (schluessel) => {
  if (schluessel.includes('@')) {
    return `#/d/${encodeURIComponent(schluessel)}`;
  }
  return `#/c/${schluessel}`;
};

async function istGewaehlt(page, schluessel) {
  await expect(page).toHaveURL(new RegExp(`${adresseVon(schluessel).replace(/[.]/g, '\\.')}$`));
  if (schluessel.includes('@')) {
    await gespraechOffen(page, schluessel);
    return;
  }
  await expect(page.getByRole('log', { name: `Verlauf # ${schluessel}` })).toBeVisible();
}

test('Alt ↓/↑ (B0791, V12): allgemein → technik → ben → cora, das vierte Mal bleibt; Alt ↑ zurück bis allgemein', async ({ browser }) => {
  const { instanz: gestartet, anna, token } = await direktStarten(browser);
  instanz = gestartet;
  expect((await direktSenden(instanz.adresse, token.cora, ANNA, 'zuerst cora', '127.0.0.9')).status).toBe(201);
  expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'dann ben', '127.0.0.3')).status).toBe(201);
  await expect(anna.page.getByRole('navigation', { name: 'Direktnachrichten' }).getByRole('link')).toHaveCount(2);
  await istGewaehlt(anna.page, 'allgemein');
  await anna.page.getByRole('log').focus();

  for (const ziel of ['technik', BEN, CORA]) {
    await anna.page.keyboard.press('Alt+ArrowDown');
    await istGewaehlt(anna.page, ziel);
  }
  await anna.page.keyboard.press('Alt+ArrowDown');
  await anna.page.waitForTimeout(200);
  await istGewaehlt(anna.page, CORA);

  for (const ziel of [BEN, 'technik', 'allgemein']) {
    await anna.page.keyboard.press('Alt+ArrowUp');
    await istGewaehlt(anna.page, ziel);
  }
  expect(anna.fehler).toEqual([]);
});
