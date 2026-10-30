// Unit-Tests der gebündelten Ansage (R00012 F0049 „Bündeln“, B0251; ADR-0038 Regel 10; node --test, gestellte Zeit).
import test from 'node:test';
import assert from 'node:assert/strict';
import { ANSAGE_LEER, ansage, ansageWartet, ansageSchreiberAnlegen } from '../../web/ansage.js';

const OFFEN = 'allgemein';
const fremd = (von = 'claude@10.0.0.5', channel = OFFEN) => ({ art: 'fremd', von, channel, offenerChannel: OFFEN });
const TAKT = { art: 'takt' };

// Wendet [ms, ereignis] nacheinander an; liefert je Schritt [ms, text].
function ablauf(schritte) {
  let zustand = ANSAGE_LEER;
  return schritte.map(([ms, ereignis]) => {
    const ergebnis = ansage(zustand, ereignis, ms);
    zustand = ergebnis.zustand;
    return [ms, ergebnis.text];
  });
}

test('die erste nach Ruhe sofort, weitere gesammelt frühestens 10 s nach der letzten Ansage (Mutanten 9 s und 11 s werden rot)', () => {
  const texte = ablauf([
    [0, fremd()],
    [1000, fremd()], [2000, fremd()], [3000, fremd()],
    [9999, TAKT],
    [10000, TAKT],
    [25000, fremd()],
    [26000, fremd()],
    [34999, TAKT],
    [35000, TAKT],
    [36000, { art: 'fehlschlag', grund: 'Keine Antwort vom Server.' }],
  ]);

  assert.deepEqual(texte, [
    [0, 'Neue Nachricht von claude@10.0.0.5 in # allgemein'],
    [1000, null], [2000, null], [3000, null],
    [9999, null],
    [10000, '3 neue Nachrichten in # allgemein'],
    [25000, 'Neue Nachricht von claude@10.0.0.5 in # allgemein'],
    [26000, null],
    [34999, null],
    [35000, '1 neue Nachricht in # allgemein'],
    [36000, 'Nachricht nicht gesendet: Keine Antwort vom Server.'],
  ]);
});

test('eigene, nachgeladene und nachgeholte Nachrichten und andere Channels werden nie angesagt', () => {
  const texte = ablauf([
    [0, { art: 'eigen' }],
    [1, { art: 'nachgeholt' }],
    [2, fremd('beta@127.0.0.1', 'technik')],
    [20000, TAKT],
  ]);

  assert.deepEqual(texte.map(([, text]) => text), [null, null, null, null]);
});

test('ein Fehlschlag wird sofort angesagt, auch direkt nach einer Ansage, und verschiebt die Bündelung nicht', () => {
  const texte = ablauf([
    [0, fremd()],
    [500, { art: 'fehlschlag', grund: 'Nicht gespeichert — der Server konnte nicht schreiben.' }],
    [1000, fremd()],
    [10000, TAKT],
  ]);

  assert.deepEqual(texte.map(([, text]) => text), [
    'Neue Nachricht von claude@10.0.0.5 in # allgemein',
    'Nachricht nicht gesendet: Nicht gespeichert — der Server konnte nicht schreiben.',
    null,
    '1 neue Nachricht in # allgemein',
  ]);
});

test('eine fremde Nachricht je Sekunde über 60 s ergibt höchstens 7 Ansagen', () => {
  const schritte = [];
  for (let ms = 0; ms <= 60000; ms += 100) {
    if (ms % 1000 === 0) {
      schritte.push([ms, fremd()]);
    }
    schritte.push([ms, TAKT]);
  }

  const ansagen = ablauf(schritte).filter(([, text]) => text !== null);

  assert.ok(ansagen.length <= 7, JSON.stringify(ansagen));
  assert.ok(ansagen.length >= 6, JSON.stringify(ansagen));
});

test('ein Channel-Wechsel verwirft Gesammeltes', () => {
  const texte = ablauf([[0, fremd()], [1000, fremd()], [2000, { art: 'gewechselt' }], [10000, TAKT]]);

  assert.deepEqual(texte.map(([, text]) => text), ['Neue Nachricht von claude@10.0.0.5 in # allgemein', null, null, null]);
});

