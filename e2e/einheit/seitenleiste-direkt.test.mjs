// Unit-Tests der verdrahteten Direktliste (R00050 F0180 B0786; Nacharbeit code-reviewer #13 Sollte 2; node --test):
// gezeichnet wird nur bei geänderten Einträgen.
import test from 'node:test';
import assert from 'node:assert/strict';
import { direktleisteEinrichten } from '../../web/seitenleiste-direkt.js';

const BEN = 'ben@127.0.0.3';
const CORA = 'cora@127.0.0.9';
const antwort = (...kennungen) => ({ serverWeg: false, status: 200, daten: { direktgespraeche: kennungen.map((kennung) => ({ kennung })) } });
const FEHLER = { serverWeg: false, status: 500, daten: null };
const online = (kennung) => ({ kennung, name: kennung.split('@')[0], status: 'online', weg_um: null });
const offline = (kennung) => ({ kennung, name: kennung.split('@')[0], status: 'offline', weg_um: null });

function aufbauen(antworten) {
  const gezeichnet = [];
  const wiederholungen = [];
  const seitenleiste = { direktZeigen: (eintraege, nichtGeladen) => gezeichnet.push({ kennungen: eintraege.map(({ kennung }) => kennung), nichtGeladen, eintraege }) };
  const leiste = direktleisteEinrichten({
    seitenleiste,
    abrufen: async () => {
      const naechste = antworten.shift();
      if (naechste instanceof Error) {
        throw naechste;
      }
      return naechste;
    },
    spaeter: (rueckruf, ms) => wiederholungen.push({ rueckruf, ms }),
  });
  return { leiste, gezeichnet, wiederholungen };
}

test('ein fremder Beitritt ändert die Einträge nicht → nicht neu gezeichnet; ben online → offline ist eine echte Änderung', async () => {
  const { leiste, gezeichnet } = aufbauen([antwort(BEN)]);
  await leiste.laden();
  leiste.anwesendeZeigen([online(BEN)], 0);
  const nachErstemStand = gezeichnet.length;

  leiste.anwesendeZeigen([online(BEN), online('dora@127.0.0.4')], 0);
  leiste.standUnbekannt(false);
  assert.equal(gezeichnet.length, nachErstemStand, 'fremder Beitritt zeichnet nichts');

  leiste.anwesendeZeigen([offline(BEN), online('dora@127.0.0.4')], 0);
  assert.equal(gezeichnet.length, nachErstemStand + 1);
  assert.equal(gezeichnet.at(-1).eintraege[0].wort, 'offline');
});

test('ein Laden ohne Neues zeichnet nichts neu', async () => {
  const { leiste, gezeichnet } = aufbauen([antwort(BEN), antwort(BEN)]);
  await leiste.laden();
  leiste.anwesendeZeigen([online(BEN)], 0);
  const vorher = gezeichnet.length;
  await leiste.laden();
  assert.equal(gezeichnet.length, vorher);
});
