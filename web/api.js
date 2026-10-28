// API aufrufen (A00005): fetch mit Sitzungs-Header (ADR-0012) und JSON. Jede Antwort hat dieselbe Form
// { serverWeg, status, daten }: Fehlerstatus sind Antworten, nur ein unerreichbarer Server ist serverWeg.

export const SITZUNGS_HEADER = 'X-LocalchatA-Sitzung';

export async function rufen(pfad, { koerper, token } = {}, holen = holenImBrowser) {
  const headers = {};
  if (token) {
    headers[SITZUNGS_HEADER] = token;
  }
  const optionen = { method: 'GET', headers };
  if (koerper !== undefined) {
    optionen.method = 'POST';
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
// + im Versatz ankommt. → { art: 'block', block } | { art: 'fehler', status } (0: Server weg) | { art: 'abgebrochen' }
export async function verlaufAbrufen(name, cursor, { token, signal }, holen = holenImBrowser) {
  const BLOCKGROESSE = 100;
  const abfrage = Object.entries(cursor).map(([schluessel, zeit]) => `${schluessel}=${encodeURIComponent(zeit)}&`).join('');
  const pfad = `/api/channels/${encodeURIComponent(name)}/nachrichten?${abfrage}n=${BLOCKGROESSE}`;
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
