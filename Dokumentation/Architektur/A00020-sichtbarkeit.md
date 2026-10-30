---
id: A00020
name: Sichtbarkeit
code: sichtbarkeit
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/
verwandt: [A00004, A00011, A00019, A00008]
domaene: []
blackbox: reine Werte und eine reine Funktion ohne Ein-/Ausgabe; die Regeln stehen vollständig in ADR-0055 Regeln 1, 2 und 5
---

# Sichtbarkeit

## Verantwortung

Entscheidet als einzige Stelle, welche Kennungen ein Gespräch sehen dürfen, über die kanonische Paarform der Direktgespräche.

## Schnittstellen

- `Kennung::kanonisch(text) -> Result<Kennung, _>` — `Kennung::bilden` plus Herkunft byte-gleich ihrer kanonischen `IpAddr`-Form ([ADR-0055](../ADRs/0055-direktgespraeche-paar-aus-zwei-kennungen-gegenueber-im-pfad-anlegen-beim-ersten-senden-eine-sichtbarkeitsregel-sichtbare-marke.md) Regel 2).
- `Direktpaar::bilden(a, b) -> Result<Direktpaar, Selbst>` (sortiert, verschieden), `beteiligt(&Kennung) -> bool`, `gegenueber(&Kennung) -> Option<&Kennung>`, `ordnername() -> String` (`:` als `%3A`), `aus_ordnername(&str) -> Option<Direktpaar>` (Regel 1).
- `sichtbar_fuer(gespraech: &Gespraech, kennung: &Kennung, archiviert: bool) -> bool` — Channel jeder, Direktgespräch genau die zwei Beteiligten, archiviert niemand (Regel 5). Gefragt von Strom und `warten` ([Live-Kanal](A00011-live-kanal.md)), Nachholen ([Gespräche](A00006-gespraeche.md)), Anhänge löschen ([HTTP-Schnittstelle](A00004-http-schnittstelle.md)), später Suche und Laufwerk; ersetzt `aktivitaet::sichtbar` ([Aktivität](A00019-aktivitaet.md)).

## Abhaengigkeiten

`Kennung` und `Name` (`src/gespraeche/kennung.rs`, `src/name.rs`); `Gespraech` aus der [Logablage](A00008-logablage.md) trägt ab I0012 ein `Direktpaar`. Keine Provider.

## Entscheidungen

- [ADR-0055: Direktgespräche — Paar aus zwei vollen Kennungen, das Gegenüber im Pfad, Anlegen beim ersten Senden, eine Sichtbarkeitsregel für alle verteilenden Wege, `seit` als sichtbare Marke](../ADRs/0055-direktgespraeche-paar-aus-zwei-kennungen-gegenueber-im-pfad-anlegen-beim-ersten-senden-eine-sichtbarkeitsregel-sichtbare-marke.md) — Vorgeschlagen, 2026-10-30

## Offene Fragen

- Für den `product-owner` (P2 in ADR-0055): bleibt ein Direktgespräch bei zwei Beteiligten? Entwurf nach Empfehlung: ja; `Direktpaar` wäre sonst eine sortierte Menge.

## Notizen / Quellen

- **Datenhoheit:** keine — reine Regel; die Gespräche gehören [Gespräche](A00006-gespraeche.md), die Sitzungen der [Anwesenheit](A00010-anwesenheit.md).
- **Grenzregel:** die Routen unter `/api/direkt/{kennung}/` fragen diese Regel **nicht** — dort bildet die Integration das Paar aus der Kennung der Sitzung und dem Pfad, ein fremdes Gespräch ist nicht adressierbar (ADR-0055 V1). Jeder Weg, der über mehrere Gespräche verteilt oder auflistet, fragt sie, mit vollständigem `match` über `Gespraech`.
- **Erster Test (Unit, rein, keine Provider):** D1–D3 in ADR-0055 — `Direktpaar::bilden(ben@10.0.0.8, anna@10.0.0.7).ordnername()` → `anna@10.0.0.7+ben@10.0.0.8`; `claude@fd00::7` → `claude@fd00%3A%3A7` und zurück; `aus_ordnername("b@1.1.1.1+a@1.1.1.1")` → `None`; `sichtbar_fuer(paar, anna@10.0.0.70)` → false; IPv6-Beteiligter → true.
- **Befund im Bestand (Ist):** `aktivitaet::sichtbar` (`src/aktivitaet/mod.rs:285`) vergleicht die Teile des **kodierten** Ordnernamens — ein Beteiligter mit IPv6-Herkunft sähe sein Gespräch nicht; `Dateiablage::gespraeche` (`src/gespraeche/logablage.rs:116`) lädt jeden Unterordner von `direkt/` ungeprüft. Beides löst der Typ `Direktpaar`.
- **Betrieb:** kein Start-Schritt, keine Einstellung, kein Health-Feld.
- Entworfen am 2026-10-30 (Karte #13, I0012) als eigener Baustein, weil fünf Wege (Strom, Warten, Nachholen, Anhänge, Aktivität) und zwei künftige (Suche, Laufwerk) dieselbe Antwort brauchen — Pre-Mortem 1 der ADR.
