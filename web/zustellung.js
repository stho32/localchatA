// Zustellung im Tab (ADR-0037 Regeln 1–4), rein: vorläufige Nachrichten aller Channels als eine Warteschlange, ein
// Sendeauftrag zugleich; bestätigt wird über die Antwort oder das eigene Ereignis, wer zuerst kommt. Bei getrenntem
// Strom ruht die Warteschlange; was dann scheitert, bleibt „wird gesendet“ und geht nach dem Wiederverbinden mit
// demselben Schlüssel hinaus.
//
// zustand: { vorlaeufige: [{ schluessel, channel, text, bezug, zustand: 'wird gesendet' | 'fehlgeschlagen', grund?,
//            wiederbeigetreten? }], laufend: schluessel | null, bestaetigt: [zeit…], getrennt }
//            (bezug: der Bezug einer Antwort, zitat.js, oder null — R00047 B0715)
// ereignis: { art: 'senden', channel, text, schluessel, bezug? } · { art: 'antwort', schluessel, status, daten }
//         · { art: 'netzfehler' | 'zeitueberschreitung', schluessel } · { art: 'ereignis', kennung, channel, nachricht }
//         · { art: 'getrennt' | 'verbunden' } · { art: 'erneut', schluessel } · { art: 'beitrittGescheitert' }
//         · { art: 'verwerfen' | 'rueckgaengig' | 'fokusRueckgaengig', schluessel, jetzt } · { art: 'verfallen', jetzt }
//         (Verworfen auf Zeit, R00025 F0085: eine verworfene steht 5 s mit `seit`, dann ist sie entfernt)
// → { zustand, auftrag: { schluessel, channel, text, antwortAuf: Zeit | null } | null, bestaetigt: { channel, nachricht } | null,
//     fremd: { channel, nachricht } | null, wiederbeitreten, fehlschlag: { grund } | null,
//     bezugAbgewiesen: { channel, text, grund } | null }
export const LEER = { vorlaeufige: [], laufend: null, bestaetigt: [], getrennt: false };

const NICHT_GESPEICHERT = 'Nicht gespeichert — der Server konnte nicht schreiben.';
const KEINE_ANTWORT = 'Keine Antwort vom Server.';
const BEITRITT_ABGELAUFEN = 'Nicht gesendet — der Beitritt war abgelaufen.';
const NICHT_ANGEMELDET = 401;

export function zustellen(zustand, ereignis) {
  switch (ereignis.art) {
    case 'senden':
      return gesendet(zustand, ereignis);
    case 'antwort':
      return beantwortet(zustand, ereignis);
    case 'netzfehler':
    case 'zeitueberschreitung':
      return ohneAntwort(zustand, ereignis.schluessel);
    case 'ereignis':
      return gehoert(zustand, ereignis);
    case 'getrennt':
      return ergebnis({ ...zustand, getrennt: true });
    case 'verbunden':
      return auftragWennFrei({ ...zustand, getrennt: false });
    case 'erneut':
      return erneutSenden(zustand, ereignis.schluessel);
    case 'verwerfen':
      return verwerfen(zustand, ereignis);
    case 'rueckgaengig':
      return rueckgaengig(zustand, ereignis);
    case 'fokusRueckgaengig':
      return fokusAufRueckgaengig(zustand, ereignis);
    case 'verfallen':
      return verfallen(zustand, ereignis.jetzt);
    case 'beitrittGescheitert':
      return beitrittGescheitert(zustand);
    default:
      throw new Error(`Zustellung: unbekanntes Ereignis ${ereignis.art}`);
  }
}

function gesendet(zustand, { channel, text, schluessel, bezug = null }) {
  const vorlaeufige = [...zustand.vorlaeufige, { schluessel, channel, text, bezug, zustand: 'wird gesendet' }];
  return auftragWennFrei({ ...zustand, vorlaeufige });
}

