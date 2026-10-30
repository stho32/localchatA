---
id: A00008
name: Logablage
code: logablage
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/logablage.rs
verwandt: [A00007, A00015]
domaene: []
blackbox: Provider zum Dateisystem ohne Fachlogik; Layout vollständig in ADR-0003
---

# Logablage

## Verantwortung

Bildet Gespräche, Tage, Anhänge und Einstellungen auf Dateien im Ordner ab.

## Schnittstellen

Port (Trait) mit zwei Implementierungen — Dateisystem (Produktion) und Speicher (Tests der Eltern):

- `gespraeche() -> Vec<GespraechsOrt>`, `tage(gespraech) -> Vec<Datum>`, `lesen(gespraech, tag) -> String`.
- `anhaengen(gespraech, tag, satz: &str)` — ein `write_all` auf eine im Anhängemodus offene Datei; Tageswechsel öffnet die nächste.
- `anhang_ablegen(gespraech, tag, name, bytes) -> Pfad` — erst temporär, dann `rename`; erst danach schreibt Gespräche den Satz, der darauf zeigt.
- `einstellungen_lesen/schreiben(gespraech)` (`einstellungen.toml`, schreiben über temporäre Datei + `rename`).
- `tag_entfernen(gespraech, tag)` — Logdatei und Anhang-Ordner des Tages.

## Abhaengigkeiten

Dateisystem. Kennt das [Logformat](A00007-logformat.md) nicht — liest und schreibt Sätze als fertigen Text.

## Entscheidungen

- [ADR-0003: Ablage-Layout im Ordner](../ADRs/0003-ablage-layout-im-ordner.md) — Akzeptiert, 2026-10-27
- [ADR-0011: Anhängen mit Rückbau bei Fehler und Zeilenende-Wächter](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md) — Akzeptiert, 2026-10-27
- [ADR-0022: Channel-Einstellungen als `einstellungen.toml` im Ordner des Channels](../ADRs/0022-channel-einstellungen-je-channel-als-toml.md) — Akzeptiert, 2026-10-28
- [ADR-0030: Entfallen beim Start, beim Tageswechsel im Sekundentakt und beim Verkürzen — Löschen der Tagesdatei unter der Gesprächssperre, dann Index](../ADRs/0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) — Akzeptiert, 2026-10-28
- [ADR-0032: Löschen nur nach Erlaubnisliste — reguläre Tagesdateien gültiger Channels, keine Symlinks; defekte Einstellungen heißen „nichts entfällt“](../ADRs/0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md) — Akzeptiert, 2026-10-28
- [ADR-0046: Anhänge ablegen — anmelden, gestreamt in einen Eingang hochladen, beim Senden in den Tag der Nachricht binden, eine Metazeile mit Anhang-ID; ohne Grenze je Datei unter der Quote, eine Reserve bleibt dem Text](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) — Akzeptiert, 2026-10-29
- [ADR-0047: Anhänge ausliefern — eine Route ohne Sitzung über die Anhang-ID, die Art bestimmt der Server, inline nur Gefahrloses unter `CSP: sandbox`, alles andere als Download; Bildvorschau aus dem Original, Video und Audio im Player des Browsers mit Range](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) — Akzeptiert, 2026-10-29
- [ADR-0055: Direktgespräche — Paar aus zwei vollen Kennungen, das Gegenüber im Pfad, Anlegen beim ersten Senden, eine Sichtbarkeitsregel für alle verteilenden Wege, `seit` als sichtbare Marke](../ADRs/0055-direktgespraeche-paar-aus-zwei-kennungen-gegenueber-im-pfad-anlegen-beim-ersten-senden-eine-sichtbarkeitsregel-sichtbare-marke.md) — Akzeptiert, 2026-10-30

## Offene Fragen

## Notizen / Quellen

