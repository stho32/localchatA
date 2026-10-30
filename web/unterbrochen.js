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
// geschrieben; ein noch ungelesener Eintrag desselben Channels geht im neuen auf. nehmen(channel) → { text, namen } | null.
export function unterbrochenAnlegen({ speicher }) {
  function sichern(offene) {
    for (const channel of new Set(offene.map((offen) => offen.channel))) {
      const frueher = lesen(speicher, channel);
      const jetzt = offene.filter((offen) => offen.channel === channel);
      schreiben(speicher, channel, zusammenfassen([...frueher, ...jetzt]));
    }
  }

  function nehmen(channel) {
    const gelesen = lesen(speicher, channel);
    speicher.removeItem(schluessel(channel));
    if (gelesen.length === 0) {
      return null;
    }
    return gelesen[0];
  }

  return { sichern, nehmen };
}

function zusammenfassen(eintraege) {
  return {
    text: eintraege.map((eintrag) => eintrag.text).filter((text) => text !== '').join('\n\n'),
    namen: eintraege.flatMap((eintrag) => eintrag.namen),
  };
}

function schreiben(speicher, channel, { text, namen }) {
  try {
    speicher.setItem(schluessel(channel), JSON.stringify({ text, namen }));
  } catch (ausnahme) {
    console.warn(`localchatA: unterbrochenes Hochladen nicht gemerkt, Neuladen verliert den Text — ${ausnahme.message}`);
  }
}

// → [] oder [{ text, namen }]: fehlt der Eintrag, ist er größer als ein voller Text mit zehn langen Namen, kein JSON
// oder anders geformt, gilt er als nicht da.
function lesen(speicher, channel) {
  const HOECHSTENS_ZEICHEN = 64 * 1024;
  const roh = speicher.getItem(schluessel(channel));
  if (roh === null || roh.length > HOECHSTENS_ZEICHEN) {
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

function schluessel(channel) {
  return `localchatA.unterbrochen.${channel}`;
}