// R00025 F0084 „Zwei Sofort-Ansagen“ (B0488; UX-Vorprüfung #36 V10, wörtlich: „Zwei Sofort-Ansagen im selben Frame →
// beide Texte erreichen die Region (in Folge oder zusammengezogen); Mutante ‚letzte gewinnt‘ rot.“): zusammengezogen in
// der Reihenfolge des Eintreffens, mit „. “ verbunden, gleiche Texte einmal (ADR-0038 Ergänzung „Karte #36
// UX-Vorprüfung“ Punkt 3).
test('Sofort-Ansagen im selben Bild erreichen die Region zusammengezogen, gleiche einmal', () => {
  const geschrieben = [];
  let bild = null;
  const sagen = ansageSchreiberAnlegen({
    schreiben: (text) => geschrieben.push(text),
    imNaechstenBild: (rueckruf) => {
      bild = rueckruf;
    },
  });

  sagen('Frist auf 3 Tage gesetzt');
  sagen('Verbindung verloren');
  sagen('Frist auf 3 Tage gesetzt');
  bild();
  sagen('Wieder verbunden');
  bild();

  assert.deepEqual(geschrieben, ['', 'Frist auf 3 Tage gesetzt. Verbindung verloren', '', 'Wieder verbunden']);
});

// R00033 F0124 B0552: nach einem Satzzeichen am Ende folgt nur ein Leerzeichen, nie „..“ (rechtschreib-reviewer #36).
test('zusammengezogene Ansagen tragen keinen doppelten Punkt', () => {
  const zusammengezogen = (...texte) => {
    const geschrieben = [];
    let bild = null;
    const sagen = ansageSchreiberAnlegen({
      schreiben: (text) => geschrieben.push(text),
      imNaechstenBild: (rueckruf) => {
        bild = rueckruf;
      },
    });
    texte.forEach(sagen);
    bild();
    return geschrieben[1];
  };

  assert.equal(zusammengezogen('a.', 'b'), 'a. b');
  assert.equal(zusammengezogen('a', 'b'), 'a. b');
  assert.equal(zusammengezogen('a…', 'b'), 'a… b');
  assert.equal(zusammengezogen('a.', 'b.'), 'a. b.');
  assert.equal(zusammengezogen('a!', 'b?', 'c'), 'a! b? c');
  assert.equal(zusammengezogen('a', 'a'), 'a');
});

test('eine Sofort-Ansage mit Text geht unverändert und außerhalb der Bündelung hinaus', () => {
  const gesammelt = ansage(ANSAGE_LEER, fremd(), 0).zustand;
  const zweite = ansage(gesammelt, fremd(), 1000).zustand;

  const ergebnis = ansage(zweite, { art: 'sofort', text: 'Steuerzeichen U+001B an Stelle 5 — bitte entfernen.' }, 2000);

  assert.equal(ergebnis.text, 'Steuerzeichen U+001B an Stelle 5 — bitte entfernen.');
  assert.deepEqual(ergebnis.zustand, zweite);
});

// R00025 F0084 „Ansage unabhängig von der Bündelung“ (B0399): t = 0 fremd → sofort; t = 2 s fremd (gesammelt); t = 3 s
// wieder verbunden mit 0 → sofort „Wieder verbunden“; t = 10 s → „1 neue Nachricht in # allgemein“ — der Takt der
// gesammelten bleibt. Zwei Meldungen `unterbrochen` hintereinander → genau eine Ansage „Verbindung verloren“ (Mutanten
// „durch die Bündelung“ und „je Versuch angesagt“ werden je rot).
test('Trennen und Wiederverbinden sofort, außerhalb der Bündelung, „Verbindung verloren“ einmal je Trennung', () => {
  const texte = ablauf([
    [0, fremd()],
    [2000, fremd()],
    [2500, { art: 'getrennt' }],
    [2600, { art: 'getrennt' }],
    [3000, { art: 'wiederverbunden', text: 'Wieder verbunden' }],
    [9999, TAKT],
    [10000, TAKT],
    [11000, { art: 'getrennt' }],
    [12000, { art: 'wiederverbunden', text: 'Wieder verbunden · 3 nachgeladen' }],
  ]);

  assert.deepEqual(texte, [
    [0, 'Neue Nachricht von claude@10.0.0.5 in # allgemein'],
    [2000, null],
    [2500, 'Verbindung verloren'],
    [2600, null],
    [3000, 'Wieder verbunden'],
    [9999, null],
    [10000, '1 neue Nachricht in # allgemein'],
    [11000, 'Verbindung verloren'],
    [12000, 'Wieder verbunden · 3 nachgeladen'],
  ]);
});

