// Auswahl folgen (ADR-0027, R00050 F0178): Adresse und Channel-Liste bestimmen, welches Gespräch Seitenleiste, Kopfzeile
// und Mitte zeigen — Start, hashchange, jede neue Liste und Alt ↓/↑ leiten es neu ab.
import { auswahl, direktAuswahl, auswahlMerken, startadresse, MERKER, nachbarChannel, kuerzelRichtung } from './auswahl.js';
import { direktgespraech } from './gespraech.js';

// Liefert { folgen, listeZeigen(liste) }: die Liste ist erst mit dem ersten Stand bekannt — bis dahin bleibt ein
// gewählter Name ohne Urteil, damit „gibt es nicht“ nie vor dem Stand erscheint. Ein Direktgespräch (`#/d/<kennung>`)
// braucht die Liste nicht: es öffnet sofort, im Privat-Modus (R00050 F0178).
export function auswahlEinrichten({
  seitenleiste, kopf, mitte, verlauf, eingabe, beiAuswahl, beiOeffnen, erneutWaehlen, eigeneKennung,
}, signal) {
  const adresse = startadresse(window.location.hash, window.sessionStorage.getItem(MERKER));
  if (adresse !== null) {
    window.history.replaceState(null, '', adresse);
  }
  let liste = null;
  let offen = null;
  function oeffnen(name) {
    if (offen === name) {
      return;
    }
    offen = name;
    if (verlauf.element.parentNode !== mitte) {
      mitteFuellen(mitte, verlauf.element, eingabe.element);
    }
    beiOeffnen(name);
    verlauf.oeffnen(name);
  }
  function schliessen() {
    offen = null;
    verlaufSchliessen(verlauf, eingabe, seitenleiste);
  }
  function direktFolgen({ gegenueber }) {
    beiAuswahl(null);
    mitte.classList.toggle('lc-privat', gegenueber !== null);
    if (gegenueber === null) {
      seitenleiste.auswahlZeigen(null);
      kopf.ortZeigen(null);
      schliessen();
      mitteFuellen(mitte, ...keinGespraech(rueckwegChannel(liste)));
      return;
    }
    seitenleiste.auswahlZeigen(direktgespraech(gegenueber));
    kopf.direktZeigen(gegenueber);
    oeffnen(direktgespraech(gegenueber));
  }
  function folgen() {
    const direkt = direktAuswahl(window.location.hash, eigeneKennung);
    if (direkt !== null) {
      direktFolgen(direkt);
      return;
    }
    mitte.classList.remove('lc-privat');
    const namen = (liste || []).map((channel) => channel.name);
    const gewaehlt = auswahl(window.location.hash, namen);
    auswahlMerken(window.sessionStorage, gewaehlt);
    let bekannt = null;
    if (gewaehlt !== null && gewaehlt.bekannt) {
      bekannt = liste.find((channel) => channel.name === gewaehlt.name);
    }
    seitenleiste.auswahlZeigen(bekannt && bekannt.name);
    kopf.ortZeigen(bekannt);
    beiAuswahl(bekannt);
    if (bekannt === null) {
      schliessen();
      mitteZeigen(mitte, gewaehlt, liste !== null);
      return;
    }
    oeffnen(bekannt.name);
  }
  window.addEventListener('hashchange', folgen, { signal });
  // Die Wahl des schon gewählten Gesprächs ändert die Adresse nicht, es kommt kein hashchange (R00007, US-13).
  seitenleiste.leiste.addEventListener('click', (ereignis) => {
    const verweis = ereignis.target.closest('a[data-gespraech]');
    if (verweis !== null && verweis.dataset.gespraech === offen) {
      erneutWaehlen();
    }
  }, { signal });
  // Alt ↓ / Alt ↑ wählt wie ein Klick über die Adresse; der Fokus bleibt, wo er ist (ADR-0027 Regel 4).
  document.addEventListener('keydown', (ereignis) => {
    const richtung = kuerzelRichtung(ereignis);
    const dialogOffen = document.querySelector('dialog[open]') !== null;
    if (richtung === null || dialogOffen || liste === null) {
      return;
    }
    ereignis.preventDefault();
    const namen = liste.map((channel) => channel.name);
    const nachbar = nachbarChannel(namen, offen, richtung);
    if (nachbar !== null) {
      fokusRettenInEingabe(verlauf, eingabe);
      window.location.hash = `#/c/${nachbar}`;
    }
  }, { signal });
  folgen();
  return {
    folgen,
    listeZeigen: (neueListe) => {
      liste = neueListe;
      seitenleiste.channelsZeigen(neueListe);
      folgen();
    },
  };
}

