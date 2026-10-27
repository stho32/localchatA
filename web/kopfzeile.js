// Eigene Kennung in der Kopfzeile (Designsystem: Bausteine Kopfzeile und Kennung): dunkle 40-px-Leiste,
// rechts Kürzel-Kachel und `name@herkunft`, Herkunft gedämpft, Farbe und Marke nach Art.

export function zeichnen(wurzel, teilnehmer) {
  const art = artDes(teilnehmer);
  const kachel = element('span', { className: `lc-tile ${art}`, textContent: kuerzel(teilnehmer.name) });
  kachel.setAttribute('aria-hidden', 'true');
  const kennung = element('span', { className: 'lc-me' }, [
    teilnehmer.name,
    element('i', { textContent: `@${teilnehmer.herkunft}` }),
  ]);
  const bedienung = element('div', { className: 'lc-head-bed' }, [kachel, kennung]);
  if (art === 'agent') {
    bedienung.append(element('span', { className: 'lc-tag', textContent: 'Agent' }));
  }
  const kopf = element('header', { className: 'lc-head' }, [
    element('div', { className: 'lc-head-ort' }, [element('span', { className: 'lc-head-inst', textContent: 'localchatA' })]),
    element('div', { className: 'lc-head-nav' }),
    bedienung,
  ]);
  kopf.setAttribute('role', 'banner');
  wurzel.replaceChildren(kopf);
}

export function kuerzel(name) {
  const teile = name.split('-');
  if (teile.length > 1) {
    return teile.slice(0, 2).map((teil) => teil.charAt(0)).join('');
  }
  return name.slice(0, 2);
}

function artDes(teilnehmer) {
  if (teilnehmer.art === 'agent') {
    return 'agent';
  }
  return 'mensch';
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
