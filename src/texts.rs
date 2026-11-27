//! User-visible texts of DRILLPRO in the eight supported languages.
//! Order: English (default), Spanish, Italian, German, French, Portuguese, Chinese, Japanese.

use crate::i18n::Lang;
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
    LblLeft => ["Time left", "Tiempo restante", "Tempo rimasto", "Verbleibende Zeit", "Temps restant", "Tempo restante", "剩余时间", "残り時間"],
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
    ToolDefaults => ["Not in the tool table; cautious values are used for tool", "No está en la tabla de herramientas; se usan valores cautelosos para la herramienta", "Non è nella tabella utensili; si usano valori prudenti per l'utensile", "Nicht in der Werkzeugtabelle; für das Werkzeug gelten vorsichtige Werte", "Absent de la table d'outils ; valeurs prudentes utilisées pour l'outil", "Não está na tabela de ferramentas; usam-se valores prudentes para a ferramenta", "不在刀具表中，该刀具使用保守值：", "工具テーブルにありません。安全側の値を使う工具："],
    RackGap => ["A place of the tool changer has no position in the machine file", "Un puesto del cambiador no tiene posición en el archivo de la máquina", "Un posto del cambio utensile non ha posizione nel file della macchina", "Ein Platz des Werkzeugwechslers hat keine Position in der Maschinendatei", "Une place du changeur n'a pas de position dans le fichier de la machine", "Um lugar do trocador não tem posição no ficheiro da máquina", "换刀器的某个位置在机床文件中没有坐标", "ツールチェンジャーのある位置が機械ファイルに定義されていません"],
    RackOutside => ["A point of the bar is outside the reach of the machine", "Un punto de la barra está fuera del alcance de la máquina", "Un punto della barra è fuori dalla portata della macchina", "Ein Punkt der Leiste liegt außerhalb der Reichweite der Maschine", "Un point de la barre est hors de portée de la machine", "Um ponto da barra está fora do alcance da máquina", "刀具排上有一个点超出了机床的行程范围", "バーの1点が機械の可動範囲外です"],
    LevelServer => ["The height map cannot be used when the job goes through the server", "El mapa de alturas no se puede usar cuando el trabajo pasa por el servidor", "La mappa delle altezze non si può usare quando il lavoro passa dal server", "Die Höhenkarte ist nicht nutzbar, wenn der Auftrag über den Server läuft", "La carte des hauteurs n'est pas utilisable quand le travail passe par le serveur", "O mapa de alturas não pode ser usado quando o trabalho passa pelo servidor", "作业经服务器运行时无法使用高度图", "ジョブがサーバー経由のときは高さマップを使えません"],
    IgnoredLines => ["The file has slots or routed moves that are not read", "El archivo tiene ranuras o movimientos de fresado que no se leen", "Il file ha asole o movimenti di fresatura che non vengono letti", "Die Datei enthält Langlöcher oder Fräsbewegungen, die nicht gelesen werden", "Le fichier contient des rainures ou des déplacements de fraisage qui ne sont pas lus", "O ficheiro tem ranhuras ou movimentos de fresagem que não são lidos", "文件含有未读取的槽孔或铣削动作", "ファイルに読み込まれないスロットまたはルーティング動作があります"],
    ProjectExists => ["{0} is there already: choose a name that is not taken", "{0} ya existe: elija un nombre que no esté usado", "{0} esiste già: scegli un nome libero", "{0} ist schon vorhanden: wählen Sie einen freien Namen", "{0} existe déjà : choisissez un nom libre", "{0} já existe: escolha um nome que não esteja usado", "{0} 已存在：请选择未被使用的名称", "{0} はすでに存在します。使われていない名前を選んでください"],
    UnitsSupposed => ["The file does not state its unit; the numbers are read as thousandths of an inch", "El archivo no indica su unidad; los números se leen como milésimas de pulgada", "Il file non indica l'unità; i numeri sono letti come millesimi di pollice", "Die Datei nennt keine Einheit; die Zahlen gelten als Tausendstel Zoll", "Le fichier n'indique pas son unité ; les nombres sont lus en millièmes de pouce", "O ficheiro não indica a unidade; os números são lidos como milésimas de polegada", "文件未声明单位，数值按千分之一英寸读取", "ファイルに単位の指定がありません。数値は1/1000インチとして読み込みます"],
    FormatChosen => ["The file does not declare its number format; it is read with the chosen one", "El archivo no declara el formato de sus números; se lee con el elegido", "Il file non dichiara il formato dei numeri; è letto con quello scelto", "Die Datei gibt ihr Zahlenformat nicht an; sie wird mit dem gewählten gelesen", "Le fichier ne déclare pas le format de ses nombres ; il est lu avec le format choisi", "O ficheiro não declara o formato dos números; é lido com o escolhido", "文件未声明数值格式，按所选格式读取", "ファイルに数値書式の宣言がないため、選択した書式で読み込みます"],
    HowFormat => ["Choose it in the format window or with --drill-format NAME; it is remembered in DRILLPRO.FMT of the data folder", "Elíjalo en la ventana de formato o con --drill-format NOMBRE; se recuerda en DRILLPRO.FMT de la carpeta de datos", "Sceglierlo nella finestra del formato o con --drill-format NOME; è ricordato in DRILLPRO.FMT della cartella dei dati", "Im Formatfenster oder mit --drill-format NAME wählen; es wird in DRILLPRO.FMT im Datenordner gemerkt", "Choisissez-le dans la fenêtre du format ou avec --drill-format NOM ; il est mémorisé dans DRILLPRO.FMT du dossier de données", "Escolha-o na janela do formato ou com --drill-format NOME; fica guardado em DRILLPRO.FMT na pasta de dados", "在格式窗口中选择，或使用 --drill-format 名称；选择会记录在数据文件夹的 DRILLPRO.FMT 中", "書式ウィンドウか --drill-format 名前 で選んでください。選択はデータフォルダーの DRILLPRO.FMT に保存されます"],
    FormatTitle => ["Number format of this drill file", "Formato numérico de este taladrado", "Formato numerico di questa foratura", "Zahlenformat dieser Bohrdatei", "Format numérique de ce perçage", "Formato numérico desta furação", "此钻孔文件的数值格式", "このドリルファイルの数値書式"],
    FormatNone => ["None (refuse)", "Ninguno (rechazar)", "Nessuno (rifiuta)", "Keines (ablehnen)", "Aucun (refuser)", "Nenhum (recusar)", "不选择（拒绝）", "選ばない（拒否）"],
    FormatLabel => ["Drill format", "Formato taladro", "Formato foratura", "Bohrformat", "Format perçage", "Formato furação", "钻孔格式", "ドリル書式"],
    FormatKept => ["The choice is kept for the next files", "La elección se guarda para los próximos archivos", "La scelta resta per i prossimi file", "Die Wahl gilt auch für die nächsten Dateien", "Le choix reste pour les fichiers suivants", "A escolha fica para os próximos ficheiros", "此选择会用于以后的文件", "この選択は次のファイルにも使われます"],
    DuplicateHoles => ["Holes that repeat the place of an earlier hole of the same tool (drilled twice)", "Agujeros que repiten el sitio de otro agujero anterior de la misma herramienta (taladrados dos veces)", "Fori che ripetono la posizione di un foro precedente dello stesso utensile (forati due volte)", "Bohrungen, die den Ort einer früheren Bohrung desselben Werkzeugs wiederholen (doppelt gebohrt)", "Trous qui répètent la place d'un trou précédent du même outil (percés deux fois)", "Furos que repetem o sítio de um furo anterior da mesma ferramenta (furados duas vezes)", "与同一刀具之前的孔位置重复的孔（重复钻孔）", "同じ工具の先の穴と同じ位置にある穴（二重にあける穴）"],
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
    SpindleLimit => ["Spindle limit:", "Límite del husillo:", "Limite del mandrino:", "Spindelgrenze:", "Limite de la broche :", "Limite do fuso:", "主轴限制：", "スピンドル上限："],
    ChainDropped => ["Server: tool changes not chained", "Servidor: cambios no encadenados", "Server: cambi non concatenati", "Server: Wechsel nicht verkettet", "Serveur : changements non enchaînés", "Servidor: trocas não encadeadas", "服务器：换刀未衔接", "サーバー：工具交換は連結されません"],
    KeyStop => ["Esc: stop", "Esc: detener", "Esc: ferma", "Esc: stoppen", "Échap : arrêter", "Esc: parar", "Esc：停止", "Esc：停止"],

    RealMachine => ["REAL MACHINE ({}) - Start moves it", "MÁQUINA REAL ({}) - Iniciar la mueve", "MACCHINA REALE ({}) - Avvia la muove", "ECHTE MASCHINE ({}) - Start bewegt sie", "MACHINE RÉELLE ({}) - Démarrer la déplace", "MÁQUINA REAL ({}) - Iniciar move-a", "真实机器 ({}) - 开始将使其运动", "実機 ({}) - 開始すると動きます"],
    RealStop => ["Esc stops it; keep a hand near the power switch", "Esc la detiene; ten una mano cerca del interruptor", "Esc la ferma; tieni una mano vicino all'interruttore", "Esc stoppt sie; Hand am Netzschalter halten", "Échap l'arrête ; gardez une main près de l'interrupteur", "Esc pára-a; tenha uma mão perto do interruptor", "Esc 停止；手放在电源开关旁", "Esc で停止；電源スイッチの近くに手を置いてください"],
    ClipCut => ["Cut", "Cortar", "Taglia", "Ausschneiden", "Couper", "Cortar", "剪切", "切り取り"],
    ClipCopy => ["Copy", "Copiar", "Copia", "Kopieren", "Copier", "Copiar", "复制", "コピー"],
    ClipPaste => ["Paste", "Pegar", "Incolla", "Einfügen", "Coller", "Colar", "粘贴", "貼り付け"],
    ClipSelectAll => ["Select all", "Seleccionar todo", "Seleziona tutto", "Alles auswählen", "Tout sélectionner", "Selecionar tudo", "全选", "すべて選択"],
    ClipUndo => ["Undo", "Deshacer", "Annulla", "Rückgängig", "Annuler", "Anular", "撤销", "元に戻す"],
    FullScreen => ["Full screen", "Pantalla completa", "Schermo intero", "Vollbild", "Plein écran", "Ecrã inteiro", "全屏", "全画面表示"],

    SumTitle => ["Check before starting", "Revisa antes de empezar", "Controlla prima di avviare", "Vor dem Start prüfen", "Vérifiez avant de démarrer", "Verifique antes de iniciar", "开始前请检查", "開始前に確認"],
    BtnStartJob => ["Start the job", "Iniciar el trabajo", "Avvia il lavoro", "Auftrag starten", "Démarrer le travail", "Iniciar o trabalho", "开始作业", "ジョブを開始"],
    BtnCancel => ["Cancel", "Cancelar", "Annulla", "Abbrechen", "Annuler", "Cancelar", "取消", "キャンセル"],
    FailInternal => ["internal error", "error interno", "errore interno", "interner Fehler", "erreur interne", "erro interno", "内部错误", "内部エラー"],
    FailConnecting => ["connecting", "al conectar", "connessione", "beim Verbinden", "connexion", "ao ligar", "连接时", "接続中"],
    FailArming => ["arming the machine", "al armar la máquina", "attivazione della macchina", "beim Scharfschalten der Maschine", "armement de la machine", "ao armar a máquina", "机器就绪时", "機械の有効化中"],
    FailRunning => ["running the job", "al ejecutar el trabajo", "esecuzione del lavoro", "beim Ausführen des Auftrags", "exécution du travail", "ao executar o trabalho", "执行作业时", "ジョブの実行中"],
    FailReference => ["reference run", "recorrido de referencia", "ricerca del riferimento", "Referenzfahrt", "prise de référence", "ida à referência", "回参考点", "原点復帰"],
    FailMoving => ["moving the head", "al mover el cabezal", "movimento della testa", "beim Bewegen des Kopfes", "déplacement de la tête", "ao mover a cabeça", "移动机头时", "ヘッドの移動中"],
    FailLifting => ["lifting the head", "al levantar el cabezal", "sollevamento della testa", "beim Anheben des Kopfes", "levée de la tête", "ao levantar a cabeça", "抬起机头时", "ヘッドの上昇中"],
    FailOrigin => ["returning to the origin", "al volver al origen", "ritorno all'origine", "bei der Rückfahrt zum Ursprung", "retour à l'origine", "no regresso à origem", "返回原点时", "原点へ戻る途中"],
    FailServerJob => ["the job failed on the server", "el trabajo falló en el servidor", "il lavoro è fallito sul server", "der Auftrag ist auf dem Server fehlgeschlagen", "le travail a échoué sur le serveur", "o trabalho falhou no servidor", "作业在服务器上失败", "サーバーでジョブが失敗しました"],
    FailNoServerSet => ["server_address and server_machine are not set in the connection file", "server_address y server_machine no están definidos en el archivo de conexión", "server_address e server_machine non sono impostati nel file di connessione", "server_address und server_machine fehlen in der Verbindungsdatei", "server_address et server_machine ne sont pas définis dans le fichier de connexion", "server_address e server_machine não estão definidos no ficheiro de ligação", "连接文件中未设置 server_address 和 server_machine", "接続ファイルに server_address と server_machine がありません"],
    FailNoJobId => ["the server did not give the job an id", "el servidor no dio un identificador al trabajo", "il server non ha dato un identificativo al lavoro", "der Server hat dem Auftrag keine Kennung gegeben", "le serveur n'a pas donné d'identifiant au travail", "o servidor não deu um identificador ao trabalho", "服务器没有给作业分配编号", "サーバーがジョブに識別子を付けませんでした"],
    FailTeachServer => ["the Offset window does not move a machine through the server", "la ventana Offset no mueve una máquina a través del servidor", "la finestra Offset non muove una macchina tramite il server", "das Fenster Versatz bewegt keine Maschine über den Server", "la fenêtre Décalage ne déplace pas une machine par le serveur", "a janela Desfase não move uma máquina através do servidor", "偏移窗口不能通过服务器移动机器", "オフセット画面はサーバー経由では機械を動かしません"],
    FailLostServer => ["lost the server ({}); the job may still be running on it", "se perdió el servidor ({}); el trabajo puede seguir en marcha en él", "server perso ({}); il lavoro può essere ancora in corso", "Server verloren ({}); der Auftrag läuft dort vielleicht noch", "serveur perdu ({}) ; le travail est peut-être encore en cours", "servidor perdido ({}); o trabalho pode continuar em execução nele", "与服务器失去联系（{}）；作业可能仍在服务器上运行", "サーバーとの接続が切れました（{}）。ジョブは実行中の可能性があります"],
    FailNotSupported => ["not supported", "no disponible", "non supportato", "nicht unterstützt", "non pris en charge", "não suportado", "不支持", "対応していません"],
    FailNoConfig => ["no configuration file", "no hay archivo de configuración", "nessun file di configurazione", "keine Konfigurationsdatei", "aucun fichier de configuration", "não há ficheiro de configuração", "没有配置文件", "設定ファイルがありません"],
    FailBadConfig => ["the configuration file is not in the expected layout", "el archivo de configuración no tiene el formato esperado", "il file di configurazione non ha il formato previsto", "die Konfigurationsdatei hat nicht den erwarteten Aufbau", "le fichier de configuration n'a pas la disposition attendue", "o ficheiro de configuração não tem o formato esperado", "配置文件的格式不符合要求", "設定ファイルの形式が想定と異なります"],
    FailNoArchiveFile => ["no file of this kind in the archive", "no hay ningún archivo de este tipo en el comprimido", "nessun file di questo tipo nell'archivio", "keine Datei dieser Art im Archiv", "aucun fichier de ce type dans l'archive", "não há nenhum ficheiro deste tipo no arquivo comprimido", "压缩包中没有此类文件", "アーカイブにこの種類のファイルがありません"],
    SmallTerminal => ["The terminal needs 80 columns by 25 rows (it has {}x{}).", "El terminal necesita 80 columnas por 25 filas (tiene {}x{}).", "Il terminale richiede 80 colonne per 25 righe (ne ha {}x{}).", "Das Terminal braucht 80 Spalten und 25 Zeilen (es hat {}x{}).", "Le terminal a besoin de 80 colonnes sur 25 lignes (il a {}x{}).", "O terminal precisa de 80 colunas por 25 linhas (tem {}x{}).", "终端需要 80 列 25 行（当前为 {}x{}）。", "端末は 80 桁 25 行が必要です（現在 {}x{}）。"],
}

