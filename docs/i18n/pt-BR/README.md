<!-- eidos-i18n: source=README.md sha=37e97bafbd2cf43f2101a28b23c1464420a5f9ec -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Um gerenciador de mods nativo do Linux que nunca toca na pasta do seu jogo.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#instalação)

O Eidos dá aos jogos da Bethesda no Linux o que o Mod Organizer 2 lhes dá no
Windows: uma visão combinada dos seus mods, montada a cada início. Ele usa
espaços de nomes de montagem do Linux e FUSE em vez de hooking da API do Windows,
e o gerenciador roda sem Wine. Nada é copiado para a pasta do jogo, então não há
nada para limpar depois. O Eidos também tem definições para Stellar Blade,
Stardew Valley, 7 Days to Die e Crimson Desert.

Cópias do Steam executam o Eidos pela opção de inicialização do jogo. Cópias GOG
e Epic no Heroic ou no Legendary o executam como wrapper. O jogo sempre inicia
pela loja que é dona da cópia; o Eidos nunca o inicia diretamente pelo Proton.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Status:** O Skyrim SE é jogado através do Eidos todo dia, com SKSE,
> preloaders de script extender, Creation Club, ordens de carga ordenadas pelo
> LOOT e saves por perfil. Uma família de jogos comprovada em jogo real até
> agora. Outros dez jogos da Bethesda e quatro de outras empresas estão ligados e
> esperando testadores.

## Por que o Eidos

A visão combinada é montada num espaço de nomes de montagem privado que só a
árvore de processos do jogo enxerga. Outros programas, um segundo jogo inclusive,
nunca a veem e não precisam de permissão para ela. Quando o jogo sai ou é morto,
ou a máquina perde energia, o espaço de nomes some junto com os processos e a
instalação fica como estava.

Cada perfil é dono da própria lista de mods, ordem de plugins, INIs e saves. No
início, os arquivos de plugins e a pasta de saves são montados por bind sobre os
caminhos do próprio jogo, então o que o jogo escreve ali cai no perfil. Trocar de
perfil troca tudo isso.

O Eidos não precisa de auxiliar setuid, de daemon do sistema, de `sudo setcap`
nem de edição de `/etc/fuse.conf`. O daemon FUSE roda como o seu usuário durante
um início e sai com o jogo. Você instala um programa e define uma opção de
inicialização do Steam.

Antes de cada execução o Eidos guarda um instantâneo da lista de plugins. Se uma
falha danificar a lista, o Diagnostics a compara com esse instantâneo, e Restore
the pre-session order devolve a ordem antiga. Uma captura que apagaria a sua
ordem de carga é recusada, com o motivo.

