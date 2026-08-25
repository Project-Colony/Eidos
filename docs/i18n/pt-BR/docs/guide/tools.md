<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Ferramentas: xEdit, BodySlide, DynDOLOD, FNIS

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
- a **pasta de ferramentas** que você define em Settings (Tools -> Tools
  folder), para o diretório compartilhado entre instâncias - `/mnt/Games/Tools`
  e afins.

A lista é por jogo, então a uma instância de Skyrim nunca é oferecido o editor
do Fallout. A busca para quatro níveis abaixo, porque um conjunto de mods tem
centenas de milhares de arquivos e isso roda toda vez que a lista de ferramentas
é montada, e ela não segue symlinks. Uma ferramenta achada assim é configurada
exatamente como uma que você digitou: os runtimes dela vêm do nome, pela mesma
regra de tudo que vem abaixo.

Se uma ferramenta está em outro lugar, ou você quer argumentos diferentes,
adicione à mão - uma entrada sua com o mesmo título sobrepõe qualquer coisa
achada automaticamente.

## Adicionar uma

Na GUI: **Tools -> Executables**, depois Add. Pela linha de comando:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # listar o que está registrado
eidos tool skyrimse run BodySlide         # rodar através da visão combinada
eidos tool skyrimse run BodySlide --print # mostrar o comando sem executá-lo
```

O script extender, o binário do jogo e o lançador são detectados
automaticamente; só ferramentas extras precisam ser registradas.

### Aponte para o arquivo de verdade, onde quer que ele esteja

Registre o executável onde ele realmente está. Se a ferramenta foi instalada
como um mod, isso é dentro da pasta do mod:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
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

### Três tipos de pré-requisito

**Tier 1 - DLLs embutidas** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). O Eidos
as traz junto e as copia para dentro do prefixo no início. Nada a fazer, sem
rede.

**Tier 2 - verbos do winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Esses escrevem chaves de registro, o GAC e hosts do CLR,
então não podem ser copiados como arquivos. Eles **baixam da Microsoft**.

**Tier 3 - runtimes** (`dotnet10`). Um runtime .NET moderno são 193 arquivos que
vivem no próprio diretório e são encontrados através de `DOTNET_ROOT`: nunca
registrados, nunca instalados no prefixo de jeito nenhum, então nenhum dos
outros dois tiers dá conta dele. O Eidos o baixa sozinho, confere contra um
checksum embutido no binário, e o guarda em cache em
`~/.local/share/Colony/Eidos/runtimes/` - **fora de qualquer instância**, porque
78 MB não é por jogo nem por perfil.

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

## Uma ferramenta que é um app Steam à parte

O Creation Kit é um aplicativo Steam separado e quer o próprio AppID; algumas
outras ferramentas de modding distribuídas pelo Steam são iguais. Defina
**Steam AppID** na entrada e o Eidos a inicia sob esse id em vez do id do jogo.

No Windows isso significa um lançador diferente. Aqui são duas variáveis de
ambiente na execução que já estava sendo montada - `SteamAppId` e `SteamGameId`,
as duas, porque o Proton lê uma e as bibliotecas do próprio Steam leem a outra,
e uma ferramenta que as vê discordar falha de forma estranha em vez de clara.
`eidos tool ... --print` mostra exatamente o que a execução real receberia.

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
