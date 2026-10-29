'use strict';
// R00011 F0044 Fremde Nachrichten im offenen Verlauf (US-17 bis US-21, Z1, Z2, Z5): gegen das echte Programm,
// Chromium 1280 × 800, Uhr ab 2026-10-28 15:00 (+01:00). A ist `shoff`, B ist `anna`; `alpha`, `beta` und `carla`
// senden per API von 127.0.0.1.
const { test, expect } = require('@playwright/test');
const { satz } = require('./hilfen');
const {
  sendenStarten, beitreten, senden, verlaufLesen, kontextOeffnen, imBrowserBeitreten, nachrichten, mitText,
} = require('./senden-hilfen');

/// Z1 aus R00011: Grenze der Zustellung an eine zweite Sicht.
const Z1_MS = 1000;

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const pille = (page) => page.locator('.lc-pillen .lc-pill');

/// Wartet im Browser vor jedem Bild, bis eine Nachricht mit `text` und Absender `von` im Verlauf steht, und liefert
/// `Date.now()` des Browsers in diesem Augenblick (gleiche Maschine wie der Testprozess).
function erscheint(page, text, von) {
  return page.waitForFunction(({ gesucht, absender }) => {
    const treffer = [...document.querySelectorAll('[role="log"] .lc-msg')]
      .find((nachricht) => nachricht.querySelector('.lc-body').textContent === gesucht && nachricht.dataset.von === absender);
    if (treffer === undefined) {
      return false;
    }
    return Date.now();
  }, { gesucht: text, absender: von }, { polling: 'raf', timeout: 10000 }).then((ergebnis) => ergebnis.jsonValue());
}

function abstandZumEnde(page) {
  return page.getByRole('log').evaluate((log) => log.scrollHeight - log.scrollTop - log.clientHeight);
}

// „Zweite Sicht“ (US-17, Z1): A, ein zweiter Tab A2 und B stehen am Ende von `# allgemein`; `beta` sendet über die
// API → in B (und A, A2) innerhalb Z1 nach der 201, B weiter am Ende, keine Pille.
test('US-17 (Z1): sofort bei allen anderen', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 20 });
  const a = await kontextOeffnen(browser, { streng: true });
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const a2 = await a.kontext.newPage();
  const a2Fehler = [];
  a2.on('pageerror', (ausnahme) => a2Fehler.push(ausnahme.message));
  a2.on('console', (meldung) => { if (meldung.type() === 'error') { a2Fehler.push(meldung.text()); } });
  await a2.goto(`${instanz.adresse}#/c/allgemein`);
  await expect(a2.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(20);
  const token = await beitreten(instanz.adresse, 'beta');

  const inB = erscheint(b.page, 'Hallo B', 'beta@127.0.0.1');
  const inA = erscheint(a.page, 'Hallo B', 'beta@127.0.0.1');
  const inA2 = erscheint(a2, 'Hallo B', 'beta@127.0.0.1');
  const antwort = await senden(instanz.adresse, token, 'allgemein', 'Hallo B');
  const dauern = [await inB, await inA, await inA2].map((zeitpunkt) => zeitpunkt - antwort.empfangen);

  console.log(`Z1 gemessen (B, A, A2): ${dauern.join(' / ')} ms, Grenze ${Z1_MS} ms`);
  expect(antwort.status).toBe(201);
  for (const dauer of dauern) {
    expect(dauer).toBeLessThan(Z1_MS);
  }
  const neue = nachrichten(b.page).last();
  await expect(neue.locator('.lc-id')).toHaveText('beta@127.0.0.1');
  await expect(neue.locator('.lc-tag')).toHaveText('Agent');
  expect(await abstandZumEnde(b.page)).toBeLessThanOrEqual(1);
  await expect(pille(b.page)).toBeHidden();
  expect([...a.fehler, ...b.fehler, ...a2Fehler]).toEqual([]);
});