- Erster Test (Integration gegen `tempfile`): zwei Sätze an `channels/allgemein/2026-10-27.log` anhängen, `lesen` liefert beide in Reihenfolge; `tag_entfernen` löscht Log und `anhaenge/2026-10-27/`, lässt andere Tage stehen.
- Kein `fsync` je Nachricht: ein Stromausfall kann die letzten Sekunden kosten; ein Absturz des Programms nicht (Daten liegen im Seitencache des Systems).
- **Ergänzung 2026-10-27 (Karte #2, I0002):** `anhaengen` merkt die Länge vor dem Schreiben und schneidet bei Fehler darauf zurück; beim Öffnen einer Tagesdatei prüft es das letzte Byte und beginnt mit `\n`, wenn die Datei nicht auf `\n` endet ([ADR-0011](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md)). Das ist das einzige Stück Format, das die Logablage kennt. Die Speicher-Attrappe des Ports kann „Fehler nach n Bytes“ simulieren, damit Gespräche den Fehlerpfad ohne Platte testet.
- Tagesdatei gewählt nach dem lokalen Datum der **Nachrichtenzeit**, nicht nach „jetzt“ — rotiert wird beim ersten Satz eines neuen Tages, ein eigener Mitternachts-Takt ist für I0002 nicht nötig (Entfallen alter Tage gehört zu I0018).
- Ein Gespräch ist ein Ordner unter `channels/` bzw. `direkt/`; fehlt `einstellungen.toml`, gelten Standardwerte (Anzeigename = Slug, Frist 7 Tage). Laden legt keine Einstellungen an — geschrieben werden sie erst mit I0007/I0018.
- **Ergänzung 2026-10-28 (Karte #6, I0007):** Der Port wächst um `channel_ordner_anlegen(name)` (`create_dir`, scheitert mit „vorhanden“, wenn der Ordner existiert — kein Überschreiben, [ADR-0021](../ADRs/0021-channel-anlegen-jeder-mit-sitzung-grenzen-je-herkunft-und-gesamt.md)), `einstellungen_lesen(gespraech) -> Option<Vec<u8>>`, `einstellungen_schreiben(gespraech, bytes)` (`einstellungen.toml.neu` + `rename`) und `ordner_entfernen(gespraech)` (nur für den Rückbau eines eben angelegten, sonst leeren Ordners). Das TOML kennt sie nicht ([Einstellungsformat](A00013-einstellungsformat.md)). `gespraeche()` liefert weiter alle Unterordner; welche davon Channels sind, entscheidet [Gespräche](A00006-gespraeche.md) nach [ADR-0020](../ADRs/0020-channelname-ist-ordnername-erlaubnisliste.md). Die Speicher-Attrappe bildet `create_dir` (vorhanden → Fehler) und „Einstellungen schreiben scheitert“ nach. Erster Test (Integration gegen `tempfile`): `channel_ordner_anlegen` zweimal → zweites Mal „vorhanden“; `einstellungen_schreiben` hinterlässt genau `einstellungen.toml`, keine `.neu`-Datei, und ersetzt eine vorhandene.
- **Ergänzung 2026-10-28 (Karte #12, I0018):** `tag_entfernen(gespraech, tag)` bildet den Pfad aus `Gespraech` und Datum, prüft mit `symlink_metadata`, dass dort eine reguläre Datei liegt, und ruft `remove_file`; fehlt die Datei, ist das Erfolg; liegt dort etwas anderes, ein Fehler ohne Löschen ([ADR-0032](../ADRs/0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md)). Kandidaten liefert das unveränderte `tage` (nur reguläre Dateien `JJJJ-MM-TT.log`, gebaut und geprobt: Symlinks und Ordner mit Tagesnamen sind nicht dabei). Ab I0010 danach `anhaenge/<tag>/` mit `remove_dir_all` (folgt keinen Symlinks, geprobt). Die Speicherablage bildet „Löschen scheitert“ nach. Erster Test (Integration gegen `tempfile`): ADR-0032, „Folgeentscheidungen“.
- **Ergänzung 2026-10-28 (Vorlauf „Betriebsprotokoll“):** `<ordner>/logs/` gehört nicht zur Logablage — sie liest weiter nur `channels/` und `direkt/` (`src/gespraeche/logablage.rs`), ein Channel namens `logs` liegt unter `channels/logs/`; kein Konflikt. Das Betriebsprotokoll ([A00015](A00015-betriebsprotokoll.md)) hat eigene Dateien, eigene Aufbewahrung (14 Tage) und einen eigenen Schreiber. Ladedefekte, Rückbaufehler und Löschfehler dieses Bausteins werden Protokolleinträge nach [ADR-0042](../ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) statt `eprintln!`.
- **Ergänzung 2026-10-29 (Karte #9, I0010 — vorgeschlagen):** `anhang_ablegen(gespraech, tag, name, bytes)` (Stand I0001) wird ersetzt durch den Weg über den Eingang ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md)): `eingang_oeffnen(id) -> Schreiber` (`.eingang/<id>.teil`, `O_CREAT|O_EXCL`), `eingang_abschliessen(id)` (`rename` auf `.eingang/<id>`), `eingang_loeschen(id)`, `eingang_leeren()` (nur reguläre Dateien, beim Start), `anhang_uebernehmen(id, gespraech, tag, datei)` (`create_dir_all` für `anhaenge/<tag>/`, dann `rename`; scheitert mit „vorhanden“, nie überschreiben), `anhang_zuruecklegen(gespraech, tag, datei, id)`, `anhang_bytes(gespraech, tag) -> u64` (Σ `symlink_metadata` regulärer Dateien), `anhang_oeffnen(gespraech, tag, datei) -> File` (letzte Stelle `O_NOFOLLOW`, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Regel 4). `tag_entfernen` löscht danach wie gehabt `anhaenge/<tag>/` (ADR-0032 Regel 7). Die Speicherablage bildet „Fehler nach n Bytes“ auch für den Eingang und „`rename` scheitert“ für die Übernahme nach. Erster Test (Integration gegen `tempfile`): Eingang schreiben, abschließen, übernehmen → Datei unter `channels/a/anhaenge/<tag>/`, `.eingang/` leer; `anhang_bytes` = Größe; ein Symlink in `anhaenge/<tag>/` zählt nicht und wird von `anhang_oeffnen` abgewiesen; `eingang_leeren` lässt einen Symlink und sein Ziel stehen.
- **Ergänzung 2026-10-29 (Karte #9, I0010 — entschieden nach den Antworten des Betreibers B1–B5; ADR-0046/0047 Akzeptiert):** Die vorgeschlagene Ergänzung darüber gilt; dazu `platz_frei() -> u64` (`statvfs` des Ordners über `rustix`, Merkmal `fs`; `f_bavail × f_frsize`) für die Plattenprüfung der Anmeldung ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 1.5) — die Speicherablage liefert einen gestellten Wert; und `anhang_oeffnen` liefert eine Datei, auf der die Auslieferung für Teilabrufe den Anfang setzt ([ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Regel 4). `eingang_oeffnen` schreibt Stücke so, wie sie kommen (≤ 1 MiB), ohne `fsync` — wie das Anhängen der Sätze (Plattenprobe in [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md)). Erster Test zusätzlich: `platz_frei` gegen `tempfile` > 0.
- **Ergänzung 2026-10-29 (Karte #9 Security-Gate, [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Security-Gate“ Regeln 1–4), Soll:** die Logablage hält ab dem Start einen Deskriptor des Instanzordners und einen des Eingangs (`.eingang`, nach der Prüfung beim Start); jeder Zugriff unter `<gespräch>/anhaenge/` und im Eingang — Kürzen beim Löschen, Öffnen zum Ausliefern, Teildatei, `rename` beim Binden und Zurücklegen, `stat` beim Laden, Entfallen eines Tags — geht komponentenweise über `openat(…, O_DIRECTORY | O_NOFOLLOW)` und relative Aufrufe (`mkdirat`, `renameat2`, `linkat`, `unlinkat`, `fstatat`), nie über einen vollen Pfad; Komponenten nur aus Typen (nie leer, `.`, `..`, `/`). Entfallen der Anhänge eines Tags über Deskriptoren statt `remove_dir_all` (ersetzt ADR-0032 Regel 7). `rustix` braucht dafür das Merkmal `fs` (keine neue Abhängigkeit). Matrix M45′.
- **Ergänzung 2026-10-29 (Karte #9 Nacharbeit, [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „`channels` muss ein echter Ordner sein“):** die Wurzeln `channels`, `direkt` und `.eingang` unter dem Instanzordner sind echte Ordner (Start-Prüfung, Exit 2); nur der Instanzordner selbst darf ein Symlink sein.
- **Ergänzung 2026-10-30 (Karte #13, I0012), Soll:** `gespraeche()` liefert einen Ordner unter `direkt/` nur, wenn `Direktpaar::aus_ordnername` ihn annimmt ([ADR-0055](../ADRs/0055-direktgespraeche-paar-aus-zwei-kennungen-gegenueber-im-pfad-anlegen-beim-ersten-senden-eine-sichtbarkeitsregel-sichtbare-marke.md) Regel 1, E12; [Sichtbarkeit](A00020-sichtbarkeit.md)); sonst übergangen mit `WARNING laden` (`ordner` gekürzt) — heute lädt `src/gespraeche/logablage.rs:116` jeden Unterordner ungeprüft. Neu: `direkt_ordner_anlegen(paar)` (legt `direkt/` und `direkt/<ordnername>/` an, vorhanden ist Erfolg, Symlink oder Nicht-Ordner ist Fehler — Regel aus ADR-0046 Ergänzung „`channels` muss ein echter Ordner sein“ gilt für `direkt`). `tag_entfernen` und die Erlaubnisliste aus ADR-0032 gelten für Direktgespräche unverändert. Erster Test: D4 (tempfile). Die Pflege entfernt leere Paarordner nur mit `rmdir` von innen nach außen, ohne Symlink in einer Komponente (Regel 10, B1; D5).
