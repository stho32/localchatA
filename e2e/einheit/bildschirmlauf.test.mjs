// Unit-Tests des Laufs auf dem Beitreten-Bildschirm (Karte #62, ADR-0012 Regeln 3–5 und 8; node --test):
// wann einem Nachbar-Tab gefolgt wird, ohne DOM, Speicher und Server.
import test from 'node:test';
import assert from 'node:assert/strict';
import { bildschirmlaufAnlegen } from '../../web/bildschirmlauf.js';
import { SITZUNG } from '../../web/sitzung.js';

const KOPFZEILE = (token) => ({ art: 'kopfzeile', teilnehmer: { kennung: 'x@127.0.0.1' }, token });

/// Ein Lauf mit gehaltenen Prüfungen: `pruefungen[i].antworten(ergebnis)` löst die i-te Prüfung eines Tokens auf.
function laufMit({ abgelehntesToken = 'tot', gemerkt = null } = {}) {
  const protokoll = { geprueft: [], gesperrt: 0, freigegeben: 0, gefolgt: [] };
  const offene = [];
  const lauf = bildschirmlaufAnlegen({
    abgelehntesToken,
    gemerktesToken: () => gemerkt,
    folgen: (token) => new Promise((antwort) => {
      protokoll.geprueft.push(token);
      offene.push((ergebnis) => antwort(ergebnis));
    }),
    gesperrt: () => { protokoll.gesperrt += 1; },
    freigegeben: () => { protokoll.freigegeben += 1; },
    gefolgt: (zustand) => { protokoll.gefolgt.push(zustand); },
  });
  return { lauf, protokoll, antworten: (ergebnis) => offene.shift()(ergebnis) };
}

const speichern = (token) => ({ key: SITZUNG, newValue: token });
const abwarten = () => new Promise((fertig) => setImmediate(fertig));

test('Signal mit neuem Token: sperren, prüfen, bei Erfolg folgen und nicht freigeben', async () => {
  const { lauf, protokoll, antworten } = laufMit();

  lauf.beimSpeichern(speichern('bert'));
  assert.deepEqual(protokoll.geprueft, ['bert']);
  assert.equal(protokoll.gesperrt, 1);
  antworten(KOPFZEILE('bert'));
  await abwarten();

  assert.deepEqual(protokoll.gefolgt, [KOPFZEILE('bert')]);
  assert.equal(protokoll.freigegeben, 0);
});

test('kein Signal: fremder Schlüssel, geleerter Wert, das abgelehnte Token', async () => {
  const { lauf, protokoll } = laufMit();

  lauf.beimSpeichern({ key: 'localchatA.name', newValue: 'bert' });
  lauf.beimSpeichern({ key: SITZUNG, newValue: null });
  lauf.beimSpeichern(speichern('tot'));
  await abwarten();

  assert.deepEqual(protokoll.geprueft, []);
  assert.equal(protokoll.gesperrt, 0);
});

test('Scheitert die Prüfung, ist das Formular wieder frei und es wird nicht gefolgt', async () => {
  const { lauf, protokoll, antworten } = laufMit();

  lauf.beimSpeichern(speichern('kaputt'));
  antworten(null);
  await abwarten();

  assert.equal(protokoll.freigegeben, 1);
  assert.deepEqual(protokoll.gefolgt, []);
});

// Regel 3 (Nachlese): ein Token, das schon im Speicher liegt, gilt als Signal; das abgelehnte nicht.
test('Nachlese: ein anderes Token im Speicher wird geprüft', async () => {
  const { lauf, protokoll, antworten } = laufMit({ gemerkt: 'shoff-token' });

  lauf.nachlesen();
  antworten(KOPFZEILE('shoff-token'));
  await abwarten();

  assert.deepEqual(protokoll.geprueft, ['shoff-token']);
  assert.deepEqual(protokoll.gefolgt, [KOPFZEILE('shoff-token')]);
});

