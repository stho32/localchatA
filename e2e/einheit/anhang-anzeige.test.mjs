// Unit-Tests der Darstellung eines Anhangs im Verlauf (R00030 F0102 B0455, „Darstellung wählen“ und „Breiten“; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { darstellungWaehlen, rahmenMasse, geloeschtBeschriftung } from '../../web/anhang-anzeige.js';

const MIB = 1024 * 1024;
const anhang = (art, extra = {}) => ({ id: 'a', name: 'datei', bytes: 307200, art, breite: null, hoehe: null, zustand: 'vorhanden', ...extra });
const bild = (art, breite, hoehe, bytes = 307200) => anhang(art, { breite, hoehe, bytes });
const vorschau = (breite, hoehe) => ({ form: 'vorschau', breite, hoehe, oeffnen: true });
const zeile = (oeffnen) => ({ form: 'zeile', breite: null, hoehe: null, oeffnen });

test('Bilder mit Maßen werden Vorschau bis 360 × 240 im Seitenverhältnis', () => {
  assert.deepEqual(darstellungWaehlen(bild('image/png', 1280, 800)), vorschau(360, 225));
  assert.deepEqual(darstellungWaehlen(bild('image/png', 800, 1200)), vorschau(160, 240));
  assert.deepEqual(darstellungWaehlen(bild('image/webp', 4032, 3024)), vorschau(320, 240));
});

test('ein Hochformatfoto kommt mit getauschten Maßen und steht hochkant (Orientation 6)', () => {
  assert.deepEqual(darstellungWaehlen(bild('image/jpeg', 3000, 4000)), vorschau(180, 240));
});

test('über den Grenzen bleibt es bei der Zeile mit „Öffnen“: 20.000 × 20.000, 10 MiB + 1 Byte, 12 MiB', () => {
  assert.deepEqual(darstellungWaehlen(bild('image/jpeg', 20000, 20000)), zeile(true));
  assert.deepEqual(darstellungWaehlen(bild('image/png', 1280, 800, 10 * MIB + 1)), zeile(true));
  assert.deepEqual(darstellungWaehlen(bild('image/jpeg', 4000, 3000, 12 * MIB)), zeile(true));
});

test('die Grenzen selbst gelten noch: 10 MiB, 50 Megapixel, eine Seite 16.384 px', () => {
  assert.equal(darstellungWaehlen(bild('image/png', 1280, 800, 10 * MIB)).form, 'vorschau');
  assert.equal(darstellungWaehlen(bild('image/png', 10000, 5000)).form, 'vorschau');
  assert.deepEqual(darstellungWaehlen(bild('image/png', 10000, 5001)), zeile(true));
  assert.equal(darstellungWaehlen(bild('image/png', 16384, 100)).form, 'vorschau');
  assert.deepEqual(darstellungWaehlen(bild('image/png', 16385, 100)), zeile(true));
});

test('AVIF mit Maßen ist Vorschau, ohne Maße Zeile mit „Öffnen“; HEIC und HEIF nur Zeile mit „Laden“', () => {
  assert.deepEqual(darstellungWaehlen(bild('image/avif', 4032, 3024)), vorschau(320, 240));
  assert.deepEqual(darstellungWaehlen(anhang('image/avif')), zeile(true));
  assert.deepEqual(darstellungWaehlen(anhang('image/heic')), zeile(false));
  assert.deepEqual(darstellungWaehlen(anhang('image/heif')), zeile(false));
});

test('ein gelöschter Anhang ist „Geloescht“, gleich welcher Art', () => {
  const geloescht = { form: 'geloescht', breite: null, hoehe: null, oeffnen: false };
  for (const art of ['image/png', 'video/webm', 'audio/ogg', 'application/pdf']) {
    assert.deepEqual(darstellungWaehlen(anhang(art, { breite: 1280, hoehe: 800, zustand: 'geloescht' })), geloescht);
  }
});

