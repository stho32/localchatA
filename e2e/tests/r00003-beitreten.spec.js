'use strict';
// R00003 F0010 Beitreten-Bildschirm — Playwright mit Chromium gegen das echte Programm (US-15 bis US-21).
const { test, expect } = require('./rahmen');
const { instanzStarten, freierPort, fehlerSammeln } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function beitretenPerApi(request, name) {
  return request.post(instanz.adresse + 'api/beitreten', { data: { name } });
}

test('Bildschirm: Namensfeld hat nach dem Öffnen den Fokus, Schalter Beitreten', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);

  await page.goto(instanz.adresse);

  await expect(page.getByLabel('Name')).toBeFocused();
  await expect(page.getByRole('button', { name: 'Beitreten' })).toBeVisible();
  expect(fehler).toEqual([]);
});

async function kopfzeile(page) {
  return page.getByRole('banner').locator('.lc-me');
}

test('US-15: erster Beitritt — Fokus im Feld, shoff und Enter, Kopfzeile shoff@127.0.0.1, Sitzung gemerkt', async ({ page, request }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.goto(instanz.adresse);
  await expect(page.getByLabel('Name')).toBeFocused();

  await page.keyboard.type('shoff');
  await page.keyboard.press('Enter');

  const kennung = await kopfzeile(page);
  await expect(kennung).toHaveText('shoff@127.0.0.1');
  await expect(kennung.locator('i')).toHaveText('@127.0.0.1');
  await expect(page.getByLabel('Name')).toHaveCount(0);
  const gemerkt = await page.evaluate(() => ({
    sitzung: localStorage.getItem('localchatA.sitzung'), name: localStorage.getItem('localchatA.name'),
  }));
  expect(gemerkt.name).toBe('shoff');
  const ich = await request.get(instanz.adresse + 'api/ich', { headers: { 'X-LocalchatA-Sitzung': gemerkt.sitzung } });
  expect(ich.status()).toBe(200);
  expect((await ich.json()).art).toBe('mensch');
  expect(fehler).toEqual([]);
});

// Entscheidung Director (UX-Review): nach dem 422 steht der Vorschlag im Feld wie bei „vergeben“.
test('US-16: Jörg zeigt vor dem Senden den Hinweis mit joerg, Enter tritt nicht bei und schlägt joerg vor', async ({ page, request }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.goto(instanz.adresse);
  const feld = page.getByLabel('Name');

  await feld.pressSequentially('Jörg');
  const hinweis = page.locator('#lc-name-hinweis');
  await expect(hinweis).toContainText('a–z, 0–9 und -');
  await expect(hinweis).toContainText('joerg');
  const antwort = page.waitForResponse('**/api/beitreten');
  await feld.press('Enter');
  expect((await antwort).status()).toBe(422);

  await expect(page.locator('#lc-meldung')).toContainText('joerg');
  await expect(feld).toHaveValue('joerg');
  await expect(feld).toBeFocused();
  expect((await beitretenPerApi(request, 'Jörg')).status()).toBe(422);
  expect((await beitretenPerApi(request, 'joerg')).status()).toBe(201);
  expect(fehler).toEqual([]);
});

test('US-17: Name vergeben nennt Träger und Uhrzeit HH:MM und füllt den Vorschlag vor', async ({ page, request }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  expect((await beitretenPerApi(request, 'shoff')).status()).toBe(201);
  await page.goto(instanz.adresse);

  await page.getByLabel('Name').pressSequentially('shoff');
  const antwort = page.waitForResponse('**/api/beitreten');
  await page.getByLabel('Name').press('Enter');
  const vergeben = await (await antwort).json();

  const uhrzeit = new Intl.DateTimeFormat('de-DE', { hour: '2-digit', minute: '2-digit', timeZone: 'Europe/Berlin' })
    .format(new Date(vergeben.seit));
  const meldung = page.locator('#lc-meldung');
  await expect(meldung).toContainText('shoff@127.0.0.1');
  await expect(meldung).toContainText(uhrzeit);
  await expect(page.getByLabel('Name')).toHaveValue('shoff-2');
  await expect(page.getByLabel('Name')).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(await kopfzeile(page)).toHaveText('shoff-2@127.0.0.1');
  expect(fehler).toEqual([]);
});

