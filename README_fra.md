<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-DRILLPRO banner" width="100%">
</p>

# 🔩 EH3D-DRILLPRO

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Une réimplémentation multiplateforme en Rust du flux de perçage du logiciel DOS classique des machines Bungard CCD, avec une interface en mode texte qui rappelle l'original, en 37 langues

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.8-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui tourne aujourd'hui :** un programme Rust avec une interface texte de 80x25 qui lit les fichiers de perçage Excellon `.NCD`, la table d'outils et la configuration de la machine, ordonne les trous, les affiche et exécute tout le travail **contre un simulateur** (par défaut) ou, quand le fichier de la machine l'indique, **sur une machine réelle** par le port série de l'ordinateur, un adaptateur USB-série ou l'EH3D-NODE-BUNGARD. 236 tests s'exécutent sur un ordinateur. Sur une Bungard CCD MTC réelle, la référence des trois axes (Z, X et Y jusqu'à leurs fins de course, avec le signal « les trois en origine » de l'unité de commande confirmé en direct) et les déplacements de chaque axe ont été essayés ; **un travail de perçage complet n'a pas encore tourné sur la machine**, et les pas par millimètre ne sont vérifiés que par rapport à la configuration de l'original. Par défaut, ce sont les écrans classiques du programme MS-DOS, vérifiés cellule par cellule avec l'original ; `--modern` lance l'interface à menus. Ils se manipulent au clavier et à la souris, la fenêtre peut être agrandie (l'écran s'adapte de 640x400 à 1080p et plus), les fichiers de configuration et d'outils se modifient et s'enregistrent depuis les dialogues, et le bouton Info indique les machines Bungard CCD et CCD/2 (MTC prises en charge, ATC pas encore). Une version console (`drillpro-console`) affiche les mêmes écrans dans un terminal, au clavier et à la souris et sans système de fenêtres (SSH, console texte, vieux PC). Les paquets de chaque version contiennent la configuration de base, expliquée ligne par ligne, et un dossier `examples` d'un trou à vingt mille. Le changeur d'outils des modèles ATC (changement automatique) fonctionne d'après un enregistrement de l'original et n'a pas tourné sur une machine réelle.

---

> **État de la sûreté et de la sécurité :** ce programme n'est pas un système de sécurité. Avec une connexion réelle (`connection` dans le fichier de la machine) **Démarrer déplace la machine** : retirez l'outil, gardez une main sur l'interrupteur de la machine et soyez prêt à appuyer sur `Échap`, qui arrête le travail aussitôt et coupe les drivers des moteurs. Les fins de course sont surveillées par l'unité de commande de la machine elle-même (elle arrête un axe à son contact toute seule) ; ce logiciel n'a pas d'arrêt d'urgence propre, et des octets de pas ou des vitesses erronés peuvent entraîner l'outil contre la pièce ou la table ou faire perdre des pas aux moteurs. La couche de protocole refuse de déplacer quoi que ce soit tant qu'elle n'a pas été armée. Le projet est indépendant : il n'est ni fabriqué, ni approuvé, ni soutenu par Bungard Elektronik, et les noms de produits n'apparaissent que pour dire avec quels formats de fichier et quelles machines il doit fonctionner.

## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.8` |
| Maturité | Échafaudage |
| Tests | 236 |
| Pile technique | Rust |
| Sur du matériel réel | Oui, sur une Bungard CCD MTC : la référence et les déplacements de chaque axe ; un travail complet n'a pas encore tourné. Sans fichier de machine, il s'exécute sur le simulateur |

