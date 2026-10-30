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

// Datum und Uhrzeit ohne Sekunden: „TT.MM.JJJJ, HH:MM“.
export function datumUhrzeit(zeit) {
  const { jahr, monat, tag, stunde, minute } = teile(zeit);
  return `${tag}.${monat}.${jahr}, ${stunde}:${minute}`;
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

// Eine Folge nach der vorigen (oder null) gegliedert; die Nachricht mit der Zeit `ersteNeue` (oder null) trägt als
// erste gezählte neue immer ihren Kopf (R00020 F0067). → [{ kopf, tagestrenner }] in der Folge der Nachrichten
export function folgeGliedern(vorige, nachrichten, ersteNeue) {
  const gliederungen = [];
  let davor = vorige;
  for (const nachricht of nachrichten) {
    const { kopf, tagestrenner } = gliederung(davor, nachricht);
    const istErsteNeue = nachricht.zeit === ersteNeue;
    gliederungen.push({ kopf: kopf || istErsteNeue, tagestrenner });
    davor = nachricht;
  }
  return gliederungen;
}

function datumDer(zeit) {
  const { jahr, monat, tag } = teile(zeit);
  return `${jahr}-${monat}-${tag}`;
}

// Name eines Tagestrenners (Designsystem VerlaufsMarken): „Heute“ und „Gestern“ nach dem heutigen Datum des
// Servers (heute: 'JJJJ-MM-TT' oder null, solange es unbekannt ist), sonst „Montag, 26. Oktober 2026“.
export function datumBenennen(datum, heute) {
  if (datum === heute) {
    return 'Heute';
  }
  if (heute !== null && datum === tagDavor(heute)) {
    return 'Gestern';
  }
  const WOCHENTAGE = ['Sonntag', 'Montag', 'Dienstag', 'Mittwoch', 'Donnerstag', 'Freitag', 'Samstag'];
  const MONATE = ['Januar', 'Februar', 'März', 'April', 'Mai', 'Juni', 'Juli', 'August', 'September', 'Oktober',
    'November', 'Dezember'];
  const kalendertag = alsKalendertag(datum);
  const wochentag = WOCHENTAGE[kalendertag.getUTCDay()];
  return `${wochentag}, ${kalendertag.getUTCDate()}. ${MONATE[kalendertag.getUTCMonth()]} ${kalendertag.getUTCFullYear()}`;
}

// Kalenderrechnung in UTC, damit die Zeitzone des Browsers nie hineinspielt.
function alsKalendertag(datum) {
  return new Date(`${datum}T00:00:00Z`);
}

function tagDavor(datum) {
  const EIN_TAG_MS = 24 * 60 * 60 * 1000;
  return new Date(alsKalendertag(datum).getTime() - EIN_TAG_MS).toISOString().slice(0, 10);
}
