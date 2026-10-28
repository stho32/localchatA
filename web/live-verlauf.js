// Fremde Nachrichten im offenen Verlauf (ADR-0037 Regel 6, ADR-0028 Regeln 5, 7), rein.

// Übernimmt in Folge nur Nachrichten, deren Zeit (als Zeitpunkt) jünger ist als die jüngste bekannte; kleinere und
// gleiche fallen weg. juengste: Zeit (RFC 3339) oder null → { neue, juengste }.
export function einordnen(juengste, nachrichten) {
  const neue = [];
  let bisher = juengste;
  for (const nachricht of nachrichten) {
    const juenger = bisher === null || Date.parse(nachricht.zeit) > Date.parse(bisher);
    if (juenger) {
      neue.push(nachricht);
      bisher = nachricht.zeit;
    }
  }
  return { neue, juengste: bisher };
}

// Sammelt angenommene Nachrichten und fügt sie einmal je Frame ein (ADR-0037 Regel 6).
export function buendlerAnlegen({ einfuegen, naechsterFrame = (rueckruf) => globalThis.requestAnimationFrame(rueckruf) }) {
  let gesammelt = [];
  function frame() {
    const jetzt = gesammelt;
    gesammelt = [];
    einfuegen(jetzt);
  }
  return {
    annehmen: (nachricht) => {
      if (gesammelt.length === 0) {
        naechsterFrame(frame);
      }
      gesammelt.push(nachricht);
    },
  };
}

// Wohin eine neue fremde Nachricht kommt: steht der neueste Block nicht im Fenster, wird nur gezählt; steht der Blick
// höchstens eine Zeile vor dem Ende, folgt er; sonst wird angefügt und gezählt, nichts scrollt.
// → 'folgen' | 'zaehlen' | 'nurZaehlen'
export function folgenOderZaehlen(abstandUnten, zeilenhoehe, neuesterImFenster) {
  if (!neuesterImFenster) {
    return 'nurZaehlen';
  }
  if (abstandUnten <= zeilenhoehe) {
    return 'folgen';
  }
  return 'zaehlen';
}

export function pillenText(anzahl) {
  if (anzahl === 1) {
    return '1 neue Nachricht';
  }
  return `${anzahl} neue Nachrichten`;
}

// Wie hoch der Auslauf unter dem Verlauf danach ist (Nacharbeit #8): angefügter Inhalt nimmt ihn ein; steht er ganz
// unter dem Fenster, fällt er weg; solange ein Teil sichtbar ist, verschwindet er nie auf einmal — sonst rutscht die
// Lesestelle nach unten. hoehe, sichtbar, angefuegt in px → neue Höhe in px
export function auslaufNach({ hoehe, sichtbar, angefuegt }) {
  if (sichtbar <= 0) {
    return 0;
  }
  const genommen = Math.max(0, angefuegt);
  return Math.max(0, hoehe - genommen);
}
