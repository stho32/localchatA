// Unit-Tests der fremden Nachrichten im offenen Verlauf (R00011 F0044; ADR-0037 Regel 6, ADR-0028 Regel 5;
// node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { einordnen, buendlerAnlegen, folgenOderZaehlen, pillenText, auslaufNach } from '../../web/live-verlauf.js';

const t = (sekunde) => `2026-10-28T12:00:${String(sekunde).padStart(2, '0')}.000+01:00`;
const n = (sekunde) => ({ zeit: t(sekunde), von: 'beta@127.0.0.1', art: 'agent', text: `n${sekunde}` });
const zeiten = (nachrichten) => nachrichten.map((nachricht) => nachricht.zeit);

// B0223 „Einordnen“: Block 8, 9, 10; vor dem Block gepuffert 9 und 11 → 11 kommt dazu; danach 10 und 12 → 12.
test('übernommen wird nur, was jünger ist als die jüngste bekannte Zeit', () => {
  const block = [n(8), n(9), n(10)];

  const nachDemBlock = einordnen(block[2].zeit, [n(9), n(11)]);
  const danach = einordnen(nachDemBlock.juengste, [n(10), n(12)]);

  assert.deepEqual(zeiten([...block, ...nachDemBlock.neue, ...danach.neue]), [8, 9, 10, 11, 12].map(t));
  assert.equal(danach.juengste, t(12));
});

test('ohne bekannte Zeit (leerer Channel) wird alles übernommen, gleiche Zeiten nur einmal', () => {
  const { neue, juengste } = einordnen(null, [n(3), n(3), n(4)]);

  assert.deepEqual(zeiten(neue), [t(3), t(4)]);
  assert.equal(juengste, t(4));
});

test('verglichen wird der Zeitpunkt, nicht der Text: 02:00+01:00 ist jünger als 02:59+02:00', () => {
  const sommer = '2026-10-25T02:59:59.999+02:00';
  const winter = { ...n(0), zeit: '2026-10-25T02:00:00.000+01:00' };

  assert.deepEqual(zeiten(einordnen(sommer, [winter]).neue), [winter.zeit]);
});

// Ein gestellter requestAnimationFrame: Rückrufe warten, bis der Test einen Frame auslöst.
function gestellterFrame() {
  let wartend = [];
  return {
    naechsterFrame: (rueckruf) => { wartend.push(rueckruf); },
    frame: () => {
      const jetzt = wartend;
      wartend = [];
      jetzt.forEach((rueckruf) => rueckruf());
    },
  };
}

// B0224 „Bündeln“: 120 Ereignisse in einem Task → genau ein Einfügen mit 120 beim nächsten Frame; ein Frame ohne
// Ereignisse → kein Einfügen.
test('je Frame wird einmal eingefügt, ohne Ereignisse gar nicht', () => {
  const { naechsterFrame, frame } = gestellterFrame();
  const eingefuegt = [];
  const buendler = buendlerAnlegen({ einfuegen: (liste) => eingefuegt.push(liste), naechsterFrame });

  for (let nummer = 0; nummer < 120; nummer += 1) {
    buendler.annehmen(n(nummer % 60));
  }
  assert.equal(eingefuegt.length, 0, 'nicht im selben Task');
  frame();
  frame();

  assert.equal(eingefuegt.length, 1);
  assert.equal(eingefuegt[0].length, 120);
});

test('nach einem Frame sammelt der nächste von vorn', () => {
  const { naechsterFrame, frame } = gestellterFrame();
  const eingefuegt = [];
  const buendler = buendlerAnlegen({ einfuegen: (liste) => eingefuegt.push(liste), naechsterFrame });

  buendler.annehmen(n(1));
  frame();
  buendler.annehmen(n(2));
  buendler.annehmen(n(3));
  frame();

  assert.deepEqual(eingefuegt.map(zeiten), [[t(1)], [t(2), t(3)]]);
});

// B0225 „Folgen oder zählen“ (Zeilenhöhe 20 px).
test('am Ende (höchstens eine Zeile Abstand) folgen, sonst anfügen und zählen, ohne neuesten Block nur zählen', () => {
  assert.equal(folgenOderZaehlen(0, 20, true), 'folgen');
  assert.equal(folgenOderZaehlen(20, 20, true), 'folgen');
  assert.equal(folgenOderZaehlen(21, 20, true), 'zaehlen');
  assert.equal(folgenOderZaehlen(0, 20, false), 'nurZaehlen');
  assert.equal(folgenOderZaehlen(5000, 20, false), 'nurZaehlen');
});

test('die Pille nennt die Zahl in Einzahl und Mehrzahl', () => {
  assert.equal(pillenText(1), '1 neue Nachricht');
  assert.equal(pillenText(3), '3 neue Nachrichten');
});

// B0329 „Auslauf baut sich mit dem Lesen ab“ (Nacharbeit #8): neuer Inhalt nimmt den Auslauf ein, ein sichtbarer
// Auslauf fällt nie auf einmal weg, einer ganz unter dem Fenster schon.
test('angefügter Inhalt nimmt den Auslauf ein, bis er aufgebraucht ist', () => {
  assert.equal(auslaufNach({ hoehe: 500, sichtbar: 480, angefuegt: 60 }), 440);
  assert.equal(auslaufNach({ hoehe: 500, sichtbar: 480, angefuegt: 500 }), 0);
  assert.equal(auslaufNach({ hoehe: 500, sichtbar: 480, angefuegt: 700 }), 0);
  assert.equal(auslaufNach({ hoehe: 500, sichtbar: 480, angefuegt: -30 }), 500);
});

test('ein sichtbarer Auslauf bleibt, ein ganz unsichtbarer fällt weg', () => {
  assert.equal(auslaufNach({ hoehe: 500, sichtbar: 480, angefuegt: 0 }), 500);
  assert.equal(auslaufNach({ hoehe: 500, sichtbar: 1, angefuegt: 0 }), 500);
  assert.equal(auslaufNach({ hoehe: 500, sichtbar: 0, angefuegt: 0 }), 0);
  assert.equal(auslaufNach({ hoehe: 0, sichtbar: 0, angefuegt: 0 }), 0);
});
