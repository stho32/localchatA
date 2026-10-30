'use strict';
// R00046 I0042 Nachricht kopieren im Browser (ADR-0052): die Aktion „Kopieren“ in Leiste, Menü „⋯“ und Tab-Weg (B0701), die
// Taste C (B0702), die Rückmeldung (B0703). Die Zwischenablage gehört dem Browser-Prozess, nicht dem Kontext — die Tests
// dieser Datei laufen seriell und belegen sie vor jeder Prüfung mit einem Wächtertext, damit ein Rest aus einem früheren
// Test nicht als Erfolg zählt.
const { test, expect } = require('./rahmen');
const { instanzStarten, channelAnlegen } = require('./hilfen');
const {
  JETZT, sendenStarten, beitreten, senden, anfragen, imBrowserBeitreten, nachrichten, eingabe,
} = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const {
  browserOeffnen, imBrowserSenden, nachrichtMitZeit, loeschenPerApi, mitAnhaengenSenden, PNG, schriftKontrast, ansagenMitschreiben,
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

/// Fokussiert die Nachricht, belegt vorher die Zwischenablage mit dem Wächter (das Prüffeld nimmt dabei den Fokus).
async function nachrichtFokussiertMitWaechter(page, zeit) {
  await ablageBelegen(page);
  await nachrichtMitZeit(page, zeit).focus();
  await expect(nachrichtMitZeit(page, zeit)).toBeFocused();
}

test('Taste C: auf der fokussierten Nachricht kopiert c, auch C bei Feststelltaste; der Fokus bleibt auf der Nachricht (B0702)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Eins', 'Zwei **fett**']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[1])).toHaveCount(1);
  await a.page.evaluate(() => {
    window.tasten = [];
    document.addEventListener('keydown', (ereignis) => window.tasten.push([ereignis.key, ereignis.shiftKey, ereignis.ctrlKey]), true);
  });

  for (const taste of ['c', 'C']) {
    await nachrichtFokussiertMitWaechter(a.page, fremd.zeiten[1]);
    await a.page.keyboard.press(taste);
    await expect(nachrichtMitZeit(a.page, fremd.zeiten[1])).toBeFocused();
    expect(await ablageLesen(a.page), `Taste „${taste}“`).toBe('Zwei **fett**');
  }
  expect(await a.page.evaluate(() => window.tasten.filter(([taste, , strg]) => taste === 'C' && !strg))).toEqual([['C', false, false]]);
  expect(a.fehler).toEqual([]);
});

test('Taste C wertet key, nicht code: key „c“ auf der Taste J kopiert, key „j“ auf der Taste C nicht (B0702)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Dvorak']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  const befehle = await a.page.context().newCDPSession(a.page);
  const druecken = async (key, code, windowsVirtualKeyCode) => {
    await befehle.send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, text: key, windowsVirtualKeyCode });
    await befehle.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode });
  };
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);

  await nachrichtFokussiertMitWaechter(a.page, fremd.zeiten[0]);
  await druecken('j', 'KeyC', 74);
  expect(await ablageLesen(a.page)).toBe(WAECHTER);

  await nachrichtFokussiertMitWaechter(a.page, fremd.zeiten[0]);
  await druecken('c', 'KeyJ', 67);
  expect(await ablageLesen(a.page)).toBe('Dvorak');
});

test('Taste C mit Strg, Alt, Meta oder Umschalt kopiert nichts über die Aktion (B0702)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Nur Abkürzung']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);

  for (const kombination of ['Control+c', 'Alt+c', 'Meta+c', 'Shift+c']) {
    await nachrichtFokussiertMitWaechter(a.page, fremd.zeiten[0]);
    await a.page.keyboard.press(kombination);
    expect(await ablageLesen(a.page), kombination).toBe(WAECHTER);
  }
});

