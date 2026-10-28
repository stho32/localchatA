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

// zustand: { getrennt, meldung: Text | null, meldungSeitMs } → { text, fehler } | null
export function leisteInhalt({ getrennt, meldung, meldungSeitMs }, jetztMs) {
  const MELDUNG_STEHT_MS = 4000;
  if (getrennt) {
    return { text: '⚠ Verbindung verloren', fehler: true };
  }
  const meldungSteht = meldung !== null && jetztMs - meldungSeitMs < MELDUNG_STEHT_MS;
  if (!meldungSteht) {
    return null;
  }
  return { text: meldung, fehler: false };
}
