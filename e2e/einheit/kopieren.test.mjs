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
