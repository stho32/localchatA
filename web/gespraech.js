// Ein Gespräch der Oberfläche (ADR-0055 Regel 3, A00005 Ergänzung Karte #13): ein Channel heißt wie er, ein
// Direktgespräch `direkt:<Kennung des Gegenübers>`. Ein Channelname enthält nie `:`, darum verwechselt kein Schlüssel
// die beiden. Unter diesem Schlüssel führen Verlauf, Eingabe, Entwurf, Zustellung und Tippanzeige jedes Gespräch;
// die Instanz kennt ihn nicht — Pfade und Ereignisse werden hier übersetzt.

const DIREKT = 'direkt:';

export function direktgespraech(kennung) {
  return DIREKT + kennung;
}

// → Kennung des Gegenübers | null (ein Channel)
export function gegenueberIn(schluessel) {
  if (!schluessel.startsWith(DIREKT)) {
    return null;
  }
  return schluessel.slice(DIREKT.length);
}

// Antworten und Ereignisse der Instanz nennen ein Direktgespräch mit `channel: null` und `direkt: <Gegenüber>`
// (ADR-0055 Regel 6); danach trägt `channel` den Schlüssel. Alles andere bleibt, wie es ist.
export function mitGespraechsschluessel(daten) {
  if (daten === null || typeof daten.direkt !== 'string') {
    return daten;
  }
  return { ...daten, channel: direktgespraech(daten.direkt) };
}

export function apiPfad(schluessel) {
  const gegenueber = gegenueberIn(schluessel);
  if (gegenueber === null) {
    return `/api/channels/${encodeURIComponent(schluessel)}`;
  }
  return `/api/direkt/${encodeURIComponent(gegenueber)}`;
}

// Derselbe Pfad, wie ein Mensch ihn liest und abschreibt (Leerzustand, R00050 V9) — nicht prozentkodiert.
export function lesbarerApiPfad(schluessel) {
  const gegenueber = gegenueberIn(schluessel);
  if (gegenueber === null) {
    return `/api/channels/${schluessel}`;
  }
  return `/api/direkt/${gegenueber}`;
}

// Die Adresse eines Direktgesprächs im Tab (ADR-0055 Regel 3): `#/d/` und die Kennung prozentkodiert.
export const DIREKT_ADRESSE = '#/d/';

export function direktHash(kennung) {
  return DIREKT_ADRESSE + encodeURIComponent(kennung);
}

// Die Adresse eines Gesprächs: `#/c/<name>` bzw. `#/d/<kennung>`.
export function adresseVon(schluessel) {
  const gegenueber = gegenueberIn(schluessel);
  if (gegenueber === null) {
    return `#/c/${schluessel}`;
  }
  return direktHash(gegenueber);
}

// Wo man ist: „# name“ bzw. „Direktnachricht mit <Kennung>“ — das Wort trägt den Privat-Modus, nicht nur die Farbe (V7).
export function ortText(schluessel) {
  const gegenueber = gegenueberIn(schluessel);
  if (gegenueber === null) {
    return `# ${schluessel}`;
  }
  return `Direktnachricht mit ${gegenueber}`;
}

// An wen man schreibt: „# name“ bzw. die Kennung des Gegenübers.
export function anredeText(schluessel) {
  const gegenueber = gegenueberIn(schluessel);
  if (gegenueber === null) {
    return `# ${schluessel}`;
  }
  return gegenueber;
}