// Der Channelwechsel ersetzt die Nachrichten des Verlaufs: liegt der Fokus auf einer von ihnen (oder auf body), geht er
// vorher in die Eingabe, damit er nicht ins Nichts fällt (Karte #82). Auf dem Verlauf selbst, in der Eingabe und
// außerhalb bleibt er, wo er ist (ADR-0027 Regel 4).
function fokusRettenInEingabe(verlauf, eingabe) {
  const aktiv = document.activeElement;
  const aufNachricht = aktiv !== null && aktiv.closest('.lc-msg') !== null && verlauf.element.contains(aktiv);
  if ((aufNachricht || aktiv === document.body) && eingabe.element.isConnected) {
    eingabe.fokussieren();
  }
}

// Liegt der Fokus im Verlauf oder in der Eingabe, die jetzt gehen, kommt er auf den Schalter „Channel anlegen“ —
// nie ins Nichts.
function verlaufSchliessen(verlauf, eingabe, seitenleiste) {
  const fokusImVerlauf = verlauf.element.contains(document.activeElement) || eingabe.element.contains(document.activeElement);
  verlauf.schliessen();
  verlauf.element.remove();
  eingabe.element.remove();
  if (fokusImVerlauf) {
    seitenleiste.plus.focus();
  }
}

function mitteZeigen(mitte, gewaehlt, listeBekannt) {
  if (gewaehlt === null) {
    mitteFuellen(mitte, element('p', { className: 'lc-mitte-hinweis', textContent: 'Links einen Channel wählen.' }));
    return;
  }
  if (!listeBekannt) {
    mitteFuellen(mitte);
    return;
  }
  mitteFuellen(mitte, element('p', { className: 'lc-mitte-hinweis', textContent: `# ${gewaehlt.name} gibt es nicht.` }));
}

// Der Channel, zu dem der Rückweg führt: der zuletzt gewählte des Tabs, sonst der erste der Liste, sonst keiner.
function rueckwegChannel(liste) {
  const gemerkt = window.sessionStorage.getItem(MERKER);
  if (gemerkt !== null) {
    return gemerkt;
  }
  if (liste !== null && liste.length > 0) {
    return liste[0].name;
  }
  return null;
}

// Eine Adresse, die zu keinem Gespräch führt (eigene Kennung, ungültige Form; R00050 V14): der Satz, ein Rückweg und
// eine gesperrte Eingabe — an niemanden geht etwas.
function keinGespraech(channel) {
  let rueckweg = element('a', { className: 'lc-link', href: '#', textContent: 'Zurück zur Übersicht' });
  if (channel !== null) {
    rueckweg = element('a', { className: 'lc-link', href: `#/c/${channel}`, textContent: `Zurück zu # ${channel}` });
  }
  const feld = element('textarea', { className: 'lc-comp-feld', rows: 1, disabled: true, placeholder: 'Kein Gespräch' });
  feld.setAttribute('aria-label', 'Nachricht — diese Adresse führt zu keinem Gespräch');
  const gesperrt = element('div', { className: 'lc-comp lc-comp-gesperrt' }, [element('div', { className: 'lc-comp-box' }, [feld])]);
  return [
    element('p', { className: 'lc-mitte-hinweis', textContent: 'Diese Adresse führt zu keinem Gespräch.' }),
    element('p', { className: 'lc-mitte-hinweis' }, [rueckweg]),
    gesperrt,
  ];
}

// Ersetzt den Inhalt der Mitte; die Leiste der Verbindung bleibt (sie gehört der Hauptansicht, nicht dem Channel).
function mitteFuellen(mitte, ...kinder) {
  const leiste = [...mitte.children].filter((kind) => kind.classList.contains('lc-leiste'));
  mitte.replaceChildren(...leiste, ...kinder);
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
