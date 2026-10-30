// Direktnachrichten der Seitenleiste verdrahten (R00050 F0180 B0786, A00005): die Liste kommt aus `GET /api/direkt` —
// bei jedem Stand der Teilnehmer, also am Anfang jedes Stroms, damit auch nach einer Trennung nichts fehlt — und aus
// jeder `nachricht` mit `direkt` (auch der eigenen: sie nennt das Gegenüber); der Status jedes Eintrags folgt der
// Teilnehmerliste. Schlägt das Laden fehl, steht „nicht geladen“ im Abschnitt — auch neben schon bekannten Einträgen,
// denn die Liste kann unvollständig sein —, bis ein neuer Versuch gelingt; er wiederholt sich alle paar Sekunden. Gezeichnet
// wird nur, wenn sich die Einträge oder der Hinweis ändern: fremde Teilnehmer lassen den Abschnitt unberührt.
// abrufen: async () → { serverWeg, status, daten: { direktgespraeche: [{ kennung }] } }
// spaeter: (rueckruf, ms) → plant einen Rückruf (Standard setTimeout)
import { direktlisteZusammenfuehren, direkteintrag } from './direktliste.js';

export function direktleisteEinrichten({ seitenleiste, abrufen, spaeter = setTimeout }) {
  const OK = 200;
  const NEUER_VERSUCH_MS = 2000;
  let kennungen = null;
  let anwesende = [];
  let nichtGeladen = false;
  let neuerVersuchGeplant = false;
  let gezeichnet = null;
  let uhr = { versatz: 0, jetzt: Date.now(), standUnbekannt: false };

  function zeichnen() {
    if (kennungen === null && !nichtGeladen) {
      return;
    }
    const eintraege = (kennungen || []).map((kennung) => direkteintrag(kennung, anwesende, uhr));
    const abbild = JSON.stringify([eintraege, nichtGeladen]);
    if (abbild === gezeichnet) {
      return;
    }
    gezeichnet = abbild;
    seitenleiste.direktZeigen(eintraege, nichtGeladen);
  }

  function aufnehmen(neue) {
    kennungen = direktlisteZusammenfuehren(kennungen, neue);
    zeichnen();
  }

  async function abrufenOderFehlschlag() {
    try {
      return await abrufen();
    } catch {
      return { serverWeg: true, status: 0, daten: null };
    }
  }

  async function laden() {
    const antwort = await abrufenOderFehlschlag();
    if (antwort.serverWeg || antwort.status !== OK) {
      nichtGeladen = true;
      zeichnen();
      neuenVersuchPlanen();
      return;
    }
    nichtGeladen = false;
    aufnehmen(antwort.daten.direktgespraeche.map((gespraech) => gespraech.kennung));
  }

  function neuenVersuchPlanen() {
    if (neuerVersuchGeplant) {
      return;
    }
    neuerVersuchGeplant = true;
    spaeter(async () => {
      neuerVersuchGeplant = false;
      await laden();
    }, NEUER_VERSUCH_MS);
  }

  return {
    laden,
    nachricht: (ereignis) => {
      if (typeof ereignis.direkt === 'string') {
        aufnehmen([ereignis.direkt]);
      }
    },
    anwesendeZeigen: (eintraege, versatz) => {
      anwesende = eintraege;
      uhr = { ...uhr, versatz, jetzt: Date.now() };
      zeichnen();
    },
    standUnbekannt: (unbekannt) => {
      uhr = { ...uhr, standUnbekannt: unbekannt, jetzt: Date.now() };
      zeichnen();
    },
    kennungen: () => kennungen || [],
  };
}
