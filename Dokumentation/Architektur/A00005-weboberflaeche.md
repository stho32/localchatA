---
id: A00005
name: Weboberfläche
code: web
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - web/
verwandt: [A00004]
domaene: []
blackbox: rahmengetrieben (Browser); Gestaltung gehört dem Designsystem, Verfeinerung mit den Dialogs
---

# Weboberfläche

## Verantwortung

Zeigt dem Menschen im Browser Verlauf, Eingabe und Teilnehmer nach dem Designsystem.

## Schnittstellen

- Eingehend: Browser lädt `/` und `/web/*`.
- Ausgehend: `fetch` auf `/api/*` mit Header `X-LocalchatA-Sitzung`, `EventSource` auf `/api/ereignisse` (Wächter für stille Abrisse wie kanbanD `web/verbindung.js`).

## Abhaengigkeiten

[HTTP-Schnittstelle](A00004-http-schnittstelle.md); Designsystem `Dokumentation/Wireframes/designsystem/` (Tokens, Bausteine).

## Entscheidungen

- [ADR-0002: Oberfläche ohne Build-Schritt in das Programm eingebettet](../ADRs/0002-oberflaeche-ohne-build-schritt-eingebettet.md) — Akzeptiert, 2026-10-27

## Offene Fragen

- Markdown wird im Browser gerendert. Welcher Renderer? Empfehlung: eigener kleiner Renderer wie kanbanD `web/markdown.js` mit Escape-zuerst (kein `innerHTML` aus Rohtext) — Prüfpunkt für den `security-reviewer` (Agenten schreiben beliebigen Text).

## Notizen / Quellen

- Vanilla-JavaScript-Dateien, je eine Verantwortung, reine Funktionen auch in Node testbar (Unit-Tests der Oberfläche unter `e2e/einheit/` wie kanbanD).
- Schriften IBM Plex lokal eingebettet (keine Netzabhängigkeit im LAN).
