'use strict';
// R00004 F0013, B0090: die Teilnehmerspalte zeichnet geordnete Gruppen nach dem Designsystem — im echten
// Browser, das Modul aus dem Programm geladen (CSP script-src 'self').
const { test, expect } = require('@playwright/test');
const { instanzStarten } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

function zeile(name, status, art, herkunft, eigene = false) {
  return { kennung: `${name}@${herkunft}`, name, herkunft, art, status, seit: '', stand: 1, eigene };
}

const GRUPPEN = {
  menschen: [zeile('mia', 'offline', 'mensch', '127.0.0.1'), zeile('shoff', 'online', 'mensch', '127.0.0.1', true)],
  agenten: [
    { herkunft: '127.0.0.1', zeilen: [zeile('a1', 'online', 'agent', '127.0.0.1')] },
    { herkunft: '127.0.0.2', zeilen: [zeile('a8', 'offline', 'agent', '127.0.0.2')] },
  ],
};

async function zeichnen(page, gruppen) {
  await page.goto(instanz.adresse + 'web/lc-tokens.css');
  await page.evaluate(async (gruppenImBrowser) => {
    const { zeichnen: spalteZeichnen } = await import('/web/teilnehmerliste.js');
    const region = document.createElement('aside');
    document.body.replaceChildren(region);
    spalteZeichnen(region, gruppenImBrowser);
  }, gruppen);
}

test('B0090: Menschen und Agenten mit Unterkopf je Herkunft, Status als Form und Wort, Agent, (du)', async ({ page }) => {
  instanz = await instanzStarten();
  await zeichnen(page, GRUPPEN);

  const region = page.locator('aside');
  await expect(region.locator('.lc-sect')).toHaveText(['Menschen · 2', 'Agenten · 2', '127.0.0.1', '127.0.0.2']);
  const zeilen = region.locator('.lc-person');
  await expect(zeilen).toHaveCount(4);
  await expect(zeilen.locator('.lc-id')).toHaveText(['mia@127.0.0.1', 'shoff@127.0.0.1', 'a1@127.0.0.1', 'a8@127.0.0.2']);

  const mia = zeilen.nth(0);
  await expect(mia).toHaveClass(/\boff\b/);
  await expect(mia.locator('.lc-dot')).toHaveClass(/\boff\b/);
  await expect(mia.locator('.lc-dot')).toHaveAttribute('aria-label', 'offline');
  await expect(mia.locator('.lc-status')).toHaveText('offline');
  await expect(mia).not.toContainText('Agent');

  const shoff = zeilen.nth(1);
  await expect(shoff.locator('.lc-dot')).toHaveClass(/\bon\b/);
  await expect(shoff.locator('.lc-dot')).toHaveAttribute('aria-label', 'online');
  await expect(shoff.locator('.lc-status')).toHaveCount(0);
  await expect(shoff.locator('.lc-du')).toHaveText('(du)');

  await expect(zeilen.nth(2).locator('.lc-tag')).toHaveText('Agent');
  await expect(zeilen.nth(3).locator('.lc-status')).toHaveText('offline');
  await expect(region).not.toHaveAttribute('aria-live', /.*/);
});

test('B0090: ohne Herkunfts-Gruppe kein Unterkopf, ohne Agenten keine Agenten-Überschrift', async ({ page }) => {
  instanz = await instanzStarten();
  await zeichnen(page, { menschen: [zeile('shoff', 'online', 'mensch', '127.0.0.1', true)], agenten: [] });
  await expect(page.locator('aside .lc-sect')).toHaveText(['Menschen · 1']);

  await zeichnen(page, { menschen: [], agenten: [{ herkunft: null, zeilen: [zeile('a1', 'online', 'agent', '127.0.0.1')] }] });
  await expect(page.locator('aside .lc-sect')).toHaveText(['Agenten · 1']);
});

test('B0090: Text vom Server nur als Text — ein Name mit Markup wird nicht ausgeführt', async ({ page }) => {
  instanz = await instanzStarten();
  const boese = zeile('<img src=x onerror=window.boese=1>', 'online', 'agent', '127.0.0.1');
  await zeichnen(page, { menschen: [], agenten: [{ herkunft: null, zeilen: [boese] }] });

  await expect(page.locator('aside .lc-id')).toHaveText(`${boese.name}@127.0.0.1`);
  await expect(page.locator('aside img')).toHaveCount(0);
});
