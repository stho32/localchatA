// Unit-Tests der Texte des Beitreten-Bildschirms (R00003 F0010, node --test): Meldung und Hinweis als Teile,
// Text oder { kennung } für Namen, Kennungen und Vorschläge.
import test from 'node:test';
import assert from 'node:assert/strict';
import { meldungsteile, hinweisteile } from '../../web/beitreten.js';

process.env.TZ = 'Europe/Berlin';

const REGEL = '1 bis 32 Zeichen aus a–z, 0–9 und -, nicht mit - am Anfang oder Ende';

function alsText(teile) {
  return teile.map((teil) => {
    if (typeof teil === 'string') {
      return teil;
    }
    return teil.kennung;
  }).join('');
}

function kennungen(teile) {
  return teile.filter((teil) => typeof teil !== 'string').map((teil) => teil.kennung);
}

test('Name vergeben nennt Name, Träger, Uhrzeit HH:MM in Ortszeit und den Vorschlag, alle drei als Kennung', () => {
  const teile = meldungsteile({
    art: 'vergeben', name: 'shoff', belegtVon: 'shoff@127.0.0.1', seit: '2026-10-27T08:04:00.000+00:00', vorschlag: 'shoff-2',
  });

  assert.equal(alsText(teile), 'Der Name shoff ist vergeben — shoff@127.0.0.1 ist seit 09:04 dabei. Wie wäre shoff-2?');
  assert.deepEqual(kennungen(teile), ['shoff', 'shoff@127.0.0.1', 'shoff-2']);
});

test('ungültig: ⚠, Name, Regel des Servers und Vorschlag — dieselbe Form wie „vergeben“', () => {
  const teile = meldungsteile({ art: 'ungueltig', name: 'Jörg', fehler: 'f', regel: REGEL, vorschlag: 'joerg' });

  assert.equal(alsText(teile), `⚠ Der Name Jörg ist nicht erlaubt — ${REGEL}. Wie wäre joerg?`);
  assert.deepEqual(kennungen(teile), ['Jörg', 'joerg']);
});

test('ungültig ohne Vorschlag endet nach der Regel', () => {
  const teile = meldungsteile({ art: 'ungueltig', name: '___', fehler: 'f', regel: REGEL, vorschlag: '' });

  assert.equal(alsText(teile), `⚠ Der Name ___ ist nicht erlaubt — ${REGEL}.`);
});

test('leer, Server weg und sonstige Fehler mit ⚠ und Wort', () => {
  assert.equal(alsText(meldungsteile({ art: 'leer' })), '⚠ Bitte einen Namen eingeben.');
  assert.equal(alsText(meldungsteile({ art: 'serverWeg', name: 'shoff' })), '⚠ Server nicht erreichbar.');
  assert.equal(alsText(meldungsteile({ art: 'fehler', name: 'n', fehler: 'Zu viele Sitzungen' })), '⚠ Zu viele Sitzungen');
});

test('ohne Meldung: leerer Bildschirm', () => {
  assert.deepEqual(meldungsteile({ art: 'beitreten', name: '' }), []);
});

test('Hinweis beim Tippen: leer bei gültigem, leerem oder nur aus Leerzeichen bestehendem Text, sonst Regel und Vorschlag', () => {
  assert.deepEqual(hinweisteile('claude-2'), []);
  assert.deepEqual(hinweisteile(''), []);
  assert.deepEqual(hinweisteile('   '), []);
  assert.equal(alsText(hinweisteile('Jörg')), `Erlaubt sind ${REGEL}. Wie wäre joerg?`);
  assert.deepEqual(kennungen(hinweisteile('Jörg')), ['joerg']);
  assert.equal(alsText(hinweisteile('___')), `Erlaubt sind ${REGEL}.`);
});