// „Reihenfolge“ (US-18, Z2): alpha und beta senden abwechselnd je 5 → B zeigt 10 in der Folge der API, je einmal.
test('US-18 (Z2): dieselbe Reihenfolge überall', async ({ browser }) => {
  instanz = await sendenStarten();
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const alpha = await beitreten(instanz.adresse, 'alpha');
  const beta = await beitreten(instanz.adresse, 'beta');

  for (let nummer = 0; nummer < 5; nummer += 1) {
    await senden(instanz.adresse, alpha, 'allgemein', `alpha ${nummer}`);
    await senden(instanz.adresse, beta, 'allgemein', `beta ${nummer}`);
  }

  await expect(nachrichten(b.page)).toHaveCount(10);
  const api = await verlaufLesen(instanz.adresse, alpha, 'allgemein');
  const zeitenB = await nachrichten(b.page).evaluateAll((liste) => liste.map((nachricht) => nachricht.dataset.zeit));
  expect(zeitenB).toEqual(api.map((nachricht) => nachricht.zeit));
  expect(new Set(zeitenB).size).toBe(10);
  expect(b.fehler).toEqual([]);
});

// „Anderer Channel“ (US-21): B in `# allgemein`, beta sendet an `technik` → keine Änderung, keine Pille; in `# technik`
// steht sie.
test('US-21: andere Channels stören nicht', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 5 });
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(5);
  const beta = await beitreten(instanz.adresse, 'beta');

  await senden(instanz.adresse, beta, 'technik', 'nur in technik');
  await b.page.waitForTimeout(500);

  await expect(nachrichten(b.page)).toHaveCount(5);
  await expect(pille(b.page)).toBeHidden();
  await b.page.getByRole('navigation', { name: 'Channels' }).getByRole('link', { name: 'technik', exact: true }).click();
  await expect(nachrichten(b.page).locator('.lc-body')).toHaveText(['nur in technik']);
  expect(b.fehler).toEqual([]);
});

// „Nicht am Ende“ (US-19, Z5): 300 Nachrichten; B zwei Bildschirmhöhen über dem Ende; beta sendet 3 → scrollTop ± 1 px,
// Pille „3 neue Nachrichten“; Klick → die erste der drei oben unter dem Tagestrenner mit einer Zeile Kontext; am Ende
// verschwindet die Pille.
test('US-19 (Z5): wer oben liest, bleibt oben', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 300 });
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const log = b.page.getByRole('log');
  await expect(nachrichten(b.page)).not.toHaveCount(0);
  const sicht = await log.evaluate((element) => element.clientHeight);
  await log.evaluate((element) => { element.scrollTop = element.scrollHeight - element.clientHeight * 3; });
  await expect.poll(() => abstandZumEnde(b.page)).toBeGreaterThanOrEqual(2 * sicht - 1);
  const vorher = await log.evaluate((element) => element.scrollTop);
  const beta = await beitreten(instanz.adresse, 'beta');

  for (const text of ['neu 1', 'neu 2', 'neu 3']) {
    await senden(instanz.adresse, beta, 'allgemein', text);
  }

  await expect(pille(b.page)).toHaveText(/^↓ 3 neue Nachrichten/);
  const nachher = await log.evaluate((element) => element.scrollTop);
  expect(Math.abs(nachher - vorher)).toBeLessThanOrEqual(1);
  await pille(b.page).click();
  const erste = mitText(b.page, /^neu 1$/);
  await expect(erste).toBeInViewport();
  // Z5 seit R00020 F0067: unter dem klebenden Tagestrenner; die Spanne ± 1 px prüft r00020-sprung.spec.js.
  const lage = await erste.evaluate((nachricht) => {
    const log = nachricht.closest('[role="log"]');
    const oberkante = nachricht.getBoundingClientRect().top - parseFloat(getComputedStyle(nachricht).marginTop);
    const trenner = log.querySelector('.lc-day').getBoundingClientRect().height;
    return { abstand: oberkante - log.getBoundingClientRect().top, trenner, zeile: parseFloat(getComputedStyle(nachricht).lineHeight) };
  });
  expect(lage.abstand).toBeLessThanOrEqual(lage.trenner + lage.zeile + 1);
  await log.evaluate((element) => { element.scrollTop = element.scrollHeight; });
  await expect(pille(b.page)).toBeHidden();
  expect(b.fehler).toEqual([]);
});

