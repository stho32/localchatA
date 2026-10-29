---
id: A00017
name: Anhangeingang
code: anhangeingang
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/anhangeingang.rs
verwandt: [A00008, A00014, A00018]
domaene: []
---

# Anhangeingang

## Verantwortung

Hält angemeldete und hochgeladene, noch nicht gesendete Dateien je Absender mit ihrer Reservierung an der Quote bis zur Bindung an eine Nachricht oder bis zu ihrer Frist bereit.

## Schnittstellen

- `anmelden(kennung, herkunft, name, bytes) -> Result<Anmeldung { id, anlauf_bis }, Abweisung>` — prüft offene Hochladungen (4 je Kennung, 16 je Herkunft, 32 gesamt), den freien Platz der Platte (Abstand 1 GiB) und reserviert `blockmass(bytes)` an der Quote über Gespräche gegen `hoechstens − reserve` ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 1, Schritte 4–6).
- `beginnen(kennung, id, laenge) -> Result<Platz, Hochladefehler>` — `unbekannt | abgelaufen | laeuft | fertig | laenge`; Zustand `angemeldet` → `laeuft`.
- `Platz::schreiben(stueck)`, `Platz::abschliessen() -> Eingangseintrag { id, name, bytes, art, masse, gueltig_bis }` — schreibt nach `.eingang/<id>.teil`, dann `rename` nach `.eingang/<id>`; Abbruch (Drop ohne Abschluss) löscht die Teildatei, Zustand zurück auf `angemeldet` mit neuer Anlauffrist 60 s ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 2.5).
- `verwerfen(kennung, id)` — `DELETE`: Teildatei oder Datei weg, Reservierung frei, laufender `Platz` endet als Abbruch; fremde und unbekannte IDs: ohne Wirkung.
- `nehmen(kennung, ids) -> Result<Vec<Eingangseintrag>, Anhangfehler>` — beim Senden unter der Schreibsperre; `unbekannt | abgelaufen | fremd | doppelt | unvollstaendig`.
- `zuruecklegen(eintraege)` — nach gescheitertem Anhängen des Satzes ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 6).
- `ablaufen(jetzt)` — im Sekundentakt der Pflege (`angemeldet` 60 s, `fertig` 15 min); `leeren()` beim Start; `offen() -> (anzahl, reserviert)` für den Health-Check.

## Abhaengigkeiten

[Logablage](A00008-logablage.md) (Port wächst um den Eingang und `platz_frei`), [Anhangprüfung](A00018-anhangpruefung.md) (Art und Maße aus den ersten Stücken, Text-Prüfung über alle), Uhr- und Zufalls-Provider (`src/uhr.rs`, `src/zufall.rs`) für Fristen und ID. Das Schreiben der Stücke ruft die HTTP-Schnittstelle in `spawn_blocking` unter der Hochladebank ([A00004](A00004-http-schnittstelle.md)) — der Baustein selbst kennt keine Laufzeit.

## Entscheidungen

- [ADR-0046: Anhänge ablegen — anmelden, gestreamt in einen Eingang hochladen, beim Senden in den Tag der Nachricht binden, eine Metazeile mit Anhang-ID; ohne Grenze je Datei unter der Quote, eine Reserve bleibt dem Text](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) — Akzeptiert, 2026-10-29

## Offene Fragen

