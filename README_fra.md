<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-DRILLPRO banner" width="100%">
</p>

# 🔩 EH3D-DRILLPRO

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Une réimplémentation multiplateforme en Rust du flux de perçage du logiciel DOS classique des machines Bungard CCD, avec une interface en mode texte qui rappelle l'original, en huit langues

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui tourne aujourd'hui :** un programme Rust avec une interface texte de 80x25 qui lit les fichiers de perçage Excellon `.NCD`, la table d'outils et la configuration de la machine, ordonne les trous, les affiche et exécute tout le travail **contre un simulateur** (par défaut) ou, quand le fichier de la machine l'indique, **sur une machine réelle** par le port série de l'ordinateur, un adaptateur USB-série ou l'EH3D-NODE-BUNGARD. 123 tests s'exécutent sur un ordinateur. Sur une Bungard CCD MTC réelle, la référence des trois axes (Z, X et Y jusqu'à leurs fins de course, avec le signal « les trois en origine » de l'unité de commande confirmé en direct) et les déplacements de chaque axe ont été essayés ; **un travail de perçage complet n'a pas encore tourné sur la machine**, et les pas par millimètre ne sont vérifiés que par rapport à la configuration de l'original.

---

> **État de la sûreté et de la sécurité :** ce programme n'est pas un système de sécurité. Avec une connexion réelle (`connection` dans le fichier de la machine) **Démarrer déplace la machine** : retirez l'outil, gardez une main sur l'interrupteur de la machine et soyez prêt à appuyer sur `Échap`, qui arrête le travail aussitôt et coupe les drivers des moteurs. Les fins de course sont surveillées par l'unité de commande de la machine elle-même (elle arrête un axe à son contact toute seule) ; ce logiciel n'a pas d'arrêt d'urgence propre, et des octets de pas ou des vitesses erronés peuvent entraîner l'outil contre la pièce ou la table ou faire perdre des pas aux moteurs. La couche de protocole refuse de déplacer quoi que ce soit tant qu'elle n'a pas été armée. Le projet est indépendant : il n'est ni fabriqué, ni approuvé, ni soutenu par Bungard Elektronik, et les noms de produits n'apparaissent que pour dire avec quels formats de fichier et quelles machines il doit fonctionner.

## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.1` |
| Maturité | Échafaudage |
| Tests | 123 |
| Pile technique | Rust |
| Sur du matériel réel | Oui, sur une Bungard CCD MTC : la référence et les déplacements de chaque axe ; un travail complet n'a pas encore tourné. Sans fichier de machine, il s'exécute sur le simulateur |

## 🎯 Vue d'ensemble

* Les **lecteurs de fichiers** : fichiers de perçage Excellon `.NCD` (avec ou sans l'en-tête `M48`, coordonnées modales, unités impériales de 1 mil), la table d'outils `DRILLPRO.DTD` et la configuration de la machine `DRILLPRO.DCF`.
* Le **planificateur de travail** : ordonne les trous (comme dans le fichier, X d'abord, Y d'abord ou chemin le plus court) et les garde regroupés par outil.
* L'**interface** : un écran en mode texte de 80x25 avec les couleurs CGA, des menus, une boîte de dialogue de fichiers et une vue vectorielle des trous, affichés dans une fenêtre. Elle parle anglais, espagnol, italien, allemand, français, portugais, chinois et japonais : choisissez la langue dans le menu ou lancez avec `--lang`.
* Le **protocole de la machine** (`protocol/`, crate `eh3d-drillpro-protocol`) : une bibliothèque sans interface : transport d'octets (port série ou simulateur), planification du mouvement en millimètres avec rampes d'accélération et une barrière de sécurité qui refuse tout mouvement tant qu'elle n'est pas armée.
* Le **simulateur** : le travail complet s'exécute jusqu'au bout sans matériel ; les tests l'utilisent.
* Les **machines** : celles dont l'unité de commande passe par le port série et utilisait le logiciel DOS 4.6 : Bungard CCD MTC et CCD/2 MTC (changement d'outil manuel ; la CCD/2 a la table plus petite de 325 x 270 mm). La taille de la table et les autres constantes viennent du fichier de configuration de la machine. Seule une CCD MTC a été essayée, et les sorties de la broche et de la commande de l'aspirateur n'ont pas encore été essayées. Les deux modèles ATC (CCD ATC et CCD/2 ATC, changement automatique) auront des applications séparées, construites avec ce que permet la rétro-ingénierie. Les modèles XL et les machines postérieures ont leur propre unité de commande et logiciel modernes et ne sont pas couverts.
* Le **fichier de la machine** (`machine.cfg`, partagé par DRILLPRO et ROUTEPRO) : comment le travail atteint la machine (`demo` simulateur, `rs232` le port série de l'ordinateur lui-même, `usb` un adaptateur USB-série ou `node` l'EH3D-NODE-BUNGARD par le réseau) et, une valeur par ligne, tout ce qui a été réglé sur une machine réelle : vitesses et rampes de recherche, le saut d'étalonnage, les limites des moteurs.
* La **référence** : Z monte jusqu'à sa fin de course, puis X et Y, puis un saut d'étalonnage doux. L'unité de commande signale « les trois axes en origine » sur la ligne RI, que le programme surveille ; quand la machine est garée, il saute les recherches, éloigne X et Y de 10 mm de leurs contacts et les cherche de nouveau, ainsi il les voit arriver en direct.
* **Mouvement régulier** : les déplacements partent en un seul flux continu dont les positions d'octet portent le temps (sinon un adaptateur USB livre les octets par rafales), Z se déplace octet par octet, et `Échap` annule un travail en une fraction de seconde et coupe les drivers des moteurs.
* La **broche** : par défaut celle de l'unité de commande elle-même (ses bits de verrou, jusqu'à 60000 tr/min) ; avec `spindle = node` dans le fichier de la machine, le travail la demande à l'EH3D-NODE-BUNGARD (un variateur de fréquence sur les sorties du nœud, toute vitesse jusqu'au maximum du nœud, arrêtée à tout arrêt). Avec un nœud, les programmes lisent aussi les voyants de la machine, si bien que chaque axe qui arrive en origine est vu en direct sans lire la ligne d'origine au milieu du flux.
* La **connexion au serveur** (`connection = server` dans le fichier de la machine, avec `server_address`, `server_token` et `server_machine`) : au lieu de piloter lui-même la machine, le programme envoie le travail à l'[EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER), qui le planifie avec les constantes de la machine, l'exécute par le nœud de la machine et dit au programme comment il avance ; le programme montre la progression et répond aux dialogues comme d'habitude, `Échap` arrête le travail sur le serveur, et fermer le programme ne l'arrête pas.

