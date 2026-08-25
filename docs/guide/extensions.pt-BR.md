<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# Extensões

Uma extensão acrescenta uma entrada ao Eidos sem fazer parte do Eidos. É um
manifesto TOML nomeando um programa e, no máximo, esse programa.

Os manifestos ficam em `~/.config/Colony/Eidos/addons/`, um `.toml` por extensão.
Abra a pasta em **View -> Extensions -> Open folder** e clique em **Reload** - sem
reiniciar.

## Por que nada é carregado dentro do Eidos

O Mod Organizer 2 carrega plugins como bibliotecas compartilhadas e hospeda os de
Python através do Qt. Nenhum dos dois se transfere. Rust não tem ABI estável,
então uma biblioteca compartilhada compilada com outro compilador - ou outra flag
de otimização, ou outro conjunto de recursos de uma dependência comum - é
comportamento indefinido, não uma divergência de versão. E os widgets do Eidos são
genéricos em tempo de compilação, de modo que uma biblioteca não conseguiria
sequer construir um para devolver, mesmo que a ABI fosse estável.

Então uma extensão é um programa que o Eidos *executa*. Ela não pode derrubar a
janela, não pode corromper uma lista de mods, e continua funcionando através das
atualizações do Eidos.

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

Ela aparece em **View -> Extensions** com um botão Run e inicia desacoplada - o
Eidos não a espera.

## Uma verificação

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Ela roda a cada atualização e imprime um achado por linha:

```
level<TAB>title<TAB>detail
```

onde `level` é `problem`, `advice` ou `ok`. O detalhe é opcional. Tudo que não
começa com um nível conhecido é ignorado, de modo que saída de progresso e avisos
perdidos não conseguem levantar uma linha que pareça uma das verificações do
próprio Eidos. Os achados aparecem na aba **Health**, prefixados com o nome da
extensão.

Uma verificação tem três segundos. A que estoura é interrompida e reportada como
um problema contra si mesma - ela roda na mesma atualização que segue cada clique,
então uma travada congelaria a janela.

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
| `{install}`     | o diretório de instalação do jogo            |
| `{data}`        | o diretório `Data` do jogo                   |

Um marcador desconhecido é deixado exatamente como foi escrito em vez de ser
esvaziado, para que um erro falhe visivelmente em vez de transformar
`--out {typo}` em `--out --next-flag`. Executar uma ferramenta cujos marcadores
não podem ser todos resolvidos é recusado, e o Eidos diz quais faltam.

## O que uma extensão não pode fazer

Ela recebe valores e executa; não pode chamar de volta o Eidos, mudar a lista de
mods, nem desenhar nada na janela. Isso é proposital. Aquilo para que o MO2 usa
plugins e que REALMENTE precisa alcançar o interior - suporte a jogos,
instaladores, o motor de conflitos - aqui é embutido em vez de aparafusado: uma
definição de jogo é seu próprio TOML em `~/.config/Colony/Eidos/games/`, e os
instaladores FOMOD e BAIN são nativos.
