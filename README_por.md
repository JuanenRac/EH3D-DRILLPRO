<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-DRILLPRO banner" width="100%">
</p>

# 🔩 EH3D-DRILLPRO

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Uma reimplementação multiplataforma em Rust do fluxo de furação do clássico software DOS das máquinas Bungard CCD, com uma interface em modo de texto semelhante à original, em oito idiomas

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** um programa em Rust com interface de texto de 80x25 que lê ficheiros de furação Excellon `.NCD`, a tabela de ferramentas e a configuração da máquina, ordena os furos, mostra-os e executa todo o trabalho **contra um simulador** (por omissão) ou, quando o ficheiro da máquina o indica, **numa máquina real** pela porta série do computador, um adaptador USB-série ou o EH3D-NODE-BUNGARD. 213 testes correm num computador. Numa Bungard CCD MTC real foram experimentados a referência dos três eixos (Z, X e Y até aos fins de curso, com o sinal «os três em origem» da unidade de controlo confirmado em direto) e os movimentos de cada eixo; **um trabalho de furação completo ainda não correu na máquina**, e os passos por milímetro só foram confrontados com a configuração do original. Por predefinição, aparecem os ecrãs clássicos do programa MS-DOS, verificados célula a célula com o original; `--modern` inicia a interface de menus. Funcionam com teclado e rato, a janela pode ser maximizada (o ecrã ajusta-se de 640x400 a 1080p e mais), os ficheiros de configuração e de ferramentas editam-se e gravam-se nos diálogos, e o botão Sobre indica as máquinas Bungard CCD e CCD/2 (MTC suportadas, ATC ainda não). Uma versão de consola (`drillpro-console`) desenha os mesmos ecrãs num terminal, com teclado e rato e sem sistema de janelas (SSH, consola de texto, um PC antigo). Os pacotes de cada versão incluem a configuração básica, explicada linha a linha, e uma pasta `examples` desde um furo até vinte mil. O trocador de ferramenta dos modelos ATC (troca automática) funciona a partir de uma gravação do original e não foi experimentado numa máquina real.

---

