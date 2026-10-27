// Unit-Tests der Texte des Beitreten-Bildschirms (R00003 F0010, node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { meldungFuer, hinweisFuer } from '../../web/beitreten.js';

process.env.TZ = 'Europe/Berlin';

test('Name vergeben nennt Name, Träger, Uhrzeit HH:MM in Ortszeit und den Vorschlag', () => {
  const text = meldungFuer({
    art: 'vergeben', name: 'shoff', belegtVon: 'shoff@127.0.0.1', seit: '2026-10-27T08:04:00.000+00:00', vorschlag: 'shoff-2',
  });

  assert.equal(text, 'Der Name shoff ist vergeben — shoff@127.0.0.1 ist seit 09:04 dabei. Wie wäre shoff-2?');
});

test('Server weg mit ⚠ und Wort, sonstige Fehler mit ⚠ und dem Text des Servers', () => {
  assert.equal(meldungFuer({ art: 'serverWeg', name: 'shoff' }), '⚠ Server nicht erreichbar.');
  assert.equal(meldungFuer({ art: 'fehler', name: 'n', fehler: 'Zu viele Sitzungen' }), '⚠ Zu viele Sitzungen');
});

test('ohne Meldung: leerer Bildschirm und Regelverstoß', () => {
  assert.equal(meldungFuer({ art: 'beitreten', name: '' }), '');
  assert.equal(meldungFuer({ art: 'ungueltig', name: 'Jörg', fehler: 'f', vorschlag: 'joerg' }), '');
});

test('Hinweis beim Tippen: leer bei gültigem oder leerem Text, sonst Regel und Vorschlag', () => {
  assert.equal(hinweisFuer('claude-2'), '');
  assert.equal(hinweisFuer(''), '');
  assert.match(hinweisFuer('Jörg'), /a–z, 0–9 und -.*Vorschlag: joerg/);
  assert.doesNotMatch(hinweisFuer('___'), /Vorschlag/);
});
