// Tasten, die den Fokus in die Eingabe holen (ADR-0038 Regel 8, 1-tastatur.md), rein: `/` außerhalb von Textfeldern
// und `Esc` im Verlauf. ort: 'textfeld' | 'verlauf' | 'dialog' | 'sonst' → 'ins-feld' | null
export function fokusTaste(ereignis, ort) {
  const mitUmschalter = ereignis.ctrlKey || ereignis.altKey || ereignis.metaKey;
  if (mitUmschalter || ort === 'dialog') {
    return null;
  }
  const schraegstrichAusserhalb = ereignis.key === '/' && ort !== 'textfeld';
  const escImVerlauf = ereignis.key === 'Escape' && ort === 'verlauf';
  if (schraegstrichAusserhalb || escImVerlauf) {
    return 'ins-feld';
  }
  return null;
}
