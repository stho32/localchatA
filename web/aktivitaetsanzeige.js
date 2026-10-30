// Tippanzeige an der Eingabe (R00045 F0161, ADR-0051 Regel 8): wer im geöffneten Channel gerade tippt oder nachdenkt.
// Je Channel eine Tabelle Kennung → Zustand und lokaler Ablauf = Empfang + (bis − jetzt), versatzfrei wie ADR-0019.
// Ein Ende-Ereignis, eine Nachricht derselben Kennung im selben Channel (Regel 7 c) und der Ablauf nehmen einen Eintrag
// heraus; eine Lücke im Signalstrom und jeder Beginn eines Stroms leeren alles — nach einem Abriss wird nichts
// nachgeholt. Die Zeile steht in der Hinweisreihe der Eingabe und wird höchstens einmal je Bild gezeichnet.
//
// ereignis: { art: 'aktivitaet', channel, kennung, zustand: 'tippt' | 'denkt_nach', bis, jetzt }
//         · { art: 'aktivitaet', channel, kennung, zustand: null } · { art: 'aktivitaet_luecke' }
export const AKTIVITAETEN_LEER = {};

export function aktivitaetEinordnen(tabelle, ereignis, empfangenMs) {
  if (ereignis.art === 'aktivitaet_luecke') {
    return AKTIVITAETEN_LEER;
  }
  if (ereignis.art !== 'aktivitaet') {
    throw new Error(`Tippanzeige: unbekanntes Ereignis ${ereignis.art}`);
  }
  const { channel, kennung, zustand } = ereignis;
  if (zustand === null) {
    return ohneEintrag(tabelle, channel, kennung);
  }
  const ablaufMs = empfangenMs + (Date.parse(ereignis.bis) - Date.parse(ereignis.jetzt));
  return { ...tabelle, [channel]: { ...tabelle[channel], [kennung]: { zustand, ablaufMs } } };
}

export function nachrichtEingetroffen(tabelle, { channel, nachricht }) {
  return ohneEintrag(tabelle, channel, nachricht.von);
}

function ohneEintrag(tabelle, channel, kennung) {
  const eintraege = { ...tabelle[channel] };
  delete eintraege[kennung];
  return { ...tabelle, [channel]: eintraege };
}

// Die geltenden Einträge eines Channels, alphabetisch nach Kennung, ohne die eigene: [{ kennung, zustand, ablaufMs }].
export function aktiveIn(tabelle, channel, eigeneKennung, jetztMs) {
  const eintraege = Object.entries(tabelle[channel] || {});
  return eintraege
    .filter(([kennung, eintrag]) => kennung !== eigeneKennung && jetztMs < eintrag.ablaufMs)
    .map(([kennung, eintrag]) => ({ kennung, ...eintrag }))
    .sort((links, rechts) => vergleichen(links.kennung, rechts.kennung));
}

function vergleichen(links, rechts) {
  if (links < rechts) {
    return -1;
  }
  if (links > rechts) {
    return 1;
  }
  return 0;
}

export function anzeigeText(aktive) {
  if (aktive.length === 0) {
    return '';
  }
  const [erster] = aktive;
  return `${erster.kennung.split('@')[0]} ${ZUSTAND_WORT[erster.zustand]} …`;
}

const ZUSTAND_WORT = { tippt: 'tippt', denkt_nach: 'denkt nach' };

// Die Zeile `p.lc-comp-tipp` für die Hinweisreihe der Eingabe; `kennung` ist die eigene.
export function aktivitaetsanzeigeAnlegen({ kennung }) {
  const zeile = document.createElement('p');
  zeile.className = 'lc-comp-tipp';
  let tabelle = AKTIVITAETEN_LEER;
  let channel = null;
  let bildBestellt = false;
  let ablaufWecker = null;

  function zeichnenBestellen() {
    if (bildBestellt) {
      return;
    }
    bildBestellt = true;
    requestAnimationFrame(zeichnen);
  }

  function zeichnen() {
    bildBestellt = false;
    const aktive = aktiveIn(tabelle, channel, kennung, Date.now());
    const text = anzeigeText(aktive);
    if (zeile.textContent !== text) {
      zeile.textContent = text;
    }
    zumAblaufWecken(aktive);
  }

  function zumAblaufWecken(aktive) {
    clearTimeout(ablaufWecker);
    if (aktive.length === 0) {
      return;
    }
    const naechsterAblaufMs = Math.min(...aktive.map((eintrag) => eintrag.ablaufMs));
    ablaufWecker = setTimeout(zeichnenBestellen, Math.max(0, naechsterAblaufMs - Date.now()));
  }

  return {
    zeile,
    ereignis: (ereignis) => {
      tabelle = aktivitaetEinordnen(tabelle, ereignis, Date.now());
      zeichnenBestellen();
    },
    nachricht: (ereignis) => {
      tabelle = nachrichtEingetroffen(tabelle, ereignis);
      zeichnenBestellen();
    },
    leeren: () => {
      tabelle = AKTIVITAETEN_LEER;
      zeichnenBestellen();
    },
    oeffnen: (name) => {
      channel = name;
      zeichnenBestellen();
    },
  };
}
