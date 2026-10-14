<!-- eidos-i18n: source=README.md sha=c4073ef7daa93b14edfa3d1e00794a2cdbdf8066 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**O gerenciador de mods nativo do Linux que nunca toca no seu jogo.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#instalação)

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

### Como ele se compara

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / deployers por links |
|---|---|---|---|---|
| Gerenciador roda nativamente | ✅ | ❌ app Windows no Wine | ✅ (port em Qt) | ✅ |
| Diretório do jogo intacto | ✅ sempre | ✅ | ✅ | ❌ links escritos dentro dele |
| Montagem visível para | só o jogo | só o jogo | **o sistema inteiro** | não se aplica |
| Limpeza após crash | nenhuma, por design | nenhuma | recuperar montagem morta | desimplantação manual |
| Mods root (ENB, preloaders) | ✅ nativo | exige plugin | exige plugin | parcial |
| Privilégios exigidos | nenhum | nenhum | editar `/etc/fuse.conf` | nenhum |

### Quão rápido ele é

| | antes | agora |
|---|---|---|
| carregar um save | ~20 segundos | **6-7 segundos** |
| leituras de diretório em uma sessão | 5,6 milhões | 465 mil |

As trocas de célula são imediatas. O ganho veio de fazer menos perguntas aos
seus mods: encontrar um arquivo antes interrogava os cinquenta, um a um, e
listar uma pasta fazia isso cinquenta vezes. Nenhum dos dois faz mais isso.
Medido numa instância real jogada normalmente, não num banco de testes.

