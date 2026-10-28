// Wächter der Live-Verbindung (B0087, ADR-0015 Regel 4; Vorbild kanbanD web/verbindung.js): rein.
// Der Server schickt alle 15 s einen Herzschlag; wer 40 s kein Byte hört, hat eine still abgerissene
// Verbindung. Getrennt wird neu verbunden: sofort, dann nach 1 s, 2 s, 5 s und danach alle 10 s.

const STILLE_MS = 40000;
const RUECKZUG_MS = [0, 1000, 2000, 5000, 10000];

export function entscheiden({ verbunden, letzteAktivitaetMs, jetztMs, fehlversuche }) {
  if (!verbunden) {
    return { aktion: 'neu', wartenMs: rueckzugInMs(fehlversuche) };
  }
  const stilleZuLang = jetztMs - letzteAktivitaetMs > STILLE_MS;
  if (stilleZuLang) {
    return { aktion: 'neu', wartenMs: 0 };
  }
  return { aktion: 'weiter', wartenMs: 0 };
}

function rueckzugInMs(fehlversuche) {
  const stufe = Math.min(fehlversuche, RUECKZUG_MS.length - 1);
  return RUECKZUG_MS[stufe];
}
