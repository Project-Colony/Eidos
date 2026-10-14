<!-- eidos-i18n: source=docs/guide/extensions.md sha=5f31f1cbc3dcbfb9655f3570f01e5ada1377268d -->

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

Uma extensão não pode fazer chamadas de volta ao Eidos, registrar-se a partir de um arquivo compactado nem desenhar seus próprios widgets. Instaladores estruturados do protocolo 1 podem retornar planos de arquivos verificados e perguntas com opções; o Eidos controla a revisão, a preparação temporária e a publicação. FOMOD e BAIN continuam sendo instaladores nativos. Extensões de prévia e informações sobre jogos salvos usam o mesmo host com respostas limitadas. Consulte o [contrato do protocolo e os exemplos executáveis](../../../../examples/extensions/README.md) para conhecer a prioridade de correspondência, as respostas CLI registradas, os limites e as verificações de identidade na repetição.
