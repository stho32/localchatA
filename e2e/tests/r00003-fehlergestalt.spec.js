'use strict';
// R00003 F0010, UX-Review (Sollte): Fehlergestalt nach Designsystem — kd-danger, ⚠ und Wort, aria-invalid;
// Vorschlag nach 422 im Feld (Entscheidung Director), leeres Feld ohne POST.
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const feld = (page) => page.getByLabel('Name');
const meldung = (page) => page.locator('#lc-meldung');

async function senden(page, name) {
  await feld(page).fill(name);
  await feld(page).press('Enter');
}

async function gefahrenfarbe(page) {
  return page.evaluate(() => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--kd-danger)';
    document.body.append(probe);
    const farbe = getComputedStyle(probe).color;
    probe.remove();
    return farbe;
  });
}

async function meldungsfarbe(page) {
  return meldung(page).locator('span').first().evaluate((element) => getComputedStyle(element).color);
}

for (const thema of ['light', 'dark']) {
  test(`Fehlergestalt ${thema}: ungültig — ⚠ in kd-danger, aria-invalid solange ungültig, Vorschlag nach 422 im Feld`, async ({ page }) => {
    instanz = await instanzStarten();
    const fehler = fehlerSammeln(page);
    await page.emulateMedia({ colorScheme: thema });
    await page.goto(instanz.adresse);

    await feld(page).pressSequentially('shoff');
    await expect(feld(page)).not.toHaveAttribute('aria-invalid');
    await feld(page).fill('');
    await feld(page).pressSequentially('Jörg');
    await expect(feld(page)).toHaveAttribute('aria-invalid', 'true');
    const antwort = page.waitForResponse('**/api/beitreten');
    await feld(page).press('Enter');
    expect((await antwort).status()).toBe(422);

    await expect(meldung(page)).toHaveText(/^⚠ Der Name Jörg ist nicht erlaubt — .*a–z, 0–9 und -.*\. Wie wäre joerg\?$/);
    expect(await meldungsfarbe(page)).toBe(await gefahrenfarbe(page));
    await expect(feld(page)).toHaveValue('joerg');
    await expect(feld(page)).not.toHaveAttribute('aria-invalid');
    await expect(feld(page)).toBeFocused();
    await expect(page.locator('#lc-name-hinweis')).toBeHidden();
    await feld(page).press('Enter');
    await expect(page.getByRole('banner').locator('.lc-me')).toHaveText('joerg@127.0.0.1');
    expect(fehler).toEqual([]);
  });

  test(`Fehlergestalt ${thema}: Server weg — ⚠ und Wort in kd-danger`, async ({ page }) => {
    instanz = await instanzStarten();
    await page.emulateMedia({ colorScheme: thema });
    await page.goto(instanz.adresse);
    await instanz.beenden();

    await senden(page, 'shoff');

    await expect(meldung(page)).toContainText('⚠ Server nicht erreichbar.');
    expect(await meldungsfarbe(page)).toBe(await gefahrenfarbe(page));
  });
}

// Nacharbeit 2 (UX-Review Sollte): auch „vergeben“ ist ein Fehler — ⚠, kd-danger, ohne Querscrollen.
for (const thema of ['light', 'dark']) {
  for (const breite of [320, 375, 1280]) {
    test(`Fehlergestalt ${thema} ${breite} px: vergeben — beginnt mit ⚠ in kd-danger, kein Querscrollen`, async ({ page, request }) => {
      instanz = await instanzStarten();
      expect((await request.post(instanz.adresse + 'api/beitreten', { data: { name: 'shoff' } })).status()).toBe(201);
      await page.setViewportSize({ width: breite, height: 640 });
      await page.emulateMedia({ colorScheme: thema });
      await page.goto(instanz.adresse);

      await senden(page, 'shoff');

      await expect(meldung(page)).toContainText('vergeben');
      expect(await meldung(page).textContent()).toMatch(/^⚠ /);
      expect(await meldungsfarbe(page)).toBe(await gefahrenfarbe(page));
      const { inhalt, fenster } = await page.evaluate(() => ({
        inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth,
      }));
      expect(inhalt).toBeLessThanOrEqual(fenster);
    });
  }
}

// Nacharbeit 2 (UX-Review Sollte, Rückschritt aus B0066): die Leer-Meldung geht mit dem ersten Zeichen.
for (const thema of ['light', 'dark']) {
  test(`Fehlergestalt ${thema}: Leer-Meldung verschwindet beim Tippen, bei „Jörg“ bleibt nur der Regelhinweis`, async ({ page }) => {
    instanz = await instanzStarten();
    const fehler = fehlerSammeln(page);
    await page.emulateMedia({ colorScheme: thema });
    await page.goto(instanz.adresse);
    await feld(page).press('Enter');
    await expect(meldung(page)).toHaveText('⚠ Bitte einen Namen eingeben.');

    await feld(page).pressSequentially('a');
    await expect(meldung(page)).toBeHidden();
    await feld(page).pressSequentially('nna');
    await expect(meldung(page)).toBeHidden();
    await feld(page).fill('');
    await feld(page).pressSequentially('Jörg');

    await expect(meldung(page)).toBeHidden();
    await expect(page.locator('#lc-name-hinweis')).toBeVisible();
    await expect(page.locator('#lc-name-hinweis')).toContainText('a–z, 0–9 und -');
    expect(fehler).toEqual([]);
  });
}

test('ohne Vorschlag des Servers bleibt der ungültige Name im Feld und aria-invalid', async ({ page }) => {
  instanz = await instanzStarten();
  await page.goto(instanz.adresse);

  await senden(page, '___');

  await expect(meldung(page)).toHaveText(/^⚠ Der Name ___ ist nicht erlaubt — .*\.$/);
  await expect(feld(page)).toHaveValue('___');
  await expect(feld(page)).toHaveAttribute('aria-invalid', 'true');
});

for (const eingabe of ['', '   ']) {
  test(`leeres Feld ${JSON.stringify(eingabe)} und Enter: kein POST, „Bitte einen Namen eingeben.“`, async ({ page }) => {
    instanz = await instanzStarten();
    const fehler = fehlerSammeln(page);
    const beitritte = [];
    page.on('request', (anfrage) => {
      if (anfrage.url().endsWith('/api/beitreten')) {
        beitritte.push(anfrage.url());
      }
    });
    await page.goto(instanz.adresse);

    await senden(page, eingabe);

    await expect(meldung(page)).toHaveText('⚠ Bitte einen Namen eingeben.');
    expect(await meldungsfarbe(page)).toBe(await gefahrenfarbe(page));
    await expect(feld(page)).toHaveAttribute('aria-invalid', 'true');
    await expect(feld(page)).toBeFocused();
    await expect(page.getByRole('button', { name: 'Beitreten' })).toBeEnabled();
    expect(beitritte).toEqual([]);
    await senden(page, 'shoff');
    await expect(page.getByRole('banner').locator('.lc-me')).toHaveText('shoff@127.0.0.1');
    expect(fehler).toEqual([]);
  });
}
