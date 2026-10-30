'use strict';
// R00046 I0042 Nachricht kopieren im Browser (ADR-0052): die Aktion „Kopieren“ in Leiste, Menü „⋯“ und Tab-Weg (B0701), die
// Taste C (B0702), die Rückmeldung (B0703). Die Zwischenablage gehört dem Browser-Prozess, nicht dem Kontext — die Tests
// dieser Datei laufen seriell und belegen sie vor jeder Prüfung mit einem Wächtertext, damit ein Rest aus einem früheren
// Test nicht als Erfolg zählt.
const { test, expect } = require('./rahmen');
const { instanzStarten, channelAnlegen } = require('./hilfen');
const {
  JETZT, sendenStarten, beitreten, senden, anfragen, imBrowserBeitreten, nachrichten,
} = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const {
  browserOeffnen, imBrowserSenden, nachrichtMitZeit, loeschenPerApi, mitAnhaengenSenden, PNG, schriftKontrast,
} = require('./r00041-hilfen');
const { WAECHTER, QUELLTEXT, lanStarten, pruefFeldAnlegen, ablageBelegen, ablageLesen, kanalPruefsumme } = require('./r00046-hilfen');

test.describe.configure({ mode: 'serial' });

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const kopierKnopf = (page, zeit) => nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Kopieren', exact: true });

/// Die Leiste steht erst bei Hover: Maus auf die Nachricht, dann „Kopieren“ klicken.
async function ueberHoverKopieren(page, zeit) {
  await nachrichtMitZeit(page, zeit).hover();
  await kopierKnopf(page, zeit).click();
}

/// `claude` (Agent) tritt per API bei und sendet `texte` nach `allgemein`; → { token, zeiten }.
async function claudeSendet(adresse, texte) {
  const token = await beitreten(adresse, 'claude');
  const zeiten = [];
  for (const text of texte) {
    const antwort = await senden(adresse, token, 'allgemein', text);
    expect(antwort.status, JSON.stringify(antwort.daten)).toBe(201);
    zeiten.push(antwort.daten.nachricht.zeit);
  }
  return { token, zeiten };
}

async function seiteOeffnen(browser, adresse, name = 'shoff', optionen = { streng: true }) {
  const a = await browserOeffnen(browser, optionen);
  await imBrowserBeitreten(a.page, adresse, name);
  await pruefFeldAnlegen(a.page);
  return a;
}

test('Aktion an eigener und fremder Nachricht: Kopieren in der Leiste bei Hover, zuerst; Bearbeiten und Löschen nur an der eigenen', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await seiteOeffnen(browser, instanz.adresse);
  const eigene = await imBrowserSenden(a.page, 'Hallo');
  const fremde = (await claudeSendet(instanz.adresse, ['Ergebnis'])).zeiten[0];
  await expect(nachrichtMitZeit(a.page, fremde)).toHaveCount(1);
  await a.page.mouse.move(0, 0);
  await expect(kopierKnopf(a.page, eigene)).toBeHidden();

  await nachrichtMitZeit(a.page, eigene).hover();
  await expect(nachrichtMitZeit(a.page, eigene).locator('.lc-aktion')).toHaveText(['Kopieren', 'Bearbeiten', 'Löschen']);
  await expect(kopierKnopf(a.page, eigene)).toBeVisible();

  await nachrichtMitZeit(a.page, fremde).hover();
  await expect(kopierKnopf(a.page, fremde)).toBeVisible();
  await expect(nachrichtMitZeit(a.page, fremde).locator('.lc-aktion')).toHaveText(['Kopieren']);
  await expect(nachrichtMitZeit(a.page, fremde).getByRole('button', { name: /Bearbeiten|lösch/i })).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

test('Quelltext genau über http im LAN (isSecureContext falsch): die Zwischenablage ist zeichengleich dem Gesendeten (P3, QK1)', async ({ browser }) => {
  const lan = await lanStarten();
  instanz = lan.instanz;
  const fremde = (await claudeSendet(lan.api, [QUELLTEXT])).zeiten[0];
  const a = await seiteOeffnen(browser, lan.lan);
  expect(await a.page.evaluate(() => window.isSecureContext)).toBe(false);
  expect(await a.page.evaluate(() => typeof navigator.clipboard)).toBe('undefined');
  await expect(nachrichtMitZeit(a.page, fremde)).toHaveCount(1);
  await ablageBelegen(a.page);

  await ueberHoverKopieren(a.page, fremde);

  const abgelegt = await ablageLesen(a.page);
  expect(abgelegt).toBe(QUELLTEXT);
  expect(abgelegt.length).toBe(QUELLTEXT.length);
  expect(a.fehler).toEqual([]);
});

test('Quelltext genau unter http://127.0.0.1 ohne Berechtigung: derselbe Weg liefert dieselbe Zeichen (P3)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremde = (await claudeSendet(instanz.adresse, [QUELLTEXT])).zeiten[0];
  const a = await seiteOeffnen(browser, instanz.adresse);
  expect(await a.page.evaluate(() => window.isSecureContext)).toBe(true);
  await expect(nachrichtMitZeit(a.page, fremde)).toHaveCount(1);
  await ablageBelegen(a.page);

  await ueberHoverKopieren(a.page, fremde);

  expect(await ablageLesen(a.page)).toBe(QUELLTEXT);
  expect(a.fehler).toEqual([]);
});

