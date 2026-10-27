---
id: A00010
name: Anwesenheit
code: anwesenheit
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/anwesenheit/
verwandt: [A00011]
domaene: []
blackbox: ein Zustandsautomat im Speicher; Regeln in ADR-0012 und ADR-0013, Verfeinerung mit I0004–I0006
---

# Anwesenheit

## Verantwortung

Führt je Sitzung einen eindeutigen Namen samt Status online, offline oder weg.

## Schnittstellen

- `beitreten(name: Name, herkunft: IpAddr, art) -> Result<Sitzung{token, kennung}, NameVergeben{belegt_von, seit, vorschlag} | ZuVieleSitzungen>` — `Name` ist schon nach [ADR-0013](../ADRs/0013-namensregel-als-erlaubnisliste.md) geprüft; Herkunft kanonisch aus der Annahmeschleife ([ADR-0012](../ADRs/0012-identitaet-und-sitzung.md)).
- `wiederkehren(token, herkunft) -> Option<Sitzung>` — gültiges Token derselben Herkunft; zählt als Lebenszeichen. Grundlage von `beitreten` mit Header und vom Extraktor `Teilnehmer` an der [HTTP-Schnittstelle](A00004-http-schnittstelle.md).
- `verbunden(token) -> Halter` (ab I0005) — solange ein Halter lebt (SSE-Strom oder Warte-Aufruf, [ADR-0007](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md)), ist die Sitzung online; sein `Drop` ist ein Lebenszeichen.
- `pflegen(jetzt)` (ab I0006) — ohne Halter und länger als die Gnadenfrist ohne Lebenszeichen → weg, Name frei, Ereignis. Bis dahin wird „weg“ beim Zugriff berechnet.
- `teilnehmer() -> Vec<Teilnehmer{kennung, art, status, offline_seit}>` (ab I0005).
- Ausgehend: Ereignis `Anwesenheit` an den [Live-Kanal](A00011-live-kanal.md) (ab I0005).

## Abhaengigkeiten

Uhr-Provider (Aspekt [Zeit](Aspekte/X00007-zeit.md)); Provider `Zufall` für Tokens (Betriebssystem-Zufall über `getrandom`, im Test feste Folge); [Live-Kanal](A00011-live-kanal.md).

## Entscheidungen

- [ADR-0008: Identität — Herkunft aus der Peer-Adresse, Nachweis über Sitzungs-Token](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md) — Abgelöst durch ADR-0012, 2026-10-27
- [ADR-0012: Identität und Sitzung — Herkunft aus der Gegenstelle, Sitzungs-Token im Speicher, eine Sitzung je Browser](../ADRs/0012-identitaet-und-sitzung.md) — Akzeptiert, 2026-10-27
- [ADR-0013: Namensregel als Erlaubnisliste — `[a-z0-9-]`, 1 bis 32 Zeichen, abweisen statt umformen](../ADRs/0013-namensregel-als-erlaubnisliste.md) — Akzeptiert, 2026-10-27

## Offene Fragen

- Gnadenfrist für Agenten: Standard 60 s für alle, ein Agent verlangt beim Beitreten bis zu 3600 s (`gnadenfrist_s`)? Empfehlung: ja, mit I0014. Fachlich, für den `planer`.

## Notizen / Quellen

- **Datenhoheit:** einziger Schreiber aller Sitzungen (Token, Name, Herkunft, Art, Gnadenfrist, letztes Lebenszeichen, Halterzahl); alle anderen lesen über die Schnittstelle. Nur im Speicher — ein Neustart beendet alle Sitzungen; Tokens kommen nie auf Platte (der Ordner ist für Dritte lesbar).
- **Lebensdauer** ([ADR-0012](../ADRs/0012-identitaet-und-sitzung.md)): lebt, solange Halter da sind oder das letzte Lebenszeichen höchstens die Gnadenfrist zurückliegt (Standard 60 s, Testknopf `LOCALCHATA_GNADENFRIST_MS`). Grenzen: 64 lebende Sitzungen je Herkunft, 512 gesamt.
- **Nebenläufigkeit:** ein `Mutex` um den ganzen Zustand; jede Operation ist kurz und ohne Ein-/Ausgabe, darum kein `spawn_blocking` ([X00008](Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)). Prüfen und Belegen eines Namens geschehen unter derselben Sperre — zwei gleichzeitige `beitreten("claude")` ergeben genau einen Erfolg.
- **Erster Test (Unit, Uhr-Attrappe, feste Zufallsfolge):** `beitreten(claude, 10.0.0.5, agent)` → Kennung `claude@10.0.0.5`, Token aus der Folge; `beitreten(claude, 127.0.0.1, mensch)` → `NameVergeben{belegt_von: claude@10.0.0.5, vorschlag: claude-2}`; Uhr +59 s → weiterhin vergeben; `wiederkehren(token, 10.0.0.5)` bei +59 s → Sitzung, Lebenszeichen erneuert; Uhr +61 s ab dort → `beitreten(claude, 127.0.0.1)` ok; `wiederkehren(alter_token, 10.0.0.5)` → keine Sitzung; `wiederkehren(token, 10.0.0.6)` → keine Sitzung (andere Herkunft). Ersetzt: Uhr, Zufall.
- **Erster Test (Integration, echtes Programm):** zwei Absender `127.0.0.1` und `127.0.0.2`; `POST /api/beitreten {"name":"claude"}` von `.2` → 201 `claude@127.0.0.2`; derselbe Name von `.1` → 409 mit `vorschlag: "claude-2"`; `GET /api/ich` mit Token von `.2` → 200, mit Token von `.1` → 401.
- Betrieb: kein eigener Start, kein Port; Gesundheit kann später `sitzungen` nennen (Früh-Indikator für Namensbesetzung). Fehler sichtbar als 409/422/429 mit deutschem `fehler`-Text, nie still.
- **Stand 2026-10-27 (Karte #3):** Entwurf nach ADR-0012/0013 angeglichen; die frühere Notiz „Tab kehrt mit seinem Token zurück“ (je Tab, ADR-0008) gilt nicht mehr — eine Sitzung je Browser (`localStorage`).
