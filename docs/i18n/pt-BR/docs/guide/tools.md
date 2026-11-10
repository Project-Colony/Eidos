<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Ferramentas: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Uma ferramenta rodada através do Eidos enxerga a visão combinada, dentro do prefixo
Wine do jogo (o prefixo do Proton do Steam, ou o que o Heroic preparou). Ela lê o que
o jogo vai ler (todo mod ativado, em ordem de prioridade), e o que quer que ela escreva
cai no Overwrite, onde um clique transforma aquilo num mod de verdade. Uma ferramenta
`runner=native` roda sem o Proton; veja
[abaixo](#variáveis-de-ambiente-e-ferramentas-rodadas-sem-proton).

## As que o Eidos acha sozinho

Algumas ferramentas têm nome único o bastante para serem encontradas em vez de
declaradas, e o xEdit é o caso óbvio: `FO4Edit.exe` para Fallout 4, `SSEEdit.exe` para
Skyrim SE, `TES5Edit.exe` para o original, e assim por diante. Cada uma vem com uma
entrada QuickAutoClean, que limpa as edições sujas de que o LOOT avisa. O
`PGPatcher.exe` é encontrado do mesmo jeito em todo jogo da Bethesda, exceto Morrowind,
e chega com um argumento sem o qual não consegue funcionar (veja
[abaixo](#por-que-o-pgpatcher-precisa-de---ignore-mo2vfscheck)). O Eidos procura por
elas, pelo nome do arquivo, em:

- `Overwrite/Root/`, a pasta do jogo de onde a instância roda (a instalação da loja ou
  a cópia do jogo) e as árvores `Root/` dos mods ativados;
- a `mods/` desta instância, que é onde usuários do MO2 instalam ferramentas;
- a pasta de ferramentas que você define em Preferences (General -> Tools), para um
  diretório compartilhado entre instâncias, como `/mnt/Games/Tools`.

Os geradores mais comuns são encontrados do mesmo jeito, cada um nos jogos que suporta:
Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen e Synthesis. O mesmo vale
para o BethINI Pie (`Bethini.exe`), nos mesmos jogos que o PGPatcher; veja
[BethINI Pie](#bethini-pie). No Stardew Valley, o SMAPI (`StardewModdingAPI.exe`) é
encontrado do mesmo jeito. Ele também é o loader do script extender desse jogo, então
inicia o jogo pela loja, como descrito em
[Chegar a uma ferramenta](#chegar-a-uma-ferramenta-esconder-fixar-e-um-atalho-no-desktop).

A lista é por jogo, então a uma instância de Skyrim nunca é oferecido o editor do
Fallout. A busca para quatro níveis abaixo, porque um conjunto de mods tem centenas de
milhares de arquivos e isso roda toda vez que a lista de ferramentas é montada, e ela
não segue symlinks. Uma ferramenta achada assim é configurada exatamente como uma que
você digitou: os pré-requisitos, os argumentos (o `--ignore-mo2vfscheck` do PGPatcher) e
o mod de captura dela vêm do nome, pelas regras abaixo.

Se uma ferramenta está em outro lugar, ou você quer argumentos diferentes, adicione-a à
mão. Uma entrada do usuário com o mesmo nome sobrepõe qualquer coisa achada
automaticamente.

## Limpar um plugin que o LOOT marca como sujo

Depois de uma ordenação do LOOT, a aba Diagnostics lista os plugins que o LOOT marca como
sujos, para os jogos que têm um xEdit e dados de limpeza do LOOT (Skyrim LE, SE e VR,
Enderal SE, Fallout 3, New Vegas, Fallout 4 e 4 VR, Oblivion, Starfield). Quando o Eidos
acha o xEdit, registrado em Executables ou em qualquer lugar onde ele procura
ferramentas na instância, o cartão tem Clean all e cada plugin sujo tem Clean with
QuickAutoClean no menu da linha; caso contrário o cartão diz para adicionar o xEdit em
Executables. O Eidos não baixa o xEdit. Ele executa o modo QuickAutoClean do xEdit num
plugin de cada vez, pela visão combinada, com `-qac -autoexit -autoload`. O xEdit grava o
plugin limpo por cima do que carregou, o que na visão combinada é um arquivo novo no
Overwrite; o Eidos o move para um mod chamado `Cleaned plugins`, criado ativado no fim
da lista, onde vence todo conflito. O plugin no mod dele, ou na pasta Data do jogo, nunca
é escrito. Desative `Cleaned plugins`, ou oculte um arquivo dentro dele, para desfazer uma
limpeza. Um plugin que está no Overwrite não é limpo, porque o xEdit reescreveria esse
próprio arquivo; transforme o Overwrite em mod primeiro.

Os backups, o cache e o log do xEdit vão para `<instance>/logs/qac-<time>/`, fora da
visão combinada, e a linha de status informa as contagens desse log. O Eidos apaga essa
pasta depois de uma limpeza que funcionou e a mantém depois de uma que falhou. O Eidos
registra o tamanho e a data do plugin de que cada cópia foi feita em
`Cleaned plugins/meta.ini`. Quando esse plugin muda, a aba Diagnostics mostra
Out-of-date copy of <plugin>, com um botão que oculta a cópia antiga.

## Adicionar uma

Na interface gráfica: Tools na barra de menus, ou Executables na barra de ferramentas, e
depois Add. Pela linha de comando:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse rm BodySlide          # remove an entry you added
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # run an exe once, without registering it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` captura a saída dessa execução em `mods/<mod>`, seja qual for a
configuração da própria ferramenta. `rm` remove só as entradas que você adicionou; um
padrão do jogo pode ser ocultado, não removido.

Um exe executado com `--exe` cujo nome de arquivo é um que o Eidos conhece para o jogo,
ignorando maiúsculas, recebe o nome dessa ferramenta e, com ele, os pré-requisitos,
argumentos e mod de captura abaixo. `GenerateFNISforUsers.exe` é o FNIS, então a linha
com `--exe` acima pede `dotnet48` e captura em `FNIS Output`. Qualquer outro exe recebe
como nome o próprio nome de arquivo. Na aba Data, uma linha de `.exe` tem Add tool, que
abre o diálogo Executables com a entrada preenchida, e Run, que executa o arquivo uma
vez como `--exe` faz. Os dois dão ao arquivo o mesmo nome.

Para um comando que não é uma ferramenta do Windows, `eidos shell skyrimse` abre o seu
`$SHELL` (`/bin/sh` quando não está definido) na pasta do jogo com a visão combinada
montada. O que o shell escreve cai no Overwrite, e sair do shell desmonta a visão.
`eidos shell skyrimse -- <command>` executa um único comando no lugar. Ctrl+C no shell
interativo interrompe o comando que você digitou e deixa o shell aberto. O shell roda sem
o Proton; um programa do Windows passa por `eidos tool`.

Na janela, Stop encerra uma execução de ferramenta que travou: pressione-o duas vezes, e o
Eidos encerra os programas da execução e depois trata os arquivos dela como após qualquer
saída. Com Notify the desktop when a long job ends ligado (Preferences, General, Running a
game), uma execução de ferramenta que termina com a janela em segundo plano envia uma
notificação da área de trabalho.

O script extender, o binário do jogo e o lançador são detectados automaticamente; só as
ferramentas extras precisam ser registradas. Numa instância que roda a partir de uma cópia
do jogo, eles são detectados na cópia; veja
[Ferramentas numa cópia do jogo](#ferramentas-numa-cópia-do-jogo).

### Aponte para o arquivo de verdade, onde quer que ele esteja

Registre o executável onde ele realmente está. Se a ferramenta foi instalada como mod, é
dentro da pasta do mod:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

Esse é o caminho da instância global. Para uma instância portátil a mesma regra vale sob a
pasta dela, `<instance>/mods/...`. Um caminho absoluto como esse é a única coisa que não
sobrevive a mover depois uma pasta portátil.

O Eidos reescreve esse caminho para o combinado antes de iniciar, então a ferramenta roda
de `<game>/Data/CalienteTools/BodySlide/` e vê ali também os arquivos de todos os outros
mods. O BodySlide traz um diretório `SliderSets` vazio, e todo corpo que ele consegue
construir vem do CBBE e dos mods de roupas. Iniciado da própria pasta do mod, ele não acha
nada e parece quebrado.

O MO2 faz a mesma reescrita, pelo mesmo motivo: o próprio comentário dele cita o FNIS.

Uma ferramenta num mod desativado não pode ser reescrita, porque os arquivos dela também
não estão na visão. O Eidos avisa e a executa da própria pasta.

## Ferramentas de uma lista do Mod Organizer 2

Uma lista do Wabbajack ou outra instância do MO2 aberta como instância do Eidos mantém o
`ModOrganizer.ini` dela, e os executáveis configurados ali podem ser copiados para o
`tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # show what would be added
eidos tool /path/to/list import-mo2 --write   # add them
```

Na janela, o diálogo Executables tem Import from ModOrganizer.ini, com uma caixa para
desmarcar em cada entrada. Add selected coloca as entradas marcadas na lista, e nada é
salvo até você clicar em Save. Só conta um `ModOrganizer.ini` que o Eidos não escreveu;
veja [PGPatcher e PGLite](#pgpatcher-e-pglite-no-modo-mod-organizer-2) para o que o Eidos
escreve.

Caminhos em `Z:` são caminhos Linux e ficam como estão. Um caminho em outra letra de
unidade vai para dentro da pasta da lista quando nomeia essa pasta
(`D:\Lists\MyList\tools\SSEEdit.exe` vira `<list>/tools/SSEEdit.exe`), no executável, no
diretório de trabalho e nos argumentos. O loader do script extender, o lançador e o
binário do jogo viram nomes simples, então rodam da pasta do jogo sobre a qual o Eidos
monta, como as entradas embutidas. O `hide` do MO2 vira Hide from picker, e `steamAppID`
vira Steam AppID. Entradas que apontam para outro lugar (um programa de sistema do
Windows, uma ferramenta fora da lista) são listadas como ignoradas, com o motivo, e um
nome que já está no `tools.ini` nunca é substituído.

Uma entrada importada mantém o nome e os argumentos do MO2, e o Eidos não preenche nada a
partir do nome: nem pré-requisitos, nem argumento extra, nem mod de captura. Defina-os no
diálogo Executables. Uma entrada de PGPatcher de uma lista feita no Windows costuma não ter
`--ignore-mo2vfscheck`; adicione-o (veja
[abaixo](#por-que-o-pgpatcher-precisa-de---ignore-mo2vfscheck)).

Um caminho importado que aponta para a pasta `Stock Game` da lista é movido para a
instalação da loja só no momento da execução, e apenas quando a instância roda a partir
dessa cópia; veja a próxima seção.

## Ferramentas numa cópia do jogo

Quando a instância roda a partir de uma cópia do jogo (a pasta `Stock Game` ou `Game Root`
de uma lista, ou uma cópia congelada; veja
[Arquivos do jogo](usage.md#arquivos-do-jogo-a-instalação-da-loja-ou-uma-cópia)), uma
ferramenta vê os arquivos da cópia, não os da instalação da loja. A visão combinada ainda
está montada sobre a instalação da loja: a visão root tem a cópia no fundo, e a visão Data
lê o `Data` da cópia. O Eidos procura o script extender, o lançador, o binário do jogo e as
ferramentas conhecidas na cópia, em vez de na instalação da loja.

Um caminho de ferramenta, um argumento ou um diretório de trabalho que aponta para dentro
da cópia é movido para a instalação da loja antes da execução. Ele pode estar escrito como
caminho Linux ou como caminho Wine `Z:`: `-D:"Z:\...\Stock Game\Data"` vira o mesmo caminho
sob a instalação da loja. As entradas importadas do `ModOrganizer.ini` de uma lista
precisam disso. Quando a instância roda a partir da instalação da loja, nada é movido, então
uma entrada assim lê a cópia pura, sem os seus mods.

O `installed path` do jogo no registro do prefixo e o `gamePath` do `ModOrganizer.ini` que
o Eidos escreve ainda nomeiam a instalação da loja, onde a visão combinada está. Se a pasta
da cópia sumiu, ou não tem binário do jogo, a execução da ferramenta para, como um
lançamento do jogo, e a mensagem nomeia `eidos stock-game <instance> store` para voltar à
instalação da loja.

`eidos stock-game <instance>` mostra quais arquivos do jogo a instância executa e para quais
cópias ela poderia trocar. `use <folder>` e `store` trocam, e `freeze --yes` copia a
instalação da loja para a pasta `Stock Game` da instância e passa a rodar dela. O cartão
Game files na aba Diagnostics faz o mesmo; veja
[Arquivos do jogo](usage.md#arquivos-do-jogo-a-instalação-da-loja-ou-uma-cópia).

## Mandar a saída de uma ferramenta para um mod dela

Um gerador (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) escreve centenas de arquivos.
Por padrão eles caem no Overwrite com todo o resto. Defina Capture output into no editor de
Executables e a saída dessa execução vai para esse mod:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide e Synthesis já vêm com um mod de captura definido:
`Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` e
`Synthesis Output`. DynDOLOD, TexGen e xLODGen vêm sem um, porque escrevem na pasta de saída
que você informa.

O mod é criado se não existir. Só os arquivos que essa execução produziu se movem; o que já
estava no Overwrite fica lá, então duas ferramentas com alvos de captura não levam a saída
uma da outra. Uma execução que não escreveu nada não deixa um mod vazio para trás.

O Eidos move os arquivos depois da execução. O MO2, em vez disso, aponta a camada de escrita
para o mod, o que promove esse mod à prioridade máxima durante toda a execução: todo
conflito em que ele está inverte, e inverte de volta depois. As escritas também passam
direto pelos arquivos do próprio mod, sem copy-up. A captura chega ao mesmo estado final sem
nenhum dos dois.

Se o mod alvo está desativado, a saída ainda é escrita, mas o jogo não a verá, então a
ferramenta geraria os mesmos arquivos na próxima execução. O Eidos avisa quando esse é o
caso.

### Quando a saída está desatualizada

O Eidos registra de quais mods, ordem de plugins e perfil a saída de uma ferramenta foi
feita, o tamanho e a data do executável da ferramenta, e o caminho e os argumentos da
entrada no `tools.ini`. Quando um deles muda, a aba Diagnostics mostra Generated output
inputs changed, e o cartão tem um botão Run <tool> again que executa a ferramenta como ela
está configurada agora.

Para um gerador cuja saída desatualizada quebra o jogo, como o Pandora ou o Nemesis,
adicione `check_before_play=1` à entrada dele no `tools.ini`, ou ligue Ask before playing
when its output is out of date para ele no diálogo Executables. Run então pergunta antes:
executar a ferramenta ou jogar assim mesmo. Play anyway fica lembrado até os mods, plugins
ou perfil mudarem de novo, ou até a janela fechar. O Eidos nunca inicia a ferramenta por
conta própria. Com Start the game at once when a store opens Eidos ligado, uma pergunta
dessas mantém a janela aberta em vez de iniciar o jogo.

## Pré-requisitos e argumentos vêm do nome

O nome que você dá a uma ferramenta decide quais pré-requisitos de runtime o Eidos
providencia para ela. A correspondência é uma substring do nome, sem distinguir maiúsculas.

| Se o nome contém | O Eidos pede |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| qualquer outro | nada |

Então uma ferramenta registrada como `BodySlide` recebe as DLLs do DirectX; o mesmo
executável registrado como `BS` não recebe nada e pode falhar ao iniciar com um erro que não
diz nada sobre DLLs. Dê às ferramentas o nome do programa.

A lista está em `default_prereqs` (`crates/eidos-instance/src/tools.rs`), e o campo
`Prereqs` no diálogo Executables é editável: o nome só preenche o padrão.

O nome decide os argumentos do mesmo jeito, para a única ferramenta que precisa de um que
ninguém adivinharia: um nome que contém `pgpatcher` (ou `parallaxgen`, o nome antigo) recebe
`--ignore-mo2vfscheck`. Por quê, e por que não é opcional, está
[abaixo](#por-que-o-pgpatcher-precisa-de---ignore-mo2vfscheck). O mod de captura também vem
do nome (veja [acima](#mandar-a-saída-de-uma-ferramenta-para-um-mod-dela)).

### Três tipos de pré-requisito

O nível 1 são as DLLs embutidas: `d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43` e
`d3dx11_43`. O Eidos as carrega dentro dos próprios binários e as copia para o prefixo na
inicialização. Num prefixo de 64 bits, o padrão do Proton, a build de 64 bits vai para
`system32` e a de 32 bits para `syswow64`. Nada a fazer, sem rede. Cada build é guardada
compactada com gzip, então os oito arquivos ocupam 7,9 MB de cada binário em vez de 17,9 MB,
e só a que está sendo copiada é descompactada (4,7 MB no máximo).

O nível 2 são os instaladores: `vcrun2022`, `dotnet6`, `dotnet7`, `dotnet8`,
`dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` e `xact_x64`. Eles escrevem chaves de
registro, o GAC e hosts CLR, então não podem ser copiados como arquivos. O Eidos instala três
deles sem winetricks: `vcrun2022` executa o `VC_redist.x86.exe` e o `VC_redist.x64.exe` da
Microsoft, e `dotnet8` e `dotnetdesktop8` executam o instalador do .NET 8.0.12 Desktop
Runtime, que carrega também o runtime .NET. O pacote de versão (`eidos-linux-bundle.tar.gz`)
traz esses instaladores na pasta `runtimes/`, e o `install.sh` os copia para
`~/.local/share/Colony/Eidos/runtimes/` para o usuário que o executa, também com `--system`.
O Eidos procura cada arquivo ali primeiro, depois numa pasta `runtimes/` ao lado do binário
`eidos` (um pacote descompactado executado no lugar). Sem nenhuma das duas cópias (o arquivo
único `eidos-linux`, uma compilação a partir do código ou outro usuário de uma instalação de
sistema), o Eidos os baixa da Microsoft (`download.visualstudio.microsoft.com`) para
`~/.local/share/Colony/Eidos/runtimes/` e os mantém lá. De qualquer modo, um arquivo é
recusado a menos que o SHA-256 dele bata com o que está embutido no Eidos, e roda sem janela
(`/install /quiet /norestart`) no prefixo do jogo, com a build do Proton ou do Wine que roda
as ferramentas do jogo. Como o winetricks faz, o `vcrun2022` primeiro define as DLLs do
Visual C++ como `native,builtin` nas sobreposições de DLL do prefixo. Depois, com uma build
do Proton, o Eidos confere que o runtime C++ do prefixo não é metade da Microsoft e metade do
Proton. Quando é, a instalação falha: a mensagem nomeia os pares de DLLs que vêm de builds
diferentes e diz para instalar `vcrun2022` com o winetricks em vez disso, que substitui o
`msvcp140.dll` ele mesmo. Só o .NET 8 de 64 bits é instalado; uma ferramenta .NET 8 de 32
bits ainda precisa de `winetricks dotnetdesktop8`.

Os outros seis verbos passam pelo `winetricks` do sistema, que precisa estar no `PATH`. O
Eidos o executa com o `wine` da build do Proton ou do Wine que roda as ferramentas do jogo,
contra o prefixo do jogo, o que contorna o contêiner pressure-vessel do Steam e o
descompasso entre protontricks e Proton-GE. Alguns verbos do winetricks precisam do
`cabextract`, e o Eidos avisa quando ele falta. Uma ferramenta que declara um verbo do nível
2 não instalado ainda inicia, com um aviso que nomeia o verbo e o comando para consertar,
porque você pode tê-lo instalado de outro jeito.

Um verbo do nível 2 conta como feito quando o próprio `winetricks.log` do prefixo o lista,
quem quer que tenha rodado o winetricks ou o protontricks, ou quando o Eidos o instalou nesse
mesmo prefixo. O Eidos registra as próprias instalações em `prereqs.done` na instância,
atreladas ao caminho e ao inode do prefixo, então um prefixo reconstruído mostra seus verbos
como pendentes de novo. Rebuild prefix, uma ação de Diagnostics para uma cópia Steam, renomeia
`compatdata/<appid>` para `<appid>.eidos-backup-<time>` e não apaga nada; o Steam cria um
prefixo novo na próxima vez que inicia o jogo.

O nível 3 são os runtimes, `dotnet10` e `dotnetsdk10`. Um runtime .NET moderno são 193
arquivos que vivem no próprio diretório e são achados por `DOTNET_ROOT`. Ele nunca é
registrado e nunca instalado no prefixo, então nenhum dos outros níveis consegue carregá-lo. O
Eidos o mantém fora de qualquer instância, porque 78 MB não é por jogo nem por perfil: cada
runtime é descompactado em `~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/`
(`dotnet10-10.0.10`, `dotnetsdk10-10.0.403`). O Eidos procura o zip do runtime pelo nome de
download nas mesmas duas pastas dos instaladores do nível 2, e descompacta a cópia que o pacote
de versão traz (`dotnet-runtime-10.0.10-win-x64.zip`, só para `dotnet10`) ou a baixa. O pacote
não traz o `dotnet-sdk-10.0.403-win-x64.zip` do SDK, mas uma cópia colocada ali à mão é usada e
poupa o download. O Eidos recusa qualquer dos dois arquivos a menos que o SHA-256 bata com o
que está embutido no Eidos. Ele descompacta o zip com `7z`, o mesmo 7-Zip que usa para os
arquivos de mods.

`dotnetsdk10` é o SDK do .NET 10, um download de 301 MB que descompacta em 808 MB. O Synthesis
precisa dele para compilar seus patchers, e chama um `dotnet` sem caminho para compilá-los e
executá-los, então uma ferramenta que declara `dotnetsdk10` também recebe a pasta do SDK em
`WINEPATH`, além de `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` e `DOTNET_CLI_TELEMETRY_OPTOUT=1`. Uma compilação de
patcher deixaria nós do MSBuild e o servidor VBCSCompiler rodando, e sob o Wine eles mantêm o
wineserver do prefixo vivo, então a ferramenta também recebe `MSBUILDDISABLENODEREUSE=1`,
`DOTNET_CLI_USE_MSBUILD_SERVER=0` e `UseSharedCompilation=false`. `DOTNET_ROLL_FORWARD=Major`
deixa um patcher construído para .NET 8 (Mutagen antes do 0.54) rodar no runtime .NET 10 do SDK
quando nenhum runtime .NET 8 é encontrado. Uma entrada Synthesis no `tools.ini` que ainda lista
o padrão antigo, `dotnet8`, `vcrun2022`, recebe `dotnetsdk10`, `vcrun2022` quando o Eidos a lê.

O NuGet confere a assinatura de cada pacote que restaura contra o repositório de raízes do
Windows, e um prefixo Wine não tem as raízes de assinatura de código e de carimbo de data que o
SDK traz como `sdk/10.0.403/trustedroots/codesignctl.pem` e `timestampctl.pem`. Sem elas,
restaurar `Mutagen.Bethesda.Synthesis` 0.36.6 falha com `NU3018` e `NU3028`. Então, antes de uma
ferramenta que declara `dotnetsdk10` rodar, depois de o SDK ser baixado, o Eidos adiciona os 385
certificados dos dois arquivos ao repositório de raízes do usuário atual no prefixo do jogo, na
mesma importação `regedit` que escreve o caminho do jogo (veja abaixo). Ele os importa de novo
se um sumir. Todo programa Windows nesse prefixo passa então a confiar nessas raízes. Com elas,
num prefixo novo com cache do NuGet vazio, a mesma restauração e compilação funcionam no Wine do
sistema 11.21 e no GE-Proton11-9.

O Eidos não instala nada dos níveis 2 ou 3 até você pedir:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # install what is missing (the bundle's copies first, then downloads)
```

Na interface gráfica os mesmos estados ficam sob o campo Prereqs, e os que faltam são botões. Um
verbo que não é nem embutido, nem um runtime, nem um verbo do nível 2 é reportado como provável
erro de digitação em vez de oferecido para download.

`eidos prereqs <instance> --install` recusa enquanto o jogo, o Steam ou uma ferramenta ainda
segura o prefixo. Os runtimes do nível 3 se instalam mesmo antes de o prefixo existir. Os
instaladores que o próprio Eidos executa precisam de um prefixo que exista: antes de a loja ter
iniciado o jogo uma vez, eles param com "start the game once through its store, then install
again". Os verbos do nível 2 são instalados um de cada vez e cada sucesso é registrado, então
depois de uma falha o mesmo comando continua de onde parou. Para uma cópia GOG ou Epic sem
configurações do Heroic, `--install` copia as DLLs embutidas e depois para antes dos verbos do
nível 2: ele diz para usar o winetricks próprio desse runner, e lê o `winetricks.log` do prefixo
na próxima verificação.

### Por que o DynDOLOD precisa de `dotnet10`

O DynDOLOD não constrói o LOD de objetos ele mesmo: ele chama o LODGen, e traz três deles. O
`LODGenx64.exe` tem como alvo o .NET Framework 4.8, que sob o Proton é roteado para o Mono do
Wine, e o inicializador de `System.Uri` do Mono chama um método que o Mono não implementa. O
LODGen morre antes da primeira linha de trabalho, deixando um log com um cabeçalho de versão e
nada mais, e um diálogo do DynDOLOD que diz apenas "failed for one or more worlds".

Instalar o .NET Framework de verdade não resolve: o Proton substitui o `mscoree.dll`, o loader
que o acharia, por um link simbólico para a própria árvore, e faz isso de novo a cada
atualização do prefixo.

A build que funciona é o `LODGenx64Win10.exe`, que tem como alvo o .NET moderno e nunca toca no
`mscoree`. Aponte `DOTNET_ROOT` para um runtime .NET 10 e ele roda. É isso que `dotnet10`
providencia, e o Eidos define a variável ao iniciar qualquer ferramenta que o declare.

### Por que o Pandora precisa de `dotnet10`

O Pandora é um app Avalonia, não WPF, e tem como alvo `net10.0` desde o Pandora 4.1.0, então só
o runtime .NET 10 o executa. Quando não acha nenhum runtime, ele diz para instalar o .NET
Desktop Runtime. É o texto genérico que o lançador do .NET mostra para qualquer programa
gráfico que não acha runtime, e `dotnetdesktop8` não resolve.

O Eidos 1.21.0 e anteriores davam `dotnetdesktop8` ao Pandora, e uma entrada mantém a linha
`prereqs=` dela. Quando o Eidos lê uma entrada do Pandora que ainda lista só `dotnetdesktop8`, e
o `.runtimeconfig.json` ao lado do executável dela nomeia `net10.0`, a entrada recebe `dotnet10`
no lugar. O Pandora 2.x tem como alvo o .NET 8 e mantém o que tem.

### Por que o PGPatcher precisa de `--ignore-mo2vfscheck`

O PGPatcher reescreve malhas e plugins para que quaisquer mods de textura que você tenha
(vanilla, parallax, complex material, PBR) usem o shader certo em cada superfície. Ele se recusa a
rodar fora de um gerenciador de mods, porque patchear uma pasta de jogo pura seria destrutivo.

O PGPatcher procura o usvfs, a camada do Windows de interceptação de DLLs que o MO2 injeta no
jogo. O Eidos não tem nenhuma: a visão combinada é uma montagem FUSE em que o Wine entra sem
saber que ela existe. Então a verificação não acha nada e a ferramenta sai imediatamente com

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

nomeando um programa que o usuário não está executando, antes de fazer qualquer trabalho. A
resposta do próprio autor aos dois relatos de Linux sobre isso ([#716][pg716] e [#725][pg725],
ambos do Fluorine, o fork do MO2 para Linux, que é baseado em FUSE pelo mesmo motivo) é uma flag
que pula a verificação.

Então o Eidos a semeia. Um PGPatcher achado num conjunto de mods chega com o argumento já
definido, e digitar `PGPatcher` como nome no diálogo Executables preenche o campo Arguments do
mesmo jeito. O que você digita sempre vence: a semeadura só preenche um campo que você deixou
vazio, exatamente como os pré-requisitos acima.

A lista está em `default_args` (`crates/eidos-instance/src/tools.rs`), ao lado de
`default_prereqs`.

O runtime dele não precisa de nada, o que é incomum para uma ferramenta .NET: ele embute o próprio
(`dotnetlib/`, com um `runtimeconfig.json` declarando `includedFrameworks`) e traz as DLLs CRT da
Microsoft ao lado do executável. Deixe Prereqs vazio.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher e PGLite no modo Mod Organizer 2

Com Mod Manager definido como `None`, o PGPatcher vê uma única pasta `Data` combinada e não
consegue dizer de qual mod uma malha ou textura veio, então não tem conflitos para mostrar nem
ordem de mods a seguir. O modo `Mod Organizer 2` dele lê a própria instância: o `modlist.txt` do
perfil, os mods ativados na ordem de prioridade e os arquivos na pasta de cada mod. O PGLite, a
versão dentro do jogo, lê a instância do mesmo jeito. Uma instância do Eidos já tem o layout do MO2
(`mods/`, `profiles/<name>/modlist.txt`, `overwrite/`), então esse modo funciona nela.

No launcher do PGPatcher:

1. Defina Mod Manager como `Mod Organizer 2`.
2. Defina MO2 Options -> Instance Location como a pasta da instância do Eidos, escrita como um
   caminho Windows na unidade `Z:` do Wine, que é `/`. Uma instância portátil em
   `/mnt/Games/Eidos-Skyrim` é `Z:\mnt\Games\Eidos-Skyrim`; uma global é
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Mantenha `--ignore-mo2vfscheck` nos argumentos da ferramenta. Nesse modo o PGPatcher procura o
   usvfs e para quando ele falta.

O PGPatcher só aceita a pasta se ela tiver um `modorganizer.ini`, então o Eidos escreve o
`ModOrganizer.ini` na raiz da instância toda vez que inicia o jogo ou uma ferramenta, para o
perfil que está sendo iniciado. O arquivo só é reescrito quando o conteúdo muda. Ele contém:

- em `[General]`: `gameName` (o nome do jogo para o MO2, então `New Vegas` para Fallout: New
  Vegas), `gamePath` (a pasta de instalação da loja do jogo como caminho `Z:`, também numa
  instância que roda a partir de uma cópia do jogo) e `selected_profile`;
- em `[Settings]`: `mod_directory`, `profiles_directory` e `overwrite_directory` como caminhos `Z:`
  absolutos, mais `download_directory` quando a instância tem uma pasta `downloads/`.

A primeira linha dele é um comentário dizendo que o Eidos o escreveu. Um `ModOrganizer.ini` sem essa
linha, com qualquer grafia do nome, nunca é alterado nem apagado: uma instância do MO2 que você abriu
no Eidos mantém a própria, e o PGPatcher lê esse arquivo como está. O MO2 descarta comentários ao
salvar as configurações, então um arquivo sobre o qual o MO2 salvou é do MO2 daí em diante. Um
arquivo vazio conta como do Eidos.

Um arquivo assim nomeia o perfil que o MO2 selecionou por último, e o PGPatcher usa esse perfil, não
o que o Eidos inicia. Quando os dois diferem, o Eidos registra um aviso na inicialização. Selecione o
mesmo perfil no MO2, ou renomeie ou remova o arquivo para o Eidos escrever o dele.

Ferramentas no modo Mod Organizer 2 também leem o jogo a partir do `gamePath` desse arquivo. Quando
ele não nomeia nem a pasta do jogo de onde a instância roda nem a instalação da loja, a aba
Diagnostics mostra ModOrganizer.ini points tools at another game folder, com esse caminho e a pasta
do jogo que esta instância executa. Mude o `gamePath` no arquivo, ou remova o arquivo para o Eidos
escrever o dele.

O Eidos só escreve o arquivo dele quando inicia alguma coisa, então entre lançamentos ele nomeia o
perfil do último. Uma ferramenta que o Eidos inicia acha ali o próprio perfil.

O PGPatcher para com um erro quando o perfil não tem nenhum mod ativado, e quando a pasta de saída
dele é a pasta de um mod ativado, como faz no MO2.

## Os programas próprios do Wine no prefixo do jogo

O menu File executa quatro programas do Wine no prefixo do jogo: winecfg, regedit, o explorador de
arquivos do Wine e o desinstalador. Eles iniciam como qualquer ferramenta, com a build do Proton ou do
Wine que roda as ferramentas do jogo, com os mods montados, e a janela os espera. Prefix drive_c abre
a unidade `C:` do prefixo no seu gerenciador de arquivos. Uma linha fica esmaecida até o prefixo ter
esse programa, o que acontece na primeira vez que a loja inicia o jogo. Num terminal,
`eidos tool <instance> run winecfg` faz o mesmo, e também `regedit`, `explorer` e `uninstaller`,
quando nenhuma ferramenta da lista tem esse nome.

Uma mudança feita no regedit no `installed path` do jogo é recolocada na próxima execução de
ferramenta; veja a próxima seção.

## O caminho do jogo no prefixo

Ferramentas Windows acham o jogo lendo `HKLM\Software\Bethesda Softworks\<game>` `installed path`, uma
chave que o instalador do próprio jogo escreve. O Steam sob o Proton nunca executa esse instalador. Sem
a chave, o xEdit, o Wrye Bash e o DynDOLOD abrem com o caminho vazio.

Antes de uma execução de ferramenta, o Eidos escreve a chave com uma importação `regedit` no prefixo do
jogo, a menos que uma importação anterior já tenha escrito o mesmo caminho e o prefixo ainda o guarde.
A chave nomeia a instalação da loja, também numa instância que roda a partir de uma cópia do jogo,
porque a visão combinada está montada ali. Para uma ferramenta que declara `dotnetsdk10`, a mesma
importação adiciona as raízes de assinatura do NuGet (veja
[acima](#três-tipos-de-pré-requisito)). Uma ferramenta `runner=native` pula esse passo. Quando o prefixo
ainda não existe, ou outro programa o segura, a ferramenta ainda roda, e o Eidos imprime um aviso que
nomeia a loja de onde iniciar o jogo uma vez, ou o programa a fechar. Quando a chave nomeia uma pasta
que não é a instalação da loja, a aba Diagnostics mostra Windows tools cannot find <game> até a próxima
execução de ferramenta escrevê-la de novo.

## Chegar a uma ferramenta: esconder, fixar e um atalho no desktop

Os padrões de um jogo incluem ferramentas que você talvez nunca use. No diálogo Executables:

- Pin to top põe uma entrada no início da lista Run.
- Hide from picker tira uma sem apagá-la.
- Desktop shortcut escreve um `.desktop` em `~/.local/share/applications`, então ele aparece no menu de
  aplicativos e numa busca, não na área de trabalho. Ele executa `eidos tool <instance> run <title>`
  diretamente, então a ferramenta sobe pela visão combinada com o perfil desta instância sem que a
  janela do Eidos esteja aberta.

Esconder e fixar mudam só como uma ferramenta é alcançada, então funcionam nos padrões de cada jogo e
também nas suas próprias entradas.

O loader do script extender, o lançador e o binário do jogo não têm botão Desktop shortcut. Eles iniciam
o jogo, que sempre inicia pela loja dele, e o atalho da própria loja já faz isso. Um atalho feito para um
deles com o Eidos 1.22 ou anterior continua funcionando: como `eidos tool <instance> run <title>`
digitado num terminal, ele pede à loja que inicie o jogo, como Run faz numa janela que nenhuma loja abriu.
Quando a configuração da loja falta, o comando não inicia nada e imprime o que definir. Um atalho não
mostra mensagem, então execute o mesmo comando num terminal para lê-la.

O Eidos passa qualquer entrada à loja desse jeito quando o executável dela tem o nome de arquivo do loader
do script extender, do lançador ou do binário do jogo, ignorando maiúsculas: uma entrada padrão, uma que
você adicionou, uma importada do MO2 ou uma execução com `--exe`. Primeiro ele marca esta instância como
a última usada, para que a janela que a loja abre mostre esta instância, depois executa
`steam -applaunch <id>`, `xdg-open heroic://launch?appName=<app>&runner=<runner>` para o Heroic, ou
`legendary launch <app>`. Com `--print` ele imprime esse comando da loja e não inicia nada. O comando
`steam` precisa estar no `PATH`: o Steam Flatpak não o coloca lá, então o início falha e a mensagem diz
para iniciar o jogo pelo Steam.

A entrada roda diretamente pela visão combinada, como qualquer ferramenta, quando uma loja iniciou o
processo: uma entre `SteamGameId`, `SteamAppId`, `HEROIC_APP_NAME`, `HEROIC_APP_RUNNER` ou
`EIDOS_STORE_STARTED` está definida. Sem nenhuma delas, o Eidos não inicia o jogo com o Proton por conta
própria. Quando a instância não tem loader do script extender (na pasta do jogo de onde roda, no `Root/` de
um mod ativado ou em `Overwrite/Root/`), nada inicia e a linha diz para instalar antes o script extender.
Para uma cópia GOG fora do Heroic, ou um id de loja que o Eidos não consegue repassar, ela diz para iniciar o
jogo por essa loja.

Um perfil pode ter o próprio atalho para o jogo. Clique com o botão direito no perfil e escolha Desktop
shortcut: a entrada executa `eidos tool <instance> run "<game>" --profile <name>`, que torna esse perfil o
ativo, como escolhê-lo na janela faz, e depois pede à loja que inicie o jogo. A janela do Eidos que a loja
abre mostra esse perfil. `--profile` funciona com qualquer `eidos tool ... run` e seleciona o perfil antes de
a ferramenta iniciar; com `--print` ele não seleciona nada.

## Uma ferramenta que é um app Steam à parte

O Creation Kit é um aplicativo Steam separado e quer o próprio AppID; algumas outras ferramentas de
modding distribuídas pelo Steam são iguais. Defina Steam AppID na entrada e o Eidos a inicia com esse id em
vez do id do jogo.

No Windows isso significa um lançador diferente. Aqui são duas variáveis de ambiente na mesma execução,
`SteamAppId` e `SteamGameId`. O Eidos define as duas, porque o Proton lê uma e as bibliotecas do próprio
Steam leem a outra, e uma ferramenta que as vê discordando falha sem um erro claro.
`eidos tool ... --print` mostra exatamente o que a execução real receberia.

## Variáveis de ambiente e ferramentas rodadas sem Proton

Três chaves de `<instance>/tools.ini` ainda não têm campo no diálogo Executables, então adicione-as à mão na
seção da ferramenta. Cada linha `env=NAME=value` define uma variável só para as execuções dessa ferramenta:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

O Eidos aplica essas linhas depois de tudo que ele mesmo define e depois das variáveis do `launch-env.ini` do
perfil, então um valor aqui substitui os dois. Isso inclui o `WINEDLLOVERRIDES` que o Eidos monta a partir das
DLLs que seus mods trazem: uma ferramenta que define um deve listar nele toda DLL de que precisa. Os wrappers
do perfil envolvem o comando de uma ferramenta como envolvem o do jogo; veja
[Variáveis e wrappers por perfil](usage.md#variáveis-e-wrappers-por-perfil). Um nome é letras, dígitos e `_`, e
não começa com dígito. Uma linha com qualquer outro nome é ignorada.

`runner=native` executa o próprio `exe`, sem o Proton. A visão combinada é montada como para qualquer
ferramenta, e as linhas `arg0=`, `arg1=` são os argumentos dele. Use para um programa Linux, ou para uma
ferramenta numa cópia que o Steam não gerencia e para a qual o Heroic não registra runner: `exe` é o Wine dessa
cópia, o programa Windows é o primeiro argumento, e o prefixo vem de `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` precisa ser um caminho absoluto. O diretório de trabalho assume por padrão a pasta que contém o `exe`, aqui a
do runner, então defina `workdir=`. Só o `exe` é reescrito para o caminho combinado: para um programa Windows
dentro de um mod, dê o caminho dele sob a pasta `Data` do jogo, onde a visão combinada o mostra. Uma ferramenta
nativa pula os passos do Proton: a chave de registro acima, o cache de shaders do Steam, as raízes .NET vazias e
a retirada do overlay do Steam de `LD_PRELOAD`.

Numa cópia GOG ou Epic instalada com o Heroic, as ferramentas não precisam dessa entrada quando as configurações
Heroic do jogo (`GamesConfig/<appName>.json`) nomeiam uma build do Wine ou do Proton: o Eidos as executa com essa
build no prefixo Heroic do jogo, e `eidos prereqs <instance> --install` a usa para os instaladores e para o
winetricks. Uma cópia instalada só com o Legendary não tem configurações do Heroic.

`slr=1` inicia a ferramenta, e a importação de registro antes dela, dentro do contêiner Steam Linux Runtime que a
build do Proton da ferramenta nomeia (`require_tool_appid` no `toolmanifest.vdf` dela), do jeito que o Steam inicia
o jogo. Esse runtime precisa estar instalado numa biblioteca do Steam; se não estiver, a execução para e nomeia o
app Steam a instalar. `eidos prereqs` ainda executa seus instaladores e o winetricks no host, e uma ferramenta
`runner=native` ignora a chave. O contêiner vê a sua pasta pessoal e a biblioteca Steam do jogo. Uma ferramenta, um
mod ou um runtime em qualquer outro lugar, noutro disco por exemplo, precisa de `env=STEAM_COMPAT_MOUNTS=/that/path`
(vários caminhos separados por `:`). Uma build do Proton instalada em `/usr/share/steam/compatibilitytools.d`
provavelmente não é utilizável com `slr=1`, porque `/usr` dentro do contêiner é o do próprio runtime. Nenhuma
execução de ferramenta dentro do contêiner foi testada ainda.

`eidos tool <instance> run <title> --print` mostra o comando final e toda variável que a execução define além do
ambiente herdado, inclusive essas linhas. Para uma entrada que inicia o jogo, ele imprime o comando da loja (veja
[Chegar a uma ferramenta](#chegar-a-uma-ferramenta-esconder-fixar-e-um-atalho-no-desktop)).

## BethINI Pie

O Eidos não distribui o BethINI Pie. Ponha a versão que você baixou da página dele no Nexus no `mods/` desta
instância ou na sua pasta de ferramentas, e o `Bethini.exe` aparece na lista de ferramentas, nos mesmos jogos que o
PGPatcher. Executado pelo Eidos, ele edita a cópia do perfil dos INIs do jogo. Antes de cada execução de uma
ferramenta cujo nome contém `bethini`, o Eidos guarda um ponto de restauração desses INIs; Backups na barra de
ferramentas, depois INIs, devolve um. Um INI que não existia quando a cópia foi feita fica como está. O diálogo
guarda os 10 pontos de restauração mais recentes de cada tipo, e restaurar um primeiro faz backup dos INIs como estão,
então uma escolha errada pode ser desfeita.

## As configurações da própria ferramenta continuam sendo dela

O Eidos não verifica a maioria das configurações de ferramentas; uma errada costuma falhar sem erro.

O Game Data Path do BodySlide (Settings) precisa apontar para o diretório `Data` do jogo, não para a pasta do jogo
acima dele. Definido um nível acima, uma compilação em lote reporta "All sets processed successfully" e escreve 1439
malhas onde o jogo nunca vai procurá-las. O Eidos as pega em `Overwrite/Root/`, então a sua instalação do jogo fica
intacta, e o único sinal no jogo é que seus corpos não foram construídos.

Antes de uma execução do BodySlide ou do Outfit Studio, a linha de status avisa quando `GameDataPath` ou
`OutputDataPath` no `CalienteTools/BodySlide/Config.xml` vencedor não leva à pasta `Data` do jogo durante a execução, e
a aba Diagnostics mostra o mesmo aviso como um cartão com o valor a definir. `eidos check <instance>` também o
reporta. Um caminho `S:` é lido do modo como o Eidos mapeia `S:` para uma ferramenta, para a pasta da biblioteca do
Steam. Um `OutputDataPath` dentro da pasta `mods/` da instância, para um mod de saída, é aceito. O Diagnostics também
mostra um cartão quando `Overwrite/Root/` guarda uma pasta `meshes` ou `textures`.

A saída das ferramentas pertence ao Overwrite. Quando uma execução produz algo que vale guardar, Overwrite -> Create
mod... a transforma num mod comum que pode ser ordenado, desativado e removido como qualquer outro.
