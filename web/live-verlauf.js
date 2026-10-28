// Fremde Nachrichten im offenen Verlauf (ADR-0037 Regel 6, ADR-0028 Regeln 5, 7), rein.

// Übernimmt in Folge nur Nachrichten, deren Zeit (als Zeitpunkt) jünger ist als die jüngste bekannte; kleinere und
// gleiche fallen weg. juengste: Zeit (RFC 3339) oder null → { neue, juengste }.
export function einordnen(juengste, nachrichten) {
  const neue = [];
  let bisher = juengste;
  for (const nachricht of nachrichten) {
    const juenger = bisher === null || Date.parse(nachricht.zeit) > Date.parse(bisher);
    if (juenger) {
      neue.push(nachricht);
      bisher = nachricht.zeit;
    }
  }
  return { neue, juengste: bisher };
}

// Sammelt angenommene Nachrichten und fügt sie einmal je Frame ein (ADR-0037 Regel 6).
export function buendlerAnlegen({ einfuegen, naechsterFrame = (rueckruf) => globalThis.requestAnimationFrame(rueckruf) }) {
  let gesammelt = [];
  function frame() {
    const jetzt = gesammelt;
    gesammelt = [];
    einfuegen(jetzt);
  }
  return {
    annehmen: (nachricht) => {
      if (gesammelt.length === 0) {
        naechsterFrame(frame);
      }
      gesammelt.push(nachricht);
    },
  };
}
