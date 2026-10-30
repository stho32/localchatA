'use strict';
// Auswertung des Lastgenerators (R00049 F0170 N5): Perzentile nach dem Rang-Verfahren und Wartezeit aus Retry-After.

export function kennwerte(dauernInMs) {
  if (dauernInMs.length === 0) {
    return { anzahl: 0, median: null, p90: null, max: null };
  }
  const aufsteigend = [...dauernInMs].sort((a, b) => a - b);
  return {
    anzahl: aufsteigend.length,
    median: wertAmRang(aufsteigend, 0.5),
    p90: wertAmRang(aufsteigend, 0.9),
    max: aufsteigend[aufsteigend.length - 1],
  };
}

function wertAmRang(aufsteigend, anteil) {
  const rang = Math.ceil(anteil * aufsteigend.length);
  return aufsteigend[Math.max(rang, 1) - 1];
}

export function wartezeitInMs(retryAfter) {
  const sekunden = Number.parseInt(retryAfter, 10);
  if (Number.isNaN(sekunden)) {
    return 1000;
  }
  return sekunden * 1000;
}
