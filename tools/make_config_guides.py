#!/usr/bin/env python3
"""Writes config/GUIDE_<lang>.txt: the plain-words guide to the configuration files, in the eight languages of the project."""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "config")

# program name and files are filled in by the caller; {drill} is the name of the program
GUIDES = {
    "eng": """CONFIGURATION GUIDE - the files explained in plain words

WHAT IS HERE
  DRILLPRO.DCF         the machine: table size, speeds, calibration. Every line says what it is right after the value.
  DRILLPRO.DTD         the tools: one line per drill with its extra stroke, feed and spindle speed.
  machine.cfg.example  the connection to a REAL machine (serial port, USB adapter or the EH3D node). Copy it as machine.cfg to use it.

HOW TO USE THEM
  1. Keep the program, DRILLPRO.DCF and DRILLPRO.DTD in the same folder and start it from there.
  2. Out of the box nothing is connected: the program runs on the SIMULATOR, so you can try everything without any risk.
  3. Make a copy of each file before you change it. The program also keeps a .BAK copy when it saves.

THE FIRST THINGS TO SET FOR YOUR MACHINE
  - Clipping X and Y: the real size of your table (CCD 495 x 325 mm, CCD/2 325 x 270 mm). Nothing farther is ever drilled.
  - Unit: the unit of your drill files (1 = mil, the usual one). A wrong unit puts the holes in the wrong place.
  - Corrections X, Y, Z: drill two holes far apart, measure them, and multiply the correction by wanted / measured.
  - Speeds: if the motors lose steps or stall, lower the fastest XY speed and try again.

CONNECTING A REAL MACHINE
  Copy machine.cfg.example as machine.cfg, set connection = and the port, and try first with the spindle OFF and the head free to move.
  Start with the example 01_one_hole.ncd. Keep your hand near the stop; the limit switches are watched by the control unit, not by this program.

MACHINES WITH AN AUTOMATIC TOOL CHANGER (ATC)
  Their configuration file has the line "Tool change 1=Auto, 0=Man." and the positions of the rack after it; those lines are not in this basic file.
  The ATC support is new and has only been compared with a recording, not run on a real machine.

IF SOMETHING GOES WRONG
  Put your copy back. Do not change the lines marked RESERVED. The examples folder has files from one hole to twenty thousand.
""",
    "spa": """GUÍA DE CONFIGURACIÓN - los ficheros explicados en palabras sencillas

QUÉ HAY AQUÍ
  DRILLPRO.DCF         la máquina: tamaño de la mesa, velocidades, calibración. Cada línea explica qué es justo después del valor.
  DRILLPRO.DTD         las herramientas: una línea por broca con su carrera extra, avance y velocidad del husillo.
  machine.cfg.example  la conexión a una máquina REAL (puerto serie, adaptador USB o el nodo EH3D). Cópialo como machine.cfg para usarlo.

CÓMO USARLOS
  1. Deja el programa, DRILLPRO.DCF y DRILLPRO.DTD en la misma carpeta y arráncalo desde ahí.
  2. Tal como vienen no hay nada conectado: el programa funciona en el SIMULADOR, así que puedes probarlo todo sin ningún riesgo.
  3. Haz una copia de cada fichero antes de cambiarlo. El programa también guarda una copia .BAK cuando graba.

LO PRIMERO QUE HAY QUE AJUSTAR PARA TU MÁQUINA
  - Clipping X e Y: el tamaño real de tu mesa (CCD 495 x 325 mm, CCD/2 325 x 270 mm). Nunca se taladra más allá.
  - Unidad: la unidad de tus ficheros de taladros (1 = mil, la habitual). Una unidad equivocada coloca mal los agujeros.
  - Correcciones X, Y, Z: taladra dos agujeros muy separados, mídelos y multiplica la corrección por lo deseado / lo medido.
  - Velocidades: si los motores pierden pasos o se atascan, baja la velocidad máxima de XY y vuelve a probar.

CONECTAR UNA MÁQUINA REAL
  Copia machine.cfg.example como machine.cfg, pon connection = y el puerto, y prueba primero con el husillo APAGADO y el cabezal libre para moverse.
  Empieza con el ejemplo 01_one_hole.ncd. Ten la mano cerca del paro; los finales de carrera los vigila la central, no este programa.

MÁQUINAS CON CAMBIADOR AUTOMÁTICO DE HERRAMIENTA (ATC)
  Su fichero de configuración tiene la línea "Tool change 1=Auto, 0=Man." y las posiciones del rack a continuación; esas líneas no están en este fichero básico.
  El soporte de ATC es nuevo y solo se ha comparado con una grabación, no se ha probado en una máquina real.

SI ALGO SALE MAL
  Vuelve a poner tu copia. No cambies las líneas marcadas RESERVED. La carpeta examples tiene ficheros desde un agujero hasta veinte mil.
""",
    "deu": """KONFIGURATIONSANLEITUNG - die Dateien in einfachen Worten

WAS HIER LIEGT
  DRILLPRO.DCF         die Maschine: Tischgröße, Geschwindigkeiten, Kalibrierung. Jede Zeile sagt direkt hinter dem Wert, was sie bedeutet.
  DRILLPRO.DTD         die Werkzeuge: eine Zeile je Bohrer mit Zusatzhub, Vorschub und Spindeldrehzahl.
  machine.cfg.example  die Verbindung zu einer ECHTEN Maschine (serielle Schnittstelle, USB-Adapter oder der EH3D-Knoten). Als machine.cfg kopieren.

SO WERDEN SIE BENUTZT
  1. Programm, DRILLPRO.DCF und DRILLPRO.DTD im selben Ordner halten und von dort starten.
  2. Ab Werk ist nichts angeschlossen: das Programm läuft im SIMULATOR, man kann alles ohne Risiko ausprobieren.
  3. Vor jeder Änderung eine Kopie anlegen. Beim Speichern legt das Programm außerdem eine .BAK-Kopie an.

DIE ERSTEN EINSTELLUNGEN FÜR IHRE MASCHINE
  - Clipping X und Y: die wirkliche Größe Ihres Tisches (CCD 495 x 325 mm, CCD/2 325 x 270 mm). Weiter außen wird nie gebohrt.
  - Einheit: die Einheit Ihrer Bohrdateien (1 = mil, üblich). Eine falsche Einheit setzt die Löcher an die falsche Stelle.
  - Korrekturen X, Y, Z: zwei weit entfernte Löcher bohren, messen und die Korrektur mit Soll / Ist multiplizieren.
  - Geschwindigkeiten: verlieren die Motoren Schritte oder bleiben stehen, die höchste XY-Geschwindigkeit senken.

ECHTE MASCHINE ANSCHLIESSEN
  machine.cfg.example als machine.cfg kopieren, connection = und den Anschluss eintragen, zuerst bei AUSGESCHALTETER Spindel und frei beweglichem Kopf testen.
  Mit dem Beispiel 01_one_hole.ncd beginnen. Die Hand am Not-Aus halten; die Endschalter überwacht die Steuerung, nicht dieses Programm.

MASCHINEN MIT AUTOMATISCHEM WERKZEUGWECHSLER (ATC)
  Deren Konfigurationsdatei hat die Zeile "Tool change 1=Auto, 0=Man." und danach die Rack-Positionen; diese Zeilen fehlen in der Basisdatei.
  Die ATC-Unterstützung ist neu und wurde nur mit einer Aufzeichnung verglichen, nicht an einer echten Maschine erprobt.

WENN ETWAS SCHIEFGEHT
  Die Kopie zurückspielen. Zeilen mit RESERVED nicht ändern. Der Ordner examples enthält Dateien von einem Loch bis zu zwanzigtausend.
""",
    "fra": """GUIDE DE CONFIGURATION - les fichiers expliqués en mots simples

CE QUE VOUS TROUVEZ ICI
  DRILLPRO.DCF         la machine : taille de la table, vitesses, étalonnage. Chaque ligne dit ce qu'elle est juste après la valeur.
  DRILLPRO.DTD         les outils : une ligne par foret avec sa course supplémentaire, son avance et la vitesse de la broche.
  machine.cfg.example  la connexion à une machine RÉELLE (port série, adaptateur USB ou le nœud EH3D). Copiez-le sous le nom machine.cfg.

COMMENT LES UTILISER
  1. Gardez le programme, DRILLPRO.DCF et DRILLPRO.DTD dans le même dossier et lancez-le depuis ce dossier.
  2. Tel quel, rien n'est connecté : le programme tourne sur le SIMULATEUR, vous pouvez tout essayer sans risque.
  3. Faites une copie de chaque fichier avant de le modifier. Le programme garde aussi une copie .BAK quand il enregistre.

LES PREMIERS RÉGLAGES POUR VOTRE MACHINE
  - Clipping X et Y : la taille réelle de votre table (CCD 495 x 325 mm, CCD/2 325 x 270 mm). On ne perce jamais plus loin.
  - Unité : l'unité de vos fichiers de perçage (1 = mil, la plus courante). Une mauvaise unité place mal les trous.
  - Corrections X, Y, Z : percez deux trous éloignés, mesurez-les et multipliez la correction par voulu / mesuré.
  - Vitesses : si les moteurs perdent des pas ou calent, baissez la vitesse maximale XY et réessayez.

CONNECTER UNE MACHINE RÉELLE
  Copiez machine.cfg.example en machine.cfg, indiquez connection = et le port, et essayez d'abord broche ARRÊTÉE et tête libre de bouger.
  Commencez par l'exemple 01_one_hole.ncd. Gardez la main près de l'arrêt ; les fins de course sont surveillées par la centrale, pas par ce programme.

MACHINES AVEC CHANGEUR D'OUTILS AUTOMATIQUE (ATC)
  Leur fichier de configuration a la ligne "Tool change 1=Auto, 0=Man." et les positions du râtelier ensuite ; ces lignes ne sont pas dans ce fichier de base.
  Le support ATC est nouveau et n'a été comparé qu'à un enregistrement, pas essayé sur une machine réelle.

SI QUELQUE CHOSE VA MAL
  Remettez votre copie. Ne changez pas les lignes marquées RESERVED. Le dossier examples contient des fichiers d'un trou à vingt mille.
""",
    "ita": """GUIDA ALLA CONFIGURAZIONE - i file spiegati in parole semplici

COSA C'È QUI
  DRILLPRO.DCF         la macchina: dimensione del piano, velocità, taratura. Ogni riga dice cos'è subito dopo il valore.
  DRILLPRO.DTD         gli utensili: una riga per punta con corsa extra, avanzamento e velocità del mandrino.
  machine.cfg.example  il collegamento a una macchina REALE (porta seriale, adattatore USB o il nodo EH3D). Copialo come machine.cfg per usarlo.

COME USARLI
  1. Tieni il programma, DRILLPRO.DCF e DRILLPRO.DTD nella stessa cartella e avvialo da lì.
  2. Così come sono, nulla è collegato: il programma gira nel SIMULATORE, puoi provare tutto senza rischi.
  3. Fai una copia di ogni file prima di modificarlo. Il programma salva anche una copia .BAK quando scrive.

LE PRIME COSE DA IMPOSTARE PER LA TUA MACCHINA
  - Clipping X e Y: la dimensione reale del piano (CCD 495 x 325 mm, CCD/2 325 x 270 mm). Non si fora mai oltre.
  - Unità: l'unità dei tuoi file di foratura (1 = mil, la più comune). Un'unità sbagliata sposta i fori.
  - Correzioni X, Y, Z: fora due fori lontani, misurali e moltiplica la correzione per voluto / misurato.
  - Velocità: se i motori perdono passi o si bloccano, abbassa la velocità massima XY e riprova.

COLLEGARE UNA MACCHINA REALE
  Copia machine.cfg.example come machine.cfg, imposta connection = e la porta, e prova prima con il mandrino SPENTO e la testa libera di muoversi.
  Inizia con l'esempio 01_one_hole.ncd. Tieni la mano vicino all'arresto; i finecorsa li controlla la centralina, non questo programma.

MACCHINE CON CAMBIO UTENSILE AUTOMATICO (ATC)
  Il loro file di configurazione ha la riga "Tool change 1=Auto, 0=Man." e le posizioni del rack dopo; queste righe non sono in questo file di base.
  Il supporto ATC è nuovo ed è stato confrontato solo con una registrazione, non provato su una macchina reale.

SE QUALCOSA VA STORTO
  Rimetti la tua copia. Non cambiare le righe segnate RESERVED. La cartella examples ha file da un foro a ventimila.
""",
    "por": """GUIA DE CONFIGURAÇÃO - os ficheiros explicados em palavras simples

O QUE AQUI ESTÁ
  DRILLPRO.DCF         a máquina: tamanho da mesa, velocidades, calibração. Cada linha diz o que é logo a seguir ao valor.
  DRILLPRO.DTD         as ferramentas: uma linha por broca com o curso extra, o avanço e a velocidade do fuso.
  machine.cfg.example  a ligação a uma máquina REAL (porta série, adaptador USB ou o nó EH3D). Copie-o como machine.cfg para o usar.

COMO USAR
  1. Mantenha o programa, o DRILLPRO.DCF e o DRILLPRO.DTD na mesma pasta e arranque-o a partir dela.
  2. Tal como vêm, nada está ligado: o programa corre no SIMULADOR, pode experimentar tudo sem qualquer risco.
  3. Faça uma cópia de cada ficheiro antes de o alterar. O programa também guarda uma cópia .BAK quando grava.

O QUE AJUSTAR PRIMEIRO PARA A SUA MÁQUINA
  - Clipping X e Y: o tamanho real da sua mesa (CCD 495 x 325 mm, CCD/2 325 x 270 mm). Nunca se fura mais longe.
  - Unidade: a unidade dos seus ficheiros de furação (1 = mil, a habitual). Uma unidade errada coloca mal os furos.
  - Correções X, Y, Z: fure dois furos afastados, meça-os e multiplique a correção por pretendido / medido.
  - Velocidades: se os motores perderem passos ou encravarem, baixe a velocidade máxima de XY e tente de novo.

LIGAR UMA MÁQUINA REAL
  Copie machine.cfg.example como machine.cfg, indique connection = e a porta, e experimente primeiro com o fuso DESLIGADO e a cabeça livre para se mover.
  Comece pelo exemplo 01_one_hole.ncd. Tenha a mão perto da paragem; os fins de curso são vigiados pela central, não por este programa.

MÁQUINAS COM TROCADOR AUTOMÁTICO DE FERRAMENTA (ATC)
  O seu ficheiro de configuração tem a linha "Tool change 1=Auto, 0=Man." e as posições do rack a seguir; essas linhas não estão neste ficheiro básico.
  O suporte ATC é novo e só foi comparado com uma gravação, não foi experimentado numa máquina real.

SE ALGO CORRER MAL
  Reponha a sua cópia. Não altere as linhas marcadas RESERVED. A pasta examples tem ficheiros desde um furo até vinte mil.
""",
    "zho": """配置指南 - 用简单的话解释这些文件

这里有什么
  DRILLPRO.DCF         机器：工作台尺寸、速度、校准。每一行在数值后面直接说明它是什么。
  DRILLPRO.DTD         刀具：每个钻头一行，含额外行程、进给和主轴转速。
  machine.cfg.example  连接真实机器（串口、USB 适配器或 EH3D 节点）。复制为 machine.cfg 即可使用。

怎么用
  1. 把程序、DRILLPRO.DCF 和 DRILLPRO.DTD 放在同一个文件夹，并从该文件夹启动。
  2. 默认没有连接任何设备：程序运行在模拟器上，可以放心尝试一切，没有任何风险。
  3. 修改任何文件之前先备份。程序保存时也会留下 .BAK 副本。

为您的机器首先要设置的内容
  - Clipping X 和 Y：工作台的实际尺寸（CCD 为 495 x 325 毫米，CCD/2 为 325 x 270 毫米）。永远不会钻到更远的地方。
  - 单位：钻孔文件的单位（1 = mil，最常见）。单位错误会使孔位置错误。
  - 校正 X、Y、Z：钻两个相距较远的孔并测量，把校正值乘以 期望值 / 测量值。
  - 速度：如果电机丢步或堵转，请降低 XY 最高速度后重试。

连接真实机器
  把 machine.cfg.example 复制为 machine.cfg，填写 connection = 和端口，先在主轴关闭、机头可自由移动的情况下试验。
  先用示例 01_one_hole.ncd。手放在急停附近；限位开关由控制单元监视，而不是本程序。

带自动换刀器（ATC）的机器
  它们的配置文件有 "Tool change 1=Auto, 0=Man." 这一行，后面是刀库位置；这个基本文件中没有这些行。
  ATC 支持是新的，只与一段录制数据进行了比对，尚未在真实机器上运行。

出现问题时
  放回您的备份。不要修改标有 RESERVED 的行。examples 文件夹里有从一个孔到两万个孔的文件。
""",
    "jpn": """設定ガイド - ファイルをやさしい言葉で説明します

ここにあるもの
  DRILLPRO.DCF         機械の設定：テーブルの大きさ、速度、校正。各行の値のすぐ後ろに意味が書いてあります。
  DRILLPRO.DTD         工具：ドリル1本につき1行（追加ストローク、送り、主軸回転数）。
  machine.cfg.example  実機（シリアルポート、USBアダプター、EH3Dノード）への接続。machine.cfg としてコピーして使います。

使い方
  1. プログラム、DRILLPRO.DCF、DRILLPRO.DTD を同じフォルダーに置き、そこから起動します。
  2. 初期状態では何も接続されず、シミュレーターで動きます。危険なくすべて試せます。
  3. 変更する前に各ファイルのコピーを取ってください。保存時にプログラムも .BAK のコピーを残します。

機械に合わせて最初に設定するもの
  - Clipping X と Y：テーブルの実際の大きさ（CCD は 495 x 325 mm、CCD/2 は 325 x 270 mm）。それより外側は穴あけしません。
  - 単位：ドリルファイルの単位（1 = mil が一般的）。単位が違うと穴の位置がずれます。
  - 補正 X、Y、Z：離れた2つの穴をあけて測り、補正値に 目標 / 実測 を掛けます。
  - 速度：モーターが脱調したり止まったりしたら、XY の最高速度を下げて再試行します。

実機をつなぐ
  machine.cfg.example を machine.cfg としてコピーし、connection = とポートを設定します。まず主軸をオフ、ヘッドが自由に動ける状態で試します。
  最初は例 01_one_hole.ncd から。停止にすぐ手が届くようにしてください。リミットスイッチは制御ユニットが監視し、このプログラムは監視しません。

自動工具交換機（ATC）付きの機械
  その設定ファイルには "Tool change 1=Auto, 0=Man." の行とその後にラックの位置があります。この基本ファイルにはありません。
  ATC対応は新しく、記録データとの比較のみで、実機では動かしていません。

うまくいかないとき
  取っておいたコピーに戻します。RESERVED と書かれた行は変更しないでください。examples フォルダーには1穴から2万穴までのファイルがあります。
""",
}


def main():
    os.makedirs(OUT, exist_ok=True)
    for lang, text in GUIDES.items():
        with open(os.path.join(OUT, f"GUIDE_{lang}.txt"), "w", newline="\r\n", encoding="utf-8") as f:
            f.write(text)
    print("guides written:", sorted(GUIDES))


if __name__ == "__main__":
    main()