test('Taste C nie im Eingabefeld und nie im Bearbeiten-Feld: sie tippt den Buchstaben, die Zwischenablage bleibt (B0702)', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await seiteOeffnen(browser, instanz.adresse);
  const eigene = await imBrowserSenden(a.page, 'Hallo');

  await ablageBelegen(a.page);
  await eingabe(a.page).focus();
  await a.page.keyboard.press('c');
  await expect(eingabe(a.page)).toHaveValue('c');
  expect(await ablageLesen(a.page)).toBe(WAECHTER);

  await eingabe(a.page).fill('');
  await nachrichtMitZeit(a.page, eigene).focus();
  await a.page.keyboard.press('e');
  const feld = a.page.getByRole('textbox', { name: 'Nachricht bearbeiten' });
  await expect(feld).toBeFocused();
  await ablageBelegen(a.page);
  await feld.focus();
  await a.page.keyboard.press('End');
  await a.page.keyboard.press('c');
  await expect(feld).toHaveValue('Halloc');
  expect(await ablageLesen(a.page)).toBe(WAECHTER);
});

test('Taste C wirkungslos bei offenem Dialog, auf dem Knopf „Kopieren“ und bei markiertem Text; danach kopiert Strg+C genau die Markierung (B0702)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Ein Wort steht hier']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);

  await nachrichtFokussiertMitWaechter(a.page, fremd.zeiten[0]);
  await a.page.evaluate(() => {
    const dialog = document.createElement('dialog');
    dialog.id = 'lc-pruefen-dialog';
    dialog.setAttribute('open', '');
    document.body.append(dialog);
  });
  await nachrichtMitZeit(a.page, fremd.zeiten[0]).focus();
  await a.page.keyboard.press('c');
  expect(await ablageLesen(a.page), 'Dialog offen').toBe(WAECHTER);
  await a.page.evaluate(() => document.getElementById('lc-pruefen-dialog').remove());

  await nachrichtFokussiertMitWaechter(a.page, fremd.zeiten[0]);
  await a.page.keyboard.press('Tab');
  await expect(kopierKnopf(a.page, fremd.zeiten[0])).toBeFocused();
  await a.page.keyboard.press('c');
  expect(await ablageLesen(a.page), 'Fokus auf dem Knopf').toBe(WAECHTER);

  const wortMarkieren = async () => {
    await nachrichtFokussiertMitWaechter(a.page, fremd.zeiten[0]);
    await a.page.evaluate((zeit) => {
      const knoten = document.querySelector(`.lc-msg[data-zeit="${zeit}"] .lc-body`).firstChild;
      const bereich = document.createRange();
      bereich.setStart(knoten, 4);
      bereich.setEnd(knoten, 8);
      getSelection().removeAllRanges();
      getSelection().addRange(bereich);
    }, fremd.zeiten[0]);
    await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toBeFocused();
    expect(await a.page.evaluate(() => getSelection().toString())).toBe('Wort');
  };
  await wortMarkieren();
  await a.page.keyboard.press('c');
  expect(await ablageLesen(a.page), 'markierter Text').toBe(WAECHTER);

  await wortMarkieren();
  await a.page.keyboard.press('Control+c');
  expect(await ablageLesen(a.page), 'Strg+C auf der Markierung').toBe('Wort');
});

test('Taste C bietet nichts an einer gelöschten Nachricht und an einer reinen Anhang-Nachricht (B0702)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Wird gelöscht']);
  await loeschenPerApi(instanz.adresse, fremd.token, fremd.zeiten[0], '127.0.0.1');
  const nurAnhang = await mitAnhaengenSenden(instanz.adresse, fremd.token, '', [{ name: 'a.png', inhalt: PNG }], '127.0.0.1');
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, nurAnhang)).toHaveCount(1);

  for (const zeit of [fremd.zeiten[0], nurAnhang]) {
    await nachrichtFokussiertMitWaechter(a.page, zeit);
    await a.page.keyboard.press('c');
    expect(await ablageLesen(a.page), zeit).toBe(WAECHTER);
  }
});

const FEHLER_ANSAGE = 'Kopieren nicht möglich. Text markieren und Strg+C drücken.';

/// Schreibt auf, wann der sichtbare Text des Knopfes „Kopieren“ wechselt: window.wechsel = [[Text, Millisekunden]].
function anzeigeMitschreiben(page, zeit) {
  return page.evaluate((nachrichtZeit) => {
    const anzeige = document.querySelector(`.lc-msg[data-zeit="${nachrichtZeit}"] .lc-aktion[data-aktion="kopieren"]`);
    window.wechsel = [];
    new MutationObserver(() => window.wechsel.push([anzeige.textContent, performance.now()])).observe(anzeige, { childList: true, characterData: true, subtree: true });
  }, zeit);
}

