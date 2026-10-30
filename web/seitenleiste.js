// Seitenleiste (B0135, ADR-0024 Regeln 1–3, Designsystem components/Seitenleiste; R00050 F0180 V8): zwei `nav` —
// „Channels“ mit dem Schalter „+“, darunter je Channel `# name` nach Name, leer „Noch keine Channels.“; danach
// „Direktnachrichten“ ohne „+“, immer da, je Gespräch Statuspunkt, volle Kennung und offline/weg als Wort, alphabetisch
// nach Kennung (direktliste.js), leer zwei Sätze. Unter 720 px zeigt ein Channel nur `#`, ein Direktgespräch Punkt und
// Kürzel; der Name bleibt Tooltip und zugänglicher Name. Jeder Eintrag ist ein Link, der gewählte trägt
// aria-current="page" auf kd-accent (ADR-0027 Regel 2). Keine Zähler, kein @n, kein Marker (PO O20).
import { direktgespraech, direktHash } from './gespraech.js';

export function anlegen() {
  const plus = element('button', { type: 'button', className: 'lc-side-plus', textContent: '+' });
  plus.setAttribute('aria-label', 'Channel anlegen');
  plus.setAttribute('aria-keyshortcuts', 'Control+Alt+N');
  const channelEintraege = element('div', { className: 'lc-side-eintraege' });
  const channelNav = abschnitt('Channels', [plus], channelEintraege);
  const direktEintraege = element('div', { className: 'lc-side-eintraege' });
  const direktNav = abschnitt('Direktnachrichten', [], direktEintraege);
  const leiste = element('div', { className: 'lc-side' }, [channelNav, direktNav]);
  let channels = [];
  let direkt = null;
  let gewaehlt = null;
  return {
    leiste,
    plus,
    channelsZeigen: (neueChannels) => {
      channels = neueChannels;
      channelsZeichnen(channelEintraege, channels, gewaehlt);
    },
    // eintraege: [direkteintrag] | null (noch nicht geladen — dann bleibt der Abschnitt ohne Inhalt)
    direktZeigen: (eintraege) => {
      direkt = eintraege;
      direktZeichnen(direktEintraege, direkt, gewaehlt);
    },
    direktNichtGeladen: () => {
      direktEintraege.replaceChildren(leer('Direktnachrichten nicht geladen.'));
    },
    // schluessel: das offene Gespräch (gespraech.js) oder null
    auswahlZeigen: (schluessel) => {
      gewaehlt = schluessel;
      channelsZeichnen(channelEintraege, channels, gewaehlt);
      direktZeichnen(direktEintraege, direkt, gewaehlt);
    },
  };
}

function abschnitt(titel, schalter, eintraege) {
  const kopf = element('div', { className: 'lc-side-kopf' }, [
    element('h2', { className: 'lc-label lc-side-titel', textContent: titel }),
    ...schalter,
  ]);
  const nav = element('nav', { className: 'lc-side-abschnitt' }, [kopf, eintraege]);
  nav.setAttribute('aria-label', titel);
  return nav;
}

function channelsZeichnen(eintraege, channels, gewaehlt) {
  if (channels.length === 0) {
    eintraege.replaceChildren(leer('Noch keine Channels.'));
    return;
  }
  listeZeichnen(eintraege, channels.map((channel) => channelEintrag(channel, gewaehlt)));
}

function direktZeichnen(eintraege, direkt, gewaehlt) {
  if (direkt === null) {
    eintraege.replaceChildren();
    return;
  }
  if (direkt.length === 0) {
    eintraege.replaceChildren(leer('Noch keine Direktnachrichten.'), leer('Ein Gespräch beginnt in der Teilnehmerliste.'));
    return;
  }
  listeZeichnen(eintraege, direkt.map((eintrag) => direktEintrag(eintrag, gewaehlt)));
}

// Nur die Einträge werden neu gezeichnet — Kopf und Schalter bleiben, ein Fokus darauf auch; lag er auf einem
// Eintrag, kehrt er auf den Eintrag desselben Gesprächs zurück.
function listeZeichnen(eintraege, zeilen) {
  let fokussiert;
  if (eintraege.contains(document.activeElement)) {
    fokussiert = document.activeElement.dataset.gespraech;
  }
  const liste = element('ul', { className: 'lc-side-liste' }, zeilen);
  eintraege.replaceChildren(liste);
  const wiederFokussieren = [...liste.querySelectorAll('a')].find((verweis) => verweis.dataset.gespraech === fokussiert);
  if (wiederFokussieren !== undefined) {
    wiederFokussieren.focus();
  }
}

function leer(text) {
  return element('p', { className: 'lc-side-leer', textContent: text });
}

function channelEintrag(channel, gewaehlt) {
  const raute = element('span', { className: 'hash', textContent: '#' });
  raute.setAttribute('aria-hidden', 'true');
  const verweis = element('a', { className: 'lc-link', href: `#/c/${channel.name}` }, [
    raute,
    element('span', { className: 'n', textContent: channel.name }),
  ]);
  verweis.dataset.channel = channel.name;
  verweis.dataset.gespraech = channel.name;
  verweis.setAttribute('aria-label', channel.name);
  if (channel.name === gewaehlt) {
    verweis.setAttribute('aria-current', 'page');
  }
  const zeile = element('li', { className: 'lc-item', title: channel.name }, [verweis]);
  zeile.setAttribute('aria-label', channel.name);
  return zeile;
}

// Der Name des Links trägt den Status in ganzen Worten; Punkt, Kennung, Kürzel und Wort sind nur fürs Auge.
function direktEintrag({ kennung, kuerzel, punkt, wort, name }, gewaehlt) {
  const teile = [
    element('span', { className: `lc-dot ${punkt}` }),
    element('span', { className: 'n lc-id', textContent: kennung }),
    element('span', { className: 'lc-direkt-kuerzel', textContent: kuerzel }),
  ];
  if (wort !== null) {
    teile.push(element('span', { className: 'lc-status', textContent: wort }));
  }
  const schluessel = direktgespraech(kennung);
  const verweis = element('a', { className: 'lc-link', href: direktHash(kennung) }, teile);
  verweis.dataset.kennung = kennung;
  verweis.dataset.gespraech = schluessel;
  verweis.setAttribute('aria-label', name);
  if (schluessel === gewaehlt) {
    verweis.setAttribute('aria-current', 'page');
  }
  return element('li', { className: 'lc-item lc-direkt', title: name }, [verweis]);
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
