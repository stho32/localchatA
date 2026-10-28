---
id: R00000
titel: Vision — localchatA
status: Lebend
angelegt: 2026-10-27
zuletzt-ergaenzt: 2026-10-28
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
  Nachrichten nur über die Aufbewahrungsfrist — es gibt keine zweite Löschhandlung.

- **Direktnachrichten und Erwähnungen.** Neben Channels gibt es Nachrichten an
  einzelne Teilnehmer und Erwähnungen (`@agent`), damit ein Agent auf das reagieren
  kann, was ihn betrifft.

- **Anhänge.** An eine Nachricht lassen sich eine oder mehrere Dateien hängen.

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

## Ausdrücklich nicht Ziel

- Kein öffentlicher Internet-Dienst, keine Benutzerverwaltung und keine Rechte.
- Kein Ersatz für Slack oder Teams: keine Video- oder Sprachanrufe, keine Mobil-App.
- Keine KI im Programm selbst — localchatA transportiert und speichert Nachrichten,
  die KIs laufen außerhalb.
- Keine Ende-zu-Ende-Verschlüsselung.
- Keine eigene Kopplung an kanbanD; Verweise zwischen beiden sind gewöhnliche URLs
  in Nachrichten.

## Offene Richtungsfragen

Derzeit keine.

## Ergänzungshistorie

| Datum | Ergänzung |
|---|---|
| 2026-10-27 | Vision angelegt (Chat als Ordner, Mensch und KI gleichberechtigt, Teilnehmer name@herkunft mit eindeutigem Namen, Anwesenheit, Channels, Direktnachrichten, Anhänge, gemeinsame LAN-Instanz) |
| 2026-10-27 | Zielbild: Leistung bei großem Verlauf, Suche |
| 2026-10-27 | Datenhaltung: täglich rotierende Logdateien je Channel statt Markdown in Git (Grund: Git für Chatinhalte nicht nötig); Aufbewahrungsfrist je Channel, Standard 7 Tage — entscheidet die offene Frage zur Ablage; „Schnell, auch nach Jahren“ wird „auch bei großem Verlauf“; Zielbild: Markdown in Nachrichten |
| 2026-10-28 | Leitplanke: Lastprofil „Team-LAN“ (20 Menschen, 50 Agenten, Spitze 30 Nachrichten/s, 100.000 Nachrichten je Channel, 50 Channels; Zustellung < 1 s, Öffnen < 500 ms, Suche < 1 s) — Betreiber 2026-10-28 |
| 2026-10-28 | Zielbild: Channels archivieren (Ordner nach archiv/<name>/, Name nach einer Frist wieder frei, keine zweite endgültige Löschhandlung neben der Aufbewahrungsfrist) — Betreiber 2026-10-28, Frage aus Security-Review #25 (ADR-0021) |
