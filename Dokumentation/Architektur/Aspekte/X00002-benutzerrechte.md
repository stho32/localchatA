---
id: X00002
name: Benutzerrechte
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00004, A00011, A00010]
herkunft: dialog
quellen:
  - src/http/
  - src/live/
---

# Benutzerrechte

## Hausantwort

Keine Vorlage zugeordnet.

## Systemantwort

Es gibt keine Rechte (Vision: „keine Benutzerverwaltung und keine Rechte“). Einzige Sichtbarkeitsregel: ein Direktgespräch sehen nur seine beiden Beteiligten — in Abruf, Suche, SSE und Warten. Geprüft wird an der Systemgrenze ([A00004](../A00004-http-schnittstelle.md)) bzw. im Filter des [Live-Kanals](../A00011-live-kanal.md), nie in Index oder Ablage. Durchsetzung: Integrationstest je Weg (Abruf, Suche, SSE, Warten), dass ein Dritter eine Direktnachricht nicht erhält.

Ohne Rechte bleibt die Abwehr fremder **Ursprünge**: Wirkungen nur aus dem eigenen Ursprung oder ohne Browser (Origin-/`Sec-Fetch-Site`-Prüfung), nur unter erlaubten Host-Namen (gegen DNS-Rebinding), Sicherheitsheader auf jeder Antwort ([ADR-0014](../../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md)). Gegen Namensbesetzung: Namensregel ohne Verwechslungen ([ADR-0013](../../ADRs/0013-namensregel-als-erlaubnisliste.md)), höchstens 64 Sitzungen je Herkunft ([ADR-0012](../../ADRs/0012-identitaet-und-sitzung.md)). Durchsetzung: Angriffsmatrix aus ADR-0014 über das echte Programm, Mutationsprobe je Wächter.

## Abweichungen

## Offene Fragen

- Designsystem-Frage 10 (wer sieht welche Channels) — fachlich, `planer`.

## Notizen / Quellen

Sichtbarkeit ist kein Schutz gegen Absicht: Dateien sind lesbar, Tokens sind nicht an Personen gebunden, nur an die Herkunft ([ADR-0012](../../ADRs/0012-identitaet-und-sitzung.md)); wer im LAN direkt per IP spricht, wird durch den Grenzschutz nicht abgehalten — gewollt, es gibt keine Anmeldung.
