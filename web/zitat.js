// Der Bezug einer Antwort im Tab (R00047 I0044, ADR-0053 Regeln 5–7), rein. Ein Bezug hat die Form, die die Instanz in
// `antwort_auf` liefert: { zeit, von, zustand: 'da' | 'bearbeitet' | 'geloescht' | 'entfallen', textanfang, anhang, geaendert }.
import { uhrzeit } from './gliederung.js';

// Der Bezug der Antwortzeile aus der Nachricht des Fensters, auf die geantwortet wird: bis zur Bestätigung zeigt die Zeile
// den Text des Originals (gekürzt per CSS, kein Textanfang im Tab gerechnet); `anhang` nennt nur bei leerem Text den
// ersten vorhandenen Anhang.
export function bezugAusNachricht({ zeit, von, text = '', zustand, geaendert = null, anhaenge = [] }) {
  let zustandDesOriginals = 'da';
  if (zustand === 'bearbeitet') {
    zustandDesOriginals = 'bearbeitet';
  }
  let anhang = null;
  const ersterAnhang = anhaenge.find((einzeln) => einzeln.zustand !== 'geloescht');
  if (text === '' && ersterAnhang !== undefined) {
    anhang = ersterAnhang.name;
  }
  return { zeit, von, zustand: zustandDesOriginals, textanfang: text, anhang, geaendert };
}

// Was eine Antwortzeile oder ein Zitat zu einem Bezug zeigt (Wortlaute der Vorprüfung `ux-reviewer`).
export function zitatText({ zustand, textanfang, anhang }) {
  switch (zustand) {
    case 'geloescht':
      return 'Nachricht gelöscht';
    case 'entfallen':
      return 'Ältere Nachricht, nicht mehr vorhanden';
    case 'da':
    case 'bearbeitet':
      return textOderAnhang(textanfang, anhang);
    default:
      throw new Error(`Zitat: unbekannter Zustand ${zustand}`);
  }
}

function textOderAnhang(textanfang, anhang) {
  if (textanfang === '' && anhang !== null) {
    return `Anhang: ${anhang}`;
  }
  return textanfang;
}

// Der zugängliche Name des Zitats (Vorprüfung `ux-reviewer`): „Zur Nachricht springen: <Kennung> <HH:MM>, <was es zeigt>“.
export function zitatName(bezug) {
  return `Zur Nachricht springen: ${bezug.von} ${uhrzeit(bezug.zeit)}, ${zitatText(bezug)}`;
}

// Der Teil im Namen einer Antwort, der das Original nennt (Z1): „Antwort auf <Kennung> <HH:MM>“, bei gelöschtem Original
// mit „, Nachricht gelöscht“, bei entfallenem „Antwort auf ältere Nachricht, nicht mehr vorhanden“.
export function antwortAufText(bezug) {
  if (bezug.zustand === 'entfallen') {
    return 'Antwort auf ältere Nachricht, nicht mehr vorhanden';
  }
  const original = `Antwort auf ${bezug.von} ${uhrzeit(bezug.zeit)}`;
  if (bezug.zustand === 'geloescht') {
    return `${original}, Nachricht gelöscht`;
  }
  return original;
}
