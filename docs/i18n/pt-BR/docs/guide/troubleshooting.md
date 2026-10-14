<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

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

O passthrough entrega ao kernel o arquivo de respaldo real, então as leituras pulam
este daemon inteiramente. É um ganho de vazão que aqui custa correção. Medido em A/B
no Skyrim SE 1.6.1170, proton-cachyos 11.0, kernel 7.1.4, a mesma ordem de carga de
82 plugins, com a única variável sendo se o binário carregava a capacidade:

| passthrough | falhas de `NtCreateFile` com `STATUS_ACCESS_VIOLATION` |
|-------------|---------------------------------------------------------|
| ligado      | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| desligado   | 0                                                       |

Com ele ligado o jogo não abre nenhum dos próprios arquivos nem plugins, o que
aparece dentro do jogo como mods que simplesmente não estão lá - sem erro, sem linha
de log. Com ele desligado, a mesma ordem de carga chega à jogatina com seus plugins,
arquivos e scripts Papyrus vivos.

A falha é invisível de dentro do daemon, o que a tornou cara de achar: nosso próprio
`open` tem sucesso toda vez e o kernel nunca recusa um arquivo de respaldo
(verificado ao longo de uma sessão inteira falhando com `EIDOS_FUSE_TRACE=open`:
zero `open FAILED`, zero `passthrough refused`). O erro é produzido depois que o
daemon respondeu `opened_passthrough`, então nenhum log do lado do daemon consegue
vê-lo. Também não depende da extensão - atinge arquivos e plugins igualmente, ou
seja, os arquivos que o jogo mantém abertos por toda a execução.

`EIDOS_FUSE_PASSTHROUGH=1` religa, para medir o que ele rende ou para retestar o
mecanismo. Os avisos de capacidade no lançador e na aba Diagnostics só aparecem
quando você o pediu.

Para lançar o próprio jogo através do Eidos, defina a opção de inicialização do
Steam como:

```
eidos play skyrimse -- %command%
```

Prefixe com `WINEDLLOVERRIDES="d3dcompiler_47=n"` se o Proton precisar do
d3dcompiler nativo para compilar shaders; o Eidos funde isso com qualquer
sobreposição de DLL que um mod traga (carregadores ENB/ReShade/`.asi`).

### O índice de camadas está mesmo em uso?

O índice é tudo ou nada e é construído em silêncio: `LayerStack::new` recebe ou um
mapa completo das camadas somente-leitura ou `None`, após o que cada consulta as
percorre exatamente como antes. Nada num log de sessão distingue os dois, então uma
pilha que caiu caladamente para o percurso parece idêntica a uma que funciona -
enquanto paga o custo antigo.

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

Não há API para "este sistema de arquivos é insensível a maiúsculas", então o
`get_dir_case_sensitivity` do Wine fareja o marcador que o CIOPFS deixa nos
diretórios que serve. Ausente, o Wine supõe SENSÍVEL, e toda busca cuja grafia não
bate byte a byte recai em ler o diretório INTEIRO para achar uma correspondência
insensível a maiúsculas. Jogos da Bethesda pedem `data/ccbgssse001-fish.bsa`
enquanto o arquivo é `ccBGSSSE001-Fish.bsa`, então isso disparava em quase todo
recurso: 4471 sondagens do marcador e 2236 releituras completas de diretório em oito
segundos, e 195796 enumerações de `Data` em noventa. O Skyrim SE nunca chegava ao
menu principal - ficava em 240 MB residentes enquanto o daemon queimava 92 % de um
núcleo.

O Eidos dobrava a caixa em `resolve_read` desde o início. Todo o custo estava em
nunca dizer isso. Agora `lookup` responde `.ciopfs`; `readdir` continua não o
listando.

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

**`FOPEN_KEEP_CACHE` está desligado.** Corrigido, e vale saber por quê. Ele travava o
Skyrim SE numa desreferência nula segundos após o menu principal, de forma
determinística, sem nenhum mod instalado; os outros três caches do lado do kernel
foram eliminados um a um por bissecção e só este importava. Perdê-lo foi medido como
gratuito na época, mas aquela medição foi feita com `FUSE_PASSTHROUGH` ativo, onde o
daemon serve *zero* leituras (`EIDOS_FUSE_STATS` reportou `read 0` numa carga
completa) e o kernel já estava cacheando aquelas páginas contra o arquivo de
respaldo. O passthrough agora vem desligado (abaixo), então aquele argumento não
vale mais e o custo real está sem medição - o travamento já basta para deixá-lo
desligado. Religue com `EIDOS_FUSE_KEEP_CACHE=1` para investigar; as duas flags não
estão mais emaranhadas, então agora ele pode ser testado sozinho.

### O passthrough do FUSE impede o jogo de carregar qualquer conteúdo de mod

Corrigido desligando-o; `EIDOS_FUSE_PASSTHROUGH=1` traz de volta. Com o passthrough
ligado, o Skyrim SE falha em abrir 152 dos próprios arquivos (75 `.bsa`, 65 `.esl`,
10 `.esm`, 2 `.esp`) com `STATUS_ACCESS_VIOLATION`, contra 0 com ele desligado, no
kernel 7.1.4 - então nenhum conteúdo de mod carrega, em silêncio. O kernel levanta o
erro depois que o daemon respondeu `opened_passthrough`, então os logs do próprio
daemon mostram uma execução limpa (zero aberturas falhas, zero arquivos de respaldo
recusados). A causa raiz no caminho do kernel não está estabelecida; a chave é
mantida para que se possa retestar, e para que o passthrough possa ser estreitado
apenas às DLLs caso o mapeamento de imagens acabe precisando dele.
