// SSE zerlegen (B0086, ADR-0015 Regel 6): Teilstücke aus dem fetch-Strom → Ereignisse { event, data } und
// der Rest, der auf das nächste Teilstück wartet. Zeilenenden \n, \r\n und \r; Kommentarzeilen (`:`)
// zählen nicht; ein Block ohne data ergibt kein Ereignis. Rein, in Node getestet.

const LEERZEILE = '\n\n';

export function zerlegen(rest, stueck) {
  const text = zeilenendenVereinheitlichen(rest + stueck);
  const bloecke = text.split(LEERZEILE);
  const unvollstaendig = bloecke.pop();
  const ereignisse = bloecke.map(blockLesen).filter((ereignis) => ereignis !== null);
  return { ereignisse, rest: unvollstaendig };
}

// Ein \r am Ende kann die erste Hälfte von \r\n sein — es bleibt stehen, bis das nächste Stück zeigt, was folgt.
function zeilenendenVereinheitlichen(text) {
  let vorn = text;
  let zurueckgehalten = '';
  if (text.endsWith('\r')) {
    vorn = text.slice(0, -1);
    zurueckgehalten = '\r';
  }
  return vorn.replace(/\r\n?/g, '\n') + zurueckgehalten;
}

function blockLesen(block) {
  let event = '';
  const datenzeilen = [];
  for (const zeile of block.split('\n')) {
    if (zeile.startsWith(':')) {
      continue;
    }
    const { feld, wert } = feldLesen(zeile);
    if (feld === 'event') {
      event = wert;
    }
    if (feld === 'data') {
      datenzeilen.push(wert);
    }
  }
  if (datenzeilen.length === 0) {
    return null;
  }
  return { event, data: datenzeilen.join('\n') };
}

function feldLesen(zeile) {
  const doppelpunkt = zeile.indexOf(':');
  if (doppelpunkt === -1) {
    return { feld: zeile, wert: '' };
  }
  let wert = zeile.slice(doppelpunkt + 1);
  if (wert.startsWith(' ')) {
    wert = wert.slice(1);
  }
  return { feld: zeile.slice(0, doppelpunkt), wert };
}
