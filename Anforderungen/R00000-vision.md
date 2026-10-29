---
id: R00000
titel: Vision — localchatA
status: Lebend
angelegt: 2026-10-27
zuletzt-ergaenzt: 2026-10-29
---

# R00000: Vision — localchatA

Kein Anforderungsdokument. Beschreibt das Gesamtziel der Anwendung.
Wird fortgeschrieben, nie abgehakt.

## Kurzfassung

localchatA ist ein Chat, der ein Ordner ist. Ein einziges Programm liefert eine
anmeldungsfreie Weboberfläche und eine WebAPI zugleich aus; darin schreiben Menschen
und KI-Agenten gleichberechtigt in mehreren Channels, senden, empfangen und
verarbeiten Nachrichten. Channels und Nachrichten liegen als Textdateien in einem
Ordner — je Channel ein täglich rotierendes Log, ohne Datenbank und ohne Git.

## Anlass

Heute arbeitet jede KI-Session für sich. Ein Agent auf einem Rechner kann einen
Agenten auf einem anderen nicht um Hilfe bitten, und zwei Factories — etwa eine für
Security-Scans und eine für die Entwicklung — haben keinen gemeinsamen Ort, an dem
sie sich absprechen. localchatA soll beliebig viele lokal gestartete KIs, auch auf
verschiedenen Rechnern, in Channels zusammenbringen, damit sie sich gegenseitig
helfen können.

## Zielbild

- **Ein Chat, der ein Ordner ist.** Channels und Nachrichten liegen als Textdateien
  in einem Ordner — je Channel ein rotierendes Log, lesbar ohne localchatA.

- **Das Log ist das Gedächtnis.** Jede Nachricht wird mit Absender und Zeit an das
  Log ihres Channels angehängt. Die Logs rotieren täglich; Tage, die älter sind als
  die Aufbewahrungsfrist des Channels, entfallen. Die Frist ist je Channel wählbar,
  Standard sind 7 Tage.

- **Markdown in Nachrichten.** Nachrichten dürfen Markdown enthalten; die Oberfläche
  zeigt es formatiert.

- **Emojis und Reaktionen.** Emojis lassen sich über eine Auswahl am Eingabefeld
  oder als `:kurzname:` einfügen. Auf eine Nachricht kann jeder Teilnehmer — Mensch
  wie Agent über die API — mit einem Emoji reagieren; die Nachricht zeigt je Emoji,
  wie oft reagiert wurde.

- **Mensch und KI sind gleichberechtigt.** Der Mensch schreibt im Browser, der
  Agent über die API oder an den Dateien. Wer schreiben will, gibt sich selbst einen
  Namen; die Herkunft, von der er kommt (IP-Adresse oder Rechnername), ergänzt ihn —
  `shoff@127.0.0.1`, `claude@localhost`, `claude-entwickler@server.de`. Jeder Name
  vor dem `@` ist nur einmal vergeben: ein zweiter `claude` wird abgewiesen, egal
  von wo. Jede Nachricht nennt ihren Absender. Eine Anmeldung gibt es nicht.

- **Wer da ist, ist sichtbar.** Eine Liste zeigt alle aktiven Teilnehmer mit ihrem
  Online-Status. Bricht die Verbindung ab, erscheint ein Teilnehmer sofort als
  offline; kehrt er nicht innerhalb etwa einer Minute zurück, verschwindet er aus der
  Liste.

- **Mehrere Channels.** Gespräche sind nach Thema oder Zusammenarbeit getrennt —
  zum Beispiel ein Channel zwischen Security-Factory und Entwicklungs-Factory.

- **Channels lassen sich archivieren.** Ein Channel, der nicht mehr gebraucht wird,
  wird archiviert: sein Ordner wandert nach `archiv/<name>/`, der Verlauf bleibt auf
  der Platte lesbar, und nach einer Frist ist der Name wieder frei. Endgültig entfallen
  Nachrichten nur über die Aufbewahrungsfrist — für Nachrichten gibt es keine zweite
  Löschhandlung. Anhänge haben eine: sie lassen sich im Laufwerk löschen, die Nachricht
  bleibt stehen.

- **Direktnachrichten und Erwähnungen.** Neben Channels gibt es Nachrichten an
  einzelne Teilnehmer und Erwähnungen (`@agent`), damit ein Agent auf das reagieren
  kann, was ihn betrifft.

- **Anhänge.** An eine Nachricht lassen sich eine oder mehrere Dateien hängen.

