// Unit-Tests „Text in die Zwischenablage“ (R00046 I0042 B0700, ADR-0052 Regel 2; node --test) mit einer Attrappe des
// Dokuments, die aufzeichnet, in welchem Zustand das versteckte Feld beim Kopieren stand.
import test from 'node:test';
import assert from 'node:assert/strict';
import { kopieren } from '../../web/kopieren.js';

// → { dokument, aufruf: Aufzeichnung }; `ausgang` steuert execCommand ('ok' | 'nein' | 'wirft') und die Umgebung.
function attrappe({ ausgang = 'ok', sicher = true, schreiben = async () => {} } = {}) {
  const aufruf = { befehle: [], felderBeimKopieren: [], schreibtexte: [], fokus: [], angehaengt: [], entfernt: [], markierung: [] };
  const vorigesElement = { name: 'vorigesElement', focus: (optionen) => aufruf.fokus.push(optionen) };
  const felder = [];
  const dokument = {
    activeElement: vorigesElement,
    body: { append: (feld) => { felder.push(feld); aufruf.angehaengt.push(feld); } },
    createElement: (tag) => {
      const feld = {
        tag, value: '', readOnly: false, attribute: {}, style: {}, ausgewaehlt: null, istAngehaengt: false,
        setAttribute(name, wert) { this.attribute[name] = wert; },
        select() { aufruf.markierung.push('select'); },
        setSelectionRange(von, bis) { this.ausgewaehlt = [von, bis]; },
        remove() { aufruf.entfernt.push(this); },
      };
      return feld;
    },
    getSelection: () => ({ rangeCount: 0, removeAllRanges() {}, addRange() {} }),
    execCommand: (befehl) => {
      aufruf.befehle.push(befehl);
      aufruf.felderBeimKopieren.push(felder.map((feld) => ({ ...feld, entferntBisher: aufruf.entfernt.includes(feld) })));
      if (ausgang === 'wirft') {
        throw new Error('nicht erlaubt');
      }
      return ausgang === 'ok';
    },
    defaultView: {
      isSecureContext: sicher,
      navigator: { clipboard: { writeText: async (text) => { aufruf.schreibtexte.push(text); return schreiben(text); } } },
    },
  };
  return { dokument, aufruf, vorigesElement, felder };
}

const TEXT = 'Ein **fett** und ein Wort\n\n```sh\nls -la\n```\nEnde';

test('Textfeld mit dem Text, schreibgeschützt und ganz markiert, execCommand("copy") einmal im markierten Zustand', async () => {
  const { dokument, aufruf } = attrappe();

  const ergebnis = await kopieren(TEXT, dokument);

  assert.equal(ergebnis, 'kopiert');
  assert.deepEqual(aufruf.befehle, ['copy']);
  const [feldBeimKopieren] = aufruf.felderBeimKopieren[0];
  assert.equal(feldBeimKopieren.tag, 'textarea');
  assert.equal(feldBeimKopieren.value, TEXT);
  assert.equal(feldBeimKopieren.value.length, TEXT.length);
  assert.equal(feldBeimKopieren.readOnly, true);
  assert.deepEqual(feldBeimKopieren.ausgewaehlt, [0, TEXT.length]);
  assert.equal(feldBeimKopieren.entferntBisher, false);
  assert.equal(feldBeimKopieren.attribute['aria-hidden'], 'true');
  assert.equal(feldBeimKopieren.attribute.tabindex, '-1');
  assert.equal(feldBeimKopieren.style.position, 'fixed');
  assert.equal(feldBeimKopieren.style.fontSize, '16px');
  assert.deepEqual(aufruf.schreibtexte, []);
});

test('Feld danach entfernt, Fokus ohne Scrollen zurück auf das vorige Element', async () => {
  const { dokument, aufruf, felder } = attrappe();

  await kopieren(TEXT, dokument);

  assert.deepEqual(aufruf.entfernt, felder);
  assert.deepEqual(aufruf.fokus, [{ preventScroll: true }]);
});

test('wirft execCommand: Feld trotzdem entfernt, Fokus zurück, Ergebnis „nicht_moeglich“ im unsicheren Kontext', async () => {
  const { dokument, aufruf, felder } = attrappe({ ausgang: 'wirft', sicher: false });

  const ergebnis = await kopieren(TEXT, dokument);

  assert.equal(ergebnis, 'nicht_moeglich');
  assert.deepEqual(aufruf.entfernt, felder);
  assert.deepEqual(aufruf.fokus, [{ preventScroll: true }]);
  assert.deepEqual(aufruf.schreibtexte, []);
});

test('execCommand false im unsicheren Kontext → „nicht_moeglich“, writeText nie aufgerufen', async () => {
  const { dokument, aufruf } = attrappe({ ausgang: 'nein', sicher: false });

  assert.equal(await kopieren(TEXT, dokument), 'nicht_moeglich');
  assert.deepEqual(aufruf.schreibtexte, []);
});

test('execCommand false im sicheren Kontext → writeText(text) genau einmal als zweiter Weg', async () => {
  const { dokument, aufruf } = attrappe({ ausgang: 'nein', sicher: true });

  assert.equal(await kopieren(TEXT, dokument), 'kopiert');
  assert.deepEqual(aufruf.schreibtexte, [TEXT]);
});

