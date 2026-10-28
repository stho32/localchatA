// Channel im Browser anlegen (F0025, ADR-0024 Regeln 4, 5, 7): `+` oder `Strg Alt N` öffnet den Dialog.

// `Strg Alt N` überall — außer mit AltGraph: auf deutschen Tastaturen ist AltGr gleich Strg Alt, und AltGr N
// tippt in Textfeldern ein Zeichen (Regel 7).
export function kuerzelOeffnet(ereignis) {
  const strgAlt = ereignis.ctrlKey && ereignis.altKey;
  const altGraph = ereignis.getModifierState('AltGraph');
  return strgAlt && !altGraph && ereignis.key.toLowerCase() === 'n';
}