function beantwortet(zustand, { schluessel, status, daten }) {
  if (status === NICHT_ANGEMELDET) {
    return nichtAngemeldet(zustand, schluessel, daten);
  }
  if (bezugAbgelehnt(status, daten)) {
    return bezugAbweisen(zustand, schluessel, daten.fehler);
  }
  const erfolgreich = status === 200 || status === 201;
  if (!erfolgreich) {
    return fehlschlagen(zustand, schluessel, grundAus(status, daten));
  }
  const vorlaeufige = ohne(zustand.vorlaeufige, schluessel);
  const bestaetigung = bestaetigen({ ...zustand, vorlaeufige }, daten.channel, daten.nachricht);
  const weiter = naechsterAuftrag(bestaetigung.zustand);
  return { ...weiter, bestaetigt: bestaetigung.bestaetigt };
}

// Weist die Instanz den Bezug einer Antwort ab (422 `antwort_auf`, ADR-0053 Regel 7), ist nichts gespeichert: die
// vorläufige geht weg, ihr Text geht zurück an die Eingabe — anders als die übrigen Fehlschläge, die als fehlgeschlagen
// mit „Erneut senden“ stehen bleiben; ein Erneut-Senden mit demselben Bezug schlüge wieder fehl.
function bezugAbgelehnt(status, daten) {
  const UNVERARBEITBAR = 422;
  return status === UNVERARBEITBAR && daten !== null && typeof daten === 'object' && daten.regel === 'antwort_auf';
}

function bezugAbweisen(zustand, schluessel, grund) {
  const betroffene = zustand.vorlaeufige.find((vorlaeufig) => vorlaeufig.schluessel === schluessel);
  const weiter = naechsterAuftrag({ ...zustand, vorlaeufige: ohne(zustand.vorlaeufige, schluessel) });
  if (betroffene === undefined) {
    return weiter;
  }
  return { ...weiter, bezugAbgewiesen: { channel: betroffene.channel, text: betroffene.text, grund } };
}

// Verworfen auf Zeit (ADR-0037 Ergänzung „Verworfen auf Zeit“): 5 s lässt sich „Verwerfen“ zurücknehmen; Fokus auf
// „Rückgängig“ setzt die Zeit neu. Eine verworfene ist nicht in der Warteschlange und wird nie bestätigt.
const RUECKGAENGIG_MS = 5000;

function verwerfen(zustand, { schluessel, jetzt }) {
  return ergebnis({ ...zustand, vorlaeufige: mitGeaenderter(zustand.vorlaeufige, schluessel, (vorlaeufig) => ({ ...vorlaeufig, zustand: 'verworfen', seit: jetzt })) });
}

function rueckgaengig(zustand, { schluessel, jetzt }) {
  const zuruecknehmbar = (vorlaeufig) => vorlaeufig.zustand === 'verworfen' && jetzt - vorlaeufig.seit < RUECKGAENGIG_MS;
  return ergebnis({
    ...zustand,
    vorlaeufige: mitGeaenderter(zustand.vorlaeufige, schluessel, (vorlaeufig) => {
      if (!zuruecknehmbar(vorlaeufig)) {
        return vorlaeufig;
      }
      const { seit, ...fehlgeschlagene } = vorlaeufig;
      return { ...fehlgeschlagene, zustand: 'fehlgeschlagen' };
    }),
  });
}

function fokusAufRueckgaengig(zustand, { schluessel, jetzt }) {
  return ergebnis({
    ...zustand,
    vorlaeufige: mitGeaenderter(zustand.vorlaeufige, schluessel, (vorlaeufig) => {
      if (vorlaeufig.zustand !== 'verworfen' || jetzt - vorlaeufig.seit >= RUECKGAENGIG_MS) {
        return vorlaeufig;
      }
      return { ...vorlaeufig, seit: jetzt };
    }),
  });
}

function verfallen(zustand, jetzt) {
  const steht = (vorlaeufig) => vorlaeufig.zustand !== 'verworfen' || jetzt - vorlaeufig.seit < RUECKGAENGIG_MS;
  return ergebnis({ ...zustand, vorlaeufige: zustand.vorlaeufige.filter(steht) });
}

