// Unit-Tests des Sekundentakts (R00005 F0018, B0112, ADR-0019 Regel 5; node --test): gestellter Zeitgeber.
import test from 'node:test';
import assert from 'node:assert/strict';
import { sekundentaktAnlegen } from '../../web/sekundentakt.js';

const T = Date.parse('2026-10-28T09:00:00.000+01:00');

function zeitgeberErsatz() {
  const geplant = [];
  return {
    geplant,
    planen: (rueckruf, ms) => { const eintrag = { rueckruf, ms, aktiv: true }; geplant.push(eintrag); return eintrag; },
    abbrechen: (eintrag) => { eintrag.aktiv = false; },
    schlagen: () => geplant.filter((eintrag) => eintrag.aktiv).forEach((eintrag) => eintrag.rueckruf()),
  };
}

function zeile(kennung, status, millisekundenNachT = null) {
  let wegUm = null;
  if (millisekundenNachT !== null) {
    wegUm = new Date(T + millisekundenNachT).toISOString();
  }
  return { kennung, status, weg_um: wegUm };
}

test('eine Zeile „gleich weg“: der Takt läuft jede Sekunde und zeichnet nur sie neu', () => {
  const zeit = zeitgeberErsatz();
  const takten = sekundentaktAnlegen({ planen: zeit.planen, abbrechen: zeit.abbrechen });
  const neuGezeichnet = [];

  takten({
    zeilen: [zeile('shoff@127.0.0.1', 'online'), zeile('mia@127.0.0.1', 'offline', 40000), zeile('a1@127.0.0.2', 'offline', 90000)],
    uhr: () => ({ versatz: 0, jetzt: T }),
    zeileNeuZeichnen: (eintrag) => neuGezeichnet.push(eintrag.kennung),
  });
  zeit.schlagen();
  zeit.schlagen();

  assert.deepEqual(zeit.geplant.map((eintrag) => eintrag.ms), [1000]);
  assert.deepEqual(neuGezeichnet, ['mia@127.0.0.1', 'mia@127.0.0.1']);
});

test('keine Zeile „gleich weg“: kein Takt; ein neuer Aufruf beendet den alten Takt', () => {
  const zeit = zeitgeberErsatz();
  const takten = sekundentaktAnlegen({ planen: zeit.planen, abbrechen: zeit.abbrechen });
  const uhr = () => ({ versatz: 0, jetzt: T });

  takten({ zeilen: [zeile('mia@127.0.0.1', 'offline', 40000)], uhr, zeileNeuZeichnen: () => {} });
  takten({ zeilen: [zeile('shoff@127.0.0.1', 'online'), zeile('a1@127.0.0.2', 'offline', 90000)], uhr, zeileNeuZeichnen: () => {} });

  assert.equal(zeit.geplant.length, 1);
  assert.equal(zeit.geplant[0].aktiv, false);
});
