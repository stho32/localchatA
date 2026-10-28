// Senden verdrahten (ADR-0037 Regeln 1–4): Eingabe → Zustellung → Abruf → Verlauf; Ereignisse `nachricht` des
// Stroms bestätigen eigene oder kommen als fremde in den offenen Verlauf. Ein Sendeauftrag zugleich je Tab.
import { LEER, zustellen } from './zustellung.js';

// abschicken: async (channel, text, schluessel) → { art: 'antwort', status, daten } | { art: 'netzfehler' }.
export function sendenEinrichten({ verlauf, kennung, abschicken }) {
  let zustand = LEER;
  let offenerChannel = null;

  function anwenden(ergebnis) {
    zustand = ergebnis.zustand;
    if (ergebnis.bestaetigt !== null && ergebnis.bestaetigt.channel === offenerChannel) {
      verlauf.nachrichtAnnehmen(ergebnis.bestaetigt.nachricht, true);
    }
    if (ergebnis.fremd !== null && ergebnis.fremd.channel === offenerChannel) {
      verlauf.nachrichtAnnehmen(ergebnis.fremd.nachricht, false);
    }
    verlauf.vorlaeufigeZeigen(zustand.vorlaeufige);
    if (ergebnis.auftrag !== null) {
      auftragAusfuehren(ergebnis.auftrag);
    }
  }

  async function auftragAusfuehren({ schluessel, channel, text }) {
    const antwort = await abschicken(channel, text, schluessel);
    if (antwort.art === 'netzfehler') {
      anwenden(zustellen(zustand, { art: 'netzfehler', schluessel }));
      return;
    }
    anwenden(zustellen(zustand, { art: 'antwort', schluessel, status: antwort.status, daten: antwort.daten }));
  }

  return {
    senden: (text) => {
      anwenden(zustellen(zustand, { art: 'senden', channel: offenerChannel, text, schluessel: neuerSchluessel() }));
      verlauf.zumEndeFuerSenden();
    },
    empfangen: ({ channel, nachricht }) => anwenden(zustellen(zustand, { art: 'ereignis', kennung, channel, nachricht })),
    channelGeoeffnet: (channel) => {
      offenerChannel = channel;
      verlauf.vorlaeufigeZeigen(zustand.vorlaeufige);
    },
  };
}

// Ein Wiederholschlüssel je Auftrag; crypto.randomUUID gibt es nur in sicheren Kontexten — über eine LAN-Adresse
// entsteht er aus getRandomValues.
function neuerSchluessel() {
  if (typeof globalThis.crypto.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }
  const bytes = globalThis.crypto.getRandomValues(new Uint8Array(16));
  return [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}
