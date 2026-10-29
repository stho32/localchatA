// Ein Weg zum Wiederbeitritt (R00025 F0088, ADR-0012 Ergänzung Karte #36 Regeln 1–3): Strom und Senden melden jede
// Ablehnung hierher. Hat ein anderer Tab schon ein neues Token gemerkt, gilt es; sonst tritt der Tab still mit dem
// gemerkten Namen bei — unter der Sperre der Sitzung, ein Lauf je abgelehntem Token: ein zweiter Aufrufer bekommt das
// Ergebnis des ersten.
import * as sitzung from './sitzung.js';

// speicher: localStorage · api: { ich, beitreten } wie in start.js · sperre: (arbeit(geschwister)) → Promise (Web Lock
// oder Warten auf den Nachbar-Tab) · gelungen: (neuesToken) · gescheitert: (zustand des Beitreten-Bildschirms, abgelehntesToken)
// → wiederBeitreten(abgelehntesToken) → Promise<zustand>
export function wiederbeitrittAnlegen({ speicher, api, sperre, gelungen, gescheitert }) {
  const laeufe = new Map();
  return function wiederBeitreten(abgelehntesToken) {
    if (!laeufe.has(abgelehntesToken)) {
      laeufe.set(abgelehntesToken, beitreten(abgelehntesToken));
    }
    return laeufe.get(abgelehntesToken);
  };

  async function beitreten(abgelehntesToken) {
    const zustand = await sperre((geschwister) => sitzung.nachAblehnung(speicher, api, abgelehntesToken, geschwister));
    // Das eben geprüfte Token, nie neu aus dem Speicher: ohne Web Locks kann ein Nachbar-Tab ihn inzwischen geleert
    // haben (Befund #58), oder er hat das neue Token nie angenommen.
    if (zustand.art === 'kopfzeile') {
      gelungen(zustand.token);
    } else {
      gescheitert(zustand, abgelehntesToken);
    }
    return zustand;
  }
}
