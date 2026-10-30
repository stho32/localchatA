'use strict';
// R00030 F0101 B0449 — gewählte Dateien über dem Feld: Liste „Angehängte Dateien“ mit Name, Größe in Worten und
// „Entfernen“, höchstens 10, je Channel im Speicher des Tabs, Auswahl angesagt (UX M2), Bereit mit Dateien, Fokus
// sichtbar (UX M3), Zielflächen, und die Liste verdeckt die letzte Nachricht nicht (R00048).
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe, nachrichten } = require('./senden-hilfen');
const { geraetOeffnen, knopf, box } = require('./r00027-hilfen');
const {
  schalter, dateiliste, dateizeilen, waehlen, fotoPng, textDatei, ansagenMitschreiben, liveRegionen, ringGegenFlaeche,
} = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

async function seiteAuf(browser, geraet, optionen = {}, start = {}) {
  instanz = await sendenStarten(start);
  const seite = await geraetOeffnen(browser, geraet, optionen);
  await imBrowserBeitreten(seite.page, instanz.adresse, 'shoff');
  return seite;
}

const entfernen = (page, name) => dateiliste(page).getByRole('button', { name: `${name} entfernen`, exact: true });
const zwoelf = () => Array.from({ length: 12 }, (_, stelle) => textDatei(`d${stelle + 1}.txt`, 100));

test('Telefon: tippen, foto.png wählen → über dem Feld eine Zeile foto.png mit „300 KiB“ und „Entfernen“; „Senden“ ist bereit', async ({ browser }) => {
  const { page, fehler } = await seiteAuf(browser, 'telefon');
  await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');

  await waehlen(page, [fotoPng()], { geste: 'tippen' });

  await expect(dateizeilen(page)).toHaveCount(1);
  await expect(dateizeilen(page).first()).toContainText('foto.png');
  await expect(dateizeilen(page).first()).toContainText('300 KiB');
  await expect(entfernen(page, 'foto.png')).toHaveText('Entfernen');
  await expect(knopf(page)).not.toHaveAttribute('aria-disabled', /.*/);
  expect((await box(dateiliste(page))).bottom).toBeLessThanOrEqual((await box(eingabe(page))).top);
  await expect(eingabe(page)).toBeFocused();
  expect(fehler).toEqual([]);
});

test('Zehn Dateien (Desktop): 12 gewählt → 10 Zeilen und der Hinweis; „Entfernen“ an der dritten → 9, Fokus auf dem nächsten; zuletzt im Feld', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'desktop');

  await waehlen(page, zwoelf());

  await expect(dateizeilen(page)).toHaveCount(10);
  await expect(page.locator('#lc-comp-hinweis')).toHaveText('Höchstens 10 Dateien je Nachricht — 2 nicht übernommen.');
  await entfernen(page, 'd3.txt').click();
  await expect(dateizeilen(page)).toHaveCount(9);
  await expect(entfernen(page, 'd4.txt')).toBeFocused();
  for (let rest = 9; rest > 0; rest -= 1) {
    await page.keyboard.press('Enter');
    await expect(dateizeilen(page)).toHaveCount(rest - 1);
    const aktiv = await page.evaluate(() => document.activeElement.getAttribute('aria-label') || document.activeElement.tagName);
    expect(aktiv).not.toBe('BODY');
  }
  await expect(eingabe(page)).toBeFocused();
  await expect(dateiliste(page)).toBeHidden();
  await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
});

test('Channelwechsel (Desktop): foto.png in # allgemein, in # technik keine Zeile, zurück ist sie wieder da', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'desktop');
  await waehlen(page, [fotoPng()]);
  await expect(dateizeilen(page)).toHaveCount(1);

  await page.getByRole('link', { name: 'technik' }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(dateizeilen(page)).toHaveCount(0);
  await expect(knopf(page)).toHaveAttribute('aria-disabled', 'true');
  await page.getByRole('link', { name: 'allgemein' }).click();
  await expect(page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await expect(dateizeilen(page)).toHaveCount(1);
  await expect(dateizeilen(page).first()).toContainText('foto.png');
});

