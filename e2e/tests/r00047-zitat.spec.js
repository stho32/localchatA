'use strict';
// R00047 I0044 F0159: das Zitat über dem Text einer Antwort (B0716, ADR-0053 Regel 6, P11, Mutant Z9) — Kopf, zwei Zeilen,
// Zustände, Name des Knopfes und der Antwort (Z1), Tab-Weg, nur als Text.
const http = require('http');
const { test, expect } = require('./rahmen');
const { channelAnlegen } = require('./hilfen');
const { sendenStarten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const { imBrowserBeitreten } = require('./senden-hilfen');
const { nachrichtMitZeit, loeschenPerApi, mitAnhaengenSenden, PNG } = require('./r00041-hilfen');
const {
  ANNA, CLAUDE, annaSchreibt, claudeSchreibt, mitBezugSenden, benOeffnen, zitatVon,
} = require('./r00047-hilfen');

let instanz;
let server = null;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
  if (server !== null) {
    await new Promise((fertig) => server.close(fertig));
    server = null;
  }
});

const ANNA_KENNUNG = 'anna@127.0.0.2';
const CLAUDE_KENNUNG = 'claude@127.0.0.3';
/// HH:MM einer Zeit, wie die Instanz sie liefert (im Versatz der Nachricht).
const hhmm = (zeit) => zeit.slice(11, 16);

test('Zitat über dem Text: Kopf „↩ <Kennung> <HH:MM>“, darunter der Textanfang; ein Knopf „Zur Nachricht springen: …“ ohne das ↩; die Antwort nennt ihr Original (Z1) (B0716)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Ja', anna.zeiten[0], CLAUDE);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);

  const zitat = zitatVon(b.page, antwort);
  await expect(zitat).toHaveCount(1);
  expect(await zitat.evaluate((knopf) => knopf.tagName)).toBe('BUTTON');
  await expect(zitat).toHaveAccessibleName(`Zur Nachricht springen: ${ANNA_KENNUNG} ${hhmm(anna.zeiten[0])}, Frage?`);
  await expect(zitat.locator('.lc-zitat-kopf')).toHaveText(`↩ ${ANNA_KENNUNG} ${hhmm(anna.zeiten[0])}`);
  await expect(zitat.locator('[aria-hidden="true"]')).toHaveText('↩');
  await expect(zitat.locator('.lc-zitat-text')).toHaveText('Frage?');
  const stehtUeberDemText = await nachrichtMitZeit(b.page, antwort).evaluate((knoten) => knoten.querySelector('.lc-zitat').getBoundingClientRect().bottom <= knoten.querySelector('.lc-body').getBoundingClientRect().top);
  expect(stehtUeberDemText).toBe(true);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveAttribute('aria-label', `${CLAUDE_KENNUNG}, ${hhmm(antwort)}, Antwort auf ${ANNA_KENNUNG} ${hhmm(anna.zeiten[0])}: Ja`);
  await expect(nachrichtMitZeit(b.page, anna.zeiten[0]).locator('.lc-zitat')).toHaveCount(0);

  await zitat.hover();
  expect(await zitat.locator('.lc-zitat-von').evaluate((kennung) => getComputedStyle(kennung).textDecorationLine)).toContain('underline');
  expect(b.fehler).toEqual([]);
});

test('Zitat im Detail: 300 Zeichen auf höchstens zwei Zeilen, auch bei 320 px ohne Querscroll; nur Anhang → „Anhang: bericht.pdf“; Antwort auf eine Antwort zeigt nur diese (B0716)', async ({ browser }) => {
  instanz = await sendenStarten();
  const lang = `Anfang ${'Wort '.repeat(59)}Ende`;
  const anna = await annaSchreibt(instanz.adresse, [lang]);
  const nurAnhang = await mitAnhaengenSenden(instanz.adresse, anna.token, '', [{ name: 'bericht.pdf', inhalt: PNG }], ANNA);
  const claude = await claudeSchreibt(instanz.adresse, []);
  const aufLang = await mitBezugSenden(instanz.adresse, claude.token, 'Zu lang', anna.zeiten[0], CLAUDE);
  const aufAnhang = await mitBezugSenden(instanz.adresse, claude.token, 'Danke', nurAnhang, CLAUDE);
  const aufAntwort = await mitBezugSenden(instanz.adresse, anna.token, 'Gern', aufAnhang, ANNA);

  for (const breite of [1280, 320]) {
    const b = await geraetOeffnen(browser, 'desktop', { fenster: { width: breite, height: 800 }, streng: true });
    await imBrowserBeitreten(b.page, instanz.adresse, `ben${breite}`);
    await expect(nachrichtMitZeit(b.page, aufAntwort)).toHaveCount(1);

    const zeilen = await zitatVon(b.page, aufLang).locator('.lc-zitat-text').evaluate((text) => ({
      hoehe: text.getBoundingClientRect().height, zeile: parseFloat(getComputedStyle(text).lineHeight),
    }));
    expect(zeilen.hoehe, `${breite} px`).toBeLessThanOrEqual(2 * zeilen.zeile + 0.5);
    expect(zeilen.hoehe, `${breite} px`).toBeGreaterThan(0);
    const breiten = await b.page.evaluate(() => ({ inhalt: document.documentElement.scrollWidth, fenster: document.documentElement.clientWidth }));
    expect(breiten.inhalt, `${breite} px`).toBe(breiten.fenster);

    await expect(zitatVon(b.page, aufAnhang).locator('.lc-zitat-text')).toHaveText('Anhang: bericht.pdf');
    await expect(zitatVon(b.page, aufAntwort).locator('.lc-zitat-kopf')).toContainText(CLAUDE_KENNUNG);
    await expect(zitatVon(b.page, aufAntwort).locator('.lc-zitat-text')).toHaveText('Danke');
    await expect(zitatVon(b.page, aufAntwort)).toHaveCount(1);
    expect(b.fehler).toEqual([]);
    await b.kontext.close();
  }
});