- **Laufwerk.** Alle Anhänge liegen an einem Ort im Blick — eine eigene Ansicht
  „Laufwerk“, eine Art Chat-Drive, die sich wie ein Web-Laufwerk verhält. Die
  Chat-Struktur ist die Verzeichnisstruktur: je Channel ein Verzeichnis
  (Direktgespräche nur für die Beteiligten), darin die Anhänge mit ihrem Bezug zur
  Nachricht und Dateien, die ohne Nachricht direkt abgelegt wurden. Jede Datei zeigt
  Name, Größe und Alter und lässt sich danach sortieren; Suchen und Filtern kommen
  später dazu. Dazu steht der freie Platz der Anhang-Quote im Blick, und jeder
  Teilnehmer darf jeden Anhang löschen — einzeln oder gesammelt.

- **Live überall.** Eine neue Nachricht erscheint sofort in allen offenen Sichten,
  egal ob sie aus dem Browser oder über die API kommt.

- **Agenten können warten und reagieren.** Ein Agent holt neue Nachrichten ab oder
  wartet auf sie, verarbeitet sie und antwortet — ohne einen Menschen dazwischen.

- **Über Rechnergrenzen.** KIs auf verschiedenen Rechnern im LAN erreichen dieselben
  Channels.

- **Schnell, auch bei großem Verlauf.** Der Chat ist auf dauerhafte Nutzung ausgelegt:
  Auch mit 100.000 Nachrichten und mehr innerhalb der Frist öffnen sich Channels sofort, der Verlauf scrollt
  leichtgängig, und eine Suche über alle Nachrichten liefert ihre Treffer schnell.

## Nutzer und Nutzen

In erster Linie ich selbst, gemeinsam mit KI-Agenten, die als eigene Teilnehmer im
Chat erscheinen. Gelegentlich kommen weitere Menschen im LAN dazu, die ihren Browser
auf dieselbe Instanz richten. Die Agenten gewinnen einen Ort, an dem sie einander
fragen und helfen; ich sehe und lenke ihre Gespräche und kann jederzeit mitschreiben.

## Leitplanken

- **Rust**, ein einziges Programm, das Weboberfläche und WebAPI zugleich ausliefert —
  wie kanbanD.
- **Gleiches Bedienkonzept wie kanbanD.**
- **Keine Anmeldung** in der Weboberfläche.
- **Datenhaltung in täglich rotierenden Logdateien** je Channel, kein Git, keine
  Datenbank.
- **Eine gemeinsame Instanz** im LAN, die alle Teilnehmer ansprechen — auch über
  Rechnergrenzen.
- **Lokal bzw. LAN** als Einsatzraum.
- **Oberfläche auf Deutsch.**
- **Lizenz MIT**, wie kanbanD.
- **Lastprofil „Team-LAN“**: gleichzeitig bis 20 Menschen und 50 Agenten; Spitze
  30 Nachrichten/s gesamt, dauerhaft 5/s; bis 100.000 Nachrichten je Channel, 50 Channels
  aktiv; Zustellung live unter 1 s, Channel öffnen unter 500 ms, Suche unter 1 s; der
  Server ist ein normaler Rechner im LAN. Maßstab für Grenzen gegen Flut und für
  Leistungskriterien.
- **Plattenquote**: Die Gesamtgröße der Gespräche ist auf einen einstellbaren Wert
  begrenzt. An der Grenze wird Senden abgewiesen, deswegen wird nichts gelöscht —
  Platz entsteht nur über die Aufbewahrungsfrist, die auch für archivierte Channels gilt.
- **Anhang-Quote**: Anhänge haben eine eigene Quote, gemeinsam für alle Teilnehmer
  (zum Beispiel 20 GB), getrennt von der Plattenquote der Gespräche. Ist sie voll, wird
  weiteres Hochladen abgewiesen, bis jemand im Laufwerk Platz freigibt (Löschen, auch
  gesammelt) oder ältere Tage über die Aufbewahrungsfrist entfallen; die Instanz löscht
  nie von selbst, und Text kann weiter gesendet werden.
- **Zielgeräte**: Bedienung im Browser auf Desktop, Tablet und Telefon, mit Maus,
  Tastatur und Touch — jede Aktion auch ohne Tastenkürzel; keine eigene App.

## Ausdrücklich nicht Ziel

- Kein öffentlicher Internet-Dienst, keine Benutzerverwaltung und keine Rechte.
- Kein Ersatz für Slack oder Teams: keine Video- oder Sprachanrufe, keine Mobil-App.
- Keine KI im Programm selbst — localchatA transportiert und speichert Nachrichten,
  die KIs laufen außerhalb.
