<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=af0c14c5192ccae592dd1c1661fd02d5242ecd5a -->

# Solução de problemas e diagnóstico

As chaves de ambiente, como ler os contadores de operações, os cartões Health da
aba Diagnostics, como achar o mod por trás de um travamento, o que anexar a um
relatório de bug e os problemas conhecidos com sua história.

## Diagnosticando o VFS

Duas variáveis de ambiente cobrem a maioria dos casos em que o jogo vê algo com que
o sistema de arquivos não concorda:

```sh
EIDOS_FUSE_STATS=1                  # contadores de operações, despejados na desmontagem
EIDOS_FUSE_NO_CACHE=1               # todo cache do lado do kernel desligado
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # ou nomeie um a um
```

A forma granular foi o que achou o travamento de `FOPEN_KEEP_CACHE` descrito em
Problemas conhecidos: desligar os quatro responde "é o cache?", e só nomeá-los
responde "qual deles". Os contadores respondem à outra metade. Com o passthrough
ligado (ele vem desligado por padrão), uma carga que mostra `read 0` é uma em que o
`FUSE_PASSTHROUGH` serviu cada byte dentro do kernel, então tudo que você ia ajustar
no caminho de leitura já sai de graça.

Mais algumas chaves estreitam ainda mais a falha:

```sh
EIDOS_FUSE_TRACE=open,opendir       # registra cada open / opendir (1 = todos os canais)
EIDOS_FUSE_THREADS=1                # uma thread do daemon (padrão: número de CPUs, no máximo 4)
EIDOS_FUSE_OPENDIR=1                # mantém os handles de opendir mesmo quando o kernel pode dispensá-los
EIDOS_NO_INDEX=1                    # percorre as camadas em vez de usar o índice de camadas
EIDOS_NO_OW_INDEX=1                 # o mesmo para o índice de pertencimento do Overwrite
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE de volta, veja Problemas conhecidos
```

Num lançamento pelo Steam, coloque-as antes do comando na opção de inicialização,
por exemplo `EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. A saída vai para
o log da execução: o `logs/run-*.log` da instância num lançamento pela janela, o
console do Steam num lançamento pelo Steam.

Para olhar a própria visão combinada, `eidos shell <instance>` abre um shell na
pasta do jogo como o jogo a vê, e `eidos shell <instance> -- ls Data/meshes`
executa um único comando ali; tudo que for escrito cai no Overwrite
([usage.md](usage.md#um-shell-na-visão-combinada)). O botão Save tree da aba Data
grava a árvore `Data` combinada inteira num arquivo de texto, um arquivo por linha
com o mod de onde ele vem, com a sua pasta pessoal oculta.

## Montar uma união à mão

O `eidos-fuse` vem com o pacote do Arch e com as compilações a partir do código:
`just build` o compila, e `just install` ou `install.sh` o copia quando ele existe.
O pacote de versão e o `eidos-linux` não o trazem. A linha de uso dele:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

A primeira `--layer` vence no conflito; a última são seus dados de jogo intactos. A
montagem só precisa de `/dev/fuse` e `fusermount3` (sem overlayfs, sem Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... leia e escreva através de /mnt/point ...
fusermount3 -u /mnt/point
```

As escritas caem em `--overwrite <dir>` (um diretório temporário se omitido), de
modo que as próprias camadas permanecem intactas mesmo aqui.

## Por que o passthrough vem desligado

O passthrough entrega ao kernel o arquivo de respaldo real, então as leituras
pulam este daemon inteiramente. O A/B abaixo foi medido com uma versão anterior do
daemon, antes da mudança descrita mais adiante no problema conhecido "O passthrough
do FUSE impede o jogo de carregar qualquer conteúdo de mod". Essa mudança evita a
causa mais provável, mas ainda não foi medida de novo em um jogo, então o
passthrough continua desligado por padrão até que seja. Medido em A/B no Skyrim SE
1.6.1170, proton-cachyos 11.0, kernel 7.1.4, a mesma ordem de carga de 82 plugins,
com a única variável sendo se o binário carregava a capacidade:

| passthrough | falhas de `NtCreateFile` com `STATUS_ACCESS_VIOLATION` |
|-------------|---------------------------------------------------------|
| ligado      | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| desligado   | 0                                                       |

