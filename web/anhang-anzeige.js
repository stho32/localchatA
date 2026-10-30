// Anhänge im Verlauf zeigen (R00030 F0102, ADR-0047 Regeln 5–7, Designsystem components/Anhang).

const RAHMEN_HOECHSTENS = 360;

// Welche Gestalt ein Anhang im Verlauf hat (B0455), rein: `vorschau` für Bilder mit Maßen in den Grenzen (bis 360 × 240 im
// Seitenverhältnis, nie größer als das Bild, höchstens so breit wie der Inhalt), `video` und `audio` für die fünf
// Medienarten (ADR-0014 Ergänzung „Karte #71 (Chrome 154)“ Regel 5), sonst die Zeile; „Öffnen“ nur für Bilder außer HEIC
// und HEIF, PDF und Text bis 16 MiB — was der Server inline ausliefert. `art` kann Parameter tragen
// („text/plain; charset=utf-8“). anhang: { art, bytes, breite, hoehe, zustand } aus der API.
// → { form: 'vorschau' | 'video' | 'audio' | 'zeile' | 'geloescht', breite: Zahl | null, hoehe: Zahl | null, oeffnen }
export function darstellungWaehlen(anhang, inhaltsbreite = RAHMEN_HOECHSTENS) {
  const art = anhang.art.split(';')[0].trim();
  if (anhang.zustand === 'geloescht') {
    return gestalt('geloescht', false);
  }
  if (['video/mp4', 'video/webm'].includes(art)) {
    return gestalt('video', false);
  }
  if (['audio/mp4', 'audio/ogg', 'audio/mpeg'].includes(art)) {
    return gestalt('audio', false);
  }
  if (alsVorschau(art, anhang)) {
    return vorschauIn(anhang, rahmenMasse(inhaltsbreite).breite);
  }
  return gestalt('zeile', oeffnenBar(art, anhang.bytes));
}

// Breite der Rahmen von Vorschau, Player und Datei-Zeile und Höhe eines Videos (16 : 9) aus der Inhaltsbreite neben
// Gutter und Reserve (UX V1).
export function rahmenMasse(inhaltsbreite) {
  const breite = Math.min(RAHMEN_HOECHSTENS, inhaltsbreite);
  return { breite, videoHoehe: Math.floor((breite * 9) / 16) };
}

function alsVorschau(art, { bytes, breite, hoehe }) {
  const BYTES_HOECHSTENS = 10 * 1024 * 1024;
  const PIXEL_HOECHSTENS = 50_000_000;
  const SEITE_HOECHSTENS = 16_384;
  const bildMitMassen = art.startsWith('image/') && Number.isInteger(breite) && Number.isInteger(hoehe);
  if (!bildMitMassen || ['image/heic', 'image/heif'].includes(art)) {
    return false;
  }
  return bytes <= BYTES_HOECHSTENS && breite * hoehe <= PIXEL_HOECHSTENS && Math.max(breite, hoehe) <= SEITE_HOECHSTENS;
}

function vorschauIn({ breite, hoehe }, rahmenbreite) {
  const HOEHE_HOECHSTENS = 240;
  const faktor = Math.min(rahmenbreite / breite, HOEHE_HOECHSTENS / hoehe, 1);
  return { form: 'vorschau', breite: Math.round(breite * faktor), hoehe: Math.round(hoehe * faktor), oeffnen: true };
}

function oeffnenBar(art, bytes) {
  const TEXT_INLINE_HOECHSTENS = 16 * 1024 * 1024;
  if (art === 'text/plain') {
    return bytes <= TEXT_INLINE_HOECHSTENS;
  }
  const bildOhneHeif = art.startsWith('image/') && !['image/heic', 'image/heif'].includes(art);
  return bildOhneHeif || art === 'application/pdf';
}

function gestalt(form, oeffnen) {
  return { form, breite: null, hoehe: null, oeffnen };
}