**Aller à :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure](#-structure-du-dépôt) · [Liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Logiciel | 🟡 | Un programme Rust à écran texte 80x25 lit le fichier de perçage, la table d'outils et le fichier de la machine, ordonne les trous et exécute tout le travail sur un simulateur, ou sur une vraie machine si le fichier de la machine le dit. |
| Tests | 🟢 | 236 tests réussissent (`cargo test --workspace`) : les formats, la planification du travail, le protocole sur le simulateur et les écrans dans les 37 langues. |
| Lecteurs de fichiers | 🟡 | Le fichier de perçage `.NCD`, la table d'outils `DRILLPRO.DTD` et le fichier de la machine `DRILLPRO.DCF` sont lus ; seulement avec de petits fichiers d'exemple écrits pour les tests, pas avec des fichiers de vraies cartes. |
| Protocole de la machine | 🟠 | Le protocole d'octets de l'unité de commande a été reconstruit par rétro-ingénierie et `docs/PROTOCOL.md` liste encore des parties qui sont des hypothèses ; la couche de protocole refuse de bouger quoi que ce soit avant d'être armée. |
| Interface | 🟡 | L'écran texte, les menus, la boîte de fichiers et la vue vectorielle des trous parlent les 37 langues (testé) ; le chinois et le japonais demandent une police CJK dans le système. |
| Matériel réel | 🟠 | Sur une Bungard CCD MTC, la prise de référence des trois axes et les déplacements de chaque axe ont été essayés ; un travail de perçage complet n'a pas tourné, les pas par millimètre ne sont comparés qu'à la configuration de l'original, et les sorties de broche et d'aspirateur n'ont pas été essayées. |
| Sécurité d'emploi | 🟡 | `Esc` arrête le travail en une fraction de seconde et coupe les pilotes de moteurs, et l'unité de commande de la machine surveille les fins de course ; le logiciel n'a pas d'arrêt d'urgence propre. |
| Connexion au serveur | 🟠 | Avec `connection = server`, le programme envoie le travail à EH3D-SERVER et suit sa progression ; ce chemin n'a été exercé que sur le simulateur. |
| Paquets prêts à l'emploi | 🟡 | La première publication contient des paquets pour Windows, Linux et macOS (Apple silicon et Intel), chacun avec son SHA-256, construits par le workflow de publication. |
| Documentation | 🟢 | README en huit langues, les notes du protocole avec ce qui reste hypothèse et le guide du fichier de la machine avec chaque valeur de réglage. |

## 🎯 Présentation

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
* Les **archives de travail, la fenêtre Offset et la simulation** : un travail peut être empaqueté dans une archive ZIP (la fenêtre Select File y entre, et `drillpro JOB.ZIP` charge son premier fichier de perçage) ; la flèche à côté du nom du fichier liste les motifs `*.NCD`, `*.ZIP` et `*.*` ; les boutons Load des fenêtres d'outils et de configuration choisissent le fichier à charger ; sur une vraie machine la fenêtre Offset fait la prise d'origine, suit chaque mouvement de la fenêtre et refait la prise d'origine ; et le travail simulé dure le temps que prendrait la machine.
* Les **sons** : `sound_finish` et `sound_tool_change` dans le fichier de la machine valent chacun off (par défaut), `default` (une tonalité du programme) ou le chemin d'un fichier WAV personnel ; le premier joue quand un travail se termine seul, le second quand la machine demande un outil. `sound_check_ok` et `sound_check_bad` font de même pour le contrôle visuel de la carte avec la caméra de la tête (éteints par défaut ; une voix dit que le contrôle est correct ou n'est pas correct).
* **Plusieurs machines et stations** : avec `connection = server`, `drillpro --machines` liste toutes les machines du serveur, groupées par station, avec ce que fait chacune, son travail et les travaux prêts (le serveur répond pour toutes à la fois), et `drillpro --machine ID` envoie les travaux de ce programme à cette machine au lieu de celle du fichier de machine ; les machines travaillent en même temps, un travail sur chacune.
* **La connexion choisie dans la fenêtre Config :** une liste déroulante avec les cinq façons dont le travail atteint la machine (le simulateur, le port série, un adaptateur USB, le nœud EH3D et le serveur EH3D) et, dessous, ce dont chacune a besoin : un port, une adresse et un jeton, une machine du serveur ; ce qui est choisi est vérifié, appliqué avec OK et écrit dans le fichier de machine sans toucher à ses autres lignes.
* **Un curseur qui clignote :** le curseur d'un champ numérique ou de texte en cours de modification clignote (une demi-seconde allumé, une demi-seconde éteint), comme sur les écrans qu'il suit.
* **Panneaux avec ponts et travaux qui reprennent :** un panneau peut porter des ponts à l'extérieur (combien, quelle largeur, où) et un travail peut laisser de côté les trous déjà faits, pour reprendre après un arrêt ; la fenêtre s'ouvre agrandie.
* **Une table qui n'est pas plane, des avis parlés et le nœud de la tête de perçage :** les options d'un travail peuvent porter la carte des hauteurs de la table, et chaque trou est percé à la même profondeur dans la carte que la table soit haute ou basse (sans carte rien ne change) ; le son de la fin du travail et de la demande d'outil peut être une voix (`sound_finish = voice:es:f`) du dossier `voices` à côté du programme ; et le fichier de machine peut nommer le nœud de la tête de perçage (`head_node`, `head_address`, `head_token`, `head_camera`) pour l'interface moderne.

