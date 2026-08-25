<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Instalar o Eidos

Três caminhos de entrada. Todos entregam os mesmos dois binários - `eidos` (a
linha de comando) e `eidos-gui` - mais o manipulador `nxm://`, que faz o botão
"Mod Manager Download" do Nexus cair na sua instância.

## O que você precisa antes

| | |
|---|---|
| **Linux com FUSE** | `fusermount3` no seu PATH. Toda distribuição atual já traz. |
| **Um jogo no Proton, iniciado uma vez** | O Steam só cria o prefixo Wine do jogo no primeiro início, e o Eidos trabalha dentro dele. |
| **`7z`** | Para instalar arquivos de mods. `p7zip` na maioria das distribuições. |

Sem root, sem daemon, sem editar `/etc/fuse.conf` e sem nada para acrescentar aos
seus grupos. O Eidos monta dentro de um espaço de nomes privado que pertence ao
processo do jogo.

## Arch

```bash
cd packaging && makepkg -si
```

## Um pacote de versão

```bash
./install.sh
```

Instala em `~/.local/bin` por padrão. `--system` coloca em `/usr/local/bin`,
`--bindir DIR` em qualquer outro lugar. Rodar de novo é a forma prevista de
atualizar.

## A partir do código

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## Depois: apontar o Steam para ele

O Eidos roda *como* o comando de inicialização do seu jogo, e é assim que ele
consegue montar antes de o jogo começar. No Steam, clique direito no jogo ->
Propriedades -> Opções de inicialização:

```
~/.local/bin/eidos-gui %command%
```

Aperte Jogar. O Eidos abre na instância daquele jogo; instale mods, ordene com o
LOOT, clique em Run. Ao sair, a montagem vai embora junto e sua instalação fica
exatamente como estava.

Use o caminho absoluto - o Steam não lê o `PATH` do seu shell.

### Se você prefere o terminal

```sh
eidos init skyrimse               # criar uma instância (informe uma pasta para torná-la portátil)
eidos install skyrimse mod.7z     # mods Simple / FOMOD / BAIN / root
eidos sort skyrimse               # ordenar a carga com o LOOT
eidos play skyrimse -- %command%  # rodar qualquer coisa através da visão combinada
```

Todo comando que aceita um identificador de jogo aceita também a pasta de uma
instância portátil - veja [usage.pt-BR.md](usage.pt-BR.md). O passeio completo
está lá.

## Opcional: passthrough do FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` liga o passthrough FUSE do
kernel. Ele está **desligado por padrão e você quase certamente quer que
continue assim**: medido no Skyrim SE, ele impede o jogo de abrir os próprios
arquivos e plugins, de modo que os mods silenciosamente não carregam. A chave
existe para retestar o mecanismo, não porque seja recomendada.

Detalhes, e as medições por trás dessa decisão, em
[troubleshooting.pt-BR.md](troubleshooting.pt-BR.md).

## Já deu algo errado?

[troubleshooting.pt-BR.md](troubleshooting.pt-BR.md) cobre as chaves de
ambiente, como ler os contadores de operações e todo problema que já mordeu
alguém até agora.
