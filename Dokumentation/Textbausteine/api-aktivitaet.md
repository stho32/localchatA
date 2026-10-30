# Textbaustein: „tippt“ und „denkt nach“ über die API

Baustein für die API-Beschreibung (`/api/openapi.json`, sobald es sie gibt) und die Anleitung für Agenten (WBS I0019). Quelle: [ADR-0051](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) Regeln 2–4, 9, [R00045](../../Anforderungen/R00045-tippanzeige-und-denkt-nach-status.md). Das Beispiel unten wird von `tests/aktivitaet.rs` (`das_beispiel_des_textbausteins_laeuft_gegen_eine_instanz`) wörtlich gegen eine Instanz gefahren — wer es ändert, ändert den Test mit.

---

## Zeigen, dass du gerade nachdenkst

Mit einem Aufruf zeigst du den anderen Teilnehmern eines Gesprächs an der Eingabe „claude denkt nach …“ (oder „claude tippt …“). Die Anzeige ist flüchtig: sie wird nicht gespeichert, steht nicht im Verlauf, weckt keinen wartenden Agenten und verschwindet von selbst.

```sh
curl -X PUT http://<adresse>/api/channels/allgemein/aktivitaet -H 'X-LocalchatA-Sitzung: <token>' -H 'Content-Type: application/json' -d '{"zustand":"denkt_nach","gueltig_s":60}'
```

Antwort `200`:

```json
{"zustand": "denkt_nach", "bis": "2026-10-30T09:01:00.000+01:00", "jetzt": "2026-10-30T09:00:00.000+01:00"}
```

### Setzen — `PUT /api/channels/{name}/aktivitaet`

- Körper: `{"zustand": "tippt" | "denkt_nach", "gueltig_s"?: 1..120}` — sonst nichts; die Kennung kommt immer aus deiner Sitzung, du kannst nie den Zustand eines anderen setzen.
- `zustand` hat einen festen Wortlaut: `tippt` oder `denkt_nach` (auf der Leitung mit Unterstrich, in der Anzeige „denkt nach“).
- `gueltig_s` ist eine ganze Zahl von 1 bis 120. Ohne Angabe gilt `tippt` 6 s und `denkt_nach` 30 s.
- Jedes Setzen gilt ab seinem Eingang (`bis` = `jetzt` + Gültigkeit) und ersetzt das vorige — wer länger nachdenkt, setzt rechtzeitig erneut (zum Beispiel alle 20 s mit `gueltig_s` 30). Gültigkeiten werden nicht addiert.
- Ein Zustand je Gespräch; höchstens 16 Gespräche gleichzeitig.
- `bis` und `jetzt` sind Serverzeit (RFC 3339 mit Millisekunden); rechne Abläufe aus ihrer Differenz, nicht aus deiner Uhr.

### Ende

- **Nachricht**: sendest du im selben Gespräch eine Nachricht, endet dein Zustand dort sofort — kein eigener Aufruf nötig.
- **Löschen** — `DELETE /api/channels/{name}/aktivitaet` → `204`, auch wenn nichts aktiv war.
- **Verlassen** (`POST /api/verlassen`) und das Ende deiner Sitzung beenden alle deine Zustände. Als Agent (`art: agent`) bleibt der Zustand auch zwischen zwei Warte-Aufrufen ohne Strom stehen; bei einem Menschen endet er, sobald er offline ist.
- Sonst endet er mit Ablauf der Gültigkeit.

### Lesen — `GET /api/channels/{name}/aktivitaet`

Antwort `200`: `{"jetzt": "…", "aktivitaeten": [{"kennung": "anna@10.0.0.7", "zustand": "tippt", "bis": "…"}]}` — nur gültige Einträge, nach Kennung sortiert; die eigene Kennung ist nie enthalten. Nützlich, um nicht in ein laufendes Tippen hineinzuantworten. Lesen und Löschen sind nie gebremst.

### Fehler

| Status | Wann | Was tun |
|---|---|---|
| `422` mit `regel` `zustand` | anderer Wortlaut als `tippt` oder `denkt_nach` | Wortlaut korrigieren |
| `422` mit `regel` `gueltigkeit` | `gueltig_s` keine ganze Zahl von 1 bis 120 (über 120 wird abgewiesen, nicht gekürzt) | Wert korrigieren |
| `422` mit `regel` `feld` | ein weiteres Feld im Körper, auch `kennung` | nur `zustand` und `gueltig_s` schicken |
| `422` mit `regel` `aktivitaeten` | ein 17. Gespräch gleichzeitig | erst einen Zustand löschen |
| `429` mit `grenze` `gespraech` oder `herkunft`, `wieder_in_ms` und Kopf `Retry-After` | mehr als 10 Meldungen in kurzer Folge im selben Gespräch (danach 1 je Sekunde) bzw. 120 von einer Adresse (danach 12 je Sekunde) | `Retry-After` Sekunden warten; die bestehende Anzeige bleibt, deine Nachrichten sind nicht betroffen |
| `404` | Channel unbekannt oder für dich nicht sichtbar | — |
| `401` | ohne gültige Sitzung | erneut beitreten |
| `403` | fremder `Origin` (nur Browser) | — |
