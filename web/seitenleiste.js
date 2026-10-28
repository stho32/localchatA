// Seitenleiste (B0135, ADR-0024 Regeln 1–3, Designsystem components/Seitenleiste): links die Channels — Kopf
// „Channels“ mit dem Schalter „+“, darunter je Channel `# name` nach Name, leer „Noch keine Channels.“. Unter
// 720 px zeigt jeder Eintrag nur `#`; der Name bleibt Tooltip und zugänglicher Name. Jeder Eintrag ist ein Link
// `#/c/<name>`, der gewählte trägt aria-current="page" auf kd-accent (ADR-0027 Regel 2). Keine Zähler.

export function anlegen() {
  const plus = element('button', { type: 'button', className: 'lc-side-plus', textContent: '+' });
  plus.setAttribute('aria-label', 'Channel anlegen');
  plus.setAttribute('aria-keyshortcuts', 'Control+Alt+N');
  const kopf = element('div', { className: 'lc-side-kopf' }, [
    element('h2', { className: 'lc-label lc-side-titel', textContent: 'Channels' }),
    plus,
  ]);
  const eintraege = element('div', { className: 'lc-side-eintraege' });
  const nav = element('nav', { className: 'lc-side' }, [kopf, eintraege]);
  nav.setAttribute('aria-label', 'Channels');
  let gezeigt = [];
  let gewaehlterName = null;
  return {
    nav,
    plus,
    channelsZeigen: (channels) => {
      gezeigt = channels;
      eintraegeZeichnen(eintraege, gezeigt, gewaehlterName);
    },
    auswahlZeigen: (name) => {
      gewaehlterName = name;
      eintraegeZeichnen(eintraege, gezeigt, gewaehlterName);
    },
  };
}

// Nur die Einträge werden neu gezeichnet — Kopf und Schalter bleiben, ein Fokus darauf auch; lag er auf einem
// Eintrag, kehrt er auf den Eintrag gleichen Namens zurück.
function eintraegeZeichnen(eintraege, channels, gewaehlterName) {
  let fokussiert;
  if (eintraege.contains(document.activeElement)) {
    fokussiert = document.activeElement.dataset.channel;
  }
  if (channels.length === 0) {
    eintraege.replaceChildren(element('p', { className: 'lc-side-leer', textContent: 'Noch keine Channels.' }));
    return;
  }
  const liste = element('ul', { className: 'lc-side-liste' }, channels.map((channel) => eintrag(channel, gewaehlterName)));
  eintraege.replaceChildren(liste);
  const wiederFokussieren = [...liste.querySelectorAll('a')].find((verweis) => verweis.dataset.channel === fokussiert);
  if (wiederFokussieren !== undefined) {
    wiederFokussieren.focus();
  }
}

function eintrag(channel, gewaehlterName) {
  const raute = element('span', { className: 'hash', textContent: '#' });
  raute.setAttribute('aria-hidden', 'true');
  const verweis = element('a', { className: 'lc-link', href: `#/c/${channel.name}` }, [
    raute,
    element('span', { className: 'n', textContent: channel.name }),
  ]);
  verweis.dataset.channel = channel.name;
  verweis.setAttribute('aria-label', channel.name);
  if (channel.name === gewaehlterName) {
    verweis.setAttribute('aria-current', 'page');
  }
  const zeile = element('li', { className: 'lc-item', title: channel.name }, [verweis]);
  zeile.setAttribute('aria-label', channel.name);
  return zeile;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