/// Bis der Text des Knopfes „Kopieren“ wieder „Kopieren“ lautet; → Dauer der Anzeige in Millisekunden (aus dem Browser).
async function anzeigedauer(page) {
  await page.waitForFunction(() => window.wechsel.length >= 2 && window.wechsel[window.wechsel.length - 1][0] === 'Kopieren', null, { polling: 'raf', timeout: 10000 });
  const wechsel = await page.evaluate(() => window.wechsel);
  return wechsel[wechsel.length - 1][1] - wechsel[0][1];
}

test('Rückmeldung (P5): „✓ Kopiert.“ 2000 ms am Knopf, Name bleibt „Kopieren“, Breite und Nachbarn unverändert, die Region sagt genau einmal „Kopiert.“', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await seiteOeffnen(browser, instanz.adresse);
  const eigene = await imBrowserSenden(a.page, 'Hallo');
  const lagen = () => nachrichtMitZeit(a.page, eigene).locator('.lc-aktion').evaluateAll((knoepfe) => knoepfe.map((knopf) => {
    const rand = knopf.getBoundingClientRect();
    return [rand.left, rand.width];
  }));
  const angesagt = await ansagenMitschreiben(a.page);
  await nachrichtMitZeit(a.page, eigene).hover();
  const vorher = await lagen();
  await anzeigeMitschreiben(a.page, eigene);
  await ablageBelegen(a.page);
  await nachrichtMitZeit(a.page, eigene).hover();

  await kopierKnopf(a.page, eigene).click();

  const anzeige = kopierKnopf(a.page, eigene).locator('.lc-aktion-anzeige');
  await expect(anzeige).toHaveText('✓ Kopiert.');
  await expect(anzeige).toHaveAttribute('aria-hidden', 'true');
  await expect(kopierKnopf(a.page, eigene)).toHaveAccessibleName('Kopieren');
  expect(await lagen()).toEqual(vorher);
  await a.page.waitForTimeout(1000);
  await kopierKnopf(a.page, eigene).click();
  const dauer = await anzeigedauer(a.page);
  expect(dauer).toBeGreaterThanOrEqual(2000 - 300);
  expect(dauer).toBeLessThanOrEqual(3000 + 300);
  await expect(anzeige).toHaveText('Kopieren');
  expect(await lagen()).toEqual(vorher);
  expect(await angesagt()).toEqual(['Kopiert.']);
  expect(a.fehler).toEqual([]);
});

test('Rückmeldung: die Anzeige endet 2000 ms nach dem Kopieren (± 300 ms)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Zeit']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);
  await anzeigeMitschreiben(a.page, fremd.zeiten[0]);

  await nachrichtMitZeit(a.page, fremd.zeiten[0]).hover();
  await kopierKnopf(a.page, fremd.zeiten[0]).click();

  const dauer = await anzeigedauer(a.page);
  console.log(`Anzeigedauer Kopiert: ${Math.round(dauer)} ms`);
  expect(dauer).toBeGreaterThanOrEqual(2000 - 300);
  expect(dauer).toBeLessThanOrEqual(2000 + 300);
});

