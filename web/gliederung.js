// Gliederung des Verlaufs (ADR-0026 Regel 4, A00005 „Zeit im Verlauf“): Uhrzeit, Tagestrenner und Gruppierung aus
// dem Text von `zeit` mit dem Versatz des Servers — so zeigen Browser und Logdatei dieselbe Uhrzeit, unabhängig
// von der Zeitzone des Browsers. `zeit` ist RFC 3339 mit Versatz, wie die API sie liefert.

// → { jahr, monat, tag, stunde, minute, sekunde } als Text, in der Ortszeit des Versatzes
function teile(zeit) {
  const [, jahr, monat, tag, stunde, minute, sekunde] = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})/.exec(zeit);
  return { jahr, monat, tag, stunde, minute, sekunde };
}

export function uhrzeit(zeit) {
  const { stunde, minute } = teile(zeit);
  return `${stunde}:${minute}`;
}

export function zeitTitel(zeit) {
  const { jahr, monat, tag, stunde, minute, sekunde } = teile(zeit);
  return `${tag}.${monat}.${jahr}, ${stunde}:${minute}:${sekunde}`;
}
