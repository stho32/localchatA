// Eigene Kennung in der Kopfzeile (Designsystem: Bausteine Kopfzeile und Kennung): dunkle 40-px-Leiste,
// rechts Kürzel-Kachel und `name@herkunft`, Herkunft gedämpft, Farbe und Marke nach Art, danach die Schalter
// der Bedienung (z. B. „Teilnehmer“). Zone „Ort“: nach dem Instanznamen der gewählte Channel `# name` in lc-title
// und danach seine Beschreibung (ADR-0027 Regel 5); der Tab heißt dann „# name — localchatA“.

export function zeichnen(wurzel, teilnehmer, { kennungFokussieren = false, schalter = [] } = {}) {
  const art = artDes(teilnehmer);
  const kachel = element('span', { className: `lc-tile ${art}`, textContent: kuerzel(teilnehmer.name) });
  kachel.setAttribute('aria-hidden', 'true');
  const kennung = element('span', { className: 'lc-me' }, [
    teilnehmer.name,
    element('i', { textContent: `@${teilnehmer.herkunft}` }),
  ]);
  kennung.tabIndex = -1;
  const bedienung = element('div', { className: 'lc-head-bed' }, [kachel, kennung]);
  if (art === 'agent') {
    bedienung.append(element('span', { className: 'lc-tag', textContent: 'Agent' }));
  }
  bedienung.append(...schalter);
  const ort = element('div', { className: 'lc-head-ort' }, [element('span', { className: 'lc-head-inst', textContent: 'localchatA' })]);
  const kopf = element('header', { className: 'lc-head' }, [
    ort,
    element('div', { className: 'lc-head-nav' }),
    bedienung,
  ]);
  kopf.setAttribute('role', 'banner');
  wurzel.replaceChildren(kopf);
  if (kennungFokussieren) {
    kennung.focus();
  }
  return { ortZeigen: (channel) => ortZeigen(ort, channel) };
}

// channel: { name, beschreibung } des gewählten bekannten Channels oder null.
function ortZeigen(ort, channel) {
  const inst = ort.querySelector('.lc-head-inst');
  if (channel === null) {
    ort.replaceChildren(inst);
    document.title = 'localchatA';
    return;
  }
  ort.replaceChildren(inst, element('span', { className: 'lc-head-chan', textContent: `# ${channel.name}` }));
  if (channel.beschreibung) {
    ort.append(beschreibungZeichnen(channel.beschreibung));
  }
  document.title = `# ${channel.name} — localchatA`;
}

// Einzeilig mit Ellipse, ganz im title, die Schreibrichtung in <bdi> isoliert (ADR-0022 Regel 5); unter 720 px
// blendet das CSS sie aus.
function beschreibungZeichnen(beschreibung) {
  return element('span', { className: 'lc-head-desc', title: beschreibung }, [element('bdi', { textContent: beschreibung })]);
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
