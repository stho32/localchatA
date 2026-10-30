// Fremde Nachrichten im offenen Verlauf (ADR-0037 Regel 6, ADR-0028 Regeln 5, 7), rein.
import { gegenueberIn } from './gespraech.js';

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
// höchstens eine Zeile vor dem Ende, folgt er — außer solange ein Medium spielt (R00030 B0733, U1); sonst wird angefügt
// und gezählt, nichts scrollt. → 'folgen' | 'zaehlen' | 'nurZaehlen'
export function folgenOderZaehlen(abstandUnten, zeilenhoehe, neuesterImFenster, mediumSpielt) {
  if (!neuesterImFenster) {
    return 'nurZaehlen';
  }
  if (abstandUnten <= zeilenhoehe && !mediumSpielt) {
    return 'folgen';
  }
  return 'zaehlen';
}

// Wohin der Sprung zur ersten neuen scrollt (R00020 F0067, R00026 F0089): die Unterkante der Nachricht davor — ohne
// sie die Oberkante des Platzes der ersten neuen — kommt eine Zeilenhöhe unter den klebenden Trenner. Alle Maße in px
// vom oberen Rand des Verlaufs; unterkanteKontext: null, wenn keine Nachricht davor steht.
// → Scrollstand in px
export function sprungZiel({ scrollTop, oberkanteErste, unterkanteKontext, trennerUnterkante, zeilenhoehe }) {
  const kante = unterkanteKontext === null ? oberkanteErste : unterkanteKontext;
  return scrollTop + kante - trennerUnterkante - zeilenhoehe;
}