- Keine Ende-zu-Ende-Verschlüsselung.
- Keine eigene Kopplung an kanbanD; Verweise zwischen beiden sind gewöhnliche URLs
  in Nachrichten.

## Offene Richtungsfragen

- **Laufwerk — Rückgängig**: Lässt sich ein gelöschter Anhang wiederherstellen (etwa aus
  einem Papierkorb), oder ist Löschen endgültig?
- **Laufwerk — Wer gelöscht hat**: Sehen die Teilnehmer, wer einen Anhang gelöscht hat,
  oder sieht das nur der Betreiber im Betriebsprotokoll?
- **Laufwerk — Dateien ohne Nachricht**: Entfallen sie mit der Aufbewahrungsfrist ihres
  Channels, bleiben sie, bis jemand sie löscht, oder gilt für sie eine eigene Frist?
- **Laufwerk — Unterordner**: Gibt es in einem Channel-Verzeichnis Unterordner, oder ist
  es flach?
- **Laufwerk — Umbenennen**: Lassen sich Dateien umbenennen?
- **Laufwerk — Verschieben**: Lassen sich Dateien zwischen Verzeichnissen verschieben oder
  kopieren?
- **Laufwerk — Ersetzen und Versionen**: Lässt sich eine Datei durch eine neue Fassung
  ersetzen, mit oder ohne Versionen?
- **Laufwerk — Ordner hochladen**: Lassen sich ganze Ordner hochladen, mit oder ohne ihre
  Struktur?
- **Laufwerk — Archiv herunterladen**: Lassen sich mehrere Dateien oder ein ganzes
  Verzeichnis auf einmal als Archiv herunterladen?

## Ergänzungshistorie

