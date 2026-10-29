// Lauf des Beitreten-Bildschirms (Karte #62, ADR-0012 Regeln 3–5 und 8): wann einem Nachbar-Tab gefolgt wird.
// Ohne DOM und Speicher, damit Node ihn prüft; start.js reicht Speicher, Formular und Anzeige hinein.
import { SITZUNG } from './sitzung.js';

// `folgen(token)` prüft ein gemeldetes Token und liefert den Zustand der Hauptansicht oder null;
// `gesperrt`/`freigegeben` sperren und öffnen das Formular; `gefolgt(zustand)` zeigt die Hauptansicht.
export function bildschirmlaufAnlegen({ abgelehntesToken, gemerktesToken, folgen, gesperrt, freigegeben, gefolgt }) {
  const lauf = { abgemeldet: false, folgt: false, klickt: false, gemerkt: null };

  // Ein Lauf zugleich (Regel 5): ein Signal während des Folgens wird gemerkt und geprüft, wenn der Lauf scheitert.
  async function nachbarFolgen(gemeldet) {
    if (lauf.klickt || lauf.abgemeldet) {
      return;
    }
    if (lauf.folgt) {
      lauf.gemerkt = gemeldet;
      return;
    }
    lauf.folgt = true;
    gesperrt();
    let ergebnis = null;
    try {
      for (let pruefen = gemeldet; pruefen !== null && ergebnis === null;) {
        lauf.gemerkt = null;
        ergebnis = await folgen(pruefen);
        if (lauf.gemerkt !== pruefen) {
          pruefen = lauf.gemerkt;
        } else {
          pruefen = null;
        }
      }
    } finally {
      lauf.folgt = false;
      if (ergebnis === null && !lauf.abgemeldet) {
        freigegeben();
      }
    }
    if (ergebnis !== null && !lauf.abgemeldet) {
      gefolgt(ergebnis);
    }
  }

  return {
    // Der eigene Klick geht vor: danach folgt der Lauf nicht mehr (Regel 6).
    klickt() {
      lauf.klickt = true;
    },
    // Der Bildschirm steht nicht mehr (Regel 8): kein Signal wirkt mehr.
    abmelden() {
      lauf.abgemeldet = true;
    },
    // Das `storage`-Ereignis: nur ein neues Token unter dem Sitzungsschlüssel, nicht das abgelehnte.
    beimSpeichern(ereignis) {
      if (ereignis.key === SITZUNG && ereignis.newValue && ereignis.newValue !== abgelehntesToken) {
        nachbarFolgen(ereignis.newValue);
      }
    },
    // Nachlese (Regel 3): ein Token, das schon im Speicher liegt, gilt als Signal, auch wenn sein Ereignis ohne Lauscher kam.
    nachlesen() {
      const token = gemerktesToken();
      if (token && token !== abgelehntesToken) {
        nachbarFolgen(token);
      }
    },
  };
}