### Trabalhos anteriores e agradecimentos

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) e
  [usvfs](https://github.com/ModOrganizer2/usvfs) - a semântica que o Eidos
  reproduz, e a base de código contra a qual sua paridade foi estudada
- [LOOT](https://loot.github.io/) - o motor de ordenação, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) e os outros gerenciadores para Linux -
  prova de que existe uma comunidade que quer isso resolvido

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

**Instâncias.** Global - administrada centralmente em `~/.local/share/Colony/Eidos/instances/` -
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

**Nexus.** Conecte uma conta para receber links de “Mod Manager Download”, verificar atualizações e consultar autores e seus perfis. As coleções são instaladas a partir de suas receitas, com progresso retomável e relatórios explícitos sobre etapas não suportadas. A comparação verifica os arquivos de origem exatos; metadados antigos incompletos continuam não verificados. A aba Downloads permite filtrar, ordenar, ocultar e remover arquivos. Um botão **offline** interrompe o acesso ao Nexus.

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

**Onde ele guarda os próprios arquivos.** O layout que todo programa da família
Colony usa: `~/.config/Colony/Eidos/` para o que você escolheu - preferências,
sua sessão do Nexus, sua lista de instâncias, as definições de jogos e de
add-ons que você escreveu - `~/.local/share/Colony/Eidos/` para as instâncias
globais, os runtimes baixados e os logs de sessão, e `~/.cache/Colony/Eidos/`
para as masterlists do LOOT. Um Eidos mais antigo guardava isso em
`~/.config/eidos/`, `~/.local/share/eidos/` e `~/.local/state/`; o primeiro
início depois da atualização transfere tudo, diz isso no log, e deixa os
diretórios antigos exatamente como estavam. Enquanto uma transferência não
termina, o Eidos continua usando os arquivos antigos. Detalhes em
[internals/paths.md](../../internals/paths.md) (em inglês).

Consulte [Verificações de integridade e arquivos gerados](../../../docs/internals/integrity.md) (em inglês) para saber sobre a proteção nas substituições, conflitos entre arquivos BSA/BA2, diagnósticos estáticos do SKSE e registros de geração, incluindo suas limitações.

### Jogos suportados

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

## Instalação

### Via Colony (recomendado)

Procure **Eidos** na [Colony](https://github.com/Project-Colony/Colony) e
instale. As atualizações chegam pelo lançador. A Colony instala um único
arquivo, `eidos-linux`: a janela, e também a linha de comando `eidos` quando ele
é chamado por um link chamado `eidos`. A prévia de modelos NIF precisa de um
programa auxiliar que só o pacote abaixo traz; sem ele, a prévia avisa e todo o
resto funciona. O que fazer depois de instalar, e como apontar o Steam para ele:
[docs/guide/install.pt-BR.md](docs/guide/install.md#via-colony).

### Download direto do binário

Baixe um arquivo da [versão mais recente](https://github.com/Project-Colony/Eidos/releases/latest):

| Arquivo | O que é |
|---|---|
| `eidos-linux` | o programa inteiro num único arquivo, como a Colony o instala |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, o programa auxiliar de prévia de NIF, ícones e um instalador que registra a entrada da área de trabalho e o manipulador de downloads do Nexus |

Verifique o download antes, como mostra a
[Política de assinatura de código](#política-de-assinatura-de-código), e depois:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

O instalador põe tudo em `~/.local/bin` sem sudo; `--system` e `--bindir DIR`
escolhem outro lugar. Os dois arquivos exigem Linux x86_64 com glibc 2.39 ou
mais recente, FUSE (`fusermount3`) e 7-Zip para os arquivos de mods.

### Compilar a partir do código

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Exige Rust 1.89 ou mais recente (o `rust-version` que a CI verifica; a CI e as
versões publicadas compilam com 1.99.0), além de `just`, CMake 3.20+ e um
compilador C++17 para o programa auxiliar de prévia de NIF. Um pacote do Arch é
compilado a partir de `packaging/PKGBUILD`. Pré-requisitos e os comandos sem
`just`: [docs/guide/install.pt-BR.md](docs/guide/install.md).

### Opções de inicialização do Steam

O Eidos roda *como* o comando de inicialização do jogo, e é assim que ele
consegue montar antes de o jogo começar. A linha básica é tudo de que a maioria
das configurações precisa, com o caminho absoluto da sua instalação (o Steam
não lê o seu `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Pela Colony, esse caminho é `~/.local/share/Colony/apps/Eidos/eidos-linux`.
Depois aperte Jogar: o Eidos abre na instância daquele jogo, e Run inicia o jogo
através da visão combinada.

Todo o resto são variáveis de ambiente empilhadas na frente dela, e elas se
combinam livremente:

| Você quer... | Ponha na frente |
|---|---|
| DLSS com Community Shaders | `PROTON_ENABLE_NVAPI=1` - sem ela o DLSS silenciosamente nunca inicializa; a lista completa está em [guide/graphics.pt-BR.md](docs/guide/graphics.md) |
| um contador de FPS na tela | `DXVK_HUD=fps` |
| interpolação de quadros no nível do driver, zero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nunca junto com a geração de quadros do próprio Community Shaders |
| logs detalhados para um relatório de bug | `EIDOS_LOG=debug` (os logs de sessão caem em `~/.local/share/Colony/Eidos/logs/`) |
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

## Documentação

Tudo está em [docs/](../../), indexado por quem lê em
[docs/README.pt-BR.md](docs/README.md): o guia do usuário (instalação, a CLI e a
GUI, ferramentas como xEdit e DynDOLOD, gráficos, solução de problemas), como o
Eidos funciona por dentro, e por que ele existe. Política de segurança e como
relatar uma vulnerabilidade: [SECURITY.md](../../../SECURITY.md).

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
`CHANGELOG.md` é gerado. Traduzi-los seriam mais 18.536 palavras para manter
honestas para um público que não precisa delas.

Cada tradução carrega o hash do arquivo em inglês do qual foi feita, e a CI
falha quando o inglês avança - veja
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Uma tradução que não puder ser
posta em dia é **apagada**, não deixada no lugar: uma página velha ainda parece
ter autoridade e distribui os comandos do mês passado, o que é pior para o
leitor do que ser mandado para o inglês.

Um idioma é um único diretório. `docs/i18n/<lang>/` espelha a raiz do
repositório, então `docs/i18n/de/docs/guide/install.md` é o
`docs/guide/install.md` em alemão - o que faz um link entre duas páginas
traduzidas ser IDÊNTICO ao link entre seus originais em inglês, e o que faz
aposentar um idioma ser um único `rm -r`.

## Política de assinatura de código

Todo arquivo de uma versão é assinado pela organização Project Colony na CI,
nunca na máquina de um desenvolvedor. Ao lado de cada arquivo, na página da
versão:

| Arquivo | O que é |
|---|---|
| `<asset>.sig` | uma assinatura ed25519 sobre o arquivo, feita com a chave de versões da organização |
| `<asset>.meta` | três linhas que ligam o arquivo ao seu nome, ao seu sha256 e à versão |
| `<asset>.meta.sig` | uma assinatura ed25519 sobre o `.meta` |

A chave privada é um segredo da organização, usado apenas pelo
[fluxo de trabalho sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
compartilhado, num job que não compila nada; os jobs que compilam o Eidos nunca
a veem. A Colony confere os três arquivos antes de instalar ou atualizar o
Eidos, e recusa uma versão mais antiga que a instalada. Para conferir um
download você mesmo com o OpenSSL 3 (os mesmos comandos servem para todo
arquivo):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

Como as versões são compiladas, assinadas e publicadas:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)
(em inglês).

## Privacidade

O Eidos não envia telemetria, nem estatísticas de uso, nem relatórios de crash.
Ele não se conecta a nenhum servidor além dos desta tabela, e só para a ação
indicada ali. O modo **Offline** (Preferences -> Nexus) corta toda conexão com
o Nexus.

| Dado | Guardado ou enviado | Onde, e por quê |
|---|---|---|
| Preferências, lista de instâncias, definições de jogos e de add-ons | guardado | `~/.config/Colony/Eidos/` |
| Instâncias, runtimes baixados, logs de sessão | guardado | `~/.local/share/Colony/Eidos/`, ou a pasta que você escolheu para uma instância portátil |
| Masterlists do LOOT | guardado | `~/.cache/Colony/Eidos/loot/` |
| Login no Nexus | enviado | Entrar abre `users.nexusmods.com` no seu navegador, que devolve o resultado a um receptor que o Eidos abre em `127.0.0.1` só para esse login. Depois o Eidos troca esse resultado por tokens em `users.nexusmods.com/oauth/token` e busca as chaves públicas com que o Nexus os assina. [Política de privacidade do Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Credenciais do Nexus | guardado | Os tokens de acesso e de renovação, em `~/.config/Colony/Eidos/nexus.ini` com modo 0600. Uma linha `api_key=` escrita ali por um Eidos mais antigo é mantida, mas nunca lida; apague-a se quiser. |
| Requisições ao Nexus | enviado | `api.nexusmods.com`, quando você usa um recurso do Nexus: o nome da sua conta e o status premium, detalhes de mods e arquivos, verificações de atualização, coleções e links de download. Toda requisição leva o seu token de acesso e identifica o Eidos e a versão dele. |
| Downloads do Nexus | enviado | Os arquivos que você pede, do servidor da CDN do Nexus indicado no link de download, salvos na pasta de downloads da instância com um `.meta` no formato do MO2 ao lado de cada um. |
| Links `nxm://` | guardado | `eidos nxm --register`, que o `install.sh` roda para você, escreve `~/.local/share/applications/eidos-nxm.desktop` e o torna o manipulador de `nxm://` via `xdg-mime`, para que o botão Mod Manager Download do site entregue o link ao Eidos. O pacote do Arch instala a mesma entrada para todo o sistema. |
| Masterlists do LOOT | enviado | `raw.githubusercontent.com/loot/...`, na primeira ordenação do LOOT de um jogo e sempre que você pede uma atualização. Nada seu é enviado. [Declaração de privacidade do GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtimes de ferramentas | enviado | Só quando você instala um pré-requisito de ferramenta ou um mod precisa de um: o runtime .NET de `builds.dotnet.microsoft.com`, recusado a menos que bata com o SHA-256 embutido no Eidos, e o `winetricks`, que baixa o que instala no prefixo Proton do jogo (redistribuíveis do Visual C++ e do .NET) da Microsoft e dos outros servidores que o winetricks usa. [Declaração de privacidade da Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Logs de sessão | guardado | `~/.local/share/Colony/Eidos/logs/`, modo 0600: dez sessões por instância, apagadas depois de 30 dias. Sua pasta pessoal é escrita como `~` e seu nome de usuário como `<user>`, então um log pode ser colado numa issue do jeito que está. Eles nunca saem da sua máquina a menos que você os envie. |
| Atalhos | guardado | O `install.sh` escreve `eidos.desktop` e os ícones dele em `~/.local/share`; um atalho de ferramenta que você cria vai para `~/.local/share/applications`. |
| Links que você abre | nenhum dos dois | Páginas de mods e outros links abrem no seu próprio navegador via `xdg-open`. |
| Add-ons | nenhum dos dois | Extensões que você instala rodam como programas próprios na sua máquina; o que elas fazem é com elas. |

## Licença

O código original do Eidos está sob GPL-3.0-or-later; veja
[LICENSE](../../../LICENSE). Estas partes estão sob GPL-3.0 sem a opção "or
later":

- as traduções dos scripts OMOD em `eidos-install`; consulte seus
  [créditos originais](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- o programa auxiliar `eidos-nif-preview`, construído sobre a biblioteca nifly
  incluída no repositório; consulte as
  [licenças e os créditos de seus componentes](../../../crates/eidos-nif-preview/UPSTREAM.md);
- os crates `esplugin` e `libloadorder` aos quais o código de ordem de carga se
  liga, que declaram `GPL-3.0`.

Os outros crates Rust nos binários mantêm as próprias licenças (MIT, Apache-2.0,
BSD e semelhantes). O pacote de versão traz os avisos deles em
`licenses/crates/`, com um índice de cada crate e sua licença.

Os binários também embutem oito DLLs redistribuíveis genuínas do Microsoft
DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` e `d3dx11_43`, de 32 e
64 bits), que o Eidos implanta no prefixo do jogo quando um mod ou uma
ferramenta precisa delas. Elas continuam sob os termos de redistribuição da
Microsoft; veja a [procedência](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md)
delas.

Gerenciar mods pertence a todo mundo.
