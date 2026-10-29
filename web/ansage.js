// Gebündelte Ansage (ADR-0038 Regel 10), rein mit gestellter Zeit: fremde Nachrichten des offenen Channels — sind
// seit der letzten Ansage mindestens 10 s vergangen und ist nichts gesammelt, sofort; sonst gesammelt und frühestens
// 10 s nach der letzten Ansage als eine. Eigene, nachgeladene und nachgeholte nie; ein Fehlschlag sofort.
//
// ereignis: { art: 'fremd', von, channel, offenerChannel } · { art: 'eigen' | 'nachgeholt' | 'takt' | 'gewechselt' }
//         · { art: 'fehlschlag', grund } · { art: 'sofort', text } (Verbindung, Statuszeile, Hinweis, Verworfen,
//         Entfernt — sofort, der Takt der gesammelten bleibt; R00025 F0084) · { art: 'getrennt' } · { art: 'wiederverbunden',
//         text } (sofort; „Verbindung verloren“ einmal je Trennung, nicht je Versuch)
// → { zustand, text: Ansage | null }
export const ANSAGE_LEER = { letzteAnsageMs: null, gesammelt: 0, channel: null, getrennt: false };

const ABSTAND_MS = 10000;

export function ansage(zustand, ereignis, jetztMs) {
  switch (ereignis.art) {
    case 'fremd':
      return fremdAnsagen(zustand, ereignis, jetztMs);
    case 'takt':
      return gesammeltAnsagen(zustand, jetztMs);
    case 'fehlschlag':
      return { zustand, text: `Nachricht nicht gesendet: ${ereignis.grund}` };
    case 'sofort':
      return { zustand, text: ereignis.text };
    case 'getrennt':
      return getrenntAnsagen(zustand);
    case 'wiederverbunden':
      return { zustand: { ...zustand, getrennt: false }, text: ereignis.text };
    case 'gewechselt':
      return { zustand: { ...zustand, gesammelt: 0 }, text: null };
    case 'eigen':
    case 'nachgeholt':
      return { zustand, text: null };
    default:
      throw new Error(`Ansage: unbekanntes Ereignis ${ereignis.art}`);
  }
}

function getrenntAnsagen(zustand) {
  if (zustand.getrennt) {
    return { zustand, text: null };
  }
  return { zustand: { ...zustand, getrennt: true }, text: 'Verbindung verloren' };
}

function fremdAnsagen(zustand, { von, channel, offenerChannel }, jetztMs) {
  if (channel !== offenerChannel) {
    return { zustand, text: null };
  }
  if (zustand.gesammelt === 0 && abstandErreicht(zustand, jetztMs)) {
    return { zustand: { ...zustand, letzteAnsageMs: jetztMs, channel }, text: `Neue Nachricht von ${von} in # ${channel}` };
  }
  return { zustand: { ...zustand, gesammelt: zustand.gesammelt + 1, channel }, text: null };
}

function gesammeltAnsagen(zustand, jetztMs) {
  if (zustand.gesammelt === 0 || !abstandErreicht(zustand, jetztMs)) {
    return { zustand, text: null };
  }
  let anzahl = `${zustand.gesammelt} neue Nachrichten`;
  if (zustand.gesammelt === 1) {
    anzahl = '1 neue Nachricht';
  }
  return { zustand: { ...zustand, gesammelt: 0, letzteAnsageMs: jetztMs }, text: `${anzahl} in # ${zustand.channel}` };
}

function abstandErreicht(zustand, jetztMs) {
  return zustand.letzteAnsageMs === null || jetztMs - zustand.letzteAnsageMs >= ABSTAND_MS;
}

// Der Schreiber der Ansage-Region: leert sie und setzt im nächsten Bild den Text, damit auch derselbe Text wieder
// angesagt wird. Alle Ansagen bis zu diesem Bild gehen zusammengezogen hinaus — in der Reihenfolge des Eintreffens, mit
// „. “ verbunden, gleiche einmal; nie gewinnt nur die letzte (R00025 F0084, ADR-0038 Ergänzung „Karte #36
// UX-Vorprüfung“ Punkt 3).
export function ansageSchreiberAnlegen({ schreiben, imNaechstenBild }) {
  let wartende = [];
  return (text) => {
    const erstesImBild = wartende.length === 0;
    wartende.push(text);
    if (!erstesImBild) {
      return;
    }
    schreiben('');
    imNaechstenBild(() => {
      const texte = [...new Set(wartende)];
      wartende = [];
      schreiben(texte.join('. '));
    });
  };
}
