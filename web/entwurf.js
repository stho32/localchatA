// Entwurf je Channel (ADR-0038 Regel 7): sessionStorage unter `localchatA.entwurf.<channel>` — geschrieben bei Eingabe
// höchstens alle 500 ms und beim Verlassen des Channels sofort, gelesen beim Öffnen, gelöscht beim Senden und wenn
// das Feld leer wird. Nie localStorage: Tabs überschrieben einander.
// speicher: sessionStorage-förmig · zeitgeber: { stellen(ms, rueckruf) → id, loeschen(id) }
export function entwurfAnlegen({ speicher, zeitgeber = ECHTE_ZEITGEBER }) {
  const SCHREIBEN_HOECHSTENS_ALLE_MS = 500;
  let ausstehend = null;

  function schreiben(channel, text) {
    abbrechen();
    if (text === '') {
      speicher.removeItem(schluessel(channel));
      return;
    }
    try {
      speicher.setItem(schluessel(channel), text);
    } catch (ausnahme) {
      console.warn(`localchatA: Entwurf nicht gemerkt, Neuladen verliert ihn — ${ausnahme.message}`);
    }
  }

  function abbrechen() {
    if (ausstehend !== null) {
      zeitgeber.loeschen(ausstehend.zeitgeber);
      ausstehend = null;
    }
  }

  return {
    eingabe: (channel, text) => {
      const ausstehendFuerAnderen = ausstehend !== null && ausstehend.channel !== channel;
      if (ausstehendFuerAnderen) {
        schreiben(ausstehend.channel, ausstehend.text);
      }
      if (text === '') {
        schreiben(channel, text);
        return;
      }
      if (ausstehend !== null) {
        ausstehend.text = text;
        return;
      }
      ausstehend = { channel, text, zeitgeber: null };
      ausstehend.zeitgeber = zeitgeber.stellen(SCHREIBEN_HOECHSTENS_ALLE_MS, () => schreiben(channel, ausstehend.text));
    },
    weggehen: (channel, text) => schreiben(channel, text),
    oeffnen: (channel) => speicher.getItem(schluessel(channel)) || '',
    gesendet: (channel) => schreiben(channel, ''),
  };
}

function schluessel(channel) {
  return `localchatA.entwurf.${channel}`;
}

const ECHTE_ZEITGEBER = {
  stellen: (ms, rueckruf) => setTimeout(rueckruf, ms),
  loeschen: (id) => clearTimeout(id),
};
