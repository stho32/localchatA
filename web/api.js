// API aufrufen (A00005): fetch mit Sitzungs-Header (ADR-0012) und JSON. Jede Antwort hat dieselbe Form
// { serverWeg, status, daten }: Fehlerstatus sind Antworten, nur ein unerreichbarer Server ist serverWeg. Ein Gespräch
// heißt mit seinem Schlüssel (gespraech.js): ein Channel oder ein Direktgespräch.
import { apiPfad } from './gespraech.js';

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

// Tippen melden (ADR-0051 Regel 4): setzen ist PUT {"zustand":"tippt"}, loeschen DELETE ohne Körper.
// Der Körper der Antwort wird gelesen, sonst hält der Browser die Verbindung fest.
// → { art: 'antwort', status, wartenS } (wartenS aus Retry-After nur bei 429, sonst null) | { art: 'netzfehler' }
export async function aktivitaetMelden({ art, channel }, { token }, holen = holenImBrowser) {
  const ZU_VIELE_ANFRAGEN = 429;
  const optionen = { method: 'DELETE', headers: { [SITZUNGS_HEADER]: token } };
  if (art === 'setzen') {
    optionen.method = 'PUT';
    optionen.headers['Content-Type'] = 'application/json';
    optionen.body = JSON.stringify({ zustand: 'tippt' });
  }
  let antwort;
  try {
    antwort = await holen(`${apiPfad(channel)}/aktivitaet`, optionen);
    await antwort.text();
  } catch (ausnahme) {
    return { art: 'netzfehler' };
  }
  let wartenS = null;
  if (antwort.status === ZU_VIELE_ANFRAGEN) {
    wartenS = sekundenAusRetryAfter(antwort.headers.get('Retry-After'));
  }
  return { art: 'antwort', status: antwort.status, wartenS };
}

// Einen Anhang anmelden (R00029 F0097, ADR-0046 Regel 1): POST {name, bytes} mit Sitzung. Der Körper der Antwort wird
// gelesen; Retry-After zählt bei 429 und 507 (`hochladungen`).
// → { art: 'antwort', status, daten, wartenS } | { art: 'netzfehler' }
export async function anhangAnmelden(name, bytes, { token }, holen = holenImBrowser) {
  const ZU_VIELE_ANFRAGEN = 429;
  const KEIN_PLATZ = 507;
  const headers = { [SITZUNGS_HEADER]: token, 'Content-Type': 'application/json' };
  let antwort;
  let inhalt;
  try {
    antwort = await holen('/api/anhaenge', { method: 'POST', headers, body: JSON.stringify({ name, bytes }) });
    inhalt = await antwort.text();
  } catch (ausnahme) {
    return { art: 'netzfehler' };
  }
  let wartenS = null;
  if (antwort.status === ZU_VIELE_ANFRAGEN || antwort.status === KEIN_PLATZ) {
    wartenS = sekundenAusRetryAfter(antwort.headers.get('Retry-After'));
  }
  return { art: 'antwort', status: antwort.status, daten: alsJson(inhalt), wartenS };
}

// Eine Anmeldung verwerfen (ADR-0046): DELETE mit Sitzung; die Antwort zählt nicht — die Anmeldung verfällt sonst am
// Server von selbst. → { art: 'antwort', status } | { art: 'netzfehler' }
export async function anhangVerwerfen(id, { token }, holen = holenImBrowser) {
  try {
    const antwort = await holen(`/api/anhaenge/${encodeURIComponent(id)}`, { method: 'DELETE', headers: { [SITZUNGS_HEADER]: token } });
    await antwort.text();
    return { art: 'antwort', status: antwort.status };
  } catch (ausnahme) {
    return { art: 'netzfehler' };
  }
}

// Eine angemeldete Datei hochladen (R00029 F0097, ADR-0046 Regel 2): PUT als application/octet-stream mit Sitzung über
// XMLHttpRequest — nur er meldet den Fortschritt des Hochladens. fortschritt(geladen, gesamt) je Ereignis.
// → { abbrechen(), ergebnis: Promise<{ art: 'antwort', status, daten } | { art: 'netzfehler' } | { art: 'abgebrochen' }> }
export function anhangHochladen(id, datei, { token, fortschritt }, Anfrage = globalThis.XMLHttpRequest) {
  const anfrage = new Anfrage();
  const ergebnis = new Promise((fertig) => {
    anfrage.upload.addEventListener('progress', (ereignis) => fortschritt(ereignis.loaded, ereignis.total));
    anfrage.addEventListener('load', () => fertig({ art: 'antwort', status: anfrage.status, daten: alsJson(anfrage.responseText) }));
    anfrage.addEventListener('error', () => fertig({ art: 'netzfehler' }));
    anfrage.addEventListener('timeout', () => fertig({ art: 'netzfehler' }));
    anfrage.addEventListener('abort', () => fertig({ art: 'abgebrochen' }));
  });
  anfrage.open('PUT', `/api/anhaenge/${encodeURIComponent(id)}`);
  anfrage.setRequestHeader(SITZUNGS_HEADER, token);
  anfrage.setRequestHeader('Content-Type', 'application/octet-stream');
  anfrage.send(datei);
  return { abbrechen: () => anfrage.abort(), ergebnis };
}

