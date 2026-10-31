'use strict';
// Vergleichshilfe für Tests: die Oberfläche trennt Zahl und Einheit mit U+00A0 (geschütztes Leerzeichen), die Erwartungen
// in den Tests stehen mit normalem Leerzeichen — beide Seiten werden vor dem Vergleich auf das normale Leerzeichen gebracht.
function normal(text) {
  return String(text).replace(/\u00a0/g, ' ');
}

// Wie `normal`, tief durch Objekte und Listen.
function normalTief(wert) {
  if (typeof wert === 'string') {
    return normal(wert);
  }
  if (Array.isArray(wert)) {
    return wert.map(normalTief);
  }
  if (wert !== null && typeof wert === 'object') {
    return Object.fromEntries(Object.entries(wert).map(([schluessel, inhalt]) => [schluessel, normalTief(inhalt)]));
  }
  return wert;
}

// Die Funktion, deren Ergebnis normalisiert zurückkommt — für Unit-Tests, die Texte der Oberfläche mit normalem Leerzeichen erwarten.
function normalisiert(funktion) {
  return (...argumente) => normalTief(funktion(...argumente));
}

module.exports = { normal, normalTief, normalisiert };
