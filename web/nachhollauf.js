// Nachhol-Lauf im Tab (ADR-0037 Regel 7, Ergänzung Karte #22/#36, 7b–7e): nach dem Wiederverbinden und nach
// `NeuerStand` holt der Tab für den offenen Channel nach — ein Lauf zugleich. Er beginnt am Cursor (jüngste bestätigte
// Zeit vor der Trennung bzw. vor `NeuerStand`), der nur mit gelungenen Blöcken vorrückt, nie mit live angenommenen
// Nachrichten. Scheitert ein Versuch, gibt es keine Meldung, und der nächste folgt nach dem Rückzug des Stroms ab dem
// Ende des gescheiterten Abrufs. Eine Trennung beendet den Lauf (das Wiederverbinden beginnt am selben Cursor neu), ein
// Channelwechsel ebenso (der neue Channel ist vollständig geöffnet).
// Ist der offene Channel nie geladen (erster Abruf gescheitert oder noch unterwegs), ist der Lauf einer der Art „laden“
// (Ergänzung Karte #61, 7g–7i): er öffnet den Channel neu, ohne Cursor, mit Zeitgrenze 10 s und demselben Rückzug;
// „abgebrochen“ (ein anderes Öffnen hat übernommen) beendet ihn still, und eine neue Wahl desselben Channels zieht
// den nächsten Versuch vor.
// Jeder Versuch beider Arten meldet seinen Beginn und sein Ende über `versuch` (Ergänzung Karte #65, Regel 2): die
// Leiste zeigt „wird verbunden …“, solange einer unterwegs ist. Das Ende steht an einer Stelle (`versuchEnde`), die
// jeder Ausgang durchläuft, und meldet je Beginn genau einmal.
import { nachholen } from './nachholen.js';
import { rueckzugInMs } from './verbindung.js';

// stand: () → { channel, geladen, gescheitert, juengste, neuesterImFenster } (Verlauf) · abrufen: async (channel, cursor,
// signal, n) → Ergebnis wie api.js verlaufNachholen · laden: async (channel, signal) → { art: 'geladen' | 'fehler' |
// 'abgebrochen' } (verlauf.oeffnen) · annehmen: (nachrichten) → Zahl der erst jetzt angenommenen · neuOeffnen: (channel)
// · fertig: (ergebnis, mitLeiste) mit ergebnis { art: 'nachgeholt', zahl } | { art: 'neu' } | { art: 'geladen' }
// · zeitgeber: { stellen(ms, rueckruf) → id, loeschen(id) } · jetzt: () → ms seit der Epoche · gescheitert:
// (naechsterVersuchUm) nach jedem gescheiterten Versuch · versuch: (laeuft) bei Beginn (true) und Ende (false) jedes Versuchs
export function nachholLaufAnlegen({
  stand, abrufen, laden, annehmen, neuOeffnen, fertig, zeitgeber = ECHTE_ZEITGEBER, jetzt = () => Date.now(),
  gescheitert = () => {}, versuch = () => {},
}) {
  let lauf = null;
  let beiTrennung = null;

  function beginnen(channel, cursor, mitLeiste) {
    lauf = { art: 'nachholen', channel, cursor, mitLeiste, fehlversuche: 0, zahl: 0, abbruch: null, neuversuch: null, unterwegs: false };
    versuchen(lauf);
  }

  function ladenBeginnen(channel, mitLeiste) {
    lauf = { art: 'laden', channel, mitLeiste, fehlversuche: 0, abbruch: null, neuversuch: null, unterwegs: false };
    ladenVersuchen(lauf);
  }

  async function versuchen(dieser) {
    dieser.abbruch = new AbortController();
    versuchBeginn(dieser);
    const holung = await bloeckeHolen(dieser);
    versuchEnde(dieser);
    if (dieser !== lauf) {
      return;
    }
    if (holung.ergebnis.art === 'neu') {
      lauf = null;
      neuOeffnen(dieser.channel);
      fertig({ art: 'neu' }, dieser.mitLeiste);
      return;
    }
    dieser.zahl += annehmen(holung.nachrichten);
    dieser.cursor = holung.cursor;
    if (holung.ergebnis.art === 'nachgeholt') {
      lauf = null;
      fertig({ art: 'nachgeholt', zahl: dieser.zahl }, dieser.mitLeiste);
      return;
    }
    neuversuchStellen(dieser, versuchen);
  }

  async function ladenVersuchen(dieser) {
    const ZEITGRENZE_MS = 10000;
    dieser.neuversuch = null;
    const abbruch = new AbortController();
    dieser.abbruch = abbruch;
    versuchBeginn(dieser);
    const zeitgrenze = zeitgeber.stellen(ZEITGRENZE_MS, () => abbruch.abort());
    const ergebnis = await laden(dieser.channel, abbruch.signal);
    zeitgeber.loeschen(zeitgrenze);
    versuchEnde(dieser);
    if (dieser !== lauf) {
      return;
    }
    if (ergebnis.art === 'abgebrochen') {
      lauf = null;
      return;
    }
    if (ergebnis.art === 'geladen') {
      lauf = null;
      fertig({ art: 'geladen' }, dieser.mitLeiste);
      return;
    }
    neuversuchStellen(dieser, ladenVersuchen);
  }

  function versuchBeginn(dieser) {
    dieser.unterwegs = true;
    versuch(true);
  }

  function versuchEnde(dieser) {
    if (!dieser.unterwegs) {
      return;
    }
    dieser.unterwegs = false;
    versuch(false);
  }

  function neuversuchStellen(dieser, erneut) {
    dieser.fehlversuche += 1;
    const wartenMs = rueckzugInMs(dieser.fehlversuche);
    dieser.neuversuch = zeitgeber.stellen(wartenMs, () => erneut(dieser));
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
    versuchEnde(lauf);
    zeitgeber.loeschen(lauf.neuversuch);
    lauf = null;
  }

  function nachholbar(jetzt) {
    return jetzt.channel !== null && jetzt.geladen && jetzt.neuesterImFenster;
  }

  function nieGeladen(jetzt) {
    return jetzt.channel !== null && !jetzt.geladen;
  }

  return {
    getrennt: () => {
      const jetzt = stand();
      beiTrennung = null;
      if (lauf !== null && lauf.art === 'laden') {
        beenden();
        return;
      }
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
      if (nieGeladen(jetzt)) {
        ladenBeginnen(jetzt.channel, true);
        return;
      }
      if (!nachholbar(jetzt)) {
        fertig({ art: 'nachgeholt', zahl: 0 }, true);
        return;
      }
      beginnen(jetzt.channel, cursor, true);
    },
    // Ein erster Abruf, der noch unterwegs ist, bringt den neuen Stand selbst — geladen wird nur nach einem Fehlschlag.
    neuerStand: () => {
      const jetzt = stand();
      if (lauf !== null) {
        return;
      }
      if (jetzt.channel !== null && jetzt.gescheitert) {
        ladenBeginnen(jetzt.channel, false);
        return;
      }
      if (!nachholbar(jetzt)) {
        return;
      }
      beginnen(jetzt.channel, jetzt.juengste, false);
    },
    laedt: () => lauf !== null && lauf.art === 'laden',
    // Wartet ein Laden-Lauf auf seinen Neuversuch, versucht er es jetzt; läuft ein Versuch, bleibt es bei ihm (7i).
    jetztVersuchen: () => {
      if (lauf === null || lauf.art !== 'laden' || lauf.neuversuch === null) {
        return;
      }
      zeitgeber.loeschen(lauf.neuversuch);
      ladenVersuchen(lauf);
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