Com ele ligado, o jogo não abria nenhum dos próprios arquivos nem plugins, o que
aparecia dentro do jogo como mods que não estavam lá - sem erro, sem linha de log.
Com ele desligado, a mesma ordem de carga chegava à jogatina com seus plugins,
arquivos e scripts Papyrus ativos.

A falha era invisível de dentro do daemon: o próprio `open` dele tinha sucesso toda
vez e o kernel nunca recusava um arquivo de respaldo (verificado ao longo de uma
sessão inteira falhando com `EIDOS_FUSE_TRACE=open`: zero `open FAILED`, zero
`passthrough refused`). O erro era produzido depois que o daemon tinha respondido
`opened_passthrough`, então nenhum log do lado do daemon conseguia vê-lo. Também não
dependia da extensão - atingia arquivos e plugins igualmente, ou seja, os arquivos
que o jogo mantém abertos por toda a execução.

`EIDOS_FUSE_PASSTHROUGH=1` religa, para medir o que ele rende ou para retestar o
mecanismo. Os avisos de capacidade no lançador e na aba Diagnostics só aparecem
quando você o pediu.

## O índice de camadas está mesmo em uso?

`LayerStack::new` constrói um mapa completo das camadas somente-leitura ou nenhum.
Sem mapa, cada consulta percorre as camadas como antes: as respostas são as mesmas,
carregar um save leva cerca de três vezes mais tempo e nada mais parece diferente.

Quando o índice está desligado, a montagem diz por quê, uma vez, ao iniciar:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

O motivo nomeia um diretório ilegível, uma camada com mais de 64 níveis de
diretório, mais de 4.000.000 de entradas ou `EIDOS_NO_INDEX`. A linha vai para o log
da execução: o `logs/run-*.log` da instância num lançamento pela janela, o console
do Steam num lançamento pelo Steam. Com `EIDOS_FUSE_STATS=1`, o relatório ao
desmontar também tem uma linha `layer index:`, e um contador `unindexed` diferente
de zero numa montagem normal significa que o índice estava desligado.