test('eine bearbeitete Nachricht kopiert die neue Fassung, nicht die gezeichnete Sicht (Regel 1, Mutant K2)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Alt **fett**']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);
  const bearbeitet = await anfragen(instanz.adresse, {
    methode: 'POST', pfad: 'api/channels/allgemein/nachrichten/bearbeiten', koerper: { zeit: fremd.zeiten[0], text: 'Neu **fett**\nzweite Zeile' }, token: fremd.token,
  });
  expect(bearbeitet.status, JSON.stringify(bearbeitet.daten)).toBe(200);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0]).locator('.lc-body')).toHaveText('Neu **fett**\nzweite Zeile');
  await ablageBelegen(a.page);

  await ueberHoverKopieren(a.page, fremd.zeiten[0]);

  expect(await ablageLesen(a.page)).toBe('Neu **fett**\nzweite Zeile');
});

test('Ruhig (P6): kein Netzaufruf, Fokus auf dem Knopf, Lage unverändert, die Dateien des Channels unverändert', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Eins', 'Zwei', 'Drei']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[2])).toHaveCount(1);
  await ablageBelegen(a.page);
  const anfragenNachher = [];
  a.page.on('request', (anfrage) => anfragenNachher.push(`${anfrage.method()} ${anfrage.url()}`));
  const summeVorher = kanalPruefsumme(instanz.ordner);
  const lage = () => nachrichtMitZeit(a.page, fremd.zeiten[1]).evaluate((knoten) => ({
    oben: knoten.getBoundingClientRect().top, scrollTop: knoten.closest('[role="log"]').scrollTop,
  }));
  const lageVorher = await lage();

  await ueberHoverKopieren(a.page, fremd.zeiten[1]);

  await expect(kopierKnopf(a.page, fremd.zeiten[1])).toBeFocused();
  expect(await lage()).toEqual(lageVorher);
  expect(anfragenNachher).toEqual([]);
  expect(kanalPruefsumme(instanz.ordner)).toBe(summeVorher);
  expect(await ablageLesen(a.page)).toBe('Zwei');
});

test('Tastatur: von der fokussierten Nachricht führt Tab zu „Kopieren“, Enter und Leertaste kopieren', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Erste', 'Zweite']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[1])).toHaveCount(1);

  for (const taste of ['Enter', ' ']) {
    await ablageBelegen(a.page);
    await nachrichtMitZeit(a.page, fremd.zeiten[1]).focus();
    await a.page.keyboard.press('Tab');
    await expect(kopierKnopf(a.page, fremd.zeiten[1])).toBeFocused();
    await a.page.keyboard.press(taste);
    await expect(kopierKnopf(a.page, fremd.zeiten[1])).toBeFocused();
    expect(await ablageLesen(a.page), `Taste „${taste}“`).toBe('Zweite');
  }
  expect(a.fehler).toEqual([]);
});

