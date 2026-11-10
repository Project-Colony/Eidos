<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=c9bb5a3ae78695b89ce4595223a2d2ff1902345e -->

# Solução de problemas e diagnóstico

Tudo para o dia em que o jogo vê algo com que o sistema de arquivos não concorda:
as chaves de ambiente, como ler os contadores de operações, os problemas conhecidos
e sua história, e a questão do passthrough.

### Diagnosticando o VFS

Existem duas variáveis de ambiente para quando o jogo vê algo com que o sistema de
arquivos não concorda:

```sh
EIDOS_FUSE_STATS=1                  # contadores de operações, despejados na desmontagem
EIDOS_FUSE_NO_CACHE=1               # todo cache do lado do kernel desligado
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # ou nomeie um a um
```

A forma granular foi o que achou o travamento descrito abaixo: desligar os quatro
responde "é o cache?", e só nomeá-los responde "qual deles". Os contadores respondem
à outra metade - uma carga que mostra `read 0` é uma em que o `FUSE_PASSTHROUGH`
serviu cada byte dentro do kernel, então tudo que você ia ajustar no caminho de
leitura já sai de graça.

## Montar uma união à mão

A primeira `--layer` vence no conflito; a última são seus dados de jogo intactos. A
montagem só precisa de `/dev/fuse` e `fusermount3` (sem overlayfs, sem Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... leia e escreva através de /mnt/point ...
fusermount3 -u /mnt/point
```

As escritas caem em `--overwrite <dir>` (um diretório temporário se omitido), de
modo que as próprias camadas permanecem intactas mesmo aqui.

#### Por que o passthrough vem desligado

O passthrough entrega ao kernel o arquivo de respaldo real, então as leituras
pulam este daemon inteiramente. O A/B abaixo foi medido com uma versão anterior
do daemon, antes da mudança descrita mais adiante no problema conhecido "O
passthrough do FUSE impede o jogo de carregar qualquer conteúdo de mod". Essa
mudança evita a causa mais provável, mas ainda não foi medida de novo em um
jogo, então o passthrough continua desligado por padrão até que seja. Medido em
A/B no Skyrim SE 1.6.1170, proton-cachyos 11.0, kernel 7.1.4, a mesma ordem de
carga de 82 plugins, com a única variável sendo se o binário carregava a
capacidade:

| passthrough | falhas de `NtCreateFile` com `STATUS_ACCESS_VIOLATION` |
|-------------|---------------------------------------------------------|
| ligado      | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| desligado   | 0                                                       |

Com ele ligado o jogo não abria nenhum dos próprios arquivos nem plugins, o que
aparecia dentro do jogo como mods que não estavam lá - sem erro, sem linha de
log. Com ele desligado, a mesma ordem de carga chegava à jogatina com seus
plugins, arquivos e scripts Papyrus vivos.

A falha era invisível de dentro do daemon, o que a tornou cara de achar: o
próprio `open` dele tinha sucesso toda vez e o kernel nunca recusava um arquivo
de respaldo (verificado ao longo de uma sessão inteira falhando com
`EIDOS_FUSE_TRACE=open`: zero `open FAILED`, zero `passthrough refused`). O erro
era produzido depois que o daemon tinha respondido `opened_passthrough`, então
nenhum log do lado do daemon conseguia vê-lo. Também não dependia da extensão -
atingia arquivos e plugins igualmente, ou seja, os arquivos que o jogo mantém
abertos por toda a execução.

`EIDOS_FUSE_PASSTHROUGH=1` religa, para medir o que ele rende ou para retestar o
mecanismo. Os avisos de capacidade no lançador e na aba Diagnostics só aparecem
quando você o pediu.

Para lançar o próprio jogo através do Eidos, defina a opção de inicialização do
Steam como:

```
eidos play skyrimse -- %command%
```

Nenhum prefixo `WINEDLLOVERRIDES` é necessário. Quando um mod ativado importa
`d3dcompiler_47`, o Eidos implanta a cópia que traz embutida dentro do prefixo,
força-a como nativa e funde isso com qualquer sobreposição de DLL que um mod
traga (carregadores ENB/ReShade/`.asi`).

### O índice de camadas está mesmo em uso?

`LayerStack::new` constrói um mapa completo das camadas somente-leitura ou
nenhum. Sem mapa, cada consulta percorre as camadas como antes: as respostas são
as mesmas, carregar um save leva cerca de três vezes mais tempo e nada mais
parece diferente.

Quando o índice está desligado, a montagem diz por quê, uma vez, ao iniciar:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

O motivo nomeia um diretório ilegível, uma camada com mais de 64 níveis de
diretório, mais de 4.000.000 de entradas ou `EIDOS_NO_INDEX`. A linha vai para o
log da execução: o `logs/run-*.log` da instância num lançamento pela janela, o
console da Steam num lançamento pela Steam. Com `EIDOS_FUSE_STATS=1`, o relatório
ao desmontar também tem uma linha `layer index:`, e um contador `unindexed`
diferente de zero numa montagem normal significa que o índice estava desligado.

Os exemplos abaixo verificam a mesma coisa offline, nas pastas de uma instância,
sem montagem:

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

## Um jogo ou ferramenta que não fecha

Enquanto um jogo ou ferramenta roda, o botão Run vira **Stop**, e o cartão de
bloqueio também tem um botão Stop. Pressione-o duas vezes. O segundo toque
encerra todos os programas que a execução iniciou: primeiro SIGTERM, depois
SIGKILL após 10 segundos para o que sobrar. O que o jogo não tinha salvo se
perde. Depois, o Eidos faz o mesmo que faz após qualquer saída: as mudanças de
INI e de plugins voltam para o perfil, a saída das ferramentas é registrada e os
mods são desmontados. **Unlock** é diferente: ele devolve a janela e deixa o
programa rodando. Num terminal, `kill <pid>` no processo `eidos` (o pid está no
cartão de bloqueio) faz o mesmo que Stop.

## Qual Proton roda o jogo, e um prefixo quebrado

A aba Diagnostics tem um cartão **Proton**: a versão que o Eidos e o Steam usam
para o jogo, a configuração do próprio Steam e a versão que rodou o prefixo por
último. Quando um Proton mais novo que o selecionado foi o último a preparar o
prefixo, o cartão vira um conselho, porque um Proton mais antigo num prefixo
mais novo pode redefinir os arquivos do Wine dele ou não iniciar. Escolha no
Steam uma versão pelo menos tão nova, em Propriedades do jogo, Compatibilidade.

**Rebuild prefix**, nesse cartão, é para um prefixo que não funciona mais.
Pressione-o duas vezes. O Eidos não apaga nada: ele renomeia
`steamapps/compatdata/<appid>` para `<appid>.eidos-backup-<date>-<time>` na
mesma pasta, e o Steam cria um prefixo novo na próxima vez que você iniciar o
jogo pelo Steam. Ele se recusa enquanto um jogo, uma ferramenta ou qualquer
programa do Wine usa o prefixo. Saves e configurações guardados só no prefixo
antigo ficam na pasta renomeada: copie de volta o que precisar e apague essa
pasta você mesmo quando não a quiser mais.

## Problemas conhecidos

### DLSS ou geração de quadros silenciosamente não faz nada

Três causas distintas, cada uma sem qualquer mensagem de erro: NVAPI não habilitado
nas opções de inicialização, tela cheia exclusiva, ou um teto de FPS do Reflex
obsoleto. A lista inteira está em [graphics.pt-BR.md](graphics.md).

**Um mod que escreve um diretório de duas formas perdia tudo sob a segunda.**
Corrigido. O ext4 mantém `meshes/` e `Meshes/` separados; a visão combinada não pode,
e mods reais trazem as duas - o XP32 Maximum Skeleton tem suas animações e seu
arquivo de comportamento FNIS sob a versão com maiúscula, e seus `character assets`
sob a outra.

O resolvedor pegava a correspondência exata de caixa para cada componente do caminho
e se comprometia com ela: entrava em `meshes/`, não achava lá o resto do caminho e
abandonava A CAMADA INTEIRA. Todo arquivo sob a outra grafia era invisível para o
jogo - sem erro, sem log, nada em diagnóstico algum. Numa instância real de 50
camadas isso deu 74 arquivos.

Um componente que casa agora é um candidato, não uma decisão; a caixa exata continua
sendo tentada primeiro, e só quando o restante falha embaixo dela é que a varredura
procura irmãos equivalentes ignorando maiúsculas. As listagens tinham o mesmo
defeito um diretório acima e agora leem cada diretório equivalente por camada.

**O LODGen do DynDOLOD morre deixando um log vazio.** Corrigido pelo `dotnet10`;
veja [tools.md](tools.md). O sintoma é inconfundível: `LODGen_SSE_<world>_log.txt`
com um cabeçalho de versão, uma linha `.NET Version:` e nada mais, para cada mundo, e
um diálogo dizendo apenas "failed to generate object LOD for one or more worlds". A
causa é o Mono do Wine respondendo pelo .NET Framework, e nenhuma quantidade de .NET
Framework instalado resolve - o Proton substitui `mscoree.dll` por um link simbólico
para a própria árvore a cada atualização do prefixo.

**O Wine não conseguia perceber que a montagem dobra a caixa.** Corrigido, e era o
que importava.

O `get_dir_case_sensitivity` do Wine pergunta primeiro ao diretório com
`FS_IOC_GETFLAGS` e depois fareja o marcador que o CIOPFS deixa nos diretórios
que serve. Sem nenhum dos dois, o Wine supõe SENSÍVEL, e toda busca cuja grafia
não bate byte a byte recai em ler o diretório INTEIRO para achar uma
correspondência insensível a maiúsculas. Jogos da Bethesda pedem
`data/ccbgssse001-fish.bsa` enquanto o arquivo é `ccBGSSSE001-Fish.bsa`, então
isso disparava em quase todo recurso: 4471 sondagens do marcador e 2236
releituras completas de diretório em oito segundos, e 195796 enumerações de
`Data` em noventa. O Skyrim SE nunca chegava ao menu principal - ficava em
240 MB residentes enquanto o daemon queimava 92 % de um núcleo.

O Eidos dobrava a caixa em `resolve_read` desde o início. Todo o custo estava em
nunca dizer isso. Agora `lookup` responde `.ciopfs`; `readdir` continua não o
listando. `ioctl` também responde `FS_IOC_GETFLAGS` nos diretórios com a flag
casefold, que o Wine atual consulta antes de procurar o marcador.

Duas coisas o tornaram fatal em vez de meramente lento. O custo escala com o tamanho
do diretório, então instalar o conteúdo Anniversary (`Data` de 37 para 177 arquivos)
fez transbordar. E o `opendir` construía avidamente a listagem combinada, o que é
puro desperdício quando o Wine abre um diretório só para dar `stat` naquele marcador
dentro dele - o instantâneo agora é tirado no primeiro `readdir`.

Depois: o menu principal, 2,1 GB residentes, daemon a 0 % de CPU.

`EIDOS_FUSE_TRACE=opendir` foi o que achou isso, e ele acompanha o programa. Os
contadores de operações dizem quantas; 195796 enumerações de um único diretório são
invisíveis dentro de um total.

**O jogo reescrever `plugins.txt` vazio** era muito provavelmente a mesma coisa - um
`Data` que ele não conseguia enumerar em tempo razoável, do que concluía que não
havia nada ali e salvava isso. Não provado, e vale reconferir. De qualquer forma, a
guarda de captura (uma captura que zera inteiramente o conjunto ativo é recusada em
qualquer tamanho) significa que ela não pode mais danificar o perfil.

**`FOPEN_KEEP_CACHE` está desligado.** Corrigido, e vale saber por quê. Ele
travava o Skyrim SE numa desreferência nula segundos após o menu principal, de
forma determinística, sem nenhum mod instalado; os outros três caches do lado do
kernel foram eliminados um a um por bissecção e só este importava. Essa
bissecção rodou com `FUSE_PASSTHROUGH` ativo, e o kernel faz falhar com EIO uma
abertura em passthrough que carrega `FOPEN_KEEP_CACHE`, então com a flag ligada
toda abertura em passthrough falhava. Talvez tenham sido essas aberturas falhas
que travavam o jogo, e a flag sozinha, com o passthrough desligado, nunca foi
testada. As respostas em passthrough não a carregam mais. Perdê-lo foi medido
como gratuito na época, mas com o passthrough ativo o daemon serve *zero*
leituras (`EIDOS_FUSE_STATS` reportou `read 0` numa carga completa), então com o
passthrough desligado por padrão (abaixo) o custo real está sem medição. Religue
com `EIDOS_FUSE_KEEP_CACHE=1` para testá-lo sozinho.

### O passthrough do FUSE impede o jogo de carregar qualquer conteúdo de mod

O daemon agora evita a causa mais provável (abaixo), mas isso ainda não foi
medido de novo em um jogo, então o passthrough continua desligado por padrão;
`EIDOS_FUSE_PASSTHROUGH=1` o liga. Antes dessa mudança, com o passthrough
ligado, o Skyrim SE falhava em abrir 152 dos próprios arquivos (75 `.bsa`, 65
`.esl`, 10 `.esm`, 2 `.esp`) com `STATUS_ACCESS_VIOLATION`, contra 0 com ele
desligado, no kernel 7.1.4 - então nenhum conteúdo de mod carregava, em
silêncio. O kernel levantava o erro depois que o daemon tinha respondido
`opened_passthrough`, então os logs do próprio daemon mostravam uma execução
limpa (zero aberturas falhas, zero arquivos de respaldo recusados).

O kernel associa um único arquivo de respaldo a cada inode (`fs/fuse/iomode.c`).
Uma abertura que traz um segundo identificador de respaldo para um inode que já
tem um falha com EIO, assim como uma abertura normal (sem `FOPEN_PASSTHROUGH`)
de um inode que tem arquivo de respaldo. Uma abertura em passthrough que carrega
`FOPEN_KEEP_CACHE` também falha com EIO. O daemon registrava um identificador
novo a cada abertura, então falhava a segunda abertura de um arquivo que o jogo
já mantinha aberto. O Wine reporta EIO como `STATUS_ACCESS_VIOLATION`, o erro
por trás das 152 aberturas falhas. Essa regra vem do código-fonte do kernel e
bate com essas 152 falhas, o que a torna a causa mais provável.

O daemon agora registra um único arquivo de respaldo por inode, compartilhado
por todas as suas aberturas em passthrough, e nunca põe `FOPEN_KEEP_CACHE` numa
abertura em passthrough. O passthrough só cobre aberturas somente leitura de
arquivos das camadas do jogo e dos mods que tenham 1 MiB ou mais ou terminem em
`.dll`, `.exe` ou `.asi`. Uma vez registrado esse arquivo de respaldo, toda
abertura somente leitura do arquivo o usa, e enquanto o arquivo tiver aberturas
normais, os novos leitores também recebem aberturas normais. Um arquivo do
Overwrite nunca passa pelo passthrough, e uma montagem que projeta os carimbos
de data dos plugins não o usa de jeito nenhum. Uma abertura para escrita de um
arquivo que leitores em passthrough mantêm aberto recebe ETXTBSY (no Wine:
`STATUS_SHARING_VIOLATION`) antes de qualquer copy-up, então uma abertura com
`O_TRUNC` não pode deixar no Overwrite uma cópia vazia que esconda o arquivo do
mod.
