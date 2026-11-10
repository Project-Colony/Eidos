<!-- eidos-i18n: source=docs/guide/tools.md sha=4cb3ab1260fe32bf41a1a9fdf03479f0c1d74b50 -->

# Ferramentas: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Uma ferramenta rodada através do Eidos enxerga **a visão combinada**, dentro do
próprio prefixo Proton do jogo. Ela lê o que o jogo vai ler - todo mod ativado,
em ordem de prioridade - e o que quer que ela escreva cai no Overwrite, onde um
clique transforma aquilo num mod de verdade.

## As que o Eidos acha sozinho

Algumas ferramentas têm nome único o bastante para serem encontradas em vez de
declaradas, e o xEdit é o caso óbvio: `FO4Edit.exe` para Fallout 4,
`SSEEdit.exe` para Skyrim SE, `TES5Edit.exe` para o original, e assim por
diante - junto com o gêmeo **QuickAutoClean** de cada uma, que é o botão para
as edições sujas de que o LOOT vive avisando. O Eidos procura por elas, pelo
nome do arquivo, em:

- a pasta de instalação do jogo, e as árvores `Root/` dos mods ativados;
- **a `mods/` desta instância**, que é onde usuários do MO2 instalam ferramentas;
- a **pasta de ferramentas** que você define em Preferences (General -> Tools),
  para o diretório compartilhado entre instâncias - `/mnt/Games/Tools`
  e afins.

