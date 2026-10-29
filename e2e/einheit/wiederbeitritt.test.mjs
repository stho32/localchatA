// Unit-Tests des einen Wegs zum Wiederbeitritt (R00025 F0088, B0410; ADR-0012 Ergänzung Karte #36 Regeln 1–3; node
// --test) mit gestellter API, gestelltem Speicher und einer Sperre ohne Web Locks.
import test from 'node:test';
import assert from 'node:assert/strict';
import { wiederbeitrittAnlegen } from '../../web/wiederbeitritt.js';
import { SITZUNG, NAME } from '../../web/sitzung.js';

function speicherMit(eintraege) {
  const werte = new Map(Object.entries(eintraege));
  return {
    getItem: (schluessel) => (werte.has(schluessel) ? werte.get(schluessel) : null),
    setItem: (schluessel, wert) => werte.set(schluessel, wert),
    removeItem: (schluessel) => werte.delete(schluessel),
  };
}

function apiMit(antwort) {
  const beitritte = [];
  return {
    beitritte,
    beitreten: async (name, token) => {
      beitritte.push({ name, token });
      await new Promise((weiter) => setImmediate(weiter));
      return antwort;
    },
    ich: async () => ({ status: 401 }),
  };
}

const NEU = { status: 201, daten: { token: 'neu', name: 'anna', kennung: 'anna@127.0.0.1', herkunft: '127.0.0.1', art: 'mensch' } };

function anlegen(speicher, api) {
  const gelungen = [];
  const gescheitert = [];
  const wiederBeitreten = wiederbeitrittAnlegen({
    speicher,
    api,
    sperre: (arbeit) => arbeit({ neuesToken: async () => null }),
    gelungen: (token) => gelungen.push(token),
    gescheitert: (zustand) => gescheitert.push(zustand),
  });
  return { wiederBeitreten, gelungen, gescheitert };
}

// ADR-0012 Ergänzung Karte #36, erster Test (wörtlich): „Strom und Senden melden dasselbe abgelehnte Token im selben
// Task → genau ein `POST /api/beitreten`. Mutante „zwei Läufe“ → rot.“
test('Ein Lauf: Strom und Senden melden dasselbe abgelehnte Token → genau ein Beitritt, beide mit demselben Ergebnis', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'anna' });
  const api = apiMit(NEU);
  const { wiederBeitreten, gelungen } = anlegen(speicher, api);

  const ausDemStrom = wiederBeitreten('alt');
  const ausDemSenden = wiederBeitreten('alt');
  const [strom, senden] = await Promise.all([ausDemStrom, ausDemSenden]);

  assert.equal(api.beitritte.length, 1);
  assert.equal(strom, senden);
  assert.equal(strom.art, 'kopfzeile');
  assert.deepEqual(gelungen, ['neu']);
  assert.equal(speicher.getItem(SITZUNG), 'neu');
});

// „Ein Weg“: hat ein anderer Tab schon ein neues Token geschrieben, gilt es — kein Beitritt; sonst genau einer mit dem
// gemerkten Namen und dem abgelehnten Token (lebt die Sitzung doch, ist es eine Wiederkehr), danach trägt der Speicher
// das neue Token (der eine neue Versuch des Sendens liest es dort).
test('Ein Weg: das Token eines anderen Tabs gilt ohne Beitritt; sonst ein stiller Beitritt mit dem gemerkten Namen', async () => {
  const geschwister = speicherMit({ [SITZUNG]: 'vom-anderen-tab', [NAME]: 'anna' });
  const ohneBeitritt = apiMit(NEU);
  const mitGeschwister = anlegen(geschwister, ohneBeitritt);
  const allein = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'anna' });
  const mitBeitritt = apiMit(NEU);
  const ohneGeschwister = anlegen(allein, mitBeitritt);

  await mitGeschwister.wiederBeitreten('alt');
  await ohneGeschwister.wiederBeitreten('alt');

  assert.deepEqual(ohneBeitritt.beitritte, []);
  assert.deepEqual(mitGeschwister.gelungen, ['vom-anderen-tab']);
  assert.deepEqual(mitBeitritt.beitritte, [{ name: 'anna', token: 'alt' }]);
  assert.deepEqual(ohneGeschwister.gelungen, ['neu']);
});

