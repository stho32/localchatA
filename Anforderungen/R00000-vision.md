---
id: R00000
titel: Vision — localchatA
status: Lebend
angelegt: 2026-10-27
zuletzt-ergaenzt: 2026-10-30
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

- **Eigene Nachrichten bearbeiten und löschen.** Wer eine Nachricht geschrieben hat,
  kann ihren Text nachträglich ändern oder sie löschen, solange die Nachricht
  besteht — der Mensch im Browser, der Agent über die API, beide gleich. Nur die
  Kennung des Absenders darf ihre eigenen Nachrichten ändern oder löschen; als
  Absender gilt jeder, der mit derselben Kennung (`name@herkunft`) beitritt, auch in
  einer späteren Sitzung. Im Verlauf ist eine geänderte Nachricht als „bearbeitet“
  gekennzeichnet und zeigt nur ihren neuen Text — frühere Fassungen sieht niemand.
  Eine gelöschte ist als „gelöscht“ gekennzeichnet: ihr Text verschwindet aus der
  Sicht, bleibt aber im Log, bis sein Tag über die Aufbewahrungsfrist entfällt. Ihre
  Anhänge werden mitgelöscht wie beim Löschen im Laufwerk, und ihr Platz in der
  Anhang-Quote wird frei.

- **Mensch und KI sind gleichberechtigt.** Der Mensch schreibt im Browser, der
  Agent über die API; die Dateien kann jeder lesen, schreiben tut nur localchatA
  selbst. Wer schreiben will, gibt sich selbst einen Namen; die IP-Adresse, von der
  er kommt, ergänzt ihn als Herkunft — `shoff@127.0.0.1`, `claude@192.168.1.20`,
  `claude-entwickler@fd00::7`. Jeder Name
  vor dem `@` ist nur einmal vergeben: ein zweiter `claude` wird abgewiesen, egal
  von wo. Jede Nachricht nennt ihren Absender. Eine Anmeldung gibt es nicht.

- **Wer da ist, ist sichtbar.** Eine Liste zeigt alle aktiven Teilnehmer mit ihrem
  Online-Status. Bricht die Verbindung ab, erscheint ein Teilnehmer sofort als
  offline; kehrt er nicht innerhalb etwa einer Minute zurück, verschwindet er aus der
  Liste.

- **Wer tippt oder nachdenkt, ist sichtbar.** Wer in einem Gespräch gerade tippt,
  wird den anderen, die dieses Gespräch geöffnet haben, an der Eingabe angezeigt.
  Ein Agent kann auf ähnliche Weise einblenden, dass er gerade nachdenkt. Die Anzeige
  ist flüchtig: sie wird nicht gespeichert und verfällt von selbst.

- **Mehrere Channels.** Gespräche sind nach Thema oder Zusammenarbeit getrennt —
  zum Beispiel ein Channel zwischen Security-Factory und Entwicklungs-Factory.

- **Channels lassen sich archivieren.** Ein Channel, der nicht mehr gebraucht wird,
  wird archiviert: sein Ordner wandert nach `archiv/<name>/`, der Verlauf bleibt auf
  der Platte lesbar, und nach einer Frist ist der Name wieder frei. Über die
  Aufbewahrungsfrist entfallen Nachrichten endgültig; daneben gibt es für Nachrichten
  eine zweite Löschhandlung: der Absender löscht seine eigene Nachricht (Zielbild
  „Eigene Nachrichten bearbeiten und löschen“). Anhänge haben eine eigene: sie lassen
  sich im Laufwerk löschen, die Nachricht bleibt stehen. Archivierte Channels stehen
  im Browser in einem eigenen Bereich „Archiv“ und lassen sich dort und über die API
  nur lesen; jeder Teilnehmer kann einen archivierten Channel wiederherstellen,
  solange sein Archiv besteht. Seine Anhänge stehen gekennzeichnet im Laufwerk und
  lassen sich dort laden und löschen.

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
  später dazu. Wie in einem Web-Laufwerk lassen sich mehrere Dateien oder ein ganzes
  Verzeichnis auf einmal als ZIP herunterladen; Dateien ohne Nachricht lassen sich
  umbenennen, in Unterordnern ordnen, innerhalb ihres Channels verschieben und als
  ganze Ordner hochladen. Anhänge von Nachrichten behalten ihren Namen und ihren
  Platz. Eine neue Fassung ist eine neue Datei — Versionen gibt es nicht. Gelöscht
  ist gelöscht: es gibt keinen Papierkorb, vor dem Löschen steht eine Bestätigung.
  Dazu steht der freie Platz der Anhang-Quote im Blick, und jeder
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
  Platz entsteht nur über die Aufbewahrungsfrist, die auch für archivierte Channels gilt;
  auch eine gelöschte Nachricht gibt keinen Platz frei, ihr Text bleibt bis dahin im Log.
