---
id: A00014
name: Schreibkonto
code: schreibkonto
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/
verwandt: [A00004, A00010]
domaene: []
---

# Schreibkonto

## Verantwortung

Entscheidet je Schreibvorgang anhand zweier Token-Eimer (Kennung, Herkunft), ob er jetzt ausgeführt werden darf.

## Schnittstellen

- `abbuchen(kennung: &Kennung, herkunft: IpAddr, kosten: u32) -> Result<(), Gebremst>` — prüft beide Eimer und bucht nur, wenn beide reichen (alles oder nichts, unter einer Sperre); `Gebremst { grenze: Kennung | Herkunft, wieder_in: Duration }` ([ADR-0036](../ADRs/0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md) Regeln 1–4).
- `kosten_senden(bytes) -> u32` — ⌈bytes / 4.096⌉, mindestens 1.
- `gebremst() -> u64` — Zahl der Bremsungen seit dem Start, für `/api/gesundheit`.
- Konstanten: je Kennung Vorrat 30, Nachschub 1/s; je Herkunft Vorrat 120, Nachschub 8/s.

Aufgerufen von der [HTTP-Schnittstelle](A00004-http-schnittstelle.md) nach allen 4xx-Prüfungen und vor dem Aufruf des Fachbausteins; Fachbausteine kennen das Konto nicht.

## Abhaengigkeiten

Uhr-Provider ([Zeit](Aspekte/X00007-zeit.md)); Kennung aus [Gespräche](A00006-gespraeche.md) (`Kennung`), Herkunft aus dem Extraktor `Teilnehmer` ([Anwesenheit](A00010-anwesenheit.md), [ADR-0012](../ADRs/0012-identitaet-und-sitzung.md)).

## Entscheidungen

- [ADR-0036: Schreibkonto gegen Flut — Token-Eimer je Kennung und je Herkunft, Kosten nach Größe, 429 mit Wartezeit; zuerst für das Senden](../ADRs/0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md) — Akzeptiert, 2026-10-28
- [ADR-0044: Plattenquote — die Gesamtgröße der Gespräche ist begrenzt (`--plattenquote`, Standard 4 GiB), gezählt am Verlaufsindex; an der Quote wird Senden mit 507 abgewiesen, gelöscht wird deswegen nichts](../ADRs/0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) — Akzeptiert, 2026-10-29

## Offene Fragen

- ~~Bucht auch `PUT /api/channels/{name}/frist`?~~ Entschieden 2026-10-28 (Karte #31): ja, pauschal 10 Einheiten am selben Konto — [ADR-0036](../ADRs/0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md), Ergänzung Karte #31 (b).
- ~~Plattenquote (Deckel für die Gesprächslogs): Frage an den Betreiber aus Karte #31; die Bremse verlangsamt nur (≤ 5,7 GB/Tag je Herkunft, ADR-0036 Ergänzung Karte #31 (a)).~~ Entschieden 2026-10-29 (Karte #34, Betreiber: „Ja, als Quote“): [ADR-0044](../ADRs/0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) — der Zähler liegt in [Gespräche](A00006-gespraeche.md), nicht hier; das Konto bestimmt nur, wie schnell die Quote erreicht wird.

## Notizen / Quellen

- Datenhoheit: einziger Halter der Eimer, nur im Speicher; ein Neustart füllt alle Eimer. Volle Eimer werden entfernt; die Tabelle bleibt durch die Sitzungsgrenzen (512 gesamt) begrenzt (ADR-0036 Regel 6).
- Erster Test (Unit, rein, gestellte Uhr): siehe ADR-0036, „Folgeentscheidungen“ — 30/31 je Kennung, 120/121 je Herkunft, Nachschub als Spanne (999 ms gebremst, 1.000 ms frei), alles oder nichts, Uhr zurück ohne Nachschub. Ersetzt: Uhr.
- Betrieb: kein Start-Schritt, keine Konfiguration (Konstanten im Code); Gesundheit über `gebremst`; Fehlerpfad 429 mit `Retry-After` und eine stderr-Zeile je gebremster Kennung und Episode.
- Entworfen am 2026-10-28 (Karte #8, I0009) als eigener Baustein statt als Teil der Anwesenheit: das Konto gilt je Kennung über Sitzungen hinweg (Regel 5 — `verlassen` setzt es nicht zurück) und für Routen, die nichts mit Anwesenheit zu tun haben.
- **Ergänzung 2026-10-28 (Karte #31):** (1) Plattenrechnung berichtigt — die Einrückung des Logformats (A00007) macht aus einem Text bis zu doppelt so viele Bytes in der Datei; Kosten bleiben nach Textbytes, der Höchstwert je Herkunft liest sich ≈ 5,7 GB/Tag. (2) `PUT …/frist` bucht 1 vorab + 9 nach den Prüfungen; `vorab` nennt die vollen Kosten für `wieder_in` (Senden 1, Frist 10); 429-Text „Zu viele Änderungen …“; Episode „Frist gebremst: zu viele Änderungen“. (3) Die Episode der Bremse zählt je Grenze (`art`), kein `gebremst_herkunft` im Health-Check. Quelle: [ADR-0036](../ADRs/0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md), [ADR-0042](../ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), [ADR-0043](../ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md), je Ergänzung Karte #31.
