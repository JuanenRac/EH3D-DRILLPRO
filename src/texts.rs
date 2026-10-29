//! User-visible texts of DRILLPRO in the eight supported languages.
//! Order: English (default), Spanish, Italian, German, French, Portuguese, Chinese, Japanese.

use crate::translations;

translations! { Text, tr, ALL_TEXTS;
    MenuLoad => ["~L~oad", "~C~argar", "~C~arica", "~L~aden", "~C~harger", "~C~arregar", "载入(~L~)", "読込(~L~)"],
    MenuVectors => ["~V~ectors", "~V~ectores", "~V~ettori", "~V~ektoren", "~V~ecteurs", "~V~etores", "矢量(~V~)", "ベクター(~V~)"],
    MenuTools => ["~T~ools", "~H~erramientas", "~U~tensili", "~W~erkzeuge", "~O~utils", "~F~erramentas", "刀具(~T~)", "工具(~T~)"],
    MenuConfig => ["~C~onfig.", "C~o~nfig.", "Co~n~fig.", "~K~onfig.", "Co~n~fig.", "Co~n~fig.", "配置(~C~)", "設定(~C~)"],
    MenuSort => ["S~o~rting", "O~r~den", "~O~rdina", "~S~ortierung", "~T~ri", "~O~rdenar", "排序(~S~)", "並替(~S~)"],
    MenuStart => ["~S~tart", "~I~niciar", "~A~vvia", "S~t~art", "~D~émarrer", "~I~niciar", "开始(~A~)", "開始(~A~)"],
    MenuLanguage => ["Lan~g~uage", "I~d~ioma", "~L~ingua", "S~p~rache", "~L~angue", "I~d~ioma", "语言(~G~)", "言語(~G~)"],
    MenuQuit => ["~Q~uit", "~S~alir", "~E~sci", "~B~eenden", "~Q~uitter", "~S~air", "退出(~Q~)", "終了(~Q~)"],

    TitleSelectFile => ["Select File", "Seleccionar archivo", "Seleziona file", "Datei wählen", "Choisir un fichier", "Selecionar ficheiro", "选择文件", "ファイル選択"],
    TitleVectors => ["Vectors", "Vectores", "Vettori", "Vektoren", "Vecteurs", "Vetores", "矢量", "ベクター"],
    TitleTools => ["Tool Data", "Datos de herramientas", "Dati utensili", "Werkzeugdaten", "Données d'outils", "Dados das ferramentas", "刀具数据", "工具データ"],
    TitleConfig => ["Configuration", "Configuración", "Configurazione", "Konfiguration", "Configuration", "Configuração", "配置", "設定"],
    TitleSort => ["Coordinate sorting", "Orden de coordenadas", "Ordinamento coordinate", "Koordinatensortierung", "Tri des coordonnées", "Ordenação de coordenadas", "坐标排序", "座標の並べ替え"],
    TitleLanguage => ["Language", "Idioma", "Lingua", "Sprache", "Langue", "Idioma", "语言", "言語"],
    TitleDrilling => ["Drilling", "Taladrado", "Foratura", "Bohren", "Perçage", "Furação", "钻孔", "ドリル加工"],
    TitleQuit => ["Quit", "Salir", "Esci", "Beenden", "Quitter", "Sair", "退出", "終了"],

    LblFile => ["File", "Archivo", "File", "Datei", "Fichier", "Ficheiro", "文件", "ファイル"],
    LblHoles => ["Holes", "Agujeros", "Fori", "Bohrungen", "Trous", "Furos", "孔数", "穴数"],
    LblToolsUsed => ["Tools", "Herramientas", "Utensili", "Werkzeuge", "Outils", "Ferramentas", "刀具", "工具"],
    LblSize => ["Size (mm)", "Tamaño (mm)", "Dimensioni (mm)", "Größe (mm)", "Taille (mm)", "Tamanho (mm)", "尺寸 (mm)", "サイズ (mm)"],
    LblTravel => ["Travel (mm)", "Recorrido (mm)", "Percorso (mm)", "Verfahrweg (mm)", "Déplacement (mm)", "Percurso (mm)", "行程 (mm)", "移動量 (mm)"],
    LblSorting => ["Sorting", "Orden", "Ordine", "Sortierung", "Tri", "Ordem", "排序", "並べ替え"],
    LblProgress => ["Progress", "Progreso", "Avanzamento", "Fortschritt", "Progression", "Progresso", "进度", "進捗"],
    LblTime => ["Simulated time (s)", "Tiempo simulado (s)", "Tempo simulato (s)", "Simulierte Zeit (s)", "Temps simulé (s)", "Tempo simulado (s)", "模拟时间 (秒)", "シミュレーション時間 (秒)"],
    LblSerial => ["Serial interface", "Interfaz serie", "Interfaccia seriale", "Serielle Schnittstelle", "Interface série", "Interface série", "串口", "シリアル"],

    SortNone => ["None", "Ninguno", "Nessuno", "Keine", "Aucun", "Nenhum", "无", "なし"],
    SortX => ["X first", "Primero X", "Prima X", "X zuerst", "X d'abord", "X primeiro", "先X", "X優先"],
    SortY => ["Y first", "Primero Y", "Prima Y", "Y zuerst", "Y d'abord", "Y primeiro", "先Y", "Y優先"],
    SortSpecial => ["Special (shortest path)", "Especial (camino más corto)", "Speciale (percorso più breve)", "Spezial (kürzester Weg)", "Spécial (chemin le plus court)", "Especial (caminho mais curto)", "特殊（最短路径）", "特殊（最短経路）"],

    ColTool => ["Tool", "Herram.", "Utens.", "Werkz.", "Outil", "Ferram.", "刀具", "工具"],
    ColStroke => ["Stroke (mm)", "Carrera (mm)", "Corsa (mm)", "Hub (mm)", "Course (mm)", "Curso (mm)", "行程 (mm)", "ストローク (mm)"],
    ColFeed => ["Feed (mm/min)", "Avance (mm/min)", "Avanzam. (mm/min)", "Vorschub (mm/min)", "Avance (mm/min)", "Avanço (mm/min)", "进给 (mm/min)", "送り (mm/min)"],
    ColSpindle => ["Spindle (rpm)", "Husillo (rpm)", "Mandrino (giri/min)", "Spindel (U/min)", "Broche (tr/min)", "Fuso (rpm)", "主轴 (rpm)", "スピンドル (rpm)"],

    NoFile => ["No file loaded", "Ningún archivo cargado", "Nessun file caricato", "Keine Datei geladen", "Aucun fichier chargé", "Nenhum ficheiro carregado", "未载入文件", "ファイル未読込"],
    NoToolFile => ["Tool data file not found", "No se encuentra el archivo de herramientas", "File utensili non trovato", "Werkzeugdatei nicht gefunden", "Fichier d'outils introuvable", "Ficheiro de ferramentas não encontrado", "未找到刀具数据文件", "工具データファイルが見つかりません"],
    NoConfigFile => ["Configuration file not found", "No se encuentra el archivo de configuración", "File di configurazione non trovato", "Konfigurationsdatei nicht gefunden", "Fichier de configuration introuvable", "Ficheiro de configuração não encontrado", "未找到配置文件", "設定ファイルが見つかりません"],
    TooManyTools => ["Too many tools for the tool changer (most:", "Demasiadas herramientas para el cambiador (máximo:", "Troppi utensili per il cambio utensile (massimo:", "Zu viele Werkzeuge für den Werkzeugwechsler (höchstens:", "Trop d'outils pour le changeur (maximum :", "Demasiadas ferramentas para o trocador (máximo:", "刀具过多，超出换刀器容量（最多：", "ツールチェンジャーに対して工具が多すぎます（最大："],
    ErrRead => ["Error reading drill file", "Error al leer el archivo de taladros", "Errore di lettura del file di foratura", "Fehler beim Lesen der Bohrdatei", "Erreur de lecture du fichier de perçage", "Erro ao ler o ficheiro de furação", "读取钻孔文件出错", "ドリルファイルの読込エラー"],
    SimOnly => ["SIMULATION - no machine connected", "SIMULACIÓN - sin máquina conectada", "SIMULAZIONE - nessuna macchina collegata", "SIMULATION - keine Maschine verbunden", "SIMULATION - aucune machine connectée", "SIMULAÇÃO - sem máquina ligada", "模拟 - 未连接机器", "シミュレーション - 機械未接続"],
    Uncalibrated => ["Machine not calibrated: motion is simulated only", "Máquina sin calibrar: el movimiento es solo simulado", "Macchina non calibrata: movimento solo simulato", "Maschine nicht kalibriert: Bewegung nur simuliert", "Machine non calibrée : mouvement simulé uniquement", "Máquina não calibrada: movimento apenas simulado", "机器未校准：仅模拟运动", "機械は未校正：動作はシミュレーションのみ"],
    Connecting => ["Connecting control unit ...", "Conectando con la controladora ...", "Connessione all'unità di controllo ...", "Verbinde mit der Steuereinheit ...", "Connexion à l'unité de commande ...", "A ligar à unidade de controlo ...", "正在连接控制器 ...", "コントローラーに接続中 ..."],
    Drilling => ["Drilling ...", "Taladrando ...", "Foratura ...", "Bohre ...", "Perçage ...", "A furar ...", "钻孔中 ...", "ドリル加工中 ..."],
    Done => ["Done", "Terminado", "Fatto", "Fertig", "Terminé", "Concluído", "完成", "完了"],
    QuitAsk => ["Do you really want to quit?", "¿Seguro que quieres salir?", "Vuoi davvero uscire?", "Wirklich beenden?", "Voulez-vous vraiment quitter ?", "Tem a certeza de que quer sair?", "确定要退出吗？", "本当に終了しますか？"],
    BtnYes => ["Yes", "Sí", "Sì", "Ja", "Oui", "Sim", "是", "はい"],
    BtnNo => ["No", "No", "No", "Nein", "Non", "Não", "否", "いいえ"],
    BtnStop => ["Stop", "Detener", "Ferma", "Stopp", "Arrêter", "Parar", "停止", "停止"],
    Directory => ["Directory", "Directorio", "Cartella", "Verzeichnis", "Répertoire", "Pasta", "目录", "フォルダー"],
    StatusMain => [
        "Alt+letter: menu   Esc: back   Enter: accept   Up/Down: move",
        "Alt+letra: menú   Esc: volver   Intro: aceptar   Arriba/Abajo: mover",
        "Alt+lettera: menu   Esc: indietro   Invio: conferma   Su/Giù: muovi",
        "Alt+Buchstabe: Menü   Esc: zurück   Enter: übernehmen   Hoch/Runter: bewegen",
        "Alt+lettre : menu   Échap : retour   Entrée : valider   Haut/Bas : déplacer",
        "Alt+letra: menu   Esc: voltar   Enter: aceitar   Cima/Baixo: mover",
        "Alt+字母：菜单  Esc：返回  回车：确定  上/下：移动",
        "Alt+文字：メニュー  Esc：戻る  Enter：決定  上/下：移動"
    ],

    WaitToolChange => ["Insert tool {} and press Enter", "Inserte la herramienta {} y pulse Intro", "Inserire l'utensile {} e premere Invio", "Werkzeug {} einsetzen und Enter drücken", "Insérez l'outil {} et appuyez sur Entrée", "Insira a ferramenta {} e prima Enter", "请装入刀具 {} 并按回车", "工具 {} を取り付けて Enter を押してください"],
    WaitSpindle => ["Press Enter to start the spindle", "Pulse Intro para arrancar el husillo", "Premere Invio per avviare il mandrino", "Enter drücken, um die Spindel zu starten", "Appuyez sur Entrée pour démarrer la broche", "Prima Enter para arrancar o fuso", "按回车启动主轴", "Enter を押してスピンドルを起動"],
    Stopped => ["Stopped (drivers off)", "Detenido (drivers apagados)", "Fermato (driver spenti)", "Gestoppt (Treiber aus)", "Arrêté (drivers coupés)", "Parado (drivers desligados)", "已停止（驱动器已关闭）", "停止しました（ドライバ off）"],
    LinkFailed => ["Connection failed", "Fallo de conexión", "Connessione fallita", "Verbindung fehlgeschlagen", "Échec de la connexion", "Falha na ligação", "连接失败", "接続に失敗しました"],
    LblLink => ["Link", "Enlace", "Collegamento", "Verbindung", "Liaison", "Ligação", "连接", "接続"],
    LblItems => ["Steps of the job", "Pasos del trabajo", "Passi del lavoro", "Arbeitsschritte", "Étapes du travail", "Passos do trabalho", "作业步骤", "作業ステップ"],
    KeyStop => ["Esc: stop", "Esc: detener", "Esc: ferma", "Esc: stoppen", "Échap : arrêter", "Esc: parar", "Esc：停止", "Esc：停止"],

    RealMachine => ["REAL MACHINE ({}) - Start moves it", "MÁQUINA REAL ({}) - Iniciar la mueve", "MACCHINA REALE ({}) - Avvia la muove", "ECHTE MASCHINE ({}) - Start bewegt sie", "MACHINE RÉELLE ({}) - Démarrer la déplace", "MÁQUINA REAL ({}) - Iniciar move-a", "真实机器 ({}) - 开始将使其运动", "実機 ({}) - 開始すると動きます"],
    RealStop => ["Esc stops it; keep a hand near the power switch", "Esc la detiene; ten una mano cerca del interruptor", "Esc la ferma; tieni una mano vicino all'interruttore", "Esc stoppt sie; Hand am Netzschalter halten", "Échap l'arrête ; gardez une main près de l'interrupteur", "Esc pára-a; tenha uma mão perto do interruptor", "Esc 停止；手放在电源开关旁", "Esc で停止；電源スイッチの近くに手を置いてください"],
}
