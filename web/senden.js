// Senden verdrahten (ADR-0037 Regeln 1–4): Eingabe → Zustellung → Abruf → Verlauf; Ereignisse `nachricht` des
// Stroms bestätigen eigene oder kommen als fremde in den offenen Verlauf. Ein Sendeauftrag zugleich je Tab. Bei
// getrennter Verbindung ruht die Warteschlange, Ereignisse fallen weg; ab dem Wiederverbinden des Stroms werden sie
// wieder angenommen, die Warteschlange läuft erst nach dem Nachholen (ADR-0037 Ergänzung Karte #22/#36, 7f);
// eine 401 tritt still wieder bei und versucht genau einmal neu; scheitert der Beitritt, wird nicht gesendet (R00025
// F0088). Die Zustellung gehört dem Tab (`zustellung`), nicht der Hauptansicht: sie übersteht den Beitreten-Bildschirm.
// Fehlschläge und fremde Nachrichten gehen an die Ansage.
import { LEER, zustellen, naechsterVerfall } from './zustellung.js';
import { zahlMitEinheit } from './groesse.js';

// abschicken: async (channel, text, schluessel, antwortAuf, anhaenge) → { art: 'antwort', status, daten, token } | { art: 'zeitueberschreitung'
// | 'netzfehler' } · bezugAbgewiesen({ channel, text, grund }): die Instanz hat den Bezug einer Antwort abgewiesen, ihr Text
// geht an die Eingabe zurück (R00047 B0715) · wiederBeitreten: async (abgelehntesToken) → zustand der Sitzung · netzAus: () → bool · ansagen:
// (ereignis der Ansage) · zustellung: { zustand, verfall } des Tabs · jetzt: () → ms seit der Epoche · zeitgeber:
// { stellen(ms, rueckruf) → id, loeschen(id) }
export function sendenEinrichten({
  verlauf, kennung, abschicken, wiederBeitreten, netzAus, ansagen, bezugAbgewiesen = () => {}, zustellung = { zustand: LEER }, jetzt = () => Date.now(),
  zeitgeber = ECHTE_ZEITGEBER,
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
    if (ergebnis.bezugAbgewiesen !== null) {
      bezugAbgewiesen(ergebnis.bezugAbgewiesen);
    }
    verlauf.vorlaeufigeZeigen(zustellung.zustand.vorlaeufige);
    verfallStellen();
    if (ergebnis.auftrag !== null) {
      auftragAusfuehren(ergebnis.auftrag, ergebnis.wiederbeitreten);
    }
  }

  // Ein Zeitgeber je Tab für die nächste fällige verworfene (ADR-0037 Ergänzung „Verworfen auf Zeit“ Regel 4); er
  // gehört der Zustellung des Tabs, nicht dem DOM.
  function verfallStellen() {
    zeitgeber.loeschen(zustellung.verfall);
    zustellung.verfall = null;
    const faellig = naechsterVerfall(zustellung.zustand);
    if (faellig === null) {
      return;
    }
    zustellung.verfall = zeitgeber.stellen(Math.max(0, faellig - jetzt()), () => {
      anwenden(zustellen(zustellung.zustand, { art: 'verfallen', jetzt: jetzt() }));
    });
  }

  async function auftragAusfuehren({ schluessel, channel, text, antwortAuf, anhaenge }, nachAblehnung) {
    if (nachAblehnung) {
      const sitzung = await wiederBeitreten(zuletztGesendetMitToken);
      if (sitzung.art !== 'kopfzeile') {
        return;
      }
    }
    const antwort = await abschicken(channel, text, schluessel, antwortAuf, anhaenge);
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

  // Eine neue Hauptansicht beginnt verbunden (VERBINDUNG_ANFANG); die Zustellung des Tabs kann noch getrennt sein — aus
  // der Ansicht, deren stiller Beitritt gescheitert ist (R00025 F0088); sonst ruhte ihre Warteschlange für immer.
  anwenden(zustellen(zustellung.zustand, { art: 'verbunden' }));

  return {
    // bezug: der Bezug einer Antwort (zitat.js) oder null; channel: sonst der offene (eine Nachricht nach dem Hochladen
    // geht in ihren Channel), anhaenge: die IDs der hochgeladenen Dateien (R00030 B0450, B0453).
    senden: (text, bezug = null, { channel = offenerChannel, anhaenge = [] } = {}) => {
      anwenden(zustellen(zustellung.zustand, { art: 'senden', channel, text, bezug, anhaenge, schluessel: neuerSchluessel() }));
      verlauf.zumEndeFuerSenden();
    },
    empfangen: ({ channel, nachricht }) => {
      if (ereignisseRuhen) {
        return;
      }
      anwenden(zustellen(zustellung.zustand, { art: 'ereignis', kennung, channel, nachricht }));
    },
    // Eine geänderte Fassung (ADR-0048 Regel 11) und ihr Zitat (R00047 B0721, ADR-0053 Regel 5) ruhen mit den Ereignissen;
    // was in der Trennung geschah, bringt das Nachholen.
    fassungEmpfangen: ({ channel, nachricht, zitat }) => {
      if (ereignisseRuhen) {
        return;
      }
      verlauf.fassungAnnehmen(channel, nachricht);
      if (zitat !== undefined && zitat !== null) {
        verlauf.zitatAnnehmen(channel, zitat);
      }
    },
    // Gelöschte Anhänge (R00030 B0518) ruhen wie die Fassungen mit den Ereignissen; das Nachholen bringt, was in der
    // Trennung geschah.
    anhaengeEmpfangen: ({ channel, geloescht }) => {
      if (ereignisseRuhen) {
        return;
      }
      verlauf.anhaengeAnnehmen(channel, geloescht);
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
    schalter: (aktion, schluessel) => {
      anwenden(zustellen(zustellung.zustand, { art: aktion, schluessel, jetzt: jetzt() }));
      if (aktion === 'verwerfen') {
        ansagen({ art: 'sofort', text: `Verworfen · Rückgängig ist ${zahlMitEinheit(5, 'Sekunden')} lang möglich` });
      }
    },
    // Fokus auf „Rückgängig“ setzt die 5 s neu (R00025 F0085, V2).
    rueckgaengigFokussiert: (schluessel) => {
      anwenden(zustellen(zustellung.zustand, { art: 'fokusRueckgaengig', schluessel, jetzt: jetzt() }));
    },
  };
}

// Ein Wiederholschlüssel je Auftrag; crypto.randomUUID gibt es nur in sicheren Kontexten — über eine LAN-Adresse
// entsteht er aus getRandomValues.
export function neuerSchluessel() {
  if (typeof globalThis.crypto.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }
  const bytes = globalThis.crypto.getRandomValues(new Uint8Array(16));
  return [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

const ECHTE_ZEITGEBER = {
  stellen: (ms, rueckruf) => setTimeout(rueckruf, ms),
  loeschen: (id) => clearTimeout(id),
};