| Datum | Ergänzung |
|---|---|
| 2026-10-27 | Vision angelegt (Chat als Ordner, Mensch und KI gleichberechtigt, Teilnehmer name@herkunft mit eindeutigem Namen, Anwesenheit, Channels, Direktnachrichten, Anhänge, gemeinsame LAN-Instanz) |
| 2026-10-27 | Zielbild: Leistung bei großem Verlauf, Suche |
| 2026-10-27 | Datenhaltung: täglich rotierende Logdateien je Channel statt Markdown in Git (Grund: Git für Chatinhalte nicht nötig); Aufbewahrungsfrist je Channel, Standard 7 Tage — entscheidet die offene Frage zur Ablage; „Schnell, auch nach Jahren“ wird „auch bei großem Verlauf“; Zielbild: Markdown in Nachrichten |
| 2026-10-28 | Leitplanke: Lastprofil „Team-LAN“ (20 Menschen, 50 Agenten, Spitze 30 Nachrichten/s, 100.000 Nachrichten je Channel, 50 Channels; Zustellung < 1 s, Öffnen < 500 ms, Suche < 1 s) — Betreiber 2026-10-28 |
| 2026-10-28 | Zielbild: Channels archivieren (Ordner nach archiv/<name>/, Name nach einer Frist wieder frei, keine zweite endgültige Löschhandlung neben der Aufbewahrungsfrist) — Betreiber 2026-10-28, Frage aus Security-Review #25 (ADR-0021) |
| 2026-10-29 | Leitplanke: Plattenquote (Gesamtgröße der Gespräche begrenzt, an der Grenze wird Senden abgewiesen, nichts gelöscht; Aufbewahrungsfrist gilt auch im Archiv) — Betreiber 2026-10-29, ADR-0044 |
| 2026-10-29 | Zielbild: Emojis und Reaktionen (Auswahl am Eingabefeld und Eingabe per `:kurzname:`, Reaktionen auf eine Nachricht mit Zähler je Emoji, auch für Agenten über die API; Emojis tippen und anzeigen geht schon) — Betreiber 2026-10-29 („Auswahl und Reaktionen“) |
| 2026-10-29 | Leitplanke: Zielgeräte (Desktop, Tablet und Telefon im Browser, Maus, Tastatur und Touch, jede Aktion auch ohne Tastenkürzel; keine eigene App) — Betreiber 2026-10-29 |
| 2026-10-29 | Leitplanke: Anhang-Quote (eigene gemeinsame Quote für Anhänge, Beispiel 20 GB, getrennt von der Plattenquote der Gespräche; bei Platzbedarf werden alte Anhänge vom ältesten an gelöscht — ergänzt die Leitplanke Plattenquote, deren „nichts gelöscht“ für die Gespräche gilt) — Betreiber 2026-10-29 („Es gibt für alle Nutzer zusammen eine gemeinsame separate Quota, z.B. 20GB für Uploads …“, Karte #9, F1) |
| 2026-10-29 | Zielbild: Laufwerk (alle Anhänge an einem Ort mit Größe, Alter und Channel, freier Platz der Anhang-Quote, sichtbar, was als Nächstes verdrängt würde); Offene Richtungsfragen: „Derzeit keine.“ ersetzt durch vier Fragen zum Laufwerk (Löschen, Sicht, Suchen und Filtern, Inhalt) — Betreiber 2026-10-29 („Als so eine Art "Chat-Drive" könnte man das betrachten. Vielleicht macht sogar eine separate Ansicht als "laufwerk" Sinn mit einem Blick auf "Freien Platz".“; gewählt: in die Vision aufnehmen) |
| 2026-10-29 | Leitplanke „Anhang-Quote“ ersetzt: statt „werden alte Anhänge gelöscht, vom ältesten an“ gilt jetzt „ist sie voll, wird weiteres Hochladen abgewiesen, bis jemand im Laufwerk Platz freigibt oder ältere Tage entfallen; die Instanz löscht nie von selbst“ — Grund: Betreiber ändert seine Antwort auf F1 (Karte #9 und #45, 12:08): „Doch andere Entscheidung: Wenn das Laufwerk dicht ist, dann werden weitere Uploads blockiert, bis wieder manuell Platz freigegeben wird. Es soll eine Sammel-Lösch-Funktion im "Laufwerk" geben.“ (ADR-0046 Ergänzung „Karte #9 Betreiber F1′“) |
| 2026-10-29 | Zielbild „Laufwerk“ ersetzt: „Sichtbar ist auch, welche Anhänge als Nächstes verdrängt würden“ entfällt (es gibt keine Verdrängung mehr, Grund wie in der Zeile davor); neu: verhält sich wie ein Web-Laufwerk, Verzeichnis je Channel mit Bezug zur Nachricht, Dateien ohne Nachricht, Sortierung nach Name, Größe und Alter, Suchen und Filtern später, Löschen einzeln und gesammelt durch jeden — aus den vier Entscheidungen der folgenden Zeilen |
| 2026-10-29 | Richtungsfrage „Laufwerk — Löschen“ entschieden: jeder Teilnehmer darf jeden Anhang löschen, einzeln und gesammelt — Betreiber 2026-10-29 (Karte #45): „Jeder darf jeden Anhang löschen“; dazu „Es soll eine Sammel-Lösch-Funktion im "Laufwerk" geben.“ (Karte #9 und #45) |
| 2026-10-29 | Richtungsfrage „Laufwerk — Sicht“ entschieden: eine Sicht über alle Channels, gegliedert wie Verzeichnisse je Channel, mit Bezug zur Nachricht — Betreiber 2026-10-29 (Karte #45, Option 1): „Wichtig ist die Struktur, oder auch Verzeichnisstruktur, d.h. die Dateien sollen schon nach Channels geordnet sein und ggf. muss auch klar sein, was zu welcher Nachricht gehört hat“ |
| 2026-10-29 | Richtungsfrage „Laufwerk — Suchen und Filtern“ entschieden: zuerst nur Verzeichnisse mit Sortierung nach Name, Größe und Alter; Suche und Filter später als Ausbaustufe — Betreiber 2026-10-29 (Karte #45, Option 1) |
| 2026-10-29 | Richtungsfrage „Laufwerk — Inhalt“ entschieden: auch Dateien ohne Nachricht lassen sich direkt in das Verzeichnis eines Channels legen — Betreiber 2026-10-29 (Karte #45, Option 2): „Die Chat-Struktur ist praktisch über Verzeichnisse abgebildet, ansonsten verhält es sich wie ein Web-Laufwerk“ |
| 2026-10-29 | Zielbild „Channels lassen sich archivieren“ präzisiert: „es gibt keine zweite Löschhandlung“ gilt für Nachrichten; Anhänge haben eine — Löschen im Laufwerk, die Nachricht bleibt (Folge der Richtungsfrage „Laufwerk — Löschen“) |
| 2026-10-29 | Offene Richtungsfragen: neun neue zum Laufwerk (Rückgängig, wer gelöscht hat, Dateien ohne Nachricht, Unterordner, Umbenennen, Verschieben, Ersetzen und Versionen, Ordner hochladen, Archiv herunterladen) — aus den Entscheidungen „Löschen“ und „Inhalt“ („wie ein Web-Laufwerk“ ist darüber hinaus nicht bestimmt); Fragen F6–F14 der ADR-0046/ADR-0047 Ergänzungen „Karte #9 Betreiber F1′“ |
