// Größe einer Datei in Worten — gemeinsam für die Liste der gewählten Dateien, die Einträge „wird hochgeladen …“, die
// Datei-Zeile im Verlauf und die Rückfrage vor dem Löschen.

// Größe in Worten (R00030 „Größe in Worten“, wie der Server ADR-0044 Regel 4), rein: unter 1.024 Bytes „<n> Byte“; unter
// 1 MiB „<n> KiB“ ganz und kaufmännisch gerundet (mindestens 1; ergibt die Rundung 1.024 KiB, gilt „1,0 MiB“); darüber
// MiB bzw. ab 1 GiB GiB mit einer Nachkommastelle und Komma.
export function groesseInWorten(bytes) {
  const KIB = 1024;
  const MIB = KIB * KIB;
  const GIB = KIB * MIB;
  if (bytes < KIB) {
    return zahlMitEinheit(tausender(bytes), 'Byte');
  }
  const kib = Math.max(1, Math.round(bytes / KIB));
  if (bytes < MIB && kib < KIB) {
    return zahlMitEinheit(tausender(kib), 'KiB');
  }
  if (bytes < GIB) {
    return mitZehnteln(bytes, MIB, 'MiB');
  }
  return mitZehnteln(bytes, GIB, 'GiB');
}

// Zahl und Einheit der Oberfläche, getrennt durch ein geschütztes Leerzeichen (U+00A0): „64 KiB“ bricht nie zwischen den
// beiden Wörtern, eine Zeile endet nie mit einer nackten Zahl. Nur die Oberfläche tut das — Texte der Instanz (API) und
// der Agenten behalten das normale Leerzeichen, damit sie sich durchsuchen lassen.
export function zahlMitEinheit(zahl, einheit) {
  return `${zahl}\u00a0${einheit}`;
}

// Die Größe genau, in Byte — für den Fall, dass zwei verschiedene Größen in Worten gleich lauten (Rundung).
export function groesseInByte(bytes) {
  return zahlMitEinheit(tausender(bytes), 'Byte');
}

function mitZehnteln(bytes, einheit, name) {
  const zehntel = Math.floor((bytes * 20 + einheit) / (2 * einheit));
  return zahlMitEinheit(`${tausender(Math.floor(zehntel / 10))},${zehntel % 10}`, name);
}

function tausender(wert) {
  return new Intl.NumberFormat('de-DE').format(wert);
}