test('Nachlese: leerer Speicher oder das abgelehnte Token lösen keine Prüfung aus', () => {
  for (const gemerkt of [null, '', 'tot']) {
    const { lauf, protokoll } = laufMit({ gemerkt });

    lauf.nachlesen();

    assert.deepEqual(protokoll.geprueft, [], String(gemerkt));
  }
});

// Regel 5: ein Signal während des Folgens wird gemerkt; scheitert der Lauf, wird das gemerkte neu geprüft.
test('gemerktes Signal: scheitert die erste Prüfung, wird das spätere Token geprüft und ihm gefolgt', async () => {
  const { lauf, protokoll, antworten } = laufMit();

  lauf.beimSpeichern(speichern('erstes'));
  lauf.beimSpeichern(speichern('bert'));
  assert.deepEqual(protokoll.geprueft, ['erstes'], 'ein Lauf zugleich');
  antworten(null);
  await abwarten();
  assert.deepEqual(protokoll.geprueft, ['erstes', 'bert']);
  antworten(KOPFZEILE('bert'));
  await abwarten();

  assert.deepEqual(protokoll.gefolgt, [KOPFZEILE('bert')]);
  assert.equal(protokoll.freigegeben, 0);
  assert.equal(protokoll.gesperrt, 1, 'gesperrt wird einmal, nicht je Prüfung');
});

test('gemerktes Signal: scheitern beide Prüfungen, ist das Formular einmal frei', async () => {
  const { lauf, protokoll, antworten } = laufMit();

  lauf.beimSpeichern(speichern('erstes'));
  lauf.beimSpeichern(speichern('zweites'));
  antworten(null);
  await abwarten();
  antworten(null);
  await abwarten();

  assert.deepEqual(protokoll.geprueft, ['erstes', 'zweites']);
  assert.equal(protokoll.freigegeben, 1);
  assert.deepEqual(protokoll.gefolgt, []);
});

test('gemerktes Signal: dasselbe Token zweimal wird nicht ein zweites Mal geprüft', async () => {
  const { lauf, protokoll, antworten } = laufMit();

  lauf.beimSpeichern(speichern('erstes'));
  lauf.beimSpeichern(speichern('erstes'));
  antworten(null);
  await abwarten();

  assert.deepEqual(protokoll.geprueft, ['erstes']);
  assert.equal(protokoll.freigegeben, 1);
});

// Wächter lauf.klickt (A2): der eigene Klick geht vor, kein Signal folgt ihm mehr.
test('nach dem eigenen Klick folgt kein Signal und keine Nachlese', async () => {
  const { lauf, protokoll } = laufMit({ gemerkt: 'anderes' });

  lauf.klickt();
  lauf.beimSpeichern(speichern('bert'));
  lauf.nachlesen();
  await abwarten();

  assert.deepEqual(protokoll.geprueft, []);
  assert.equal(protokoll.gesperrt, 0);
});

// Regel 8 (Hauptansicht folgt nicht): nach dem Abmelden wirkt kein Signal, kein laufendes Folgen zeigt noch etwas.
test('nach dem Abmelden löst kein Signal und keine Nachlese eine Prüfung aus', async () => {
  const { lauf, protokoll } = laufMit({ gemerkt: 'anderes' });

  lauf.abmelden();
  lauf.beimSpeichern(speichern('bert'));
  lauf.nachlesen();
  await abwarten();

  assert.deepEqual(protokoll.geprueft, []);
});

test('Abmelden während der Prüfung: weder folgen noch freigeben', async () => {
  const { lauf, protokoll, antworten } = laufMit();
  lauf.beimSpeichern(speichern('bert'));

  lauf.abmelden();
  antworten(KOPFZEILE('bert'));
  await abwarten();

  assert.deepEqual(protokoll.gefolgt, []);
  assert.equal(protokoll.freigegeben, 0);
});

test('Abmelden während der Prüfung, die scheitert: das Formular wird nicht mehr angefasst', async () => {
  const { lauf, protokoll, antworten } = laufMit();
  lauf.beimSpeichern(speichern('bert'));

  lauf.abmelden();
  antworten(null);
  await abwarten();

  assert.equal(protokoll.freigegeben, 0);
});
