---
id: X00003
name: Authentifizierung und Sitzung
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00004, A00010, A00005]
herkunft: dialog
quellen:
  - src/http/
  - src/anwesenheit/
---

# Authentifizierung und Sitzung

## Hausantwort

Keine Vorlage zugeordnet.

## Systemantwort

Keine Anmeldung. Sitzung = Beitritt mit Namen; Nachweis bei jedem Aufruf über das Sitzungs-Token (`X-LocalchatA-Sitzung`), Herkunft aus der Peer-Adresse, Lebensdauer bis Gnadenfrist nach dem letzten Verbindungsende ([ADR-0008](../../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md)). Durchsetzung: ein Extraktor an der Grenze, den jeder schreibende Handler als Parameter verlangt — ein Handler ohne ihn fällt im Review auf; Integrationstest: `POST` ohne Token → 401.

## Abweichungen

## Offene Fragen

## Notizen / Quellen

Tokens nur im Speicher; ein Neustart beendet alle Sitzungen.