Os exemplos abaixo verificam a mesma coisa offline, nas pastas de uma instância, sem
montagem:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` resolve caminhos reais com e sem o índice e compara as varreduras de
diretório. `index_agrees` confere que os dois respondem A MESMA coisa, em cada
caminho e cada listagem de uma instância real. `listing_cost` mede o que o mapa de
filhos combinados poupa no `readdir`.

`EIDOS_NO_INDEX=1` força o percurso, para quando a diferença entre as duas respostas
é justamente o que está sendo depurado.

## Iniciar o jogo pela CLI

Para iniciar o jogo pela CLI `eidos` em vez de pela janela, defina a opção de
inicialização do Steam como o caminho absoluto do `eidos`, por exemplo:

```
~/.local/bin/eidos play skyrimse -- %command%
```

Nenhum prefixo `WINEDLLOVERRIDES` é necessário. Quando um mod ativado importa
`d3dcompiler_47`, o Eidos implanta a cópia que traz embutida dentro do prefixo,
força-a como nativa e funde isso com qualquer sobreposição de DLL que um mod traga
(carregadores ENB/ReShade/`.asi`).

O Eidos nunca inicia o jogo pelo Proton por conta própria.
`eidos tool <instance> run <title>` com uma entrada que inicia o jogo (o loader do
script extender, o lançador ou o binário do jogo) pede à loja dona da cópia que o
inicie, como Run faz numa janela que nenhuma loja abriu, e imprime o que definir
quando a configuração da loja não inicia o Eidos. `--print` imprime o comando da
loja em vez de executá-lo.

## Um jogo ou ferramenta que não fecha

Enquanto um jogo ou ferramenta roda, o botão Run vira Stop, e o cartão de bloqueio
também tem um botão Stop. Pressione-o duas vezes. O segundo toque encerra todos os
programas que a execução iniciou: primeiro SIGTERM, depois SIGKILL após 10 segundos
para o que sobrar. O que o jogo não tinha salvo se perde. Depois, o Eidos faz o
mesmo que faz após qualquer saída: as mudanças de INI e de plugins voltam para o
perfil, a saída das ferramentas é registrada e os mods são desmontados. A execução é
registrada com o código de saída 143, seja o que for que o programa devolveu, então
`eidos sessions <instance> diff` nunca a toma como a última execução boa. Unlock é
diferente: ele devolve a janela e deixa o programa rodando. Num terminal, `kill <pid>`
no processo `eidos` (o pid está no cartão de bloqueio) faz o mesmo que Stop.

## Achar o que quebrou

Para achar qual mudança quebrou o jogo:

- `eidos sessions <instance> diff` lista o que mudou desde a última execução do jogo
  que saiu com código 0: mods, versões, ordem de plugins, INIs e o executável do
  jogo ([usage.md](usage.md#o-que-mudou-desde-a-última-execução-boa)).
- Test run, ao lado de Run, joga com os mods do perfil, sem mods, ou com os mods
  selecionados e os masters de que eles precisam, e envia tudo que o jogo escreve
  para `<instance>/test-run/`. Depois da execução você guarda ou joga fora cada grupo
  de arquivos: saves, mudanças de INI, estado dos plugins, saída do Overwrite e logs
  ([usage.md](usage.md#execuções-de-teste)).
- Find the mod behind a problem, na aba Diagnostics, executa execuções de teste com
  metade dos suspeitos restantes a cada vez e pergunta se o problema aconteceu. 200
  suspeitos levam cerca de 8 execuções, e a busca sobrevive ao fechamento do Eidos
  ([usage.md](usage.md#encontrar-o-mod-por-trás-de-um-problema)).

Uma execução interrompida nunca preenche um Yes nessa busca, porque é registrada com
o código de saída 143 (veja acima).

## Cartões Health

A aba Diagnostics mostra um cartão Health para cada problema que encontra, com sua
causa e, em alguns, um botão que o corrige. Os cartões da opção de inicialização
estão descritos em [install.md](install.md), os cartões de NVAPI, tela cheia e ENB em
[graphics.md](graphics.md), e os cartões do Proton e do prefixo na próxima seção.
`eidos check <instance>` imprime os achados que não precisam de janela
([usage.md](usage.md#verificações-de-saúde-pelo-terminal) os lista), entre eles os
cartões de prefixo, registro e da unidade `S:`. Só a janela mostra o cartão Proton
com suas variantes Flatpak, build desconhecida e build mais antiga, a ordem dos
patches, os scripts do extender, os arquivos que nenhum plugin carrega, os
requisitos do Nexus, as cópias desatualizadas de plugins e o cartão Display.

### O sistema

- "vm.max_map_count is <value>" aparece abaixo de 1048576, o padrão no Arch Linux,
  no Fedora 39 em diante e no Ubuntu 24.04 em diante. Alguns jogos no Proton mapeiam
  mais regiões de memória e então travam sem erro. Ponha
  `vm.max_map_count = 1048576` num arquivo em `/etc/sysctl.d`, por exemplo
  `80-gamecompatibility.conf`, e rode `sudo sysctl --system`.
- "The open files limit is <value>" aparece quando o limite rígido é menor que
  524288, de que o esync do Proton precisa; jogos podem não iniciar ou travar abaixo
  dele. O systemd 240 e posteriores o definem. Num sistema mais antigo, defina
  `DefaultLimitNOFILE=524288` em `/etc/systemd/system.conf` e
  `/etc/systemd/user.conf` e faça login de novo.
- "Game files on a Windows file system" nomeia a pasta do jogo, o prefixo ou a raiz
  da instância quando ela está em `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g, exfat-fuse),
  `exfat` ou `vfat`. O Proton precisa de links simbólicos, permissões Unix e nomes
  sensíveis a maiúsculas. Mova o jogo, o prefixo dele e a instância para um sistema de
  arquivos Linux como ext4, btrfs ou xfs.

### Os arquivos do jogo

