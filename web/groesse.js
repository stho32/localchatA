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
    return `${tausender(bytes)} Byte`;
  }
  const kib = Math.max(1, Math.round(bytes / KIB));
  if (bytes < MIB && kib < KIB) {
    return `${tausender(kib)} KiB`;
  }
  if (bytes < GIB) {
    return mitZehnteln(bytes, MIB, 'MiB');
  }
  return mitZehnteln(bytes, GIB, 'GiB');
}

// Die Größe genau, in Byte — für den Fall, dass zwei verschiedene Größen in Worten gleich lauten (Rundung).
export function groesseInByte(bytes) {
  return `${tausender(bytes)} Byte`;
}

function mitZehnteln(bytes, einheit, name) {
  const zehntel = Math.floor((bytes * 20 + einheit) / (2 * einheit));
  return `${tausender(Math.floor(zehntel / 10))},${zehntel % 10} ${name}`;
}

function tausender(wert) {
  return new Intl.NumberFormat('de-DE').format(wert);
}
