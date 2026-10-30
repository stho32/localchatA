// Direktnachrichten der Seitenleiste verdrahten (R00050 F0180 B0786, A00005): die Liste kommt aus `GET /api/direkt` —
// bei jedem Stand der Teilnehmer, also am Anfang jedes Stroms, damit auch nach einer Trennung nichts fehlt — und aus
// jeder `nachricht` mit `direkt` (auch der eigenen: sie nennt das Gegenüber); der Status jedes Eintrags folgt der
// Teilnehmerliste. Gezeichnet wird nur, wenn sich die Einträge ändern: fremde Teilnehmer lassen den Abschnitt unberührt.
// Schlägt das Laden fehl und ist noch nichts bekannt, steht das im Abschnitt; der nächste Stand
// versucht es wieder.
// abrufen: async () → { serverWeg, status, daten: { direktgespraeche: [{ kennung }] } }
import { direktlisteZusammenfuehren, direkteintrag } from './direktliste.js';

export function direktleisteEinrichten({ seitenleiste, abrufen }) {
  const OK = 200;
  let kennungen = null;
  let anwesende = [];
  let uhr = { versatz: 0, jetzt: Date.now(), standUnbekannt: false };

  let gezeichnet = null;

  function zeichnen() {
    if (kennungen === null) {
      return;
    }
    const eintraege = kennungen.map((kennung) => direkteintrag(kennung, anwesende, uhr));
    const abbild = JSON.stringify(eintraege);
    if (abbild === gezeichnet) {
      return;
    }
    gezeichnet = abbild;
    seitenleiste.direktZeigen(eintraege);
  }

  function aufnehmen(neue) {
    kennungen = direktlisteZusammenfuehren(kennungen, neue);
    zeichnen();
  }

  async function laden() {
    const antwort = await abrufen();
    if (antwort.serverWeg || antwort.status !== OK) {
      if (kennungen === null) {
        seitenleiste.direktNichtGeladen();
      }
      return;
    }
    aufnehmen(antwort.daten.direktgespraeche.map((gespraech) => gespraech.kennung));
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
