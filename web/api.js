// API aufrufen (A00005): fetch mit Sitzungs-Header (ADR-0012) und JSON. Jede Antwort hat dieselbe Form
// { serverWeg, status, daten }: Fehlerstatus sind Antworten, nur ein unerreichbarer Server ist serverWeg.

export const SITZUNGS_HEADER = 'X-LocalchatA-Sitzung';

export async function rufen(pfad, { koerper, token, methode = 'POST' } = {}, holen = holenImBrowser) {
  const headers = {};
  if (token) {
    headers[SITZUNGS_HEADER] = token;
  }
  const optionen = { method: 'GET', headers };
  if (koerper !== undefined) {
    optionen.method = methode;
    headers['Content-Type'] = 'application/json';
    optionen.body = JSON.stringify(koerper);
  }
  try {
    const antwort = await holen(pfad, optionen);
    const text = await antwort.text();
    return { serverWeg: false, status: antwort.status, daten: alsJson(text) };
  } catch (ausnahme) {
    return { serverWeg: true, status: 0, daten: null };
  }
}

// Frist setzen (ADR-0031): PUT {tage}, im zweiten Schritt mit bestaetigt: true → { serverWeg, status, daten }.
export function fristSetzen(name, tage, bestaetigt, { token }, holen = holenImBrowser) {
  const koerper = { tage };
  if (bestaetigt) {
    koerper.bestaetigt = true;
  }
  return rufen(`/api/channels/${encodeURIComponent(name)}/frist`, { koerper, token, methode: 'PUT' }, holen);
}

function holenImBrowser(pfad, optionen) {
  return globalThis.fetch(pfad, optionen);
}

function alsJson(text) {
  try {
    return JSON.parse(text);
  } catch (ausnahme) {
    return null;
  }
}

// Ein Block des Verlaufs (ADR-0025): cursor { vor } | { nach } | {} — kodiert mit encodeURIComponent, damit das
// + im Versatz ankommt; n Nachrichten je Block (Standard 100, Nachholen 200). → { art: 'block', block } | { art: 'fehler', status } (0: Server weg) | { art: 'abgebrochen' }
export async function verlaufAbrufen(name, cursor, { token, signal, n = 100 }, holen = holenImBrowser) {
  const abfrage = Object.entries(cursor).map(([schluessel, zeit]) => `${schluessel}=${encodeURIComponent(zeit)}&`).join('');
  const pfad = `/api/channels/${encodeURIComponent(name)}/nachrichten?${abfrage}n=${n}`;
  try {
    const antwort = await holen(pfad, { method: 'GET', headers: { [SITZUNGS_HEADER]: token }, signal });
    if (!antwort.ok) {
      return { art: 'fehler', status: antwort.status };
    }
    return { art: 'block', block: await antwort.json() };
  } catch (ausnahme) {
    if (signal !== undefined && signal.aborted) {
      return { art: 'abgebrochen' };
    }
    return { art: 'fehler', status: 0 };
  }
}

// Ein Block beim Nachholen (ADR-0037 Ergänzung Karte #22/#36, 7a): wie verlaufAbrufen, aber 10 s nach Beginn
// abgebrochen wie das Senden — ein hängender Abruf hält Lücke und Warteschlange nicht fest. signal: Abbruch durch den
// Aufrufer (Trennung, Channelwechsel). → { art: 'block', block } | { art: 'fehler', status } | { art: 'zeitueberschreitung' }
// | { art: 'abgebrochen' }
export async function verlaufNachholen(name, cursor, { token, signal, n, zeitgeber = ECHTE_ZEITGEBER }, holen = holenImBrowser) {
  const ZEITGRENZE_MS = 10000;
  const abbruch = new AbortController();
  const abbrechen = () => abbruch.abort();
  const zeitgrenze = zeitgeber.stellen(ZEITGRENZE_MS, abbrechen);
  signal.addEventListener('abort', abbrechen);
  try {
    const ergebnis = await verlaufAbrufen(name, cursor, { token, signal: abbruch.signal, n }, holen);
    const zeitUeberschritten = ergebnis.art === 'abgebrochen' && !signal.aborted;
    if (zeitUeberschritten) {
      return { art: 'zeitueberschreitung' };
    }
    return ergebnis;
  } finally {
    zeitgeber.loeschen(zeitgrenze);
    signal.removeEventListener('abort', abbrechen);
  }
}

// Eine Nachricht senden (ADR-0034, ADR-0037 Regeln 2, 4): POST {text} mit Sitzung und Wiederholschlüssel; ohne Antwort
// bricht der Abruf 10 s nach Beginn ab. zeitgeber: { stellen(ms, rueckruf) → id, loeschen(id) }.
// → { art: 'antwort', status, daten } | { art: 'zeitueberschreitung' } | { art: 'netzfehler' }
export async function nachrichtSenden(channel, text, schluessel, { token, zeitgeber = ECHTE_ZEITGEBER }, holen = holenImBrowser) {
  const ZEITGRENZE_MS = 10000;
  const pfad = `/api/channels/${encodeURIComponent(channel)}/nachrichten`;
  const headers = { [SITZUNGS_HEADER]: token, 'Content-Type': 'application/json', 'Idempotency-Key': schluessel };
  const abbruch = new AbortController();
  const zeitgrenze = zeitgeber.stellen(ZEITGRENZE_MS, () => abbruch.abort());
  try {
    const antwort = await holen(pfad, { method: 'POST', headers, body: JSON.stringify({ text }), signal: abbruch.signal });
    const inhalt = await antwort.text();
    return { art: 'antwort', status: antwort.status, daten: alsJson(inhalt) };
  } catch (ausnahme) {
    if (abbruch.signal.aborted) {
      return { art: 'zeitueberschreitung' };
    }
    return { art: 'netzfehler' };
  } finally {
    zeitgeber.loeschen(zeitgrenze);
  }
}

const ECHTE_ZEITGEBER = {
  stellen: (ms, rueckruf) => setTimeout(rueckruf, ms),
  loeschen: (id) => clearTimeout(id),
};
