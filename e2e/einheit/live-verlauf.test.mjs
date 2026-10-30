// Unit-Tests der fremden Nachrichten im offenen Verlauf (R00011 F0044; ADR-0037 Regel 6, ADR-0028 Regel 5;
// node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  einordnen, zusammenfuehren, buendlerAnlegen, folgenOderZaehlen, pillenText, auslaufNach, nichtGeladenText, hinweisTextNachziehen, sprungZiel, scrollTopNachHoehe,
  zeitDavor,
} from '../../web/live-verlauf.js';

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

// B0390 „Zusammenführen“ (R00012 F0047, Nacharbeit #22; ADR-0037 Ergänzung Karte #22/#36, 7f): bekannt 1–3, live
// angenommen 6, nachgeholter Block 4, 5 → 1–6 je einmal. Die Mutante „nur Zeiten größer als die jüngste“ (einordnen)
// verlöre 4 und 5.
test('Lücke und Live-Strom: der nachgeholte Block kommt vor eine schon gezeigte jüngere Nachricht', () => {
  const bekannt = [n(1), n(2), n(3), n(6)];

  const { folge, neue } = zusammenfuehren(bekannt, [n(4), n(5)]);

  assert.deepEqual(zeiten(folge), [1, 2, 3, 4, 5, 6].map(t));
  assert.deepEqual(zeiten(neue), [4, 5].map(t));
});

test('enthält der Block die live angenommene 6 schon, steht sie einmal da', () => {
  const { folge, neue } = zusammenfuehren([n(1), n(2), n(3), n(6)], [n(4), n(5), n(6)]);

  assert.deepEqual(zeiten(folge), [1, 2, 3, 4, 5, 6].map(t));
  assert.deepEqual(zeiten(neue), [4, 5].map(t));
});

test('ein leerer Block lässt die Folge unverändert', () => {
  const bekannt = [n(1), n(2), n(3), n(6)];

  const { folge, neue } = zusammenfuehren(bekannt, []);

  assert.deepEqual(zeiten(folge), zeiten(bekannt));
  assert.deepEqual(neue, []);
});

test('gleiche Zeiten im Block nur einmal; ohne Bekanntes (leerer Channel) die Folge nach Zeit', () => {
  const { folge, neue } = zusammenfuehren([], [n(5), n(4), n(5)]);

  assert.deepEqual(zeiten(folge), [4, 5].map(t));
  assert.deepEqual(zeiten(neue), [4, 5].map(t));
});