test('nicht angeboten: gelöschte Nachricht, reine Anhänge, „wird gesendet …“; „Bild“ mit Anhang kopiert „Bild“', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Wird gelöscht']);
  await loeschenPerApi(instanz.adresse, fremd.token, fremd.zeiten[0], '127.0.0.1');
  const nurAnhang = await mitAnhaengenSenden(instanz.adresse, fremd.token, '', [{ name: 'a.png', inhalt: PNG }], '127.0.0.1');
  const mitBild = await mitAnhaengenSenden(instanz.adresse, fremd.token, 'Bild', [{ name: 'b.png', inhalt: PNG }], '127.0.0.1');
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, mitBild)).toHaveCount(1);
  await expect(nachrichtMitZeit(a.page, nurAnhang)).toHaveCount(1);

  for (const zeit of [fremd.zeiten[0], nurAnhang]) {
    await nachrichtMitZeit(a.page, zeit).hover();
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-aktionen')).toHaveCount(0);
    await expect(nachrichtMitZeit(a.page, zeit).getByRole('button', { name: /Kopieren|Aktionen/ })).toHaveCount(0);
  }

  await ablageBelegen(a.page);
  await ueberHoverKopieren(a.page, mitBild);
  expect(await ablageLesen(a.page)).toBe('Bild');

  await a.page.route('**/api/channels/allgemein/nachrichten', async (route) => {
    await new Promise((weiter) => setTimeout(weiter, 1500));
    await route.continue();
  });
  await a.page.locator('.lc-comp').getByRole('textbox').fill('Unterwegs');
  await a.page.locator('.lc-comp').getByRole('button', { name: 'Senden', exact: true }).click();
  const vorlaeufig = a.page.getByRole('log').locator('.lc-vorlaeufig');
  await expect(vorlaeufig).toHaveCount(1);
  await vorlaeufig.hover();
  await expect(vorlaeufig.getByRole('button', { name: /Kopieren|Aktionen/ })).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

test('Bearbeiten und Löschen bleiben an eigenen Nachrichten: an fremden gibt es kein solches Element', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Fremd']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);

  await nachrichtMitZeit(a.page, fremd.zeiten[0]).focus();
  await a.page.keyboard.press('e');
  await a.page.keyboard.press('Delete');

  await expect(a.page.getByRole('textbox', { name: 'Nachricht bearbeiten' })).toHaveCount(0);
  await expect(a.page.getByRole('dialog')).toHaveCount(0);
  await expect(a.page.getByRole('log').getByRole('button', { name: /Bearbeiten|lösch/i })).toHaveCount(0);
});

test('Leiste (Sollte): Knöpfe ≥ 24 × 24 px und innerhalb der Zeile, die Zeile wächst bei Hover und Fokus nicht, Text ≥ 4,5 : 1 auf kd-surface-2', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Eins', 'Zwei', 'Drei']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[2])).toHaveCount(1);
  await a.page.mouse.move(0, 0);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[1])).not.toHaveClass(/\bfirst\b/);
  const hoehe = (zeit) => nachrichtMitZeit(a.page, zeit).evaluate((knoten) => knoten.getBoundingClientRect().height);

  for (const zeit of [fremd.zeiten[0], fremd.zeiten[1]]) {
    const vorher = await hoehe(zeit);
    await nachrichtMitZeit(a.page, zeit).hover();
    await expect(kopierKnopf(a.page, zeit)).toBeVisible();
    expect(await hoehe(zeit), `Hover ${zeit}`).toBe(vorher);
    const lagen = await nachrichtMitZeit(a.page, zeit).evaluate((knoten) => {
      const zeile = knoten.getBoundingClientRect();
      return [...knoten.querySelectorAll('.lc-aktion')].map((knopf) => {
        const rand = knopf.getBoundingClientRect();
        return { breite: rand.width, hoehe: rand.height, innen: rand.top >= zeile.top && rand.bottom <= zeile.bottom };
      });
    });
    for (const lage of lagen) {
      expect(lage.breite).toBeGreaterThanOrEqual(24);
      expect(lage.hoehe).toBeGreaterThanOrEqual(24);
      expect(lage.innen, `innerhalb der Zeile ${zeit}`).toBe(true);
    }
    expect(await schriftKontrast(kopierKnopf(a.page, zeit))).toBeGreaterThanOrEqual(4.5);
  }

  await a.page.mouse.move(0, 0);
  await nachrichtMitZeit(a.page, fremd.zeiten[2]).focus();
  await expect(kopierKnopf(a.page, fremd.zeiten[2])).toBeVisible();
});

