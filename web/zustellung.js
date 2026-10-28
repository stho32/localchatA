// Zustellung im Tab (ADR-0037 Regeln 1–4), rein: vorläufige Nachrichten aller Channels als eine Warteschlange, ein
// Sendeauftrag zugleich; bestätigt wird über die Antwort oder das eigene Ereignis, wer zuerst kommt.
//
// zustand: { vorlaeufige: [{ schluessel, channel, text, zustand: 'wird gesendet' | 'fehlgeschlagen', grund? }],
//            laufend: schluessel | null, bestaetigt: [zeit…] }
// ereignis: { art: 'senden', channel, text, schluessel } · { art: 'antwort', schluessel, status, daten }
//         · { art: 'netzfehler', schluessel } · { art: 'ereignis', kennung, channel, nachricht }
// → { zustand, auftrag: { schluessel, channel, text } | null, bestaetigt: { channel, nachricht } | null,
//     fremd: { channel, nachricht } | null }
export const LEER = { vorlaeufige: [], laufend: null, bestaetigt: [] };

const NICHT_GESPEICHERT = 'Nicht gespeichert — der Server konnte nicht schreiben.';
const KEINE_ANTWORT = 'Keine Antwort vom Server.';

export function zustellen(zustand, ereignis) {
  if (ereignis.art === 'senden') {
    return gesendet(zustand, ereignis);
  }
  if (ereignis.art === 'antwort') {
    return beantwortet(zustand, ereignis);
  }
  if (ereignis.art === 'netzfehler') {
    return naechsterAuftrag(fehlschlagen(zustand, ereignis.schluessel, KEINE_ANTWORT));
  }
  if (ereignis.art === 'ereignis') {
    return gehoert(zustand, ereignis);
  }
  throw new Error(`Zustellung: unbekanntes Ereignis ${ereignis.art}`);
}

function gesendet(zustand, { channel, text, schluessel }) {
  const vorlaeufige = [...zustand.vorlaeufige, { schluessel, channel, text, zustand: 'wird gesendet' }];
  if (zustand.laufend !== null) {
    return ergebnis({ ...zustand, vorlaeufige });
  }
  return ergebnis({ ...zustand, vorlaeufige, laufend: schluessel }, { auftrag: { schluessel, channel, text } });
}

function beantwortet(zustand, { schluessel, status, daten }) {
  const erfolgreich = status === 200 || status === 201;
  if (!erfolgreich) {
    return naechsterAuftrag(fehlschlagen(zustand, schluessel, grundAus(status, daten)));
  }
  const vorlaeufige = zustand.vorlaeufige.filter((vorlaeufig) => vorlaeufig.schluessel !== schluessel);
  const bestaetigung = bestaetigen({ ...zustand, vorlaeufige }, daten.channel, daten.nachricht);
  const weiter = naechsterAuftrag(bestaetigung.zustand);
  return { ...weiter, bestaetigt: bestaetigung.bestaetigt };
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

function fehlschlagen(zustand, schluessel, grund) {
  const vorlaeufige = zustand.vorlaeufige.map((vorlaeufig) => {
    if (vorlaeufig.schluessel !== schluessel) {
      return vorlaeufig;
    }
    return { ...vorlaeufig, zustand: 'fehlgeschlagen', grund };
  });
  return { ...zustand, vorlaeufige };
}

// Der laufende Auftrag ist beantwortet: die älteste noch wartende vorläufige wird der nächste.
function naechsterAuftrag(zustand) {
  const naechste = zustand.vorlaeufige.find((vorlaeufig) => vorlaeufig.zustand === 'wird gesendet');
  if (naechste === undefined) {
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

// Wie der Verlauf ihn liefert (ADR-0034 Regel 4): \r\n als \n, abschließende Leerzeilen entfallen.
function wieVomServer(text) {
  return text.replaceAll('\r\n', '\n').replace(/\n+$/, '');
}

function ergebnis(zustand, { auftrag = null, bestaetigt = null, fremd = null } = {}) {
  return { zustand, auftrag, bestaetigt, fremd };
}
