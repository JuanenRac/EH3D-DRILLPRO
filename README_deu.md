<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-DRILLPRO banner" width="100%">
</p>

# 🔩 EH3D-DRILLPRO

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Eine plattformübergreifende Neuimplementierung des Bohr-Ablaufs der klassischen DOS-Software für Bungard-CCD-Maschinen in Rust, mit einer an das Original angelehnten Textmodus-Oberfläche in acht Sprachen

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute tatsächlich läuft:** ein Rust-Programm mit einer 80x25-Textoberfläche, das Excellon-Bohrdateien `.NCD`, die Werkzeugtabelle und die Maschinenkonfiguration liest, die Bohrungen ordnet, anzeigt und den ganzen Auftrag entweder **gegen einen Simulator** (Standard) oder, wenn die Maschinendatei es vorgibt, **an einer echten Maschine** über die serielle Schnittstelle des Rechners, einen USB-Seriell-Adapter oder den EH3D-NODE-BUNGARD ausführt. 123 Tests laufen auf einem Rechner. An einer echten Bungard CCD MTC wurden die Referenzfahrt der drei Achsen (Z, X und Y bis zu ihren Endschaltern, mit dem Signal „alle drei in Referenz“ der Steuereinheit live bestätigt) und die Bewegungen jeder Achse erprobt; **ein vollständiger Bohrauftrag lief noch nicht an der Maschine**, und die Schritte pro Millimeter sind nur gegen die Konfiguration des Originals geprüft.

---

> **Stand von Sicherheit und IT-Sicherheit:** dieses Programm ist kein Sicherheitssystem. Bei einer echten Verbindung (`connection` in der Maschinendatei) **bewegt Start die Maschine**: Werkzeug herausnehmen, eine Hand am Netzschalter der Maschine halten und `Esc` bereithalten, das den Auftrag sofort stoppt und die Schrittmotortreiber abschaltet. Die Endschalter überwacht die Steuereinheit der Maschine selbst (sie stoppt eine Achse am Schalter von allein); diese Software hat keinen eigenen Not-Aus, und falsche Schrittbytes oder Geschwindigkeiten können das Werkzeug in das Werkstück oder den Tisch fahren oder die Motoren Schritte verlieren lassen. Die Protokollschicht weigert sich, etwas zu bewegen, bevor sie scharf geschaltet wurde. Das Projekt ist unabhängig: Es wird nicht von Bungard Elektronik hergestellt, empfohlen oder unterstützt, und Produktnamen erscheinen nur, um zu sagen, mit welchen Dateiformaten und Maschinen es arbeiten soll.

## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.1` |
| Reifegrad | Gerüst |
| Tests | 123 |
| Technologie-Stack | Rust |
| Auf echter Hardware | Ja, an einer Bungard CCD MTC: die Referenzfahrt und die Bewegungen jeder Achse; ein ganzer Auftrag lief noch nicht. Ohne Maschinendatei läuft es im Simulator |

## 🎯 Überblick

