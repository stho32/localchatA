// Unit-Tests zum Anlegen im Browser (R00006 F0025, B0137, B0139; ADR-0024 Regeln 5 und 7; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { anlegeAntwort, kuerzelOeffnet } from '../../web/channel-anlegen.js';

function taste({ ctrlKey = false, altKey = false, key, altGraph = false }) {
  return { ctrlKey, altKey, key, getModifierState: (name) => name === 'AltGraph' && altGraph };
}

test('Strg Alt N öffnet, auch mit Umschalt (N)', () => {
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, altKey: true, key: 'n' })), true);
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, altKey: true, key: 'N' })), true);
});

test('AltGr N öffnet nicht — auf deutschen Tastaturen ist AltGr Strg Alt und tippt ein Zeichen', () => {
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, altKey: true, key: 'n', altGraph: true })), false);
});

test('ohne Alt, ohne Strg oder mit anderer Taste öffnet nichts', () => {
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, key: 'n' })), false);
  assert.equal(kuerzelOeffnet(taste({ altKey: true, key: 'n' })), false);
  assert.equal(kuerzelOeffnet(taste({ ctrlKey: true, altKey: true, key: 'm' })), false);
});

const allgemein = { name: 'allgemein', angelegt: '2026-10-28T10:15:00.123+01:00', angelegt_von: 'alpha@127.0.0.2', beschreibung: null };
const antwort = (status, daten) => ({ serverWeg: false, status, daten });

test('201 → angelegt mit der Statuszeile', () => {
  assert.deepEqual(anlegeAntwort(antwort(201, { channel: allgemein })), {
    art: 'angelegt', text: 'Channel # allgemein angelegt.', vorschlag: '',
  });
});

test('409 mit channel → „# allgemein gibt es schon.“; 409 ordner-vorhanden → der Text des Servers', () => {
  assert.deepEqual(anlegeAntwort(antwort(409, { fehler: 'x', channel: allgemein })), {
    art: 'vorhanden', text: '# allgemein gibt es schon.', vorschlag: '',
  });
  const ordner = 'channels/notizen besteht im Ordner schon; localchatA übernimmt ihn beim nächsten Start.';
  assert.deepEqual(anlegeAntwort(antwort(409, { fehler: ordner, regel: 'ordner-vorhanden' })), {
    art: 'vorhanden', text: ordner, vorschlag: '',
  });
});

test('422 mit Vorschlag → Regel und Vorschlag; 422 reserviert → Regel ohne Vorschlag', () => {
  const regel = 'Ein Channelname hat 1 bis 32 Zeichen aus a–z, 0–9 und -, nicht mit - am Anfang oder Ende.';
  assert.deepEqual(anlegeAntwort(antwort(422, { fehler: regel, regel: 'x', vorschlag: 'allgemein' })), {
    art: 'ungueltig', text: regel, vorschlag: 'allgemein',
  });
  const reserviert = anlegeAntwort(antwort(422, { fehler: 'Gerätename', regel: 'reserviert' }));
  assert.deepEqual(reserviert, { art: 'ungueltig', text: 'Gerätename', vorschlag: '' });
});

test('429 → beginnt mit „Zu viele Channels“ und nennt die Grenze aus hoechstens', () => {
  for (const [grenze, hoechstens] of [['herkunft', 32], ['gesamt', 256]]) {
    const gedeutet = anlegeAntwort(antwort(429, { fehler: 'x', grenze, hoechstens }));
    assert.equal(gedeutet.art, 'grenze');
    assert.match(gedeutet.text, /^Zu viele Channels/);
    assert.ok(gedeutet.text.includes(String(hoechstens)), gedeutet.text);
  }
});

test('Server weg → „Nicht angelegt — Verbindung prüfen.“; andere Status mit dem Text des Servers', () => {
  assert.deepEqual(anlegeAntwort({ serverWeg: true, status: 0, daten: null }), {
    art: 'netzfehler', text: 'Nicht angelegt — Verbindung prüfen.', vorschlag: '',
  });
  assert.deepEqual(anlegeAntwort(antwort(500, { fehler: 'Platte voll' })), {
    art: 'fehler', text: 'Platte voll', vorschlag: '',
  });
  assert.deepEqual(anlegeAntwort(antwort(502, null)), {
    art: 'fehler', text: 'Nicht angelegt (Status 502).', vorschlag: '',
  });
});

test('422 mit regel beschreibung betrifft das Feld Beschreibung (R00007 F0026)', () => {
  const gedeutet = anlegeAntwort({ serverWeg: false, status: 422, daten: { fehler: 'Ein Satz aus 1 bis 200 Zeichen.', regel: 'beschreibung' } });

  assert.deepEqual(gedeutet, { art: 'beschreibung', text: 'Ein Satz aus 1 bis 200 Zeichen.', vorschlag: '' });
});
