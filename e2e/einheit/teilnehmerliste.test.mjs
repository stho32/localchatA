// Unit-Tests der Teilnehmerliste (R00004 F0013, ADR-0016 Regel 5; node --test): Zusammenführung nach `stand`.
import test from 'node:test';
import assert from 'node:assert/strict';
import { LEER, ordnen, zusammenfuehren } from '../../web/teilnehmerliste.js';

function zeile(name, status, art = 'agent', herkunft = '127.0.0.1') {
  return { kennung: `${name}@${herkunft}`, name, herkunft, art, status, seit: '2026-10-28T09:00:00.000+01:00' };
}

function stand(nummer, zeilen) {
  return { art: 'teilnehmer', stand: nummer, liste: zeilen };
}

function wechsel(nummer, teilnehmer) {
  return { art: 'anwesenheit', stand: nummer, teilnehmer };
}

function statusVon(liste) {
  return Object.fromEntries(liste.eintraege.map((eintrag) => [eintrag.kennung, eintrag.status]));
}

test('Stand 5, danach Wechsel mit stand 4 verworfen, mit stand 6 übernommen; weg entfernt die Zeile', () => {
  const nachStand = zusammenfuehren(LEER, stand(5, [zeile('alpha', 'online'), zeile('beta', 'online')]));
  const alt = zusammenfuehren(nachStand, wechsel(4, zeile('alpha', 'offline')));
  const neu = zusammenfuehren(alt, wechsel(6, zeile('alpha', 'offline')));
  const weg = zusammenfuehren(neu, wechsel(7, zeile('beta', 'weg')));

  assert.deepEqual(statusVon(alt), { 'alpha@127.0.0.1': 'online', 'beta@127.0.0.1': 'online' });
  assert.deepEqual(statusVon(neu), { 'alpha@127.0.0.1': 'offline', 'beta@127.0.0.1': 'online' });
  assert.deepEqual(statusVon(weg), { 'alpha@127.0.0.1': 'offline' });
});

test('vertauschte Veröffentlichung: ein älterer Wechsel überschreibt keinen jüngeren, auch nicht nach weg', () => {
  const start = zusammenfuehren(LEER, stand(5, []));
  const online = zusammenfuehren(start, wechsel(7, zeile('gamma', 'online')));
  const spaetesWeg = zusammenfuehren(online, wechsel(6, zeile('gamma', 'weg')));
  const weg = zusammenfuehren(start, wechsel(9, zeile('delta', 'weg')));
  const spaetesOnline = zusammenfuehren(weg, wechsel(8, zeile('delta', 'online')));

  assert.deepEqual(statusVon(spaetesWeg), { 'gamma@127.0.0.1': 'online' });
  assert.deepEqual(statusVon(spaetesOnline), {});
});

test('ein neuer Stand ersetzt die Liste ganz, auch mit kleinerer Nummer (Neustart des Programms)', () => {
  const vorher = zusammenfuehren(LEER, stand(50, [zeile('alpha', 'online'), zeile('beta', 'offline')]));
  const nachNeustart = zusammenfuehren(vorher, stand(1, [zeile('shoff', 'online', 'mensch')]));
  const danach = zusammenfuehren(nachNeustart, wechsel(2, zeile('mia', 'online', 'mensch')));

  assert.deepEqual(statusVon(danach), { 'shoff@127.0.0.1': 'online', 'mia@127.0.0.1': 'online' });
});

test('die alte Liste bleibt unverändert', () => {
  const vorher = zusammenfuehren(LEER, stand(1, [zeile('alpha', 'online')]));

  zusammenfuehren(vorher, wechsel(2, zeile('alpha', 'weg')));

  assert.deepEqual(statusVon(vorher), { 'alpha@127.0.0.1': 'online' });
});

function kennungen(zeilen) {
  return zeilen.map((zeile) => zeile.kennung);
}

function listeAus(zeilen) {
  return zusammenfuehren(LEER, stand(1, zeilen));
}

test('Menschen zuerst, dann Agenten, je alphabetisch; die eigene Zeile ist markiert', () => {
  const liste = listeAus([
    zeile('shoff', 'online', 'mensch'), zeile('claude', 'offline'), zeile('mia', 'online', 'mensch'), zeile('a1', 'online'),
  ]);

  const gruppen = ordnen(liste, 'shoff@127.0.0.1');

  assert.deepEqual(kennungen(gruppen.menschen), ['mia@127.0.0.1', 'shoff@127.0.0.1']);
  assert.deepEqual(gruppen.menschen.map((zeile) => zeile.eigene), [false, true]);
  assert.equal(gruppen.agenten.length, 1);
  assert.equal(gruppen.agenten[0].herkunft, null);
  assert.deepEqual(kennungen(gruppen.agenten[0].zeilen), ['a1@127.0.0.1', 'claude@127.0.0.1']);
});

test('sieben Agenten ohne Unterkopf, ab dem achten nach Herkunft gruppiert, alphabetisch innerhalb', () => {
  const sieben = ['a7', 'a3', 'a1', 'a5', 'a2', 'a6', 'a4'].map((name) => zeile(name, 'online'));
  const mitAchtem = [...sieben, zeile('a8', 'online', 'agent', '127.0.0.2')];

  const ungruppiert = ordnen(listeAus(sieben), 'shoff@127.0.0.1');
  const gruppiert = ordnen(listeAus(mitAchtem), 'shoff@127.0.0.1');

  assert.deepEqual(ungruppiert.agenten.map((gruppe) => gruppe.herkunft), [null]);
  assert.deepEqual(kennungen(ungruppiert.agenten[0].zeilen), ['a1', 'a2', 'a3', 'a4', 'a5', 'a6', 'a7'].map((n) => `${n}@127.0.0.1`));
  assert.deepEqual(gruppiert.agenten.map((gruppe) => [gruppe.herkunft, gruppe.zeilen.length]), [['127.0.0.1', 7], ['127.0.0.2', 1]]);
  assert.deepEqual(kennungen(gruppiert.agenten[0].zeilen), ['a1', 'a2', 'a3', 'a4', 'a5', 'a6', 'a7'].map((n) => `${n}@127.0.0.1`));
  assert.deepEqual(gruppiert.menschen, []);
});

test('Herkünfte in natürlicher Reihenfolge: 127.0.0.2 vor 127.0.0.10', () => {
  const agenten = ['b1', 'b2', 'b3', 'b4', 'b5', 'b6', 'b7'].map((name) => zeile(name, 'online', 'agent', '127.0.0.10'));
  const gruppen = ordnen(listeAus([...agenten, zeile('c1', 'online', 'agent', '127.0.0.2')]), '');

  assert.deepEqual(gruppen.agenten.map((gruppe) => gruppe.herkunft), ['127.0.0.2', '127.0.0.10']);
});