`PGPatcher.exe` e encontrado da mesma forma nos jogos Creation Engine, e chega com um argumento sem o qual nao consegue funcionar - veja [abaixo](#por-que-o-pgpatcher-precisa-de---ignore-mo2vfscheck).

A lista é por jogo, então a uma instância de Skyrim nunca é oferecido o editor
do Fallout. A busca para quatro níveis abaixo, porque um conjunto de mods tem
centenas de milhares de arquivos e isso roda toda vez que a lista de ferramentas
é montada, e ela não segue symlinks. Uma ferramenta achada assim é configurada
exatamente como uma que você digitou: os runtimes dela vêm do nome, pela mesma
regra de tudo que vem abaixo.

Se uma ferramenta está em outro lugar, ou você quer argumentos diferentes,
adicione à mão - uma entrada sua com o mesmo título sobrepõe qualquer coisa
achada automaticamente.

## Limpar um plugin que o LOOT marca como sujo

Depois de uma ordenação do LOOT, a aba Diagnostics lista os plugins que o LOOT
marca como sujos (dirty), nos jogos que têm ao mesmo tempo um xEdit e dados de
limpeza do LOOT (Skyrim LE, SE e VR, Enderal SE, Fallout 3, New Vegas, Fallout 4
e 4 VR, Oblivion, Starfield). Quando o Eidos acha o xEdit, registrado em
Executables ou em qualquer lugar da instância onde ele procura ferramentas, o
cartão tem Clean all e cada plugin sujo tem Clean with QuickAutoClean no menu da
sua linha; senão, o cartão diz para você adicionar o xEdit em Executables. O
Eidos não baixa o xEdit. Ele roda o modo QuickAutoClean do xEdit em um plugin
por vez, através da visão combinada, com `-qac -autoexit -autoload`. O xEdit
salva o plugin limpo por cima do que carregou, o que na visão combinada é um
arquivo novo no Overwrite; o Eidos o move para um mod chamado `Cleaned plugins`,
criado ativado no fim da lista, onde ganha todos os conflitos. O plugin no seu
próprio mod, ou na pasta Data do jogo, nunca é escrito. Para desfazer uma
limpeza, desative `Cleaned plugins` ou esconda um arquivo dentro dele.

Os backups, o cache e o log do xEdit vão para `<instance>/logs/qac-<time>/`,
fora da visão combinada, e a linha de status mostra os números desse log. O
Eidos apaga essa pasta depois de uma limpeza que deu certo e a mantém depois de
uma que falhou. O Eidos registra em `Cleaned plugins/meta.ini` o tamanho e a
data do plugin de onde cada cópia saiu. Quando esse plugin muda, a aba Health informa que a cópia está desatualizada e oferece ocultá-la.

## Adicionar uma

Na GUI: **Tools -> Executables**, depois Add. Pela linha de comando:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # listar o que está registrado
eidos tool skyrimse run BodySlide         # rodar através da visão combinada
eidos tool skyrimse run BodySlide --print # mostrar o comando sem executá-lo
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` captura a saída dessa execução em `mods/<mod>`, seja qual for a configuração da própria ferramenta.

O script extender, o binário do jogo e o lançador são detectados
automaticamente; só ferramentas extras precisam ser registradas.

### Aponte para o arquivo de verdade, onde quer que ele esteja

Registre o executável onde ele realmente está. Se a ferramenta foi instalada
como um mod, isso é dentro da pasta do mod:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(esse é o caminho da instância global - para uma instância portátil vale a mesma
regra dentro da pasta dela, `<instance>/mods/...`; note que um caminho absoluto
como esse é a única coisa que não sobrevive a MOVER uma pasta portátil depois).

O Eidos reescreve esse caminho para o caminho combinado antes de iniciar, então
a ferramenta roda a partir de `<game>/Data/CalienteTools/BodySlide/` e enxerga
ali os arquivos de todos os outros mods também. Isso importa mais do que parece:
o BodySlide vem com um diretório `SliderSets` **vazio**, e todo corpo que ele
consegue construir vem do CBBE e dos mods de roupa. Iniciado a partir da própria
pasta de mod, ele não acha nada e parece quebrado.

O MO2 faz a mesma reescrita, pelo mesmo motivo - o comentário dele mesmo cita o
FNIS.

Uma ferramenta dentro de um mod **desativado** não pode ser reescrita, porque os
arquivos dela também não estão na visão. O Eidos diz isso e a roda a partir da
própria pasta em vez de fingir.

## Ferramentas de uma lista do Mod Organizer 2

Uma lista do Wabbajack ou outra instância do MO2 aberta como instância do Eidos mantém o seu `ModOrganizer.ini`, e os executáveis configurados nele podem ser copiados para o `tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # mostra o que seria adicionado
eidos tool /path/to/list import-mo2 --write   # adiciona
```

Na janela, o diálogo Executáveis tem **Importar de ModOrganizer.ini**, com uma caixa para desmarcar em cada entrada. Nada é salvo até você apertar Salvar.

Caminhos em `Z:` são caminhos do Linux e ficam como estão. Um caminho em outra letra de unidade passa para a pasta da lista quando cita essa pasta (`D:\Lists\MyList\tools\SSEEdit.exe` vira `<list>/tools/SSEEdit.exe`), no executável, no diretório de trabalho e nos argumentos. O carregador do extensor de scripts e o binário do jogo viram nomes simples, e por isso rodam a partir da pasta do jogo sobre a qual o Eidos monta a visão, como as entradas internas. Entradas que apontam para outro lugar (um programa de sistema do Windows, uma ferramenta fora da lista) aparecem como ignoradas, com o motivo, e um título que já está no `tools.ini` nunca é substituído.

## Mandar a saída de uma ferramenta para um mod dela

Um gerador - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - escreve centenas de
arquivos. Por padrão eles caem no Overwrite junto com todo o resto. Defina
**Capture output into** no editor de Executables e a saída desta execução vai
para aquele mod:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

O mod é criado se não existir. Só os arquivos que ESTA execução produziu se
movem; qualquer coisa que já estava no Overwrite fica lá, então duas ferramentas
com alvos de captura não roubam a saída uma da outra. Uma execução que não
escreveu nada não deixa um mod vazio para trás.

Isso é feito depois da execução, e não apontando a camada de escrita para o mod,
que é como o MO2 faz. Apontar a camada de escrita para um mod o promoveria à
prioridade máxima durante toda a execução - invertendo todo conflito de que ele
participa e desinvertendo depois - e escreveria direto por cima dos arquivos do
próprio mod, sem copy-up. A captura chega ao mesmo estado final sem nenhum dos
dois.

Se o mod de destino está desativado, a saída é escrita mesmo assim, mas o jogo
não vai vê-la, então a ferramenta regeraria os mesmos arquivos na próxima
execução. O Eidos avisa quando é esse o caso.

### Quando a saída está desatualizada

O Eidos registra com quais mods, ordem de plugins, perfil e executável a saída de uma ferramenta foi gerada. Quando um deles muda, a aba Diagnostics avisa, e o cartão tem um botão **Run <tool> again** que executa a ferramenta com a configuração atual.

Para um gerador cuja saída desatualizada quebra o jogo, como Pandora ou Nemesis, adicione `check_before_play=1` à entrada dele em `tools.ini`, ou ative **Ask before playing when its output is out of date** na janela Executables. O Run então pergunta antes: executar a ferramenta ou jogar mesmo assim. Jogar mesmo assim fica lembrado até que os mods, os plugins ou o perfil mudem de novo, ou até a janela fechar. O Eidos nunca inicia a ferramenta sozinho.

## As DLLs de que uma ferramenta precisa são escolhidas pelo NOME dela

Essa é a parte surpreendente, então vale dizer sem rodeios: **o título que você
dá a uma ferramenta decide quais pré-requisitos de runtime o Eidos provisiona
para ela.** A comparação é por substring do título, sem diferenciar maiúsculas
de minúsculas.

| Se o título contém | O Eidos pede |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| qualquer outra coisa | nada |

Então uma ferramenta registrada como **`BodySlide`** recebe suas DLLs do
DirectX; o mesmo executável registrado como **`BS`** não recebe nada e pode
falhar ao iniciar com um erro que não diz nada sobre DLLs. Dê às ferramentas o
nome do programa.

A lista está em `default_prereqs` (`crates/eidos-instance/src/tools.rs`), e o
campo `Prereqs` no diálogo de Executables é editável - a detecção é um padrão,
não uma regra.

O titulo decide os **argumentos** da mesma forma, para a unica ferramenta que
precisa de um que ninguem adivinharia: um titulo contendo `pgpatcher` (ou
`parallaxgen`, seu nome antigo) recebe `--ignore-mo2vfscheck`. Por que, e por
que nao e opcional, [abaixo](#por-que-o-pgpatcher-precisa-de---ignore-mo2vfscheck).

### Três tipos de pré-requisito

**Tier 1 - DLLs embutidas** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). O Eidos as traz junto e as copia para dentro do prefixo no início. Nada a fazer, sem
rede.

**Tier 2 - verbos do winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Esses escrevem chaves de registro, o GAC e hosts do CLR,
então não podem ser copiados como arquivos. Eles **baixam da Microsoft**.

**Tier 3 - runtimes** (`dotnet10`, `dotnetsdk10`). Um runtime .NET moderno são 193 arquivos que
vivem no próprio diretório e são encontrados através de `DOTNET_ROOT`: nunca
registrados, nunca instalados no prefixo de jeito nenhum, então nenhum dos
outros dois tiers dá conta dele. O Eidos o baixa sozinho, confere contra um
checksum embutido no binário, e o guarda em cache em
`~/.local/share/Colony/Eidos/runtimes/` - **fora de qualquer instância**, porque
78 MB não é por jogo nem por perfil.

`dotnetsdk10` é o SDK do .NET 10, um download de 301 MB que ocupa 808 MB
descompactado. O Synthesis precisa dele para compilar os seus patchers, e chama
um `dotnet` sem caminho para construí-los e executá-los, então uma ferramenta
que declara `dotnetsdk10` recebe também a pasta do SDK no `WINEPATH`, além de
`NUGET_CERT_REVOCATION_MODE=offline`, `DOTNET_GENERATE_ASPNET_CERTIFICATE=false`
e `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
Compilar um patcher deixaria nós do MSBuild e o servidor VBCSCompiler rodando, e
sob o Wine eles mantêm vivo o wineserver do prefixo, então a ferramenta recebe
também `MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` e
`UseSharedCompilation=false`. `DOTNET_ROLL_FORWARD=Major` deixa um patcher
compilado para o .NET 8 (Mutagen anterior à 0.54) rodar no runtime .NET 10 do
SDK quando nenhum runtime do .NET 8 é encontrado.
Uma entrada do Synthesis no `tools.ini` que
ainda traz o antigo padrão, `dotnet8`, `vcrun2022`, passa a `dotnetsdk10`,
`vcrun2022` quando o Eidos a lê.

O NuGet verifica a assinatura de cada pacote que restaura contra o repositório
de raízes do Windows, e a um prefixo do Wine faltam as raízes de assinatura de
código e de carimbo de data/hora que o SDK traz como
`sdk/10.0.403/trustedroots/codesignctl.pem` e `timestampctl.pem`. Sem elas,
restaurar `Mutagen.Bethesda.Synthesis` 0.36.6 falha com `NU3018` e `NU3028`. Por
isso, antes de executar uma ferramenta que declara `dotnetsdk10`, e depois que o
SDK foi baixado, o Eidos adiciona os 385 certificados dos dois arquivos ao
repositório de raízes do usuário atual no prefixo do jogo, na mesma importação
com `regedit` que grava o caminho do jogo (veja abaixo). Se algum sumir, ele os
importa de novo. A partir daí, todo programa do Windows nesse prefixo confia
nessas raízes. Com elas, num prefixo novo e com o cache do NuGet vazio, a mesma
restauração e a compilação dão certo com o Wine 11.21 do sistema e com o
GE-Proton11-9.

Nada nos tiers 2 ou 3 roda em silêncio:

```sh
eidos prereqs skyrimse            # mostrar o que as ferramentas registradas precisam, e o estado delas
eidos prereqs skyrimse --install  # buscar o que está faltando (baixa)
```

Na GUI os mesmos estados ficam embaixo do campo Prereqs, e os que faltam são
botões. Um verbo que não é embutido, nem um runtime, nem um verbo conhecido do
winetricks é reportado como provável erro de digitação em vez de ser oferecido
como download.

### Por que o DynDOLOD precisa de `dotnet10`

O DynDOLOD não constrói o LOD de objetos sozinho: ele chama o LODGen por fora, e
traz três deles. O `LODGenx64.exe` mira o .NET Framework 4.8, que sob o Proton é
roteado para o Mono do Wine - cujo inicializador de `System.Uri` chama um método
que o Mono não implementa. Ele morre antes da primeira linha de trabalho,
deixando um log com um banner de versão e mais nada, e um diálogo do DynDOLOD
que diz apenas "failed for one or more worlds".

Instalar o .NET Framework de verdade não resolve: o Proton substitui a
`mscoree.dll` - o carregador que a encontraria - por um symlink dentro da
própria árvore, e refaz isso a cada atualização do prefixo.

A build que funciona é a `LODGenx64Win10.exe`, que mira o .NET moderno e nunca
toca em `mscoree`. Aponte `DOTNET_ROOT` para um runtime .NET 10 e ela roda. É
isso que o `dotnet10` provisiona, e o Eidos define a variável ao iniciar
qualquer ferramenta que o declare.

O Eidos roda o `winetricks` do sistema contra o `wine` do próprio Proton e o
prefixo do jogo, o que contorna o contêiner pressure-vessel do Steam e o
descasamento entre protontricks e Proton-GE. Uma ferramenta que declara um verbo
Tier-2 não instalado ainda assim inicia, com um aviso nomeando o verbo e o
comando para consertar - o usuário pode tê-lo de outro lugar.

### Por que o PGPatcher precisa de `--ignore-mo2vfscheck`

O PGPatcher reescreve meshes e plugins para que os mods de textura que voce tem
- vanilla, parallax, complex material, PBR - usem o shader certo em cada
superficie. Ele se recusa a rodar fora de um gerenciador de mods, e com razao:
aplicar patches numa pasta de jogo nua seria destrutivo.

O problema e COMO ele verifica: procura o **usvfs**, a camada de hooking de DLL
que o MO2 injeta no jogo no Windows. O Eidos nao tem uma e nunca tera - a visao
unificada e uma montagem FUSE na qual o Wine entra sem saber que ela existe -
entao a verificacao nao acha nada e a ferramenta sai imediatamente com

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

nomeando um programa que a pessoa nao esta executando, antes de qualquer
trabalho. A resposta do proprio autor aos dois relatos de Linux ([#716][pg716] e
[#725][pg725], ambos do Fluorine, o port do MO2 para Linux, que usa FUSE pelo
mesmo motivo) e uma flag que pula a verificacao.

Por isso o Eidos ja a preenche. Um PGPatcher encontrado num pool de mods chega
com o argumento posto, e digitar `PGPatcher` como titulo na janela Executables
preenche o campo Arguments do mesmo jeito. O que voce digita sempre vence: o
preenchimento so toca um campo deixado vazio, igual aos pre-requisitos acima.

A lista esta em `default_args` (`crates/eidos-instance/src/tools.rs`), ao lado
de `default_prereqs`. Tem uma entrada, e uma basta para justificar uma funcao:
ninguem adivinha uma flag, e a falha que ela evita nomeia o programa errado.

Em execucao nao exige nada, o que e raro para uma ferramenta .NET: ela traz o
proprio (`dotnetlib/`, com um `runtimeconfig.json` declarando
`includedFrameworks`) e entrega as DLLs do CRT da Microsoft ao lado do
executavel. Deixe Prereqs vazio.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher e PGLite no modo Mod Organizer 2

Com Mod Manager em `None`, o PGPatcher vê uma única pasta `Data` mesclada e não
consegue saber de qual mod veio uma mesh ou uma textura, então não tem conflitos
para mostrar nem ordem de mods para seguir. O modo `Mod Organizer 2` lê a
própria instância: o `modlist.txt` do perfil, os mods ativados na ordem de
prioridade e os arquivos da pasta de cada mod. O PGLite, a versão dentro do
jogo, lê a instância do mesmo jeito. Uma instância do Eidos já tem a estrutura do
MO2 (`mods/`, `profiles/<name>/modlist.txt`, `overwrite/`), então esse modo
funciona com ela.

No inicializador do PGPatcher:

1. Defina **Mod Manager** como `Mod Organizer 2`.
2. Defina **MO2 Options -> Instance Location** como a pasta da instância do
   Eidos, escrita como caminho do Windows na unidade `Z:` do Wine, que é `/`. Uma
   instância portátil em `/mnt/Games/Eidos-Skyrim` fica
   `Z:\mnt\Games\Eidos-Skyrim`; uma global fica
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Mantenha `--ignore-mo2vfscheck` nos argumentos da ferramenta. Nesse modo o
   PGPatcher procura o usvfs e para quando ele não está presente.

O PGPatcher só aceita a pasta se ela tiver um `modorganizer.ini`, por isso o
Eidos escreve `ModOrganizer.ini` na raiz da instância toda vez que inicia o jogo
ou uma ferramenta, para o perfil que está sendo iniciado. O arquivo só é
reescrito quando o conteúdo muda. Ele contém:

- em `[General]`: `gameName` (o nome do jogo no MO2, então `New Vegas` para Fallout: New Vegas), `gamePath` (a pasta de instalação do jogo como
  caminho `Z:`) e `selected_profile`;
- em `[Settings]`: `mod_directory`, `profiles_directory` e
  `overwrite_directory` como caminhos `Z:` absolutos, mais `download_directory`
  quando a instância tem uma pasta `downloads/`.

A primeira linha é um comentário dizendo que o Eidos escreveu o arquivo. Um
`ModOrganizer.ini` sem essa linha, com qualquer combinação de maiúsculas e
minúsculas no nome, nunca é alterado nem apagado: uma instância do MO2 que você
abriu no Eidos mantém o seu, e o PGPatcher lê esse arquivo como está. O MO2
descarta comentários quando salva as configurações, então um arquivo sobre o
qual o MO2 salvou passa a ser do MO2. Um arquivo vazio conta como do Eidos.

Um arquivo assim nomeia o perfil que o MO2 selecionou por último, e o PGPatcher
usa esse perfil, não o que o Eidos inicia. Quando os dois diferem, o Eidos
registra um aviso ao lançar. Selecione o mesmo perfil no MO2, ou renomeie ou
remova o arquivo para que o Eidos escreva o próprio.

O Eidos só escreve o seu arquivo quando inicia algo, então entre dois lançamentos
ele nomeia o perfil do último. Uma ferramenta que o Eidos inicia encontra
ali o próprio perfil.

O PGPatcher para com um erro quando o perfil não tem nenhum mod ativado e quando
a pasta de saída é a pasta de um mod ativado, como faz no MO2.

## Os programas próprios do Wine no prefixo do jogo

O menu Arquivo executa quatro programas do Wine no prefixo do jogo: winecfg, regedit, o explorador de arquivos do Wine e o desinstalador. Eles iniciam como qualquer ferramenta, com a versão do Proton ou do Wine que executa as ferramentas do jogo, com os mods montados, e a janela espera por eles. **Prefix drive_c** abre a unidade `C:` do prefixo no seu gerenciador de arquivos. Uma linha fica esmaecida até o prefixo ter esse programa, o que acontece na primeira vez que a loja inicia o jogo.

O Eidos escreve por conta própria um único valor do registro, o `installed path` do jogo em `HKLM\Software\Bethesda Softworks\<game>`, antes de cada execução de ferramenta quando ele está errado. Uma alteração feita nele no regedit é refeita na próxima execução.

## O caminho do jogo no prefixo

Ferramentas de Windows acham o jogo lendo
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, uma chave que o
instalador do próprio jogo escreve - e que o Steam sob Proton nunca executa. Sem
ela, xEdit, Wrye Bash e DynDOLOD abrem num caminho vazio. O Eidos a escreve
antes de rodar uma ferramenta: idempotente, aditiva, e pulada se o prefixo não
estiver inicializado ou estiver em uso.

## Chegar a uma ferramenta: esconder, fixar e um atalho no desktop

Os padrões de um jogo incluem ferramentas que você talvez nunca use, e um
seletor que lista oito entradas para chegar à segunda é um seletor que ninguém
lê. No diálogo de Executables:

- **Pin to top** põe uma entrada na cabeça da lista Run.
- **Hide from picker** tira uma de lá sem apagá-la.
- **Desktop shortcut** escreve um `.desktop` em
  `~/.local/share/applications` - onde um lançador deve ficar num sistema
  freedesktop, então ele aparece no seu menu de aplicativos e numa busca, e não
  na área de trabalho. Ele roda `eidos tool <instance> run <title>` diretamente,
  o que significa que a ferramenta sobe **através da visão combinada e com o
  perfil desta instância** sem que a janela do Eidos esteja aberta.

Esconder e fixar dizem respeito a como se *chega* a uma ferramenta, e não ao que
ela executa, então valem tanto para os padrões por jogo quanto para as suas
próprias entradas.

O loader do script extender, o lançador e o binário do jogo não têm o botão
Desktop shortcut. Eles iniciam o jogo, e o jogo sempre inicia pela loja, o que o
atalho da própria loja já faz. Um atalho criado para um deles com o Eidos 1.22
ou anterior continua funcionando: como `eidos tool <instance> run <title>`
digitado num terminal, ele pede à loja que inicie o jogo, como o Run faz numa
janela que nenhuma loja abriu. Quando a configuração da loja está faltando, o
comando não inicia nada e mostra o que definir. Um atalho não mostra mensagem
nenhuma, então rode o mesmo comando num terminal para lê-la.

Um perfil pode ter seu próprio atalho para o jogo. Clique com o botão direito no
perfil e escolha **Desktop shortcut**: o atalho roda
`eidos tool <instance> run "<game>" --profile <name>`, que torna esse perfil o
ativo, como escolhê-lo na janela, e depois pede à loja que inicie o jogo. A
janela do Eidos que a loja abre mostra esse perfil. `--profile` funciona com
qualquer `eidos tool ... run` e seleciona o perfil antes de a ferramenta
iniciar; com `--print`, não seleciona nada.

## Uma ferramenta que é um app Steam à parte

O Creation Kit é um aplicativo Steam separado e quer o próprio AppID; algumas
outras ferramentas de modding distribuídas pelo Steam são iguais. Defina
**Steam AppID** na entrada e o Eidos a inicia sob esse id em vez do id do jogo.

No Windows isso significa um lançador diferente. Aqui são duas variáveis de
ambiente na execução que já estava sendo montada - `SteamAppId` e `SteamGameId`,
as duas, porque o Proton lê uma e as bibliotecas do próprio Steam leem a outra,
e uma ferramenta que as vê discordar falha de forma estranha em vez de clara.
`eidos tool ... --print` mostra exatamente o que a execução real receberia.

## Variáveis de ambiente e ferramentas rodadas sem Proton

Três chaves de `<instance>/tools.ini` ainda não têm campo na janela Executables,
então adicione-as à mão na seção da ferramenta. Cada linha `env=NAME=value`
define uma variável só para as execuções daquela ferramenta:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

O Eidos aplica essas linhas depois de tudo o que ele mesmo define, então um
valor posto aqui substitui o dele. Isso inclui o `WINEDLLOVERRIDES` que ele monta
a partir das DLLs que os seus mods trazem: uma ferramenta que o define precisa
listar nele toda DLL de que precisa. Um nome é feito de letras, dígitos e `_`, e
não começa com dígito. Uma linha com qualquer outro nome é ignorada.

`runner=native` roda o próprio `exe`, sem Proton. A visão combinada é montada
como para qualquer ferramenta, e as linhas `arg0=`, `arg1=` são os argumentos
dele. Use isso para um programa Linux, ou para uma ferramenta de uma cópia que o
Steam não gerencia e para a qual o Heroic não registra nenhum runner: `exe` é o
Wine dessa cópia, o programa Windows é o primeiro argumento e o prefixo vem de
`env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` precisa ser um caminho absoluto. O diretório de trabalho padrão é a pasta
que contém `exe`, aqui a do runner, então defina `workdir=`. Só `exe` é
reescrito para o caminho combinado: para um programa Windows dentro de um mod,
informe o caminho dele sob a pasta `Data` do jogo, onde a visão combinada o
mostra. Uma ferramenta nativa pula as etapas do Proton: a chave de registro
descrita acima, o cache de shaders do Steam, as raízes .NET vazias e a remoção
do overlay do Steam de `LD_PRELOAD`.

Numa cópia GOG ou Epic instalada pelo Heroic, as ferramentas não precisam dessa
entrada quando as configurações do jogo no Heroic (`GamesConfig/<appName>.json`)
indicam uma versão de Wine ou Proton: o Eidos as roda com essa versão no prefixo
Heroic do jogo, e `eidos prereqs <instance> --install` a passa para o
winetricks. Uma cópia instalada só com o Legendary não tem configurações do
Heroic.

`slr=1` inicia a ferramenta, e a importação no registro que vem antes dela,
dentro do contêiner do Steam Linux Runtime que o Proton da ferramenta indica
(`require_tool_appid` no seu `toolmanifest.vdf`), do jeito que o Steam inicia o
jogo. Esse runtime precisa estar instalado em uma biblioteca do Steam; se não
estiver, a execução para e indica o app do Steam a instalar. `eidos prereqs`
continua rodando o winetricks no host, e uma ferramenta com `runner=native`
ignora a chave. O contêiner enxerga sua pasta pessoal e a biblioteca do Steam do
jogo. Uma ferramenta, um mod ou um runtime em outro lugar, em outro disco por
exemplo, precisa de `env=STEAM_COMPAT_MOUNTS=/esse/caminho` (vários caminhos
separados por `:`). Um Proton instalado em
`/usr/share/steam/compatibilitytools.d` provavelmente não funciona com `slr=1`,
porque o `/usr` dentro do contêiner é o do runtime. Nenhuma execução de
ferramenta dentro do contêiner foi testada ainda.

`eidos tool <instance> run <title> --print` mostra o comando final e cada
variável que a execução define por cima do ambiente herdado, incluindo essas
linhas.

## As configurações da própria ferramenta continuam sendo dela

O Eidos põe uma ferramenta no lugar certo com as DLLs certas. O que a ferramenta
faz depois com a configuração dela é entre você e ela, e a falha costuma ser
silenciosa.

O exemplo resolvido, porque senão custa uma hora: o **Game Data Path** do
BodySlide (Settings) precisa apontar para o diretório `Data` do jogo, não para a
pasta do jogo acima dele. Um nível alto demais e um batch build reporta "All
sets processed successfully" e escreve 1439 malhas onde o jogo nunca vai
procurá-las. O Eidos as pega - elas caem em `Overwrite/Root/` em vez da sua
instalação - mas nada está errado do ponto de vista do jogo, exceto que os seus
corpos não estão construídos.

A saída de ferramenta pertence ao Overwrite. Quando uma execução produz algo que
vale guardar, **Overwrite -> Create mod...** a transforma num mod comum, que
pode ser ordenado, desativado e removido como qualquer outro.