for (const [ort, wie] of [['http://<LAN-IP> (unsicherer Kontext)', 'lan'], ['http://127.0.0.1 (writeText ohne Berechtigung abgelehnt)', 'loopback']]) {
  test(`Rückmeldung bei Fehlschlag, ${ort}: „⚠ Kopieren nicht möglich“ 5000 ms in kd-danger, die Region nennt den Ausweg, die Zwischenablage bleibt`, async ({ browser }) => {
    let adresseApi;
    let adresseSeite;
    if (wie === 'lan') {
      const lan = await lanStarten();
      instanz = lan.instanz;
      adresseApi = lan.api;
      adresseSeite = lan.lan;
    } else {
      instanz = await sendenStarten();
      adresseApi = instanz.adresse;
      adresseSeite = instanz.adresse;
    }
    const fremd = await claudeSendet(adresseApi, ['Geht nicht']);
    const a = await seiteOeffnen(browser, adresseSeite);
    await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);
    await ablageBelegen(a.page);
    await a.page.evaluate(() => { document.execCommand = () => false; });
    const angesagt = await ansagenMitschreiben(a.page);
    await anzeigeMitschreiben(a.page, fremd.zeiten[0]);
    await nachrichtMitZeit(a.page, fremd.zeiten[0]).hover();

    await kopierKnopf(a.page, fremd.zeiten[0]).click();

    const anzeige = kopierKnopf(a.page, fremd.zeiten[0]).locator('.lc-aktion-anzeige');
    await expect(anzeige).toHaveText('⚠ Kopieren nicht möglich');
    await expect(kopierKnopf(a.page, fremd.zeiten[0])).toHaveAccessibleName('Kopieren');
    const farben = await anzeige.evaluate((knoten) => {
      const probe = document.createElement('span');
      probe.style.color = 'var(--kd-danger)';
      document.body.append(probe);
      const gefahr = getComputedStyle(probe).color;
      probe.remove();
      return { ist: getComputedStyle(knoten).color, soll: gefahr };
    });
    expect(farben.ist).toBe(farben.soll);
    const dauer = await anzeigedauer(a.page);
    console.log(`Anzeigedauer Fehler (${wie}): ${Math.round(dauer)} ms`);
    expect(dauer).toBeGreaterThanOrEqual(5000 - 300);
    expect(dauer).toBeLessThanOrEqual(5000 + 300);
    expect(await angesagt()).toEqual([FEHLER_ANSAGE]);
    expect(await ablageLesen(a.page)).toBe(WAECHTER);
  });
}

for (const fehlschlag of [false, true]) {
  test(`Rückmeldung auf Touch (K2, ${fehlschlag ? 'Fehlschlag' : 'Gelingen'}): der Menüeintrag zeigt „${fehlschlag ? '⚠ Kopieren nicht möglich' : '✓ Kopiert.'}“, das Menü ist nach 1000 ms offen und nach 1500 ms zu, der Fokus liegt auf „Aktionen“`, async ({ browser }) => {
    instanz = await sendenStarten();
    const fremd = await claudeSendet(instanz.adresse, ['Eins', 'Zwei']);
    const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await pruefFeldAnlegen(a.page);
    await expect(nachrichtMitZeit(a.page, fremd.zeiten[1])).toHaveCount(1);
    await ablageBelegen(a.page);
    if (fehlschlag) {
      await a.page.evaluate(() => { document.execCommand = () => false; });
    }
    const zeit = fremd.zeiten[1];
    await schalter(a.page, zeit).tap();
    await expect(kopierKnopf(a.page, zeit)).toBeVisible();

    await kopierKnopf(a.page, zeit).tap();
    const angetippt = Date.now();

    await expect(kopierKnopf(a.page, zeit).locator('.lc-aktion-anzeige')).toHaveText(fehlschlag ? '⚠ Kopieren nicht möglich' : '✓ Kopiert.');
    await expect(kopierKnopf(a.page, zeit)).toHaveAccessibleName('Kopieren');
    const menueRechts = await nachrichtMitZeit(a.page, zeit).locator('.lc-aktionen').evaluate((menue) => menue.getBoundingClientRect().right);
    const schalterLinks = await schalter(a.page, zeit).evaluate((knopf) => knopf.getBoundingClientRect().left);
    expect(menueRechts, 'das Menü überdeckt den Schalter nicht').toBeLessThanOrEqual(schalterLinks);
    await a.page.waitForTimeout(Math.max(0, 1000 - (Date.now() - angetippt)));
    await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'true');
    await a.page.waitForTimeout(Math.max(0, 1500 - (Date.now() - angetippt)));
    await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'false');
    await expect(schalter(a.page, zeit)).toBeFocused();
    if (fehlschlag) {
      expect(await ablageLesen(a.page)).toBe(WAECHTER);
    } else {
      expect(await ablageLesen(a.page)).toBe('Zwei');
    }
    expect(a.fehler).toEqual([]);
  });
}

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

// Nacharbeit #78 (UX S1, code-reviewer A2): scrollt der Verlauf von selbst — eine neue Nachricht am Ende zieht ihn mit —,
// bleibt das Menü offen und wandert mit seinem Schalter; scrollt der Nutzer (Rad, Wischen), schließt es.