test('die fünf Medienarten werden Video bzw. Audio, ohne „Öffnen“; video/ogg nur Zeile mit „Laden“', () => {
  for (const art of ['video/webm', 'video/mp4']) {
    assert.deepEqual(darstellungWaehlen(anhang(art)), { form: 'video', breite: null, hoehe: null, oeffnen: false });
  }
  for (const art of ['audio/ogg', 'audio/mpeg', 'audio/mp4']) {
    assert.deepEqual(darstellungWaehlen(anhang(art)), { form: 'audio', breite: null, hoehe: null, oeffnen: false });
  }
  assert.deepEqual(darstellungWaehlen(anhang('video/ogg')), zeile(false));
});

test('PDF und Text bis 16 MiB öffnen, Text darüber und jede andere Datei nur laden', () => {
  assert.deepEqual(darstellungWaehlen(anhang('application/pdf', { bytes: 2457600 })), zeile(true));
  assert.deepEqual(darstellungWaehlen(anhang('text/plain; charset=utf-8', { bytes: 16 * MIB })), zeile(true));
  assert.deepEqual(darstellungWaehlen(anhang('text/plain; charset=utf-8', { bytes: 16 * MIB + 1 })), zeile(false));
  assert.deepEqual(darstellungWaehlen(anhang('application/octet-stream')), zeile(false));
});

test('Breiten (V1): Rahmen min(360, Inhaltsbreite), Video-Höhe floor(Breite × 9/16)', () => {
  const erwartet = [[232, 232, 130], [268, 268, 150], [287, 287, 161], [323, 323, 181], [360, 360, 202], [716, 360, 202], [184, 184, 103]];
  for (const [inhaltsbreite, breite, videoHoehe] of erwartet) {
    assert.deepEqual(rahmenMasse(inhaltsbreite), { breite, videoHoehe });
  }
});

test('die Vorschau folgt der Inhaltsbreite proportional: 1280 × 800 bei 232 → 232 × 145', () => {
  assert.deepEqual(darstellungWaehlen(bild('image/png', 1280, 800), 232), vorschau(232, 145));
  assert.deepEqual(darstellungWaehlen(bild('image/png', 800, 1200), 232), vorschau(160, 240));
});

test('ein kleines Bild wird nicht vergrößert: 100 × 50 → 100 × 50', () => {
  assert.deepEqual(darstellungWaehlen(bild('image/png', 100, 50)), vorschau(100, 50));
});

// Nacharbeit F0102 (code-reviewer #43 S1): ein extrem schmales oder flaches Bild behält mindestens 1 px je Seite.
test('sehr schmale und flache Bilder: jede Seite der Vorschau mindestens 1 px', () => {
  assert.deepEqual(darstellungWaehlen({ art: 'image/png', bytes: 1000, breite: 1000, hoehe: 1, zustand: 'vorhanden' }), vorschau(360, 1));
  assert.deepEqual(darstellungWaehlen({ art: 'image/png', bytes: 1000, breite: 1, hoehe: 1000, zustand: 'vorhanden' }), vorschau(1, 240));
  assert.deepEqual(darstellungWaehlen({ art: 'image/png', bytes: 1000, breite: 16384, hoehe: 10, zustand: 'vorhanden' }), vorschau(360, 1));
});

// B0517 (UX V4): die Zeile „foto.png · 300 KiB · Gelöscht“ — gesehen in drei Teilen, vorgelesen mit Kommas.
test('Zeile „Gelöscht“: Name, Größe in Worten und „Gelöscht“; vorgelesen „foto.png, 300 KiB, Gelöscht“', () => {
  const geloescht = (name, bytes) => geloeschtBeschriftung({ ...anhang('image/png', { name, bytes }), zustand: 'geloescht' });
  assert.deepEqual(geloeschtBeschriftung({ ...anhang('image/png', { name: 'foto.png' }), zustand: 'geloescht' }), {
    name: 'foto.png', groesse: '300 KiB', marke: 'Gelöscht', vorgelesen: 'foto.png, 300 KiB, Gelöscht',
  });
  assert.equal(geloescht('film.webm', 1048575).vorgelesen, 'film.webm, 1,0 MiB, Gelöscht');
  assert.equal(geloescht('bericht.pdf', 2457600).vorgelesen, 'bericht.pdf, 2,3 MiB, Gelöscht');
  assert.equal(geloescht('<img onerror=x>.png', 12).vorgelesen, '<img onerror=x>.png, 12 Byte, Gelöscht');
});
