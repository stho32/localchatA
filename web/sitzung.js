// Zustand beim Laden und nach dem Beitreten wählen (ADR-0012 „Browser“): eine Sitzung je Browser,
// Token und zuletzt gewählter Name im localStorage des Ursprungs, alle Tabs teilen sie.
//
// Zustände: kopfzeile {teilnehmer, token} · beitreten {name} · vergeben {name, belegtVon, seit, vorschlag}
// · ungueltig {name, fehler, regel, vorschlag} · serverWeg {name} · fehler {name, fehler}

export const SITZUNG = 'localchatA.sitzung';
export const NAME = 'localchatA.name';

const KEINE_GESCHWISTER = { neuesToken: async () => null };

export async function beimLaden(speicher, api, geschwister = KEINE_GESCHWISTER) {
  const token = speicher.getItem(SITZUNG);
  const gemerkterName = speicher.getItem(NAME) || '';
  if (token) {
    const ich = await api.ich(token);
    if (ich.serverWeg) {
      return { art: 'serverWeg', name: gemerkterName };
    }
    if (ich.status === 200) {
      return { art: 'kopfzeile', teilnehmer: ich.daten, token };
    }
    if (ich.status !== 401) {
      return { art: 'fehler', name: gemerkterName, fehler: fehlertext(ich) };
    }
    tokenVergessen(speicher, token);
  }
  if (gemerkterName) {
    return stillBeitreten(gemerkterName, speicher, api, geschwister);
  }
  return { art: 'beitreten', name: '' };
}

// Nur das eigene, abgelehnte Token: hat ein anderer Tab inzwischen ein neues geschrieben, bleibt es stehen.
function tokenVergessen(speicher, abgelehntesToken) {
  if (speicher.getItem(SITZUNG) === abgelehntesToken) {
    speicher.removeItem(SITZUNG);
  }
}

// Ein 409 auf den eigenen gemerkten Namen kann von einem anderen Tab desselben Browsers stammen, der gerade
// zuerst beigetreten ist: dessen Token ist dann die gemeinsame Sitzung.
async function stillBeitreten(name, speicher, api, geschwister, abgelehntesToken = null) {
  const zustand = await beitreten(name, speicher, api);
  if (zustand.art !== 'vergeben') {
    return zustand;
  }
  if (abgelehntesToken !== null) {
    tokenVergessen(speicher, abgelehntesToken);
  }
  let tokenEinesAnderenTabs = speicher.getItem(SITZUNG);
  if (!tokenEinesAnderenTabs) {
    tokenEinesAnderenTabs = await geschwister.neuesToken();
  }
  if (!tokenEinesAnderenTabs) {
    return zustand;
  }
  const ich = await api.ich(tokenEinesAnderenTabs);
  if (ich.serverWeg || ich.status !== 200) {
    return zustand;
  }
  return { art: 'kopfzeile', teilnehmer: ich.daten, token: tokenEinesAnderenTabs };
}

// Mit dem gemerkten Token: lebt die Sitzung noch (nach einem Fehler oder Server weg beim Laden), ist der
// Beitritt eine Wiederkehr statt eines 409 auf den eigenen Namen.
export async function beitreten(name, speicher, api) {
  const antwort = await api.beitreten(name, speicher.getItem(SITZUNG));
  if (antwort.serverWeg) {
    return { art: 'serverWeg', name };
  }
  const daten = antwort.daten || {};
  switch (antwort.status) {
    case 200:
    case 201:
      sitzungMerken(speicher, daten);
      return { art: 'kopfzeile', teilnehmer: teilnehmerAus(daten), token: daten.token };
    case 409:
      return { art: 'vergeben', name, belegtVon: daten.belegt_von, seit: daten.seit, vorschlag: daten.vorschlag };
    case 422:
      return { art: 'ungueltig', name, fehler: daten.fehler, regel: daten.regel, vorschlag: daten.vorschlag || '' };
    default:
      return { art: 'fehler', name, fehler: fehlertext(antwort) };
  }
}

function fehlertext(antwort) {
  if (antwort.daten && antwort.daten.fehler) {
    return antwort.daten.fehler;
  }
  return `Unerwartete Antwort ${antwort.status}`;
}

// Der Beitritt gilt auch, wenn der Browser nichts speichern lässt (Speicher voll, gesperrt) — nur das
// Wiederfinden beim Neuladen fehlt dann.
function sitzungMerken(speicher, daten) {
  try {
    speicher.setItem(SITZUNG, daten.token);
    speicher.setItem(NAME, daten.name);
  } catch (ausnahme) {
    console.warn(`localchatA: Sitzung nicht gemerkt, Neuladen fragt erneut nach dem Namen — ${ausnahme.message}`);
  }
}

function teilnehmerAus(daten) {
  return { kennung: daten.kennung, name: daten.name, herkunft: daten.herkunft, art: daten.art };
}

// Nach einem 401 aus Strom oder Senden (Neustart, Gnadenfrist; R00025 F0088 ein Weg): ohne GET /api/ich wieder
// beitreten — dessen 401 (Vertrag aus R00003) stünde sonst als Konsolenfehler. Hat ein anderer Tab schon ein neues
// Token gemerkt, gilt es; sonst still beitreten wie beim Laden, mit dem abgelehnten Token im Kopf: lebt die Sitzung
// doch, ist es eine Wiederkehr (200), sonst ein neuer Beitritt. Ein 409 kann vom Nachbar-Tab stammen, dessen Token
// übernommen wird (Nacharbeit 3, B2); gelingt nichts, ist das abgelehnte Token vergessen.
export async function nachAblehnung(speicher, api, abgelehntesToken, geschwister = KEINE_GESCHWISTER) {
  const token = speicher.getItem(SITZUNG);
  if (token && token !== abgelehntesToken) {
    return { art: 'kopfzeile', teilnehmer: null, token };
  }
  const gemerkterName = speicher.getItem(NAME) || '';
  if (!gemerkterName) {
    return { art: 'beitreten', name: '' };
  }
  const zustand = await stillBeitreten(gemerkterName, speicher, api, geschwister, abgelehntesToken);
  if (zustand.art !== 'kopfzeile') {
    tokenVergessen(speicher, abgelehntesToken);
  }
  return zustand;
}