/// Lage von Menü, Schalter und sichtbarem Teil des Verlaufs der Nachricht `zeit` im Fenster.
function menueUndSchalter(page, zeit) {
  return page.evaluate((nachrichtZeit) => {
    const artikel = document.querySelector(`.lc-msg[data-zeit="${nachrichtZeit}"]`);
    const menue = artikel.querySelector('.lc-aktionen').getBoundingClientRect();
    const knopf = artikel.querySelector('.lc-aktionen-schalter').getBoundingClientRect();
    const log = document.querySelector('[role="log"]').getBoundingClientRect();
    return {
      menue: { oben: menue.top, unten: menue.bottom }, schalter: { oben: knopf.top, unten: knopf.bottom }, verlauf: { oben: log.top, unten: log.bottom },
    };
  }, zeit);
}

test('Touch-Menü bei Live-Autoscroll (UX S1): an der letzten Nachricht offen, ein zweiter Teilnehmer sendet — nach 900 ms offen, ganz im Verlauf, in Höhe des Schalters; Kopieren zeigt „✓ Kopiert.“', async ({ browser }) => {
  const verlauf = verlaufVon('beta@127.0.0.2', 30);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await pruefFeldAnlegen(a.page);
  const letzte = verlauf.zeiten[29];
  await expect(nachrichtMitZeit(a.page, letzte)).toBeVisible();
  await schalter(a.page, letzte).tap();
  await expect(schalter(a.page, letzte)).toHaveAttribute('aria-expanded', 'true');

  const zweiter = await claudeSendet(instanz.adresse, ['Einunddreißig kommt live']);
  await expect(nachrichtMitZeit(a.page, zweiter.zeiten[0])).toBeVisible();
  await a.page.waitForTimeout(900);

  await expect(schalter(a.page, letzte)).toHaveAttribute('aria-expanded', 'true');
  const lage = await menueUndSchalter(a.page, letzte);
  expect(lage.menue.oben).toBeGreaterThanOrEqual(80);
  expect(lage.menue.unten).toBeLessThanOrEqual(581);
  expect(lage.menue.oben).toBeGreaterThanOrEqual(lage.verlauf.oben);
  expect(lage.menue.unten).toBeLessThanOrEqual(lage.verlauf.unten);
  const inHoehe = Math.abs(lage.menue.oben - lage.schalter.oben) <= 1;
  const buendigHochgeschoben = Math.abs(lage.menue.unten - (lage.verlauf.unten - 4)) <= 1 && lage.menue.oben <= lage.schalter.oben;
  expect(inHoehe || buendigHochgeschoben, JSON.stringify(lage)).toBe(true);
  const t = await treffer(kopierKnopf(a.page, letzte));
  expect(t.getroffen).toEqual([true, true, true, true, true]);
  await ablageBelegen(a.page);
  await kopierKnopf(a.page, letzte).tap();
  await expect(kopierKnopf(a.page, letzte).locator('.lc-aktion-anzeige')).toHaveText('✓ Kopiert.');
  expect(await ablageLesen(a.page)).toBe('Nachricht 30');
  expect(a.fehler).toEqual([]);
});

test('Touch-Menü an der vorletzten von 30 Nachrichten, ein Agent sendet (A2 Probe 2): das Menü bleibt offen, „Kopieren“ trifft an Mitte und vier Ecken', async ({ browser }) => {
  const verlauf = verlaufVon('beta@127.0.0.2', 30);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const vorletzte = verlauf.zeiten[28];
  await expect(nachrichtMitZeit(a.page, verlauf.zeiten[29])).toBeVisible();
  await schalter(a.page, vorletzte).tap();
  await expect(schalter(a.page, vorletzte)).toHaveAttribute('aria-expanded', 'true');

  const agent = await claudeSendet(instanz.adresse, ['Agent meldet sich']);
  await expect(nachrichtMitZeit(a.page, agent.zeiten[0])).toBeVisible();
  await a.page.waitForTimeout(900);

  await expect(schalter(a.page, vorletzte)).toHaveAttribute('aria-expanded', 'true');
  const t = await treffer(kopierKnopf(a.page, vorletzte));
  expect(t.imSichtbaren).toBe(true);
  expect(t.getroffen).toEqual([true, true, true, true, true]);
  expect(a.fehler).toEqual([]);
});

