// Zustellung im Tab (ADR-0037 Regeln 1–4), rein: vorläufige Nachrichten aller Channels als eine Warteschlange, ein
// Sendeauftrag zugleich; bestätigt wird über die Antwort oder das eigene Ereignis, wer zuerst kommt. Bei getrenntem
// Strom ruht die Warteschlange; was dann scheitert, bleibt „wird gesendet“ und geht nach dem Wiederverbinden mit
// demselben Schlüssel hinaus.
//
// zustand: { vorlaeufige: [{ schluessel, channel, text, zustand: 'wird gesendet' | 'fehlgeschlagen', grund?,
//            wiederbeigetreten? }], laufend: schluessel | null, bestaetigt: [zeit…], getrennt }
// ereignis: { art: 'senden', channel, text, schluessel } · { art: 'antwort', schluessel, status, daten }
//         · { art: 'netzfehler' | 'zeitueberschreitung', schluessel } · { art: 'ereignis', kennung, channel, nachricht }
//         · { art: 'getrennt' | 'verbunden' } · { art: 'erneut' | 'verwerfen', schluessel }
// → { zustand, auftrag: { schluessel, channel, text } | null, bestaetigt: { channel, nachricht } | null,
//     fremd: { channel, nachricht } | null, wiederbeitreten, fehlschlag: { grund } | null }
export const LEER = { vorlaeufige: [], laufend: null, bestaetigt: [], getrennt: false };

const NICHT_GESPEICHERT = 'Nicht gespeichert — der Server konnte nicht schreiben.';
const KEINE_ANTWORT = 'Keine Antwort vom Server.';
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
      return ergebnis({ ...zustand, vorlaeufige: ohne(zustand.vorlaeufige, ereignis.schluessel) });
    default:
      throw new Error(`Zustellung: unbekanntes Ereignis ${ereignis.art}`);
  }
}

function gesendet(zustand, { channel, text, schluessel }) {
  const vorlaeufige = [...zustand.vorlaeufige, { schluessel, channel, text, zustand: 'wird gesendet' }];
  return auftragWennFrei({ ...zustand, vorlaeufige });
}

function beantwortet(zustand, { schluessel, status, daten }) {
  if (status === NICHT_ANGEMELDET) {
    return nichtAngemeldet(zustand, schluessel, daten);
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

// Die erste 401 eines Versuchs: still wieder beitreten und genau einmal mit demselben Schlüssel neu senden.
function nichtAngemeldet(zustand, schluessel, daten) {
  const betroffene = zustand.vorlaeufige.find((vorlaeufig) => vorlaeufig.schluessel === schluessel);
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
  const auftrag = { schluessel, channel: betroffene.channel, text: betroffene.text };
  return ergebnis({ ...zustand, vorlaeufige }, { auftrag, wiederbeitreten: true });
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
  const wieder = { schluessel, channel: fehlgeschlagene.channel, text: fehlgeschlagene.text, zustand: 'wird gesendet' };
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
  const auftrag = { schluessel: naechste.schluessel, channel: naechste.channel, text: naechste.text };
  return ergebnis({ ...zustand, laufend: naechste.schluessel }, { auftrag });
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
  return { zustand, auftrag, bestaetigt, fremd, wiederbeitreten, fehlschlag };
}
