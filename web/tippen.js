// Tippmeldung des Browsers (R00045 F0161, ADR-0051 Regel 8), rein mit gestellter Zeit: eine Eingabe in das Feld des
// Gesprächs, nach der es nicht leer ist, meldet `tippt` sofort, danach höchstens alle 3 s; die Meldung gilt am Server
// 6 s. Wird das Feld leer oder wechselt der Channel, während eine Meldung gilt, wird sie gelöscht. Nach dem Senden
// beendet der Server selbst (Regel 7 b). Gemeldet wird nur auf eine Eingabe — Entwurf, Fokus und gesetzter Feldwert
// sind keine (ux-reviewer F). Ohne Strom wird nicht gemeldet: der Server beendet den Zustand eines Menschen, der
// offline ist, ohnehin (Regel 7 e), und eine Sitzung, die er nicht mehr kennt, antwortete nur 401.
//
// ereignis: { art: 'eingabe', channel, text } · { art: 'gesendet', channel } · { art: 'gewechselt', channel } (der
//           bisherige) · { art: 'gebremst', wartenS } (429 mit Retry-After) · { art: 'verbindung', verbunden }
// → { zustand, aufruf: null | { art: 'setzen' | 'loeschen', channel } }
import { nurLeerraum } from './textregel.js';

export const TIPPEN_LEER = { channel: null, gemeldetMs: null, gesperrtBisMs: 0, verbunden: true };

const MELDEN_HOECHSTENS_ALLE_MS = 3000;
const MELDUNG_GILT_MS = 6000;

export function tippen(zustand, ereignis, jetztMs) {
  switch (ereignis.art) {
    case 'eingabe':
      return eingabeMelden(zustand, ereignis, jetztMs);
    case 'gewechselt':
      return meldungBeenden(zustand, ereignis.channel, jetztMs);
    case 'gesendet':
      return { zustand: ohneMeldung(zustand), aufruf: null };
    case 'gebremst':
      return { zustand: { ...zustand, gesperrtBisMs: jetztMs + ereignis.wartenS * 1000 }, aufruf: null };
    case 'verbindung':
      return { zustand: { ...ohneMeldung(zustand), verbunden: ereignis.verbunden }, aufruf: null };
    default:
      throw new Error(`Tippen: unbekanntes Ereignis ${ereignis.art}`);
  }
}

function eingabeMelden(zustand, { channel, text }, jetztMs) {
  if (!zustand.verbunden) {
    return { zustand, aufruf: null };
  }
  if (nurLeerraum(text)) {
    return meldungBeenden(zustand, channel, jetztMs);
  }
  const gebremst = jetztMs < zustand.gesperrtBisMs;
  const kuerzlichGemeldet = zustand.channel === channel && jetztMs - zustand.gemeldetMs < MELDEN_HOECHSTENS_ALLE_MS;
  if (gebremst || kuerzlichGemeldet) {
    return { zustand, aufruf: null };
  }
  return { zustand: { ...zustand, channel, gemeldetMs: jetztMs }, aufruf: { art: 'setzen', channel } };
}

function meldungBeenden(zustand, channel, jetztMs) {
  const giltNoch = zustand.verbunden && zustand.channel === channel && jetztMs < zustand.gemeldetMs + MELDUNG_GILT_MS;
  if (!giltNoch) {
    return { zustand: ohneMeldung(zustand), aufruf: null };
  }
  return { zustand: ohneMeldung(zustand), aufruf: { art: 'loeschen', channel } };
}

function ohneMeldung(zustand) {
  return { ...zustand, channel: null, gemeldetMs: null };
}

// Die Verdrahtung für die Eingabe: hält den Zustand und schickt jeden Aufruf über `melden`
// (async { art, channel } → { art: 'antwort', status, wartenS } | { art: 'netzfehler' }). Ein 429 sperrt bis
// Retry-After; jeder andere Fehler bleibt ohne Hinweis — die Anzeige ist Beiwerk (ADR-0051 Regel 8).
export function tippenAnlegen({ melden, jetzt = () => Date.now() }) {
  const ZU_VIELE_ANFRAGEN = 429;
  let zustand = TIPPEN_LEER;

  function anwenden(ereignis) {
    const ergebnis = tippen(zustand, ereignis, jetzt());
    zustand = ergebnis.zustand;
    if (ergebnis.aufruf !== null) {
      ausfuehren(ergebnis.aufruf);
    }
  }

  async function ausfuehren(aufruf) {
    const antwort = await melden(aufruf);
    if (antwort.art === 'antwort' && antwort.status === ZU_VIELE_ANFRAGEN) {
      anwenden({ art: 'gebremst', wartenS: antwort.wartenS });
    }
  }

  return {
    eingabe: (channel, text) => anwenden({ art: 'eingabe', channel, text }),
    gesendet: (channel) => anwenden({ art: 'gesendet', channel }),
    gewechselt: (channel) => anwenden({ art: 'gewechselt', channel }),
    verbindung: (verbunden) => anwenden({ art: 'verbindung', verbunden }),
  };
}
