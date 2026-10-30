// Hochladen unterbrochen (R00030 F0119 B0519, ADR-0046 Frage 5): verlässt der Tab die Seite oder weicht die Hauptansicht
// dem Beitreten-Bildschirm, während Einträge „wird hochgeladen …“ offen sind, merkt er sich je Channel Text und
// Dateinamen in sessionStorage unter `localchatA.unterbrochen.<channel>` — nie den Inhalt, nie eine Anhang-ID, nie das
// Sitzungs-Token (Gate #9 R1). Gelesen wird der Eintrag einmal, wenn sein Channel geöffnet wird; danach ist er weg.
// Ein kaputter oder übergroßer Eintrag wird ohne Fehler übergangen.

// Der Hinweis nach dem Neuladen (UX V8), rein: bis zu 3 Namen, im Hinweis je höchstens 30 Zeichen (die Ellipse zählt
// mit), darüber „und <k> weitere“; die Ansage nennt dieselben Namen ungekürzt. Steht der gesicherte Text über einem
// Entwurf, enden beide mit „Text steht oben im Feld.“ (product-owner Zeile 163).
export function unterbrochenHinweis(namen, textOben) {
  const HOECHSTENS_NAMEN = 3;
  const genannt = namen.slice(0, HOECHSTENS_NAMEN);
  let weitere = '';
  if (namen.length > HOECHSTENS_NAMEN) {
    weitere = ` und ${namen.length - HOECHSTENS_NAMEN} weitere`;
  }
  let schluss = '';
  if (textOben) {
    schluss = ' Text steht oben im Feld.';
  }
  const satz = (liste) => `Hochladen unterbrochen — ${liste.join(', ')}${weitere} bitte erneut wählen.${schluss}`;
  return { hinweis: satz(genannt.map(gekuerzt)), ansage: satz(genannt) };
}

function gekuerzt(name) {
  const HOECHSTENS_ZEICHEN = 30;
  const zeichen = Array.from(name);
  if (zeichen.length <= HOECHSTENS_ZEICHEN) {
    return name;
  }
  return `${zeichen.slice(0, HOECHSTENS_ZEICHEN - 1).join('')}…`;
}

// Was nach dem Neuladen im Feld steht, rein: der gesicherte Text geht nie verloren — im leeren Feld allein, sonst vor dem
// Entwurf, durch eine Leerzeile getrennt; der Entwurf bleibt unverändert dahinter.
export function feldNachNeuladen(gesichert, entwurf) {
  if (gesichert === '') {
    return { text: entwurf, textOben: false };
  }
  if (entwurf === '') {
    return { text: gesichert, textOben: false };
  }
  return { text: `${gesichert}\n\n${entwurf}`, textOben: true };
}

// speicher: sessionStorage-förmig. sichern(offene) mit offene: [{ channel, text, namen }] — weitere Felder werden nie
// geschrieben; ein noch ungelesener Eintrag desselben Channels bleibt erhalten, jeder offene Eintrag liegt in einem
// eigenen Speicherplatz (so bleibt jeder unter der Lesegrenze, auch viele lange Texte), und derselbe Eintrag wird nie
// zweimal gesichert (pagehide und Wechsel zum Beitreten-Bildschirm, bfcache). nehmen(channel) → { text, namen } | null.
export function unterbrochenAnlegen({ speicher }) {
  const gesichert = new Set();

  function sichern(offene) {
    for (const channel of new Set(offene.map((offen) => offen.channel))) {
      const neue = offene.filter((offen) => offen.channel === channel && !gesichert.has(merkmal(offen)));
      if (neue.length === 0) {
        continue;
      }
      const alle = [...lesenAlle(speicher, channel), ...neue.map(({ text, namen }) => ({ text, namen }))];
      entfernenAlle(speicher, channel);
      alle.forEach((eintrag, stelle) => schreiben(speicher, channel, stelle, eintrag));
      neue.forEach((offen) => gesichert.add(merkmal(offen)));
    }
  }

  function nehmen(channel) {
    const gelesen = lesenAlle(speicher, channel);
    entfernenAlle(speicher, channel);
    for (const gemerkt of [...gesichert].filter((eintrag) => eintrag.startsWith(`${JSON.stringify(channel)}|`))) {
      gesichert.delete(gemerkt);
    }
    if (gelesen.length === 0) {
      return null;
    }
    return zusammenfassen(gelesen);
  }

  return { sichern, nehmen };
}

function merkmal({ channel, text, namen }) {
  return `${JSON.stringify(channel)}|${JSON.stringify([text, namen])}`;
}

function zusammenfassen(eintraege) {
  return {
    text: eintraege.map((eintrag) => eintrag.text).filter((text) => text !== '').join('\n\n'),
    namen: eintraege.flatMap((eintrag) => eintrag.namen),
  };
}

function schreiben(speicher, channel, stelle, { text, namen }) {
  try {
    speicher.setItem(schluessel(channel, stelle), JSON.stringify({ text, namen }));
  } catch (ausnahme) {
    console.warn(`localchatA: unterbrochenes Hochladen nicht gemerkt, Neuladen verliert den Text — ${ausnahme.message}`);
  }
}

// Die Speicherplätze eines Channels, vom ersten an lückenlos: `localchatA.unterbrochen.<channel>`, dann `…#1`, `…#2`.
function plaetze(speicher, channel) {
  const gefunden = [];
  for (let stelle = 0; speicher.getItem(schluessel(channel, stelle)) !== null; stelle += 1) {
    gefunden.push(stelle);
  }
  return gefunden;
}

function entfernenAlle(speicher, channel) {
  plaetze(speicher, channel).forEach((stelle) => speicher.removeItem(schluessel(channel, stelle)));
}

// → [{ text, namen }]: ein Platz, der fehlt, größer ist als ein voller Text mit zehn langen Namen, kein JSON oder anders
// geformt ist, gilt als nicht da.
function lesenAlle(speicher, channel) {
  return plaetze(speicher, channel).flatMap((stelle) => lesen(speicher.getItem(schluessel(channel, stelle))));
}

function lesen(roh) {
  const HOECHSTENS_ZEICHEN = 64 * 1024;
  if (roh.length > HOECHSTENS_ZEICHEN) {
    return [];
  }
  let eintrag = null;
  try {
    eintrag = JSON.parse(roh);
  } catch {
    return [];
  }
  if (!wohlgeformt(eintrag)) {
    return [];
  }
  return [{ text: eintrag.text, namen: eintrag.namen }];
}

function wohlgeformt(eintrag) {
  const objekt = eintrag !== null && typeof eintrag === 'object' && !Array.isArray(eintrag);
  if (!objekt || typeof eintrag.text !== 'string' || !Array.isArray(eintrag.namen) || eintrag.namen.length === 0) {
    return false;
  }
  return eintrag.namen.every((name) => typeof name === 'string' && name !== '');
}

function schluessel(channel, stelle) {
  const grundschluessel = `localchatA.unterbrochen.${channel}`;
  if (stelle === 0) {
    return grundschluessel;
  }
  return `${grundschluessel}#${stelle}`;
}
