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

// Nachgeholtes und live Angenommenes über die Zeit zusammenführen (ADR-0037 Ergänzung Karte #22/#36, 7f): jede Zeit
// (als Zeitpunkt) genau einmal, in der Folge der Zeiten — auch Nachrichten, die älter sind als eine schon gezeigte.
// bekannt: Folge nach Zeit → { folge, neue } (neue: was aus `dazu` hinzukam, nach Zeit)
export function zusammenfuehren(bekannt, dazu) {
  const bekannteZeitpunkte = new Set(bekannt.map((nachricht) => Date.parse(nachricht.zeit)));
  const neueNachZeitpunkt = new Map();
  for (const nachricht of dazu) {
    const zeitpunkt = Date.parse(nachricht.zeit);
    if (!bekannteZeitpunkte.has(zeitpunkt)) {
      neueNachZeitpunkt.set(zeitpunkt, nachricht);
    }
  }
  const neue = nachZeitpunkt([...neueNachZeitpunkt.values()]);
  return { folge: nachZeitpunkt([...bekannt, ...neue]), neue };
}

function nachZeitpunkt(nachrichten) {
  return nachrichten.sort((links, rechts) => Date.parse(links.zeit) - Date.parse(rechts.zeit));
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

// Der Hinweis „Verlauf nicht geladen“ (R00037 Regel 1, ADR-0037 Ergänzung Karte #65) sagt, was geschieht: lädt der Tab
// von selbst (Strom getrennt oder ein Laden-Lauf läuft), wartet der Mensch; sonst hilft nur die neue Wahl des Channels.
export function nichtGeladenText(selbstversuchend) {
  if (selbstversuchend) {
    return '⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.';
  }
  return '⚠ Verlauf nicht geladen — Channel erneut wählen, um es noch einmal zu versuchen.';
}

// Zieht den Text des stehenden Hinweises nach, wenn sich der Zustand geändert hat — nur der Text desselben Knotens
// (role="alert" bleibt), und nur bei Unterschied, damit der Bildschirmleser nichts doppelt ansagt. → der Knoten
export function hinweisTextNachziehen(knoten, selbstversuchend) {
  const text = nichtGeladenText(selbstversuchend);
  if (knoten.textContent !== text) {
    knoten.textContent = text;
  }
  return knoten;
}
