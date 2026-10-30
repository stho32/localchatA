// Unit-Tests der Direktnachrichten in der Seitenleiste (R00050 F0180 B0786, V8; node --test): welche Gespräche, in welcher
// Reihenfolge, und was jeder Eintrag über das Gegenüber sagt.
import test from 'node:test';
import assert from 'node:assert/strict';
import { direktlisteZusammenfuehren, direkteintrag } from '../../web/direktliste.js';

test('alphabetisch nach Kennung, jede einmal — die Reihenfolge des Eintreffens (Aktivität) zählt nicht', () => {
  const ausDerAntwort = direktlisteZusammenfuehren(null, ['cora@127.0.0.1', 'ben@127.0.0.3']);
  assert.deepEqual(ausDerAntwort, ['ben@127.0.0.3', 'cora@127.0.0.1']);
  const neueste = direktlisteZusammenfuehren(ausDerAntwort, ['ben@127.0.0.3', 'anton@127.0.0.9', 'cora@127.0.0.1']);
  assert.deepEqual(neueste, ['anton@127.0.0.9', 'ben@127.0.0.3', 'cora@127.0.0.1']);
});

test('gleicher Name, verschiedene Herkunft: je ein Eintrag, geordnet wie die Teilnehmerliste (natürlich, Zahlen als Zahlen)', () => {
  const liste = direktlisteZusammenfuehren(null, ['ben@127.0.0.10', 'ben@127.0.0.9', 'ben@::1']);
  assert.deepEqual(liste, ['ben@::1', 'ben@127.0.0.9', 'ben@127.0.0.10']);
});

test('nichts Neues → dieselbe Liste (nichts wird neu gezeichnet), leere Antwort → leere Liste statt unbekannt', () => {
  const liste = ['ben@127.0.0.3'];
  assert.equal(direktlisteZusammenfuehren(liste, ['ben@127.0.0.3']), liste);
  assert.deepEqual(direktlisteZusammenfuehren(null, []), []);
});

const UHR = { versatz: 0, jetzt: Date.parse('2026-10-28T15:00:00+01:00'), standUnbekannt: false };
const anwesend = (status, wegUm = null) => [{ kennung: 'ben@127.0.0.3', name: 'ben', status, weg_um: wegUm }];

test('Eintrag: Statuspunkt wie in der Teilnehmerliste, offline und „gleich weg“ zusätzlich als Wort, Name in ganzen Worten', () => {
  assert.deepEqual(direkteintrag('ben@127.0.0.3', anwesend('online'), UHR), {
    kennung: 'ben@127.0.0.3', kuerzel: 'be', punkt: 'on', wort: null, name: 'Direktnachricht mit ben@127.0.0.3, online',
  });
  assert.deepEqual(direkteintrag('ben@127.0.0.3', anwesend('offline'), UHR), {
    kennung: 'ben@127.0.0.3', kuerzel: 'be', punkt: 'off', wort: 'offline', name: 'Direktnachricht mit ben@127.0.0.3, offline',
  });
  assert.deepEqual(direkteintrag('ben@127.0.0.3', anwesend('offline', '2026-10-28T15:00:30+01:00'), UHR), {
    kennung: 'ben@127.0.0.3', kuerzel: 'be', punkt: 'gleich', wort: 'gleich weg',
    name: 'Direktnachricht mit ben@127.0.0.3, offline, gleich weg',
  });
});

test('Eintrag eines Gegenübers, das nicht mehr in der Liste steht: „weg“; ohne Verbindung „zuletzt online“', () => {
  assert.deepEqual(direkteintrag('claude-2@127.0.0.9', [], UHR), {
    kennung: 'claude-2@127.0.0.9', kuerzel: 'c2', punkt: 'off', wort: 'weg', name: 'Direktnachricht mit claude-2@127.0.0.9, weg',
  });
  const ohneVerbindung = direkteintrag('ben@127.0.0.3', anwesend('online'), { ...UHR, standUnbekannt: true });
  assert.equal(ohneVerbindung.name, 'Direktnachricht mit ben@127.0.0.3, zuletzt online');
  assert.equal(ohneVerbindung.punkt, 'on');
});