test('Zustände: Original gelöscht → das Zitat bleibt ein Knopf mit „Nachricht gelöscht“ ohne Text; entfallen → „Ältere Nachricht, nicht mehr vorhanden“; die Namen nennen es (B0716, Z1, Z4)', async ({ browser }) => {
  const entfallenAuf = '2026-10-20T10:00:00.000+01:00';
  const antwortAufEntfallenes = '2026-10-28T14:05:00.000+01:00';
  instanz = await sendenStarten({
    vorbereiten: (ordner) => {
      channelAnlegen(ordner, 'allgemein');
      require('fs').writeFileSync(`${ordner}/channels/allgemein/2026-10-28.log`,
        `@ ${antwortAufEntfallenes} ${ANNA_KENNUNG}\n+ art: mensch\n+ antwort_auf: ${entfallenAuf} ${CLAUDE_KENNUNG}\n  Weiß ich nicht\n\n`);
    },
  });
  const anna = await annaSchreibt(instanz.adresse, ['Geheim und bald weg']);
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Ja', anna.zeiten[0], CLAUDE);
  const geloescht = await loeschenPerApi(instanz.adresse, anna.token, anna.zeiten[0], ANNA);
  expect(geloescht.status).toBe(200);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);

  const zitat = zitatVon(b.page, antwort);
  expect(await zitat.evaluate((knopf) => knopf.tagName)).toBe('BUTTON');
  await expect(zitat.locator('.lc-zitat-text')).toHaveText('Nachricht gelöscht');
  await expect(zitat).not.toContainText('Geheim');
  await expect(zitat).toHaveAccessibleName(`Zur Nachricht springen: ${ANNA_KENNUNG} ${hhmm(anna.zeiten[0])}, Nachricht gelöscht`);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveAttribute('aria-label', `${CLAUDE_KENNUNG}, ${hhmm(antwort)}, Antwort auf ${ANNA_KENNUNG} ${hhmm(anna.zeiten[0])}, Nachricht gelöscht: Ja`);

  const alt = zitatVon(b.page, antwortAufEntfallenes);
  await expect(alt.locator('.lc-zitat-text')).toHaveText('Ältere Nachricht, nicht mehr vorhanden');
  await expect(alt.locator('.lc-zitat-kopf')).toHaveText(`↩ ${CLAUDE_KENNUNG} 10:00`);
  await expect(nachrichtMitZeit(b.page, antwortAufEntfallenes)).toHaveAttribute('aria-label', `${ANNA_KENNUNG}, 14:05, Antwort auf ältere Nachricht, nicht mehr vorhanden: Weiß ich nicht`);
  expect(b.fehler).toEqual([]);
});

test('Tab-Weg in der Antwort: Zitat → Aktionen → Eingabe (B0716, US-27)', async ({ browser }) => {
  instanz = await sendenStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Frage?']);
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Ja', anna.zeiten[0], CLAUDE);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);

  await nachrichtMitZeit(b.page, antwort).focus();
  const weg = [];
  for (let schritt = 0; schritt < 4; schritt += 1) {
    await b.page.keyboard.press('Tab');
    weg.push(await b.page.evaluate(() => document.activeElement.getAttribute('aria-label') || document.activeElement.textContent));
  }
  expect(weg).toEqual([
    `Zur Nachricht springen: ${ANNA_KENNUNG} ${hhmm(anna.zeiten[0])}, Frage?`, 'Antworten', 'Kopieren', 'Nachricht an # allgemein',
  ]);
  await expect(eingabe(b.page)).toBeFocused();
  expect(b.fehler).toEqual([]);
});

test('Zitat ist Text (P11, Mutant Z9): HTML und Markdown im Original erscheinen im Zitat als Text, kein Skript läuft, keine Anfrage an einen fremden Port (B0716)', async ({ browser }) => {
  const anfragen = [];
  server = http.createServer((anfrage, antwort) => {
    anfragen.push(anfrage.url);
    antwort.end();
  });
  await new Promise((bereit) => server.listen(0, '127.0.0.1', bereit));
  const fremderPort = server.address().port;
  instanz = await sendenStarten();
  const original = `<img src=x onerror="window.__x=1"> ![b](http://127.0.0.1:${fremderPort}/b.png)`;
  const anna = await annaSchreibt(instanz.adresse, [original]);
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Gesehen', anna.zeiten[0], CLAUDE);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);

  await expect(zitatVon(b.page, antwort).locator('.lc-zitat-text')).toHaveText(original);
  await expect(zitatVon(b.page, antwort).locator('img')).toHaveCount(0);
  await b.page.waitForTimeout(500);
  expect(await b.page.evaluate(() => window.__x)).toBeUndefined();
  expect(anfragen).toEqual([]);
  expect(b.fehler).toEqual([]);
});
