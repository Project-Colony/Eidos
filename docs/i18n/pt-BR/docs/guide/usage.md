<!-- eidos-i18n: source=docs/guide/usage.md sha=15b7f6e81560a7045c8b62dd59dde35c8e45d32c -->

# Usar o Eidos

Como usar o Eidos: instâncias e cópias do jogo, a linha de comando, execuções de
teste, a janela, coleções e atualizações do Nexus, mover uma instância para outra
máquina, ferramentas e extensões. Para problemas, veja
[troubleshooting.md](troubleshooting.md); para compilar a partir do código e o
layout do repositório, veja
[../internals/contributing.md](../../../../internals/contributing.md).

## Instâncias: global e portátil

Os comandos que recebem uma instância aceitam um identificador de jogo ou a pasta de
uma instância portátil. `skyrimse` nomeia a instância global, que o Eidos mantém em
`~/.local/share/Colony/Eidos/instances/skyrimse`. Uma instância portátil é uma pasta
autocontida onde você quiser (um segundo disco, uma partição de jogos), e você pode
movê-la, como as instâncias portáteis do MO2:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # criar uma instância portátil ali
eidos install /mnt/games/EidosSkyrim mod.7z  # todo comando aceita a pasta
eidos play /mnt/games/EidosSkyrim -- %command%
```

O `eidos-instance.ini` da pasta nomeia o jogo, então a pasta não precisa de mais nada.
`EIDOS_INSTANCE=<folder>` no ambiente redireciona um identificador de jogo para essa
pasta, por exemplo numa opção de inicialização do Steam. As instâncias portáteis que
você criou ou abriu são lembradas, a mais recente primeiro, em
`~/.config/Colony/Eidos/instances.ini`: a tela de boas-vindas da janela as lista, o
início pelo Steam cai na que você jogou por último, e o manipulador `nxm://` baixa para
ela.