O cartão Game files, no topo da aba, mostra se a instância executa a instalação da
loja ou uma cópia do jogo dentro da instância
([usage.md](usage.md#arquivos-do-jogo-a-instalação-da-loja-ou-uma-cópia)). Os
lançamentos são recusados, com um cartão que diz o motivo, em três casos:

- "The game copy '<folder>' is missing": a instância executa uma cópia que sumiu.
  Ponha-a de volta, ou volte para a versão da loja.
- "'<folder>' has no <binary>": a pasta que a instância executa não tem nenhum
  executável do jogo, então não é uma cópia do jogo.
- "<game> cannot run from a copy yet": Morrowind, e Oblivion com
  `bUseMyGamesDirectory=0`, guardam a lista de plugins na pasta do jogo.

"ModOrganizer.ini points tools at another game folder" aparece quando o
`ModOrganizer.ini` próprio de uma lista, não escrito pelo Eidos, tem um `gamePath`
diferente dos arquivos do jogo que a instância executa. Ferramentas no modo Mod
Organizer 2, como o PGPatcher, leem o jogo dali. Mude o `gamePath` nesse arquivo, ou
remova o arquivo para o Eidos escrever o dele.

### A ordem de carga

- "sTestFile lines make the game ignore plugins.txt" aparece para Fallout 4,
  Fallout 4 VR e Starfield quando o `Fallout4Custom.ini` ou o `StarfieldCustom.ini`
  do perfil tem linhas `sTestFile1` a `sTestFile10` em `[General]`. Enquanto uma
  estiver definida, o jogo carrega esses plugins e a ordem de carga que o Eidos
  escreve não tem efeito. Fix remove as linhas da cópia do perfil e guarda o arquivo
  anterior como `<ini>.pre-fix`.
- Starfield: plugins `.esp` ativados sem a flag master ganham um cartão, porque o
  Starfield não consegue ordená-los entre os `.esm` e os plugins com flag master e
  mantém alguns registros deles carregados durante toda a sessão. Uma versão `.esm`
  do autor evita os dois problemas.
- Starfield: "Invalid blueprint plugins" lista plugins que têm algumas, mas não
  todas, das marcas de blueprint: a flag de blueprint, um nome que começa com
  `BlueprintShips-` e a extensão `.esm`. "Blueprint plugins without their main
  plugin" lista aqueles cujo plugin principal não está ativado:
  `BlueprintShips-Example.esm` precisa de `Example.esm`, `.esp` ou `.esl`.
- "<patch> loses <n> file(s) to <master>" aparece quando um plugin de um mod precisa
  de um master de outro mod, e esse outro mod tem a prioridade maior, de modo que as
  cópias dele dos arquivos que ambos trazem vencem. Um patch costuma ir abaixo do mod
  que ele corrige, e o botão Move do cartão o coloca lá. Mantenha a ordem se o patch
  deve mesmo perder esses arquivos.
- "Out-of-date copy of <plugin>" aparece quando um mod guarda uma cópia limpa ou com
  flag light de um plugin cujo original mudou desde então (atualização de mod ou do
  jogo). O jogo carrega a cópia antiga. Oculte-a, depois limpe ou marque o plugin
  novo de novo.

### A lista de mods

- "Order rules of the collection <name> that no longer hold" aparece quando os mods
  de uma coleção instalada não estão mais na ordem que as regras dela pedem. Restore
  collection order move de volta só os mods dessa coleção, e Ctrl+Z desfaz.
- "<mod> requires <mod>, which is not installed" (ou "which is disabled") vem dos
  requisitos do Nexus que Check for updates lê. Open on Nexus vai para a página do
  mod exigido, Download main file o baixa numa conta premium, e Ignore oculta um
  requisito que outro mod já atende. Um requisito de DLC é só um conselho: o Eidos
  procura um plugin com o nome da DLC.
- "fontconfig.txt names a font library that is missing" (motores do Skyrim) nomeia as
  fontes que nenhuma camada e nenhum arquivo contém, e o mod cujo
  `Interface/fontconfig.txt` vence. O texto dos menus pode aparecer como quadrados até
  a fonte ser instalada ou a linha removida.
- "BA2 archive stores audio compressed" (Fallout 4 e Fallout 4 VR) conta os arquivos
  `.wav` e `.xwm` que o BA2 de um mod guarda comprimidos. O jogo pode travar ao tocá-los.
  Reempacote o arquivo com o áudio sem compressão.

### O script extender e as atualizações do jogo

- "<loader> is not installed" aparece quando há plugins do script extender instalados
  (em `SKSE/Plugins`, `F4SE/Plugins` e assim por diante), mas o loader não está em
  nenhum mod ativado nem na pasta do jogo. O jogo inicia sem eles.
- "The script extender does not match game version <version>" aparece quando o loader
  está instalado, mas a DLL de runtime da versão do jogo em execução não está, por
  exemplo `skse64_1_6_1170.dll`. O loader para com um erro. Isso costuma acontecer
  depois de uma atualização do jogo. Este cartão e o de cima têm um botão Open the
  download page para o site do extender e um botão Install the downloaded archive, que
  coloca o loader na pasta `Root/` do mod.
- "<game> changed since the last clean launch" aparece quando o executável do jogo,
  ou, numa cópia Steam, o build id dele, mudou desde o último lançamento pelo Eidos que
  terminou sem erro. Uma atualização do Steam que deixa o executável como estava não o
  levanta. Plugins do script extender construídos para a versão antiga podem deixar de
  carregar, e os cartões abaixo dele nomeiam os que declaram isso. Para manter a versão
  do jogo sobre a qual uma lista foi montada, congele-a na instância com Freeze this
  version no cartão Game files, ou com `eidos stock-game <instance> freeze --yes`
  depois que `freeze` sozinho mostrou o tamanho: atualizações posteriores da loja
  mudam a instalação da loja, não a cópia.
- "<mod> replaces <n> <extender> script(s)" aparece quando um mod com prioridade maior
  que a do script extender tem cópias soltas `.pex` dos scripts do próprio extender. Uma
  cópia antiga quebra as funções que ela declara, e o jogo não mostra erro. Mova o
  extender para baixo desse mod (o botão Move do cartão faz isso quando os dois são
  mods), ou oculte esses arquivos no mod. Quando os scripts do extender estão soltos no
  `Data` do jogo, instale o extender como um mod para que possa ser ordenado. Arquivos
  compactados não são verificados, só os arquivos soltos.

### A saída das ferramentas

- "Overwrite/Root holds meshes" (ou texturas) significa que uma ferramenta cujo
  caminho de dados apontava para a pasta do jogo escreveu essas pastas ali, onde o jogo
  nunca as lê. Corrija o caminho da ferramenta, apague as pastas e rode a ferramenta de
  novo. O botão Open folder abre `Overwrite/Root`.
- "BodySlide's GameDataPath is not the game's Data folder" (ou `OutputDataPath`): veja
  [Ferramentas](tools.md#as-configurações-da-própria-ferramenta-continuam-sendo-dela).

### Um travamento na última sessão

"The game crashed in the last session" aparece quando um registrador de falhas
escreveu um log durante a última inicialização. O cartão nomeia a exceção e as DLLs e
plugins do log que seus mods ou o Overwrite instalam. Trate-os como pistas: um mod
nomeado num log de falha costuma ser onde a falha apareceu, não a causa dela. Quando o
log não nomeia nenhum arquivo dos seus mods, o cartão diz isso. Open the log o abre, e
Copy the log (home path hidden) o copia com a sua pasta pessoal oculta.

### O tamanho da janela

Para Skyrim SE, Enderal SE e Fallout 4, um cartão Display mostra o tamanho e o modo da
janela com que o perfil inicia o jogo, e diz "No window size set for this profile"
quando nem o INI de Prefs do perfil nem o do prefixo definem um. Apply grava o tamanho e
o modo na cópia do perfil e, quando o SSE Display Tweaks define a janela, numa cópia do
INI dele no Overwrite ([usage.md](usage.md#diagnostics)).

## Qual Proton roda o jogo, e um prefixo quebrado

A aba Diagnostics tem um cartão Proton: a versão que o Eidos e o Steam usam para o
jogo, a configuração do próprio Steam e a versão que rodou o prefixo por último. Quando
um Proton mais novo que o selecionado foi o último a preparar o prefixo, o cartão vira
um conselho, porque um Proton mais antigo num prefixo mais novo pode redefinir os
arquivos do Wine dele ou não iniciar. Escolha no Steam uma versão pelo menos tão nova,
em Propriedades do jogo, Compatibilidade, ou reconstrua o prefixo.

Os outros cartões do Proton e do prefixo:

- "Proton comes from the Flatpak Steam install": esse Proton guarda o runtime e as
  bibliotecas steamclient dentro da sandbox do Flatpak, então executá-lo a partir do
  host pode falhar. O Eidos não o inicia por `flatpak run`, o que poria o jogo na
  sandbox, fora da visão combinada. Instale um Proton em
  `~/.steam/root/compatibilitytools.d/` e selecione-o para o jogo.
- "Eidos cannot tell which Proton runs this game": o `config.vdf` do Steam não nomeia
  nenhuma ferramenta de compatibilidade para ele, e o prefixo não registra nenhuma
  versão instalada. As ferramentas não iniciam até que uma seja escolhida no Steam
  (Propriedades do jogo, Compatibilidade).
- "<game> has no Proton prefix yet": inicie o jogo uma vez pelo Steam e depois aperte
  F5. Se o Steam não criar o prefixo, confira se a pasta do jogo em
  `steamapps/compatdata` é gravável; uma pasta vazia pertencente ao root o bloqueia sem
  mensagem.
- "Windows tools cannot find <game>": o registro do caminho de instalação no prefixo
  está errado ou ausente, e xEdit, DynDOLOD, Wrye Bash e o Creation Kit o leem. O Eidos
  escreve o caminho certo na próxima vez que você executar uma ferramenta.
- "The prefix's S: drive points outside the Steam library": `S:` não coincide nem com a
  raiz da biblioteca, para onde o Eidos a aponta numa ferramenta, nem com
  `<library>/steamapps`, para onde o Steam a aponta para o jogo. A biblioteca
  provavelmente mudou de lugar, ou o prefixo veio de outra máquina. Todo caminho relativo
  a `S:` guardado no prefixo não resolve em lugar nenhum até o próximo lançamento pelo
  Eidos ou pelo Steam apontá-la de novo.
- "<store> installation: started through its store" é o cartão de uma cópia GOG ou Epic.
  Run pede ao Heroic ou ao Legendary que inicie o jogo quando o Eidos é o wrapper do jogo
  ali, e as ferramentas rodam com a versão do Wine ou do Proton que o Heroic registra
  para a cópia.

O menu File executa winecfg, regedit, o explorador de arquivos do Wine e o
desinstalador no prefixo do jogo, com o Proton que roda as ferramentas do jogo
([tools.md](tools.md#os-programas-próprios-do-wine-no-prefixo-do-jogo)). As linhas
ficam esmaecidas até o prefixo ter esses programas.

O botão Rebuild prefix, no cartão Proton, é para um prefixo que não funciona mais.
Pressione-o duas vezes. O Eidos não apaga nada: ele renomeia
`steamapps/compatdata/<appid>` para `<appid>.eidos-backup-<date>-<time>` na mesma
pasta, e o Steam cria um prefixo novo na próxima vez que você iniciar o jogo pelo
Steam. Ele se recusa enquanto um jogo, uma ferramenta ou qualquer programa do Wine usa
o prefixo. Até o prefixo novo existir, Run na janela não consegue passar o jogo ao Steam
e mostra a opção de inicialização, então aperte Jogar no Steam. Saves e configurações
guardados só no prefixo antigo ficam na pasta renomeada: copie de volta o que precisar e
apague essa pasta você mesmo quando não a quiser mais.

## O que anexar a um relatório de bug

- Copy report, na aba Diagnostics, copia a versão do Eidos, o jogo, a versão do Proton,
  se o Steam é o Flatpak, o estado do passthrough, o tipo de sessão, a distribuição, o
  kernel, `vm.max_map_count`, o limite de arquivos abertos, se `/dev/fuse` e
  `fusermount3` existem, os nomes das variáveis de inicialização e dos programas wrapper
  do perfil, e todo cartão Health. Sua pasta pessoal aparece como `~` e seu nome de login
  como `<user>`. `eidos check <instance>` imprime o mesmo cabeçalho num terminal, e
  `--json` o entrega como JSON.
- O log da execução, `<instance>/logs/run-*.log`, começa com o comando exato que o Eidos
  executou. File, Eidos logs abre a pasta dos logs do próprio Eidos,
  `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden), no cartão de falha, copia o log de falha.
- Save tree, na aba Data, grava a árvore `Data` combinada com a origem de cada arquivo.

## Problemas conhecidos

### DLSS ou geração de quadros silenciosamente não faz nada

Três causas distintas, cada uma sem qualquer mensagem de erro: NVAPI não habilitado nas
opções de inicialização, tela cheia exclusiva, ou um teto de FPS do Reflex obsoleto. A
lista inteira está em [graphics.md](graphics.md).

### Run não faz nada com o Steam Flatpak

Numa janela que nenhuma loja abriu, Run pede ao Steam que inicie o jogo executando
`steam -applaunch <appid>`. O Steam Flatpak não coloca nenhum `steam` no `PATH` do host,
então a linha de status diz "Could not start Steam" com o erro. Aperte Jogar no Steam.

### Um save de outro dispositivo não aparece

Um save que chega ao prefixo sem passar pelo Eidos, como um download do Steam Cloud
vindo de um Steam Deck, fica oculto enquanto os saves do perfil estão montados. Nada o
apaga. A aba Saves conta esses saves sob o título dela, e Import os copia para o perfil
sem substituir um save do perfil
([usage.md](usage.md#os-saves-pertencem-ao-perfil)).

### Um mod que escreve um diretório de duas formas perdia tudo sob a segunda

Corrigido. O ext4 mantém `meshes/` e `Meshes/` separados; a visão combinada não pode, e
mods reais trazem as duas - o XP32 Maximum Skeleton tem suas animações e seu arquivo de
comportamento FNIS sob a versão com maiúscula, e seus `character assets` sob a outra.

O resolvedor pegava a correspondência exata de caixa para cada componente do caminho e
se comprometia com ela: entrava em `meshes/`, não achava lá o resto do caminho e
abandonava A CAMADA INTEIRA. Todo arquivo sob a outra grafia era invisível para o jogo -
sem erro, sem log, nada em diagnóstico algum. Numa instância real de 50 camadas isso deu
74 arquivos.

Um componente que casa agora é um candidato, não uma decisão; a caixa exata continua
sendo tentada primeiro, e só quando o restante falha embaixo dela é que a varredura
procura irmãos equivalentes ignorando maiúsculas. As listagens tinham o mesmo defeito um
diretório acima e agora leem cada diretório equivalente por camada.

O índice de caminhos nunca teve esse bug, porque percorre todo diretório que encontra.
Então, por um tempo, o índice devolvia arquivos que o caminho alternativo não conseguia,
embora o alternativo seja a resposta que deveria nunca estar errada.

### O LODGen do DynDOLOD morre deixando um log vazio

Corrigido pelo `dotnet10`; veja [tools.md](tools.md). O `LODGen_SSE_<world>_log.txt` de
cada mundo guarda um cabeçalho de versão, uma linha `.NET Version:` e nada mais, e um
diálogo diz apenas "failed to generate object LOD for one or more worlds". A causa é o
Mono do Wine respondendo pelo .NET Framework, e nenhuma quantidade de .NET Framework
instalado resolve - o Proton substitui `mscoree.dll` por um link simbólico para a própria
árvore a cada atualização do prefixo. O pacote de versão traz o runtime .NET 10; as
outras instalações o baixam uma vez.

### O Wine não conseguia perceber que a montagem dobra a caixa

Corrigido.

O `get_dir_case_sensitivity` do Wine pergunta primeiro ao diretório com
`FS_IOC_GETFLAGS` e depois fareja o marcador que o CIOPFS deixa nos diretórios que
serve. Sem nenhum dos dois, o Wine supõe SENSÍVEL, e toda busca cuja grafia não bate byte
a byte recai em ler o diretório INTEIRO para achar uma correspondência insensível a
maiúsculas. Jogos da Bethesda pedem `data/ccbgssse001-fish.bsa` enquanto o arquivo é
`ccBGSSSE001-Fish.bsa`, então isso disparava em quase todo recurso: 4471 sondagens do
marcador e 2236 releituras completas de diretório em oito segundos, e 195796
enumerações de `Data` em noventa. O Skyrim SE nunca chegava ao menu principal - ficava
em 240 MB residentes enquanto o daemon queimava 92 % de um núcleo.

O Eidos dobrava a caixa em `resolve_read` desde o início. Todo o custo estava em nunca
dizer isso. Agora `lookup` responde `.ciopfs`; `readdir` continua não o listando.
`ioctl` também responde `FS_IOC_GETFLAGS` nos diretórios com a flag casefold, que o Wine
atual consulta antes de procurar o marcador.

Duas coisas o tornaram fatal em vez de meramente lento. O custo escala com o tamanho do
diretório, então instalar o conteúdo Anniversary (`Data` de 37 para 177 arquivos) fez
transbordar. E o `opendir` construía avidamente a listagem combinada, o que é puro
desperdício quando o Wine abre um diretório só para dar `stat` naquele marcador dentro
dele - o instantâneo agora é tirado no primeiro `readdir`.

Depois: o menu principal, 2,1 GB residentes, daemon a 0 % de CPU.

`EIDOS_FUSE_TRACE=opendir` foi o que achou isso, e ele acompanha o programa. Os
contadores de operações dizem quantas; 195796 enumerações de um único diretório são
invisíveis dentro de um total.

### O jogo reescreveu `plugins.txt` vazio

Muito provavelmente a mesma coisa que a dobra de caixa acima - um `Data` que ele não
conseguia enumerar em tempo razoável, do que concluía que não havia nada ali e salvava
isso. Não provado, e vale reconferir. De qualquer forma, a guarda de captura (uma
captura que zera inteiramente o conjunto ativo é recusada em qualquer tamanho) significa
que ela não pode mais danificar o perfil.

### `FOPEN_KEEP_CACHE` está desligado

Corrigido. Ele travava o Skyrim SE numa desreferência nula segundos após o menu
principal, de forma determinística, sem nenhum mod instalado; os outros três caches do
lado do kernel foram eliminados um a um por bissecção e só este importava. Essa bissecção
rodou com `FUSE_PASSTHROUGH` ativo, e o kernel faz falhar com EIO uma abertura em
passthrough que carrega `FOPEN_KEEP_CACHE`, então com a flag ligada toda abertura em
passthrough falhava. Talvez tenham sido essas aberturas falhas que travavam o jogo, e a
flag sozinha, com o passthrough desligado, nunca foi testada. As respostas em passthrough
não a carregam mais. Perdê-lo foi medido como gratuito na época, mas com o passthrough
ativo o daemon serve *zero* leituras (`EIDOS_FUSE_STATS` reportou `read 0` numa carga
completa), então com o passthrough desligado por padrão o custo real está sem medição.
Religue com `EIDOS_FUSE_KEEP_CACHE=1` para testá-lo sozinho.

### O passthrough do FUSE impede o jogo de carregar qualquer conteúdo de mod

O daemon agora evita a causa mais provável (abaixo), mas isso ainda não foi medido de
novo em um jogo, então o passthrough continua desligado por padrão;
`EIDOS_FUSE_PASSTHROUGH=1` o liga. Antes dessa mudança, com o passthrough ligado, o
Skyrim SE falhava em abrir 152 dos próprios arquivos com `STATUS_ACCESS_VIOLATION`, então
nenhum conteúdo de mod carregava, em silêncio. A medição e o motivo de os logs do próprio
daemon mostrarem uma execução limpa estão em "Por que o passthrough vem desligado" acima.

O kernel associa um único arquivo de respaldo a cada inode (`fs/fuse/iomode.c`). Uma
abertura que traz um segundo identificador de respaldo para um inode que já tem um falha
com EIO, assim como uma abertura normal (sem `FOPEN_PASSTHROUGH`) de um inode que tem
arquivo de respaldo. Uma abertura em passthrough que carrega `FOPEN_KEEP_CACHE` também
falha com EIO. O daemon registrava um identificador novo a cada abertura, então falhava a
segunda abertura de um arquivo que o jogo já mantinha aberto. O Wine reporta EIO como
`STATUS_ACCESS_VIOLATION`, o erro por trás das 152 aberturas falhas. Essa regra vem do
código-fonte do kernel e bate com essas 152 falhas, o que a torna a causa mais provável.

O daemon agora registra um único arquivo de respaldo por inode, compartilhado por todas as
suas aberturas em passthrough, e nunca põe `FOPEN_KEEP_CACHE` numa abertura em
passthrough. O passthrough só cobre aberturas somente leitura de arquivos das camadas do
jogo e dos mods que tenham 1 MiB ou mais ou terminem em `.dll`, `.exe` ou `.asi`. Uma vez
registrado esse arquivo de respaldo, toda abertura somente leitura do arquivo o usa, e
enquanto o arquivo tiver aberturas normais, os novos leitores também recebem aberturas
normais. Um arquivo do Overwrite nunca passa pelo passthrough, e uma montagem que projeta
os carimbos de data dos plugins não o usa de jeito nenhum. Uma abertura para escrita de um
arquivo que leitores em passthrough mantêm aberto recebe ETXTBSY (no Wine:
`STATUS_SHARING_VIOLATION`) antes de qualquer copy-up, então uma abertura com `O_TRUNC` não
pode deixar no Overwrite uma cópia vazia que esconda o arquivo do mod.

### O botão Mod Manager Download não faz nada

Preferences, Nexus, Downloads mostra qual programa recebe os links `nxm://`. Um cartão
Health aparece quando esse programa não é o Eidos, não existe ou quando nenhum está
definido. O botão Register grava `eidos-nxm.desktop` e o define como manipulador padrão de
`x-scheme-handler/nxm`. Num terminal, `eidos nxm --register` faz o mesmo.

Se o status já indica o Eidos e o botão continua sem fazer nada, o navegador pode estar
usando uma escolha anterior. Apague a escolha salva para links nxm nas configurações do
navegador para que ele pergunte de novo. Um navegador Flatpak não consegue iniciar
programas do host e entrega o link pelo portal da área de trabalho, que lê o manipulador
registrado no host.