const FENSTER_TELEFON = { width: 375, height: 667 };
const FENSTER_MIT_TASTATUR = { width: 375, height: 367 };

/// Alle Nachrichten ganz im sichtbaren Teil des Verlaufs, von oben: [{ zeit, oben, unten }].
function sichtbareNachrichten(page) {
  return page.evaluate(() => {
    const log = document.querySelector('[role="log"]').getBoundingClientRect();
    return [...document.querySelectorAll('[role="log"] .lc-msg[data-zeit]')]
      .map((knoten) => ({ zeit: knoten.dataset.zeit, rand: knoten.getBoundingClientRect() }))
      .filter(({ rand }) => rand.top >= log.top && rand.bottom <= log.bottom)
      .map(({ zeit, rand }) => ({ zeit, oben: rand.top, unten: rand.bottom }));
  });
}

/// Liegt das Element mit Mitte und vier Ecken (1 px nach innen) im visualViewport und wird dort selbst (oder von einem Kind) getroffen?
function treffer(locator) {
  return locator.evaluate((knoten) => {
    const rand = knoten.getBoundingClientRect();
    const sicht = window.visualViewport;
    const punkte = [
      [rand.left + rand.width / 2, rand.top + rand.height / 2], [rand.left + 1, rand.top + 1], [rand.right - 1, rand.top + 1],
      [rand.left + 1, rand.bottom - 1], [rand.right - 1, rand.bottom - 1],
    ];
    return {
      imSichtbaren: rand.top >= sicht.offsetTop && rand.bottom <= sicht.offsetTop + sicht.height
        && rand.left >= sicht.offsetLeft && rand.right <= sicht.offsetLeft + sicht.width,
      getroffen: punkte.map(([x, y]) => {
        const oben = document.elementFromPoint(x, y);
        return oben !== null && knoten.contains(oben);
      }),
      breite: rand.width,
      hoehe: rand.height,
    };
  });
}

const schalter = (page, zeit) => nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Aktionen', exact: true });

test('Touch: „Aktionen“ an der fremden Nachricht öffnet das Menü mit „Kopieren“ (≥ 44 × 44 px), Tippen kopiert; höchstens ein Menü', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Eins', 'Zwei', 'Drei']);
  const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await pruefFeldAnlegen(a.page);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[2])).toHaveCount(1);
  await expect(kopierKnopf(a.page, fremd.zeiten[1])).toBeHidden();

  for (const zeit of fremd.zeiten) {
    const t = await treffer(schalter(a.page, zeit));
    expect(t.getroffen, `Schalter ${zeit}`).toEqual([true, true, true, true, true]);
    expect(t.breite).toBeGreaterThanOrEqual(44);
    expect(t.hoehe).toBeGreaterThanOrEqual(44);
  }

  await schalter(a.page, fremd.zeiten[0]).tap();
  await expect(schalter(a.page, fremd.zeiten[0])).toHaveAttribute('aria-expanded', 'true');
  await schalter(a.page, fremd.zeiten[1]).tap();
  await expect(schalter(a.page, fremd.zeiten[0])).toHaveAttribute('aria-expanded', 'false');
  await expect(a.page.locator('.lc-aktionen-schalter[aria-expanded="true"]')).toHaveCount(1);
  await expect(kopierKnopf(a.page, fremd.zeiten[1])).toBeVisible();
  const t = await treffer(kopierKnopf(a.page, fremd.zeiten[1]));
  expect(t.getroffen).toEqual([true, true, true, true, true]);
  expect(t.breite).toBeGreaterThanOrEqual(44);
  expect(t.hoehe).toBeGreaterThanOrEqual(44);

  await ablageBelegen(a.page);
  await kopierKnopf(a.page, fremd.zeiten[1]).tap();
  expect(await ablageLesen(a.page)).toBe('Zwei');
  expect(a.fehler).toEqual([]);
});