// Wann die nächste verworfene verfällt (ms), null ohne verworfene — für den einen Zeitgeber des Tabs.
export function naechsterVerfall(zustand) {
  const seit = zustand.vorlaeufige.filter((vorlaeufig) => vorlaeufig.zustand === 'verworfen').map((vorlaeufig) => vorlaeufig.seit);
  if (seit.length === 0) {
    return null;
  }
  return Math.min(...seit) + RUECKGAENGIG_MS;
}

function mitGeaenderter(vorlaeufige, schluessel, aendern) {
  return vorlaeufige.map((vorlaeufig) => {
    if (vorlaeufig.schluessel !== schluessel) {
      return vorlaeufig;
    }
    return aendern(vorlaeufig);
  });
}

// Scheitert der stille Wiederbeitritt (ADR-0012 Ergänzung Karte #36 Regel 4): die Warteschlange hält sofort an, jede
// Nachricht „wird gesendet“ wird fehlgeschlagen; fehlgeschlagene bleiben, wie sie sind.
function beitrittGescheitert(zustand) {
  const vorlaeufige = zustand.vorlaeufige.map((vorlaeufig) => {
    if (vorlaeufig.zustand !== 'wird gesendet') {
      return vorlaeufig;
    }
    return { ...vorlaeufig, zustand: 'fehlgeschlagen', grund: BEITRITT_ABGELAUFEN };
  });
  return ergebnis({ ...zustand, vorlaeufige, laufend: null });
}

// Die erste 401 eines Versuchs: still wieder beitreten und genau einmal mit demselben Schlüssel neu senden — nur,
// solange die Nachricht noch gesendet wird (nach einem gescheiterten Beitritt ist sie schon fehlgeschlagen).
function nichtAngemeldet(zustand, schluessel, daten) {
  const betroffene = zustand.vorlaeufige.find((vorlaeufig) => vorlaeufig.schluessel === schluessel);
  if (betroffene !== undefined && betroffene.zustand !== 'wird gesendet') {
    return ergebnis(zustand);
  }
  const schonWiederbeigetreten = betroffene === undefined || betroffene.wiederbeigetreten === true;
  if (schonWiederbeigetreten) {
    return fehlschlagen(zustand, schluessel, grundAus(NICHT_ANGEMELDET, daten));
  }
  const vorlaeufige = zustand.vorlaeufige.map((vorlaeufig) => {
    if (vorlaeufig !== betroffene) {
      return vorlaeufig;
    }
    return { ...vorlaeufig, wiederbeigetreten: true };
  });
  return ergebnis({ ...zustand, vorlaeufige }, { auftrag: auftragFuer(betroffene), wiederbeitreten: true });
}

function grundAus(status, daten) {
  if (status === 500) {
    return NICHT_GESPEICHERT;
  }
  const mitText = daten !== null && typeof daten === 'object' && typeof daten.fehler === 'string';
  if (mitText) {
    return daten.fehler;
  }
  return `Nicht gesendet (Status ${status}).`;
}

// Netzfehler oder Zeitgrenze: bei getrenntem Strom bleibt die Nachricht vorläufig und die Warteschlange ruht.
function ohneAntwort(zustand, schluessel) {
  if (zustand.getrennt) {
    return ergebnis({ ...zustand, laufend: null });
  }
  return fehlschlagen(zustand, schluessel, KEINE_ANTWORT);
}

function fehlschlagen(zustand, schluessel, grund) {
  const betroffene = zustand.vorlaeufige.find((vorlaeufig) => vorlaeufig.schluessel === schluessel);
  const vorlaeufige = zustand.vorlaeufige.map((vorlaeufig) => {
    if (vorlaeufig !== betroffene) {
      return vorlaeufig;
    }
    return { ...vorlaeufig, zustand: 'fehlgeschlagen', grund };
  });
  const weiter = naechsterAuftrag({ ...zustand, vorlaeufige });
  if (betroffene === undefined) {
    return weiter;
  }
  return { ...weiter, fehlschlag: { grund } };
}

