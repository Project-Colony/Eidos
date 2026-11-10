<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Extensões

Uma extensão acrescenta uma entrada ao Eidos sem fazer parte do Eidos. É um
manifesto TOML nomeando um programa e, no máximo, esse programa.

Os manifestos ficam em `~/.config/Colony/Eidos/addons/`, um `.toml` por extensão.
Abra a pasta em View -> Extensions -> Open folder e clique em Reload - sem
reiniciar.

`id`, `kind` e `exec` são obrigatórios; `name` assume o `id` por padrão. `exec`
nomeia o programa. Um nome sem `/` é procurado no `PATH`. Se o programa não
existir, o Eidos ignora a extensão, e a linha dela em View -> Extensions diz o
motivo (`<exec> is not on PATH` ou `<exec> is not there`). `games` limita a
extensão a esses identificadores de jogo; sem ele, a extensão vale para todos os
jogos, e em outro jogo a linha dela diz "not for this game".

Um manifesto que o Eidos recusa aparece primeiro na lista, com o motivo: um
arquivo que não pode ser lido, TOML inválido ou campo obrigatório ausente, um
`id` vazio ou com espaço, um `exec` vazio, um `id` que um manifesto anterior já
tomou (os arquivos são lidos em ordem de nome), um `kind` que o Eidos não
conhece, ou um tipo instalador, prévia ou informações de jogo salvo sem
`protocol = 1` e sem o nome de arquivo `extensions`.

## Por que nada é carregado dentro do Eidos

O Mod Organizer 2 carrega plugins como bibliotecas compartilhadas e hospeda os de
Python através do Qt. Nenhum dos dois se transfere. Rust não tem ABI estável,
então uma biblioteca compartilhada compilada com outro compilador - ou outra flag
de otimização, ou outro conjunto de recursos de uma dependência comum - é
comportamento indefinido, e nada o reporta como divergência de versão. E os
widgets do Eidos são genéricos em tempo de compilação, de modo que uma biblioteca
não conseguiria sequer construir um para devolver, mesmo que a ABI fosse estável.

Uma extensão é um programa que o Eidos executa com as permissões de acesso ao sistema de arquivos da sua conta. Instale apenas programas auxiliares em que confia. O isolamento de processos e a verificação de respostas não fornecem uma sandbox do sistema operacional.

## Uma ferramenta

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # omita para todos os jogos
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Ela aparece em View -> Extensions com um botão Run e inicia desacoplada - o
Eidos não a espera, e a linha de status mostra o pid dela. A saída é descartada.
Sem `workdir`, ela inicia na pasta em que o próprio Eidos foi iniciado.

## Uma verificação

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Ela roda a cada atualização, na raiz da instância a menos que `workdir` diga
outra coisa, e imprime um achado por linha:

```
level<TAB>title<TAB>detail
```

onde `level` é `problem`, `advice` ou `ok`. O detalhe é opcional; uma linha sem
título é descartada. Tudo que não começa com um nível conhecido é ignorado, de
modo que saída de progresso e avisos perdidos não conseguem levantar uma linha
que pareça uma das verificações do próprio Eidos. Os achados aparecem na aba
Diagnostics como `<name> - <title>`, depois dos cartões de prefixo e do Nexus e
antes dos outros cartões Health.

Uma verificação tem três segundos. A que estoura é interrompida e reportada como
um problema contra si mesma - ela roda na mesma atualização que segue cada clique,
então uma travada congelaria a janela. O relógio corre até a verificação terminar
e seus pipes de saída serem fechados, de modo que um processo filho em segundo
plano que ainda os segura conta contra os três segundos. Uma verificação que
termina com erro e não imprime nada é reportada do mesmo jeito, com a primeira
linha da saída de erro, e o mesmo vale para a que imprime mais de 1 MiB em
qualquer um dos fluxos. Nos dois casos o Eidos não a executa de novo até você
clicar em Reload, e a linha dela diz "It will not be retried until you press
Reload." Uma verificação não lê nada da entrada padrão.

## Marcadores

Tanto `args` quanto `workdir` expandem estes:

| Marcador        | O que é                                      |
| --------------- | -------------------------------------------- |
| `{instance}`    | a raiz da instância                          |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | o nome do perfil ativo                       |
| `{profile_dir}` | o diretório do perfil ativo                  |
| `{game}`        | o identificador do jogo, p. ex. `skyrimse`   |
| `{game_name}`   | o nome de exibição do jogo                   |
| `{install}`     | a pasta do jogo que a instância executa      |
| `{data}`        | o diretório `Data` dessa pasta               |

`{install}` é a instalação da loja, ou a cópia do jogo quando a instância executa
uma (um `Stock Game` congelado ou a pasta de jogo própria de uma lista; veja
[usage.md](usage.md#arquivos-do-jogo-a-instalação-da-loja-ou-uma-cópia)). `{data}` a acompanha.

Uma verificação também recebe `{addon_dir}`, a pasta que guarda o manifesto dela,
e o mesmo vale para as extensões do protocolo 1 abaixo. Uma ferramenta não recebe.

Um marcador desconhecido nunca é esvaziado, o que transformaria `--out {typo}` em
`--out --next-flag`. O Eidos recusa executar uma ferramenta ou uma verificação
cujos marcadores não podem ser todos resolvidos e diz quais faltam: na linha de
status, no caso de uma ferramenta; na linha da verificação na aba Diagnostics, no
caso de uma verificação.

## O que uma extensão não pode fazer

Uma extensão não pode fazer chamadas de volta ao Eidos, registrar-se a partir de um arquivo compactado nem desenhar seus próprios widgets. Instaladores estruturados do protocolo 1 podem retornar planos de arquivos verificados e perguntas com opções; o Eidos controla a revisão, a preparação temporária e a publicação. FOMOD e BAIN continuam sendo instaladores nativos. Extensões de prévia e informações sobre jogos salvos usam o mesmo host com respostas limitadas. Consulte o [contrato do protocolo e os exemplos executáveis](../../../../guide/examples/extensions/README.md) para conhecer a prioridade de correspondência, as respostas CLI registradas, os limites e as verificações de identidade na repetição.