/// Vorbereitung für sendenStarten: `allgemein` mit `anzahl` Nachrichten von `von` (eine je Sekunde ab 14:00); → die Zeiten.
function verlaufVon(von, anzahl) {
  const zeiten = Array.from({ length: anzahl }, (_, nummer) => `2026-10-28T14:${String(Math.floor(nummer / 60)).padStart(2, '0')}:${String(nummer % 60).padStart(2, '0')}.000+01:00`);
  return {
    zeiten,
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein', {
      '2026-10-28': zeiten.map((zeit, nummer) => [zeit, von, `Nachricht ${nummer + 1}`]),
    }),
  };
}

test('Touch: der Schalter „⋯“ an fremden Nachrichten verengt .lc-body nicht anders als an eigenen (höchstens 4 px)', async ({ browser }) => {
  const verlauf = verlaufVon('shoff@127.0.0.1', 1);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const fremd = await claudeSendet(instanz.adresse, ['Nachricht 1']);
  const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);
  const breite = (zeit) => nachrichtMitZeit(a.page, zeit).locator('.lc-body').evaluate((knoten) => knoten.getBoundingClientRect().width);

  const eigen = await breite(verlauf.zeiten[0]);
  const fremde = await breite(fremd.zeiten[0]);

  expect(Math.abs(eigen - fremde)).toBeLessThanOrEqual(4);
});

for (const fenster of [FENSTER_TELEFON, FENSTER_MIT_TASTATUR]) {
  // Eigene Nachrichten tragen drei Einträge im Menü (Kopieren, Bearbeiten, Löschen), fremde einen.
  for (const [wessen, von] of [['eigene', 'shoff@127.0.0.1'], ['fremde', 'beta@127.0.0.2']]) {
    test(`Touch-Menü vollständig sichtbar (${fenster.width} × ${fenster.height}, ${wessen} Nachrichten): bei jeder sichtbaren Nachricht (auch der ersten und der letzten), ohne Scrollen; Esc, Tipp daneben und zweites Tippen schließen`, async ({ browser }) => {
      const verlauf = verlaufVon(von, 30);
      instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
      const a = await geraetOeffnen(browser, 'telefon', { fenster, streng: true });
      await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
      await expect(nachrichtMitZeit(a.page, verlauf.zeiten[29])).toBeVisible();
      const sichtbare = await sichtbareNachrichten(a.page);
      const letzte = sichtbare[sichtbare.length - 1].zeit;
      const erste = sichtbare[0].zeit;
      expect(letzte).toBe(verlauf.zeiten[29]);
      expect(erste).not.toBe(letzte);
      const scrollTop = () => a.page.locator('[role="log"]').evaluate((log) => log.scrollTop);
      const vorher = await scrollTop();

      for (const { zeit } of sichtbare) {
        await schalter(a.page, zeit).tap();
        await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'true');
        const eintraege = nachrichtMitZeit(a.page, zeit).locator('.lc-aktion');
        await expect(eintraege).toHaveCount(wessen === 'eigene' ? 3 : 1);
        for (let stelle = 0; stelle < await eintraege.count(); stelle += 1) {
          const t = await treffer(eintraege.nth(stelle));
          expect(t.imSichtbaren, `Eintrag ${stelle} bei ${zeit}`).toBe(true);
          expect(t.getroffen, `Eintrag ${stelle} bei ${zeit}`).toEqual([true, true, true, true, true]);
        }
        expect(await scrollTop()).toBe(vorher);

        await a.page.keyboard.press('Escape');
        await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'false');
        await expect(schalter(a.page, zeit)).toBeFocused();

        await schalter(a.page, zeit).tap();
        await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'true');
        await schalter(a.page, zeit).tap();
        await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'false');

        await schalter(a.page, zeit).tap();
        await a.page.locator('.lc-comp-feld').tap();
        await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'false');
        expect(await scrollTop()).toBe(vorher);
      }
      expect(a.fehler).toEqual([]);
    });
  }
}
