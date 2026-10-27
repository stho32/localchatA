# VerlaufsMarken

Die Orientierungsmarken im Verlauf: Tagestrenner, Ungelesen-Linie, „neue Nachrichten“, Nachladen und Verbindungsstatus.

- **Tagestrenner**: zentriertes Datum zwischen zwei Haarlinien, `kd-ink-muted`, 12 px fett. „Heute“, „Gestern“, sonst „Montag, 26. Oktober 2026“. Der Trenner des obersten sichtbaren Tages bleibt beim Scrollen oben kleben.
- **Ungelesen-Linie** „Neu“: Linie und Wort in `lc-mention`, rechts vom Gutter. Steht vor der ersten Nachricht, die beim Öffnen des Channels ungelesen war; bleibt, bis man den Channel verlässt. Der Channel öffnet an dieser Linie (eine Bildschirmhöhe Kontext darüber), nicht am Ende.
- **„12 neue Nachrichten“**: Ist man nicht am Ende des Verlaufs, wird **nie** gescrollt. Neue Nachrichten zählen in einer Pille unten mittig hoch (dunkle Fläche `kd-header-bg`, damit sie über jedem Inhalt steht). Klick oder `Ende` springt zur ersten neuen. Steht man am Ende, folgt der Verlauf ohne Pille.
- **„Zum Neuesten“**: dieselbe Pille ohne Zahl, sobald man mehr als eine Bildschirmhöhe nach oben gescrollt hat — ständig erreichbar.
- **Nachladen nach oben**: ab etwa zwei Bildschirmhöhen vor dem oberen Ende werden die nächsten 100 Nachrichten geholt; oben erscheint eine 28-px-Zeile „Ältere Nachrichten werden geladen …“. Die Ansicht bleibt auf der Nachricht verankert, die man gerade liest — nichts springt.
- **Verbindung**: Verloren → 28-px-Leiste oben im Verlauf, ⚠ und Wort in `kd-danger`, Countdown bis zum nächsten Versuch; Lesen bleibt möglich, Gesendetes wird gepuffert und als „wird gesendet“ gezeigt. Wieder da → ✓ „Wieder verbunden · n nachgeladen“, verschwindet nach 4 s.
