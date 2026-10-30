// Eigene Nachricht löschen (R00041 F0139, ADR-0048 Regel 12): vor jedem Löschen genau eine Rückfrage, die bei Anhängen
// Zahl und Größe der mitgelöschten nennt (UX S1, PO 95).
import { uhrzeit } from './gliederung.js';

// Die Texte der Rückfrage, rein: Frage (mit Zahl und Größe der Anhänge im Zustand `vorhanden`), darunter Uhrzeit und
// Textanfang (40 Zeichen, eine Zeile, längerer Text mit „…“; ohne Text nur die Uhrzeit), zuletzt die Warnung.
// → { frage, auszug, warnung }
export function rueckfrageText({ zeit, text, anhaenge = [] }) {
  return { frage: frageText(anhaenge), auszug: auszugText(zeit, text), warnung: 'Das lässt sich nicht rückgängig machen.' };
}

function frageText(anhaenge) {
  const vorhanden = anhaenge.filter((anhang) => anhang.zustand !== 'geloescht');
  const groesse = groesseInWorten(vorhanden.reduce((summe, anhang) => summe + anhang.bytes, 0));
  if (vorhanden.length === 0) {
    return 'Diese Nachricht löschen?';
  }
  if (vorhanden.length === 1) {
    return `Diese Nachricht mit 1 Anhang (${groesse}) löschen? Der Anhang wird mitgelöscht.`;
  }
  return `Diese Nachricht mit ${vorhanden.length} Anhängen (${groesse}) löschen? Die Anhänge werden mitgelöscht.`;
}

function auszugText(zeit, text) {
  const TEXTANFANG_ZEICHEN = 40;
  const zeichen = Array.from(text.replace(/\s+/gu, ' ').trim());
  if (zeichen.length === 0) {
    return uhrzeit(zeit);
  }
  let anfang = zeichen.join('');
  if (zeichen.length > TEXTANFANG_ZEICHEN) {
    anfang = `${zeichen.slice(0, TEXTANFANG_ZEICHEN).join('')}…`;
  }
  return `${uhrzeit(zeit)}: ${anfang}`;
}

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

function mitZehnteln(bytes, einheit, name) {
  const zehntel = Math.floor((bytes * 20 + einheit) / (2 * einheit));
  return `${tausender(Math.floor(zehntel / 10))},${zehntel % 10} ${name}`;
}

function tausender(wert) {
  return new Intl.NumberFormat('de-DE').format(wert);
}
