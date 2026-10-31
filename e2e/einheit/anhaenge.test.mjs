// Unit-Tests des Hochladens im Browser (R00030 F0101, node --test): wie eine Anmeldung ausgeht (B0450) und wie eine
// Abweisung angesagt wird.
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  anmeldungBewerten, abweisungAnsage, prozentAus, hochladenBewerten, wartetText, selbstBlockiert, selbstBlockiertGrund, eintragZustandText, zaehlmass,
} from '../../web/anhaenge.js';

const antwort = (status, daten, wartenS = null) => ({ art: 'antwort', status, daten, wartenS });

test('201 mit Anhang → angemeldet mit ID', () => {
  assert.deepEqual(anmeldungBewerten(antwort(201, { anhang: { id: 'abc' } })), { art: 'angemeldet', id: 'abc' });
});

test('429 und 507 `hochladungen` → warten nach Retry-After; 507 trägt den Stand des Platzes (UX-Review #43 F0101 S2)', () => {
  assert.deepEqual(anmeldungBewerten(antwort(429, { fehler: 'x', grenze: 'offen_kennung' }, 5)), { art: 'warten', sekunden: 5, platz: null });
  const stand = { hoechstens: 10485760, gebunden: 3670016, braucht: 3040870 };
  assert.deepEqual(
    anmeldungBewerten(antwort(507, { fehler: 'x', regel: 'quote', grund: 'hochladungen', reserviert: 6081740, frei: 733744, ...stand }, 30)),
    { art: 'warten', sekunden: 30, platz: stand },
  );
});

test('507 `voll`, 507 `platte`, 422 `zu_gross` und `name` → abgewiesen mit dem Text der Instanz', () => {
  for (const daten of [
    { fehler: 'voll', regel: 'quote', grund: 'voll' }, { fehler: 'platte', regel: 'platte' },
    { fehler: 'zu groß', regel: 'zu_gross' }, { fehler: 'name', regel: 'name' },
  ]) {
    const status = daten.regel === 'quote' || daten.regel === 'platte' ? 507 : 422;
    assert.deepEqual(anmeldungBewerten(antwort(status, daten)), { art: 'abgewiesen', grund: daten.fehler });
  }
});

test('ohne Text der Instanz der Status, ohne Antwort „ohneAntwort“', () => {
  assert.deepEqual(anmeldungBewerten(antwort(500, null)), { art: 'abgewiesen', grund: 'Nicht hochgeladen (Status 500).' });
  assert.deepEqual(anmeldungBewerten({ art: 'netzfehler' }), { art: 'ohneAntwort' });
});

test('Ansage der Abweisung: „Nicht hochgeladen: <name> — <Grund>“, das Präfix der Instanz nur einmal', () => {
  assert.equal(
    abweisungAnsage('foto.png', 'Nicht hochgeladen: der Platz für Anhänge ist voll.'),
    'Nicht hochgeladen: foto.png — der Platz für Anhänge ist voll.',
  );
  assert.equal(abweisungAnsage('a.txt', 'Der Name ist ungültig.'), 'Nicht hochgeladen: a.txt — Der Name ist ungültig.');
});

// R00030 „Fortschritt“ (B0451): die Fortschrittsrechnung, falls die Drosselung im E2E nicht greift.
test('Fortschritt in ganzen Prozent: abgerundet, ohne Länge 0, höchstens 100', () => {
  assert.equal(prozentAus(0, 2457600), 0);
  assert.equal(prozentAus(1228800, 2457600), 50);
  assert.equal(prozentAus(2457599, 2457600), 99);
  assert.equal(prozentAus(2457600, 2457600), 100);
  assert.equal(prozentAus(10, 0), 0);
  assert.equal(prozentAus(20, 10), 100);
});

test('Hochladen: 200 fertig, 422 `metadaten` abgewiesen, sonst fehlgeschlagen mit Grund; Abbruch bleibt Abbruch', () => {
  assert.deepEqual(hochladenBewerten(antwort(200, null)), { art: 'fertig' });
  assert.deepEqual(hochladenBewerten(antwort(422, { fehler: 'kaputt', regel: 'metadaten' })), { art: 'abgewiesen', grund: 'kaputt' });
  assert.deepEqual(hochladenBewerten(antwort(408, { fehler: 'Frist' })), { art: 'fehlgeschlagen', grund: 'Frist' });
  assert.deepEqual(hochladenBewerten(antwort(500, null)), { art: 'fehlgeschlagen', grund: 'Nicht hochgeladen (Status 500).' });
  assert.deepEqual(hochladenBewerten({ art: 'netzfehler' }), { art: 'fehlgeschlagen', grund: 'Keine Antwort vom Server.' });
  assert.deepEqual(hochladenBewerten({ art: 'abgebrochen' }), { art: 'abgebrochen' });
});

