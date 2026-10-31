// Die Folge eines gelöschten Anhangs im offenen Verlauf (R00030 F0118, B0518, B0517; Schnitt B0796 wie player.js): der Tab
// vermerkt die IDs, spätere Abrufe zeigen sie schon als gelöscht, die Zeile „Gelöscht“ ersetzt Vorschau und Player, und hatte
// ein ersetztes Element den Fokus, nennt die Ansage den Anhang einmal. Alles Zeichnen, das Fenster und die Ansage bleiben beim
// Verlauf und kommen als Griffe herein.
import { anhaengeAlsGeloescht } from './live-verlauf.js';

// Der Name des gelöschten Anhangs, in dem der Fokus lag, oder null. nachricht: die Nachricht vor dem Ersetzen;
// ids: Set der jetzt gelöschten Anhang-IDs; hatFokus(id): liegt der Fokus im Knoten des Anhangs?
export function fokusNameVonGeloeschtem(nachricht, ids, hatFokus) {
  const betroffen = nachricht.anhaenge.filter((anhang) => ids.has(anhang.id) && anhang.zustand !== 'geloescht');
  const mitFokus = betroffen.find((anhang) => hatFokus(anhang.id));
  if (mitFokus === undefined) {
    return null;
  }
  return mitFokus.name;
}

// Gelöschte Anhänge [{ id, zeit }] → Map Satzzeit der bekannten Nachricht → Set der IDs; unbekannte Nachrichten fallen weg.
export function idsJeNachricht(geloescht, bekannteNachricht) {
  const jeZeit = new Map();
  for (const { id, zeit } of geloescht) {
    const bekannt = bekannteNachricht(zeit);
    if (bekannt === null) {
      continue;
    }
    if (!jeZeit.has(bekannt.nachricht.zeit)) {
      jeZeit.set(bekannt.nachricht.zeit, new Set());
    }
    jeZeit.get(bekannt.nachricht.zeit).add(id);
  }
  return jeZeit;
}

// griffe.bekannteNachricht(zeit) → { nachricht, … } | null · griffe.ersetzen(bekannt, neu) → true gezeichnet, false nur
// gemerkt (Bearbeiten-Feld offen) · griffe.hatFokus(zeit, anhangId) · griffe.ansagen(text) · griffe.gehalten(arbeit): führt
// die Arbeit aus und hält die Lesestelle (Anker).
export function geloeschtFolgeAnlegen(griffe) {
  const ids = new Set();

  function nachrichtZeigen(zeit, jetzt) {
    const bekannt = griffe.bekannteNachricht(zeit);
    const neu = anhaengeAlsGeloescht(bekannt.nachricht, jetzt);
    if (neu === bekannt.nachricht) {
      return;
    }
    const verlorenerName = fokusNameVonGeloeschtem(bekannt.nachricht, jetzt, (id) => griffe.hatFokus(zeit, id));
    if (griffe.ersetzen(bekannt, neu) && verlorenerName !== null) {
      griffe.ansagen(`Anhang gelöscht: ${verlorenerName}`);
    }
  }

  return {
    // Der Tab erfährt das Löschen, auch für Channels und Nachrichten, die nicht im Fenster stehen.
    vermerken(geloescht) {
      geloescht.forEach(({ id }) => ids.add(id));
    },
    // Eine Antwort, die vor dem Löschen begann und erst danach ankommt, zeigt die Anhänge sonst als vorhanden.
    nachrichtenFiltern(nachrichten) {
      if (ids.size === 0) {
        return nachrichten;
      }
      return nachrichten.map((nachricht) => anhaengeAlsGeloescht(nachricht, ids));
    },
    ersetzen(geloescht) {
      const jeNachricht = idsJeNachricht(geloescht, griffe.bekannteNachricht);
      griffe.gehalten(() => jeNachricht.forEach((jetzt, zeit) => nachrichtZeigen(zeit, jetzt)));
    },
  };
}