### 🆚 L'original et cette version

L'original est le programme fermé pour MS-DOS de Bungard qui pilote les machines CCD. Cette version est réécrite à partir d'enregistrements de ce que fait l'original (ses écrans, ses fichiers et ce qu'il envoie à l'unité de commande) ; elle ne contient aucun code de l'original. Ce qui est identique et ce qui diffère :

| Aspect | Original (MS-DOS) | Cette version |
|---|---|---|
| Ordinateur | Seulement un PC MS-DOS (ou un émulateur) | Windows, Linux et macOS, 32 et 64 bits ; une version console pour SSH et les vieux PC |
| Écrans | Mode texte 80x25 | Par défaut les mêmes écrans classiques, vérifiés cellule par cellule avec l'original, avec la souris et une fenêtre qui s'adapte de 640x400 à 1080p et plus ; `--modern` propose une interface à menus et EH3D-BUNGARD-GUI une interface graphique |
| Langues | Celle du programme | Huit : anglais, espagnol, italien, allemand, français, portugais, chinois et japonais, à changer depuis le menu ou avec `--lang` |
| Panneaux | Copies de la carte sur la table | Copies de la carte sur la table ; un travail interrompu en cours peut reprendre au trou suivant |
| Liaison avec la machine | Le port série de l'ordinateur vers l'unité de commande | Le simulateur par défaut ; sur une vraie machine le port série, un adaptateur USB série, l'EH3D-NODE-BUNGARD par le réseau ou l'EH3D-SERVER, choisi dans une liste déroulante de la fenêtre Config |
| Ajouts | Aucun de ceux-ci | Un simulateur qui exécute un travail entier, des sons pour la fin du travail et la demande d'outil, plusieurs machines à la fois via le serveur, des archives ZIP de travail, des contours DXF et Gerber, un curseur clignotant et une suite de tests lancée à chaque modification |
| Source et licence | Fermé, propriétaire | Open source (GPL-3.0-or-later), écrit en Rust |
| Maturité | Des années d'usage sur de vraies machines | Bien plus jeune : la course de référence et les mouvements de chaque axe ont été essayés sur une vraie Bungard CCD MTC ; un travail entier n'a pas encore tourné sur une machine, et le changeur d'outils des modèles ATC n'a pas tourné sur une vraie |
* **37 langues :** les écrans de texte, les menus et les dialogues parlent anglais, espagnol, italien, allemand, français, portugais, chinois, japonais, bulgare, croate, tchèque, danois, néerlandais, estonien, finnois, grec, hongrois, irlandais, letton, lituanien, maltais, polonais, roumain, slovaque, slovène, suédois, norvégien, islandais, ukrainien, russe, serbe, albanais, catalan, galicien, basque, portugais du Brésil et français du Canada. Les mots des 29 langues au-delà des huit premières sont traduits par le modèle et attendent la relecture de locuteurs natifs ; un texte qu'une langue n'a pas s'affiche en anglais.

## 🧩 Rôle dans EH3D

EH3D-DRILLPRO est un programme autonome de la famille EH3D pour les machines dont l'unité de commande exige que le PC génère chaque pas de moteur. EH3D-ROUTEPRO, son frère pour le fraisage, réutilise son protocole, son écran et sa prise en charge des langues. Pour les travaux empaquetés dans des archives ZIP il utilise un crate d'EH3D-COMMON (`eh3d-archive`), qui doit se trouver à côté pour compiler ; le lecteur Excellon d'EH3D-CORE-LIBS pourrait remplacer le sien plus tard. Les fichiers de la machine (celui de perçage, le HP-GL, les tables d'outils et la configuration de la machine) viennent du crate `eh3d-bungard-formats` d'EH3D-COMMON, qui doit aussi se trouver à côté.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-DRILLPRO : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>

Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée à partir de `docs/architecture.json` et vérifiée par rapport au code.