> **Estado de segurança (safety) e de segurança informática:** este programa não é um sistema de segurança. Com uma ligação real (`connection` no ficheiro da máquina) **Iniciar move a máquina**: retire a ferramenta, mantenha uma mão no interruptor da máquina e tenha pronta a tecla `Esc`, que pára o trabalho de imediato e desliga os drivers dos motores. Os fins de curso são vigiados pela própria unidade de controlo da máquina (pára o eixo no seu fim de curso por si só); este software não tem paragem de emergência própria, e bytes de passo ou velocidades errados podem levar a ferramenta contra a peça ou a mesa ou fazer os motores perderem passos. A camada de protocolo recusa-se a mover o que quer que seja até ser armada. O projeto é independente: não é fabricado, apoiado nem suportado pela Bungard Elektronik, e os nomes de produtos aparecem apenas para dizer com que formatos de ficheiro e máquinas deve funcionar.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.4` |
| Maturidade | Estrutura inicial |
| Testes | 213 |
| Tecnologias | Rust |
| Em hardware real | Sim, numa Bungard CCD MTC: a referência e os movimentos de cada eixo; um trabalho completo ainda não correu. Sem ficheiro da máquina corre no simulador |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura](#-estrutura-do-repositório) · [Relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Software | 🟡 | Um programa Rust com ecrã de texto de 80x25 lê o ficheiro de furação, a tabela de ferramentas e o ficheiro da máquina, ordena os furos e executa o trabalho inteiro num simulador, ou numa máquina real se o ficheiro da máquina o indicar. |
| Testes | 🟢 | Passam 213 testes (`cargo test --workspace`): os formatos, o planeamento do trabalho, o protocolo no simulador e os ecrãs nos oito idiomas. |
| Leitores de ficheiros | 🟡 | Leem-se o ficheiro de furação `.NCD`, a tabela de ferramentas `DRILLPRO.DTD` e o ficheiro da máquina `DRILLPRO.DCF`; só com pequenos ficheiros de exemplo escritos para os testes, não com ficheiros de placas reais. |
| Protocolo da máquina | 🟠 | O protocolo de bytes da unidade de controlo foi reconstruído por engenharia inversa e o `docs/PROTOCOL.md` ainda lista partes que são hipóteses; a camada de protocolo recusa mover o que quer que seja até ser armada. |
| Interface | 🟡 | O ecrã de texto, os menus, o diálogo de ficheiros e a vista vetorial dos furos falam os oito idiomas (testado); o chinês e o japonês precisam de um tipo de letra CJK no sistema. |
| Hardware real | 🟠 | Numa Bungard CCD MTC foram experimentados o movimento de referência dos três eixos e os movimentos de cada eixo; um trabalho de furação completo não correu, os passos por milímetro só foram comparados com a configuração do original, e as saídas do fuso e do aspirador não foram experimentadas. |
| Segurança de utilização | 🟡 | `Esc` pára o trabalho numa fração de segundo e desliga os drivers dos motores, e a própria unidade de controlo da máquina vigia os fins de curso; o software não tem paragem de emergência própria. |
| Ligação ao servidor | 🟠 | Com `connection = server` o programa envia o trabalho ao EH3D-SERVER e acompanha o progresso; este caminho só foi exercitado contra o simulador. |
| Pacotes prontos a usar | 🟡 | A primeira versão publicada traz pacotes para Windows, Linux e macOS (Apple silicon e Intel), cada um com o seu SHA-256, construídos pelo fluxo de publicação. |
| Documentação | 🟢 | README em oito idiomas, as notas do protocolo com o que ainda é hipótese e o guia do ficheiro da máquina com cada valor de afinação. |

## 🎯 Visão geral

* Os **leitores de ficheiros**: ficheiros de furação Excellon `.NCD` (com ou sem o cabeçalho `M48`, coordenadas modais, unidades imperiais de 1 mil), a tabela de ferramentas `DRILLPRO.DTD` e a configuração da máquina `DRILLPRO.DCF`.
* O **planeador do trabalho**: ordena os furos (como no ficheiro, primeiro X, primeiro Y ou caminho mais curto) e mantém-nos agrupados por ferramenta.
* A **interface**: um ecrã em modo de texto de 80x25 com cores CGA, menus, uma caixa de diálogo de ficheiros e uma vista vetorial dos furos, mostrada numa janela. Fala inglês, espanhol, italiano, alemão, francês, português, chinês e japonês: escolha o idioma no menu ou arranque com `--lang`.
* O **protocolo da máquina** (`protocol/`, crate `eh3d-drillpro-protocol`): uma biblioteca sem interface: transporte de bytes (porta série ou simulador), planeamento de movimento em milímetros com rampas de aceleração e uma barreira de segurança que recusa o movimento até ser armada.
* O **simulador**: o trabalho completo corre até ao fim sem hardware; os testes usam-no.
* As **máquinas**: as que têm uma unidade de controlo por porta série que usava o software MS-DOS 4.6: Bungard CCD MTC e CCD/2 MTC (troca de ferramenta manual; a CCD/2 tem a mesa mais pequena de 325 x 270 mm). O tamanho da mesa e as restantes constantes vêm do ficheiro de configuração da máquina. Só foi experimentado numa CCD MTC, e as saídas do eixo-árvore (broca) e da ativação do aspirador ainda não foram experimentadas. Os dois modelos ATC (CCD ATC e CCD/2 ATC, troca automática) terão aplicações separadas, feitas com o que a engenharia inversa permitir. Os modelos XL e as máquinas posteriores têm unidade de controlo e software modernos próprios e não estão abrangidos.
* O **ficheiro da máquina** (`machine.cfg`, partilhado pelo DRILLPRO e pelo ROUTEPRO): como o trabalho chega à máquina (`demo` simulador, `rs232` a porta série do próprio computador, `usb` um adaptador USB-série ou `node` o EH3D-NODE-BUNGARD pela rede) e, um valor por linha, tudo o que foi afinado numa máquina real: velocidades e rampas de procura, o salto de calibração, os limites dos motores.
* A **referência**: Z sobe até ao fim de curso, depois X e Y, e por fim um salto de calibração suave. A unidade de controlo indica «os três eixos em origem» pela linha RI, que o programa vigia; com a máquina estacionada, salta as procuras, afasta X e Y 10 mm dos fins de curso e procura-os de novo, assim vê-los chegar em direto.
* **Movimento estável**: os movimentos saem como um único fluxo contínuo cujas posições de byte levam o tempo (senão um adaptador USB entrega os bytes em rajadas), Z move-se byte a byte, e `Esc` cancela um trabalho numa fração de segundo e desliga os drivers dos motores.
* O **eixo-árvore**: por omissão o da própria unidade de controlo (os seus bits de latch, até 60000 rpm); com `spindle = node` no ficheiro da máquina o trabalho pede-o ao EH3D-NODE-BUNGARD (um variador de frequência nas saídas do nó, qualquer velocidade até ao máximo do nó, parado em qualquer paragem). Com um nó, os programas leem também os LED da máquina, de modo que cada eixo que chega a casa é visto em direto sem ler a linha de home a meio do fluxo.
* A **ligação ao servidor** (`connection = server` no ficheiro da máquina, com `server_address`, `server_token` e `server_machine`): em vez de conduzir ele próprio a máquina, o programa envia o trabalho ao [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER), que o planeia com as constantes da máquina, o executa através do nó da máquina e diz ao programa como vai; o programa mostra o progresso e responde aos diálogos como sempre, `Esc` pára o trabalho no servidor, e fechar o programa não o pára.
* Os **arquivos de trabalho, a janela Offset e a simulação**: um trabalho pode vir empacotado num arquivo ZIP (a janela Select File entra nele e `drillpro JOB.ZIP` carrega o seu primeiro ficheiro de furação); a seta junto ao nome do ficheiro lista os padrões `*.NCD`, `*.ZIP` e `*.*`; os botões Load das janelas de ferramentas e de configuração escolhem o ficheiro a carregar; numa máquina real a janela Offset faz o home, segue cada movimento da janela e volta a fazer o home; e o trabalho simulado demora o que a máquina demoraria.

## 🧩 Papel no EH3D

O EH3D-DRILLPRO é um programa independente da família EH3D para máquinas cuja unidade de controlo precisa que o PC gere cada passo do motor. O EH3D-ROUTEPRO, o seu irmão de fresagem, reutiliza o seu protocolo, o seu ecrã e o suporte de idiomas. Para os trabalhos empacotados em arquivos ZIP usa um crate do EH3D-COMMON (`eh3d-archive`), que tem de estar ao lado para compilar; o leitor Excellon do EH3D-CORE-LIBS poderá substituir o seu mais tarde. Os ficheiros da máquina (o de furação, o HP-GL, as tabelas de ferramentas e a configuração da máquina) vêm do crate `eh3d-bungard-formats` do EH3D-COMMON, que também tem de estar ao lado.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-DRILLPRO: quem o usa, o que contém e de que depende" width="100%">
</p>

Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## 🖼️ Capturas de ecrã

<p align="center">
  <img src="images/screenshots/main.png" alt="main screen" width="80%">
</p>

<p align="center"><em>O ecrã principal do programa clássico, como o desenha o programa próprio da máquina: a escala, o deslocamento e o recorte, os vetores do trabalho e os botões.</em></p>

<p align="center">
  <img src="images/screenshots/tools.png" alt="tool data" width="80%">
</p>

<p align="center"><em>Os dados das ferramentas: o curso extra, o avanço e a velocidade do fuso de cada posição do trocador, e quanto cada ferramenta foi usada.</em></p>

<p align="center">
  <img src="images/screenshots/config.png" alt="parameters" width="80%">
</p>

<p align="center"><em>Os parâmetros da máquina: as coordenadas dos ficheiros, a ordenação dos furos, as velocidades, as rampas, a correção e o idioma.</em></p>

## 🚀 Início rápido

É preciso uma cadeia de ferramentas Rust estável.

```bash
cargo build --release --features gui
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -I tools/ci_validate.py
```

Execute-o com `run.bat` (Windows) ou `./run.sh` (Linux e macOS); ambos compilam primeiro o programa, se for preciso: `run.bat [FICHEIRO.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--dump]`. `--data` é a pasta com `DRILLPRO.DTD` e `DRILLPRO.DCF` (por omissão: `testdata/`) e `--dump` imprime o ecrã como texto simples. Teclas: `Alt` mais a letra realçada abre um menu, as setas movem, `Enter` aceita, `Esc` volta atrás; a barra de menus também se pode clicar. O chinês e o japonês precisam de um tipo de letra CJK entre os do sistema. `--connection FILE` indica outro ficheiro de máquina que não o predefinido (`%APPDATA%\EH3D\machine.cfg` no Windows, `~/.config/eh3d/machine.cfg` nos restantes); sem ficheiro de máquina o programa corre no simulador. `--modern` inicia a interface de menus em vez dos ecrãs clássicos.

## 🧪 Testes

A suite tem 213 testes (`cargo test --workspace`). Verificam os formatos de ficheiro (o ficheiro de furos, a tabela de ferramentas e o ficheiro da máquina, e a sua escrita de volta), o planeamento do trabalho, o protocolo no simulador (bytes, movimento, a referência dos eixos, os bits de latch, a ligação ao nó e a barreira de segurança que se recusa a mover até estar armada), os ecrãs clássicos célula a célula e os ecrãs nos oito idiomas. A CI também executa `cargo fmt --check` e `cargo clippy` com os avisos como erros, e `python -I tools/ci_validate.py` verifica o manifesto, as versões, o registo de alterações e os oito README.

## 🏗️ Compilação e publicação

A versão está em `Cargo.toml` e em `eh3d.project.json`, e cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). As versões são publicadas no GitHub.

**Pacotes já compilados:** cada versão publicada inclui um pacote para Windows (zip), Linux (tar.gz) e macOS (Apple silicon e Intel, tar.gz) com o programa, a licença e este README, cada um com o seu SHA-256 ao lado. São construídos pelo workflow `Release binaries` deste repositório. Descarregue um a partir da página de Releases; no Linux, a janela precisa de X11 ou Wayland.

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-DRILLPRO" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-DRILLPRO/
├── src/
│   ├── classic/                                 os ecrãs, diálogos e teclas clássicos do programa de MS-DOS
│   ├── job.rs runner.rs                         ordenar os furos e executar o trabalho na sua própria thread
│   ├── app.rs screen.rs textapp.rs              o ecrã de texto de 80x25 e os menus modernos
│   ├── i18n.rs texts.rs                         os oito idiomas e os seus textos
│   ├── gui.rs console.rs bin/                   a janela, a versão de terminal e os executáveis
│   └── server_link.rs cli.rs                    a ligação ao EH3D-SERVER e a linha de comandos
├── protocol/src/
│   ├── safety.rs                                a barreira que impede o movimento até estar armada
│   ├── motion.rs positioner.rs                  movimento em milímetros com rampas
│   ├── execute.rs job.rs                        converter um trabalho no fluxo de bytes
│   ├── homing.rs latches.rs                     a referência dos eixos e os bits de latch
│   ├── serial.rs node.rs nullmodem.rs           porta série, nó e simulador
│   └── connection.rs tuning.rs                  o ficheiro da máquina e os seus valores de ajuste
├── tests/                                       formatos, planeamento, ecrãs clássicos e ecrãs em todos os idiomas
├── docs/                                        as notas do protocolo, os ecrãs clássicos, o guia de ligação e o ficheiro da máquina com todos os valores
├── testdata/                                    pequenos ficheiros de exemplo escritos para os testes
├── config/                                      a configuração básica, explicada linha a linha
├── examples/                                    exemplos, de um furo a 20000
├── tools/                                       verificação de CI, do README e ferramentas de imagens
├── images/                                      faixa, as duas imagens e as capturas de ecrã
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── build.sh build.bat                           scripts de compilação e testes
├── run.sh run.bat                               arrancam o programa
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o pacote Rust e as suas dependências fixadas
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* Ainda não correu um trabalho de furação completo numa máquina real: só se experimentaram, numa Bungard CCD MTC, a referência dos três eixos e os movimentos de cada eixo; os passos por milímetro só estão verificados contra a configuração do original e as saídas do fuso e do aspirador não foram experimentadas.
* `docs/PROTOCOL.md` ainda lista partes do protocolo que são hipóteses; a camada de protocolo recusa-se a mover o que quer que seja até ser armada.
* Os leitores de ficheiros só foram experimentados com pequenos ficheiros de exemplo escritos para os testes, não com ficheiros de placas reais.
* A ligação ao servidor (`connection = server`) só foi exercitada contra o simulador.
* Só são suportados os modelos MTC; os modelos ATC ainda não e os modelos XL não estão cobertos. O chinês e o japonês precisam de um tipo de letra CJK entre os do sistema.

**A seguir**

* Executar um trabalho de furação completo na máquina e confirmar os passos por milímetro, o fuso e as saídas do aspirador.
* Experimentar os leitores de ficheiros com ficheiros de placas reais.
* Aplicações à parte para os modelos ATC, construídas com o que a engenharia inversa permitir.
* Substituir o leitor de Excellon próprio pelo do [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS) quando estiver pronto.

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): o irmão de fresagem; usa o protocolo, o ecrã e os idiomas deste projeto
* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): os leitores Excellon e de G-code partilhados da suite
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): o contentor de projeto e os tipos de telemetria de máquina da suite
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD): um nó ESP32-S3 (Ethernet ou Wi-Fi) que fica na caixa da máquina, envia os passos pela sua própria porta série com a temporização que um adaptador USB não consegue manter, lê os LED da máquina e comanda o variador do eixo-árvore; tem o seu próprio painel web
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): o centro da família EH3D: recebe os trabalhos deste programa (`connection = server`) e dos outros clientes, e executa-os na máquina através do seu nó

## 📚 Documentação

* [Notas do protocolo](docs/PROTOCOL.md): o que se sabe sobre como a unidade de controlo é comandada e o que continua a ser uma hipótese.
* [Ligação e afinação](docs/CONNECTION.md): o ficheiro da máquina, as três ligações e cada valor
* [Os ecrãs clássicos](docs/CLASSIC.md): como os ecrãs do programa de MS-DOS são reproduzidos e verificados
* [Registo de alterações](CHANGELOG.md)

## 🤝 Contribuir e comunidade

* Antes de uma pull request execute `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` e `python -I tools/ci_validate.py`: tudo tem de passar. Consulte [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` com `tools/make_architecture.py` e `tools/make_structure.py`.
* Não descreva como a funcionar numa máquina o que só foi experimentado no simulador.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatórios: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
