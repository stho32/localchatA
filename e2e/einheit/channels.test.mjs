// Unit-Tests der Channel-Liste in der Oberfläche (R00006 F0024, B0133, ADR-0023 Regel 6; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { LEER, channelsZusammenfuehren } from '../../web/channels.js';

const channel = (name) => ({ name, angelegt: null, angelegt_von: null, beschreibung: null });
const namen = (liste) => liste.channels.map((eintrag) => eintrag.name);
const stand = (nummer, ...liste) => ({ art: 'channels', stand: nummer, liste: liste.map(channel) });
const wechsel = (nummer, name) => ({ art: 'channel', stand: nummer, channel: channel(name) });

test('ein Stand ersetzt die Liste, auch die leere', () => {
  const liste = channelsZusammenfuehren(LEER, stand(5, 'a', 'c'));

  assert.deepEqual(namen(liste), ['a', 'c']);
  assert.equal(liste.stand, 5);
});

test('ein Wechsel mit kleinerem oder gleichem stand ändert nichts', () => {
  const liste = channelsZusammenfuehren(LEER, stand(5, 'a', 'c'));

  assert.equal(channelsZusammenfuehren(liste, wechsel(4, 'b')), liste);
  assert.equal(channelsZusammenfuehren(liste, wechsel(5, 'b')), liste);
});

test('ein jüngerer Wechsel fügt nach Name ein, ein weiterer ersetzt statt zu verdoppeln', () => {
  const liste = channelsZusammenfuehren(LEER, stand(5, 'a', 'c'));

  const eingefuegt = channelsZusammenfuehren(liste, wechsel(6, 'b'));
  const ersetzt = channelsZusammenfuehren(eingefuegt, wechsel(7, 'b'));

  assert.deepEqual(namen(eingefuegt), ['a', 'b', 'c']);
  assert.equal(eingefuegt.stand, 6);
  assert.deepEqual(namen(ersetzt), ['a', 'b', 'c']);
  assert.equal(ersetzt.stand, 7);
});

test('ein Stand mit kleinerer Nummer (nach einem Neustart) ersetzt trotzdem', () => {
  const liste = channelsZusammenfuehren(channelsZusammenfuehren(LEER, stand(5, 'a', 'c')), wechsel(6, 'b'));

  const nachNeustart = channelsZusammenfuehren(liste, stand(2, 'x'));

  assert.deepEqual(namen(nachNeustart), ['x']);
  assert.equal(nachNeustart.stand, 2);
});

test('Ereignisse anderer Art lassen die Liste stehen', () => {
  const liste = channelsZusammenfuehren(LEER, stand(5, 'a'));

  for (const anderes of [{ art: 'teilnehmer', stand: 9, liste: [] }, { art: 'herzschlag' }, { art: 'unbekannt', stand: 9 }]) {
    assert.equal(channelsZusammenfuehren(liste, anderes), liste);
  }
});

test('die Sortierung folgt dem Namen, nicht der Ankunft', () => {
  let liste = channelsZusammenfuehren(LEER, stand(0));
  for (const [nummer, name] of [[1, 'allgemein'], [2, 'zeta'], [3, 'beta'], [4, 'a-1'], [5, 'a1']]) {
    liste = channelsZusammenfuehren(liste, wechsel(nummer, name));
  }

  assert.deepEqual(namen(liste), ['a-1', 'a1', 'allgemein', 'beta', 'zeta']);
});