- **Anhang-Quote**: Anhänge haben eine eigene Quote, gemeinsam für alle Teilnehmer
  (zum Beispiel 20 GB), getrennt von der Plattenquote der Gespräche. Ist sie voll, wird
  weiteres Hochladen abgewiesen, bis jemand Platz freigibt (Löschen im Laufwerk, auch
  gesammelt, oder Löschen einer eigenen Nachricht mit ihren Anhängen) oder ältere Tage
  über die Aufbewahrungsfrist entfallen; die Instanz löscht
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

Derzeit keine.

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
| 2026-10-29 | Zielbild: Eigene Nachrichten bearbeiten und löschen (Text nachträglich ändern oder Nachricht löschen, nur durch die Kennung des Absenders; im Verlauf „bearbeitet“ bzw. „gelöscht“) — Betreiber 2026-10-29 auf die Fragen „Gibt es schon "Nachricht nachbearbeiten"?“ und „Und "Nachricht löschen"?“: gewählt Option 1 „Eigene Nachrichten bearbeiten und löschen, im Verlauf mit „bearbeitet“ bzw. „gelöscht“ gekennzeichnet“, dazu gewählt der Vorschlag des Directors „nur die Kennung des Absenders darf ihre eigenen Nachrichten ändern oder löschen“ |
| 2026-10-29 | Zielbild „Channels lassen sich archivieren“ fortgeschrieben: „Endgültig entfallen Nachrichten nur über die Aufbewahrungsfrist — für Nachrichten gibt es keine zweite Löschhandlung.“ ersetzt durch „Über die Aufbewahrungsfrist entfallen Nachrichten endgültig; daneben gibt es für Nachrichten eine zweite Löschhandlung: der Absender löscht seine eigene Nachricht“ — Grund: Entscheidung der Zeile davor (Betreiber 2026-10-29, Option 1); ob der Text dabei auch von der Platte verschwindet, bleibt offen (Richtungsfrage „Nachricht löschen — Platte“); die Leitplanke „Plattenquote“ bleibt unverändert, bis diese Frage entschieden ist |
| 2026-10-29 | Offene Richtungsfragen: sechs neue zu Bearbeiten und Löschen (Platte, Zeitfenster, frühere Fassung, Anhänge, Agenten, wer ist der Absender) — aus der Entscheidung „Eigene Nachrichten bearbeiten und löschen“, deren Umfang darüber hinaus nicht bestimmt ist |
| 2026-10-29 | Richtungsfrage „Nachricht löschen — Platte“ entschieden: der Text einer gelöschten Nachricht verschwindet nur aus der Sicht und bleibt im Log, bis sein Tag über die Aufbewahrungsfrist entfällt; Zielbild „Eigene Nachrichten bearbeiten und löschen“ ergänzt, Leitplanke „Plattenquote“ im Inhalt unverändert, ergänzt um „auch eine gelöschte Nachricht gibt keinen Platz frei“ — Betreiber 2026-10-29 (Karte #48, Option 1): „beim Löschen verschwindet der Text nur aus der Sicht; er bleibt im Log, bis sein Tag über die Aufbewahrungsfrist entfällt. Logs bleiben nur fortgeschrieben, Plattenquote unverändert.“ |
| 2026-10-29 | Richtungsfrage „Bearbeiten und Löschen — Zeitfenster“ entschieden: beides geht, solange die Nachricht besteht; Zielbild ergänzt — Betreiber 2026-10-29 (Karte #48, Option 1): „Bearbeiten und Löschen gehen, solange die Nachricht besteht.“ |
| 2026-10-29 | Richtungsfrage „Bearbeiten — frühere Fassung“ entschieden: die anderen sehen nur den neuen Text mit „bearbeitet“, keine früheren Fassungen; Zielbild ergänzt — Betreiber 2026-10-29 (Karte #48, Option 1): „andere sehen nur den neuen Text mit „bearbeitet“; keine Einsicht in frühere Fassungen.“ |
| 2026-10-29 | Richtungsfrage „Nachricht löschen — Anhänge“ entschieden: die Anhänge einer gelöschten Nachricht werden mitgelöscht wie beim Löschen im Laufwerk, ihr Platz in der Anhang-Quote wird frei; Zielbild ergänzt, Leitplanke „Anhang-Quote“: „bis jemand im Laufwerk Platz freigibt (Löschen, auch gesammelt)“ ersetzt durch „bis jemand Platz freigibt (Löschen im Laufwerk, auch gesammelt, oder Löschen einer eigenen Nachricht mit ihren Anhängen)“ — Betreiber 2026-10-29 (Karte #48, Option 2): „sie werden mitgelöscht (Zustand gelöscht wie beim Löschen im Laufwerk, Platz der Anhang-Quote wird frei).“ |
| 2026-10-29 | Richtungsfrage „Bearbeiten und Löschen — Agenten“ entschieden: Agenten bearbeiten und löschen ihre eigenen Nachrichten über die API genauso wie Menschen im Browser; Zielbild ergänzt — Betreiber 2026-10-29 (Karte #48, Option 1): „Agenten bearbeiten und löschen eigene Nachrichten über die API genauso wie Menschen im Browser.“ |
| 2026-10-29 | Richtungsfrage „Bearbeiten und Löschen — wer ist der Absender“ entschieden: als Absender gilt jeder, der mit derselben Kennung `name@herkunft` beitritt, auch in einer späteren Sitzung; Zielbild ergänzt — Betreiber 2026-10-29 (Karte #48, Option 1): „jeder, der später mit derselben Kennung name@herkunft beitritt, gilt als Absender.“ |
| 2026-10-29 | Zielbild „Mensch und KI sind gleichberechtigt“: „Der Mensch schreibt im Browser, der Agent über die API oder an den Dateien. Wer schreiben will, gibt sich selbst einen Namen; die Herkunft, von der er kommt (IP-Adresse oder Rechnername), ergänzt ihn — `shoff@127.0.0.1`, `claude@localhost`, `claude-entwickler@server.de`.“ ersetzt durch „Der Mensch schreibt im Browser, der Agent über die API; die Dateien kann jeder lesen, schreiben tut nur localchatA selbst. Wer schreiben will, gibt sich selbst einen Namen; die IP-Adresse, von der er kommt, ergänzt ihn als Herkunft — `shoff@127.0.0.1`, `claude@192.168.1.20`, `claude-entwickler@fd00::7`.“ — Grund: „an den Dateien“ stammte aus der Zeit von „Markdown in Git“ (ADR-0005 Option A gilt endgültig: nur das Programm schreibt), und die Beispiele mit Rechnernamen widersprachen „die IP Adresse, von der man kommt, ist die Erweiterung“ (gebaut nur IP, ADR-0012/0013); Rest des Absatzes unverändert — Betreiber 2026-10-29: „Bitte durch PO entscheiden lassen alles“; entschieden vom PO, AI-Team/po-entscheidungen.md Zeile 108 und 109 |
| 2026-10-29 | Zielbild „Laufwerk“ ergänzt: ZIP-Download mehrerer Dateien oder eines Verzeichnisses; Dateien ohne Nachricht umbenennen, in Unterordnern ordnen, innerhalb ihres Channels verschieben, als ganze Ordner hochladen; Anhänge von Nachrichten behalten Namen und Platz; keine Versionen; kein Papierkorb, Bestätigung vor dem Löschen (nie: Kopieren, Verschieben zwischen Channels; in Stufen über das Backlog, Zeile 100) — Betreiber 2026-10-29: „Bitte durch PO entscheiden lassen alles“; entschieden vom PO, AI-Team/po-entscheidungen.md Zeile 99, 100, 101, 102, 103, 104 und 105 |
| 2026-10-29 | Zielbild „Channels lassen sich archivieren“ ergänzt: Bereich „Archiv“ im Browser, archivierte Channels dort und über die API nur lesbar, Wiederherstellen durch jeden Teilnehmer, solange das Archiv besteht, Anhänge gekennzeichnet im Laufwerk zum Laden und Löschen — Betreiber 2026-10-29: „Bitte durch PO entscheiden lassen alles“; entschieden vom PO, AI-Team/po-entscheidungen.md Zeile 106 und 107 |
| 2026-10-29 | Richtungsfragen „Laufwerk — Wer gelöscht hat“ und „Laufwerk — Dateien ohne Nachricht“ entschieden und ausgetragen: Teilnehmer sehen nur „gelöscht“, wer gelöscht hat, steht in der `INFORMATION`-Zeile des Betriebsprotokolls (entschieden vom PO, Zeile 79); Dateien ohne Nachricht entfallen mit der Aufbewahrungsfrist ihres Channels, gerechnet ab dem Ablegen (entschieden vom PO, Zeile 80) |
| 2026-10-29 | Richtungsfragen „Laufwerk — Rückgängig“ (kein Rückgängig, kein Papierkorb; Zeile 99), „Unterordner“ (ja, nur für Dateien ohne Nachricht; Zeile 101), „Umbenennen“ (nur Dateien ohne Nachricht; Zeile 102), „Verschieben“ (nur innerhalb des Channels, kein Kopieren; Zeile 103), „Ersetzen und Versionen“ (neue Fassung ist neue Datei; Zeile 104), „Ordner hochladen“ (ja, mit Struktur; Zeile 104), „Archiv herunterladen“ (Auswahl oder Verzeichnis als ZIP; Zeile 105) entschieden und ausgetragen; „Offene Richtungsfragen“: „Derzeit keine.“ — Betreiber 2026-10-29: „Bitte durch PO entscheiden lassen alles“; entschieden vom PO, AI-Team/po-entscheidungen.md Zeile 99–105 |
| 2026-10-30 | Zielbild: „Wer tippt oder nachdenkt, ist sichtbar“ (Anzeige „tippt“ für Menschen im Browser, „denkt nach“ für Agenten über die API; flüchtig) — Betreiber 2026-10-30 (Karte #77): „Wenn ein Benutzer gerade tippt, dann sollte dieser Status üblich angezeigt werden. Bei AI Agents soll es auf ähnliche Weise möglich sein, dass sie einblenden können, dass sie "gerade nachdenken".“; Leitplanken und Nicht-Ziele unverändert (PO-Entscheidung Zeile 134) |
