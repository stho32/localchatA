// Zustand beim Laden und nach dem Beitreten wählen (ADR-0012 „Browser“): eine Sitzung je Browser,
// Token und zuletzt gewählter Name im localStorage des Ursprungs, alle Tabs teilen sie.
//
// Zustände: kopfzeile {teilnehmer} · beitreten {name} · vergeben {name, belegtVon, seit, vorschlag}
// · ungueltig {name, fehler, vorschlag} · serverWeg {name} · fehler {name, fehler}

export const SITZUNG = 'localchatA.sitzung';
export const NAME = 'localchatA.name';

export async function beimLaden(speicher, api) {
  const token = speicher.getItem(SITZUNG);
  const gemerkterName = speicher.getItem(NAME) || '';
  if (token) {
    const ich = await api.ich(token);
    if (ich.serverWeg) {
      return { art: 'serverWeg', name: gemerkterName };
    }
    if (ich.status === 200) {
      return { art: 'kopfzeile', teilnehmer: ich.daten };
    }
    speicher.removeItem(SITZUNG);
  }
  if (gemerkterName) {
    return beitreten(gemerkterName, speicher, api);
  }
  return { art: 'beitreten', name: '' };
}

export async function beitreten(name, speicher, api) {
  const antwort = await api.beitreten(name);
  if (antwort.serverWeg) {
    return { art: 'serverWeg', name };
  }
  const daten = antwort.daten || {};
  switch (antwort.status) {
    case 200:
    case 201:
      speicher.setItem(SITZUNG, daten.token);
      speicher.setItem(NAME, daten.name);
      return { art: 'kopfzeile', teilnehmer: teilnehmerAus(daten) };
    case 409:
      return { art: 'vergeben', name, belegtVon: daten.belegt_von, seit: daten.seit, vorschlag: daten.vorschlag };
    case 422:
      return { art: 'ungueltig', name, fehler: daten.fehler, vorschlag: daten.vorschlag || '' };
    default:
      return { art: 'fehler', name, fehler: daten.fehler || `Unerwartete Antwort ${antwort.status}` };
  }
}

function teilnehmerAus(daten) {
  return { kennung: daten.kennung, name: daten.name, herkunft: daten.herkunft, art: daten.art };
}