test('execCommand wirft im sicheren Kontext → ebenfalls der zweite Weg; abgelehnt → „nicht_moeglich“', async () => {
  const { dokument, aufruf } = attrappe({
    ausgang: 'wirft', sicher: true, schreiben: async () => { throw new Error('NotAllowedError'); },
  });

  assert.equal(await kopieren(TEXT, dokument), 'nicht_moeglich');
  assert.deepEqual(aufruf.schreibtexte, [TEXT]);
});

test('execCommand klappt → writeText wird nie aufgerufen (erst der erste Weg, ADR-0052 Mutant K1)', async () => {
  const { dokument, aufruf } = attrappe({ ausgang: 'ok', sicher: true });

  await kopieren(TEXT, dokument);

  assert.deepEqual(aufruf.schreibtexte, []);
});

test('der erste Weg läuft vor dem ersten await: nach dem Aufruf steht das Ergebnis des Textfelds schon fest', () => {
  const { dokument, aufruf } = attrappe();

  const versprechen = kopieren(TEXT, dokument);

  assert.deepEqual(aufruf.befehle, ['copy']);
  assert.ok(versprechen instanceof Promise);
});

test('ohne Zwischenablage-Schnittstelle (navigator.clipboard fehlt) im sicheren Kontext → „nicht_moeglich“', async () => {
  const { dokument } = attrappe({ ausgang: 'nein', sicher: true });
  dokument.defaultView.navigator = {};

  assert.equal(await kopieren(TEXT, dokument), 'nicht_moeglich');
});

// Rückmeldung (R00046 I0042 B0703, ADR-0052 Regel 3): was nach dem Kopieren am Knopf steht und angesagt wird.
import { rueckmeldungFuer, ANZEIGE_KOPIERT_MS, ANZEIGE_FEHLER_MS, MENUE_OFFEN_MS, RUECKMELDUNG_LEER } from '../../web/kopieren.js';

const ZEIT_A = '2026-10-30T14:00:00.000+01:00';
const ZEIT_B = '2026-10-30T14:01:00.000+01:00';
const FEHLER_ANSAGE = 'Kopieren nicht möglich. Text markieren und Strg+C drücken.';

test('Dauern: 2000 ms „Kopiert.“ am Knopf, 5000 ms der Fehler, 1200 ms bleibt das Menü auf Touch offen', () => {
  assert.equal(ANZEIGE_KOPIERT_MS, 2000);
  assert.equal(ANZEIGE_FEHLER_MS, 5000);
  assert.equal(MENUE_OFFEN_MS, 1200);
});

test('Gelingen: „✓ Kopiert.“ 2000 ms am Knopf und eine Ansage „Kopiert.“', () => {
  const ergebnis = rueckmeldungFuer(RUECKMELDUNG_LEER, 'kopiert', ZEIT_A, 10000);

  assert.equal(ergebnis.anzeige, 'kopiert');
  assert.equal(ergebnis.dauerMs, 2000);
  assert.equal(ergebnis.ansage, 'Kopiert.');
});

test('dieselbe Nachricht innerhalb von 2 s noch einmal: die Anzeige gilt neu, die Ansage entfällt (Mutant K6)', () => {
  const erstes = rueckmeldungFuer(RUECKMELDUNG_LEER, 'kopiert', ZEIT_A, 10000);

  const zweites = rueckmeldungFuer(erstes.zustand, 'kopiert', ZEIT_A, 11000);

  assert.equal(zweites.anzeige, 'kopiert');
  assert.equal(zweites.dauerMs, 2000);
  assert.equal(zweites.ansage, null);
  assert.equal(rueckmeldungFuer(erstes.zustand, 'kopiert', ZEIT_A, 11999).ansage, null);
});

test('nach 2 s oder an einer anderen Nachricht wird wieder angesagt', () => {
  const erstes = rueckmeldungFuer(RUECKMELDUNG_LEER, 'kopiert', ZEIT_A, 10000);

  assert.equal(rueckmeldungFuer(erstes.zustand, 'kopiert', ZEIT_A, 12000).ansage, 'Kopiert.');
  assert.equal(rueckmeldungFuer(erstes.zustand, 'kopiert', ZEIT_B, 10500).ansage, 'Kopiert.');
});

test('Misslingen: „⚠ Kopieren nicht möglich“ 5000 ms und die Ansage mit dem Ausweg — jedes Mal', () => {
  const erstes = rueckmeldungFuer(RUECKMELDUNG_LEER, 'nicht_moeglich', ZEIT_A, 10000);
  const zweites = rueckmeldungFuer(erstes.zustand, 'nicht_moeglich', ZEIT_A, 10500);

  for (const ergebnis of [erstes, zweites]) {
    assert.equal(ergebnis.anzeige, 'nicht_moeglich');
    assert.equal(ergebnis.dauerMs, 5000);
    assert.equal(ergebnis.ansage, FEHLER_ANSAGE);
  }
});

test('ein Fehlschlag löscht nicht, dass eben „Kopiert.“ angesagt wurde', () => {
  const erstes = rueckmeldungFuer(RUECKMELDUNG_LEER, 'kopiert', ZEIT_A, 10000);
  const fehler = rueckmeldungFuer(erstes.zustand, 'nicht_moeglich', ZEIT_A, 10300);

  assert.equal(rueckmeldungFuer(fehler.zustand, 'kopiert', ZEIT_A, 10600).ansage, null);
});