test('US-20: Server weg zeigt ⚠ Server nicht erreichbar mit Erneut versuchen', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page, { serverWegErwartet: true });
  await page.goto(instanz.adresse);
  await expect(page.getByLabel('Name')).toBeFocused();
  await instanz.beenden();

  await page.getByLabel('Name').pressSequentially('shoff');
  await page.getByLabel('Name').press('Enter');

  const meldung = page.locator('#lc-meldung');
  await expect(meldung).toContainText('⚠');
  await expect(meldung).toContainText('Server nicht erreichbar');
  await expect(page.getByRole('button', { name: 'Erneut versuchen' })).toBeVisible();
  await expect(page.getByLabel('Name')).toHaveValue('shoff');
  expect(fehler).toEqual([]);
});

async function beitretenImBrowser(page, name) {
  await page.goto(instanz.adresse);
  await expect(page.getByLabel('Name')).toBeFocused();
  await page.keyboard.type(name);
  await page.keyboard.press('Enter');
  await expect(await kopfzeile(page)).toHaveText(name + '@127.0.0.1');
}

function beitrittsaufrufeZaehlen(page) {
  const aufrufe = [];
  page.on('request', (anfrage) => {
    if (anfrage.url().endsWith('/api/beitreten')) {
      aufrufe.push(anfrage.url());
    }
  });
  return aufrufe;
}

test('US-18: Neuladen, neuer Tab, Neustart auf demselben Port — derselbe Teilnehmer ohne Tippen', async ({ context, page, request }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  const fehler = fehlerSammeln(page);
  await beitretenImBrowser(page, 'shoff');
  const erstesToken = await page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  const aufrufe = beitrittsaufrufeZaehlen(page);

  await page.reload();
  await expect(await kopfzeile(page)).toHaveText('shoff@127.0.0.1');
  await expect(page.getByLabel('Name')).toHaveCount(0);
  expect(aufrufe).toEqual([]);

  const tab = await context.newPage();
  const fehlerImTab = fehlerSammeln(tab);
  await tab.goto(instanz.adresse);
  await expect(await kopfzeile(tab)).toHaveText('shoff@127.0.0.1');
  expect((await beitretenPerApi(request, 'shoff')).status()).toBe(409);

  await instanz.beenden();
  await instanz.starten();
  await page.reload();
  await expect(await kopfzeile(page)).toHaveText('shoff@127.0.0.1');
  await expect(page.getByLabel('Name')).toHaveCount(0);
  const neuesToken = await page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  expect(neuesToken).not.toBe(erstesToken);
  expect(aufrufe.length).toBe(1);
  expect((await beitretenPerApi(request, 'shoff')).status()).toBe(409);
  expect(fehler).toEqual([]);
  expect(fehlerImTab).toEqual([]);
});

// Seit R00004 hält ein offener Tab die Sitzung über seinen Ereignisstrom am Leben (Halter, ADR-0012) — untätig
// über die Gnadenfrist heißt deshalb: keine Seite der Instanz offen, kein Strom, keine Anfrage. Die Frist
// läuft erst, wenn der Strom am Server geschlossen ist (der SharedWorker endet nach dem letzten Tab, unter
// Last spürbar später) — darum zuerst auf „halter: 0“ warten, dann die Dauer.
async function untaetigOhneOffeneSeite(page, dauerMs) {
  await page.goto('about:blank');
  await expect.poll(async () => {
    const antwort = await fetch(instanz.adresse + 'api/gesundheit');
    return (await antwort.json()).halter;
  }, { timeout: 10000 }).toBe(0);
  await page.waitForTimeout(dauerMs);
}

test('US-19: untätig über die Gnadenfrist — Neuladen tritt still wieder bei', async ({ page }) => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_GNADENFRIST_MS: '2000' } });
  const fehler = fehlerSammeln(page);
  await beitretenImBrowser(page, 'shoff');

  await untaetigOhneOffeneSeite(page, 3000);
  const aufrufe = beitrittsaufrufeZaehlen(page);
  await page.goto(instanz.adresse);

  await expect(await kopfzeile(page)).toHaveText('shoff@127.0.0.1');
  await expect(page.getByLabel('Name')).toHaveCount(0);
  expect(aufrufe.length).toBe(1);
  expect(fehler).toEqual([]);
});

test('US-19: hat in der Gnadenfrist ein anderer den Namen genommen, zeigt Neuladen Name vergeben mit shoff-2', async ({ page, request }) => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_GNADENFRIST_MS: '2000' } });
  const fehler = fehlerSammeln(page);
  await beitretenImBrowser(page, 'shoff');

  await untaetigOhneOffeneSeite(page, 3000);
  expect((await beitretenPerApi(request, 'shoff')).status()).toBe(201);
  await page.goto(instanz.adresse);

  await expect(page.locator('#lc-meldung')).toContainText('vergeben');
  await expect(page.getByLabel('Name')).toHaveValue('shoff-2');
  await expect(page.getByLabel('Name')).toBeFocused();
  expect(fehler).toEqual([]);
});

