---
id: X00003
name: Authentifizierung und Sitzung
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00004, A00010, A00005, A00003]
herkunft: dialog
quellen:
  - src/http/
  - src/anwesenheit/
---

# Authentifizierung und Sitzung

## Hausantwort

Keine Vorlage zugeordnet.

## Systemantwort

Keine Anmeldung. Sitzung = Beitritt mit Namen (`POST /api/beitreten`); Nachweis bei jedem Aufruf über das Sitzungs-Token im Header `X-LocalchatA-Sitzung` (128 Bit Betriebssystem-Zufall, nur im Speicher der [Anwesenheit](../A00010-anwesenheit.md)), gültig nur von derselben Herkunft. Herkunft = kanonische Peer-Adresse, einmal in der Annahmeschleife gesetzt ([A00003](../A00003-start.md)). Lebensdauer: solange Halter da sind oder das letzte Lebenszeichen (Anfrage mit Token, Ende des letzten Halters) höchstens die Gnadenfrist (60 s) zurückliegt. Browser: eine Sitzung je Browser und Ursprung (`localStorage`). Davor liegt der Grenzschutz (Host, Origin, Sicherheitsheader) aus [ADR-0014](../../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md). Entscheidung: [ADR-0012](../../ADRs/0012-identitaet-und-sitzung.md) (löst ADR-0008 ab).

Durchsetzung: ein Extraktor `Teilnehmer` an der Grenze, den jeder Handler mit Wirkung als Parameter verlangt — ein Handler ohne ihn fällt im Review auf; Integrationstests über das echte Programm: Anfrage ohne Token → 401, mit Token einer anderen Herkunft (`127.0.0.2`) → 401; Mutationsprobe je Wächter (Token, Herkunftsvergleich, Gnadenfrist).

## Abweichungen

## Offene Fragen

## Notizen / Quellen

Tokens nur im Speicher; ein Neustart beendet alle Sitzungen, die Oberfläche tritt mit dem gemerkten Namen still wieder bei. Ausnahmen von der Sitzungspflicht: `beitreten` selbst, `/api/gesundheit`, die Oberfläche (`/`, `/web/*`).
