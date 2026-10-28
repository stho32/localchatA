# ChannelEinstellungen

Der modale Dialog für die Einstellungen des geöffneten Channels; heute nur die Aufbewahrungsfrist (ADR-0033). Geöffnet über `# name ▾` in der [Kopfzeile](../Kopfzeile/README.md). Gleiche Gestalt wie „Channel anlegen“ (4.3): `<dialog>` mit Titel, Feldern, Primärschalter `kd-accent` und „Abbrechen“; eckig, ohne Schatten.

- **Titel** „# name — Einstellungen“ in `lc-title`.
- **Feld** „Aufbewahrungsfrist“: Zahl (nur Ziffern, 1–3650), dahinter das Wort „Tage“; vorbelegt mit der geltenden Frist. Hinweis darunter in `kd-ink-muted`: „Ältere Tage entfallen. Heute zählt mit. Standard: 7 Tage.“
- **Schritt 1** — Schalter „Speichern“ (Primär) und „Abbrechen“. `Enter` speichert, `Esc` schließt; der Fokus kehrt zu `# name` zurück. Fehler (ungültige Zahl, Netz, defekte Datei) als Fehlerzeile in `kd-danger` mit ⚠ und Wort, Feld mit `aria-invalid`; die Eingabe bleibt stehen.
- **Schritt 2 — nur beim Verkürzen, wenn Tage entfallen würden**: das Feld wird schreibgeschützt, darunter in `kd-ink` der Satz „Bei 7 Tagen entfallen sofort 23 Tage bis einschließlich 21. Oktober 2026 mit 4.210 Nachrichten. Das lässt sich nicht rückgängig machen.“ Schalter „Zurück“ (**Fokus hier**, sicherer Standard) und „Frist verkürzen“ (Primär). Zahlen deutsch („4.210“), Datum ausgeschrieben, Singular bei 1.
- **Gesperrt**: ist die Einstellungsdatei defekt, ist das Feld gesperrt, und der Dialog sagt „⚠ Die Einstellungsdatei `channels/name/einstellungen.toml` ist defekt — in diesem Channel entfällt nichts. Von Hand reparieren und localchatA neu starten.“ — Pfad in Mono.
- **Live**: ändert jemand anders die Frist, während der Dialog offen ist, bleibt die Eingabe; der Hinweis ergänzt „Inzwischen geändert: 30 Tage.“
- **Rückmeldung** nach dem Speichern in der Statuszeile (`role="status"`): „Frist für # name: 30 Tage.“ bzw. „Frist für # name: 7 Tage — 23 Tage mit 4.210 Nachrichten entfallen.“

Der Verbraucher liefert: Channelname, geltende Frist (oder „gesperrt“ mit Dateipfad). Kein Artboard; Prüfung durch den `ux-reviewer` gegen Tokens und diese Beschreibung.
