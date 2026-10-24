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
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** um programa em Rust com interface de texto de 80x25 que lê ficheiros de furação Excellon `.NCD`, a tabela de ferramentas e a configuração da máquina, ordena os furos, mostra-os e executa todo o trabalho **contra um simulador** (por omissão) ou, quando o ficheiro da máquina o indica, **numa máquina real** pela porta série do computador, um adaptador USB-série ou o EH3D-NODE-BUNGARD. 123 testes correm num computador. Numa Bungard CCD MTC real foram experimentados a referência dos três eixos (Z, X e Y até aos fins de curso, com o sinal «os três em origem» da unidade de controlo confirmado em direto) e os movimentos de cada eixo; **um trabalho de furação completo ainda não correu na máquina**, e os passos por milímetro só foram confrontados com a configuração do original.

---

> **Estado de segurança (safety) e de segurança informática:** este programa não é um sistema de segurança. Com uma ligação real (`connection` no ficheiro da máquina) **Iniciar move a máquina**: retire a ferramenta, mantenha uma mão no interruptor da máquina e tenha pronta a tecla `Esc`, que pára o trabalho de imediato e desliga os drivers dos motores. Os fins de curso são vigiados pela própria unidade de controlo da máquina (pára o eixo no seu fim de curso por si só); este software não tem paragem de emergência própria, e bytes de passo ou velocidades errados podem levar a ferramenta contra a peça ou a mesa ou fazer os motores perderem passos. A camada de protocolo recusa-se a mover o que quer que seja até ser armada. O projeto é independente: não é fabricado, apoiado nem suportado pela Bungard Elektronik, e os nomes de produtos aparecem apenas para dizer com que formatos de ficheiro e máquinas deve funcionar.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.1` |
| Maturidade | Estrutura inicial |
| Testes | 123 |
| Tecnologias | Rust |
| Em hardware real | Sim, numa Bungard CCD MTC: a referência e os movimentos de cada eixo; um trabalho completo ainda não correu. Sem ficheiro da máquina corre no simulador |

## 🎯 Descrição geral

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

## 🧩 Papel no EH3D

O EH3D-DRILLPRO é um programa independente da família EH3D para máquinas cuja unidade de controlo precisa que o PC gere cada passo do motor. O EH3D-ROUTEPRO, o seu irmão de fresagem, reutiliza o seu protocolo, o seu ecrã e o suporte de idiomas. Ainda não depende de nenhum outro repositório EH3D; o leitor Excellon do EH3D-CORE-LIBS poderá substituir o seu mais tarde.

## 📂 Estrutura do repositório

```text
EH3D-DRILLPRO/
├── src/          library: file formats, job planning, screen, application, languages
├── src/bin/      the drillpro executable (feature gui)
├── protocol/     eh3d-drillpro-protocol: serial transport, motion, safety gate
├── tests/        formats, job planning, screens in all languages
├── testdata/     small sample files written for the tests
├── docs/         PROTOCOL.md e CONNECTION.md: o protocolo, o ficheiro da máquina e cada valor de afinação; machine.cfg.example: o ficheiro com todos os seus valores
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ COMPILAR E EXECUTAR

É preciso uma cadeia de ferramentas Rust estável.

```bash
cargo build --release --features gui
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -I tools/ci_validate.py
```

Execute-o com `run.bat` (Windows) ou `./run.sh` (Linux e macOS); ambos compilam primeiro o programa, se for preciso: `run.bat [FICHEIRO.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--dump]`. `--data` é a pasta com `DRILLPRO.DTD` e `DRILLPRO.DCF` (por omissão: `testdata/`) e `--dump` imprime o ecrã como texto simples. Teclas: `Alt` mais a letra realçada abre um menu, as setas movem, `Enter` aceita, `Esc` volta atrás; a barra de menus também se pode clicar. O chinês e o japonês precisam de um tipo de letra CJK entre os do sistema. `--connection FILE` indica outro ficheiro de máquina que não o predefinido (`%APPDATA%\EH3D\machine.cfg` no Windows, `~/.config/eh3d/machine.cfg` nos restantes); sem ficheiro de máquina o programa corre no simulador.

## 🔗 Projetos relacionados

* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): o irmão de fresagem; usa o protocolo, o ecrã e os idiomas deste projeto
* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): os leitores Excellon e de G-code partilhados da suite
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): o contentor de projeto e os tipos de telemetria de máquina da suite
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD): um nó ESP32-S3 (Ethernet ou Wi-Fi) que fica na caixa da máquina, envia os passos pela sua própria porta série com a temporização que um adaptador USB não consegue manter, lê os LED da máquina e comanda o variador do eixo-árvore; tem o seu próprio painel web
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): o centro da família EH3D: recebe os trabalhos deste programa (`connection = server`) e dos outros clientes, e executa-os na máquina através do seu nó

## 📚 Documentação e comunidade

* [Notas do protocolo](docs/PROTOCOL.md): o que se sabe sobre como a unidade de controlo é comandada e o que continua a ser uma hipótese.
* [Ligação e afinação](docs/CONNECTION.md): o ficheiro da máquina, as três ligações e cada valor
* [Registo de alterações](CHANGELOG.md)
* [Como contribuir](CONTRIBUTING.md)
* [Política de segurança](SECURITY.md)
* [Suporte](SUPPORT.md)

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.