// R00030 „Fehlschlag und Erneut“ (B0452): nach 404 `abgelaufen` oder `unbekannt` meldet der Browser neu an.
test('Hochladen: 404 `abgelaufen` und `unbekannt` → neu anmelden, ein anderes 404 ist ein Fehlschlag', () => {
  const abgelaufen = { fehler: 'Diese Anmeldung ist abgelaufen — neu anmelden.', grund: 'abgelaufen' };
  assert.deepEqual(hochladenBewerten(antwort(404, abgelaufen)), { art: 'neuAnmelden', grund: abgelaufen.fehler });
  assert.deepEqual(hochladenBewerten(antwort(404, { fehler: 'weg', grund: 'unbekannt' })), { art: 'neuAnmelden', grund: 'weg' });
  assert.deepEqual(hochladenBewerten(antwort(404, { fehler: 'anders' })), { art: 'fehlgeschlagen', grund: 'anders' });
});

// UX-Review #43 F0101 S2: der Wartezustand zählt herunter und nennt beim 507 `hochladungen` den Platz.
test('Wartetext: „wartet · neuer Versuch in <s> s“, beim belegten Platz mit „· Platz für Anhänge belegt“', () => {
  assert.equal(wartetText(30, false), 'wartet · neuer Versuch in 30 s');
  assert.equal(wartetText(17, true), 'wartet · neuer Versuch in 17 s · Platz für Anhänge belegt');
});

// S2: belegen die eigenen Anmeldungen (Dateien dieser oder einer anderen offenen Nachricht des Tabs) den Platz so, dass
// die Datei nie passt, solange sie gehalten werden, hilft Warten nicht.
test('selbst blockiert: braucht > höchstens − gebunden − eigene Anmeldungen', () => {
  const MIB = 1024 * 1024;
  const platz = { hoechstens: 10 * MIB, gebunden: 3.5 * MIB, braucht: 2.9 * MIB };
  assert.equal(selbstBlockiert(platz, 5.8 * MIB), true);
  assert.equal(selbstBlockiert(platz, 2.9 * MIB), false);
  assert.equal(selbstBlockiert(platz, 0), false);
  assert.equal(selbstBlockiert({ ...platz, braucht: 0.7 * MIB }, 5.8 * MIB), false);
});

test('Grund „selbst blockiert“ nennt den Platz, was frei bleibt, was die Datei braucht, und „Entfernen“', () => {
  const MIB = 1024 * 1024;
  assert.equal(
    selbstBlockiertGrund({ hoechstens: 10 * MIB, gebunden: 3.5 * MIB, braucht: 2.9 * MIB }, 5.8 * MIB),
    'Nicht hochgeladen: die übrigen Dateien dieser Nachricht belegen den Platz für Anhänge — daneben bleiben 717 KiB, die Datei braucht 2,9 MiB. Jede Datei belegt mindestens 64 KiB. „Entfernen“ sendet die übrigen; diese Datei danach einzeln senden.',
  );
  assert.match(selbstBlockiertGrund({ hoechstens: MIB, gebunden: MIB, braucht: 2 * MIB }, MIB), /daneben bleiben 0 Byte,/);
});

