// Nachhol-Lauf im Tab (ADR-0037 Regel 7, Ergänzung Karte #22/#36, 7b–7e): nach dem Wiederverbinden und nach
// `NeuerStand` holt der Tab für den offenen Channel nach — ein Lauf zugleich. Er beginnt am Cursor (jüngste bestätigte
// Zeit vor der Trennung bzw. vor `NeuerStand`), der nur mit gelungenen Blöcken vorrückt, nie mit live angenommenen
// Nachrichten. Scheitert ein Versuch, gibt es keine Meldung, und der nächste folgt nach dem Rückzug des Stroms ab dem
// Ende des gescheiterten Abrufs. Eine Trennung beendet den Lauf (das Wiederverbinden beginnt am selben Cursor neu), ein
// Channelwechsel ebenso (der neue Channel ist vollständig geöffnet).
import { nachholen } from './nachholen.js';
import { rueckzugInMs } from './verbindung.js';

// stand: () → { channel, geladen, juengste, neuesterImFenster } (Verlauf) · abrufen: async (channel, cursor, signal, n)
// → Ergebnis wie api.js verlaufNachholen · annehmen: (nachrichten) · neuOeffnen: (channel) · fertig: (ergebnis, mitLeiste)
// mit ergebnis { art: 'nachgeholt', zahl } | { art: 'neu' } · zeitgeber: { stellen(ms, rueckruf) → id, loeschen(id) }
// · jetzt: () → ms seit der Epoche · gescheitert: (naechsterVersuchUm) nach jedem gescheiterten Versuch
export function nachholLaufAnlegen({
  stand, abrufen, annehmen, neuOeffnen, fertig, zeitgeber = ECHTE_ZEITGEBER, jetzt = () => Date.now(), gescheitert = () => {},
}) {
  let lauf = null;
  let beiTrennung = null;

  function beginnen(channel, cursor, mitLeiste) {
    lauf = { channel, cursor, mitLeiste, fehlversuche: 0, zahl: 0, abbruch: null, neuversuch: null };
    versuchen(lauf);
  }

  async function versuchen(dieser) {
    dieser.abbruch = new AbortController();
    const versuch = await bloeckeHolen(dieser);
    if (dieser !== lauf) {
      return;
    }
    if (versuch.ergebnis.art === 'neu') {
      lauf = null;
      neuOeffnen(dieser.channel);
      fertig({ art: 'neu' }, dieser.mitLeiste);
      return;
    }
    annehmen(versuch.nachrichten);
    dieser.zahl += versuch.nachrichten.length;
    dieser.cursor = versuch.cursor;
    if (versuch.ergebnis.art === 'nachgeholt') {
      lauf = null;
      fertig({ art: 'nachgeholt', zahl: dieser.zahl }, dieser.mitLeiste);
      return;
    }
    dieser.fehlversuche += 1;
    const wartenMs = rueckzugInMs(dieser.fehlversuche);
    dieser.neuversuch = zeitgeber.stellen(wartenMs, () => versuchen(dieser));
    gescheitert(jetzt() + wartenMs);
  }

  // Ein Versuch über höchstens 5 Blöcke (nachholen.js); was bis zu einem Fehlschlag kam, zählt mit seinem Ende als
  // Cursor — außer der Tab öffnet ohnehin neu.
  async function bloeckeHolen(dieser) {
    const nachrichten = [];
    let cursor = dieser.cursor;
    const ergebnis = await nachholen(dieser.cursor, async (abfrage, anzahl) => {
      const antwort = await abrufen(dieser.channel, abfrage, dieser.abbruch.signal, anzahl);
      if (antwort.art === 'block' && antwort.block.nachrichten.length > 0) {
        nachrichten.push(...antwort.block.nachrichten);
        cursor = nachrichten[nachrichten.length - 1].zeit;
      }
      return antwort;
    });
    return { ergebnis, nachrichten, cursor };
  }

  function beenden() {
    lauf.abbruch.abort();
    zeitgeber.loeschen(lauf.neuversuch);
    lauf = null;
  }

  function nachholbar(jetzt) {
    return jetzt.channel !== null && jetzt.geladen && jetzt.neuesterImFenster;
  }

  return {
    getrennt: () => {
      const jetzt = stand();
      beiTrennung = null;
      if (lauf !== null) {
        beiTrennung = { channel: lauf.channel, cursor: lauf.cursor };
        beenden();
        return;
      }
      if (nachholbar(jetzt)) {
        beiTrennung = { channel: jetzt.channel, cursor: jetzt.juengste };
      }
    },
    wiederverbunden: () => {
      const jetzt = stand();
      let cursor = jetzt.juengste;
      if (beiTrennung !== null && beiTrennung.channel === jetzt.channel) {
        cursor = beiTrennung.cursor;
      }
      beiTrennung = null;
      if (!nachholbar(jetzt)) {
        fertig({ art: 'nachgeholt', zahl: 0 }, true);
        return;
      }
      beginnen(jetzt.channel, cursor, true);
    },
    neuerStand: () => {
      const jetzt = stand();
      if (lauf !== null || !nachholbar(jetzt)) {
        return;
      }
      beginnen(jetzt.channel, jetzt.juengste, false);
    },
    channelGewechselt: () => {
      beiTrennung = null;
      if (lauf === null) {
        return;
      }
      const mitLeiste = lauf.mitLeiste;
      beenden();
      fertig({ art: 'nachgeholt', zahl: 0 }, mitLeiste);
    },
  };
}

const ECHTE_ZEITGEBER = {
  stellen: (ms, rueckruf) => setTimeout(rueckruf, ms),
  loeschen: (id) => clearTimeout(id),
};
