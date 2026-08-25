<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# Usar o Eidos

O manual prático: a CLI, a GUI, a opção de inicialização do Steam, compilar a
partir do código e o script de prova de conceito. Para o que fazer quando algo
parece errado, veja [troubleshooting.pt-BR.md](troubleshooting.md).

## Use-o (CLI)

```sh
eidos games                       # jogos suportados instalados aqui (como a lista do MO2)
eidos init skyrimse               # criar uma instância de modding
# ...largue cada mod como uma pasta em <instance>/mods/ (a instância global fica
#    em ~/.local/share/eidos/skyrimse; `eidos init` mostra a sua)...
eidos install skyrimse mod.7z     # ou instalar um arquivo baixado (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adotar a ordem e o estado dos plugins de um perfil MO2 existente
eidos sort skyrimse               # ordenar a carga dos plugins com o LOOT
eidos play skyrimse               # mostrar o que seria montado
eidos play skyrimse -- <command>  # rodar <command> com os mods montados sobre o jogo
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` e `eidos export`
completam o conjunto; rode `eidos` sem argumentos para a lista inteira.

### Instâncias: global e portátil

Todo comando acima se dirige a uma instância. `skyrimse` nomeia a **global** -
guardada centralmente em `~/.local/share/eidos/skyrimse`, administrada pelo
Eidos. O outro tipo é a **portátil**: uma pasta autossuficiente onde você quiser
(um segundo disco, uma partição de jogos), móvel e isolada, exatamente como as
instâncias portáteis do MO2. Onde um comando aceita um identificador de jogo,
ele também aceita a pasta de uma instância portátil:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # criar uma instância portátil ali
eidos install /mnt/games/EidosSkyrim mod.7z  # todo comando aceita a pasta
eidos play /mnt/games/EidosSkyrim -- %command%
```

A pasta se descreve sozinha (o `eidos-instance.ini` dela nomeia o jogo), então
nada mais é preciso - e `EIDOS_INSTANCE=<folder>` no ambiente redireciona um
identificador de jogo para aquela pasta, o que ajuda nas opções de inicialização
do Steam. As instâncias portáteis que você criou ou abriu ficam lembradas (a
mais recente primeiro) em `~/.config/Colony/Eidos/instances.ini`; a tela de
boas-vindas da GUI lista todas para abrir com um clique, a inicialização pelo
Steam cai naquela que você jogou por último, e o manipulador `nxm://` baixa
dentro dela. Duas ressalvas que vale conhecer: mover uma pasta portátil preserva
tudo, menos as entradas de ferramentas que você registrou com caminhos absolutos
para o local antigo (readicione essas), e o cache compartilhado de runtimes
(`~/.local/share/Colony/Eidos/runtimes/`) fica de propósito global para a
máquina - um host .NET de 78 MB não é por instância.

O Eidos guarda os próprios arquivos em `Colony/Eidos`, o layout que todo
programa da família Colony usa: `~/.config/Colony/Eidos/` para o que você
escolheu (preferências, sua sessão do Nexus, sua lista de instâncias, as
definições de jogos e de add-ons que você escreveu),
`~/.local/state/Colony/Eidos/logs/` para os logs de sessão, e
`~/.local/share/Colony/Eidos/` para o que o Eidos baixou. Um Eidos mais antigo
guardava isso em `~/.config/eidos/` e `~/.local/state/eidos/`; o primeiro início
depois da atualização **copia** tudo para o lugar novo e diz isso no log. Os
diretórios antigos ficam exatamente como estavam - nada é apagado, então uma
atualização ruim não pode lhe custar um login - e você mesmo pode removê-los
quando estiver satisfeito.

Seus mods não fazem parte disso. Uma instância global continua em
`~/.local/share/eidos/<game>/`, e uma portátil onde você a pôs, porque esses
caminhos estão escritos na sua lista de instâncias e possivelmente numa opção de
inicialização do Steam: movê-los quebraria um vínculo cujas duas pontas não
pertencem ao Eidos.

Um lugar é recusado de saída: **dentro da pasta de instalação de um jogo** (o
reflexo de veterano do MO2). Aquela árvore pertence ao Steam - uma atualização,
um "verificar integridade" ou uma desinstalação pode reescrevê-la ou apagá-la,
levando junto toda a sua configuração - e o Eidos monta sobre a raiz do jogo,
então uma instância ali dentro ficaria dentro do próprio alvo da montagem. O
assistente, o `eidos init` e o `eidos play` dizem não; ponha a pasta AO LADO do
jogo (uma irmã no mesmo disco dá a mesma comodidade).