// „Erneut senden“: ans Ende der Warteschlange mit demselben Schlüssel, wieder mit einem Versuch nach einer 401.
function erneutSenden(zustand, schluessel) {
  const fehlgeschlagene = zustand.vorlaeufige.find((vorlaeufig) => vorlaeufig.schluessel === schluessel);
  if (fehlgeschlagene === undefined) {
    return ergebnis(zustand);
  }
  const wieder = { schluessel, channel: fehlgeschlagene.channel, text: fehlgeschlagene.text, bezug: fehlgeschlagene.bezug, zustand: 'wird gesendet' };
  return auftragWennFrei({ ...zustand, vorlaeufige: [...ohne(zustand.vorlaeufige, schluessel), wieder] });
}

function auftragWennFrei(zustand) {
  if (zustand.laufend !== null) {
    return ergebnis(zustand);
  }
  return naechsterAuftrag(zustand);
}

// Die älteste noch wartende vorläufige wird der nächste Auftrag — außer bei getrenntem Strom, dann ruht die
// Warteschlange.
function naechsterAuftrag(zustand) {
  const naechste = zustand.vorlaeufige.find((vorlaeufig) => vorlaeufig.zustand === 'wird gesendet');
  if (naechste === undefined || zustand.getrennt) {
    return ergebnis({ ...zustand, laufend: null });
  }
  return ergebnis({ ...zustand, laufend: naechste.schluessel }, { auftrag: auftragFuer(naechste) });
}

// Was hinausgeht: Schlüssel, Channel, Text und bei einer Antwort die Zeit des Originals.
function auftragFuer({ schluessel, channel, text, bezug = null }) {
  let antwortAuf = null;
  if (bezug !== null) {
    antwortAuf = bezug.zeit;
  }
  return { schluessel, channel, text, antwortAuf };
}

// Ein Ereignis des Stroms: passt es zur gerade gesendeten vorläufigen, bestätigt es sie; eine schon bestätigte
// Zeit kommt nicht noch einmal; alles andere ist fremd.
function gehoert(zustand, { kennung, channel, nachricht }) {
  if (zustand.bestaetigt.includes(nachricht.zeit)) {
    return ergebnis(zustand);
  }
  const laufende = zustand.vorlaeufige.find((vorlaeufig) => vorlaeufig.schluessel === zustand.laufend);
  const passt = laufende !== undefined
    && laufende.zustand === 'wird gesendet'
    && nachricht.von === kennung
    && laufende.channel === channel
    && wieVomServer(laufende.text) === nachricht.text;
  if (!passt) {
    return ergebnis(zustand, { fremd: { channel, nachricht } });
  }
  const vorlaeufige = zustand.vorlaeufige.filter((vorlaeufig) => vorlaeufig !== laufende);
  const bestaetigung = bestaetigen({ ...zustand, vorlaeufige }, channel, nachricht);
  return ergebnis(bestaetigung.zustand, { bestaetigt: bestaetigung.bestaetigt });
}

function bestaetigen(zustand, channel, nachricht) {
  if (zustand.bestaetigt.includes(nachricht.zeit)) {
    return { zustand, bestaetigt: null };
  }
  return {
    zustand: { ...zustand, bestaetigt: [...zustand.bestaetigt, nachricht.zeit] },
    bestaetigt: { channel, nachricht },
  };
}

function ohne(vorlaeufige, schluessel) {
  return vorlaeufige.filter((vorlaeufig) => vorlaeufig.schluessel !== schluessel);
}

// Wie der Verlauf ihn liefert (ADR-0034 Regel 4): \r\n als \n, abschließende Leerzeilen entfallen.
function wieVomServer(text) {
  return text.replaceAll('\r\n', '\n').replace(/\n+$/, '');
}

function ergebnis(zustand, { auftrag = null, bestaetigt = null, fremd = null, wiederbeitreten = false, fehlschlag = null } = {}) {
  return { zustand, auftrag, bestaetigt, fremd, wiederbeitreten, fehlschlag, bezugAbgewiesen: null };
}

// Wie viele Nachrichten nach dem nächsten Beitritt mit „Erneut senden“ dastehen: die fehlgeschlagenen.
export function ungesendete(zustand) {
  return zustand.vorlaeufige.filter((vorlaeufig) => vorlaeufig.zustand === 'fehlgeschlagen').length;
}