// Wohin scrollTop nach einer Änderung der Sichthöhe des Verlaufs geht (R00048 F0164): der Abstand der Lesestelle zur
// Unterkante bleibt, die Rechnung gilt für Wachsen und Schrumpfen gleich. Das Ergebnis liegt in 0 … Inhalt − Sicht.
// Alle Maße in px. → neuer scrollTop in px
export function scrollTopNachHoehe({ scrollTop, hoeheVorher, hoeheNachher, inhaltsHoehe }) {
  const hoechstens = Math.max(0, inhaltsHoehe - hoeheNachher);
  const nachgefuehrt = scrollTop + hoeheVorher - hoeheNachher;
  return Math.min(hoechstens, Math.max(0, nachgefuehrt));
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
// von selbst (Strom getrennt oder ein Laden-Lauf läuft), wartet der Mensch; sonst hilft nur die neue Wahl des Gesprächs —
// eines Channels oder der Direktnachricht (schluessel aus gespraech.js).
export function nichtGeladenText(selbstversuchend, schluessel = '') {
  if (selbstversuchend) {
    return '⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.';
  }
  let gespraech = 'Channel';
  if (gegenueberIn(schluessel) !== null) {
    gespraech = 'Direktnachricht';
  }
  return `⚠ Verlauf nicht geladen — ${gespraech} erneut wählen, um es noch einmal zu versuchen.`;
}

// Zieht den Text des stehenden Hinweises nach, wenn sich der Zustand geändert hat — nur der Text desselben Knotens
// (role="alert" bleibt), und nur bei Unterschied, damit der Bildschirmleser nichts doppelt ansagt. → der Knoten
export function hinweisTextNachziehen(knoten, selbstversuchend, schluessel = '') {
  const text = nichtGeladenText(selbstversuchend, schluessel);
  if (knoten.textContent !== text) {
    knoten.textContent = text;
  }
  return knoten;
}

// Eine geänderte Fassung einer Nachricht übernehmen (R00041 F0134, ADR-0048 Regel 11) — Ausnahme zu ADR-0028 Regel 5
// nur über `geaendert`: die neue gilt, wenn ihr `geaendert` größer ist als das bekannte (`null` ist kleiner als jede
// Zeit); gelöscht wird nie zurückgenommen; eine Nachricht, die nicht im Fenster steht (bekannt fehlt), bringt das
// nächste Laden. → die neue Fassung oder null
export function fassungUebernehmen(bekannt, neu) {
  if (bekannt === undefined || bekannt === null || bekannt.zustand === 'geloescht' || neu.geaendert === null) {
    return null;
  }
  if (bekannt.geaendert === null || bekannt.geaendert === undefined) {
    return neu;
  }
  if (Date.parse(neu.geaendert) > Date.parse(bekannt.geaendert)) {
    return neu;
  }
  return null;
}

// Nach einer Lücke das Fenster neu lesen (ADR-0048 Regel 11, R00041 F0134): vom Ende her mit `vor` (exklusiv) eine
// Millisekunde hinter der jüngsten Nachricht des Fensters, Block für Block bis zu seinem Anfang — auch acht Blöcke und
// einen live gewachsenen jüngsten —, und jede gelesene Nachricht an `uebernehmen`. Vom Ende her, weil dort Änderungen am
// wahrscheinlichsten sind; `nach` bleibt dem Nachholen neuer Nachrichten. Jeder Abruf muss vor den vorigen reichen; ein
// Fehlschlag, ein Block ohne Fortschritt oder `gilt()` falsch (Wechsel des Channels) beendet das Lesen still.
// abrufen: async (cursor) → Ergebnis wie api.js verlaufAbrufen · anfang, juengste: Zeiten des Fensters (RFC 3339)
export async function fensterNeuLesen({ abrufen, anfang, juengste, uebernehmen, gilt }) {
  let vor = zeitDanach(juengste);
  let anfangGelesen = false;
  while (!anfangGelesen) {
    const ergebnis = await abrufen({ vor });
    if (!gilt() || ergebnis.art !== 'block' || ergebnis.block.nachrichten.length === 0) {
      return;
    }
    const gelesen = ergebnis.block.nachrichten;
    const ohneFortschritt = Date.parse(gelesen[0].zeit) >= Date.parse(vor);
    if (ohneFortschritt) {
      return;
    }
    gelesen.forEach(uebernehmen);
    vor = gelesen[0].zeit;
    anfangGelesen = !ergebnis.block.aelter || Date.parse(vor) <= Date.parse(anfang);
  }
}

// Eine Millisekunde nach `zeit`, als RFC 3339 in UTC — der Cursor `vor` ist exklusiv (ADR-0025).
export function zeitDanach(zeit) {
  return new Date(Date.parse(zeit) + 1).toISOString().replace('Z', '+00:00');
}

// Eine Millisekunde vor `zeit`, als RFC 3339 in UTC — der Cursor `nach` ist exklusiv: der Block ab ihm beginnt mit der
// Nachricht dieser Zeit (Sprung zum Original, R00047 B0718, ADR-0053 Regel 8 b).
export function zeitDavor(zeit) {
  return new Date(Date.parse(zeit) - 1).toISOString().replace('Z', '+00:00');
}

// Ein gelöschter Anhang ändert nur seinen `zustand` (R00030 B0518, ADR-0047 Ergänzung „F1′“ Regel 3: Name, Größe, Art und
// Adresse bleiben); die Nachricht behält ihre Fassung. ids: Set der Anhang-IDs. → die Nachricht mit den Anhängen als
// `geloescht` — dieselbe, wenn keiner der Anhänge zu ändern ist
export function anhaengeAlsGeloescht(nachricht, ids) {
  const anhaenge = anhaengeVon(nachricht);
  const betroffen = anhaenge.some((anhang) => ids.has(anhang.id) && anhang.zustand !== 'geloescht');
  if (!betroffen) {
    return nachricht;
  }
  const neue = anhaenge.map((anhang) => {
    if (ids.has(anhang.id)) {
      return { ...anhang, zustand: 'geloescht' };
    }
    return anhang;
  });
  return { ...nachricht, anhaenge: neue };
}

// Die Anhänge einer Nachricht, die schon `geloescht` sind, als Einträge des Ereignisses `anhaenge`. → [{ id, zeit }]
export function geloeschteAnhaenge(nachricht) {
  return anhaengeVon(nachricht).filter((anhang) => anhang.zustand === 'geloescht').map((anhang) => ({ id: anhang.id, zeit: nachricht.zeit }));
}

function anhaengeVon(nachricht) {
  if (nachricht.anhaenge === undefined) {
    return [];
  }
  return nachricht.anhaenge;
}
