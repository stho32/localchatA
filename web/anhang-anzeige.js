// Anhänge im Verlauf zeigen (R00030 F0102, ADR-0047 Regeln 5–7, Designsystem components/Anhang).
import { groesseInWorten } from './loeschen.js';

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

// Die Anhänge einer Nachricht in Sendefolge, gestapelt unter dem Text.
export function anhaengeZeichnen(anhaenge) {
  return element('div', { className: 'lc-anhaenge' }, anhaenge.map(anhangZeichnen));
}

function anhangZeichnen(anhang) {
  const darstellung = darstellungWaehlen(anhang);
  const teile = [dateiZeileZeichnen(anhang, darstellung)];
  if (darstellung.form === 'vorschau') {
    teile.unshift(vorschauZeichnen(anhang, darstellung));
  }
  return element('div', { className: 'lc-anhang' }, teile);
}

// Bildvorschau (B0458): der Platz steht aus `width`/`height`, bevor ein Byte lädt — bis dahin mit Name und Maßen auf
// kd-surface-2; das Bild zeichnet gedreht (EXIF, die Maße kommen schon getauscht) und unverzerrt. Scheitert die Anfrage,
// bleibt der Platz und sagt „Bild nicht verfügbar.“ — ein Ladefehler belegt kein Löschen (F0118). Die Vorschau ist der
// Schalter „<name> groß anzeigen“ (B0459).
function vorschauZeichnen(anhang, { breite, hoehe }) {
  const platz = element('span', { className: 'lc-vorschau-platz', textContent: `${anhang.name} · ${anhang.breite} × ${anhang.hoehe}` });
  const bild = element('img', { width: breite, height: hoehe, loading: 'lazy', decoding: 'async', alt: anhang.name });
  const vorschau = element('button', { type: 'button', className: 'lc-vorschau' }, [platz, bild]);
  vorschau.setAttribute('aria-label', `${anhang.name} groß anzeigen`);
  vorschau.addEventListener('click', () => bildGrossZeigen(anhang, vorschau));
  vorschau.dataset.zustand = 'laedt';
  bild.addEventListener('load', () => {
    vorschau.dataset.zustand = 'geladen';
  });
  bild.addEventListener('error', () => {
    vorschau.dataset.zustand = 'fehler';
    platz.textContent = 'Bild nicht verfügbar.';
  });
  bild.src = anhang.url;
  return vorschau;
}

// Bild im Dialog (B0459): das Original in den Bildschirm eingepasst, modal, mit dem Anzeigenamen als Namen; der Fokus
// steht auf „Schließen“ und bleibt im Dialog. Esc, „Schließen“ oder Tippen daneben schließen, der Fokus kehrt auf die
// Vorschau zurück. Der Dialog hängt am Dokument, nicht an der Nachricht: er überlebt das Neuzeichnen des Verlaufs.
function bildGrossZeigen(anhang, vorschau) {
  const schliessen = element('button', { type: 'button', className: 'lc-btn', textContent: 'Schließen' });
  const bild = element('img', { className: 'lc-bild-gross', src: anhang.url, alt: anhang.name });
  const dialog = element('dialog', { className: 'lc-bild-dialog' }, [element('div', { className: 'lc-bild-rahmen' }, [bild, schliessen])]);
  dialog.setAttribute('aria-label', anhang.name);
  schliessen.addEventListener('click', () => dialog.close());
  dialog.addEventListener('click', (ereignis) => {
    if (ereignis.target === dialog) {
      dialog.close();
    }
  });
  dialog.addEventListener('keydown', (ereignis) => {
    if (ereignis.key === 'Tab') {
      ereignis.preventDefault();
      schliessen.focus();
    }
  });
  dialog.addEventListener('close', () => {
    dialog.remove();
    if (vorschau.isConnected) {
      vorschau.focus();
    }
  });
  document.body.append(dialog);
  dialog.showModal();
  schliessen.focus();
}

// Datei-Zeile (B0457, Designsystem Anhang): Kürzel (nur für die Augen), Name mit Ellipse, Größe in Worten, „Öffnen“ in
// einem neuen Tab, wo der Browser die Art darstellt, und „Laden“ unter dem Anzeigenamen; alles nur als Text. Reicht der
// Platz dem Namen nicht für 96 px, brechen die Links in eine zweite Zeile um (CSS, UX V1). Ein gelöschter Anhang hat
// keine Links (Wortlaut und Ansage bringt F0118).
function dateiZeileZeichnen(anhang, { form, oeffnen }) {
  const kuerzel = element('span', { className: 'lc-datei-kuerzel', textContent: typKuerzel(anhang.name) });
  kuerzel.setAttribute('aria-hidden', 'true');
  const kopf = element('span', { className: 'lc-datei-kopf' }, [
    kuerzel,
    element('span', { className: 'lc-datei-name', textContent: anhang.name, title: anhang.name }),
    element('span', { className: 'lc-datei-groesse', textContent: groesseInWorten(anhang.bytes) }),
  ]);
  if (form === 'geloescht') {
    return element('div', { className: 'lc-datei' }, [kopf]);
  }
  const links = [];
  if (oeffnen) {
    links.push(verweisZeichnen(anhang, 'Öffnen', 'öffnen', { target: '_blank', rel: 'noopener' }));
  }
  links.push(verweisZeichnen(anhang, 'Laden', 'laden', { download: anhang.name }));
  return element('div', { className: 'lc-datei' }, [kopf, element('span', { className: 'lc-datei-links' }, links)]);
}

function verweisZeichnen(anhang, beschriftung, tun, eigenschaften) {
  const verweis = element('a', { href: anhang.url, textContent: beschriftung, ...eigenschaften });
  verweis.setAttribute('aria-label', `${anhang.name} ${tun}`);
  return verweis;
}

// Typkürzel aus der letzten Endung des Namens, nur zur Anzeige (UX S3, S7): groß, höchstens 5 Zeichen; ohne Endung „DATEI“.
export function typKuerzel(name) {
  const KUERZEL_ZEICHEN = 5;
  const punkt = name.lastIndexOf('.');
  const endung = name.slice(punkt + 1);
  if (punkt <= 0 || endung === '') {
    return 'DATEI';
  }
  return Array.from(endung.toUpperCase()).slice(0, KUERZEL_ZEICHEN).join('');
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
