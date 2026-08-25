<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
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
[usage.pt-BR.md](docs/guide/usage.pt-BR.md#instâncias-global-e-portátil).

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
o caminho pela CLI: **[docs/guide/install.pt-BR.md](docs/guide/install.pt-BR.md)**.

## Opções de inicialização do Steam

A linha básica é tudo de que a maioria das configurações precisa:

```
~/.local/bin/eidos-gui %command%
```

Todo o resto são variáveis de ambiente empilhadas na frente dela, e elas se
combinam livremente:

| Você quer... | Ponha na frente |
|---|---|
| DLSS com Community Shaders | `PROTON_ENABLE_NVAPI=1` - sem ela o DLSS silenciosamente nunca inicializa; a lista completa está em [guide/graphics.pt-BR.md](docs/guide/graphics.pt-BR.md) |
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
[guide/troubleshooting.pt-BR.md](docs/guide/troubleshooting.pt-BR.md).

## Para onde ir depois

| Se você quiser... | |
|---|---|
| instalá-lo | [guide/install.pt-BR.md](docs/guide/install.pt-BR.md) |
| aprender a CLI e a GUI | [guide/usage.pt-BR.md](docs/guide/usage.pt-BR.md) |
| configurar xEdit, BodySlide ou DynDOLOD | [guide/tools.pt-BR.md](docs/guide/tools.pt-BR.md) |
| jogar Fallout 4 (F4SE, versões, o crash dos detritos da NVIDIA) | [guide/fallout4.pt-BR.md](docs/guide/fallout4.pt-BR.md) |
| fazer o DLSS / a geração de quadros funcionarem (Community Shaders) | [guide/graphics.pt-BR.md](docs/guide/graphics.pt-BR.md) |
| consertar algo que parece errado | [guide/troubleshooting.pt-BR.md](docs/guide/troubleshooting.pt-BR.md) |
| saber por que ele é rápido, e conferir você mesmo | [internals/performance.md](docs/internals/performance.md) |
| entender como ele funciona por dentro | [internals/architecture.md](docs/internals/architecture.md) |
| compilá-lo, testá-lo, contribuir | [internals/contributing.md](docs/internals/contributing.md) |
| saber por que ele existe | [project/landscape.md](docs/project/landscape.md) |

O índice inteiro está em [docs/README.pt-BR.md](docs/README.pt-BR.md); a
política de segurança e como relatar uma vulnerabilidade em
[SECURITY.md](SECURITY.md).

## Idioma

As páginas de que um jogador precisa são traduzidas. **O inglês é canônico**:
quando uma tradução discorda dele, quem está certo é o arquivo em inglês.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**Todo o resto está em inglês de propósito, não por omissão.** `docs/internals/`
e `docs/project/` são lidos por gente que também está lendo o Rust, e o
`CHANGELOG.md` é gerado. Traduzi-los seriam mais 17.678 palavras para manter
honestas para um público que não precisa delas.

Cada tradução carrega o hash do arquivo em inglês do qual foi feita, e a CI
falha quando o inglês avança - veja
[`scripts/i18n-check.sh`](scripts/i18n-check.sh). Uma tradução que não puder ser
posta em dia é **apagada**, não deixada no lugar: uma página velha ainda parece
ter autoridade e distribui os comandos do mês passado, o que é pior para o
leitor do que ser mandado para o inglês.

Acrescentar um idioma são quatro arquivos e uma linha nesta tabela;
[`docs/internals/contributing.md`](docs/internals/contributing.md) tem os passos.

## Jogos suportados

**Skyrim SE/AE** - comprovado em jogo real. O **Fallout 4** também está ligado
de ponta a ponta (F4SE trocado automaticamente, invalidação de arquivos, ordem
de carga com asterisco, LOOT, saves `.fos`) - veja
[guide/fallout4.pt-BR.md](docs/guide/fallout4.pt-BR.md). Ligados pelo descritor
de jogos compartilhado e à procura de testadores: Skyrim LE, Skyrim VR, Enderal
SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion e Morrowind (os
dois últimos montam e administram mods; suas listas de plugins ordenadas por
data ainda não são administradas).

Acrescentar uma família é uma linha de descritor:
[internals/adding-games.md](docs/internals/adding-games.md).

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