test('Auswahl angesagt (Desktop, UX M2): zwei, eine und zwölf Dateien je genau einmal', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'desktop');
  const angesagt = await ansagenMitschreiben(page);

  await waehlen(page, [fotoPng(), textDatei()]);
  await expect.poll(angesagt).toEqual(['2 Dateien angehängt: foto.png, log.txt']);
  await entfernen(page, 'foto.png').click();
  await entfernen(page, 'log.txt').click();
  await waehlen(page, [textDatei()]);
  await expect.poll(angesagt).toEqual(['2 Dateien angehängt: foto.png, log.txt', 'Datei angehängt: log.txt']);
  await entfernen(page, 'log.txt').click();
  await waehlen(page, zwoelf());
  const zehn = Array.from({ length: 10 }, (_, stelle) => `d${stelle + 1}.txt`).join(', ');
  await expect.poll(angesagt).toEqual([
    '2 Dateien angehängt: foto.png, log.txt',
    'Datei angehängt: log.txt',
    `10 Dateien angehängt: ${zehn} — 2 nicht übernommen, höchstens 10 je Nachricht`,
  ]);
});

test('Bildschirmleser (Desktop): Liste „Angehängte Dateien“, je Datei „<name> entfernen“, keine zusätzliche Live-Region', async ({ browser }) => {
  const { page } = await seiteAuf(browser, 'desktop');
  const vorher = await liveRegionen(page);

  await waehlen(page, [fotoPng(), textDatei()]);

  await expect(dateiliste(page)).toHaveCount(1);
  await expect(dateizeilen(page)).toHaveCount(2);
  await expect(entfernen(page, 'foto.png')).toHaveCount(1);
  await expect(entfernen(page, 'log.txt')).toHaveCount(1);
  await expect(dateizeilen(page).first().locator('.lc-datei-kuerzel')).toHaveAttribute('aria-hidden', 'true');
  expect(await liveRegionen(page)).toBe(vorher);
});

for (const thema of ['light', 'dark']) {
  test(`Fokus sichtbar (Desktop, ${thema}, UX M3): „foto.png entfernen“ mit eigenem Ring`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, 'desktop', { thema });
    await waehlen(page, [fotoPng()]);

    await eingabe(page).focus();
    await page.keyboard.press('Shift+Tab');
    await expect(entfernen(page, 'foto.png')).toBeFocused();
    const ring = await entfernen(page, 'foto.png').evaluate(ringGegenFlaeche);

    expect(ring.sichtbar).toBe(true);
    expect(ring.stil).not.toBe('none');
    expect(ring.breite).toBeGreaterThanOrEqual(2);
    expect(ring.versatz).toBeGreaterThanOrEqual(0);
    expect(ring.kontrast).toBeGreaterThanOrEqual(3);
  });
}

for (const [geraet, mindestens, abstand] of [['telefon', 44, 8], ['tablet', 44, 8], ['desktop', 24, 0]]) {
  test(`Zielfläche (${geraet}): „Entfernen“ mindestens ${mindestens} × ${mindestens} px, ${abstand} px zwischen zwei Schaltern`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, geraet);
    await waehlen(page, [fotoPng(), textDatei()], { geste: geraet === 'desktop' ? 'klick' : 'tippen' });

    const erster = await box(entfernen(page, 'foto.png'));
    const zweiter = await box(entfernen(page, 'log.txt'));
    for (const masse of [erster, zweiter]) {
      expect(masse.width).toBeGreaterThanOrEqual(mindestens);
      expect(masse.height).toBeGreaterThanOrEqual(mindestens);
    }
    expect(zweiter.top - erster.bottom).toBeGreaterThanOrEqual(abstand);
  });
}

for (const thema of ['light', 'dark']) {
  test(`Kontrast (Desktop, ${thema}): Name und Größe ≥ 4,5 : 1 gegen ihre Fläche`, async ({ browser }) => {
    const { page } = await seiteAuf(browser, 'desktop', { thema });
    await waehlen(page, [fotoPng()]);

    for (const teil of ['.lc-datei-name', '.lc-datei-groesse']) {
      const kontrast = await dateiliste(page).locator(teil).evaluate(schriftKontrast);
      expect(kontrast, teil).toBeGreaterThanOrEqual(4.5);
    }
  });
}