test('im offenen Direktgespräch nennt die Ansage das Gespräch ohne „#“ (R00050 B0787)', () => {
  const offen = 'direkt:ben@127.0.0.3';
  const fremd = { art: 'fremd', von: 'ben@127.0.0.3', channel: offen, offenerChannel: offen };

  const erste = ansage(ANSAGE_LEER, fremd, 0);
  const zweite = ansage(erste.zustand, fremd, 1000);
  const gesammelt = ansage(zweite.zustand, { art: 'takt' }, 10000);

  // Seit F0180 (V11, US-26; ux-reviewer #13 F0178 Optional 4 „redundant“) ohne die Kennung zweimal.
  assert.equal(erste.text, 'Neue Direktnachricht von ben@127.0.0.3');
  assert.equal(gesammelt.text, '1 neue Nachricht in Direktnachricht mit ben@127.0.0.3');
});

// ---- R00050 F0180 B0790 (V11): Direktnachrichten ------------------------------------------------------------------

const BEN_DIREKT = 'direkt:ben@127.0.0.3';
const direktVon = (von, offenerChannel = OFFEN) => ({ art: 'fremd', von, channel: `direkt:${von}`, offenerChannel });

test('eine fremde Direktnachricht in einem nicht offenen Gespräch: „Neue Direktnachricht von <kennung>“, ohne „#“', () => {
  const texte = ablauf([[0, direktVon('ben@127.0.0.3')]]);
  assert.deepEqual(texte, [[0, 'Neue Direktnachricht von ben@127.0.0.3']]);
});

test('im offenen Direktgespräch dieselbe Ansage — ohne „#“ und ohne die Kennung doppelt; gesammelt „… in Direktnachricht mit …“', () => {
  const texte = ablauf([
    [0, direktVon('ben@127.0.0.3', BEN_DIREKT)],
    [1000, direktVon('ben@127.0.0.3', BEN_DIREKT)], [2000, direktVon('ben@127.0.0.3', BEN_DIREKT)],
    [10000, TAKT],
  ]);
  assert.deepEqual(texte, [
    [0, 'Neue Direktnachricht von ben@127.0.0.3'],
    [1000, null], [2000, null],
    [10000, '2 neue Nachrichten in Direktnachricht mit ben@127.0.0.3'],
  ]);
  assert.ok(texte.every(([, text]) => text === null || !text.includes('#')));
});

test('Takt: kurz nach einer Ansage gesammelt, je Absender einmal, mit dem Takt zusammen mit den Nachrichten des offenen Channels', () => {
  const texte = ablauf([
    [0, fremd()],
    [1000, direktVon('ben@127.0.0.3')], [2000, direktVon('ben@127.0.0.3')], [3000, direktVon('cora@127.0.0.9')],
    [4000, fremd()],
    [9999, TAKT],
    [10000, TAKT],
    [10001, TAKT],
  ]);
  assert.deepEqual(texte, [
    [0, 'Neue Nachricht von claude@10.0.0.5 in # allgemein'],
    [1000, null], [2000, null], [3000, null], [4000, null],
    [9999, null],
    [10000, '1 neue Nachricht in # allgemein. Neue Direktnachricht von ben@127.0.0.3. Neue Direktnachricht von cora@127.0.0.9'],
    [10001, null],
  ]);
});

test('eine Direktnachricht in Ruhe nach einer gesammelten wartet auf den Takt — sie überholt nichts; ansageWartet sagt, ob der Takt nötig ist', () => {
  let zustand = ANSAGE_LEER;
  assert.equal(ansageWartet(zustand), false);
  zustand = ansage(zustand, direktVon('ben@127.0.0.3'), 0).zustand;
  assert.equal(ansageWartet(zustand), false);
  const gesammelt = ansage(zustand, direktVon('cora@127.0.0.9'), 5000);
  assert.equal(gesammelt.text, null);
  assert.equal(ansageWartet(gesammelt.zustand), true);
  const nachDemWechsel = ansage(gesammelt.zustand, { art: 'gewechselt' }, 6000);
  assert.equal(ansageWartet(nachDemWechsel.zustand), true, 'ein Wechsel verwirft keine Direktnachricht eines anderen Gesprächs');
  assert.equal(ansage(nachDemWechsel.zustand, TAKT, 10000).text, 'Neue Direktnachricht von cora@127.0.0.9');
});

test('eine fremde Nachricht in einem anderen, nicht offenen Channel bleibt still (Kontrolle zu V11)', () => {
  assert.deepEqual(ablauf([[0, fremd('claude@10.0.0.5', 'technik')]]), [[0, null]]);
});
