// Leiste oben im Verlauf (Designsystem VerlaufsMarken „Verbindung“, ADR-0037 Regel 7), rein: getrennt → „Verbindung
// verloren“ als Fehler; nach dem Nachholen 4 s lang „✓ Wieder verbunden · n nachgeladen“.

// ergebnis des Nachholens: { art: 'nachgeholt', zahl } | { art: 'neu' }
export function wiederVerbundenText(ergebnis) {
  if (ergebnis.art === 'neu') {
    return '✓ Wieder verbunden · neueste geladen';
  }
  if (ergebnis.zahl === 0) {
    return '✓ Wieder verbunden';
  }
  return `✓ Wieder verbunden · ${new Intl.NumberFormat('de-DE').format(ergebnis.zahl)} nachgeladen`;
}

// zustand: { getrennt, naechsterVersuchUm: ms seit der Epoche | null, meldung: Text | null, meldungSeitMs }
// → { text, fehler } | null
export function leisteInhalt({ getrennt, naechsterVersuchUm = null, meldung, meldungSeitMs }, jetztMs) {
  const MELDUNG_STEHT_MS = 4000;
  if (getrennt) {
    return { text: warnungText(naechsterVersuchUm, jetztMs), fehler: true };
  }
  const meldungSteht = meldung !== null && jetztMs - meldungSeitMs < MELDUNG_STEHT_MS;
  if (!meldungSteht) {
    return null;
  }
  return { text: meldung, fehler: false };
}

// Die Warnung mit Countdown (R00025 F0084, ADR-0015 Ergänzung Karte #36): Rest bis zum nächsten Versuch in ganzen
// Sekunden, aufgerundet, begrenzt auf 0–10 s (eine verstellte Uhr zählt nie über 10); erreicht heißt „wird verbunden …“.
// Ohne Zeitpunkt (Netz des Tabs aus, der Strom hat noch nicht gemeldet) kein Countdown.
export function warnungText(naechsterVersuchUm, jetztMs) {
  const HOECHSTENS_MS = 10000;
  if (naechsterVersuchUm === null) {
    return '⚠ Verbindung verloren';
  }
  const restMs = Math.min(Math.max(naechsterVersuchUm - jetztMs, 0), HOECHSTENS_MS);
  if (restMs === 0) {
    return '⚠ Verbindung verloren · wird verbunden …';
  }
  return `⚠ Verbindung verloren · neuer Versuch in ${Math.ceil(restMs / 1000)} s`;
}
