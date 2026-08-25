<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**O gerenciador de mods nativo do Linux que nunca toca no seu jogo.**

</div>

O Eidos dá aos jogos da Bethesda no Linux o que o Mod Organizer 2 lhes dá no
Windows - uma visão combinada dos seus mods, virtual e refeita a cada início -
construída sobre primitivas do Linux em vez de hooking da API do Windows. Sem
Wine para o gerenciador. Nenhum arquivo copiado para o diretório do jogo. Nenhum
procedimento de limpeza, porque não há nada para limpar.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Status:** O Skyrim SE é jogado através do Eidos todo dia - SKSE, preloaders
> de script extender, Creation Club, ordens de carga ordenadas pelo LOOT, saves
> por perfil, tudo. Uma família de jogos comprovada em jogo real até agora;
> outras dez estão ligadas e esperando testadores.

## Por que o Eidos

- 🔒 **Uma montagem que só o seu jogo enxerga.** A visão combinada vive num
  espaço de nomes de montagem privado: seu gerenciador de arquivos, sua rotina
  de backup, um segundo jogo - nenhum deles a vê, nenhum deles precisa de
  permissão para ela. Mate o jogo, tire da tomada: o espaço de nomes morre junto
  com a árvore de processos e sua instalação fica exatamente como estava. Não há
  resíduo *por construção*.
- 🧾 **Uma única cópia da verdade.** Seu perfil é dono da própria lista de mods,
  ordem dos plugins, INIs e saves. Os arquivos de plugin e o diretório de saves
  são bind-montados sobre os caminhos do próprio jogo no início, então até as
  escritas do próprio jogo caem no seu perfil. Trocar de perfil troca tudo.
- 🐧 **Totalmente sem root.** Sem auxiliar setuid, sem daemon, sem `sudo setcap`,
  sem editar `/etc/fuse.conf`. Um binário, uma opção de inicialização do Steam.
- 🛡️ **Salvaguardas com comprovante.** Um crash que destrua sua lista de plugins
  é sinalizado contra um instantâneo anterior à sessão, com restauração em um
  clique. Uma captura que apagaria sua ordem de carga é recusada e diz por quê.

## O que ele faz

**Mods.** Arquivos Simple, assistentes FOMOD, pacotes BAIN do Wrye Bash, um
seletor manual para o resto - e **mods root nativamente** (preloaders de script
extender, ENB, Engine Fixes), sem plugin Root Builder e sem nada copiado para a
sua instalação. Esconda arquivos avulsos, agrupe com separadores, mova para
posições específicas, notas e categorias por mod, e um importador de perfis do
MO2.

A lista é a do MO2, com os hábitos dela: oito colunas opcionais e ordenação por
qualquer uma delas, agrupamento por categoria ou por origem, gestos de duplo
clique, digitar para saltar, backups por mod que ficam inertes até você
restaurá-los, e sinalizações informativas para um mod cujo layout este jogo não
vai carregar ou que foi baixado para outro. A árvore de arquivos dele faz as
operações comuns - nova pasta, renomear, apagar, abrir - e pré-visualiza imagens
e texto sem abrir nada.

**Plugins.** A ordem de carga com a ordenação do LOOT embutida, índices de mod
como o jogo os calcula, avisos de master faltando, e seu conteúdo de DLC e
Creation Club mostrado como as linhas não gerenciadas que são.

