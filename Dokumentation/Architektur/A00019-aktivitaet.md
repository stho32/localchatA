---
id: A00019
name: Aktivität
code: aktivitaet
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/aktivitaet/
verwandt: [A00010, A00011, A00004, A00014]
domaene: []
blackbox: eine Tabelle im Speicher mit Ablauf und zwei Eimern; Regeln vollständig in ADR-0051
---

# Aktivität

## Verantwortung

Führt je Kennung und Gespräch den flüchtigen Zustand `tippt` oder `denkt_nach` bis zu seinem Ablauf, gebremst gegen Flut.

## Schnittstellen

- `setzen(kennung, herkunft, gespraech, zustand, gueltig: Option<u32>) -> Result<Gesetzt{zustand, bis, jetzt}, Abweisung{Gueltigkeit | ZuViele | Gebremst{grenze: Gespraech | Herkunft, wieder_in}}>` — Standard `tippt` 6 s, `denkt_nach` 30 s, angebbar 1–120 s, ab Eingang, ersetzt den Eintrag dieser Kennung in diesem Gespräch; höchstens 16 Einträge je Kennung ([ADR-0051](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) Regeln 2, 3, 9).
- `beenden(kennung, gespraech)` — Löschen über die API und nach dem Senden; ohne aktiven Eintrag kein Ereignis.
- `beenden_alle(kennung)` — bei `weg` und bei `offline` einer Sitzung der Art `mensch` (Regel 7).
- `lesen(gespraech, ohne: &Kennung) -> {jetzt, Vec<{kennung, zustand, bis}>}` — nur gültige, nach Kennung sortiert, ohne den Aufrufer.
- Ausgehend: genau eine eingesetzte Funktion `Veroeffentlichen(Signal::Aktivitaet{channel, kennung, aktiv})`, gerufen unter der eigenen Sperre; den Bus kennt der Baustein nicht.
- Die Sichtbarkeit prüft die Integration an der [HTTP-Schnittstelle](A00004-http-schnittstelle.md) vor dem Aufruf (wie bei Senden und Verlauf).

## Abhaengigkeiten

Uhr-Provider ([Zeit](Aspekte/X00007-zeit.md)); Eimer-Typ aus dem [Schreibkonto](A00014-schreibkonto.md) (`src/schreibkonto/eimer.rs`, reine Mechanik, eigene Tabellen); `Kennung` und `Gespraech` aus [Gespräche](A00006-gespraeche.md). Gerufen von der [HTTP-Schnittstelle](A00004-http-schnittstelle.md) (Route, Senden) und von der Veröffentlichung der [Anwesenheit](A00010-anwesenheit.md)-Wechsel (Sperrfolge Anwesenheit → Aktivität); veröffentlicht auf den Signalbus des [Live-Kanals](A00011-live-kanal.md).

## Entscheidungen

- [ADR-0051: Aktivität „tippt“ / „denkt nach“ — flüchtig im Speicher mit Ablauf, eigener Signalbus neben dem Ereignisbus, eine Route je Gespräch für Mensch und Agent, eigene Bremse](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) — Akzeptiert, 2026-10-30
- [ADR-0055: Direktgespräche — Paar aus zwei vollen Kennungen, das Gegenüber im Pfad, Anlegen beim ersten Senden, eine Sichtbarkeitsregel für alle verteilenden Wege, `seit` als sichtbare Marke](../ADRs/0055-direktgespraeche-paar-aus-zwei-kennungen-gegenueber-im-pfad-anlegen-beim-ersten-senden-eine-sichtbarkeitsregel-sichtbare-marke.md) — Akzeptiert, 2026-10-30

## Offene Fragen

- Für den `product-owner` (Karte #77): beendet `offline` auch den Zustand eines Agenten? Entwurf nach Empfehlung: nein — für Agenten gilt `weg` (Gnadenfrist), weil ein Agent zwischen zwei Warte-Aufrufen immer `offline` ist ([ADR-0051](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) F5).

## Notizen / Quellen

- **Datenhoheit:** einziger Schreiber der Aktivitäten und ihrer Eimer; nur im Speicher, ein Neustart leert alles (Browser leeren bei neuem Strom). Nichts davon kommt in Tageslog, Verlaufsindex, Plattenquote oder Schreibkonto.
- **Nebenläufigkeit:** ein `std::sync::Mutex` um Tabelle und Eimer, kurze Operationen ohne Ein-/Ausgabe, kein `spawn_blocking`; ruft nie die Anwesenheit ([X00008](Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)).
- **Erster Test (Unit, gestellte Uhr, aufzeichnende Veröffentlichung):** P1–P3 in [ADR-0051](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) — `setzen(anna, allgemein, tippt, None)` → `bis = t0 + 6 s`; `lesen(ohne: ben)` bei t0 + 5,999 s → `[anna]`, bei t0 + 6 s → `[]`; Erneuerung bei t0 + 20 s für `denkt_nach` → t0 + 50 s; 11. Setzen in derselben Sekunde → `Gebremst{gespraech}`. Ersetzt: Uhr, Veröffentlichen.
- **Betrieb:** kein Start-Schritt, keine Einstellung, kein Health-Feld; Fehlerpfad 4xx als Debug, 429 als Episode `WARNING aktivitaet` (ADR-0042, Ergänzung Karte #77).
- Entworfen am 2026-10-30 (Karte #77, I0040/I0041) als eigener Baustein statt als Teil der Anwesenheit (ADR-0051 Option C verworfen: Stand holt nach, Teilnehmerliste flackert) oder des Schreibkontos (F6: PO Zeile 139).
- **Ergänzung 2026-10-30 (Karte #13, I0012), Soll:** Route im Direktgespräch `PUT|DELETE|GET /api/direkt/{kennung}/aktivitaet` ([ADR-0055](../ADRs/0055-direktgespraeche-paar-aus-zwei-kennungen-gegenueber-im-pfad-anlegen-beim-ersten-senden-eine-sichtbarkeitsregel-sichtbare-marke.md) Regel 3; ADR-0051 Regel 10 eingelöst). `aktivitaet::sichtbar` entfällt zugunsten von `sichtbar_fuer` aus [Sichtbarkeit](A00020-sichtbarkeit.md) — die heutige Fassung vergleicht Teile des kodierten Ordnernamens und verfehlt IPv6-Beteiligte. Das Signal trägt `gespraech` (schon heute) und wird im Strom nur an das Gegenüber ausgegeben (`direkt: <Gegenüber>`). Der Unit-Test der Regel aus B0686 wird durch D3 und den echten Fall in D8 ersetzt.
