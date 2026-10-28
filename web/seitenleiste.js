// Seitenleiste (B0135, ADR-0024 Regeln 1–3, Designsystem components/Seitenleiste): links die Channels — Kopf
// „Channels“ mit dem Schalter „+“, darunter je Channel `# name` nach Name, leer „Noch keine Channels.“. Unter
// 720 px zeigt jeder Eintrag nur `#`; der Name bleibt Tooltip und zugänglicher Name. Noch keine Links, kein
// aktiver Eintrag, keine Zähler (I0008).

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
  return {
    nav,
    plus,
    channelsZeigen: (channels) => eintraegeZeichnen(eintraege, channels),
  };
}

// Nur die Einträge werden neu gezeichnet — Kopf und Schalter bleiben, ein Fokus darauf auch.
function eintraegeZeichnen(eintraege, channels) {
  if (channels.length === 0) {
    eintraege.replaceChildren(element('p', { className: 'lc-side-leer', textContent: 'Noch keine Channels.' }));
    return;
  }
  const liste = element('ul', { className: 'lc-side-liste' }, channels.map(eintrag));
  eintraege.replaceChildren(liste);
}

function eintrag(channel) {
  const raute = element('span', { className: 'hash', textContent: '#' });
  raute.setAttribute('aria-hidden', 'true');
  const zeile = element('li', { className: 'lc-item', title: channel.name }, [
    raute,
    element('span', { className: 'n', textContent: channel.name }),
  ]);
  zeile.setAttribute('aria-label', channel.name);
  return zeile;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
