---
id: X00001
name: Logging und Diagnose
art: Aspekt
geltung: Makro
befund: Ungeklaert
hausantwort:
betrifft: [A00003, A00006, A00004, A00014, A00015]
herkunft: dialog
quellen:
  - src/
---

# Logging und Diagnose

## Hausantwort

Keine App-Architektur-Vorlage zugeordnet (`AI-Team/architekt.md` nennt keine); Vorbild kanbanD schreibt Startzeile auf stdout und Fehler mit `eprintln!` auf stderr.

## Systemantwort

Geplant: genau eine Startzeile auf stdout (`localchatA läuft für <ordner> unter <adresse>`, maschinenlesbar für Tests und Director) plus eine Zeile mit Kennzahlen (Nachrichten, Loggröße, Ladezeit, defekte Zeilen); Fehler und Warnungen einzeilig auf stderr mit Ort (Datei:Zeile im Log bei defekten Sätzen). Keine Nachrichteninhalte in Diagnosezeilen. Durchsetzung: Review-Punkt — `println!` nur in `start`, sonst `eprintln!` über eine Funktion in `fehler`.

## Abweichungen

## Offene Fragen

- ~~Reicht stderr, oder soll das Programm eine eigene Diagnosedatei schreiben? Empfehlung: stderr — der Ordner gehört den Gesprächen.~~ Beantwortet vom Betreiber 2026-10-28 (`AI-Team/umgebung.md`, „Logging-Konzept“): Betriebsprotokoll als Datei unter `<ordner>/logs/` — siehe Ergänzung unten.

## Notizen / Quellen

Entworfen am 2026-10-27; `Ungeklaert`, bis Code existiert.

- **Ergänzung 2026-10-28 (Vorlauf „Betriebsprotokoll“, Vorgabe des Betreibers):** Die Systemantwort wird erweitert, nicht ersetzt: (1) **Ziel**: Betriebsprotokoll `<ordner>/logs/JJJJ-MM-TT.log` nach lokalem Datum der Eintragszeit, eine escapte Zeile je Eintrag `<zeit> <STUFE> <bereich> <text> | schlüssel=wert …`, Kontext nur über eine Erlaubnisliste, nie Token, Nachrichtentexte oder Körper; stderr spiegelt ab Warning; stdout bleibt allein der Startzeile; vor der Sperre nur stderr ([ADR-0040](../../ADRs/0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md)). (2) **Rotation und Aufbewahrung**: eigenes Modul [A00015](../A00015-betriebsprotokoll.md) ohne neue Crate (Probe `tracing-appender`: UTC-Datum, Uhr nicht ersetzbar, Aufbewahrung nach Dateizahl), Bereinigung nach 14 Kalendertagen beim Start und über einen gemeinsamen Tagestakt ([ADR-0041](../../ADRs/0041-betriebsprotokoll-als-eigenes-modul-rotation-beim-schreiben-bereinigung-14-tage.md)). (3) **Stufen** Debug · Information · Warning · Error (Operation scheitert erwartbar) · Exception (Programmfehler, Panik-Haken), Ereigniskatalog, 4xx einzeln nur Debug, Abweisungen mit Betriebsbedeutung als Episode, Tagesbudget je Stufenklasse; wo ältere ADRs „eine Zeile auf stderr“ sagen, gilt ein Protokolleintrag der Stufe aus dem Katalog ([ADR-0042](../../ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md)). (4) **Einstellung und Monitoring**: `--protokoll-stufe`, `LOCALCHATA_PROTOKOLL_STUFE`, Standard Information; Health-Check mit `warnings`, `errors`, `exceptions`, `letzte_exception`, `protokoll_ok`, `aufgaben_ausgefallen` ([ADR-0043](../../ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md)). **Durchsetzung** neu: Review-Punkt des `observability-reviewer` (Auto-Review je Paket mit Fehlerpfaden) — `eprintln!`/`println!` nur in `src/protokoll/` und für die Startzeile, jeder Fehlerpfad mit Eintrag der Katalog-Stufe; Integrationstest „Token nie im Protokoll“; später `clippy::disallowed_macros` für `eprintln`. Befund bleibt `Ungeklaert`, bis das Modul gebaut ist.
