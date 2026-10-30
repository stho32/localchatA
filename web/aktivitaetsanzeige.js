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

// Wortlaut (ux-reviewer A): je Zustand eine Gruppe, `tippt` zuerst, getrennt durch ` · `, am Ende ` …`; eine Person mit
// Namen, zwei mit „und“, ab drei gezählt. Der Name ist der Teil der Kennung vor dem @ und wird halbfett gesetzt.
// → [{ text, name }] (leer, wenn niemand aktiv ist)
export function anzeigeTeile(aktive) {
  const unbekannt = aktive.find((eintrag) => !GRUPPEN.some((gruppe) => gruppe.zustand === eintrag.zustand));
  if (unbekannt !== undefined) {
    throw new Error(`Tippanzeige: unbekannter Zustand ${unbekannt.zustand}`);
  }
  const gruppen = GRUPPEN
    .map((gruppe) => gruppeTeile(gruppe, aktive.filter((eintrag) => eintrag.zustand === gruppe.zustand)))
    .filter((teile) => teile.length > 0);
  if (gruppen.length === 0) {
    return [];
  }
  const verbunden = gruppen.flatMap((teile, stelle) => {
    if (stelle === 0) {
      return teile;
    }
    return [beiwerk(' · '), ...teile];
  });
  return [...verbunden, beiwerk(' …')];
}

export function anzeigeText(aktive) {
  return anzeigeTeile(aktive).map((teil) => teil.text).join('');
}

const GRUPPEN = [
  { zustand: 'tippt', einzahl: 'tippt', mehrzahl: 'tippen' },
  { zustand: 'denkt_nach', einzahl: 'denkt nach', mehrzahl: 'denken nach' },
];

function gruppeTeile({ einzahl, mehrzahl }, mitglieder) {
  const GEZAEHLT_AB = 3;
  if (mitglieder.length >= GEZAEHLT_AB) {
    return [beiwerk(`${mitglieder.length} Teilnehmer ${mehrzahl}`)];
  }
  if (mitglieder.length === 2) {
    return [name(mitglieder[0]), beiwerk(' und '), name(mitglieder[1]), beiwerk(` ${mehrzahl}`)];
  }
  if (mitglieder.length === 1) {
    return [name(mitglieder[0]), beiwerk(` ${einzahl}`)];
  }
  return [];
}

function name({ kennung }) {
  return { text: kennung.split('@')[0], name: true };
}

function beiwerk(text) {
  return { text, name: false };
}

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
    const teile = anzeigeTeile(aktive);
    if (zeile.textContent !== teile.map((teil) => teil.text).join('')) {
      zeile.replaceChildren(...teile.map(teilAlsKnoten));
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

function teilAlsKnoten(teil) {
  if (!teil.name) {
    return document.createTextNode(teil.text);
  }
  const knoten = document.createElement('span');
  knoten.className = 'lc-comp-tipp-name';
  knoten.textContent = teil.text;
  return knoten;
}