test('Touch-Menü: verlässt der Schalter durch neue Nachrichten den sichtbaren Teil des Verlaufs, schließt das Menü', async ({ browser }) => {
  const verlauf = verlaufVon('beta@127.0.0.2', 30);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_MIT_TASTATUR, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const letzte = verlauf.zeiten[29];
  await expect(nachrichtMitZeit(a.page, letzte)).toBeVisible();
  await schalter(a.page, letzte).tap();
  await expect(schalter(a.page, letzte)).toHaveAttribute('aria-expanded', 'true');

  const agent = await claudeSendet(instanz.adresse, Array.from({ length: 12 }, (_, nummer) => `Flut ${nummer + 1}`));
  await expect(nachrichtMitZeit(a.page, agent.zeiten[11])).toBeVisible();

  await expect(schalter(a.page, letzte)).toHaveAttribute('aria-expanded', 'false');
  await expect(nachrichtMitZeit(a.page, letzte).locator('.lc-aktionen')).toBeHidden();
  expect(a.fehler).toEqual([]);
});

test('Touch-Menü: scrollt der Nutzer den Verlauf um 40 px (Rad), schließt es (A2 Probe 1)', async ({ browser }) => {
  const verlauf = verlaufVon('beta@127.0.0.2', 30);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = verlauf.zeiten[27];
  await expect(nachrichtMitZeit(a.page, verlauf.zeiten[29])).toBeVisible();
  await schalter(a.page, zeit).tap();
  await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'true');
  const scrollTop = () => a.page.locator('[role="log"]').evaluate((log) => log.scrollTop);
  const vorher = await scrollTop();

  const mitte = await a.page.locator('[role="log"]').evaluate((log) => {
    const rand = log.getBoundingClientRect();
    return { x: rand.left + 40, y: rand.top + rand.height / 2 };
  });
  await a.page.mouse.move(mitte.x, mitte.y);
  await a.page.mouse.wheel(0, -40);
  await expect.poll(scrollTop).toBeLessThan(vorher);

  await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'false');
  expect(a.fehler).toEqual([]);
});

test('Touch-Menü: Wischen im Verlauf schließt es (UX S1 Zusatz)', async ({ browser }) => {
  const verlauf = verlaufVon('beta@127.0.0.2', 30);
  instanz = await sendenStarten({ vorbereiten: verlauf.vorbereiten });
  const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = verlauf.zeiten[27];
  await expect(nachrichtMitZeit(a.page, verlauf.zeiten[29])).toBeVisible();
  await schalter(a.page, zeit).tap();
  await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'true');
  const scrollTop = () => a.page.locator('[role="log"]').evaluate((log) => log.scrollTop);
  const vorher = await scrollTop();
  const start = await a.page.locator('[role="log"]').evaluate((log) => {
    const rand = log.getBoundingClientRect();
    return { x: rand.left + 60, y: rand.top + 60 };
  });

  const befehle = await a.page.context().newCDPSession(a.page);
  await befehle.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [start] });
  for (let schritt = 1; schritt <= 10; schritt += 1) {
    await befehle.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x: start.x, y: start.y + schritt * 20 }] });
  }
  await befehle.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await expect.poll(scrollTop).toBeLessThan(vorher);

  await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'false');
  expect(a.fehler).toEqual([]);
});

// Nacharbeit #78 (UX S2): an der Nachricht im Bearbeiten-Feld gibt es kein Menü — der Schalter „⋯“ ist weg, solange das
// Feld offen ist, und ein offenes Menü schließt, wenn das Feld aufgeht.
const schalterAnzeige = (page, zeit) => nachrichtMitZeit(page, zeit).locator('.lc-aktionen-schalter').evaluate((knopf) => getComputedStyle(knopf).display);

for (const ende of ['Abbrechen', 'Speichern']) {
  test(`Touch: Bearbeiten über das Menü — am Feld kein Schalter „⋯“ und kein Menü; nach ${ende} wieder da (UX S2)`, async ({ browser }) => {
    instanz = await sendenStarten();
    const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const zeit = await imBrowserSenden(a.page, 'Eigene');
    await schalter(a.page, zeit).tap();
    await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Bearbeiten', exact: true }).tap();
    const feld = a.page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' });
    await expect(feld).toBeFocused();

    expect(await schalterAnzeige(a.page, zeit)).toBe('none');
    await expect(a.page.locator('.lc-aktionen-schalter[aria-expanded="true"]')).toHaveCount(0);
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-aktionen')).toBeHidden();

    if (ende === 'Speichern') {
      await feld.fill('Eigene, geändert');
    }
    await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: ende, exact: true }).tap();
    await expect(feld).toHaveCount(0);
    expect(await schalterAnzeige(a.page, zeit)).toBe('flex');
    await schalter(a.page, zeit).tap();
    await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'true');
    await expect(kopierKnopf(a.page, zeit)).toBeVisible();
    expect(a.fehler).toEqual([]);
  });
}