test('zusammengeführt wird nach dem Zeitpunkt, nicht nach dem Text: 02:00+01:00 nach 02:59+02:00', () => {
  const sommer = { ...n(0), zeit: '2026-10-25T02:59:59.999+02:00' };
  const winter = { ...n(1), zeit: '2026-10-25T02:00:00.000+01:00' };

  assert.deepEqual(zeiten(zusammenfuehren([winter], [sommer]).folge), [sommer.zeit, winter.zeit]);
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

// ---- Karte #65 (R00037 Regel 1): der Hinweis „Verlauf nicht geladen“ sagt, ob etwas von selbst lädt --------------------

// Mutante „alter Wortlaut“ (…  — Verbindung prüfen.) → rot.
test('Hinweistext je Zustand: selbstversuchend „wird geladen …“, sonst „Channel erneut wählen …“', () => {
  assert.equal(nichtGeladenText(true), '⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.');
  assert.equal(nichtGeladenText(false), '⚠ Verlauf nicht geladen — Channel erneut wählen, um es noch einmal zu versuchen.');
});

// Ein Knoten, der jeden Schreibzugriff zählt (ein Textwechsel im role="alert"-Knoten würde angesagt).
function alarmKnoten(text) {
  const knoten = { schreibzugriffe: 0, inhalt: text, role: 'alert' };
  Object.defineProperty(knoten, 'textContent', {
    get: () => knoten.inhalt,
    set: (neu) => { knoten.inhalt = neu; knoten.schreibzugriffe += 1; },
  });
  return knoten;
}

// Mutante „Hinweis nicht nachgezogen“ (der Text bleibt, wie er war) → rot.
test('Hinweis nachziehen: ein anderer Zustand tauscht nur den Text desselben Knotens, role="alert" bleibt', () => {
  const knoten = alarmKnoten(nichtGeladenText(false));

  const nachTrennung = hinweisTextNachziehen(knoten, true);

  assert.equal(nachTrennung, knoten);
  assert.equal(knoten.textContent, '⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.');
  assert.equal(knoten.role, 'alert');
  assert.equal(knoten.schreibzugriffe, 1);
  hinweisTextNachziehen(knoten, false);
  assert.equal(knoten.textContent, '⚠ Verlauf nicht geladen — Channel erneut wählen, um es noch einmal zu versuchen.');
});

test('Hinweis nachziehen: gleicher Text → kein Schreibzugriff (nichts wird erneut angesagt)', () => {
  const knoten = alarmKnoten(nichtGeladenText(true));

  hinweisTextNachziehen(knoten, true);
  hinweisTextNachziehen(knoten, true);

  assert.equal(knoten.schreibzugriffe, 0);
});

// B0578 „Ziel des Sprungs“ (R00026 F0089, Rechenbeispiele wörtlich): Scrollstand 1.000, Platz der ersten neuen beginnt
// 400 px unter dem oberen Rand, klebender Trenner endet bei 20 px, Zeilenhöhe 20 px. Alle Maße in px vom oberen Rand
// des Verlaufs. Mutante „Zeilenhöhe im Rückfall nicht abgezogen“ → 1.380 statt 1.360 → rot.
const lage = { scrollTop: 1000, oberkanteErste: 400, unterkanteKontext: null, trennerUnterkante: 20, zeilenhoehe: 20 };

test('Sprungziel ohne Nachricht davor: Oberkante des Platzes, eine Zeilenhöhe unter den klebenden Trenner', () => {
  assert.equal(sprungZiel(lage), 1360);
});

test('Sprungziel ohne Nachricht davor und ohne klebenden Trenner: eine Zeilenhöhe unter den oberen Rand', () => {
  assert.equal(sprungZiel({ ...lage, trennerUnterkante: 0 }), 1380);
});

test('Sprungziel mit Nachricht davor: deren Unterkante kommt eine Zeilenhöhe unter den klebenden Trenner', () => {
  assert.equal(sprungZiel({ ...lage, unterkanteKontext: 380 }), 1340);
});

// B0723 „Lesestelle nach Höhenänderung“ (R00048): der Abstand der Lesestelle zur Unterkante des Verlaufs bleibt.
test('schrumpft die Sicht von 584 auf 548, rückt scrollTop um 36 vor, und zurück um 36', () => {
  const inhaltsHoehe = 3000;

  const kleiner = scrollTopNachHoehe({ scrollTop: 1000, hoeheVorher: 584, hoeheNachher: 548, inhaltsHoehe });
  const zurueck = scrollTopNachHoehe({ scrollTop: kleiner, hoeheVorher: 548, hoeheNachher: 584, inhaltsHoehe });

  assert.equal(kleiner, 1036);
  assert.equal(zurueck, 1000);
});

test('gleiche Sichthöhe lässt scrollTop unverändert', () => {
  assert.equal(scrollTopNachHoehe({ scrollTop: 1000, hoeheVorher: 584, hoeheNachher: 584, inhaltsHoehe: 3000 }), 1000);
});

test('das Ergebnis liegt nie unter 0 und nie über Inhalt minus Sichthöhe', () => {
  const zuWeitOben = scrollTopNachHoehe({ scrollTop: 10, hoeheVorher: 400, hoeheNachher: 600, inhaltsHoehe: 3000 });
  const zuWeitUnten = scrollTopNachHoehe({ scrollTop: 2440, hoeheVorher: 560, hoeheNachher: 500, inhaltsHoehe: 3000 });
  const inhaltKuerzerAlsSicht = scrollTopNachHoehe({ scrollTop: 0, hoeheVorher: 600, hoeheNachher: 500, inhaltsHoehe: 300 });

  assert.equal(zuWeitOben, 0);
  assert.equal(zuWeitUnten, 2500);
  assert.equal(inhaltKuerzerAlsSicht, 0);
});

// Der Sprung zum Original (R00047 B0718, ADR-0053 Regel 8 b): der Abruf beginnt eine Millisekunde vor seiner Zeit — `nach`
// ist exklusiv, Zeiten sind Millisekunden und eindeutig, also steht das Original vorn im Block.
test('zeitDavor: eine Millisekunde vor der Zeit, als RFC 3339 in UTC, auch über eine Sekunden- und Tagesgrenze', () => {
  assert.equal(zeitDavor('2026-10-28T14:03:00.123+01:00'), '2026-10-28T13:03:00.122+00:00');
  assert.equal(zeitDavor('2026-10-28T00:00:00.000+01:00'), '2026-10-27T22:59:59.999+00:00');
  assert.equal(Date.parse(zeitDavor('2026-10-28T14:03:00.123+01:00')), Date.parse('2026-10-28T14:03:00.123+01:00') - 1);
});