/// The failures the program words itself that carry a detail after a colon (`connecting: <detail>`).
const FAILED_STEPS: [Text; 8] = [
    Text::FailInternal,
    Text::FailConnecting,
    Text::FailArming,
    Text::FailRunning,
    Text::FailReference,
    Text::FailMoving,
    Text::FailLifting,
    Text::FailOrigin,
];

/// The failures the program words itself that are a whole sentence.
const FAILED_WHOLE: [Text; 8] = [
    Text::FailInternal,
    Text::FailServerJob,
    Text::FailNoServerSet,
    Text::FailNoJobId,
    Text::FailTeachServer,
    Text::FailNotSupported,
    Text::FailNoConfig,
    Text::FailBadConfig,
];

/// An error as a screen shows it: the sentences the program words itself in the language of the screen, with the detail of the system or of the link as it came. The
/// error itself is not changed (it is what the logs, the command line and other programs read); a text that is not one of these is shown as it is.
pub fn failure_text(lang: Lang, why: &str) -> String {
    if lang == Lang::En {
        return why.to_string();
    }
    if let Some(k) = FAILED_WHOLE.into_iter().find(|k| tr(Lang::En, *k) == why) {
        return tr(lang, k).to_string();
    }
    for k in FAILED_STEPS {
        if let Some(detail) = why
            .strip_prefix(tr(Lang::En, k))
            .and_then(|rest| rest.strip_prefix(": "))
        {
            return format!("{}: {}", tr(lang, k), failure_text(lang, detail));
        }
    }
    if let Some((head, tail)) = tr(Lang::En, Text::FailLostServer).split_once("{}") {
        if let Some(detail) = why.strip_prefix(head).and_then(|r| r.strip_suffix(tail)) {
            return tr(lang, Text::FailLostServer).replace("{}", detail);
        }
    }
    let archive = format!(": {}", tr(Lang::En, Text::FailNoArchiveFile));
    if let Some(path) = why.strip_suffix(archive.as_str()) {
        return format!("{path}: {}", tr(lang, Text::FailNoArchiveFile));
    }
    why.to_string()
}