test('Touch mit Tastatur: Menü offen, Taste E auf der Nachricht öffnet das Feld — das Menü schließt (UX S2)', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Eigene');
  await schalter(a.page, zeit).tap();
  await expect(schalter(a.page, zeit)).toHaveAttribute('aria-expanded', 'true');

  await nachrichtMitZeit(a.page, zeit).focus();
  await a.page.keyboard.press('e');

  await expect(a.page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' })).toBeFocused();
  await expect(a.page.locator('.lc-aktionen-schalter[aria-expanded="true"]')).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-aktionen')).toBeHidden();
  expect(a.fehler).toEqual([]);
});

// Nacharbeit #78 (code-reviewer A1): Kopieren lässt die Markierung des Lesenden stehen — im Verlauf wie im Eingabefeld.
test('Markierung bleibt: „Wort“ markiert, „Kopieren“ an einer anderen Nachricht — die Markierung ist wieder „Wort“, die Ablage „Zweite“ (A1)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Ein Wort steht hier', 'Zweite']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[1])).toHaveCount(1);
  await ablageBelegen(a.page);
  await nachrichtMitZeit(a.page, fremd.zeiten[0]).locator('.lc-body').evaluate((body) => {
    const textknoten = document.createTreeWalker(body, NodeFilter.SHOW_TEXT);
    let knoten = textknoten.nextNode();
    while (knoten !== null && !knoten.textContent.includes('Wort')) {
      knoten = textknoten.nextNode();
    }
    const anfang = knoten.textContent.indexOf('Wort');
    const bereich = document.createRange();
    bereich.setStart(knoten, anfang);
    bereich.setEnd(knoten, anfang + 'Wort'.length);
    document.getSelection().removeAllRanges();
    document.getSelection().addRange(bereich);
  });
  expect(await a.page.evaluate(() => document.getSelection().toString())).toBe('Wort');

  await ueberHoverKopieren(a.page, fremd.zeiten[1]);

  await expect(kopierKnopf(a.page, fremd.zeiten[1]).locator('.lc-aktion-anzeige')).toHaveText('✓ Kopiert.');
  expect(await a.page.evaluate(() => document.getSelection().toString())).toBe('Wort');
  expect(await ablageLesen(a.page)).toBe('Zweite');
  expect(a.fehler).toEqual([]);
});

test('Markierung im Eingabefeld bleibt: „Welt“ markiert, „Kopieren“ an einer Nachricht — Feld und Markierung unverändert (A1 Variante)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Zweite']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(a.page, fremd.zeiten[0])).toHaveCount(1);
  await ablageBelegen(a.page);
  await eingabe(a.page).fill('Hallo Welt');
  await eingabe(a.page).evaluate((feld) => feld.setSelectionRange(6, 10));

  await ueberHoverKopieren(a.page, fremd.zeiten[0]);

  await expect(kopierKnopf(a.page, fremd.zeiten[0]).locator('.lc-aktion-anzeige')).toHaveText('✓ Kopiert.');
  expect(await eingabe(a.page).evaluate((feld) => [feld.value, feld.selectionStart, feld.selectionEnd])).toEqual(['Hallo Welt', 6, 10]);
  expect(await ablageLesen(a.page)).toBe('Zweite');
  expect(a.fehler).toEqual([]);
});

