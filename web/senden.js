// Senden verdrahten (ADR-0037 Regeln 1–4): Eingabe → Zustellung → Abruf → Verlauf; Ereignisse `nachricht` des
// Stroms bestätigen eigene oder kommen als fremde in den offenen Verlauf. Ein Sendeauftrag zugleich je Tab. Bei
// getrennter Verbindung ruht die Warteschlange, Ereignisse fallen weg; ab dem Wiederverbinden des Stroms werden sie
// wieder angenommen, die Warteschlange läuft erst nach dem Nachholen (ADR-0037 Ergänzung Karte #22/#36, 7f);
// eine 401 tritt still wieder bei und versucht genau einmal neu; scheitert der Beitritt, wird nicht gesendet (R00025
// F0088). Die Zustellung gehört dem Tab (`zustellung`), nicht der Hauptansicht: sie übersteht den Beitreten-Bildschirm.
// Fehlschläge und fremde Nachrichten gehen an die Ansage.
import { LEER, zustellen } from './zustellung.js';

// abschicken: async (channel, text, schluessel) → { art: 'antwort', status, daten, token } | { art: 'zeitueberschreitung'
// | 'netzfehler' } · wiederBeitreten: async (abgelehntesToken) → zustand der Sitzung · netzAus: () → bool · ansagen:
// (ereignis der Ansage) · zustellung: { zustand } des Tabs · jetzt: () → ms seit der Epoche
export function sendenEinrichten({
  verlauf, kennung, abschicken, wiederBeitreten, netzAus, ansagen, zustellung = { zustand: LEER }, jetzt = () => Date.now(),
}) {
  let offenerChannel = null;
  let zuletztGesendetMitToken = null;
  let ereignisseRuhen = false;

  function anwenden(ergebnis) {
    zustellung.zustand = ergebnis.zustand;
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
    verlauf.vorlaeufigeZeigen(zustellung.zustand.vorlaeufige);
    if (ergebnis.auftrag !== null) {
      auftragAusfuehren(ergebnis.auftrag, ergebnis.wiederbeitreten);
    }
  }

  async function auftragAusfuehren({ schluessel, channel, text }, nachAblehnung) {
    if (nachAblehnung) {
      const sitzung = await wiederBeitreten(zuletztGesendetMitToken);
      if (sitzung.art !== 'kopfzeile') {
        return;
      }
    }
    const antwort = await abschicken(channel, text, schluessel);
    zuletztGesendetMitToken = antwort.token;
    if (antwort.art === 'antwort') {
      anwenden(zustellen(zustellung.zustand, { art: 'antwort', schluessel, status: antwort.status, daten: antwort.daten }));
      return;
    }
    if (netzAus()) {
      anwenden(zustellen(zustellung.zustand, { art: 'getrennt' }));
    }
    anwenden(zustellen(zustellung.zustand, { art: antwort.art, schluessel }));
  }

  return {
    senden: (text) => {
      anwenden(zustellen(zustellung.zustand, { art: 'senden', channel: offenerChannel, text, schluessel: neuerSchluessel() }));
      verlauf.zumEndeFuerSenden();
    },
    empfangen: ({ channel, nachricht }) => {
      if (ereignisseRuhen) {
        return;
      }
      anwenden(zustellen(zustellung.zustand, { art: 'ereignis', kennung, channel, nachricht }));
    },
    channelGeoeffnet: (channel) => {
      offenerChannel = channel;
      verlauf.vorlaeufigeZeigen(zustellung.zustand.vorlaeufige);
    },
    getrennt: () => {
      ereignisseRuhen = true;
      anwenden(zustellen(zustellung.zustand, { art: 'getrennt' }));
    },
    stromWieder: () => {
      ereignisseRuhen = false;
    },
    verbunden: () => anwenden(zustellen(zustellung.zustand, { art: 'verbunden' })),
    // „Erneut senden“ | „Verwerfen“ an einer fehlgeschlagenen Nachricht.
    schalter: (aktion, schluessel) => anwenden(zustellen(zustellung.zustand, { art: aktion, schluessel, jetzt: jetzt() })),
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
