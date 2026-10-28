// Senden verdrahten (ADR-0037 Regeln 1–4): Eingabe → Zustellung → Abruf → Verlauf; Ereignisse `nachricht` des
// Stroms bestätigen eigene oder kommen als fremde in den offenen Verlauf. Ein Sendeauftrag zugleich je Tab. Bei
// getrennter Verbindung ruht die Warteschlange, Ereignisse fallen weg (nach dem Wiederverbinden wird nachgeholt);
// eine 401 tritt still wieder bei und versucht genau einmal neu; Fehlschläge und fremde Nachrichten gehen an die Ansage.
import { LEER, zustellen } from './zustellung.js';

// abschicken: async (channel, text, schluessel) → { art: 'antwort', status, daten, token } | { art: 'zeitueberschreitung'
// | 'netzfehler' } · wiederBeitreten: async (abgelehntesToken) · netzAus: () → bool · ansagen: (ereignis der Ansage)
export function sendenEinrichten({ verlauf, kennung, abschicken, wiederBeitreten, netzAus, ansagen }) {
  let zustand = LEER;
  let offenerChannel = null;
  let zuletztGesendetMitToken = null;

  function anwenden(ergebnis) {
    zustand = ergebnis.zustand;
    if (ergebnis.bestaetigt !== null && ergebnis.bestaetigt.channel === offenerChannel) {
      verlauf.nachrichtAnnehmen(ergebnis.bestaetigt.nachricht, true);
    }
    if (ergebnis.fremd !== null && ergebnis.fremd.channel === offenerChannel) {
      verlauf.nachrichtAnnehmen(ergebnis.fremd.nachricht, false);
    }
    if (ergebnis.fremd !== null && ergebnis.fremd.nachricht.von !== kennung) {
      ansagen({ art: 'fremd', von: ergebnis.fremd.nachricht.von, channel: ergebnis.fremd.channel, offenerChannel });
    }
    if (ergebnis.fehlschlag !== null) {
      ansagen({ art: 'fehlschlag', grund: ergebnis.fehlschlag.grund });
    }
    verlauf.vorlaeufigeZeigen(zustand.vorlaeufige);
    if (ergebnis.auftrag !== null) {
      auftragAusfuehren(ergebnis.auftrag, ergebnis.wiederbeitreten);
    }
  }

  async function auftragAusfuehren({ schluessel, channel, text }, nachAblehnung) {
    if (nachAblehnung) {
      await wiederBeitreten(zuletztGesendetMitToken);
    }
    const antwort = await abschicken(channel, text, schluessel);
    zuletztGesendetMitToken = antwort.token;
    if (antwort.art === 'antwort') {
      anwenden(zustellen(zustand, { art: 'antwort', schluessel, status: antwort.status, daten: antwort.daten }));
      return;
    }
    if (netzAus()) {
      anwenden(zustellen(zustand, { art: 'getrennt' }));
    }
    anwenden(zustellen(zustand, { art: antwort.art, schluessel }));
  }

  return {
    senden: (text) => {
      anwenden(zustellen(zustand, { art: 'senden', channel: offenerChannel, text, schluessel: neuerSchluessel() }));
      verlauf.zumEndeFuerSenden();
    },
    empfangen: ({ channel, nachricht }) => {
      if (zustand.getrennt) {
        return;
      }
      anwenden(zustellen(zustand, { art: 'ereignis', kennung, channel, nachricht }));
    },
    channelGeoeffnet: (channel) => {
      offenerChannel = channel;
      verlauf.vorlaeufigeZeigen(zustand.vorlaeufige);
    },
    getrennt: () => anwenden(zustellen(zustand, { art: 'getrennt' })),
    verbunden: () => anwenden(zustellen(zustand, { art: 'verbunden' })),
    // „Erneut senden“ | „Verwerfen“ an einer fehlgeschlagenen Nachricht.
    schalter: (aktion, schluessel) => anwenden(zustellen(zustand, { art: aktion, schluessel })),
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