// Nacharbeit #78 (code-reviewer A3): die Zeitgrenzen der Rückmeldung, im Browser ereignisgenau gemessen — das Menü auf
// Touch schließt 1200 ms nach dem Tippen, und ein zweites Kopieren beginnt die 2000 ms am Knopf neu.
for (const fehlschlag of [false, true]) {
  test(`Touch (${fehlschlag ? 'Fehlschlag' : 'Gelingen'}): das Menü schließt 1200 bis 1500 ms nach dem Tippen auf „Kopieren“ (A3 Probe a)`, async ({ browser }) => {
    instanz = await sendenStarten();
    const fremd = await claudeSendet(instanz.adresse, ['Eins', 'Zwei']);
    const a = await geraetOeffnen(browser, 'telefon', { fenster: FENSTER_TELEFON, streng: true });
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const zeit = fremd.zeiten[1];
    await expect(nachrichtMitZeit(a.page, zeit)).toHaveCount(1);
    if (fehlschlag) {
      await a.page.evaluate(() => { document.execCommand = () => false; });
    }
    await schalter(a.page, zeit).tap();
    await expect(kopierKnopf(a.page, zeit)).toBeVisible();
    await a.page.evaluate((nachrichtZeit) => {
      const menueSchalter = document.querySelector(`.lc-msg[data-zeit="${nachrichtZeit}"] .lc-aktionen-schalter`);
      window.menueZeiten = { getippt: null, zu: null };
      document.addEventListener('click', (ereignis) => {
        if (ereignis.target.closest('.lc-aktion[data-aktion="kopieren"]') !== null) {
          window.menueZeiten.getippt = performance.now();
        }
      }, true);
      new MutationObserver(() => {
        if (menueSchalter.getAttribute('aria-expanded') === 'false' && window.menueZeiten.zu === null) {
          window.menueZeiten.zu = performance.now();
        }
      }).observe(menueSchalter, { attributes: true, attributeFilter: ['aria-expanded'] });
    }, zeit);

    await kopierKnopf(a.page, zeit).tap();

    await a.page.waitForFunction(() => window.menueZeiten.zu !== null, null, { polling: 'raf', timeout: 10000 });
    const { getippt, zu } = await a.page.evaluate(() => window.menueZeiten);
    console.log(`Menü offen nach dem Tippen (${fehlschlag ? 'Fehlschlag' : 'Gelingen'}): ${Math.round(zu - getippt)} ms`);
    expect(getippt).not.toBeNull();
    expect(zu - getippt).toBeGreaterThanOrEqual(1200);
    expect(zu - getippt).toBeLessThanOrEqual(1500);
    expect(a.fehler).toEqual([]);
  });
}

test('Desktop: Kopieren bei t = 0 und t ≈ 1000 — bei t ≈ 2500 steht noch „✓ Kopiert.“, die Anzeige endet 2000 ms nach dem zweiten Mal (A3 Probe b)', async ({ browser }) => {
  instanz = await sendenStarten();
  const fremd = await claudeSendet(instanz.adresse, ['Zweimal']);
  const a = await seiteOeffnen(browser, instanz.adresse);
  const zeit = fremd.zeiten[0];
  await expect(nachrichtMitZeit(a.page, zeit)).toHaveCount(1);
  await anzeigeMitschreiben(a.page, zeit);
  await a.page.evaluate(() => {
    window.klicks = [];
    document.addEventListener('click', (ereignis) => {
      if (ereignis.target.closest('.lc-aktion[data-aktion="kopieren"]') !== null) {
        window.klicks.push(performance.now());
      }
    }, true);
  });

  await ueberHoverKopieren(a.page, zeit);
  await a.page.waitForTimeout(1000);
  await kopierKnopf(a.page, zeit).click();

  await a.page.waitForFunction(() => window.wechsel.some(([text]) => text === 'Kopieren'), null, { polling: 'raf', timeout: 10000 });
  const { klicks, wechsel } = await a.page.evaluate(() => ({ klicks: window.klicks, wechsel: window.wechsel }));
  const ende = wechsel.find(([text]) => text === 'Kopieren')[1];
  console.log(`Anzeige nach zweimal Kopieren: Abstand ${Math.round(klicks[1] - klicks[0])} ms, Ende ${Math.round(ende - klicks[0])} ms nach dem ersten`);
  expect(klicks).toHaveLength(2);
  expect(klicks[1] - klicks[0]).toBeGreaterThanOrEqual(900);
  expect(ende - klicks[0], 'bei t ≈ 2500 steht noch „✓ Kopiert.“').toBeGreaterThanOrEqual(2500);
  expect(ende - klicks[1]).toBeGreaterThanOrEqual(2000 - 300);
  expect(ende - klicks[1]).toBeLessThanOrEqual(2000 + 300);
  expect(a.fehler).toEqual([]);
});
