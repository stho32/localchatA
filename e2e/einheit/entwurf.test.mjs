// Unit-Tests des Entwurfs je Channel (R00012 F0048 „Entwurf“, B0249; ADR-0038 Regel 7; node --test, gestellte Zeit).
import test from 'node:test';
import assert from 'node:assert/strict';
import { entwurfAnlegen } from '../../web/entwurf.js';

function gestellt() {
  let jetzt = 0;
  let wartende = [];
  const daten = new Map();
  return {
    speicher: {
      getItem: (schluessel) => (daten.has(schluessel) ? daten.get(schluessel) : null),
      setItem: (schluessel, wert) => daten.set(schluessel, String(wert)),
      removeItem: (schluessel) => daten.delete(schluessel),
    },
    zeitgeber: {
      stellen: (ms, rueckruf) => {
        const eintrag = { faellig: jetzt + ms, rueckruf };
        wartende.push(eintrag);
        return eintrag;
      },
      loeschen: (eintrag) => { wartende = wartende.filter((wartend) => wartend !== eintrag); },
    },
    vorstellen: (ms) => {
      jetzt += ms;
      const faellige = wartende.filter((wartend) => wartend.faellig <= jetzt);
      wartende = wartende.filter((wartend) => wartend.faellig > jetzt);
      faellige.forEach((wartend) => wartend.rueckruf());
    },
    daten,
  };
}

const SCHLUESSEL = 'localchatA.entwurf.allgemein';

test('Eingabe wird höchstens alle 500 ms geschrieben, spätestens 500 ms nach der ersten Taste mit dem letzten Stand', () => {
  const welt = gestellt();
  const entwurf = entwurfAnlegen(welt);

  entwurf.eingabe('allgemein', 'h');
  welt.vorstellen(200);
  entwurf.eingabe('allgemein', 'ha');
  welt.vorstellen(299);
  const bei499 = welt.daten.get(SCHLUESSEL);
  welt.vorstellen(1);
  const bei500 = welt.daten.get(SCHLUESSEL);
  entwurf.eingabe('allgemein', 'hal');
  entwurf.eingabe('allgemein', 'halb');
  welt.vorstellen(500);

  assert.equal(bei499, undefined);
  assert.equal(bei500, 'ha');
  assert.equal(welt.daten.get(SCHLUESSEL), 'halb');
});

test('beim Verlassen sofort geschrieben, beim Öffnen zurückgegeben, je Channel getrennt', () => {
  const welt = gestellt();
  const entwurf = entwurfAnlegen(welt);

  entwurf.eingabe('allgemein', 'halb');
  entwurf.weggehen('allgemein', 'halb');
  const sofort = welt.daten.get(SCHLUESSEL);

  assert.equal(sofort, 'halb');
  assert.equal(entwurf.oeffnen('allgemein'), 'halb');
  assert.equal(entwurf.oeffnen('technik'), '');
});

test('ein leeres Feld und das Senden löschen den Entwurf; ein ausstehendes Schreiben fällt weg', () => {
  const welt = gestellt();
  const entwurf = entwurfAnlegen(welt);

  entwurf.eingabe('allgemein', 'x');
  entwurf.eingabe('allgemein', '');
  welt.vorstellen(1000);
  const nachLeeren = welt.daten.has(SCHLUESSEL);
  entwurf.weggehen('allgemein', 'halb');
  entwurf.eingabe('allgemein', 'halb!');
  entwurf.gesendet('allgemein');
  welt.vorstellen(1000);

  assert.equal(nachLeeren, false);
  assert.equal(welt.daten.has(SCHLUESSEL), false);
});

test('ein Speicher, der nichts annimmt, lässt das Tippen weiterlaufen', () => {
  const welt = gestellt();
  welt.speicher.setItem = () => { throw new Error('QuotaExceededError'); };
  const entwurf = entwurfAnlegen(welt);

  entwurf.weggehen('allgemein', 'halb');

  assert.equal(entwurf.oeffnen('allgemein'), '');
});

test('steht beim Tippen in einem anderen Channel noch ein Schreiben aus, wird es zuerst geschrieben', () => {
  const welt = gestellt();
  const entwurf = entwurfAnlegen(welt);

  entwurf.eingabe('allgemein', 'a');
  entwurf.eingabe('technik', 'b');
  welt.vorstellen(500);

  assert.equal(welt.daten.get(SCHLUESSEL), 'a');
  assert.equal(welt.daten.get('localchatA.entwurf.technik'), 'b');
});