- keine — F1 neu beantwortet 2026-10-29 (eigene gemeinsame Anhangquote, kein Anteil je Herkunft, **kein Verdrängen**: voll heißt abgewiesen bis zum Löschen; ADR-0046 Ergänzung „Karte #9 Betreiber F1′“). F5 entfällt.

## Notizen / Quellen

- Datenhoheit: `<ordner>/.eingang/` gehört allein diesem Baustein (über den Port der Logablage); `reserviert` hält Gespräche ([ADR-0044](../ADRs/0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) Regel 2 bleibt: ein Halter der Quote).
- Keine Grenze je Datei (Betreiber B2); die Grenze ist der freie Platz für Anhänge und der Platte.
- Erster Test (Unit, Speicherablage, gestellte Uhr, fester Zufall, gestellter freier Platz): `anmelden` 200 Bytes → Anmeldung, `reserviert` = 4.096; `beginnen` mit Länge 199 → `laenge`; `beginnen` 200, drei Stücke, `abschliessen` → Eintrag mit `gueltig_bis` = jetzt + 15 min; zweites `beginnen` → `fertig`; `nehmen` mit fremder Kennung → `fremd`; zweimal `nehmen` → beim zweiten `unbekannt`; Anmeldung ohne `beginnen`, Uhr + 60 s + 1 ms, `ablaufen` → Reservierung frei; `Platz` nach zwei Stücken fallen lassen → keine `.teil`-Datei, Zustand `angemeldet`, Anlauffrist neu; `verwerfen` während `laeuft` → Teildatei weg, Reservierung 0; fünfte offene Anmeldung derselben Kennung → `offen_kennung`; freier Platz = Länge + 1 GiB − 1 → `platte`. Ersetzte Provider: Logablage, Uhr, Zufall, Plattenauskunft.
- Betrieb: `.eingang/` wird beim Start vor dem Laden geleert; Symlinks und Unterordner dort bleiben liegen und werden als `WARNING` gemeldet; Health-Felder `reserviert`, `hochladungen` ([ADR-0043](../ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md), Ergänzung Karte #9).
- **Ergänzung 2026-10-29 (Karte #9 Gate, [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Gate“), Soll:** jeder Eintrag trägt ein `lebensende` = Anmeldung + 60 s + 2 × `bytes` / 1 MiB/s, gesetzt nur in `anmelden`; Anlauffrist (auch nach Abbruch) und Gesamtfrist eines `PUT` sind höchstens das Lebensende; `anmelden` antwortet zusätzlich `hochladen_bis`. `offen_gesamt` zählt nur `laeuft` (höchstens 64), geprüft in `anmelden` und in `beginnen`; je Kennung 4 und je Herkunft 16 zählen alle drei Zustände. Die Einträge liegen mit `reserviert` unter der Schreibsperre (eine Sperre für alle Übergänge); wer einen Übergang verliert, räumt seine Datei. Jedes `rename` mit `RENAME_NOREPLACE`; `.eingang` als Symlink oder Nicht-Ordner → Exit 2. Erster Test dazu: ADR-0046, Ergänzung „Karte #9 Gate“, „Erste Tests“ (Proben M20, M21).
- **Ergänzung 2026-10-29 (Karte #9 Betreiber F1 und F4), Soll:** jeder Eintrag trägt neben dem Anspruch `z(bytes)` (bei der Anmeldung, gegen die **Anhangquote** `W + reserviert + z ≤ Q`, ohne Textreserve, ohne Anteil je Herkunft) den **beanspruchten** Teil; der `Platz` beansprucht vor dem Schreiben abschnittsweise (64 MiB) über Gespräche und kürzt die Opfer außerhalb der Sperre, bevor er schreibt ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Betreiber F1“ Regeln 3–4). `anmelden` neu: 422 `zu_gross`, 507 `quote` nur mit `grund: "hochladungen"` und `Retry-After: 30`, 201 mit `verdraengt_voraussichtlich`. `abschliessen` tilgt vor dem `rename` die Metadaten über die [Anhangprüfung](A00018-anhangpruefung.md) bei gleicher Länge; nicht tilgbar → 422 `metadaten`, Eintrag verworfen ([ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Betreiber F4“). Eingangseinträge werden nie verdrängt.
- **Ergänzung 2026-10-29 (Karte #9 Betreiber F1′), Soll — ersetzt „beansprucht“ und Abschnitte der Ergänzung „F1 und F4“:** `anmelden` beansprucht die ganze Länge (`A += z(bytes)`) gegen `G + A + z ≤ Q` über Gespräche; Abweisungen 422 `zu_gross`, 507 `voll` (ohne `Retry-After`) und 507 `hochladungen` (`Retry-After: 30`); kein `verdraengt_voraussichtlich`. Der `Platz` schreibt ohne Abschnitte und prüft die Quote nicht; die Platte prüft `anmelden` gegen die noch zu schreibenden Bytes aller Einträge. Einträge des Eingangs sind für die Route Löschen `unbekannt`; Abbrechen bleibt `verwerfen`. Erster Test: [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Betreiber F1′“, „Erste Tests“ (Anmeldungen 507 `voll`/`hochladungen`, PUT ohne Quote-Prüfung, Mutante „PUT prüft die Quote“).
