// Unit-Tests der gebündelten Ansage (R00012 F0049 „Bündeln“, B0251; ADR-0038 Regel 10; node --test, gestellte Zeit).
import test from 'node:test';
import assert from 'node:assert/strict';
import { ANSAGE_LEER, ansage } from '../../web/ansage.js';

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