Mover uma pasta portátil à mão mantém tudo, exceto as entradas de ferramentas que você
registrou com caminhos absolutos para o local antigo. `eidos pack` e `eidos unpack`
([abaixo](#mover-uma-instância-para-outra-máquina)) corrigem essas; um simples `mv`
não. O cache de runtimes em `~/.local/share/Colony/Eidos/runtimes/` é compartilhado por
toda instância da máquina, então um host .NET de 78 MB não é copiado para dentro de cada
uma.

Um lugar é recusado: dentro da pasta de instalação de um jogo, onde usuários do MO2
costumam pôr a instância. O Steam é dono dessa árvore, e uma atualização, um "verificar
integridade" ou uma desinstalação pode reescrevê-la ou apagá-la junto com a sua
configuração. O Eidos também monta sobre a raiz do jogo, então uma instância ali ficaria
dentro do próprio alvo de montagem. O assistente, `eidos init`, `eidos play` e
`eidos shell` a recusam. Ponha a pasta ao lado do jogo; uma pasta irmã no mesmo disco é
igualmente cômoda.

### Onde o Eidos guarda os arquivos dele

O Eidos guarda os próprios arquivos em `Colony/Eidos`, o layout que todo programa da
família Colony usa: `~/.config/Colony/Eidos/` para o que você escolheu (preferências, sua
sessão do Nexus, sua lista de instâncias, as definições de jogos e extensões que você
escreveu), `~/.local/share/Colony/Eidos/` para as instâncias globais (`instances/`), os
runtimes baixados e os logs de sessão (`logs/`), e `~/.cache/Colony/Eidos/` para as
masterlists do LOOT, que são baixadas de novo sempre que faltam.

Quando encontra arquivos no layout de um Eidos mais antigo (`~/.config/eidos/`,
`~/.local/share/eidos/` e `~/.local/state/`), o primeiro início os copia para o novo
lugar e diz isso no log. Uma instância global é movida com links rígidos em vez de uma
cópia de bytes, então os mods dela não ocupam espaço extra, e as entradas de ferramentas
dela são reapontadas para a pasta nova. Nada nos diretórios antigos é alterado nem
apagado; remova-os você mesmo quando o novo layout funcionar. Se uma cópia não puder ser
feita (disco cheio ou somente leitura, uma pasta de instância antiga em outro disco, um
jogo ainda rodando dela), o Eidos continua usando o local antigo e tenta de novo no
próximo início. Uma opção de inicialização do Steam que ainda nomeia a pasta de instância
antiga abre a movida.

Os links rígidos só economizam espaço no começo. Um mod ou download que você remove ou
reinstala depois mantém a cópia antiga no disco pela pasta de instância antiga, e apagar
`~/.local/share/eidos/<game>/` devolve esse espaço.

Uma instância portátil nunca é movida: ela fica onde você a pôs.

## A linha de comando

```sh
eidos games                                # jogos suportados instalados aqui, como a lista do MO2
eidos init skyrimse [folder]               # criar uma instância; com uma pasta ela é portátil
eidos init skyrimse --game-path <folder>   # usar a cópia instalada em <folder> quando várias estão instaladas
eidos install skyrimse mod.7z [name]       # instalar um arquivo (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # reinstalar sobre o mod (--merge instala sobre os arquivos dele)
eidos import skyrimse <mo2-profile>        # assumir a ordem de mods e o estado dos plugins de um perfil do MO2
eidos sort skyrimse [--dry-run]            # ordenar a carga com o LOOT (--update-masterlist a atualiza antes)
eidos play skyrimse                        # mostrar o que seria montado
eidos play skyrimse -- <command>           # executar <command> com os mods montados sobre o jogo
eidos shell skyrimse                       # um shell na visão combinada; as escritas vão para o Overwrite
eidos check skyrimse [--json]              # verificações Health sem a janela
eidos sessions skyrimse                    # as execuções registradas de jogos e ferramentas, a mais nova primeiro
eidos sessions skyrimse diff [<id>]        # o que mudou desde a última execução que saiu com código 0
eidos stock-game skyrimse                  # quais arquivos do jogo a instância executa; congelar uma cópia ou trocar
eidos tool skyrimse                        # listar as ferramentas (add, rm e run: veja Ferramentas abaixo)
eidos tool skyrimse run --exe <path>       # executar um exe uma vez pela visão combinada
eidos tool skyrimse import-mo2 [--write]   # os executáveis de um ModOrganizer.ini; --write os adiciona
eidos prereqs skyrimse [--install]         # o que as ferramentas precisam; --install baixa o que falta
eidos collection skyrimse <link>           # instalar uma coleção do Nexus (--dry-run, --no-optional)
eidos nexus status                         # conferir o login guardado do Nexus
eidos nexus update skyrimse [--apply]      # verificar atualizações de mods; --apply as instala
eidos nxm <link>                           # baixar um link Mod Manager Download
eidos nxm --register                       # fazer o botão Mod Manager Download do site abrir o Eidos
eidos export skyrimse -o mods.csv --active # formato CSV do MO2; --active lista só os mods ativados
eidos pack skyrimse backup.eidos           # a instância inteira num arquivo
eidos unpack backup.eidos [folder]         # sem pasta, uma instância global volta ao lugar central
eidos --version                            # imprimir a versão
```

`eidos` sem argumentos imprime a lista de comandos. A maioria dos comandos imprime as
próprias opções quando falta um argumento (`eidos init` e `eidos import` imprimem a lista
completa), e `eidos tool --help` lista os subcomandos de ferramentas. `eidos pack` aceita
`--dry-run`, `--level`, `--no-downloads`, `--no-recovery` e `--force`, e `eidos unpack`
aceita `--info` e `--force`; os dois estão descritos
[abaixo](#mover-uma-instância-para-outra-máquina). `eidos stock-game` está descrito em
[Arquivos do jogo](#arquivos-do-jogo-a-instalação-da-loja-ou-uma-cópia).

### Executar um jogo pela visão combinada

`eidos play` monta os mods da instância sobre o diretório `Data` do próprio jogo dentro de
um espaço de nomes privado e depois executa o comando por essa visão. Os arquivos do
próprio jogo são antes colocados de lado por bind, então o daemon ainda os lê sem
alteração. As escritas (saves, configs regeneradas) caem na camada `overwrite/` da
instância; a instalação do jogo e toda pasta de mod ficam byte a byte como estavam.

O Eidos não precisa de root. Ele monta num espaço de nomes privado de usuário e de
montagem, sem auxiliar setuid, sem serviço do sistema e sem nada a conceder. Uma capacidade
opcional no binário `eidos` permite o passthrough do FUSE, que continua desligado a menos
que `EIDOS_FUSE_PASSTHROUGH=1` esteja definido; veja
[install.md](install.md#opcional-passthrough-do-fuse). Com a capacidade, o Eidos usa um
espaço de nomes de montagem simples em vez de um de usuário, e os mods são implantados do
mesmo jeito nos dois.

### Um shell na visão combinada

`eidos shell <instance>` inicia o seu `$SHELL` (ou `/bin/sh`) na pasta do jogo como o jogo a
veria: todo mod ativado montado sobre `Data`, os mods Root sobre a raiz do jogo, e tudo que
é escrito ali caindo no Overwrite. `eidos shell <instance> -- <command>` executa um único
comando no lugar, por exemplo `eidos shell skyrimse -- ls Data/meshes` ou uma ferramenta
nativa como o NifSkope. A execução segura o bloqueio da instância como uma sessão de jogo,
então a janela não consegue salvar mudanças até você sair, e é registrada em
`eidos sessions` com o nome `shell`, nunca como um lançamento de jogo. Ctrl+C num comando
interrompe a execução do jeito que Stop faz: o Eidos encerra o comando, captura a sessão e
desmonta. No shell interativo o Ctrl+C pertence ao shell, como de costume; digite `exit`
para encerrar a execução.

### Verificações de saúde pelo terminal

`eidos check <instance>` roda as verificações Health que não precisam de janela, então elas
podem rodar por SSH ou no Game Mode e ir para um relatório de bug:

- o prefixo do Proton e o registro dele;
- diagnósticos de plugins e masters faltando;
- o script extender e seus plugins;
- uma atualização do jogo desde o último lançamento limpo;
- a opção de inicialização do Steam;
- configurações gráficas (NVAPI, tela cheia, ENB);
- o log de falha da última execução;
- linhas `sTestFile` nos INIs do perfil;
- plugins `.esp` e blueprint do Starfield;
- `vm.max_map_count` e o limite de arquivos abertos;
- a cópia do jogo que a instância executa, e um `gamePath` no `ModOrganizer.ini` de uma
  lista que nomeia outra pasta de jogo;
- regras de ordem de coleção que não valem mais;
- arquivos do jogo em NTFS, exFAT ou FAT;
- caminhos de dados do BodySlide;
- pastas `Data` deixadas em `Overwrite/Root`;
- bibliotecas de fontes que `Interface/fontconfig.txt` nomeia e nenhum mod fornece;
- áudio comprimido nos arquivos BA2 de mods de Fallout 4 e Fallout 4 VR.

Alguns cartões só a janela calcula, na aba Diagnostics: a ordem dos patches, os scripts do
extender que um mod substitui, os arquivos que nada carrega, os requisitos do Nexus, os
plugins que o LOOT marca como sujos, a build do Proton que roda o jogo, o cartão Display e a
saída de ferramentas mais velha que suas entradas.

O comando imprime as mesmas linhas de sistema que Copy report, depois uma linha por achado,
com a sua pasta pessoal mostrada como `~` e seu nome de login como `<user>`. O código de
saída é 0 quando nenhum problema foi encontrado (só conselhos ainda dão 0), 1 quando pelo
menos um problema foi encontrado e 2 quando o Eidos não conseguiu verificar: argumentos
errados, uma instância que ele não consegue abrir ou um jogo que ele não detecta.

`eidos check <instance> --json` imprime um único objeto JSON no lugar. Seus campos de nível
superior são `eidos` (a versão), `game`, `instance`, `system` (`proton`, `flatpak_steam`,
`passthrough`, `session`, `distro`, `kernel`, `dev_fuse`, `fusermount3`, `max_map_count`,
`open_files_hard`) e `findings`. `max_map_count` e `open_files_hard` são números, ou `null`
quando o Eidos não conseguiu lê-los. Cada achado é um `code`, um `level` (`problem` ou
`advice`) e seus `args`; a aba Diagnostics mostra a frase de cada código.

### O que mudou desde a última execução boa

Toda execução por `eidos play`, `eidos shell` ou `eidos tool run`, iniciada pela janela, por um
terminal ou pelo Steam, grava um registro em `<instance>/logs/sessions/`: o perfil, cada mod
ativado com sua versão e ids de arquivo do Nexus, a ordem dos plugins, uma soma de verificação
de cada INI do perfil, a versão do executável do jogo, a versão do Eidos, o código de saída e
os saves que a execução criou. Os 50 mais novos são mantidos, e `logs/` fica fora de uma
instância empacotada. `eidos sessions <instance> diff` compara a execução de jogo mais nova que
saiu com código 0 com o estado atual; `diff <id>` escolhe outro registro da lista. A lista marca
uma [execução de teste](#execuções-de-teste) como `test`, e uma execução de teste nunca é
escolhida como a última execução boa. Uma execução que você interrompeu registra o código de
saída 143, então também nunca é escolhida. O Eidos ainda não detecta travamentos aqui, e um jogo
que travou sob o Proton ainda pode sair com código 0: se a execução escolhida foi uma dessas,
nomeie uma anterior com `diff <id>`.

## A lista de mods

Como no MO2, cada perfil tem o próprio `modlist.txt` (quais mods estão ativados, em que
prioridade), enquanto as pastas dos mods vivem no `mods/` compartilhado da instância. Uma pasta
que você larga à mão em `mods/` ainda não está listada: ela aparece desativada na prioridade mais
alta, no fim da lista da janela como no MO2, então vence os conflitos dela quando você a ativa.

Como as pastas são compartilhadas, remover ou renomear um mod muda todo perfil: a linha dele é
removida do `modlist.txt` de todo perfil, ou renomeada nele, e cada perfil mantém o próprio
estado e ordem para o resto. Um perfil que fica sem mods não tem `modlist.txt` nenhum.

A janela trabalha com a própria cópia da lista de mods e da ordem de carga. Se outro processo do
Eidos escreve qualquer uma delas enquanto a janela está aberta (`eidos install` ou `eidos sort`
num terminal, uma coleção, uma sessão iniciada pelo Steam, uma segunda janela), a próxima edição
da janela é recusada em vez de sobrescrever esse trabalho. Ela recarrega do disco e diz *Not
saved: another Eidos process changed the mod list. It has been reloaded - redo the change.* (ou o
mesmo para a ordem de carga); só aquele clique se perde, então faça-o de novo. Rename, New mod,
Add separator, Remove e transformar o Overwrite em mod verificam antes de tocar em `mods/`, então
uma edição recusada não muda nada no disco. O editor de INI também se recusa a salvar sobre um
arquivo que mudou desde que ele o abriu.

## Os saves pertencem ao perfil

Cada perfil guarda os próprios saves em `profiles/<name>/saves/`, e durante uma execução essa
pasta é montada sobre a pasta de saves do jogo no prefixo do Proton. O primeiro início de um
perfil copia para ele, uma vez, os saves que já estão no prefixo, então um save que você apaga
depois do perfil continua apagado. Depois de cada execução, os saves mais novos do perfil são
copiados de volta para o prefixo para o Steam Cloud; um save do prefixo que o Eidos não pôs ali é
antes guardado no perfil como `orphan-<time>-<name>`, nunca sobrescrito.

Um save que cai no prefixo depois dessa primeira cópia sem passar pelo Eidos (um download do
Steam Cloud vindo de um Steam Deck, um início sem o Eidos) fica oculto enquanto os saves do perfil
estão montados. Nada o apaga: o início o nomeia no log da execução (o `logs/run-*.log` da
instância num início pela janela, o console do Steam num início pelo Steam), uma vez cada vez que
esse conjunto de arquivos muda, e a aba Saves conta esses saves na linha sob o título dela. Para
jogar um deles, clique em Import na aba Saves. Import copia cada um com o co-save dele para o
perfil e nunca substitui um save do perfil: um save cujo nome o perfil já usa é importado como
`orphan-<time>-<name>`.

## Execuções de teste

O botão Test run ao lado de Run inicia o jogo como Run faz, mas tudo que o jogo escreve vai para
`<instance>/test-run/` em vez de para o seu perfil, o Overwrite e o prefixo: saves novos e
alterados, mudanças de INI, `plugins.txt`, arquivos escritos em Data ou na pasta do jogo, e os
logs em `Documents/My Games/<game>`. Você pode testar com os mods do perfil, sem nenhum mod, ou só
com os mods selecionados que estão ativados, mais os mods cujos plugins eles precisam como masters.

Quando o jogo sai, o Eidos lista esses arquivos em cinco grupos (saves, mudanças de INI, estado dos
plugins, saída do Overwrite, logs) e pede que você guarde ou jogue fora cada um. Apply espera até
que todo grupo que tem arquivos tenha uma resposta. O estado dos plugins não é oferecido quando a
execução usou um conjunto de mods diferente do do perfil. Guardar saves nunca apaga um save do
perfil, nem um que o jogo apagou durante a execução. Guardar a saída do Overwrite também remove do
Overwrite os arquivos que o jogo apagou, e a lista os conta antes. Jogar qualquer coisa fora exige
um segundo clique. Decide later fecha a lista e deixa os arquivos em `test-run/`, e a lista abre de
novo na próxima vez que você abrir a instância ou apertar F5. Se o Eidos ou o computador parar
durante uma execução de teste, os arquivos também ficam lá e a lista abre do mesmo jeito.

Quando a janela não foi iniciada pela loja do jogo, Test run entrega o jogo a essa loja (Steam,
Heroic ou Legendary), como Run faz, e a execução que a loja inicia é a execução de teste. Se a loja
não inicia o jogo, o pedido fica esperando: a próxima execução de jogo desse perfil, por Run ou pela
loja, é a execução de teste, e Cancel the waiting test run a descarta. Uma execução de teste cobre
só as pastas listadas acima: outras escritas do jogo, como o registro do Wine, vão para onde sempre
vão. Saves de uma execução de teste só chegam ao Steam Cloud depois que você os guarda e joga uma
execução normal.

### Encontrar o mod por trás de um problema

Find the mod behind a problem, na aba Diagnostics, procura o mod que causa um problema que você
consegue reproduzir, como um travamento num certo lugar. Cada passo é uma execução de teste com
metade dos suspeitos restantes, os masters de que eles precisam e os mods que você mantém sempre
ativados: os mods ativados selecionados na lista quando você começa e o mod que fornece o script
extender. Depois de cada execução você responde se o problema aconteceu. Not sure divide os
suspeitos de outro jeito, e Run it again repete o passo. Responder, ou clicar em Run it again,
descarta os arquivos que o passo gravou, depois de um segundo clique. Para manter alguns, clique
antes em See the files: ele abre a mesma lista que aparece depois de uma execução de teste. Quando
o jogo travou e um registrador de falhas gravou um log, ou o jogo saiu com um código de erro, o
Eidos marca Yes como a resposta provável. 200 suspeitos levam cerca de 8 execuções.

A busca é guardada em `<instance>/bisect.json`, então você pode fechar o Eidos entre os passos, e ela
pertence ao perfil em que começou. Stop the search a descarta depois de um segundo clique. O
resultado nomeia o mod, ou o conjunto de mods que precisam dos masters uns dos outros, e é guardado
em `<instance>/logs/bisect-<time>.txt`. Mods gerados (uma saída do Nemesis ou do DynDOLOD, um Bashed
Patch) são feitos para a lista completa e podem falhar com metade dela: mantenha-os sempre ativados,
ou deixe-os desativados durante toda a busca.

## Listas do Wabbajack

O Eidos não tem instalador do Wabbajack. Instale a lista com o CLF3 ou o Jackify, fora do Eidos, e
depois escolha a pasta de instalação como instância portátil no assistente. O Eidos lê a pasta como
o MO2 a organiza: `mods/`, `profiles/<name>/modlist.txt` e as listas de plugins. Ele mantém o
`ModOrganizer.ini` que o instalador escreveu e nunca o sobrescreve, então os executáveis próprios da
lista continuam registrados.

Muitas listas adicionam uma pasta `Stock Game` ou `Game Root` que guarda uma cópia modificada do
jogo, e o MO2 executa o jogo dali. Quando o assistente ou `eidos init` adota uma pasta de lista pela
primeira vez e essa pasta guarda o executável do jogo, a instância também executa o jogo dela. O
jogo ainda inicia pelo Steam ou pela loja que é dona dele, então nada precisa ser movido. O Eidos
acha a pasta pelo `gamePath` do `ModOrganizer.ini` da lista, ou por esses dois nomes. Uma instância
adotada por um Eidos anterior mantém a instalação da loja até você trocá-la (veja a próxima seção).
Quando esse `gamePath` nomeia uma pasta de jogo que a instância não executa, o Diagnostics diz isso,
porque ferramentas no modo Mod Organizer 2, como o PGPatcher, leem o jogo dali. O CLF3 reconstrói
arquivos BSA e DDS com ferramentas próprias, então o resultado pode diferir de uma instalação do
Wabbajack, e os autores de listas não o suportam.

## Arquivos do jogo: a instalação da loja ou uma cópia

Por padrão o jogo roda da instalação da loja, a pasta onde o Steam, o Heroic ou o Legendary o
guardam. Uma instância pode, em vez disso, rodar de uma cópia dessa pasta dentro da instância. Os
mods ficam sobre a cópia, e a instalação da loja é só o lugar onde o Eidos monta o resultado. A loja
ainda inicia o jogo, então o Steam mantém o overlay, o cache de shaders e os saves na nuvem.

O cartão Game files, no topo da aba Diagnostics, mostra quais arquivos a instância executa, com a
versão do jogo da cópia e da instalação da loja, e alterna entre eles. Num terminal,
`eidos stock-game <instance>` imprime os mesmos fatos, e `use <folder>`, `store`, `freeze` e
`delete --yes` fazem o que os botões do cartão fazem. `freeze` imprime o tamanho e o tipo de cópia, e
`freeze --yes` copia.

Freeze this version copia a instalação da loja para `<instance>/Stock Game` e executa o jogo dessa
cópia, então uma atualização da loja não quebra mais os plugins do script extender: a atualização
muda a instalação da loja, não a cópia. O primeiro clique mostra o tamanho e o que o sistema de
arquivos faz com ele, e o segundo copia. Num sistema de arquivos que pode compartilhar blocos
(reflink), como btrfs ou XFS, a cópia quase não ocupa espaço até a loja alterar um arquivo. Em ext4,
ou quando o jogo e a instância estão em sistemas de arquivos diferentes, é uma cópia completa do
tamanho da pasta do jogo. O Eidos recusa enquanto o Steam informa que o jogo está ocupado ou não
totalmente instalado, enquanto um jogo ou ferramenta da instância roda, quando a instância já guarda
uma cópia de jogo de uma lista, uma cópia congelada ou uma inacabada, e quando o sistema de arquivos
tem menos espaço livre do que uma cópia completa precisa. Cópias do Heroic e do Legendary não são
verificadas quanto a uma atualização em andamento. Stop, uma falha ou uma atualização do jogo pelo
Steam durante a cópia deixa `Stock Game.eidos-tmp`, que o cartão se oferece para apagar; ele nunca é
usado.

Uma cópia esconde todo arquivo que a loja adiciona, altera ou remove na pasta do jogo depois de a
cópia ser feita: novos arquivos do Creation Club, um executável novo, DLLs novas. O jogo, a lista de
plugins, o LOOT e as verificações Health leem a cópia. Ela não esconde o que acontece fora da pasta
do jogo: uma atualização ainda pode instalar runtimes no prefixo do Proton no próximo início, e o
Steam pode trocar o jogo para outro Proton.

Return to the store version tira a cópia do próximo início depois de um segundo clique. A cópia
continua no disco, e Delete the copy a remove, também depois de um segundo clique. Delete é recusado
enquanto a instância ainda executa a cópia. O Eidos nunca apaga a pasta `Stock Game` ou `Game Root`
própria de uma lista. Se a pasta da cópia sumir, ou deixar de guardar o executável do jogo, os inícios
são recusados e o Diagnostics nomeia a pasta, então a versão da loja nunca roda de surpresa.

Morrowind, e Oblivion com `bUseMyGamesDirectory=0`, guardam a lista de plugins na pasta do jogo, e o
Eidos ainda não os executa a partir de uma cópia.

## Instalar uma coleção do Nexus

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # ler e dizer o que aconteceria
eidos collection skyrimse rqhcxy --no-optional  # pular os membros que ela marca como opcionais
```

Na janela, cole o link em File -> Open a Nexus collection..., ou clique num no site. O Eidos lista os
membros da revisão, cada um comparado com esta instância: instalado, baixado ou faltando. *Install this
collection* executa a receita inteira numa thread de trabalho, com a barra de progresso que os trabalhos
de pack e unpack usam, e a barra vira o relatório quando termina. A janela e `eidos collection` executam
a mesma instalação.

Sem link, File -> Open a Nexus collection... abre na aba Browse. Ela lista as coleções publicadas para o
jogo da instância aberta, 20 por vez, ordenadas por mais baixadas, atualizadas recentemente ou mais
novas, e a caixa de busca reduz a lista às coleções cujo nome ou texto combina. Cada linha dá o autor, a
revisão mais recente, o número de mods, os downloads e uma data. View abre a revisão mais recente dessa
coleção na aba Link, igual a colar o link dela. Coleções marcadas como adultas ficam de fora a menos que
a sua conta do Nexus mostre conteúdo adulto, coleções sem avaliação ficam sempre de fora, e a linha sob a
lista conta as duas. Navegar exige uma conta do Nexus com login feito, como o resto do painel.

Uma coleção só abre contra uma instância do próprio jogo. Uma coleção de Skyrim aberta enquanto uma
instância de Fallout 4 está carregada é recusada pelo nome, em vez de comparar os membros dela com a
lista de mods errada.

### O que o Eidos lê de uma coleção

Quase nada de uma coleção é visível pela API do Nexus. A ordem de instalação, as respostas que o autor
deu ao instalador com script de cada mod, qual mod vence um conflito de arquivo, quais plugins carregam
onde e os patches binários estão todos num `collection.json` dentro do arquivo da própria coleção.
Baixar os mesmos mods e instalá-los com as opções padrão dá um jogo diferente.

O Eidos baixa o arquivo, lê o `collection.json` e o aplica:

| A coleção diz | O Eidos faz |
| --- | --- |
| `phase` | instala nessa ordem, com os membros opcionais por último |
| `choices` | repete as respostas do autor a cada FOMOD, e avisa quando não consegue |
| `modRules` | reordena a lista de mods para que o mod certo vença cada arquivo |
| `plugins` / `pluginRules` | funde as regras do LOOT no seu userlist e ordena |
| `INI Tweaks/` | instala como um mod à parte |
| `tools` | diz quais ferramentas ele espera; não cria nada |

Os membros são ativados à medida que se instalam, no fim da lista de mods na ordem de instalação, antes
de `modRules` os mover. Um membro deixado fora da lista de mods não carregaria, e o relatório não o
mostraria.

O estado é gravado em `<instance>/collections/<slug>-<revision>.state.json` depois de cada membro. Ele
fica ao lado da pasta dessa revisão, nunca dentro dela, então uma coleção não pode trazer um arquivo de
estado próprio, e uma instalação interrompida no membro 147 de 200 continua do 147. É gravado com um
nome temporário e renomeado no lugar, então uma interrupção deixa o registro antigo ou o novo e nunca
metade de nenhum dos dois. Se um dia ele não puder ser lido, o Eidos para e diz isso em vez de recomeçar
a coleção inteira. Execute o mesmo comando de novo.

Instalar uma revisão mais nova de uma coleção que você já tem assume as pastas da revisão anterior: um
membro inalterado é verificado no lugar, um membro cujo arquivo o autor atualizou é substituído dentro da
pasta antiga, e um membro que a nova revisão removeu fica instalado e nomeado no relatório para você
desativar ou remover. Uma pasta que outro perfil ativa nunca é assumida, porque a pasta de mods é
compartilhada e esse perfil mudaria debaixo de você: a nova revisão instala a própria cópia ao lado, e o
relatório pede que você desative a antiga neste perfil, nunca que a remova. Voltar a uma revisão mais
antiga recupera as pastas dela do mesmo jeito, ou instala cópias novas das que não consegue assumir.
Sempre que um membro é substituído desse jeito e você tinha alterado ou adicionado arquivos na pasta dele,
a pasta antiga é antes guardada ao lado como `<name>_backup`, que nunca é carregada no jogo, e o relatório
a nomeia.

Uma sobra de uma revisão anterior que continua ativada impede que a instalação conte como a que o autor
construiu; o aviso some quando você a desativa. Um membro que você pulou com `--no-optional` não é
assumido e é nomeado do mesmo jeito. Se a própria tomada falhar, a execução para com *previous revision:
its folders could not be taken over* e o motivo; conserte isso e execute o comando de novo.

### Contas gratuitas e membros que você baixa por conta própria

Uma conta gratuita do Nexus não consegue baixar os mods membros. O Nexus não dá links de download às
contas gratuitas pela API: todo arquivo precisa do botão "Mod Manager Download" do próprio site. O
arquivo da coleção baixa numa conta gratuita, então você pode ler a receita inteira, mas cada membro é
reportado com a URL exata da página dele para você baixar por conta própria, e tentar de novo não muda
isso. Baixe-os pelo botão do site e execute o comando de novo: esses membros são procurados em
`downloads/` a cada execução, então a coleção termina à medida que você os fornece.

Na janela, uma conta gratuita recebe *Fetch the N missing required files, one by one* no painel da
coleção no lugar do download em lote. Ele abre a página do primeiro arquivo obrigatório que falta no
Nexus, espera esse arquivo cair em `downloads/` e depois abre o próximo. Você ainda clica no botão de
download da página para cada arquivo. O painel mostra até onde foi (*3 of 12*), com *Skip* e *Stop*.
Quando outro arquivo do mesmo mod cai no lugar, o painel nomeia os dois ids de arquivo e continua
esperando o fixado. Os membros opcionais não fazem parte disso e mantêm o próprio botão *Open*.

O Eidos também aplica patches binários (cada um verificado contra o CRC registrado), sobreposições de
arquivos (os arquivos nomeados são ocultados com `.mohidden`, como o MO2 faz) e membros cujos arquivos
viajam dentro do arquivo da coleção (`bundle`). Membros que a coleção espera que você baixe à mão
(`browse`, `manual`) levam as instruções do próprio autor até o relatório. Um membro cujo nome já é um mod
seu é instalado ao lado dele sob um nome livre, nunca sobre ele, e o relatório diz qual.

### O relatório

Um membro cujas respostas de instalador não puderam ser todas repetidas ganha uma linha própria,
*installed, but not the way the collection asks*, à parte dos membros que se instalaram como o autor os
construiu, porque os arquivos dele no disco diferem dos do autor. O relatório também lista regras de
ordenação que não nomearam nada na coleção, mods presos num laço de regras contraditórias, grupos do LOOT
que não existem e qualquer parte do manifesto que esta versão do Eidos não entende.

### Manter a ordem da coleção

O Eidos mantém as regras de cada coleção instalada depois da instalação. Quando você depois move dois dos
mods dela contra uma regra de antes ou depois, a aba Diagnostics mostra um cartão para essa coleção,
nomeando o primeiro par assim e contando as regras que não valem mais. Só contam os mods ativados no
perfil atual, e regras presas num laço são puladas, porque nenhuma ordem consegue cumpri-las. Restore
collection order move os mods da coleção de volta para uma ordem que cumpre toda regra, mantém a sua
própria ordem onde as regras permitem e deixa onde estão os mods que não fazem parte da coleção. Ctrl+Z
desfaz. `eidos check` lista o mesmo achado como `collection_order_broken`. A verificação lê a receita da
revisão em `collections/`, então uma coleção cuja receita em cache foi removida não é verificada.

## Mover uma instância para outra máquina

`eidos pack` grava uma instância inteira (todo mod, a ordem de carga, todos os perfis, o Overwrite com seus
saves dentro e os arquivos de que tudo foi instalado) num único arquivo. `eidos unpack` a coloca de volta:

```sh
eidos pack skyrimse ~/backup.eidos                  # ou dê uma pasta: nomeada com o jogo e a data
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # na outra máquina
```

`eidos pack --dry-run` lista exatamente o que entraria, o que não entraria e por quê, e quanto espaço ele
precisa, sem gravar nada. `eidos unpack --info` faz o mesmo para um arquivo que você já tem.

Se o 7-Zip não conseguiu ler algo, ou a varredura precisou pular algo que você esperaria num backup (um
link simbólico, uma pasta que não conseguiu ler, um nome que o 7-Zip não pode receber com segurança), o
arquivo ainda é gravado com o nome que você escolheu, mas `eidos pack` sai com 1 e diz o que falta, então
um script que executa `eidos pack ... && ...` para ali.

Um arquivo `.eidos` é um arquivo 7-Zip com extensão própria. O 7-Zip já é exigido para instalar um mod,
então isso não acrescenta dependência, e o backup abre sem o Eidos em qualquer compactador que leia 7-Zip.
Ele é comprimido com LZMA2 em `-mx1`, para cerca de 43% do original numa lista pesada em texturas; `-mx9`
leva muito mais tempo e poupa alguns pontos a mais. Texturas e malhas já são formatos comprimidos, então um
dicionário maior acha muito pouco a comprimir. `--level` (0, 1, 3, 5, 7 ou 9) escolhe outro nível, e
`--no-downloads` deixa `downloads/` de fora.

No nível 1 padrão os arquivos vão para blocos sólidos de no máximo 16 MB. Tirar um arquivo de um backup de
70 GB decodifica no máximo o bloco em que ele está, nunca tudo que vem antes, e empacotar conteúdo de jogo
roda de 2 a 5 vezes mais rápido do que com cada arquivo comprimido separadamente (desempacotar, de 2 a 4
vezes). O custo está no dano: um byte corrompido pode levar junto todo arquivo do bloco dele, até 16 MB de
arquivos vizinhos. Os dados de recuperação descritos abaixo cobrem isso. Todos os outros níveis continuam
não sólidos: cada arquivo é o próprio fluxo, e um byte corrompido custa só aquele arquivo. O nível 0 apenas
armazena os arquivos. A partir do nível 3, o LZMA2 já trabalha em pedaços de 16 MB ou mais, e blocos sólidos
medidos ali foram mais lentos: no nível 5, empacotar malhas levou 1,4 vez o tempo e desempacotar texturas
6,8 vezes.

Empacotar segura o bloqueio da instância, então não pode rodar enquanto o jogo ou outro Eidos escreve nessa
instância, e nada muda a instância enquanto ela é lida. O backup é gravado com um nome `.part`, descarregado
no disco e relido por inteiro com `7z t` antes de receber o nome que você escolheu. Essa verificação decodifica
todo bloco e compara toda soma de verificação, o mesmo trabalho que um unpack faz na outra máquina. Ela mostra
o que o disco devolveu logo após a escrita, não que a unidade vá guardá-lo; os dados de recuperação abaixo são
para isso. Se ela falha, o `.part` é apagado, nada é gravado com o seu nome e `eidos pack` cita a linha em que
o 7-Zip achou o dano. Um empacotamento interrompido ou falho deixa um arquivo `.part` ou nada, nunca um arquivo
`.eidos` que parece completo e não é.

Quando o par2cmdline está instalado (`sudo pacman -S par2cmdline` no Arch, `sudo apt install par2` no Debian e
no Ubuntu, `sudo dnf install par2cmdline` no Fedora), `eidos pack` também grava dados de recuperação PAR2 ao
lado do arquivo: `backup.eidos.par2` e alguns arquivos `backup.eidos.volNN+MM.par2`, 5% do arquivo mais um
pequeno índice. Mantenha-os na mesma pasta do arquivo `.eidos`, com os nomes com que foram gravados, e copie-os
junto: um backup movido sem eles não tem de onde ser reparado, e os dados de recuperação registram o nome do
arquivo, então um backup renomeado não combina mais com eles. `eidos unpack` os acha pelo nome e confere o
arquivo contra eles antes de extrair qualquer coisa, inclusive um backup cujo fim foi cortado de modo que o
7-Zip não consegue abri-lo. Se alguns bytes estragaram, o par2 repara o arquivo primeiro e guarda a cópia
danificada como `backup.eidos.1`, o que exige espaço livre do tamanho do arquivo na pasta dele; um dano além do
que os 5% cobrem para o unpack antes de um único arquivo ser gravado. Arquivos de recuperação que não combinam
com o arquivo, como os de um backup mais antigo com o mesmo nome, não são usados quando o 7-Zip lê o arquivo e o
acha intacto, e o unpack diz isso. Numa máquina sem par2 o unpack ainda roda, com um aviso de que os dados de
recuperação não foram usados. `--no-recovery` empacota sem eles, e `eidos pack --dry-run` diz se eles serão
gravados. `--force` substitui um arquivo existente com o mesmo nome.

### O que não está nela, e por quê

| Deixado de fora | Por quê |
| --- | --- |
| O prefixo do Proton | Ele tem a forma da máquina (mapeamentos de unidade, uma visão `Z:` *deste* sistema de arquivos) e o Eidos o reconstrói em um minuto com `eidos prereqs <instance> --install`. Levá-lo praticamente dobraria o arquivo para transportar algo que a outra máquina tem de regenerar de qualquer jeito. |
| O jogo | Uma instância modifica um jogo que ela não contém. Instale-o pelo Steam do outro lado. |
| Ferramentas fora da instância | xEdit, DynDOLOD e outras ferramentas são binários de terceiros com instaladores próprios. Elas são nomeadas no arquivo, então desempacotar diz exatamente quais faltam na máquina nova. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Registros da máquina que fez o backup. `prereqs.done` em particular alegaria que as bibliotecas de runtime já estão instaladas num prefixo que não está lá. |
| `loot/`, exceto `userlist.yaml` | Um cache de masterlist que o Eidos busca de novo sob demanda. Suas próprias regras do LOOT em `userlist.yaml` não podem ser buscadas de novo, então esse único arquivo entra. |
| `.base/`, `.base-root/` | Pontos de montagem vazios onde os arquivos do próprio jogo são guardados durante uma sessão. |
| `test-run/`, `bisect.json` | Uma [execução de teste](#execuções-de-teste) esperando revisão, e uma busca pelo mod por trás de um problema entre dois passos. Os dois pertencem a execuções nesta máquina. |
| Arquivos pela metade | Um download pausado (`*.unfinished`), uma escrita atômica em andamento (`*.eidos-tmp*`). |
| Links simbólicos | O 7-Zip seguiria um e copiaria o que ele aponta, o que para um link absoluto significa puxar uma árvore alheia para o seu backup. Eles são reportados em vez disso, e o backup conta como incompleto: um `mods/` ou `downloads/` ligado a outro disco não é empacotado. Para fazer backup sem um `downloads/` ligado de propósito, passe `--no-downloads` (ou desmarque *Include downloads/* na janela); uma exclusão deliberada não conta como falta. |

Uma cópia do jogo na instância (`Stock Game`) é empacotada como qualquer outra pasta, então o backup de uma
instância congelada fica maior mais ou menos o tamanho do jogo. Uma cópia inacabada (`Stock Game.eidos-tmp`)
fica de fora.

Cada um desses itens é listado com o motivo em `eidos-backup.ini` na raiz do arquivo, então ler esse arquivo
mostra o que foi deixado de fora. O mesmo arquivo registra onde a instância morava, qual perfil estava ativo,
quanto ela ocupa desempacotada e de quais ferramentas a máquina nova vai precisar.

Os registros de download (`downloads/*.meta`) entram com a linha `url=` esvaziada. Essa URL é um link assinado
do Nexus que deixa de funcionar em horas e carrega o `user_id` da conta que baixou o arquivo, e um backup pode
ser entregue a outra pessoa. O id do mod, o id do arquivo e a versão ficam, então o download pode ser achado de
novo.

### Na janela

Os dois estão no menu File: *Pack this instance...* e *Unpack a backup...*. Desempacotar também está na tela de
boas-vindas, sob a lista de instâncias, para que uma máquina ainda sem instância possa desempacotar uma.

Antes de qualquer coisa ser gravada, o diálogo Pack mostra quantos arquivos, que tamanho, quanto disso é
`downloads/`, mais ou menos quanto o arquivo vai dar e quais ferramentas são nomeadas mas não levadas. A caixa
*Write recovery data* vem marcada quando o par2 está instalado, e esmaecida com o pacote a instalar quando não
está. O diálogo Unpack lê o manifesto do backup assim que você escolhe o arquivo, então pode dizer qual jogo ele
guarda, quando foi feito, onde morava e quais das ferramentas dele faltam aqui, antes de você escolher uma pasta.

Os dois rodam numa thread de trabalho com uma barra de progresso, e a barra vira o relatório quando terminam.

### O que o desempacotamento conserta

Uma cópia simples da pasta deixaria todo botão de ferramenta apontando para um caminho da máquina antiga.
`eidos unpack` reescreve os valores que nomeiam a raiz antiga da instância, e só esses: `exe`, `workdir` e as
chaves `arg` numeradas em `tools.ini`, e `installationFile` no `meta.ini` de cada mod. Uma ferramenta que nunca
esteve dentro da instância (`/opt/xedit/SSEEdit.exe`) fica exatamente como estava e é listada como algo a
reinstalar; o Eidos não adivinha onde essa ferramenta está na máquina nova. Ele também corrige se a instância se
chama central ou portátil, para que os comandos seguintes a procurem onde ela está agora.

Desempacotar recusa uma pasta que não está vazia (passe `--force` se você quer sobrescrever), um disco pequeno
demais para a própria cifra do manifesto, um arquivo feito por um Eidos mais novo que o seu e qualquer arquivo
que guarde uma entrada que seria gravada *fora* da pasta escolhida. Sem pasta, um backup de uma instância global
volta ao lugar central do jogo dela; um portátil precisa de uma pasta.

Depois:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # reconstruir o prefixo do Proton
eidos play /mnt/games/EidosSkyrim
```

## A janela

Inicie o `eidos-gui`, ou o arquivo `eidos-linux` que a Colony instala.

O primeiro início abre um assistente como o do MO2: boas-vindas, tipo de instância (portátil ou global), jogo,
nome e local, resumo e depois a janela principal. A tela de boas-vindas também lista toda instância conhecida,
global e portátil, a última usada primeiro, para abrir com um clique, então serve também de alternador de
instâncias. Apontar o assistente para uma pasta que já guarda uma instância abre essa instância como está em vez
de criar uma nova por cima, e uma pasta que pertence a outro jogo é recusada.

A janela principal tem dois painéis: a lista de mods e as abas Data, Plugins, Conflicts, Overwrite, Archives,
Saves, Downloads e Diagnostics, com um botão Run, um botão Test run ([Execuções de teste](#execuções-de-teste)) e
um seletor de alvo de execução. Você filtra a lista de mods, seleciona, reordena, agrupa com separadores, restringe
por categoria e clica com o botão direito para as ações. Além de mandar para o topo e para o fim, o menu do mod
tem os movimentos direcionados do MO2: mandar acima do primeiro mod em conflito, abaixo do último, para uma
prioridade dada ou para dentro do grupo de um separador.

O seletor de perfis troca de perfil. Clique com o botão direito num perfil para Rename, Copy to new..., Desktop
shortcut, Import from MO2... e delete, que leva dois cliques e recusa o perfil ativo ou o último. Import from
MO2... assume a ordem de mods e a ordem de carga de um perfil do MO2. Desktop shortcut grava uma entrada de
desktop que executa `eidos tool <instance> run "<game>" --profile <name>`: ela torna esse perfil o ativo e pede à
loja que inicie o jogo (detalhes em
[tools.md](tools.md#chegar-a-uma-ferramenta-esconder-fixar-e-um-atalho-no-desktop)).

O menu File também executa winecfg, regedit, o explorador de arquivos do Wine e o desinstalador no prefixo do
jogo, e Prefix drive_c abre a unidade `C:` do prefixo; veja
[tools.md](tools.md#os-programas-próprios-do-wine-no-prefixo-do-jogo). *Instances...* lista as instâncias que o
Eidos conhece; Forget tira uma dessa lista e não toca em nada no disco. *Export the mod list...* grava a lista no
formato CSV do MO2, como `eidos export` faz.

### Preferências

Clique em Eidos, no canto superior esquerdo, para abrir as Preferências no lugar dos dois painéis, e clique de novo
(ou em Close) para voltar. General, Appearance e Accessibility vêm primeiro, como em todo programa da Colony, depois
Mod list, Nexus e About. Toda mudança é aplicada e salva na hora; não existe botão Save.

- General: o jogo que o Eidos abre, o tamanho da janela, o idioma da interface, a pasta de ferramentas e como um
  jogo é executado. Lock the window while a game runs vem ligado por padrão. Start the game at once when a store
  opens Eidos, e a notificação da área de trabalho que avisa quando um trabalho longo termina com a janela em
  segundo plano, vêm desligados por padrão. A janela fala inglês e francês; System segue `LC_ALL`, `LC_MESSAGES` ou
  `LANG`, e a mudança vale na hora. A linha de comando `eidos` sempre responde em inglês.
- Appearance: o tema (26 famílias, 59 paletas, o pergaminho do Eidos por padrão), a cor de destaque, o tamanho do
  texto, os efeitos e uma prévia ao vivo.
- Accessibility: alto contraste, uma fonte amigável para dislexia, movimento reduzido e um tamanho de texto de
  leitura que se multiplica pelo de Appearance.
- Mod list: a velocidade da rolagem automática enquanto um mod é arrastado para a borda de cima ou de baixo da lista.
- Nexus: login, a pasta de downloads, o manipulador `nxm://` e seu botão Register, servidores preferidos, Install
  downloads when they finish e Offline mode.
- About: onde ficam os arquivos, os atalhos de teclado, a versão e a verificação de versões.

A aparência e o idioma da interface ficam em `~/.config/Colony/Eidos/preferences.json`. As outras configurações, que
a linha de comando também lê (o jogo padrão, a pasta de ferramentas, os servidores do Nexus, o modo offline e o
resto), ficam em `settings.ini` ao lado dele, e a sessão do Nexus em `nexus.ini`. O tema que um Eidos mais antigo
guardava em `settings.ini` é trazido na primeira vez.

About -> Version guarda a verificação de versões, ligada por padrão. Ao iniciar, no máximo uma vez por dia, o Eidos
pergunta ao GitHub qual é a última versão de `Project-Colony/Eidos`. Quando ela é mais nova que a cópia em execução,
a barra de menus mostra `Eidos <version> is available: release notes` depois de Help, e o link abre a página dessa
versão com o changelog. O Eidos nunca se baixa nem se substitui sozinho. A verificação é pulada no modo offline. Ela
fica desligada para uma cópia instalada em `/usr` por um gerenciador de pacotes, que é quem a atualiza; `/usr/local`,
onde `install.sh --system` coloca o Eidos, conta como uma instalação sua.

### Colunas, ordenação e agrupamento

A lista mostra quatro colunas por padrão (Category, Content, Version e Flags, as que o MO2 mostra) e oferece oito:
Category, Content, Version, Author, Installed, Nexus id, Game, Flags. Marque-as no menu View. Com as oito ligadas, a
coluna do nome fica com pouco espaço.

Clique num cabeçalho para ordenar por ele, de novo para inverter e uma terceira vez para voltar à ordem de carga. A
lista só pode ser arrastada na ordem de carga: uma posição de soltura endereça a lista real, e sob uma ordenação o
lugar de uma linha na tela não é o lugar dela nessa lista. Enquanto uma ordenação está ligada, as faixas de inserção
não são desenhadas e um arrasto é recusado, como no MO2. O menu View diz isso e oferece o caminho de volta.

O menu View também pode agrupar a lista inteira, por categoria ou por origem (do Nexus, ou instalado à mão). Os
cabeçalhos de grupo não são separadores: não podem ser renomeados, coloridos nem movidos. Eles dobram, e um cabeçalho
dobrado mantém a contagem. Os separadores ficam ocultos enquanto uma ordenação ou um agrupamento está ligado, porque um
separador encabeça as linhas que o seguem na ordem de carga, e os dois mudam essa ordem na tela.

### Mouse e teclado

Clique duplo num mod para Information, Ctrl+clique duplo para a pasta dele, Shift+clique duplo para a página dele no
Nexus. Ctrl+F põe o cursor na caixa de filtro. Digitar uma letra salta para o próximo mod que começa com ela, e
apertá-la de novo percorre o resto em vez de ficar no primeiro. Nenhum deles cai numa linha que o filtro, um separador
dobrado ou um grupo dobrado esconde, então o próximo Space nunca alterna um mod que você não vê.

"Collapse others" no menu de um separador dobra todo grupo menos aquele. Durante um arrasto, segurar o ponteiro parado
sobre um grupo dobrado o abre, para que o mod possa ser solto dentro; passar por cima não abre.

Ctrl+Z desfaz a última mudança na lista de mods ou na ordem de carga: um visto, um movimento, uma desativação em lote,
uma ordenação do LOOT. Ctrl+Shift+Z refaz. A janela guarda 50 passos e os esquece em F5, no fim de uma execução de jogo
ou ferramenta, numa troca de perfil e quando outro processo do Eidos muda a lista. Instalar, renomear ou remover um mod
muda as pastas no disco, então um desfazer além de um desses é recusado.

### Filtrar a lista

O botão Filters abre uma linha por critério: active, conflicted, update available, no category, endorsed, tracked, has a
Nexus id, has hidden files, no valid game data, for another game, has notes, e uma linha por tipo de conteúdo (plugins,
arquivos BSA ou BA2, texturas, malhas, scripts, plugins de script extender, arquivos de interface, som, FaceGen). Um
clique alterna uma linha entre off, only e except. A linha no topo alterna entre combinar com todos os critérios ligados
e combinar com qualquer um. A caixa de nome e a lista de categorias sempre reduzem mais o resultado. A linha de arquivos
ocultos olha só os mods ativados.

### O que a lista diz sobre um mod

Dois avisos, ambos um glifo com a explicação ao passar o mouse. No valid game data significa que nada no topo do mod se
parece com algo que este jogo carrega; ele pode precisar que as pastas subam um nível, ou pode não ser um mod para este
jogo. Another game significa que o próprio `meta.ini` do mod nomeia outro. Nenhum dos dois bloqueia nada: o mod ainda é
implantado. "Mark as valid" no menu da linha silencia qualquer um pela chave `validated=` do próprio MO2, então um mod
marcado como válido num gerenciador não é sinalizado no outro.

A verificação de layout só sinaliza um mod quando tem certeza: uma árvore `Root/`, uma pasta ilegível e uma pasta vazia
contam todas como dados de jogo válidos.

### Desativar um mod que seus saves ou outros mods usam

Antes de um mod ser desativado ou removido, o Eidos lê as listas de plugins dos três saves mais novos do perfil. Quando um
deles usa um plugin desse mod, uma barra acima da lista nomeia os plugins e espera por Disable anyway ou Cancel (Escape
também cancela). Remove acrescenta os mesmos nomes à mensagem de clicar de novo. Só saves de Skyrim e Fallout 4 são lidos;
os outros jogos desativam sem perguntar.

A aba General de Information de um mod lista os mods de que ele precisa e os mods que precisam dele, lidos dos masters dos
plugins ativados. Desativar um mod de que outros precisam mostra a mesma barra com Disable them too, que também desativa
todo mod que precisa dele e os mods que precisam desses, ou Keep them enabled. O cartão de masters faltando na aba
Diagnostics tem Enable what provides them: ele ativa o mod que guarda cada master faltando e marca um plugin master que
está presente mas sem visto.

### Fazer backup de um mod antes de mexer nele

"Back up this mod" copia a pasta dele para o lado como `<name>_backup` (depois `_backup2`, e assim por diante; um backup
nunca substitui o anterior). A cópia não é um mod: a caixa dela não faz nada e ela não acrescenta nada à visão combinada,
porque marcá-la implantaria duas cópias de um mod uma sobre a outra. "Restore this backup over the mod" a devolve, em dois
cliques; o conteúdo atual é antes posto de lado e só descartado depois de a cópia ter dado certo.

Backups, na barra de ferramentas, guarda pontos de restauração da lista de mods do perfil, da ordem de carga dele
(`plugins.txt` e `loadorder.txt` juntos) e dos INIs do jogo, 10 de cada, em `<profile>/backups/`. Back up now adiciona um, e
Restore faz antes um backup do estado atual, então uma escolha errada pode ser desfeita. O Eidos também guarda um ponto de
restauração de INIs antes de cada execução do BethINI (veja [tools.md](tools.md#bethini-pie)).

### As abas

Data é uma árvore da visão combinada, expandida um nível por vez: abrir uma pasta lê essa pasta em cada camada que a tem,
nunca todo mod ativado de uma vez. Ela é construída a partir da mesma pilha de camadas que a montagem serve, então
whiteouts e arquivos ocultos são respeitados e a aba mostra o que o jogo vai ver. Filtre por nome, reduza a só os arquivos
disputados, descubra o que está onde com as colunas Size e Modified e use Reveal em qualquer linha num gerenciador de
arquivos. Passar o mouse sobre um arquivo lista os outros mods que o trazem. Os `meta.ini`, `readme.txt`, `fomod/` e
`Root/` de nível superior de um mod ficam fora de Data, nesta aba e na montagem: os três primeiros são para o gerenciador
de mods, e `Root/` vai para a pasta do jogo. Um arquivo que o jogo ou uma ferramenta escreve sob um desses nomes cai no
Overwrite e continua visível.

Uma linha de `.exe` em Data tem Add tool, que abre o editor Executables com esse arquivo preenchido (nada é salvo até você
clicar em Save), e Run, que o executa uma vez pela visão combinada como uma ferramenta registrada ([Ferramentas](#ferramentas)).
Save tree grava a árvore Data combinada inteira num arquivo de texto, uma linha por arquivo com o mod que o fornece, com a
sua pasta pessoal e o nome de usuário removidos, para um relatório de bug.

Plugins é a ordem de carga de ESP/ESM/ESL: alterne, reordene à mão, ou ordene com o LOOT e leia o relatório pós-ordenação,
cujos links de conselho abrem no seu navegador. O cabeçalho conta os plugins ativados em cada espaço de índice contra o
limite do jogo, por exemplo `full 231/254, light 512/4096`, e fica laranja quando um está cheio. A caixa de filtro reduz a
lista por nome. Space, Ctrl+A, intervalos com Shift e os itens Activate all shown e Deactivate all shown do menu da linha
passam a agir só nas linhas mostradas, e um arrasto é recusado até o filtro ser limpo. Selecionar um plugin tinge os masters
dele na lista de plugins e o mod que o traz na lista de mods. Plugins que o LOOT marca como sujos ganham um cartão no
Diagnostics que pode limpá-los com o xEdit
([tools.md](tools.md#limpar-um-plugin-que-o-loot-marca-como-sujo)).

Depois de uma ordenação do LOOT, um plugin cheio cujos registros novos cabem todos no intervalo light diz isso ao passar o
mouse, e o menu da linha dele tem Flag as light. O Eidos grava uma cópia com a flag light num mod novo chamado
`<plugin> (light)` no fim da lista de mods (a prioridade mais alta) e deixa o original em paz, então desativar esse mod
desfaz. Ele recusa quando um dos três saves mais novos do perfil carrega o plugin como plugin cheio. Só saves de Skyrim e
Fallout 4 são lidos, então no Starfield nenhum save é verificado. O LOOT verifica só intervalos de registros: olhe no xEdit
um plugin que adiciona células ou worldspaces antes de marcá-lo.

Passar o mouse sobre um plugin mostra de onde ele vem, o autor, o cabeçalho e a versão de formulário, os primeiros 300
caracteres da descrição, os masters e, depois de uma ordenação do LOOT, as mensagens do LOOT. No Skyrim, Skyrim SE, Skyrim VR
e Enderal SE o jogo também lê um INI com o nome de um plugin carregado; a coluna INI marca esses plugins, e um clique abre
esse arquivo somente leitura.

Conflicts explica os vencedores e perdedores por arquivo. Overwrite transforma o que o jogo escreveu num mod de verdade em um
passo.

Archives lista todo BSA ou BA2 que os mods ativados trazem, o mod de onde ele vem e por que carrega: um plugin ativo com o
mesmo nome, ou o INI o registra. Um que nada carrega é marcado como peso morto. Quando outro mod traz um arquivo com o mesmo
nome que vence o conflito, a linha nomeia esse mod. Extract descompacta um arquivo para um mod novo com o nome dele sem a
extensão (`Foo.ba2` dá `Foo (extracted)`) e deixa o arquivo no mod dele; arquivos soltos vencem arquivos em qualquer BSA ou
BA2, seja qual for a ordem dos mods.

Saves lê o cabeçalho de cada save (personagem, nível, local, tempo de jogo), compara a lista de plugins guardada nele com a
atual e tem um botão que ativa os mods de que ele precisa. Saves que chegaram ao prefixo sem o Eidos estão descritos em
[Os saves pertencem ao perfil](#os-saves-pertencem-ao-perfil).

Downloads e Diagnostics têm as próprias seções abaixo.

### Informações do mod

"Information..." abre um diálogo por mod com as abas General, Conflicts, Filetree, INI Tweaks, Notes, Nexus files e Nexus
page. Pela filetree (e pela árvore Data) qualquer arquivo pode ser ocultado: ele é renomeado para `<name>.mohidden`, o que o
tira da visão combinada sem apagá-lo, então alguns arquivos soltos de um mod podem ser suprimidos sem mudar prioridades. A
filetree também faz as operações comuns de arquivo: nova pasta, renomear, apagar, abrir. Cada uma recusa um caminho que não
seja um caminho simples dentro desse mod: sem `..`, sem caminho absoluto e sem link simbólico no meio, já que seguir um
poderia apagar um arquivo fora da pasta do mod. Rename muda só a última parte do caminho, então nunca move um arquivo, e
recusa um nome já usado. Delete leva dois cliques e não pode ser desfeito.

View em qualquer linha da filetree ou da árvore Data mostra uma prévia do arquivo: imagens, texto, texturas DDS (escolha o
mip, a camada, a face e o canal) e modelos NIF estáticos com as texturas difusas deles. A prévia de NIF precisa do
`eidos-nif-preview` ao lado do Eidos ou no `PATH`; o pacote de versão, o pacote do Arch e as compilações a partir do código o
têm, o `eidos-linux` não. O texto é lido até 64 KB e diz onde parou.

INI Tweaks lista os fragmentos que um mod traz na pasta `INI Tweaks/` dele; os ativados são fundidos no INI do jogo do
perfil na inicialização, em ordem de prioridade, e retirados de novo quando os INIs da execução são capturados, então
desativar um ajuste remove as configurações dele.

A aba Nexus files lista os arquivos do mod no Nexus, o mais novo primeiro, e marca o instalado aqui e o que o substitui.
Essa segunda linha tem um botão Update; veja [Atualizações de mods](#atualizações-de-mods).

A aba Nexus page mostra a descrição do mod e, do mais novo ao mais antigo, o changelog de cada versão de arquivo na página
dele. O Eidos lê os dois na primeira vez que você abre a aba, pela API GraphQL do Nexus, que não conta contra o orçamento
horário de requisições. Ele os guarda até o Eidos fechar ou você clicar em Refresh. São mostrados como texto simples: a
formatação BBCode e HTML é removida, o endereço de um link é escrito depois do texto dele, e imagens e vídeos ficam de fora.
Endorse e Track acima do texto fazem o que o menu de clique direito do mod faz. A página de um mod adulto fica oculta a menos
que a sua conta do Nexus mostre conteúdo adulto, e uma página sem avaliação fica oculta em qualquer caso.

### Downloads

Um download pode ser arrastado da lista Downloads para uma posição na lista de mods para instalá-lo nessa prioridade, e
arquivos ou pastas soltos na janela a partir de um gerenciador de arquivos também instalam. Soltar arquivos exige uma
sessão X11 ou XWayland: o winit os implementa só para X11. Downloads podem ser pausados e retomados: pausar interrompe a
transferência e guarda o arquivo parcial, e Resume pede ao Nexus um link novo e continua de onde parou.

A aba Downloads guarda os arquivos que você baixou. Filtre por nome (o nome amigável do mod também, então "skyui" acha
`SkyUI_5_2_SE-12604-5-2SE.7z`), ordene por mais novo, nome, tamanho ou estado, e oculte um arquivo com o qual você terminou:
o arquivo fica e só a linha some. "Show hidden" os traz de volta, e o mesmo botão os reexibe. "Remove N installed" apaga os
arquivos dos mods que você já instalou, em dois cliques, e só os que o filtro mostra.

Preferences -> Nexus -> Install downloads when they finish vem desligado por padrão. Ligado, um download do Nexus que termina
com a janela aberta se instala sem uma visita à aba Downloads. É a instalação de sempre: um FOMOD abre o instalador dele, e um
nome já presente na lista de mods pergunta o que fazer. Arquivos que estavam em `downloads/` antes de a janela abrir ou antes
de a opção ser ligada ficam em paz, e o mesmo vale para um download cuja instalação você cancelou. Enquanto uma coleção está
aberta no painel de coleções, nenhum download se instala sozinho, e um que termina nesse meio-tempo também não se instala
sozinho depois; a coleção instala seus membros. O arquivo que um Update pendente espera é instalado pelo Update. Só downloads
do Nexus são instalados assim, e um que o filtro de Downloads esconde espera até o filtro ser limpo. Enquanto um arrasto está
em andamento, ou enquanto o jogo ou outro Eidos segura a instância, a instalação espera.

### Atualizações de mods

O botão de atualização perto da ponta direita da barra de ferramentas verifica todo mod que tem um id do Nexus. Quando a
página do mod mostra a versão que você tem instalada, nada mais é perguntado. Caso contrário o Eidos lê a lista de arquivos do
mod (mais uma requisição, guardada pelo resto da verificação) e segue a cadeia de atualização a partir do id de arquivo que o
`meta.ini` do mod registra: os autores marcam um upload novo como sucessor de um mais antigo, e o MO2 lê a mesma cadeia. Um
arquivo mais novo nessa cadeia é a atualização. Quando a cadeia não nomeia nenhum e o arquivo instalado ainda está listado na
página, o mod conta como atual, mesmo que a página mostre outra versão. A versão da página só decide quando nenhum id de arquivo
está registrado, quando o arquivo instalado foi removido ou arquivado, ou quando o Nexus não enviou a lista de arquivos.

Cada mod usa o domínio do Nexus do jogo que o próprio `meta.ini` dele nomeia (`gameName`), não o da instância: a verificação de
atualização, Endorse, Track e Visit on Nexus vão todos para lá. O Enderal SE também aceita mods `skyrimspecialedition`, então um
mod de Skyrim SE instalado numa instância de Enderal SE é verificado na página dele do Skyrim SE. Um link Mod Manager Download
vai para a instância que você usou por último quando o jogo dela aceita o domínio do link. Caso contrário vai para uma instância
do jogo de quem o domínio é, antes de uma de um jogo que só o aceita. `EIDOS_INSTANCE=<folder>` sobrepõe os dois.

Track e Untrack no menu de clique direito de um mod adicionam o mod aos seus mods acompanhados no Nexus ou o removem, então as
notificações do site seguem o que você acompanha aqui. Endorse e Abstain no mesmo menu endossam o mod no Nexus ou retiram o
endosso; o botão de endosso ao lado do botão de atualização endossa o mod selecionado. Cada verificação de atualização também lê
seus endossos e mods acompanhados no Nexus (duas requisições) e ajusta as flags Endorsed e Tracked de todo mod para combinar, no
domínio do Nexus do próprio mod. Quando os dois discordam, o Nexus vence. Se uma das listas não puder ser lida, a verificação diz
isso e deixa essas flags como estavam.

O botão Update na aba Nexus files de "Information..." instala o arquivo mais novo. Com uma conta premium o Eidos baixa o arquivo
ele mesmo. Com uma conta gratuita ele abre a página do arquivo, onde Mod Manager Download manda o arquivo de volta, e a instalação
começa quando o arquivo chega em `downloads/`. Daí em diante é a instalação de sempre: um FOMOD abre com as respostas com que o mod
foi instalado, e depois o prompt oferece Replace com "Keep a backup of the existing mod" marcado, seja o que for que você escolheu
no último prompt. Replace mantém a prioridade e o estado ativado do mod.

Antes de um Replace trocar a pasta de um mod, o Eidos compara a pasta antiga com a nova por nome de arquivo e tamanho e lê os
cabeçalhos dos plugins nas duas. Isso cobre Update, uma atualização de coleção e um Replace escolhido no prompt de instalação. A
linha de status então conta os arquivos adicionados, removidos e alterados em tamanho, e nomeia todo master de que a versão nova
precisa e a antiga não precisava. A aba General de "Information..." termina com uma seção Last update que lista as versões, os
plugins adicionados ou removidos, os masters adicionados ou removidos, os plugins cuja flag ESM, ESL ou ESH mudou, os mods com que
este agora compartilha arquivos ou deixou de compartilhar, e os arquivos. Um arquivo reescrito com o mesmo tamanho não é listado. O
relatório é guardado em `logs/updates/<mod>.json` na instância, que `eidos pack` deixa de fora; uma instalação nova ou um Merge na
pasta o remove. `eidos install --replace` imprime as mesmas contagens numa linha `changes:`.

Uma pasta que guarda vários arquivos do Nexus, como um arquivo principal com um opcional fundido nele, é recusada: substituí-la
pelo arquivo principal novo derrubaria os outros. Atualize esses à mão.

`eidos nexus update skyrimse --apply` executa a mesma verificação e depois, numa conta premium, baixa cada arquivo novo e o instala
sobre a pasta dele com `--replace --backup`. Numa conta gratuita ele imprime `OPEN <url>` para cada página de arquivo em vez disso.
Pastas que fundem vários arquivos são puladas, com o motivo. Um mod FOMOD sem respostas registradas, como um instalado antes de o
Eidos guardá-las, não é reinstalado com os padrões do instalador: essa atualização falha com uma mensagem para atualizar o mod uma
vez pela janela, o que registra as respostas. Um FOMOD cujo instalador não tem nada a escolher é instalado como de costume. O comando
sai com 1 se alguma atualização falhou.

### Atualizações de coleções

Quando você executa Check for updates, o Eidos também pergunta ao Nexus pela revisão mais recente de cada coleção instalada nesta
instância. Uma linha acima da lista de mods nomeia toda coleção com revisão mais nova e conta os mods adicionados (+), atualizados
(~) e removidos (-); passe o mouse para ver os nomes. As contagens faltam quando a receita da revisão instalada não está mais em
cache. Update abre a revisão mais nova no painel de coleções, onde *Install this collection* a executa. A lista é reconstruída a
cada verificação e não é salva.

### Configurações do Nexus

Preferences -> Nexus -> Offline impede o Eidos de contatar o Nexus por completo, e pula a verificação diária de versões no GitHub.
Verificações de atualização, login, downloads e coleções dizem isso em vez de falhar com um erro de conexão. Vem desligado por padrão.

Preferred servers ordena os nós de CDN que um download prefere, o melhor primeiro. Só uma conta premium recebe mais de um espelho para
escolher, então para todo o resto o Nexus escolhe e isso não muda nada. Quando nenhum dos servidores que você nomeou é oferecido, o
download ainda acontece, do primeiro nó que o Nexus oferece.

### Categorias

Categorias podem ser atribuídas a um mod ou a uma seleção inteira, o catálogo pode ser editado no mesmo diálogo, e a lista oficial de
categorias do jogo pode ser puxada do Nexus. Os dois arquivos de catálogo são os do próprio MO2 (`categories.dat` e `nexuscatmap.dat`),
então uma instância compartilhada mantém um catálogo. Um mod baixado do Nexus é categorizado na instalação, depois que Fetch from Nexus
mapeou o catálogo.

### Editor de INI, log e extensões

View -> INI editor edita os INIs do jogo do perfil, a cópia que o Eidos guarda; a cópia no prefixo do Proton é sobrescrita a cada
início. A última aba dele é o `launch-env.ini` do perfil ([Variáveis e wrappers por perfil](#variáveis-e-wrappers-por-perfil)).
View -> Log lê os logs de sessão. View -> Extensions lista seus próprios complementos; veja [extensions.md](extensions.md).

Quando um início não acha o INI principal do jogo (`Skyrim.ini`, `Fallout4.ini`, `Oblivion.ini`...) nem no perfil nem no prefixo, o
Eidos primeiro copia o padrão da pasta do jogo (`Skyrim_Default.ini`, `Fallout4_Default.ini`...) para o perfil, como o MO2 faz. Quando
a instância executa uma cópia do jogo, o padrão vem da cópia. O prefixo e a pasta do jogo não são alterados.

### Instalar mods

O instalador cuida de arquivos Simple, FOMOD, pacotes BAIN do Wrye Bash (marque os subpacotes, que se fundem em ordem) e pacotes OMOD
do Oblivion. Um script OMOD roda quando usa os comandos do OBMM que o Eidos suporta ou é um dos três instaladores C# que o Eidos
conhece; o Eidos não executa nenhum outro código C# ou Visual Basic e avisa. Quando nenhuma heurística reconhece o layout, um seletor
manual mostra a árvore do arquivo e deixa você apontar a raiz de dados.

Reinstalar ou atualizar um mod FOMOD abre o assistente com as respostas da última instalação marcadas, no primeiro passo cujas opções
mudaram desde então, com as mudanças listadas. Reinstall with new choices começa dos padrões do instalador. `eidos install --replace`
ou `--merge` reaproveita as respostas guardadas e recusa, listando as mudanças, quando elas não servem mais.

### Diagnostics

A aba Diagnostics roda verificações de saúde ao vivo. Ela reporta masters faltando, plugins que o LOOT marca como sujos, arquivos que
nenhum plugin ativo vai carregar, se a lista de mods ainda combina com a pasta de mods, se o passthrough do FUSE está ligado e, quando
ele é pedido, se a capacidade de inicialização de que precisa está presente. Depois de uma execução ela mostra o que o próprio log do
script extender diz sobre cada uma das DLLs de plugin dele, para você ver se seus plugins SKSE carregaram. Ela também reporta uma
atualização do jogo desde o último lançamento limpo, um script extender faltando ou que não combina com a versão do jogo (com a página
de download do extender e um botão de instalação do arquivo que você baixa), uma opção de inicialização do Steam que não inicia o
Eidos com `%command%`, o NVAPI deixado desligado para o DLSS, tela cheia exclusiva que impede a geração de quadros, um `enblocal.ini`
do ENB sem `LinuxVersion=true`, as declarações de plugin de plugins F4SE e SFSE, e o log de falha da última sessão com os mods que ele
nomeia.

As verificações dos INIs e plugins do perfil procuram linhas `sTestFile` em `[General]` dos INIs custom de Fallout 4, Fallout 4 VR e
Starfield, que fazem o jogo ignorar `plugins.txt` (Fix as remove da cópia do perfil e guarda o arquivo anterior como `<ini>.pre-fix`),
plugins `.esp` ativados no Starfield, e plugins blueprint que não têm a flag blueprint, o prefixo `BlueprintShips-` ou a extensão
`.esm`, ou cujo plugin principal não está ativado. As verificações de sistema sinalizam um `vm.max_map_count` ou um limite rígido de
arquivos abertos abaixo do que o Proton precisa, e um jogo, prefixo ou instância em NTFS, exFAT ou FAT. As verificações de mods nomeiam
o caminho de dados do BodySlide quando ele não aponta para o `Data` do jogo, pastas `Data` que uma ferramenta escreveu em
`Overwrite/Root`, um patch cujos arquivos soltos perdem para o mod que ele corrige, e um mod que substitui os scripts Papyrus do script
extender.

Check for updates também lê os requisitos de cada mod no Nexus. O Diagnostics lista um mod exigido que está faltando ou desativado, com
Ignore para um requisito atendido de outro jeito.

A aba também mostra qual build do Proton roda o jogo e avisa antes de um Proton mais antigo rodar um prefixo que um mais novo preparou;
Rebuild põe de lado um prefixo quebrado para o Steam fazer um novo
([troubleshooting.md](troubleshooting.md#qual-proton-roda-o-jogo-e-um-prefixo-quebrado)). Outros cartões sinalizam um prefixo que nunca
foi preparado ou cujo registro está velho, um programa que não o Eidos recebendo os links Mod Manager Download (Register conserta), saída
de ferramenta mais velha que suas entradas (com um botão que executa a ferramenta de novo) e uma cópia limpa ou light de um plugin cujo
original mudou desde então. Cada cartão é explicado em [troubleshooting.md](troubleshooting.md#cartões-health). Copy report põe na área
de transferência um relatório do sistema e de todo achado, com a sua pasta pessoal e nome de login ocultos, para um relatório de bug.

Para Skyrim SE, Enderal SE e Fallout 4, o Diagnostics também tem um cartão Display. Ele mostra o tamanho e o modo da janela definidos no
`SkyrimPrefs.ini` do perfil (`EnderalPrefs.ini`, `Fallout4Prefs.ini`), ou na cópia do prefixo quando o perfil ainda não tem uma. Quando
nenhum dos dois define um tamanho, os campos partem da maior tela conectada listada em `/sys/class/drm`, então um Steam Deck mostra
1280x800. A largura e a altura são números inteiros de 320 a 16384. Apply grava `iSize W`, `iSize H`, `bFull Screen` e `bBorderless` na
cópia do perfil, depois de adotar a cópia do prefixo se o perfil não tiver uma. Quando o SSE Display Tweaks define a janela, Apply grava
o mesmo tamanho e modo na cópia dele no Overwrite; o arquivo do próprio mod não é alterado.

O cartão Game files no topo da aba está descrito em
[Arquivos do jogo](#arquivos-do-jogo-a-instalação-da-loja-ou-uma-cópia), o cartão da ordem da coleção em
[Manter a ordem da coleção](#manter-a-ordem-da-coleção), e Find the mod behind a problem em
[Execuções de teste](#encontrar-o-mod-por-trás-de-um-problema).

### Iniciar o jogo

Para iniciar o jogo pela janela, defina a opção de inicialização do Steam do jogo como o caminho absoluto do programa, porque o Steam não
lê o `PATH` do seu shell ([install.md](install.md#depois-apontar-o-steam-para-ele)):

```
~/.local/bin/eidos-gui %command%
```

Pela Colony, o caminho é `~/.local/share/Colony/apps/Eidos/eidos-linux`. O Eidos abre na instância do jogo, a que você usou por último,
então uma instância portátil é achada de novo como a global é; clique em Run para iniciá-lo pela visão combinada.

Com Start the game at once when a store opens Eidos ligado (Preferences, General, Running a game; desligado por padrão), o Eidos pula
esse clique: ele verifica o Health e inicia o jogo sem mostrar a janela. A janela abre como de costume quando o Health lista um problema,
quando a lista de mods precisa de atenção, quando uma ferramenta configurada para rodar antes de jogar tem uma saída desatualizada ou,
para Skyrim SE, Enderal SE e Fallout 4, quando o perfil ainda não tem tamanho de janela (a janela abre então em Diagnostics). Para ter a
janela em um único início, ponha `EIDOS_WINDOW=1` antes do comando: `EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

Numa janela que nenhuma loja abriu, Run não inicia o jogo por conta própria, e o Eidos nunca inicia um jogo diretamente pelo Proton ou
pelo Wine. Run pede à loja que é dona da cópia que o inicie, e a loja inicia o Eidos:

- O Steam executa `steam -applaunch <appid>` quando a opção de inicialização acima está definida e o jogo tem um prefixo do Proton, que o
  Steam cria na primeira vez que inicia o jogo. O Steam inicia primeiro se estiver fechado.
- O Heroic (uma cópia GOG ou Epic, nativo ou Flatpak) recebe um link `heroic://launch` quando o Eidos é um wrapper nas configurações do
  jogo no Heroic, com o caminho absoluto de `eidos-gui` (ou `eidos-linux`) como executável e sem argumentos. O Heroic inicia primeiro se
  estiver fechado.
- O Legendary sozinho executa `legendary launch <app>` quando `~/.config/legendary/config.ini` tem `wrapper =` e o caminho absoluto de
  `eidos-gui` (ou `eidos-linux`) sob a seção `[<app>]` do jogo, ou sob `[default]`.

O loader do script extender, o lançador e o executável do jogo na lista Run são todos entregues à loja desse jeito, e o mesmo vale para
`eidos tool <instance> run <entry>` para um deles num terminal. O Eidos não consegue entregar algumas cópias à loja delas, como uma cópia
GOG fora do Heroic: Run então diz para iniciar o jogo por essa loja.

A entrega registra esta instância como a que você usou por último, e a loja então abre uma segunda janela do Eidos nela. Com Start the game
at once desligado, você clica em Run ali; com ele ligado, essa janela inicia o jogo sem se mostrar, a menos que um dos casos acima a abra. A
primeira janela não acompanha essa execução, então aperte F5 nela depois de jogar para recarregar as listas. Quando a configuração da loja
falta, ou o loader do script extender não está ao lado do executável do jogo, Run não inicia nada e mostra o que definir e onde, com o
caminho real do binário em execução.

Enquanto um jogo ou ferramenta roda, Run vira Stop. Dois cliques encerram todo programa que a execução iniciou, e o Eidos então captura a
sessão e desmonta como após qualquer saída; uma execução interrompida nunca conta como lançamento limpo
([troubleshooting.md](troubleshooting.md#um-jogo-ou-ferramenta-que-não-fecha)).

O `%command%` do Steam para os títulos da Bethesda costuma apontar para `<Game>Launcher.exe`. O Eidos nunca o executa: o lançador é um app
de configurações separado que revarre `Data` e reescreve `plugins.txt`, desfazendo a ordem de carga que acabou de ser implantada. O Eidos
executa o loader do script extender, se houver um instalado, e o binário do jogo caso contrário, e avisa quando precisa recair no binário do
jogo.

O Eidos varre as DLLs dos mods ativados em busca de uma importação de `d3dcompiler_47.dll`, copia a DLL da Microsoft para o prefixo e só
então define `WINEDLLOVERRIDES=d3dcompiler_47=n,b`; você não precisa defini-lo você mesmo.

### Variáveis e wrappers por perfil

Um perfil pode ter as próprias variáveis de ambiente e comandos wrapper em `profiles/<name>/launch-env.ini`, uma entrada por linha:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Elas valem para todo `eidos play` desse perfil (o jogo iniciado pelo `%command%` do Steam, ou por Run numa janela que uma loja abriu) e para
toda execução de ferramenta. `eidos shell` recebe as variáveis e nenhum wrapper. A primeira linha `wrapper=` é a mais externa. Uma linha
`wrapper=` que contém `%command%` recebe o comando nesse lugar, como nas opções de inicialização do Steam; uma linha sem ele recebe o comando
no fim. Os wrappers rodam dentro do espaço de nomes privado do Eidos, então veem os mods.

Uma variável aqui substitui o valor que o Eidos define para o mesmo nome, e uma linha `env=` da própria ferramenta no `tools.ini` substitui os
dois. `WINEDLLOVERRIDES` é, em vez disso, acrescentado depois das sobreposições que o Eidos define para ENB, ReShade e as DLLs embutidas, então
uma DLL nomeada nos dois recebe a configuração do perfil e as outras mantêm a delas.

View -> INI editor abre o arquivo como última aba, para todo jogo. Linhas em branco e linhas que começam com `#` ou `;` são puladas. Ao
salvar, o Eidos lista os números das linhas que ignora: as que não são nem `env=NOME=valor` nem `wrapper=comando`, e uma linha `wrapper=` cuja
primeira palavra é `NOME=valor`, como as opções de inicialização do Steam permitem; ponha essa variável numa linha `env=`. `eidos play <instance>`
sem comando lista as variáveis que uma execução recebe e os wrappers. Copy report em Diagnostics nomeia as variáveis e os programas wrapper, sem
os valores.

## Ferramentas

xEdit, BodySlide, DynDOLOD e outras ferramentas rodam pela visão combinada dentro do prefixo Wine do jogo (o prefixo do Proton do Steam, ou o que
o Heroic preparou para uma cópia GOG ou Epic):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # executar um exe uma vez sem registrá-lo
eidos prereqs skyrimse            # o que as ferramentas registradas precisam, e o estado
eidos prereqs skyrimse --install  # baixar o que falta
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen e Synthesis são achados sem registrá-los. Pandora, Nemesis, FNIS, BodySlide e
Synthesis já vêm com um mod de captura definido, então a saída deles vai para um mod próprio (detalhes em
[tools.md](tools.md#mandar-a-saída-de-uma-ferramenta-para-um-mod-dela)).

O nome que você dá a uma ferramenta decide quais runtimes e DLLs o Eidos providencia para ela: `BodySlide` recebe as bibliotecas DirectX dela,
`BS` não recebe nada. Na janela, o diálogo Executables mostra o estado real de cada pré-requisito sob o campo, e os que faltam são botões.

O pacote de versão traz os instaladores do vcrun2022 (x86 e x64), o .NET 8 Desktop Runtime 8.0.12 e o runtime .NET 10 10.0.10 em `runtimes/`, e o
`install.sh` os copia para `~/.local/share/Colony/Eidos/runtimes/`, então `eidos prereqs <instance> --install` os instala sem download. O Eidos
executa em silêncio, no prefixo do jogo e sem winetricks, os instaladores do vcrun2022 e do .NET 8 da Microsoft. O `eidos-linux` e as compilações
a partir do código não trazem pasta `runtimes/`, então baixam os mesmos arquivos fixados da Microsoft. O SDK do .NET 10 de que o Synthesis precisa
(cerca de 300 MB) é sempre um download.

A tabela, os três níveis de pré-requisito, por que o DynDOLOD precisa de um runtime .NET que o winetricks não consegue instalar, e por que uma
ferramenta instalada como mod é iniciada pelo caminho combinado em vez da própria pasta estão em [tools.md](tools.md).

## Extensões

O Eidos pode ser estendido sem ser recompilado: um manifesto TOML em `~/.config/Colony/Eidos/addons/` adiciona uma ferramenta à lista Extensions
ou uma verificação à aba Diagnostics. Nada é carregado dentro do Eidos: uma extensão é um programa que ele executa. Veja
[extensions.md](extensions.md).

## Experimente a prova de conceito

O script não precisa de nenhum jogo. Ele mostra a visão combinada, o copy-on-write, uma pasta de jogo intocada e uma fusão que só existe dentro do
espaço de nomes dela, usando apenas OverlayFS sem privilégios num espaço de nomes de usuário (Linux 5.11 ou mais recente):

```sh
./scripts/poc-overlay.sh
```