// „Neuester Block abgeworfen“ (US-20): 1.000 Nachrichten; B nach oben, bis der neueste Block weg ist; beta sendet 2 →
// keine neue im DOM, Pille „2 neue Nachrichten“; Ende im Verlauf → am Ende mit beiden.
test('US-20: ganz weit oben wird nur gezählt', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 1000 });
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const log = b.page.getByRole('log');
  await expect(nachrichten(b.page)).not.toHaveCount(0);
  const letzteZeit = await nachrichten(b.page).last().getAttribute('data-zeit');
  await expect(async () => {
    await log.evaluate((element) => { element.scrollTop = 0; });
    await expect(log.locator(`.lc-msg[data-zeit="${letzteZeit}"]`)).toHaveCount(0, { timeout: 200 });
  }).toPass({ timeout: 20000 });
  const beta = await beitreten(instanz.adresse, 'beta');

  await senden(instanz.adresse, beta, 'allgemein', 'weit weg 1');
  await senden(instanz.adresse, beta, 'allgemein', 'weit weg 2');

  await expect(pille(b.page)).toHaveText(/^↓ 2 neue Nachrichten/);
  await expect(mitText(b.page, /^weit weg/)).toHaveCount(0);
  await log.focus();
  await b.page.keyboard.press('End');
  await expect(mitText(b.page, /^weit weg/)).toHaveCount(2);
  await expect.poll(() => abstandZumEnde(b.page)).toBeLessThanOrEqual(1);
  expect(b.fehler).toEqual([]);
});

// „Art des Absenders“ (US-21): beta (Agent), carla (`curl` mit `"art":"mensch"`) und ein Satz von claude@10.0.0.1 ohne
// Metazeile → beim Öffnen und live trägt beta die Marke „Agent“ in kd-agent, carla und claude keine, in kd-mensch.
test('US-21: Agent und Mensch auf einen Blick — beim Öffnen und live', async ({ browser }) => {
  instanz = await sendenStarten({
    vorbereiten: (ordner) => {
      require('fs').writeFileSync(`${ordner}/channels/allgemein/2026-10-28.log`, satz('2026-10-28T09:00:00.000+01:00', 'claude@10.0.0.1', 'von Hand'));
    },
  });
  const beta = await beitreten(instanz.adresse, 'beta');
  const carla = await beitreten(instanz.adresse, 'carla', { art: 'mensch' });
  await senden(instanz.adresse, beta, 'allgemein', 'vom Agenten');
  await senden(instanz.adresse, carla, 'allgemein', 'vom Menschen');
  const b = await kontextOeffnen(browser, { streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  await expect(nachrichten(b.page)).toHaveCount(3);

  await senden(instanz.adresse, beta, 'allgemein', 'live vom Agenten');
  await senden(instanz.adresse, carla, 'allgemein', 'live vom Menschen');
  await expect(nachrichten(b.page)).toHaveCount(5);

  const farben = await b.page.evaluate(() => {
    const farbe = (wert) => {
      const probe = document.createElement('span');
      probe.style.color = `var(${wert})`;
      document.body.append(probe);
      const ergebnis = getComputedStyle(probe).color;
      probe.remove();
      return ergebnis;
    };
    return { agent: farbe('--kd-agent'), mensch: farbe('--kd-mensch') };
  });
  const zeilen = await nachrichten(b.page).evaluateAll((liste) => liste.map((nachricht) => {
    const marke = nachricht.querySelector('.lc-tag');
    let markentext = null;
    if (marke !== null) {
      markentext = marke.textContent;
    }
    return {
      text: nachricht.querySelector('.lc-body').textContent,
      marke: markentext,
      name: getComputedStyle(nachricht.querySelector('.lc-meta .lc-id')).color,
      kachel: getComputedStyle(nachricht.querySelector('.lc-tile')).color,
    };
  }));
  const erwartet = (text, art) => {
    let marke = null;
    if (art === 'agent') {
      marke = 'Agent';
    }
    return { text, marke, name: farben[art], kachel: farben[art] };
  };
  expect(zeilen).toEqual([
    erwartet('von Hand', 'mensch'),
    erwartet('vom Agenten', 'agent'),
    erwartet('vom Menschen', 'mensch'),
    erwartet('live vom Agenten', 'agent'),
    erwartet('live vom Menschen', 'mensch'),
  ]);
  expect(b.fehler).toEqual([]);
});