* Die **Dateileser**: Excellon-Bohrdateien `.NCD` (mit oder ohne `M48`-Kopf, modale Koordinaten, imperiale Einheiten von 1 mil), die Werkzeugtabelle `DRILLPRO.DTD` und die Maschinenkonfiguration `DRILLPRO.DCF`.
* Der **Auftragsplaner**: ordnet die Bohrungen (wie in der Datei, zuerst X, zuerst Y oder kürzester Weg) und hält sie nach Werkzeug gruppiert.
* Die **Oberfläche**: ein 80x25-Textbildschirm mit CGA-Farben, Menüs, einem Dateidialog und einer Vektoransicht der Bohrungen in einem Fenster. Sie spricht Englisch, Spanisch, Italienisch, Deutsch, Französisch, Portugiesisch, Chinesisch und Japanisch: Die Sprache wählen Sie im Menü oder mit `--lang` beim Start.
* Das **Maschinenprotokoll** (`protocol/`, Crate `eh3d-drillpro-protocol`): eine Bibliothek ohne Oberfläche: Byte-Transport (serielle Schnittstelle oder Simulator), Bewegungsplanung in Millimetern mit Beschleunigungsrampen und eine Sicherheitssperre, die Bewegung verweigert, bis sie scharf geschaltet ist.
* Der **Simulator**: Der ganze Auftrag läuft ohne Hardware bis zum Ende; die Tests verwenden ihn.
* Die **Maschinen**: die mit einer Steuereinheit an der seriellen Schnittstelle, die die DOS-Software 4.6 nutzte: Bungard CCD MTC und CCD/2 MTC (manueller Werkzeugwechsel; die CCD/2 hat den kleineren Tisch von 325 x 270 mm). Tischgröße und die übrigen Maschinenkonstanten stammen aus der Konfigurationsdatei der Maschine. Erprobt wurde nur eine CCD MTC, und die Ausgänge für Bohrspindel und Staubsaugerschaltung sind noch nicht erprobt. Die beiden ATC-Modelle (CCD ATC und CCD/2 ATC, automatischer Werkzeugwechsel) bekommen eigene Anwendungen, gebaut mit dem, was die Rückwärtsanalyse hergibt. Die XL-Modelle und die späteren Maschinen haben eine eigene moderne Steuereinheit und Software und sind nicht abgedeckt.
* Die **Maschinendatei** (`machine.cfg`, von DRILLPRO und ROUTEPRO gemeinsam genutzt): wie der Auftrag zur Maschine gelangt (`demo` Simulator, `rs232` die serielle Schnittstelle des Rechners selbst, `usb` ein USB-Seriell-Adapter oder `node` der EH3D-NODE-BUNGARD über das Netzwerk) und, ein Wert pro Zeile, alles, was an einer echten Maschine abgestimmt wurde: Suchgeschwindigkeiten und Rampen, der Kalibriersprung, die Grenzen der Motoren.
* Die **Referenzfahrt**: Z fährt bis zu seinem Endschalter, dann X und Y, danach ein sanfter Kalibriersprung. Die Steuereinheit meldet „alle drei Achsen in Referenz“ auf der RI-Leitung, die das Programm überwacht; steht die Maschine geparkt, überspringt es die Suchläufe, fährt X und Y 10 mm von ihren Schaltern weg und sucht sie erneut, sodass es ihr Ankommen live sieht.
* **Ruhige Bewegung**: Bewegungen gehen als ein durchgehender Strom hinaus, dessen Bytepositionen die Zeit tragen (ein USB-Adapter liefert die Bytes sonst in Schüben), Z fährt Byte für Byte, und `Esc` bricht einen Auftrag in Sekundenbruchteilen ab und schaltet die Schrittmotortreiber ab.
* Die **Spindel**: standardmäßig die der Steuereinheit selbst (ihre Latch-Bits, bis 60000 U/min); mit `spindle = node` in der Maschinendatei fragt der Auftrag stattdessen beim EH3D-NODE-BUNGARD an (ein Frequenzumrichter an den Ausgängen des Knotens, jede Drehzahl bis zum Maximum des Knotens, bei jedem Stopp angehalten). Mit einem Knoten lesen die Programme außerdem die Lampen der Maschine, sodass jede Achse, die in Referenz kommt, live gesehen wird, ohne die Home-Leitung mitten im Datenstrom zu lesen.
* Die **Serververbindung** (`connection = server` in der Maschinendatei, mit `server_address`, `server_token` und `server_machine`): Statt die Maschine selbst zu steuern, schickt das Programm den Auftrag an den [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER), der ihn mit den Konstanten der Maschine plant, über den Knoten der Maschine ausführt und dem Programm meldet, wie es läuft; das Programm zeigt den Fortschritt und beantwortet die Dialoge wie gewohnt, `Esc` stoppt den Auftrag auf dem Server, und das Schließen des Programms stoppt ihn nicht.

## 🧩 Rolle in EH3D