## 🧩 Rôle dans EH3D

EH3D-DRILLPRO est un programme autonome de la famille EH3D pour les machines dont l'unité de commande exige que le PC génère chaque pas de moteur. EH3D-ROUTEPRO, son frère pour le fraisage, réutilise son protocole, son écran et sa prise en charge des langues. Il ne dépend encore d'aucun autre dépôt EH3D ; le lecteur Excellon d'EH3D-CORE-LIBS pourrait remplacer le sien plus tard.

## 📂 Structure du dépôt

```text
EH3D-DRILLPRO/
├── src/          library: file formats, job planning, screen, application, languages
├── src/bin/      the drillpro executable (feature gui)
├── protocol/     eh3d-drillpro-protocol: serial transport, motion, safety gate
├── tests/        formats, job planning, screens in all languages
├── testdata/     small sample files written for the tests
├── docs/         PROTOCOL.md et CONNECTION.md : le protocole, le fichier de la machine et chaque valeur de réglage ; machine.cfg.example : le fichier avec toutes ses valeurs
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ COMPILER ET EXÉCUTER

Il faut une chaîne d'outils Rust stable.

```bash
cargo build --release --features gui
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -I tools/ci_validate.py
```

Lancez-le avec `run.bat` (Windows) ou `./run.sh` (Linux et macOS) ; les deux compilent d'abord le programme si nécessaire : `run.bat [FICHIER.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--dump]`. `--data` est le dossier contenant `DRILLPRO.DTD` et `DRILLPRO.DCF` (par défaut : `testdata/`) et `--dump` affiche l'écran en texte brut. Touches : `Alt` plus la lettre en surbrillance ouvre un menu, les flèches déplacent, `Entrée` valide, `Échap` revient ; la barre de menus est aussi cliquable. Le chinois et le japonais nécessitent une police CJK parmi les polices du système. `--connection FILE` désigne un autre fichier de machine que celui par défaut (`%APPDATA%\EH3D\machine.cfg` sous Windows, `~/.config/eh3d/machine.cfg` ailleurs) ; sans fichier de machine, le programme s'exécute sur le simulateur.

**Paquets prêts à l'emploi :** chaque version publiée contient un paquet pour Windows (zip), Linux (tar.gz) et macOS (Apple silicon et Intel, tar.gz) avec le programme, la licence et ce README, chacun avec son SHA-256 à côté. Ils sont construits par le workflow `Release binaries` de ce dépôt. Téléchargez-en un depuis la page Releases ; sous Linux, la fenêtre a besoin de X11 ou de Wayland.

## 🔗 Projets liés

* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): le frère pour le fraisage ; il utilise le protocole, l'écran et les langues de ce projet
* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): les lecteurs Excellon et G-code partagés de la suite
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): le conteneur de projet et les types de télémétrie de machine de la suite
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD) : un nœud ESP32-S3 (Ethernet ou Wi-Fi) qui se place dans le coffret de la machine, envoie les pas par son propre port série avec le cadencement qu'un adaptateur USB ne sait pas tenir, lit les voyants de la machine et pilote le variateur de la broche ; il a son propre panneau web
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER) : le centre de la famille EH3D : il reçoit les travaux de ce programme (`connection = server`) et des autres clients, et les exécute sur la machine par son nœud

## 📚 Documentation et communauté

* [Notes sur le protocole](docs/PROTOCOL.md): ce que l'on sait de la façon dont l'unité de commande est pilotée et ce qui reste une hypothèse.
* [Connexion et réglage](docs/CONNECTION.md) : le fichier de la machine, les trois connexions et chaque valeur
* [Journal des modifications](CHANGELOG.md)
* [Contribuer](CONTRIBUTING.md)
* [Politique de sécurité](SECURITY.md)
* [Assistance](SUPPORT.md)

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.