## 🖼️ Captures d'écran

<p align="center">
  <img src="images/screenshots/main.png" alt="main screen" width="80%">
</p>

<p align="center"><em>L'écran principal du programme classique, tel que le dessine le programme propre à la machine : l'échelle, le décalage et la coupe, les vecteurs du travail et les boutons.</em></p>

<p align="center">
  <img src="images/screenshots/tools.png" alt="tool data" width="80%">
</p>

<p align="center"><em>Les données d'outils : la course supplémentaire, l'avance et la vitesse de la broche de chaque place du changeur, et combien chaque outil a servi.</em></p>

<p align="center">
  <img src="images/screenshots/config.png" alt="parameters" width="80%">
</p>

<p align="center"><em>Les paramètres de la machine : les coordonnées des fichiers, le tri des trous, les vitesses, les rampes, la correction et la langue.</em></p>

<p align="center">
  <img src="images/screenshots/connection.png" alt="connection" width="80%">
</p>

<p align="center"><em>La connexion avec la machine : la façon choisie dans la liste et ce qu'elle demande (ici le serveur EH3D).</em></p>

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable.

```bash
cargo build --release --features gui
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -I tools/ci_validate.py
```

Lancez-le avec `run.bat` (Windows) ou `./run.sh` (Linux et macOS) ; les deux compilent d'abord le programme si nécessaire : `run.bat [FICHIER.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--dump]`. `--data` est le dossier contenant `DRILLPRO.DTD` et `DRILLPRO.DCF` (par défaut : `testdata/`) et `--dump` affiche l'écran en texte brut. Touches : `Alt` plus la lettre en surbrillance ouvre un menu, les flèches déplacent, `Entrée` valide, `Échap` revient ; la barre de menus est aussi cliquable. Le chinois et le japonais nécessitent une police CJK parmi les polices du système. `--connection FILE` désigne un autre fichier de machine que celui par défaut (`%APPDATA%\EH3D\machine.cfg` sous Windows, `~/.config/eh3d/machine.cfg` ailleurs) ; sans fichier de machine, le programme s'exécute sur le simulateur. `--modern` lance l'interface à menus au lieu des écrans classiques.

## 🧪 Tests

La suite compte 236 tests (`cargo test --workspace`). Ils vérifient les formats de fichier (le fichier de perçage, la table d'outils et le fichier de la machine, et leur réécriture), la planification du travail, le protocole sur le simulateur (octets, mouvement, la prise de référence, les bits de verrouillage, le lien avec le nœud et la barrière de sécurité qui refuse de bouger tant qu'elle n'est pas armée), les écrans classiques cellule par cellule et les écrans dans les huit langues. La CI exécute aussi `cargo fmt --check` et `cargo clippy` avec les avertissements comme erreurs, et `python -I tools/ci_validate.py` contrôle le manifeste, les versions, le journal des modifications et les huit README.

## 🏗️ Compilation et publication

La version se trouve dans `Cargo.toml` et dans `eh3d.project.json`, et chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). Les versions sont publiées sur GitHub.