EH3D-DRILLPRO ist ein eigenständiges Programm der EH3D-Familie für Maschinen, deren Steuereinheit verlangt, dass der PC jeden Motorschritt erzeugt. EH3D-ROUTEPRO, sein Geschwister für das Fräsen, verwendet sein Protokoll, seinen Bildschirm und seine Sprachunterstützung wieder. Es hängt noch von keinem anderen EH3D-Repository ab; der Excellon-Leser von EH3D-CORE-LIBS könnte später seinen eigenen ersetzen.

## 📂 Repository-Struktur

```text
EH3D-DRILLPRO/
├── src/          library: file formats, job planning, screen, application, languages
├── src/bin/      the drillpro executable (feature gui)
├── protocol/     eh3d-drillpro-protocol: serial transport, motion, safety gate
├── tests/        formats, job planning, screens in all languages
├── testdata/     small sample files written for the tests
├── docs/         PROTOCOL.md und CONNECTION.md: das Protokoll, die Maschinendatei und jeder Abstimmwert; machine.cfg.example: die Datei mit allen ihren Werten
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ BAUEN UND AUSFÜHREN

Du brauchst eine stabile Rust-Toolchain.

```bash
cargo build --release --features gui
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -I tools/ci_validate.py
```

Starten Sie es mit `run.bat` (Windows) oder `./run.sh` (Linux und macOS); beide bauen das Programm bei Bedarf zuerst: `run.bat [DATEI.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--dump]`. `--data` ist der Ordner mit `DRILLPRO.DTD` und `DRILLPRO.DCF` (Standard: `testdata/`), `--dump` gibt den Bildschirm als einfachen Text aus. Tasten: `Alt` plus der hervorgehobene Buchstabe öffnet ein Menü, Pfeile bewegen, `Enter` bestätigt, `Esc` geht zurück; die Menüleiste ist auch anklickbar. Chinesisch und Japanisch brauchen eine CJK-Schrift unter den Systemschriften. `--connection FILE` verweist auf eine andere Maschinendatei als die Standarddatei (`%APPDATA%\EH3D\machine.cfg` unter Windows, sonst `~/.config/eh3d/machine.cfg`); ohne Maschinendatei läuft das Programm im Simulator.

**Fertige Pakete:** Jede Version enthält ein Paket für Windows (zip), Linux (tar.gz) und macOS (Apple Silicon und Intel, tar.gz) mit dem Programm, der Lizenz und dieser README, jeweils mit der SHA-256-Prüfsumme daneben. Sie werden vom Workflow `Release binaries` dieses Repositorys gebaut. Laden Sie eines von der Releases-Seite herunter; unter Linux braucht das Fenster X11 oder Wayland.

## 🔗 Verwandte Projekte

* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): das Geschwister für das Fräsen; es verwendet Protokoll, Bildschirm und Sprachen dieses Projekts
* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): die gemeinsamen Excellon- und G-Code-Leser der Suite
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): der Projektcontainer und die Maschinen-Telemetrietypen der Suite
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD): ein ESP32-S3-Knoten (Ethernet oder WLAN), der im Gehäuse der Maschine sitzt, die Schritte über seine eigene serielle Schnittstelle mit dem Timing ausgibt, das ein USB-Adapter nicht halten kann, die Lampen der Maschine liest und den Spindelantrieb steuert; er hat ein eigenes Web-Panel
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): das Zentrum der EH3D-Familie: Es nimmt die Aufträge dieses Programms (`connection = server`) und der anderen Clients an und führt sie über den Knoten auf der Maschine aus

## 📚 Dokumentation und Community

* [Protokollnotizen](docs/PROTOCOL.md): was über die Ansteuerung der Steuereinheit bekannt ist und was noch eine Hypothese ist.
* [Verbindung und Abstimmung](docs/CONNECTION.md): die Maschinendatei, die drei Verbindungen und jeder Wert
* [Änderungsprotokoll](CHANGELOG.md)
* [Mitwirken](CONTRIBUTING.md)
* [Sicherheitsrichtlinie](SECURITY.md)
* [Support](SUPPORT.md)

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.