// rechtschreib-reviewer #85 Sollte 1 (Abnahme-Probe wörtlich): die Selbstblockade ab 1 MiB widerspricht sich durch die
// Rundung nicht — sagen „daneben“ und „braucht“ in Worten dasselbe, steht die Zahl genau, in Byte.
test('Grund „selbst blockiert“ widerspricht sich durch Rundung nicht: gleich gerundete Größen werden genau in Byte genannt', () => {
  const MIB = 1024 * 1024;
  const frei = Math.round(2.86 * MIB);
  const braucht = Math.round(2.89 * MIB);
  const grund = selbstBlockiertGrund({ hoechstens: 10 * MIB, gebunden: 10 * MIB - frei, braucht }, 0);
  assert.doesNotMatch(grund, /bleiben 2,9 MiB, die Datei braucht 2,9 MiB/);
  assert.match(grund, /daneben bleiben 2\.998\.927 Byte, die Datei braucht 3\.030\.385 Byte\./);
  const kib = selbstBlockiertGrund({ hoechstens: 10 * MIB, gebunden: 10 * MIB - 102500, braucht: 102700 }, 0);
  assert.match(kib, /daneben bleiben 102\.500 Byte, die Datei braucht 102\.700 Byte\./);
  const verschieden = selbstBlockiertGrund({ hoechstens: 10 * MIB, gebunden: 10 * MIB - 100 * 1024, braucht: 3 * MIB }, 0);
  assert.match(verschieden, /daneben bleiben 100 KiB, die Datei braucht 3,0 MiB\./);
});

// UX-Review #43 F0101 S3: eine Nachricht, die an einer abgewiesenen Datei hängt, sagt nicht mehr „wird hochgeladen …“.
test('Zustand des Eintrags: „wird hochgeladen …“, mit abgewiesenen Dateien der Weg über „Entfernen“', () => {
  assert.equal(eintragZustandText(0), ' wird hochgeladen …');
  assert.equal(eintragZustandText(1), ' angehalten — nach „Entfernen“ der abgewiesenen Datei wird gesendet');
  assert.equal(eintragZustandText(2), ' angehalten — nach „Entfernen“ der abgewiesenen Dateien wird gesendet');
});

// Code-Review #43 F0119 B1: ein 409 auf den PUT nach dem Wiederverbinden ist kein Fehlschlag.
test('hochladenBewerten: 409 „fertig“ ist geschafft, 409 „laeuft“ heißt später noch einmal, anderes 409 ein Fehlschlag', () => {
  const antwort409 = (regel) => ({ art: 'antwort', status: 409, daten: { fehler: 'Text der Instanz', regel } });
  assert.deepEqual(hochladenBewerten(antwort409('fertig')), { art: 'fertig' });
  assert.deepEqual(hochladenBewerten(antwort409('laeuft')), { art: 'nochEinmal' });
  assert.deepEqual(hochladenBewerten(antwort409('sonst')), { art: 'fehlgeschlagen', grund: 'Text der Instanz' });
});

// Code-Review #43 F0119 B2: die eigenen Anmeldungen zählen im Zählmaß der Instanz (mindestens 64 KiB, auf 4 KiB aufgerundet).
test('zaehlmass: mindestens 64 KiB, sonst auf 4 KiB aufgerundet', () => {
  assert.equal(zaehlmass(1), 65536);
  assert.equal(zaehlmass(1024), 65536);
  assert.equal(zaehlmass(65536), 65536);
  assert.equal(zaehlmass(65537), 69632);
  assert.equal(zaehlmass(100 * 1024), 102400);
});

test('selbstBlockiert im Zählmaß: drei 1-KiB-Dateien belegen 196.608 Bytes und blockieren die vierte bei 200 KiB frei', () => {
  const platz = { hoechstens: 10485760, gebunden: 10280960, braucht: 65536 };
  const eigene = [1024, 1024, 1024].map(zaehlmass).reduce((summe, wert) => summe + wert, 0);
  assert.equal(eigene, 196608);
  assert.equal(selbstBlockiert(platz, eigene), true);
});

test('selbstBlockiert: Gegenproben — drei kleine Dateien bei 10.000.000 gebunden blockieren nicht; ein 100-KiB-Fall rechnet mit 102.400 je Datei', () => {
  const eigene = [1024, 1024, 1024].map(zaehlmass).reduce((summe, wert) => summe + wert, 0);
  assert.equal(selbstBlockiert({ hoechstens: 10485760, gebunden: 10000000, braucht: 65536 }, eigene), false);
  assert.equal(selbstBlockiert({ hoechstens: 10485760, gebunden: 10280960, braucht: 65536 }, zaehlmass(100 * 1024)), false);
  assert.equal(selbstBlockiert({ hoechstens: 10485760, gebunden: 10280960, braucht: 65536 }, 2 * zaehlmass(100 * 1024)), true);
  assert.equal(selbstBlockiert({ hoechstens: 10485760, gebunden: 10280960, braucht: 65536 }, 0), false);
});