test('scheitert der stille Beitritt, geht der Zustand an gescheitert, nicht an gelungen', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'anna' });
  const api = apiMit({ status: 409, daten: { belegt_von: '127.0.0.9', seit: '2026-10-28T15:00:00+01:00', vorschlag: 'anna-2' } });
  const { wiederBeitreten, gelungen, gescheitert } = anlegen(speicher, api);

  const zustand = await wiederBeitreten('alt');

  assert.equal(zustand.art, 'vergeben');
  assert.deepEqual(gelungen, []);
  assert.deepEqual(gescheitert.map((z) => z.art), ['vergeben']);
});

// Befund #58 (Härtung #49, r00004 B2 unter Last): ohne Web Locks kann nach dem 409 der gemeinsame Speicher leer sein —
// das eigene tokenVergessen löscht im Wettlauf über localStorage auch das eben geschriebene Token des Nachbar-Tabs.
// Abnahme-Probe wörtlich: Speicher nach dem 409 leer, geschwister.neuesToken liefert 'neu', ich('neu') 200 →
// gelungen(['neu']), nie das Token neu aus dem Speicher (heute [null]: der Strom wartet ewig auf ein Token).
test('Befund #58: gelungen bekommt das geprüfte Token des Nachbar-Tabs, nicht null', async () => {
  const speicher = speicherMit({ [SITZUNG]: 'alt', [NAME]: 'shoff' });
  const api = {
    beitreten: async () => ({ status: 409, daten: { belegt_von: 'shoff@127.0.0.1', seit: 'x', vorschlag: 'shoff-2' } }),
    ich: async (token) => ({ status: token === 'neu' ? 200 : 401, daten: { name: 'shoff' } }),
  };
  const gelungen = [];
  const wiederBeitreten = wiederbeitrittAnlegen({
    speicher, api, sperre: (arbeit) => arbeit({ neuesToken: async () => 'neu' }),
    gelungen: (token) => gelungen.push(token), gescheitert: () => {},
  });

  const zustand = await wiederBeitreten('alt');

  assert.equal(zustand.art, 'kopfzeile');
  assert.deepEqual(gelungen, ['neu']);
});

// Befund #58, Klasse: auf jedem der drei Wege zum Erfolg bekommt gelungen das Token, das eben geprüft bzw. vergeben
// wurde — auch wenn ein Nachbar-Tab den Speicher zwischen Prüfung und Meldung leert oder der Speicher nichts annimmt.
const WEGE = [
  { weg: 'Token eines anderen Tabs im Speicher', speicher: { [SITZUNG]: 'vom-anderen-tab', [NAME]: 'shoff' }, beitritt: NEU, nachbar: null, erwartet: 'vom-anderen-tab' },
  { weg: 'eigener Beitritt 201', speicher: { [SITZUNG]: 'alt', [NAME]: 'shoff' }, beitritt: NEU, nachbar: null, erwartet: 'neu' },
  { weg: '409, Token des Nachbar-Tabs über das storage-Ereignis', speicher: { [SITZUNG]: 'alt', [NAME]: 'shoff' }, beitritt: { status: 409, daten: { belegt_von: 'shoff@127.0.0.1', seit: 'x', vorschlag: 'shoff-2' } }, nachbar: 'vom-nachbarn', erwartet: 'vom-nachbarn' },
];
for (const { weg, speicher: eintraege, beitritt, nachbar, erwartet } of WEGE) {
  for (const stoerung of ['Nachbar-Tab leert den Speicher nach der Prüfung', 'Speicher nimmt nichts an']) {
    test(`Befund #58 (${weg}, ${stoerung}): gelungen bekommt ${erwartet}`, async () => {
      const speicher = speicherMit(eintraege);
      if (stoerung === 'Speicher nimmt nichts an') {
        speicher.setItem = () => { throw new Error('QuotaExceededError'); };
      }
      const api = { ...apiMit(beitritt), ich: async (token) => ({ status: token === nachbar ? 200 : 401, daten: { name: 'shoff' } }) };
      const gelungen = [];
      const wiederBeitreten = wiederbeitrittAnlegen({
        speicher,
        api,
        sperre: async (arbeit) => {
          const zustand = await arbeit({ neuesToken: async () => nachbar });
          if (stoerung === 'Nachbar-Tab leert den Speicher nach der Prüfung') {
            speicher.removeItem(SITZUNG);
          }
          return zustand;
        },
        gelungen: (token) => gelungen.push(token),
        gescheitert: () => {},
      });

      const zustand = await wiederBeitreten('alt');

      assert.equal(zustand.art, 'kopfzeile');
      assert.deepEqual(gelungen, [erwartet]);
    });
  }
}