Test run inicia o jogo com os mods do perfil, sem mods, ou só com os mods
selecionados e seus masters. Os saves, as mudanças de INI, o estado dos plugins, a
saída do Overwrite e os outros arquivos de My Games dessa execução, logs
inclusive, vão para `<instance>/test-run/`. Quando o jogo sai, o Eidos lista esses
cinco grupos e você guarda ou joga fora cada um. As outras escritas, como o
registro do Wine, vão para onde sempre vão. Find the mod behind a problem, no
Diagnostics, executa execuções de teste com metade dos mods suspeitos por vez mais
os masters de que eles precisam. Depois de cada execução você responde se o
problema aconteceu, e o Eidos reduz os suspeitos até nomear o mod, ou os mods que
precisam dos masters uns dos outros; 200 suspeitos levam cerca de 8 execuções. Os
mods selecionados na lista antes de você começar continuam ativados em todos os
passos. Detalhes em [usage.md](docs/guide/usage.md#execuções-de-teste) e
[usage.md](docs/guide/usage.md#encontrar-o-mod-por-trás-de-um-problema).

### Como ele se compara

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / implantadores de links |
|---|---|---|---|---|
| Gerenciador roda nativamente | sim | não, um app do Windows no Wine | sim (port em Qt) | sim |
| Pasta do jogo intocada | sim, sempre | sim | sim | não, links são escritos nela |
| Montagem visível para | só o jogo | só o jogo | o sistema inteiro | n/a |
| Limpeza após falha necessária | nenhuma | nenhuma | recuperação de montagem velha | remoção manual da implantação |
| Mods root (ENB, preloaders) | sim, nativamente | plugin necessário | plugin necessário | parcial |
| Privilégios necessários | nenhum | nenhum | edição de `/etc/fuse.conf` | nenhum |

### Quão rápido ele é

| | antes | agora |
|---|---|---|
| carregar um save | ~20 segundos | 6-7 segundos |
| varreduras de diretório numa sessão | 5,6 milhões | 335 mil |

As trocas de célula são imediatas. Achar um arquivo antes perguntava a cada um de
cinquenta mods em sequência, e listar uma pasta fazia isso cinquenta vezes. As
camadas somente leitura agora são indexadas uma vez na montagem, então achar um
arquivo é uma busca por hash, e a listagem combinada de uma pasta também é
calculada uma vez na montagem. Os números vêm de uma instância real jogada
normalmente, não de um benchmark:
[internals/performance.md](../../internals/performance.md).

### Trabalhos anteriores e agradecimentos

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) e
  [usvfs](https://github.com/ModOrganizer2/usvfs): a semântica que o Eidos
  reproduz, e a base de código contra a qual a paridade dele foi estudada
- [LOOT](https://loot.github.io/): o motor de ordenação, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) e os outros gerenciadores para Linux,
  que mostram que existe uma comunidade que quer isso resolvido

## O que ele faz

### Mods

O Eidos instala arquivos simples, assistentes FOMOD, pacotes BAIN do Wrye Bash,
contêineres OMOD com scripts OBMM suportados, e qualquer outra coisa por um
seletor manual. Mods root (preloaders de script extender, ENB, Engine Fixes)
funcionam nativamente, sem plugin Root Builder e sem nada copiado para a sua
instalação. Você pode ocultar arquivos avulsos, agrupar mods com separadores,
mover uma seleção para um alvo, guardar notas e categorias por mod e importar um
perfil do MO2.

A lista funciona como a do MO2: oito colunas opcionais e ordenação por qualquer
uma, agrupamento por categoria ou por origem, gestos de clique duplo, salto por
digitação, backups por mod que não fazem nada até você restaurar, e avisos para um
mod cujo layout este jogo não vai carregar ou que foi baixado para outro jogo. Os
filtros escolhem mods por estado no Nexus (endossado, acompanhado, atualização
disponível), notas, arquivos ocultos e tipo de conteúdo (plugins, arquivos,
texturas, malhas, scripts, som e mais). A árvore de arquivos cria pastas, renomeia,
apaga e abre arquivos, e mostra prévias de imagens e texto sem iniciar nada.

Ctrl+Z e Ctrl+Shift+Z desfazem e refazem mudanças na lista de mods e na ordem de
carga. Desativar ou remover um mod que um dos três saves mais novos do perfil usa
pergunta antes e nomeia os plugins (saves de Skyrim e Fallout 4). Reinstalar ou
atualizar um mod FOMOD abre o assistente com as respostas da última instalação. As
informações de um mod listam os mods de que ele precisa e os mods que precisam
dele, lidos dos masters dos plugins ativados.

### Plugins

A ordem de carga tem a ordenação do LOOT embutida, índices de mods como o jogo os
calcula, avisos de masters faltando, e o seu conteúdo de DLC e Creation Club
mostrado como linhas não gerenciadas. O cabeçalho conta as vagas usadas em cada
espaço de índice (full, medium, light). Passar o mouse sobre um plugin mostra o
autor, a descrição e a versão do cabeçalho. Nos motores do Skyrim, um plugin cujo
INI de mesmo nome o jogo carrega é marcado.

Plugins que o LOOT marca como sujos são limpos pelo QuickAutoClean do xEdit num mod
`Cleaned plugins`, nunca no lugar. Flag as light, num mod novo, grava uma cópia
light de um plugin, e recusa quando um dos saves mais novos carrega esse plugin
como plugin cheio (saves de Skyrim e Fallout 4).

### Instâncias

Uma instância é global, guardada em `~/.local/share/Colony/Eidos/instances/`, ou
portátil: uma pasta autocontida onde você quiser (um segundo disco, uma partição
de jogos), movível e isolada como as do MO2. As instâncias portáteis são lembradas
entre sessões. A interface gráfica, o início pelo Steam e todo comando da CLI
seguem a que você usou por último, e qualquer comando aceita a pasta onde aceita um
identificador de jogo. Detalhes em
[usage.md](docs/guide/usage.md#instâncias-global-e-portátil).

Uma instância também pode executar o jogo de uma cópia da pasta do jogo dentro da
instância. Quando o Eidos abre uma lista do Wabbajack pela primeira vez, ele
executa a cópia própria da lista: a pasta que o `ModOrganizer.ini` dela nomeia, ou
senão uma pasta `Stock Game` ou `Game Root`. Freeze this version, no cartão Game
files do Diagnostics, copia a instalação da loja para `<instance>/Stock Game`,
então uma atualização da loja não muda mais os arquivos que o jogo executa. Quando
as duas pastas estão num sistema de arquivos que pode compartilhar blocos, como
btrfs ou XFS, a cópia os compartilha com a instalação da loja (reflink). Em ext4,
ou entre dois sistemas de arquivos, é uma cópia completa, e o tamanho e o espaço
livre são mostrados antes. A loja ainda inicia o jogo. Return to the store version
e Delete the copy desfazem, e
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
faz o mesmo num terminal. Morrowind, e Oblivion com `bUseMyGamesDirectory=0`, não
podem rodar de uma cópia. Detalhes em
[usage.md](docs/guide/usage.md#arquivos-do-jogo-a-instalação-da-loja-ou-uma-cópia).

### Perfis

Cada perfil tem a própria ordem de mods, estado dos plugins, INIs e saves. Os saves
são lidos e comparados com os seus plugins atuais, com um botão que ativa o que um
save precisa, e copiados de volta para o prefixo para o Steam Cloud depois de toda
sessão. Saves que o Steam Cloud trouxe de outro dispositivo, ou que um início sem o
Eidos deixou no prefixo, são listados na aba Saves. Import os copia para o perfil, e
um save cujo nome o perfil já usa entra sob um novo nome `orphan-` em vez de
substituí-lo.

Um perfil pode carregar variáveis de inicialização e wrappers (`env=NOME=valor` e
`wrapper=comando`, um por linha, no editor de INI) para as execuções do jogo e das
ferramentas dele. Para Skyrim SE, Enderal SE e Fallout 4, um cartão Display no
Diagnostics define o tamanho e o modo da janela nas cópias de INI do perfil. Backups
guarda pontos de restauração dos INIs do perfil, e o Eidos faz um antes de cada
execução do BethINI Pie. O atalho de desktop de um perfil inicia o jogo pela loja
dele. `eidos sessions` lista as execuções registradas e compara uma com o estado
atual.

### Nexus

Conecte uma conta para receber links Mod Manager Download, verificar atualizações e
ver autores e perfis deles. Cada mod é verificado, endossado e linkado no domínio do
Nexus de onde veio o arquivo dele, então um mod de Skyrim SE numa instância de
Enderal SE fica no site do Skyrim SE. Uma atualização segue a cadeia de sucessores
que o autor definiu no Nexus, substitui a pasta do mod pelo arquivo novo (um backup
da antiga é oferecido), mantém a prioridade e o estado ativado dele e repete as
respostas do FOMOD guardadas na instalação. `eidos nexus update <instance> --apply`
faz o mesmo num terminal numa conta premium, e imprime as páginas dos arquivos numa
conta gratuita. Track e Endorse estão no menu do mod, e toda verificação de
atualização os sincroniza com a sua conta do Nexus. As informações de um mod têm uma
aba Nexus page com a descrição e o changelog como texto simples, e a aba General
termina com o que a última atualização mudou: os arquivos adicionados, removidos e
redimensionados, e os masters adicionados ou removidos. Install downloads when they
finish (Preferences, Nexus, desligado por padrão) instala um download assim que ele
termina, com a janela aberta.

As coleções são instaladas a partir das receitas, com progresso retomável e
relatórios explícitos para passos não suportados. A correspondência confere os
arquivos de origem exatos; metadados antigos incompletos continuam não verificados.
O painel de coleções navega e busca as coleções publicadas para um jogo, mostra
quando uma coleção instalada tem uma revisão mais nova e conduz uma conta gratuita
pelos arquivos que faltam, uma página por vez. Quando um arrasto quebra as regras de
ordem de uma coleção, o Diagnostics avisa, e Restore collection order devolve os
mods dela. A aba Downloads filtra, ordena, oculta e remove arquivos. O modo Offline
(Preferences, Nexus) interrompe o acesso ao Nexus e a verificação diária de versões.

### Ferramentas

xEdit, BodySlide, DynDOLOD e outras ferramentas de modding rodam pela visão combinada
dentro do prefixo Wine do jogo (o prefixo do Proton do Steam, ou o que o Heroic
preparou). Elas veem os seus mods, a saída delas cai no Overwrite, e um clique
transforma essa saída num mod de verdade. Para o Skyrim SE, o Eidos acha SSEEdit,
SSEEdit QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS, BodySlide, DynDOLOD,
TexGen, xLODGen, Synthesis e BethINI Pie na pasta do jogo, dentro de um mod ou na
pasta de ferramentas que você mantém ao lado dos seus jogos, cada um com os runtimes
de que precisa. Os outros jogos têm a própria lista. Pandora, Nemesis, FNIS,
BodySlide e Synthesis escrevem a saída num mod nomeado (`Nemesis Output` e assim por
diante).

O pacote de versão traz os instaladores do Visual C++ 2015-2022 e do .NET 8 Desktop
Runtime da Microsoft e o runtime .NET 10. O Eidos instala `vcrun2022` e o .NET 8 com
esses instaladores, sem winetricks, e sem o pacote ele baixa os mesmos arquivos da
Microsoft, cada um conferido contra o SHA-256 embutido no Eidos. O Synthesis recebe o
SDK do .NET 10, um download de 300 MB que o pacote não traz. Os outros pré-requisitos
(.NET Framework 4.8, XACT e versões mais antigas do .NET) ainda passam pelo
winetricks. Nada é baixado até você pedir para instalar os pré-requisitos de uma
ferramenta.

Fixe as ferramentas que você usa, oculte as que não usa, dê a uma ferramenta o
próprio Steam AppID quando ela for o próprio app Steam, e escreva um atalho `.desktop`
que a inicia pela visão combinada sem abrir o Eidos. `eidos tool <game> import-mo2`
lista os executáveis do `ModOrganizer.ini` da pasta de uma lista, e `--write` os
adiciona. Um exe na aba Data pode ser executado uma vez ou adicionado como
ferramenta, e `eidos tool <instance> run --exe <path>` executa um a partir de um
terminal. Com Ask before playing, Run pergunta antes quando a saída de um gerador está
desatualizada.

Stop encerra um jogo ou ferramenta em execução. O menu File executa winecfg, regedit,
o explorador de arquivos do Wine e o desinstalador do Windows no prefixo do jogo.
Rebuild prefix, no Diagnostics, renomeia o prefixo para o Steam criar um novo, e não
apaga nada. `eidos shell <instance>` abre um shell na visão combinada, ou executa o
comando dado depois de `--`, e o que ele escreve cai no Overwrite.

Com a janela em segundo plano, o Eidos pode enviar uma notificação da área de
trabalho quando termina uma execução de ferramenta, uma instalação de coleção, um
pack ou unpack, ou o último download em andamento, e quando uma ordenação do LOOT
falha. Notify the desktop when a long job ends, em Preferences, liga isso; vem
desligado por padrão.

### Diagnostics

A aba Diagnostics lista masters faltando, arquivos órfãos, deriva da lista de mods e
conjuntos de plugins danificados, e depois de uma execução, o que o log do próprio
script extender diz que carregou. Ela também reporta:

- um script extender que está faltando ou não combina com a versão do jogo
- um jogo que mudou desde o último lançamento limpo
- uma opção de inicialização do Steam que não inicia o Eidos ou não tem `%command%`
  depois dele
- um Proton mais antigo que o que preparou o prefixo por último, ou um vindo do Steam
  Flatpak
- `PROTON_ENABLE_NVAPI` faltando para o DLSS, e tela cheia exclusiva, que impede a
  geração de quadros do Community Shaders
- um `enblocal.ini` sem `LinuxVersion=true`
- o log de falha da última sessão e os mods que ele nomeia
- requisitos do Nexus que estão faltando ou desativados
- um manipulador `nxm://` que é outro programa ou um arquivo ausente
- linhas `sTestFile`, que fazem o jogo ignorar `plugins.txt`
- plugins `.esp` ativados no Starfield, e plugins blueprint inválidos ou sem par
- um patch cujos arquivos soltos perdem para o mod que ele corrige
- um mod cujas cópias dos scripts Papyrus do script extender vencem as do próprio
  extender
- um `vm.max_map_count` ou limite de arquivos abertos baixo
- arquivos do jogo num sistema de arquivos do Windows
- caminhos de dados do BodySlide que não são a pasta Data do jogo, e pastas em
  `Overwrite/Root` que pertencem a Data
- um `ModOrganizer.ini` de lista que aponta ferramentas para outra pasta de jogo
- uma cópia do jogo que está faltando ou não tem executável do jogo

`eidos check <instance> [--json]` executa as verificações que não precisam de janela e
sai com 1 quando acha um problema. Copy report copia um relatório com a sua pasta
pessoal escrita como `~` e o seu nome de usuário como `<user>`, e Save tree na aba Data
grava a árvore Data combinada com a origem de cada arquivo, ocultada do mesmo jeito.

Consulte [Verificações de integridade e arquivos gerados](../../../docs/internals/integrity.md) (em inglês) para saber sobre a proteção nas substituições, conflitos entre arquivos BSA/BA2, diagnósticos estáticos do SKSE e registros de geração, incluindo suas limitações.

### Onde ele guarda os próprios arquivos

O Eidos usa o layout que todo programa da família Colony usa:
`~/.config/Colony/Eidos/` para o que você escolheu (preferências, sua sessão do Nexus,
sua lista de instâncias, as definições de jogos e de add-ons que você escreveu),
`~/.local/share/Colony/Eidos/` para instâncias globais, runtimes de ferramentas
(baixados, ou copiados do pacote pelo `install.sh`) e logs de sessão, e
`~/.cache/Colony/Eidos/` para as masterlists do LOOT. Dentro de uma instância,
`Stock Game` guarda uma cópia congelada do jogo e `test-run/` guarda uma execução de
teste esperando revisão.

Um Eidos mais antigo guardava os arquivos em `~/.config/eidos/`,
`~/.local/share/eidos/` e `~/.local/state/`. O primeiro início depois da atualização
os leva para o novo lugar, diz isso no log e deixa os diretórios antigos como
estavam. Até uma movimentação ser concluída, o Eidos continua usando os arquivos
antigos. Detalhes em [internals/paths.md](../../internals/paths.md) (em inglês).

### Jogos suportados

O Skyrim SE/AE é comprovado em jogo real. O Fallout 4 também está ligado de ponta a
ponta (F4SE trocado automaticamente, invalidação de arquivos, ordem de carga com
asterisco, LOOT, saves `.fos`); veja [guide/fallout4.md](docs/guide/fallout4.md).
Ligados e procurando testadores: Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR,
Fallout 3, Fallout NV, Starfield, Oblivion e Morrowind. Fallout 3, Fallout NV,
Oblivion e Morrowind ordenam plugins pelas datas dos arquivos: o Eidos mantém uma
ordem por perfil e mostra essas datas dentro da montagem, sem alterar os arquivos no
disco.

Quatro outros jogos estão ligados e também procuram testadores: Stellar Blade (mods
Unreal `.pak` em `~mods`), Stardew Valley (mods SMAPI, com o SMAPI instalado à parte),
7 Days to Die (pastas `Mods/` ao lado do jogo) e Crimson Desert. O Crimson Desert aceita
só mods de código: plugins ASI, ReShade e DLLs de loader em `bin64`. Os mods de dados
dele, que vivem em grupos PAZ, não são suportados.

Um jogo é um único arquivo TOML, e o seu pode ir em `~/.config/Colony/Eidos/games/` sem
recompilar: [internals/adding-games.md](../../internals/adding-games.md).

## Instalação

### Via Colony (recomendado)

Procure **Eidos** na [Colony](https://github.com/Project-Colony/Colony) e instale. As
atualizações chegam pelo lançador. A Colony instala um único arquivo, `eidos-linux`: a
janela, e também a linha de comando `eidos` quando ele é chamado por um link chamado
`eidos`. Esse arquivo não traz nem o programa auxiliar de prévia de modelos NIF nem os
runtimes da Microsoft do pacote abaixo. Sem o auxiliar, a prévia avisa e todo o resto
funciona, e o Eidos baixa um runtime da Microsoft quando você instala os pré-requisitos
de uma ferramenta. O que fazer depois de instalar, e como apontar o Steam para ele:
[docs/guide/install.md](docs/guide/install.md#via-colony).

### Download direto do binário

Baixe um arquivo da [versão mais recente](https://github.com/Project-Colony/Eidos/releases/latest):

| Arquivo | O que é |
|---|---|
| `eidos-linux` | o programa inteiro num único arquivo, como a Colony o instala |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, o programa auxiliar de prévia de NIF, os instaladores do Visual C++ 2015-2022 e do .NET 8 Desktop Runtime e o runtime .NET 10, ícones, avisos de licença e um instalador que registra a entrada da área de trabalho e o manipulador de downloads do Nexus |

Verifique o download antes, como mostra a
[Política de assinatura de código](#política-de-assinatura-de-código), e depois:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

O instalador põe os binários em `~/.local/bin` sem sudo e copia o `runtimes/` do pacote
para `~/.local/share/Colony/Eidos/runtimes`. `--system` e `--bindir DIR` põem os
binários em outro lugar, e `--from DIR` os pega de outra pasta. `--cap` pede ao sudo o
`CAP_SYS_ADMIN`, de que só o passthrough experimental do FUSE precisa, e o passthrough
continua desligado; `--no-cap` é o padrão. Os dois arquivos exigem Linux x86_64 com
glibc 2.39 ou mais recente, FUSE (`fusermount3`) e 7-Zip para os arquivos de mods.

O Eidos não se atualiza sozinho. Uma vez por dia, ao iniciar, a janela pergunta ao
GitHub qual é a versão mais recente, e quando há uma mais nova mostra um link para as
notas dela depois de Help na barra de menus. Preferences desliga isso, e uma cópia em
`/usr` fora de `/usr/local`, que um gerenciador de pacotes atualiza, nunca pergunta.

### Compilar a partir do código

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Exige Rust 1.92 ou mais recente (o `rust-version` que a CI verifica; a CI e as versões
compilam com 1.99.1), mais `just`, CMake 3.20+ e um compilador C++17 para o programa
auxiliar de prévia de NIF. `just build` usa o perfil release: uma unidade de geração de
código e LTO completo. Uma compilação a frio de `eidos` e `eidos-gui` com 3 jobs levou
334 segundos assim e 243 segundos com LTO thin, e o LTO completo deixa o `eidos` 1,2 MB
e o `eidos-gui` 3,8 MB menores (medições em `Cargo.toml`). `just profile=dev build`
compila mais rápido. Um pacote do Arch é compilado a partir de `packaging/PKGBUILD`.
Pré-requisitos e os comandos sem `just`: [docs/guide/install.md](docs/guide/install.md).

### Opções de inicialização do Steam

O Eidos roda como o comando de inicialização do jogo, e é assim que ele monta antes de o
jogo começar. A linha base é tudo de que a maioria das configurações precisa, com o
caminho absoluto da sua instalação (o Steam não lê o seu `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Pela Colony esse caminho é `~/.local/share/Colony/apps/Eidos/eidos-linux`. Depois
aperte Jogar: o Eidos abre na instância desse jogo, e Run inicia o jogo pela visão
combinada.

Numa janela que você abriu por conta própria, Run pede ao Steam que inicie o jogo
(`steam -applaunch <appid>`), então a opção de inicialização continua valendo, e você
clica em Run na janela que o Steam abre. O Heroic recebe um link `heroic://launch`, e o
Legendary executa `legendary launch <app>`, depois que o Eidos é definido como wrapper
do jogo ali. Quando a configuração da loja falta, Run não inicia nada e diz o que
definir: [guide/usage.md](docs/guide/usage.md#iniciar-o-jogo). Com Start the game at once
when a store opens Eidos (Preferences, desligado por padrão), o Eidos executa as
verificações do Diagnostics e inicia o jogo sem mostrar a janela quando nada precisa de
atenção.

Todo o resto são variáveis de ambiente empilhadas na frente, e elas se combinam
livremente:

| Você quer... | Ponha na frente |
|---|---|
| DLSS com o Community Shaders | `PROTON_ENABLE_NVAPI=1`. Sem ela o DLSS nunca inicializa e não diz nada; a lista completa é [guide/graphics.md](docs/guide/graphics.md) |
| um contador de FPS na tela | `DXVK_HUD=fps` |
| interpolação de quadros no nível do driver, zero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, nunca junto com a geração de quadros do próprio Community Shaders |
| logs detalhados para um relatório de bug | `EIDOS_LOG=debug` (os logs de sessão caem em `~/.local/share/Colony/Eidos/logs/`) |
| um relatório de E/S por sessão da montagem | `EIDOS_FUSE_STATS=1` |
| outro número de workers do FUSE | `EIDOS_FUSE_THREADS=8` (padrão 4; `1` é a primeira coisa a tentar ao caçar um bug de concorrência) |
| este início preso a uma instância portátil | `EIDOS_INSTANCE=/path/to/folder`. Sem ela o Eidos abre a instância que você usou por último |
| a janela por um início com Start the game at once ligado | `EIDOS_WINDOW=1` |

A linha para manter numa configuração moderna com mods (Community Shaders, DLSS, geração
de quadros) é o comando final, não um exemplo:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Acrescente `DXVK_HUD=fps` na frente enquanto verifica que a configuração funciona, e
tire-o quando funcionar.

As mesmas variáveis podem ir nas variáveis de inicialização do perfil em vez de na linha
do Steam, o que as mantém por perfil e as aplica também às execuções de ferramentas:
[guide/usage.md](docs/guide/usage.md#variáveis-e-wrappers-por-perfil).

As chaves de diagnóstico mais profundas (`EIDOS_FUSE_TRACE`, as chaves de bissecção de
cache e índice, por que `EIDOS_FUSE_PASSTHROUGH` vem desligado) estão em
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Documentação

Tudo está em [docs/](../../), indexado por quem lê em
[docs/README.md](docs/README.md): o guia do usuário (instalação, a CLI e a interface
gráfica, ferramentas como xEdit e DynDOLOD, gráficos, solução de problemas), como o Eidos
funciona por dentro, e por que ele existe. Política de segurança e como relatar uma
vulnerabilidade: [SECURITY.md](../../../SECURITY.md).

## Idioma

As páginas de que um jogador precisa são traduzidas. O inglês é a referência: quando uma
tradução discorda dele, o arquivo em inglês está certo.

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

Todo o resto está em inglês. `docs/internals/` e `docs/project/` são lidos por gente que
também está lendo o Rust, o `CHANGELOG.md` é gerado, e o `SECURITY.md` também não é
traduzido. Traduzir os dois primeiros acrescentaria mais de 19.000 palavras para manter
em dia para leitores que não precisam delas.

Cada tradução carrega o hash do arquivo em inglês do qual foi feita, e a CI falha enquanto
esse hash não bate mais com o inglês; veja
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). A tradução é então atualizada e
recarimbada com `scripts/i18n-check.sh --fix`. Uma tradução desatualizada é atualizada, não
removida.

Um idioma é um único diretório. `docs/i18n/<lang>/` espelha a raiz do repositório, então
`docs/i18n/de/docs/guide/install.md` é o `docs/guide/install.md` em alemão. É por isso que
um link entre duas páginas traduzidas é a mesma string que o link entre seus originais em
inglês, e por isso que remover um idioma é um único `rm -r`.

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

O Eidos não envia telemetria, nem estatísticas de uso, nem relatórios de falha. Ele só se
conecta aos servidores desta tabela, e só para a ação indicada ali. O modo Offline
(Preferences, Nexus) interrompe toda conexão com o Nexus e a verificação diária de versões.

| Dado | Guardado ou enviado | Onde, e por quê |
|---|---|---|
| Preferências, lista de instâncias, definições de jogos e de add-ons | guardado | `~/.config/Colony/Eidos/` |
| Instâncias, runtimes de ferramentas, logs de sessão | guardado | `~/.local/share/Colony/Eidos/`, ou a pasta que você escolheu para uma instância portátil |
| Masterlists do LOOT | guardado | `~/.cache/Colony/Eidos/loot/` |
| Verificação de versões | enviado | Ao iniciar, no máximo uma vez por dia, uma requisição a `api.github.com/repos/Project-Colony/Eidos/releases/latest`. O Eidos lê só a tag da versão e não baixa nada. Vem ligada por padrão e se desliga em Preferences, e é pulada no modo Offline e para uma cópia instalada em `/usr` fora de `/usr/local`, que um gerenciador de pacotes atualiza. [Declaração de privacidade do GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Login no Nexus | enviado | Entrar abre `users.nexusmods.com` no seu navegador, que devolve o resultado a um receptor que o Eidos abre em `127.0.0.1` só para esse login. Depois o Eidos troca esse resultado por tokens em `users.nexusmods.com/oauth/token` e busca as chaves públicas com que o Nexus os assina. [Política de privacidade do Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Credenciais do Nexus | guardado | Os tokens de acesso e de renovação, em `~/.config/Colony/Eidos/nexus.ini` com modo 0600. Uma linha `api_key=` escrita ali por um Eidos mais antigo é mantida, mas nunca lida; apague-a se quiser. |
| Requisições ao Nexus | enviado | `api.nexusmods.com`, quando você usa um recurso do Nexus: o nome da sua conta e o status premium, detalhes de mods e arquivos, verificações de atualização, coleções e links de download. Seus endossos e mods acompanhados são lidos a cada verificação de atualização e gravados quando você clica em Endorse ou Track. Requisitos de mods, descrições e changelogs, e a busca de coleções, vão para `api.nexusmods.com/v2/graphql`. Toda requisição leva o seu token de acesso e identifica o Eidos e a versão dele. |
| Downloads do Nexus | enviado | Os arquivos que você pede, do servidor da CDN do Nexus indicado no link de download, salvos na pasta de downloads da instância com um `.meta` no formato do MO2 ao lado de cada um. |
| Links `nxm://` | guardado | `eidos nxm --register`, que o `install.sh` roda para você, escreve `~/.local/share/applications/eidos-nxm.desktop` e o torna o manipulador de `nxm://` via `xdg-mime`, para que o botão Mod Manager Download do site entregue o link ao Eidos. O pacote do Arch instala a mesma entrada para todo o sistema. |
| Masterlists do LOOT | enviado | `raw.githubusercontent.com/loot/...`, na primeira ordenação do LOOT de um jogo e sempre que você pede uma atualização. Nada seu é enviado. [Declaração de privacidade do GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtimes de ferramentas | enviado | Só quando você instala os pré-requisitos de uma ferramenta, e só para o que o pacote de versão não trouxe (o arquivo único `eidos-linux` e uma compilação a partir do código não trazem nada): o runtime .NET 10 de `builds.dotnet.microsoft.com` e os instaladores do Visual C++ e do .NET 8 de `download.visualstudio.microsoft.com`, cada um recusado a menos que bata com o SHA-256 embutido no Eidos; o SDK do .NET 10, sempre de `builds.dotnet.microsoft.com` e conferido do mesmo jeito; e o `winetricks` para os outros pré-requisitos (.NET Framework 4.8, XACT, .NET 6 e 7), que baixa o que instala da Microsoft e dos outros servidores que o winetricks usa. [Declaração de privacidade da Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Logs de sessão | guardado | `~/.local/share/Colony/Eidos/logs/`, modo 0600: dez sessões por instância, apagadas depois de 30 dias. Sua pasta pessoal é escrita como `~` e seu nome de usuário como `<user>`, então um log pode ser colado numa issue do jeito que está. Eles nunca saem da sua máquina a menos que você os envie. |
| Registros de execução | guardado | `<instance>/logs/sessions/`: de cada execução de jogo ou ferramenta, o perfil, os mods ativados com suas versões e ids de arquivo do Nexus, a ordem dos plugins, uma soma de verificação de cada INI do perfil, as versões do jogo e do Eidos, o código de saída e os saves que a execução criou. Os 50 mais novos são mantidos, `eidos sessions` os lê, e `eidos pack` os deixa de fora. Eles ficam na sua máquina. |
| Relatórios de atualização | guardado | `<instance>/logs/updates/<mod>.json`: o que a última atualização ou Replace mudou na pasta desse mod. `eidos pack` os deixa de fora. |
| Atalhos | guardado | O `install.sh` escreve `eidos.desktop` e os ícones dele em `~/.local/share`; um atalho de ferramenta ou de perfil que você cria vai para `~/.local/share/applications`. |
| Notificações da área de trabalho | nenhum dos dois | Desligadas por padrão. Quando ligadas, vão ao serviço de notificações da sua área de trabalho por D-Bus (`org.freedesktop.Notifications`) e nomeiam o trabalho que terminou. |
| Links que você abre | nenhum dos dois | Páginas de mods e outros links abrem no seu próprio navegador via `xdg-open`. |
| Add-ons | nenhum dos dois | Extensões que você instala rodam como programas próprios na sua máquina; o que elas fazem é com elas. |

## Licença

O código original do Eidos está sob GPL-3.0-or-later; veja [LICENSE](../../../LICENSE).
Estas partes estão sob GPL-3.0 sem a opção "or later":

- as traduções dos scripts OMOD em `eidos-install`; consulte seus
  [créditos originais](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- o programa auxiliar `eidos-nif-preview`, construído sobre a biblioteca nifly incluída no
  repositório; consulte as
  [licenças e os créditos de seus componentes](../../../crates/eidos-nif-preview/UPSTREAM.md);
- os crates `esplugin` e `libloadorder` aos quais o código de ordem de carga se liga, que
  declaram `GPL-3.0`.

Os outros crates Rust nos binários mantêm as próprias licenças (MIT, Apache-2.0, BSD e
semelhantes). O pacote de versão traz os avisos deles em `licenses/crates/`, com um índice de
cada crate e sua licença.

Os binários também embutem oito DLLs redistribuíveis genuínas do Microsoft DirectX
(`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` e `d3dx11_43`, de 32 e 64 bits), guardadas
compactadas com gzip (7,9 MB em vez de 17,9 MB), que o Eidos descompacta e implanta no
prefixo do jogo quando um mod ou uma ferramenta precisa delas. Elas continuam sob os termos
de redistribuição da Microsoft; veja a
[procedência](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md) delas.

O pacote de versão também traz, em `runtimes/`, os instaladores do Visual C++ 2015-2022 da
Microsoft, o instalador do .NET 8 Desktop Runtime e o zip do runtime .NET 10, como a
Microsoft os publica. Os termos deles estão em `licenses/microsoft-runtimes/`
(`vc_redist.txt`, `dotnet.txt`).
