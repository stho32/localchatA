---
application: localchatA
system: localchatA
vision: Anforderungen/R00000-vision.md
wireframes: Dokumentation/Wireframes/
sprache: de
zuletzt: 2026-10-27
---

# WBS — localchatA

## Knoten

| ID | Ebene | Eltern | Name | Status | Fertig-Kriterium | Eingabe → Ausgabe | Aufwand | Ausbaustufe | Braucht | Requirement | Notiz |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A0001 | Application | — | localchatA | rot | alle Dialogs gruen | | | | | | Leitplanken aus der Vision: Rust, ein Programm für Weboberfläche und WebAPI, Markdown in Git statt Datenbank, eine gemeinsame LAN-Instanz, Bedienkonzept wie kanbanD, Oberfläche deutsch, MIT. Leistungsziel 100.000 Nachrichten gilt für jede Interaction mit Verlauf |
| D0001 | Dialog | A0001 | Start und Ablage | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0001 | Interaction | D0001 | Instanz starten | rot | `localchatA <ordner>` startet Weboberfläche und API, im LAN erreichbar | | | | | | aus Vision, kein Requirement |
| I0002 | Interaction | D0001 | Nachricht wird Markdown und Commit | rot | jede Nachricht liegt als Markdown-Datei vor und ist ein Commit unter dem Absender | | | | I0001 | | aus Vision, kein Requirement; Ablage offen (Vision, Offene Richtungsfragen) |
| I0003 | Interaction | D0001 | Fremde Änderung erscheint live | rot | Änderung per Editor oder `git pull` erscheint ohne Reload in allen Sichten | | | | I0002 | | aus Vision, kein Requirement |
| D0002 | Dialog | A0001 | Teilnehmen | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0004 | Interaction | D0002 | Namen wählen und beitreten | rot | Name plus Herkunft ergibt `name@herkunft`; ein vergebener Name vor dem `@` wird abgewiesen, egal von wo | | | | I0001 | | aus Vision, kein Requirement |
| I0005 | Interaction | D0002 | Teilnehmer mit Online-Status sehen | rot | Liste zeigt alle aktiven Teilnehmer mit online/offline | | | | I0004 | | aus Vision, kein Requirement |
| I0006 | Interaction | D0002 | Verbindung bricht ab | rot | sofort offline; nach etwa einer Minute ohne Rückkehr aus der Liste entfernt, Name wieder frei | | | | I0005 | | aus Vision, kein Requirement |
| D0003 | Dialog | A0001 | Channels | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0007 | Interaction | D0003 | Channel anlegen | rot | neuer Channel erscheint in der Liste aller Teilnehmer | | | | I0004 | | aus Vision, kein Requirement |
| I0008 | Interaction | D0003 | Channel wechseln und Verlauf lesen | rot | Auswahl zeigt den Verlauf mit Absender und Zeit; bei 100.000 Nachrichten öffnet der Channel sofort und scrollt flüssig | | | | I0007 | | aus Vision, kein Requirement |
| I0009 | Interaction | D0003 | Nachricht senden | rot | Nachricht erscheint im Channel bei allen offenen Sichten sofort | | | | I0008, I0002 | | aus Vision, kein Requirement |
| I0010 | Interaction | D0003 | Anhänge senden und öffnen | rot | eine oder mehrere Dateien hängen an einer Nachricht und lassen sich öffnen | | | | I0009 | | aus Vision, kein Requirement |
| I0011 | Interaction | D0003 | Nachrichten durchsuchen | rot | Suche über alle sichtbaren Nachrichten liefert bei 100.000 Nachrichten Treffer ohne spürbare Wartezeit | | | | I0009 | | aus Vision, kein Requirement |
| D0004 | Dialog | A0001 | Direktnachrichten und Erwähnungen | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0012 | Interaction | D0004 | Direktnachricht senden | rot | nur Absender und Empfänger sehen das Gespräch | | | | I0009 | | aus Vision, kein Requirement |
| I0013 | Interaction | D0004 | Erwähnt werden | rot | `@name` hebt die Nachricht beim Erwähnten hervor | | | | I0009 | | aus Vision, kein Requirement |
| D0005 | Dialog | A0001 | Agent an der API | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0014 | Interaction | D0005 | Als Agent beitreten | rot | Agent wird über die API mit `name@herkunft` Teilnehmer, gleiche Eindeutigkeit wie im Browser | | | | I0004 | | aus Vision, kein Requirement |
| I0015 | Interaction | D0005 | Nachricht über die API senden | rot | in Channel oder direkt; erscheint live in der Oberfläche | | | | I0014, I0009 | | aus Vision, kein Requirement |
| I0016 | Interaction | D0005 | Auf Nachrichten warten | rot | Agent wartet blockierend auf Neues für sich (Channel, Direktnachricht, Erwähnung) und erhält es ohne Polling-Schleife | | | | I0015 | | aus Vision, kein Requirement |

## Offene Fragen

## Notizen / Quellen

- Alle Knoten aus `Anforderungen/R00000-vision.md` (Stand 2026-10-27); noch keine Anforderungen.
- Designsystem unter `Dokumentation/Wireframes/designsystem/` (Export vom 2026-10-27) mit eigenen offenen Fragen in `3-offene-fragen.md`; Briefing `design-prompt.md`.