O `play` monta os mods da instância sobre o diretório `Data` do próprio jogo
(por um bind-stash, para que o daemon continue lendo os arquivos intactos)
dentro de um espaço de nomes privado, e então roda o comando através dessa
visão. As escritas (saves, configurações regeradas) caem na camada `overwrite/`
da instância; a instalação do jogo e toda fonte de mod ficam intactas byte a
byte.

### Nenhum passo privilegiado é preciso

O Eidos roda inteiramente sem root. Ele monta num espaço de nomes privado de
usuário + montagem, então nada de auxiliar setuid, nada de daemon e nada a
conceder.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` é **opcional** e libera
exatamente uma coisa: o passthrough FUSE do kernel, que vem desligado por padrão
porque quebra o jogo (abaixo). Com a capability, o Eidos toma um espaço de nomes
de montagem simples em vez de um de usuário; os mods são implantados igual de um
jeito ou de outro.


Por que o antigo conselho do `setcap` sumiu - e por que o passthrough FUSE vem
desligado - está explicado em
[troubleshooting.pt-BR.md](troubleshooting.md#por-que-o-passthrough-vem-desligado).

## GUI

```sh
cargo run -p eidos-gui
```

Um assistente de primeiro início no estilo do MO2, no visual pergaminho / bordô
da Colony: boas-vindas -> tipo de instância (portátil / global) -> jogo -> nome
e local -> resumo -> criar -> tela principal. A tela de boas-vindas também lista
toda instância existente conhecida (global e portátil, a última usada primeiro)
para abrir com um clique - ela serve também de alternador de instâncias - e
apontar o assistente para uma pasta que já contém uma instância a ADOTA como
está, em vez de criar por cima (recusando de saída se a pasta pertence a outro
jogo).

A janela principal de dois painéis também está pronta: um seletor de perfis
(trocar, ou criar um novo copiando o atual), uma lista de mods que você filtra,
seleciona, reordena, agrupa com separadores, restringe por categoria e na qual o
clique direito abre as ações, mais as abas Data / Plugins / Conflicts /
Overwrite / Saves / Downloads / Diagnostics e um botão Run com um seletor de
alvo de execução.

Reordenar não é só mandar para o topo ou para o fim: os movimentos direcionados
do MO2 também estão aqui - mandar acima do primeiro mod em conflito, abaixo do
último, para uma prioridade explícita, ou para dentro do grupo de um separador.
Todos passam por um mesmo auxiliar de movimentação, então o erro de um a mais
que vem de remover linhas antes de reinseri-las existe em um lugar só, em vez de
cinco.

### Colunas, ordenação e agrupamento

A lista desenha quatro colunas de saída e oferece oito: Category, Content,
Version, Author, Installed, Nexus id, Game, Flags. Marque-as no menu View. O
padrão não são as oito de propósito - uma lista com todas as colunas à mostra
não deixa espaço para o NOME, que é a coluna que você está de fato lendo.

Clique em qualquer cabeçalho para ordenar por ele. Clicar de novo inverte, e um
terceiro clique volta para a **ordem de carga**, o que importa mais do que
parece: a ordem de carga é a única ordem na qual a lista pode ser arrastada,
porque um vão de inserção endereça a lista real enquanto uma linha ordenada está
em outro lugar completamente. Enquanto uma ordenação está ativa, as faixas de
inserção não são desenhadas e um arrasto é recusado em vez de cair onde ninguém
mirou - a mesma coisa que o MO2 faz, e pelo mesmo motivo. O menu View diz isso e
oferece o caminho de volta.

O menu View também pode **agrupar** a lista inteira, por categoria ou por origem
(vindo do Nexus, ou instalado à mão). Cabeçalhos de grupo não são separadores:
não há nada por trás deles para renomear, colorir ou mover, eles dobram, e a
contagem continua no cabeçalho quando dobrados. Os separadores saem da lista sob
uma ordenação ou um agrupamento - um separador encabeça as linhas que vêm depois
dele na ordem de carga, e as duas coisas as moveram.

### Mouse e teclado

Duplo clique num mod abre Information, Ctrl+duplo clique abre a pasta dele,
Shift+duplo clique abre a página dele no Nexus. Ctrl+F põe o cursor na caixa de
filtro. Digitar uma letra salta para o próximo mod que começa com ela, e
apertá-la de novo percorre os demais em vez de travar no primeiro. Nenhum deles
pode cair numa linha que o filtro, um separador dobrado ou um grupo dobrado
esteja escondendo - mover um destaque que você não vê é como o próximo Space
alterna um mod que você não estava olhando.

"Collapse others" no menu de um separador dobra todos os grupos menos aquele.
Durante um arrasto, parar sobre um grupo dobrado o abre, então um mod pode ser
solto lá dentro sem abandonar o arrasto antes - parar, não passar de raspão.

### O que a lista diz sobre um mod

Dois avisos, ambos um glifo com a explicação ao passar o mouse. **No valid game
data** quer dizer que nada no topo do mod se parece com algo que este jogo
carrega; talvez as pastas dele precisem subir um nível, ou talvez não seja um
mod para este jogo. **Another game** quer dizer que o `meta.ini` do próprio mod
nomeia outro. Nenhum dos dois bloqueia nada - o mod é implantado assim mesmo - e
"Mark as valid" no menu da linha cala qualquer um deles, pela própria chave
`validated=` do MO2, de modo que um mod pelo qual você respondeu num gerenciador
chega calado no outro.

A verificação de layout é generosa de propósito: uma árvore `Root/` conta, uma
pasta ilegível conta, uma vazia conta. Um aviso errado numa lista de quinhentas
linhas é pior do que um aviso ausente.

### Fazer backup de um mod antes de mexer nele

"Back up this mod" copia a pasta dele para o lado como `<name>_backup` (depois
`_backup2`, e assim por diante - um backup nunca substitui o anterior). A cópia
é **inerte**: não é um mod, a caixa de seleção dela não faz nada, e ela não
contribui em nada para a visão combinada, porque marcá-la implantaria duas
cópias de um mod uma sobre a outra. "Restore this backup over the mod" o repõe,
em dois cliques; o conteúdo atual é movido para o lado primeiro e só é
descartado depois que a cópia deu certo.

**Data** é uma árvore real da visão combinada, expandida um nível por vez, de
modo que abrir um nó custa uma leitura de diretório por camada que o tenha, em
vez de um percurso recursivo de todo mod ativo. Ela é respondida pela MESMA
pilha de camadas de onde a montagem serve, então whiteouts e arquivos ocultos
são respeitados e a aba não pode discordar do que o jogo verá. Filtre por nome,
restrinja só aos arquivos disputados, descubra o que está onde com as colunas
Size e Modified, e abra qualquer linha num gerenciador de arquivos com Reveal.
**Plugins** é a ordem de carga dos ESP/ESM/ESL (ligar e desligar, reordenar à
mão, ou ordenar com o LOOT e ler o relatório pós-ordenação, cujos links de
conselho abrem no seu navegador). **Conflicts** explica os vencedores e os
perdedores arquivo por arquivo. **Overwrite** transforma o que o jogo escreveu
num mod de verdade, em um passo. **Saves** lê o cabeçalho de cada save -
personagem, nível, local, tempo de jogo - e compara a lista de plugins gravada
nele com a sua lista atual, com um botão que ativa os mods de que ele precisa,
porque nomeá-los e deixar o resto com você é a metade chata.

"Information..." abre um diálogo por mod: geral, conflitos, árvore de arquivos,
INI tweaks, notas. A partir da árvore de arquivos (e da árvore Data) qualquer
arquivo pode ser **ocultado** - renomeado para `<name>.mohidden`, o que o tira
da visão virtual sem apagá-lo, de modo que os três meshes perdidos de um mod
podem ser suprimidos sem mexer nas prioridades. A árvore de arquivos também faz
as operações comuns: nova pasta, renomear, apagar, abrir. Todas passam por um
mesmo resolvedor que recusa qualquer coisa que não seja um caminho simples
dentro daquele mod - nada de `..`, nada de caminho absoluto, e nenhum componente
que seja um symlink, já que seguir um poria um apagamento inteiramente fora da
pasta do mod. Renomear substitui só o último componente, então nunca pode virar
uma movimentação, e recusa um nome já ocupado em vez de substituir aquele
arquivo em silêncio. Apagar leva dois cliques; é a única ação aqui que clicar de
novo não desfaz.

**View** em qualquer linha da árvore de arquivos ou da árvore Data
pré-visualiza o arquivo: imagens e texto. DDS ou NIF não - esses precisam de um
decodificador de blocos e de um renderizador que esta árvore não tem - mas eles
dizem isso em vez de mostrar uma caixa vazia, e apontam para o Reveal. O texto é
lido até 64 KB e diz onde parou, porque uma pré-visualização é uma olhada e um
log do Papyrus pode ter uma centena de megabytes. **INI Tweaks** lista os
fragmentos que um mod traz na pasta `INI Tweaks/` dele; os ativados são
mesclados no INI do jogo do perfil no início, em ordem de prioridade, e
retirados de novo quando os INIs da execução são capturados - senão um tweak
vira silenciosamente uma configuração e desativá-lo não faria nada.

Um download pode ser **arrastado da lista Downloads para uma posição na lista de
mods** para ser instalado naquela prioridade, e arquivos ou pastas soltos na
janela a partir de um gerenciador de arquivos também instalam (essa metade
precisa de uma sessão X11 ou XWayland - o winit implementa o solte de arquivos
só no X11). Os próprios downloads podem ser pausados e retomados: pausar
interrompe a transferência e guarda o parcial, e Resume resolve um link novo e
continua de onde parou.

A aba Downloads é uma **biblioteca** de arquivos, não uma fila de
transferências. Filtre por nome (o nome amigável do mod também, então "skyui"
acha `SkyUI_5_2_SE-12604-5-2SE.7z`), ordene por mais recente, nome, tamanho ou
estado, e **oculte** um arquivo com o qual você já terminou - o que mantém o
arquivo e só tira a linha, porque guardar um livro não é queimá-lo. "Show
hidden" traz todos de volta, e o mesmo botão desoculta. "Remove N installed"
apaga os arquivos dos mods que você já instalou, em dois cliques, e só os que
estão **na tela**: o filtro é como você disse quais eram.

### Coleções do Nexus

Cole o link de uma coleção - ou clique num na página - e o Eidos lista os
membros daquela revisão, cada um cruzado com esta instância: instalado, baixado
ou faltando. Ele **lê** uma coleção; não instala uma, e o painel diz isso.
Quatro coisas tornam um instalador desonesto aqui, e não apenas difícil: os
membros são arquivos comuns do Nexus que exigem uma chave por arquivo que só uma
conta premium consegue emitir fora do botão do próprio site; uma instalação
completa são três chamadas de API por membro contra um orçamento que este
cliente se recusa a estourar; as fases, as regras e as respostas FOMOD
reproduzidas do manifesto não puderam ser verificadas contra uma coleção
Bethesda publicada de verdade, e adivinhar produz uma ordem de carga que parece
certa e não é. Ler custa uma requisição e é exato.

Uma coleção só pode ser lida contra **o próprio jogo dela**. Abra uma coleção de
Skyrim com uma instância de Fallout 4 carregada e ele recusa dizendo o nome, em
vez de cruzar os membros com a lista de mods errada, onde cada "instalado" e
cada "faltando" seria ruído com forma de resposta.

### Modo offline

**Settings -> Nexus -> Offline** faz o Eidos parar de contatar o Nexus por
completo. Verificações de atualização, login, downloads e coleções dizem isso em
vez de falhar com um erro de conexão. Vem desligado a menos que você ligue - um
arquivo de configuração escrito por um Eidos mais antigo não tem essa chave, e
ler uma chave ausente como "ligada" cortaria a rede de todo mundo que atualiza.

**Preferred servers** classifica os nós de CDN que um download prefere, o melhor
primeiro. Só uma conta premium recebe mais de um espelho para escolher, então
para todos os outros o Nexus escolhe e isso não muda nada. É uma ordenação, não
um filtro: se nada do que você nomeou estiver em oferta hoje, o download
acontece assim mesmo, pelo nó que o Nexus ofereceu primeiro.

As **Categories** são editáveis, não só exibidas: atribua-as a um mod ou a uma
seleção inteira, edite o próprio catálogo pelo mesmo diálogo, e puxe do Nexus a
lista oficial de categorias do jogo. Os dois arquivos de catálogo são os do
próprio MO2 (`categories.dat` e `nexuscatmap.dat`), então uma instância
compartilhada mantém um catálogo só.

**View -> INI editor** edita os INIs do jogo do perfil - a cópia que persiste, e
não a que fica enterrada no prefixo Proton e é sobrescrita a cada início.
**View -> Log** lê os logs de sessão. **View -> Extensions** lista os seus
próprios add-ons; veja [extensions.pt-BR.md](extensions.md).

A instalação aceita tudo: os caminhos Simple e FOMOD, mais os pacotes **BAIN**
do Wrye Bash (marque os sub-pacotes, que são mesclados em ordem) e um seletor
**manual** que mostra a árvore do arquivo e deixa você apontar a raiz dos dados
quando nenhuma heurística reconhece o layout. Nenhum arquivo é recusado.

**Diagnostics** roda verificações de saúde ao vivo: a capacidade de iniciar
acima de tudo, masters faltando (o preditor de crash mais confiável que existe),
archives que nenhum plugin ativo vai carregar, se a lista de mods ainda
corresponde à pasta de mods e - depois de uma execução - o que o log do próprio
script extender diz sobre cada uma das DLLs de plugin dele, o que transforma
"meus plugins SKSE carregaram?" de uma inferência em uma evidência.

Para iniciar o jogo pela GUI, defina a opção de inicialização do Steam daquele
jogo com o caminho absoluto do binário (o Steam não enxerga `~/.cargo/bin` no
PATH):

```
~/.cargo/bin/eidos-gui %command%
```

O Eidos abre na instância daquele jogo - a que você usou por último, então uma
instância portátil é reencontrada igual à global; clique em Run para iniciá-la
através da visão combinada. (O botão Run mostra exatamente essa linha, com o
caminho real do binário em execução, se você o apertar fora do Steam.)

O `%command%` do Steam para os títulos da Bethesda costuma apontar para
`<Game>Launcher.exe`. O Eidos nunca o roda: o launcher é um aplicativo de
configurações à parte que re-escaneia `Data` e reescreve `plugins.txt`,
desfazendo a ordem de carga que acabou de ser implantada. Ele troca pelo loader
do script extender se houver um instalado, pelo binário do jogo caso contrário,
e avisa quando precisa recorrer a este - um jogo que começa com todo mod SKSE
inerte é pior do que um que não começa.

Instruções mais antigas aqui forçavam `WINEDLLOVERRIDES="d3dcompiler_47=n"`.
Isso não é mais necessário e nunca esteve totalmente certo: um override para
*native* só ajuda se uma `d3dcompiler_47.dll` de verdade já estiver no prefixo.
O Eidos agora escaneia as importações de DLL dos mods ativos, implanta ele mesmo
a DLL de verdade da Microsoft, e só então define o override.

## Experimente a prova de conceito

Nenhum jogo é preciso. Ela prova union + copy-on-write + zero-touch + escopo por
espaço de nomes usando só OverlayFS sem privilégios num espaço de nomes de
usuário (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Ferramentas

xEdit, BodySlide, DynDOLOD e companhia rodam através da visão combinada dentro
do prefixo Proton do jogo:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # do que as ferramentas registradas precisam, e o estado disso
eidos prereqs skyrimse --install  # buscar o que estiver faltando
```

Uma coisa a saber antes de nomear uma ferramenta: **o título decide quais DLLs
de runtime o Eidos provisiona para ela** - `BodySlide` recebe as bibliotecas
DirectX dele, `BS` não recebe nada. Na GUI, o diálogo Executables mostra o
estado real de cada pré-requisito embaixo do campo, e os que faltam são botões.

A tabela, os três níveis de pré-requisitos, por que o DynDOLOD precisa de um
runtime .NET que o winetricks não consegue instalar, e por que uma ferramenta
instalada como mod é iniciada a partir do caminho combinado em vez da pasta dela
estão em [tools.pt-BR.md](tools.md).

Compilar a partir do código e o layout do repositório estão em
[../internals/contributing.md](../../../../internals/contributing.md).

## Extensões

O Eidos pode ser estendido sem ser recompilado: um manifesto TOML em
`~/.config/Colony/Eidos/addons/` adiciona uma ferramenta à lista Extensions ou
uma verificação à aba Health. Nada é carregado dentro do Eidos - uma extensão é
um programa que ele roda. Veja [extensions.pt-BR.md](extensions.md).
