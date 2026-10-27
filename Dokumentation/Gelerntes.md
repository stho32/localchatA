# Gelerntes

Was ein Lauf an Verständnis gebracht hat. Eine Zeile je Lauf; „nichts Neues" ist ein gültiger Eintrag.

| Datum | Arbeitspaket | Erkenntnis | Folge |
|---|---|---|---|
| 2026-10-27 | I0001 Instanz starten | Was nur außerhalb des Prozesses beobachtbar ist (Browser öffnen, Signale, Standard-Port), wird über das echte Programm prüfbar, sobald es einen Testknopf (`LOCALCHATA_BROWSER`) oder selbst belegte Ports gibt; `File::try_lock` sperrt je geöffneter Datei, darum ist die Ordner-Sperre schon im selben Testprozess prüfbar, ihre Lebensdauer aber nur über das Programm (US-6) | Testknopf-Muster für weitere Außenwirkungen; Prüf- und Commit-Befehle nie in einer Kette (ein Commit lief trotz rotem clippy, Fix-Commit nachgezogen) |