/// Kontrast der Schrift gegen die erste deckende Fläche darunter, mit der opacity aller Vorfahren.
function schriftKontrast(element) {
  function anteile(farbe) {
    return farbe.match(/[\d.]+/g).map(Number);
  }
  function leuchtdichte([r, g, b]) {
    const [lr, lg, lb] = [r, g, b].map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
  }
  let flaeche = [255, 255, 255];
  let deckkraft = 1;
  for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
    deckkraft *= parseFloat(getComputedStyle(knoten).opacity);
  }
  for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
    const farbe = anteile(getComputedStyle(knoten).backgroundColor);
    if (farbe.length === 3 || farbe[3] === 1) {
      flaeche = farbe.slice(0, 3);
      break;
    }
  }
  const schrift = anteile(getComputedStyle(element).color).slice(0, 3).map((wert, stelle) => wert * deckkraft + flaeche[stelle] * (1 - deckkraft));
  const hell = Math.max(leuchtdichte(schrift), leuchtdichte(flaeche));
  const dunkel = Math.min(leuchtdichte(schrift), leuchtdichte(flaeche));
  return (hell + 0.05) / (dunkel + 0.05);
}

// Probe aus R00048 auf die Liste angewandt: die letzte Nachricht ist ganz sichtbar, drei Dateien kommen dazu → sie
// bleibt ganz sichtbar (Unterkante ≤ Oberkante der Eingabe); „Entfernen“ an allen dreien → scrollTop ändert sich in
// keinem Schritt um mehr als die Höhe der entfernten Zeile.
for (const fenster of [{ width: 375, height: 667 }, { width: 320, height: 568 }]) {
  test(`Liste verdeckt nichts (Telefon ${fenster.width} × ${fenster.height}): die letzte Nachricht bleibt ganz sichtbar, beim Entfernen springt nichts`, async ({ browser }) => {
    const { page, fehler } = await seiteAuf(browser, 'telefon', { fenster }, { anzahl: 30 });
    await expect(nachrichten(page)).toHaveCount(30);
    await expect.poll(() => letzteGanzSichtbar(page)).toBe(true);

    await waehlen(page, [fotoPng(), textDatei(), textDatei('notiz.txt')], { geste: 'tippen' });
    await expect(dateizeilen(page)).toHaveCount(3);
    await expect.poll(() => letzteGanzSichtbar(page), { intervals: [25], timeout: 3000 }).toBe(true);

    for (const name of ['foto.png', 'log.txt', 'notiz.txt']) {
      const vorher = await page.getByRole('log').evaluate((log) => log.scrollTop);
      const zeilenhoehe = (await box(dateizeilen(page).first())).height;
      await entfernen(page, name).tap();
      await expect(entfernen(page, name)).toHaveCount(0);
      await expect.poll(() => letzteGanzSichtbar(page), { intervals: [25], timeout: 3000 }).toBe(true);
      const nachher = await page.getByRole('log').evaluate((log) => log.scrollTop);
      expect(Math.abs(nachher - vorher), `scrollTop nach dem Entfernen von ${name}`).toBeLessThanOrEqual(zeilenhoehe + 8);
    }
    expect(fehler).toEqual([]);
  });
}

/// Die letzte Nachricht steht ganz im Verlauf und über der Eingabe.
function letzteGanzSichtbar(page) {
  return page.evaluate(() => {
    const log = document.querySelector('[role="log"]');
    const alle = log.querySelectorAll('.lc-msg');
    const letzte = alle[alle.length - 1].getBoundingClientRect();
    const sicht = log.getBoundingClientRect();
    const eingabeOben = document.querySelector('.lc-comp').getBoundingClientRect().top;
    return letzte.top >= sicht.top - 0.5 && letzte.bottom <= sicht.bottom + 0.5 && letzte.bottom <= eingabeOben + 0.5;
  });
}
