// Restzeit (B0109, ADR-0019 Regel 5): wie lange eine offline-Zeile noch steht, in Serverzeit gerechnet. Rein —
// `uhr` ist Date.now() des Lesers, `versatz` der Abgleich mit der Serverzeit (jetzt_server − Date.now()).
// Bis 60 s „gleich weg“ mit „weg in m:ss“, darüber „offline“; bei 0 bleibt „weg in 0:00“ stehen, bis der Server
// die Zeile mit `weg` entfernt.

export function restzeit({ wegUm, versatz, uhr }) {
  const OFFLINE = { art: 'offline', text: 'offline' };
  if (!wegUm) {
    return OFFLINE;
  }
  const GLEICH_WEG_BIS_S = 60;
  const serverzeit = uhr + versatz;
  const restInSekunden = Math.max(0, Math.ceil((Date.parse(wegUm) - serverzeit) / 1000));
  if (restInSekunden > GLEICH_WEG_BIS_S) {
    return OFFLINE;
  }
  return { art: 'gleich-weg', text: `weg in ${minutenUndSekunden(restInSekunden)}` };
}

function minutenUndSekunden(sekunden) {
  const minuten = Math.floor(sekunden / 60);
  const rest = String(sekunden % 60).padStart(2, '0');
  return `${minuten}:${rest}`;
}