function sekundenAusRetryAfter(kopf) {
  const sekunden = Number(kopf);
  if (kopf === null || !Number.isInteger(sekunden) || sekunden < 1) {
    return 1;
  }
  return sekunden;
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
  const pfad = `${apiPfad(name)}/nachrichten?${abfrage}n=${n}`;
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

// Eine Nachricht senden (ADR-0034, ADR-0037 Regeln 2, 4): POST {text} mit Sitzung und Wiederholschlüssel, eine Antwort
// mit `antwort_auf` (Zeit des Originals, R00047 B0715), mit Dateien `anhaenge` (IDs, R00030 B0453); ohne Antwort bricht
// der Abruf 10 s nach Beginn ab.
// zeitgeber: { stellen(ms, rueckruf) → id, loeschen(id) }.
// → { art: 'antwort', status, daten } | { art: 'zeitueberschreitung' } | { art: 'netzfehler' }
export async function nachrichtSenden(channel, text, schluessel, {
  token, antwortAuf = null, anhaenge = [], zeitgeber = ECHTE_ZEITGEBER,
}, holen = holenImBrowser) {
  const ZEITGRENZE_MS = 10000;
  const pfad = `${apiPfad(channel)}/nachrichten`;
  const headers = { [SITZUNGS_HEADER]: token, 'Content-Type': 'application/json', 'Idempotency-Key': schluessel };
  const abbruch = new AbortController();
  const zeitgrenze = zeitgeber.stellen(ZEITGRENZE_MS, () => abbruch.abort());
  try {
    const koerper = { text };
    if (antwortAuf !== null) {
      koerper.antwort_auf = antwortAuf;
    }
    if (anhaenge.length > 0) {
      koerper.anhaenge = anhaenge;
    }
    const antwort = await holen(pfad, { method: 'POST', headers, body: JSON.stringify(koerper), signal: abbruch.signal });
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

// Eine eigene Nachricht bearbeiten (R00041 F0135, ADR-0048 Regel 5): POST {zeit, text} mit Sitzung und einem
// Wiederholschlüssel je Speichern-Versuch; ohne Antwort Abbruch nach 10 s.
// → { art: 'antwort', status, daten } | { art: 'zeitueberschreitung' } | { art: 'netzfehler' }
export function nachrichtBearbeiten(channel, zeit, text, schluessel, { token, zeitgeber = ECHTE_ZEITGEBER }, holen = holenImBrowser) {
  const pfad = `${apiPfad(channel)}/nachrichten/bearbeiten`;
  return aendernSchicken(pfad, { zeit, text }, { token, schluessel, zeitgeber }, holen);
}

// Eine eigene Nachricht löschen (R00041 F0139, ADR-0048 Regeln 5, 12): POST {zeit} in einem Schritt, ohne
// Wiederholschlüssel (Löschen ist von sich aus wiederholbar). → wie nachrichtBearbeiten
export function nachrichtLoeschen(channel, zeit, { token, zeitgeber = ECHTE_ZEITGEBER }, holen = holenImBrowser) {
  const pfad = `${apiPfad(channel)}/nachrichten/loeschen`;
  return aendernSchicken(pfad, { zeit }, { token, schluessel: null, zeitgeber }, holen);
}

async function aendernSchicken(pfad, koerper, { token, schluessel, zeitgeber }, holen) {
  const ZEITGRENZE_MS = 10000;
  const headers = { [SITZUNGS_HEADER]: token, 'Content-Type': 'application/json' };
  if (schluessel !== null) {
    headers['Idempotency-Key'] = schluessel;
  }
  const abbruch = new AbortController();
  const zeitgrenze = zeitgeber.stellen(ZEITGRENZE_MS, () => abbruch.abort());
  try {
    const antwort = await holen(pfad, { method: 'POST', headers, body: JSON.stringify(koerper), signal: abbruch.signal });
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
