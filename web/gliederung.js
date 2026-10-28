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

// Wie eine Nachricht nach der vorigen (oder null) steht (Designsystem Nachricht, R00007): ohne Kopf, wenn derselbe
// Absender sie höchstens 5 Minuten nach der vorigen schrieb und kein Tagestrenner dazwischen steht; ein
// Tagestrenner steht vor der ersten Nachricht eines Tages (Datum im Versatz der Nachricht).
// → { kopf, tagestrenner: 'JJJJ-MM-TT' | null }
export function gliederung(vorige, nachricht) {
  const GRUPPE_HOECHSTENS_MS = 5 * 60 * 1000;
  const datum = datumDer(nachricht.zeit);
  if (vorige === null || datumDer(vorige.zeit) !== datum) {
    return { kopf: true, tagestrenner: datum };
  }
  const abstand = Date.parse(nachricht.zeit) - Date.parse(vorige.zeit);
  const gruppiert = vorige.von === nachricht.von && abstand <= GRUPPE_HOECHSTENS_MS;
  return { kopf: !gruppiert, tagestrenner: null };
}

function datumDer(zeit) {
  const { jahr, monat, tag } = teile(zeit);
  return `${jahr}-${monat}-${tag}`;
}