**Instâncias.** Global - administrada centralmente em `~/.local/share/eidos` -
ou portátil: uma pasta autossuficiente onde você quiser (um segundo disco, uma
partição de jogos), móvel e isolada, como as do MO2. As instâncias portáteis
ficam lembradas entre sessões; a GUI, a inicialização pelo Steam e todo comando
da CLI seguem a que você usou por último, e qualquer comando aceita a pasta onde
aceita um identificador de jogo. Detalhes em
[usage.pt-BR.md](docs/guide/usage.md#instâncias-global-e-portátil).

**Perfis.** Ordem dos mods, estado dos plugins, INIs e saves por perfil. Os
saves são lidos, comparados com seus plugins atuais - com um botão que ativa o
que um save precisa - e sincronizados de volta para o Steam Cloud depois de cada
sessão.

**Nexus.** Conecte uma conta e o botão "Mod Manager Download" do site cai direto
na sua instância, com verificações de atualização contra o que você tem
instalado, quem fez cada mod e um link para o perfil dele. O link de uma
**coleção** lista os membros dela cruzados com a sua instância - instalados,
baixados, faltando - o que é ler uma coleção em vez de instalar uma, e o painel
diz isso. A aba Downloads é uma biblioteca de arquivos: filtre, ordene, esconda
sem apagar, e purgue os que já estão instalados. Uma chave **offline** interrompe tudo
isso.

**Ferramentas.** xEdit, BodySlide, DynDOLOD e afins rodam *através da visão
combinada* dentro do prefixo Proton do jogo - eles enxergam seus mods, a saída
deles cai no Overwrite, e um clique transforma isso num mod de verdade. O
runtime de que cada um precisa é baixado sob demanda, então uma DLL faltando é
um botão em vez de uma tarde perdida. O xEdit e seu gêmeo QuickAutoClean são
encontrados para você - na pasta do jogo, dentro de um mod, ou no diretório de
ferramentas que você mantém ao lado dos seus jogos - com os runtimes certos já
escolhidos. Fixe os que você usa, esconda os que não usa, dê a uma ferramenta o
próprio Steam AppID quando ela for um app do Steam por si só, e escreva um atalho
`.desktop` que a inicie através da visão combinada sem sequer abrir o Eidos.

**Diagnósticos.** Masters faltando, arquivos órfãos, desvio da lista de mods,
conjuntos de plugins danificados - e, depois de uma sessão, o que o log do
próprio script extender diz que realmente carregou.

**Onde ele guarda os próprios arquivos.** `~/.config/Colony/Eidos/` para o que
você escolheu - preferências, sua sessão do Nexus, sua lista de instâncias, as
definições de jogos e de add-ons que você escreveu - com os logs em
`~/.local/state/Colony/Eidos/`. O layout que todo programa da família Colony
usa. Um Eidos mais antigo guardava isso em `~/.config/eidos/`; o primeiro início
depois da atualização copia tudo para o lugar novo, diz isso no log, e deixa o
diretório antigo exatamente como estava.

## Como ele se compara

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / deployers por links |
|---|---|---|---|---|
| Gerenciador roda nativamente | ✅ | ❌ app Windows no Wine | ✅ (port em Qt) | ✅ |
| Diretório do jogo intacto | ✅ sempre | ✅ | ✅ | ❌ links escritos dentro dele |
| Montagem visível para | só o jogo | só o jogo | **o sistema inteiro** | não se aplica |
| Limpeza após crash | nenhuma, por design | nenhuma | recuperar montagem morta | desimplantação manual |
| Mods root (ENB, preloaders) | ✅ nativo | exige plugin | exige plugin | parcial |
| Privilégios exigidos | nenhum | nenhum | editar `/etc/fuse.conf` | nenhum |

## Quão rápido ele é

| | antes | agora |
|---|---|---|
| carregar um save | ~20 segundos | **6-7 segundos** |
| leituras de diretório em uma sessão | 5,6 milhões | 465 mil |

As trocas de célula são imediatas. O ganho veio de fazer menos perguntas aos
seus mods: encontrar um arquivo antes interrogava os cinquenta, um a um, e
listar uma pasta fazia isso cinquenta vezes. Nenhum dos dois faz mais isso.
Medido numa instância real jogada normalmente, não num banco de testes.

## Começar

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Depois ponha a opção de inicialização do Steam do seu jogo em
`~/.local/bin/eidos-gui %command%` e aperte Jogar.

Pacotes do Arch e tarballs de versão, o que você precisa ter instalado antes, e
o caminho pela CLI: **[docs/guide/install.pt-BR.md](docs/guide/install.md)**.

## Opções de inicialização do Steam

A linha básica é tudo de que a maioria das configurações precisa:

```
~/.local/bin/eidos-gui %command%
```

Todo o resto são variáveis de ambiente empilhadas na frente dela, e elas se
combinam livremente:

| Você quer... | Ponha na frente |
|---|---|
| DLSS com Community Shaders | `PROTON_ENABLE_NVAPI=1` - sem ela o DLSS silenciosamente nunca inicializa; a lista completa está em [guide/graphics.pt-BR.md](docs/guide/graphics.md) |
| um contador de FPS na tela | `DXVK_HUD=fps` |
| interpolação de quadros no nível do driver, zero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nunca junto com a geração de quadros do próprio Community Shaders |
| logs detalhados para um relatório de bug | `EIDOS_LOG=debug` (os logs de sessão caem em `~/.local/state/Colony/Eidos/logs/`) |
| um relatório de E/S por sessão vindo da montagem | `EIDOS_FUSE_STATS=1` |
| outro número de workers do FUSE | `EIDOS_FUSE_THREADS=8` (4 por padrão; `1` é a primeira coisa a tentar quando se caça um bug de concorrência) |
| esta inicialização presa a uma instância portátil | `EIDOS_INSTANCE=/path/to/folder` - sem ela o Eidos abre a instância que você usou por último, o que geralmente é o que você quer |

A linha para guardar numa instalação moddada moderna (Community Shaders, DLSS,
geração de quadros) - este é o comando final, não um exemplo:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Acrescente `DXVK_HUD=fps` na frente enquanto confere que tudo funciona, e tire
depois que funcionar.

Os interruptores de diagnóstico mais profundos (`EIDOS_FUSE_TRACE`, as chaves de
bissecção do cache e do índice, por que `EIDOS_FUSE_PASSTHROUGH` vem desligado
por padrão) vivem em
[guide/troubleshooting.pt-BR.md](docs/guide/troubleshooting.md).

## Para onde ir depois

| Se você quiser... | |
|---|---|
| instalá-lo | [guide/install.pt-BR.md](docs/guide/install.md) |
| aprender a CLI e a GUI | [guide/usage.pt-BR.md](docs/guide/usage.md) |
| configurar xEdit, BodySlide ou DynDOLOD | [guide/tools.pt-BR.md](docs/guide/tools.md) |
| jogar Fallout 4 (F4SE, versões, o crash dos detritos da NVIDIA) | [guide/fallout4.pt-BR.md](docs/guide/fallout4.md) |
| fazer o DLSS / a geração de quadros funcionarem (Community Shaders) | [guide/graphics.pt-BR.md](docs/guide/graphics.md) |
| consertar algo que parece errado | [guide/troubleshooting.pt-BR.md](docs/guide/troubleshooting.md) |
| saber por que ele é rápido, e conferir você mesmo | [internals/performance.md](../../internals/performance.md) |
| entender como ele funciona por dentro | [internals/architecture.md](../../internals/architecture.md) |
| compilá-lo, testá-lo, contribuir | [internals/contributing.md](../../internals/contributing.md) |
| saber por que ele existe | [project/landscape.md](../../project/landscape.md) |

Um idioma é um único diretório: `docs/i18n/pt-BR/` espelha a raiz do repositório,
o que faz um link entre duas páginas traduzidas ser idêntico ao link entre seus
originais em inglês.

## Idioma

As páginas de que um jogador precisa são traduzidas. **O inglês é canônico**:
quando uma tradução discorda dele, quem está certo é o arquivo em inglês.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

**Todo o resto está em inglês de propósito, não por omissão.** `docs/internals/`
e `docs/project/` são lidos por gente que também está lendo o Rust, e o
`CHANGELOG.md` é gerado. Traduzi-los seriam mais 17.678 palavras para manter
honestas para um público que não precisa delas.

Cada tradução carrega o hash do arquivo em inglês do qual foi feita, e a CI
falha quando o inglês avança - veja
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Uma tradução que não puder ser
posta em dia é **apagada**, não deixada no lugar: uma página velha ainda parece
ter autoridade e distribui os comandos do mês passado, o que é pior para o
leitor do que ser mandado para o inglês.

Acrescentar um idioma são quatro arquivos e uma linha nesta tabela;
[`docs/internals/contributing.md`](../../internals/contributing.md) tem os passos.

## Jogos suportados

**Skyrim SE/AE** - comprovado em jogo real. O **Fallout 4** também está ligado
de ponta a ponta (F4SE trocado automaticamente, invalidação de arquivos, ordem
de carga com asterisco, LOOT, saves `.fos`) - veja
[guide/fallout4.pt-BR.md](docs/guide/fallout4.md). Ligados pelo descritor
de jogos compartilhado e à procura de testadores: Skyrim LE, Skyrim VR, Enderal
SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion e Morrowind (os
dois últimos montam e administram mods; suas listas de plugins ordenadas por
data ainda não são administradas).

Acrescentar uma família é uma linha de descritor:
[internals/adding-games.md](../../internals/adding-games.md).

## Trabalhos anteriores e agradecimentos

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) e
  [usvfs](https://github.com/ModOrganizer2/usvfs) - a semântica que o Eidos
  reproduz, e a base de código contra a qual sua paridade foi estudada
- [LOOT](https://loot.github.io/) - o motor de ordenação, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) e os outros gerenciadores para Linux -
  prova de que existe uma comunidade que quer isso resolvido

## Licença

GPL-3.0-or-later. Gerenciar mods pertence a todo mundo.