test('US-20: nach dem Neustart auf demselben Port führt Erneut versuchen per Tab und Enter zur Kopfzeile', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  const fehler = fehlerSammeln(page, { serverWegErwartet: true });
  await page.goto(instanz.adresse);
  await expect(page.getByLabel('Name')).toBeFocused();
  await instanz.beenden();
  await page.keyboard.type('shoff');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: 'Erneut versuchen' })).toBeVisible();

  await instanz.starten();
  await page.keyboard.press('Tab');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('button', { name: 'Erneut versuchen' })).toBeFocused();
  await page.keyboard.press('Enter');

  await expect(await kopfzeile(page)).toHaveText('shoff@127.0.0.1');
  expect(fehler).toEqual([]);
});

test('US-21: ohne Maus erreichbar, jeder Fokus sichtbar umrandet', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  const fehler = fehlerSammeln(page, { serverWegErwartet: true });
  const umrandung = () => page.evaluate(() => {
    const stil = getComputedStyle(document.activeElement);
    return `${stil.outlineStyle} ${stil.outlineWidth}`;
  });
  await page.goto(instanz.adresse);
  await instanz.beenden();
  await page.keyboard.type('shoff');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: 'Erneut versuchen' })).toBeVisible();

  const reihe = [];
  for (const schritt of ['Feld', 'Beitreten', 'Erneut versuchen']) {
    reihe.push([schritt, await page.evaluate(() => document.activeElement.textContent || document.activeElement.id), await umrandung()]);
    await page.keyboard.press('Tab');
  }

  expect(reihe).toEqual([
    ['Feld', 'lc-name', 'solid 2px'],
    ['Beitreten', 'Beitreten', 'solid 2px'],
    ['Erneut versuchen', 'Erneut versuchen', 'solid 2px'],
  ]);
  expect(fehler).toEqual([]);
});

test('verzögerter Server: während des Beitritts ist der Schalter gesperrt, zweimal Enter tritt einmal bei', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.route('**/api/beitreten', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 800));
    await route.continue();
  });
  const aufrufe = beitrittsaufrufeZaehlen(page);
  await page.goto(instanz.adresse);
  await page.keyboard.type('shoff');

  await page.keyboard.press('Enter');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: 'Beitreten' })).toBeDisabled();

  await expect(await kopfzeile(page)).toHaveText('shoff@127.0.0.1');
  expect(aufrufe.length).toBe(1);
  expect(fehler).toEqual([]);
});

test('verzögerter Server: Erneut versuchen zweimal gedrückt tritt einmal bei', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  const fehler = fehlerSammeln(page, { serverWegErwartet: true });
  await page.goto(instanz.adresse);
  await instanz.beenden();
  await page.keyboard.type('shoff');
  await page.keyboard.press('Enter');
  const erneut = page.getByRole('button', { name: 'Erneut versuchen' });
  await expect(erneut).toBeVisible();
  await instanz.starten();
  await page.route('**/api/beitreten', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 800));
    await route.continue();
  });
  const aufrufe = beitrittsaufrufeZaehlen(page);

  await erneut.focus();
  await page.keyboard.press('Enter');
  await page.keyboard.press('Enter');

  await expect(await kopfzeile(page)).toHaveText('shoff@127.0.0.1');
  expect(aufrufe.length).toBe(1);
  expect(fehler).toEqual([]);
});

test('dunkles Farbschema des Systems: Seite nimmt die dunklen Tokens', async ({ page }) => {
  instanz = await instanzStarten();
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto(instanz.adresse);
  await expect(page.getByLabel('Name')).toBeFocused();

  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  const grund = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(grund).toBe('rgb(20, 24, 29)');
});

test('der Fehlersammler erkennt Konsolenfehler und CSP-Verstöße', async ({ page }) => {
  instanz = await instanzStarten();
  const fehler = fehlerSammeln(page);
  await page.goto(instanz.adresse);
  await expect(page.getByLabel('Name')).toBeFocused();

  await page.evaluate(() => {
    console.error('Probe');
    const verboten = document.createElement('div');
    verboten.innerHTML = '<span style="color: red">x</span>';
    document.body.append(verboten);
  });

  await expect.poll(() => fehler.length).toBeGreaterThanOrEqual(2);
  expect(fehler.some((eintrag) => eintrag.includes('Probe'))).toBe(true);
  expect(fehler.some((eintrag) => eintrag.includes('Content Security Policy'))).toBe(true);
});
