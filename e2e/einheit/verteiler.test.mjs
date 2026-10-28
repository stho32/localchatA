// Unit-Tests des Verteilers im SharedWorker (R00004 F0014, B0094/B0096, ADR-0015 Regel 2 und 4; node --test):
// ein Strom für alle Tabs, Ersatz für Ports, Halter und Zeitgeber.
import test from 'node:test';
import assert from 'node:assert/strict';
import { verteilerAnlegen } from '../../web/verteiler.js';

function port() {
  const erhalten = [];
  const ersatz = {
    erhalten,
    onmessage: null,
    postMessage: (nachricht) => erhalten.push(nachricht),
    senden: (daten) => ersatz.onmessage({ data: daten }),
  };
  return ersatz;
}

function halterErsatz() {
  const angelegt = [];
  const halten = (optionen) => {
    const halter = { optionen, tokens: [optionen.token], beendet: false };
    halter.tokenSetzen = (token) => halter.tokens.push(token);
    halter.beenden = () => { halter.beendet = true; };
    angelegt.push(halter);
    return halter;
  };
  return { angelegt, halten };
}

function zeitgeberErsatz() {
  const geplant = [];
  return {
    geplant,
    warten: (ms, rueckruf) => { const eintrag = { ms, rueckruf, aktiv: true }; geplant.push(eintrag); return eintrag; },
    abbrechen: (eintrag) => { eintrag.aktiv = false; },
    ablaufen: () => geplant.filter((eintrag) => eintrag.aktiv).forEach((eintrag) => { eintrag.aktiv = false; eintrag.rueckruf(); }),
  };
}

const ereignisseIn = (einPort) => einPort.erhalten.filter((n) => n.art === 'ereignis').map((n) => n.ereignis);
const zustaendeIn = (einPort) => einPort.erhalten.filter((n) => n.art === 'zustand');

function anlegen() {
  const halter = halterErsatz();
  const zeit = zeitgeberErsatz();
  const verteiler = verteilerAnlegen({ halten: halter.halten, warten: zeit.warten, abbrechen: zeit.abbrechen });
  return { halter, zeit, verteiler };
}

test('höchstens ein Strom: das erste Token legt den Halter an, dasselbe Token vom zweiten Tab keinen weiteren', () => {
  const { halter, verteiler } = anlegen();
  const [eins, zwei] = [port(), port()];
  verteiler.verbinden(eins);
  verteiler.verbinden(zwei);

  eins.senden({ art: 'token', token: 't1' });
  zwei.senden({ art: 'token', token: 't1' });

  assert.equal(halter.angelegt.length, 1);
  assert.deepEqual(halter.angelegt[0].tokens, ['t1', 't1']);
});

test('Ereignisse und Zustände gehen an alle Tabs; ein neuer Tab bekommt sofort den letzten Stand', () => {
  const { halter, verteiler } = anlegen();
  const eins = port();
  verteiler.verbinden(eins);
  eins.senden({ art: 'token', token: 't1' });
  const { beiEreignis, beiZustand } = halter.angelegt[0].optionen;

  beiZustand('verbunden');
  beiEreignis({ art: 'teilnehmer', stand: 3, liste: [{ kennung: 'shoff@127.0.0.1', status: 'online' }] });
  beiEreignis({ art: 'anwesenheit', stand: 5, teilnehmer: { kennung: 'mia@127.0.0.1', status: 'online' } });
  const zwei = port();
  verteiler.verbinden(zwei);

  assert.deepEqual(ereignisseIn(eins).map((e) => e.stand), [3, 5]);
  assert.deepEqual(zustaendeIn(eins).map((n) => n.zustand), ['verbunden']);
  const [ersterStand] = ereignisseIn(zwei);
  assert.equal(ersterStand.art, 'teilnehmer');
  assert.equal(ersterStand.stand, 5);
  assert.deepEqual(ersterStand.liste.map((zeile) => zeile.kennung).sort(), ['mia@127.0.0.1', 'shoff@127.0.0.1']);
  assert.deepEqual(zustaendeIn(zwei).map((n) => n.zustand), ['verbunden']);
});

test('geht der letzte Tab, endet der Strom; ein späterer Tab legt einen neuen an', () => {
  const { halter, verteiler } = anlegen();
  const [eins, zwei] = [port(), port()];
  verteiler.verbinden(eins);
  verteiler.verbinden(zwei);
  eins.senden({ art: 'token', token: 't1' });

  eins.senden({ art: 'tschuess' });
  const nachDemErsten = halter.angelegt[0].beendet;
  zwei.senden({ art: 'tschuess' });
  const drei = port();
  verteiler.verbinden(drei);
  drei.senden({ art: 'token', token: 't1' });

  assert.equal(nachDemErsten, false);
  assert.equal(halter.angelegt[0].beendet, true);
  assert.equal(halter.angelegt.length, 2);
  assert.deepEqual(ereignisseIn(drei), [], 'kein alter Stand nach dem Ende');
});

test('401: genau ein Tab soll still beitreten; bleibt sein Token aus, übernimmt der nächste', () => {
  const { halter, zeit, verteiler } = anlegen();
  const [eins, zwei, drei] = [port(), port(), port()];
  [eins, zwei, drei].forEach((einPort) => verteiler.verbinden(einPort));
  eins.senden({ art: 'token', token: 'alt' });

  halter.angelegt[0].optionen.beiZustand('abgelehnt', 'alt');
  const erste = [eins, zwei, drei].map((einPort) => zustaendeIn(einPort).at(-1).beitreten);
  zeit.ablaufen();
  const zweite = [eins, zwei, drei].map((einPort) => zustaendeIn(einPort).at(-1).beitreten);
  zwei.senden({ art: 'token', token: 'alt' });
  zwei.senden({ art: 'token', token: 'neu' });
  zeit.ablaufen();

  assert.deepEqual(erste, [true, false, false]);
  assert.deepEqual(zweite, [false, true, false]);
  assert.deepEqual(halter.angelegt[0].tokens, ['alt', 'neu'], 'das abgelehnte Token wird nicht wieder gesetzt');
  assert.equal(zustaendeIn(drei).length, 2, 'nach dem neuen Token keine weitere Übernahme');
});