**Paquets prêts à l'emploi :** chaque version publiée contient un paquet pour Windows (zip), Linux (tar.gz) et macOS (Apple silicon et Intel, tar.gz) avec le programme, la licence et ce README, chacun avec son SHA-256 à côté. Ils sont construits par le workflow `Release binaries` de ce dépôt. Téléchargez-en un depuis la page Releases ; sous Linux, la fenêtre a besoin de X11 ou de Wayland.

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-DRILLPRO" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-DRILLPRO/
├── src/
│   ├── classic/                                 les écrans, boîtes de dialogue et touches classiques du programme MS-DOS
│   ├── job.rs runner.rs                         ordonner les trous et exécuter le travail dans son propre fil
│   ├── app.rs screen.rs textapp.rs              l'écran texte de 80x25 et les menus modernes
│   ├── i18n.rs texts.rs                         les 37 langues et leurs textes
│   ├── gui.rs console.rs bin/                   la fenêtre, la version terminal et les exécutables
│   └── server_link.rs cli.rs                    le lien avec EH3D-SERVER et la ligne de commande
├── protocol/src/
│   ├── safety.rs                                la barrière qui refuse de bouger tant qu'elle n'est pas armée
│   ├── motion.rs positioner.rs                  mouvement en millimètres avec rampes
│   ├── execute.rs job.rs                        transformer un travail en flux d'octets
│   ├── homing.rs latches.rs                     la prise de référence et les bits de verrouillage
│   ├── serial.rs node.rs nullmodem.rs           port série, nœud et simulateur
│   └── connection.rs tuning.rs                  le fichier de la machine et ses valeurs de réglage
├── tests/                                       formats, planification, écrans classiques et écrans dans toutes les langues
├── docs/                                        les notes de protocole, les écrans classiques, le guide de connexion et le fichier de la machine avec toutes ses valeurs
├── testdata/                                    petits fichiers d'exemple écrits pour les tests
├── config/                                      la configuration de base, expliquée ligne par ligne
├── examples/                                    exemples, d'un trou à 20000
├── locales/                                     les mots des 29 langues lues dans des fichiers, un fichier par langue
├── tools/                                       contrôle de CI, du README et outils d'images
├── images/                                      bannière, les deux images et les captures d'écran
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── build.sh build.bat                           scripts de compilation et de test
├── run.sh run.bat                               lancent le programme
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le paquet Rust et ses dépendances figées
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* Un travail de perçage complet n'a pas encore tourné sur une machine réelle : seuls la prise de référence des trois axes et les mouvements de chaque axe ont été essayés sur une Bungard CCD MTC, les pas par millimètre ne sont vérifiés que par rapport à la configuration de l'original, et les sorties de la broche et de l'aspirateur n'ont pas été essayées.
* `docs/PROTOCOL.md` liste encore des parties du protocole qui sont des hypothèses ; la couche de protocole refuse de déplacer quoi que ce soit tant qu'elle n'a pas été armée.
* Les lecteurs de fichiers n'ont été essayés qu'avec de petits fichiers d'exemple écrits pour les tests, pas avec des fichiers de vraies cartes.
* La connexion au serveur (`connection = server`) n'a été exercée que contre le simulateur.
* Seuls les modèles MTC sont pris en charge ; les modèles ATC pas encore et les modèles XL ne sont pas couverts. Le chinois et le japonais demandent une police CJK parmi les polices du système.

**Ensuite**

* Exécuter un travail de perçage complet sur la machine et confirmer les pas par millimètre, la broche et les sorties de l'aspirateur.
* Essayer les lecteurs de fichiers avec des fichiers de vraies cartes.
* Des applications séparées pour les modèles ATC, construites avec ce que permet la rétro-ingénierie.
* Remplacer son propre lecteur Excellon par celui d'[EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS) quand il sera prêt.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): le frère pour le fraisage ; il utilise le protocole, l'écran et les langues de ce projet
* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): les lecteurs Excellon et G-code partagés de la suite
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): le conteneur de projet et les types de télémétrie de machine de la suite
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD) : un nœud ESP32-S3 (Ethernet ou Wi-Fi) qui se place dans le coffret de la machine, envoie les pas par son propre port série avec le cadencement qu'un adaptateur USB ne sait pas tenir, lit les voyants de la machine et pilote le variateur de la broche ; il a son propre panneau web
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER) : le centre de la famille EH3D : il reçoit les travaux de ce programme (`connection = server`) et des autres clients, et les exécute sur la machine par son nœud

## 📚 Documentation

* [Notes sur le protocole](docs/PROTOCOL.md): ce que l'on sait de la façon dont l'unité de commande est pilotée et ce qui reste une hypothèse.
* [Connexion et réglage](docs/CONNECTION.md) : le fichier de la machine, les trois connexions et chaque valeur
* [Les écrans classiques](docs/CLASSIC.md) : comment les écrans du programme MS-DOS sont reproduits et vérifiés
* [Journal des modifications](CHANGELOG.md)

## 🤝 Contribuer et communauté

* Avant une pull request, exécutez `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` et `python -I tools/ci_validate.py` : tout doit passer. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : une modification du texte se fait dans les huit fichiers, et les deux images sont dessinées à partir de `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Ne décrivez pas comme fonctionnant sur une machine ce qui n'a été essayé que sur le simulateur.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
