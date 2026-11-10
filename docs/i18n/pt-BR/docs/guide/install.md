<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Instalar o Eidos

Quatro caminhos de entrada. A Colony instala um único arquivo, `eidos-linux`, que
é o programa inteiro. O pacote de versão, o pacote do Arch e uma compilação a
partir do código dão `eidos` (a CLI), `eidos-gui` e o programa auxiliar estático de
prévia de NIF `eidos-nif-preview` lado a lado. O pacote do Arch e a compilação a
partir do código instalam também `eidos-fuse` e `eidos-launch`, as duas CLIs de
nível mais baixo. Todo caminho pode registrar o manipulador `nxm://` que faz o
botão "Mod Manager Download" do Nexus chegar à sua instância.

## O que você precisa antes

| Requisito | Por quê |
|---|---|
| Linux com FUSE | `fusermount3` no seu PATH. Toda distribuição atual já traz. |
| Um jogo no Proton, iniciado uma vez | O Steam só cria o prefixo Wine do jogo no primeiro início, e o Eidos trabalha dentro dele. O Eidos também detecta as cópias GOG e Epic do Skyrim Special Edition, as cópias Epic de Fallout 3 e Fallout: New Vegas e a cópia GOG de Stardew Valley. Elas rodam pelo Heroic ou pelo Legendary, com o Eidos como wrapper do jogo ali: veja [usage.md](usage.md#iniciar-o-jogo). |
| Um binário 7-Zip: `7z`, `7zz` ou `7za` | Para instalar arquivos de mods. `7zip` no Arch, `p7zip` na maioria das outras distribuições. |
| `par2` (opcional) | Dados de recuperação PAR2 para os backups do `eidos pack`. `par2cmdline` no Arch e no Fedora, `par2` no Debian e no Ubuntu. |

Sem root, sem daemon, sem editar `/etc/fuse.conf` e sem nada para acrescentar aos
seus grupos. O Eidos monta dentro de um espaço de nomes privado que pertence ao
processo do jogo.

Algumas configurações do sistema só aparecem como travamentos. A aba Diagnostics
verifica `vm.max_map_count` (1048576 ou mais), o limite rígido de arquivos abertos
(524288 ou mais) e se o jogo, o prefixo dele ou a instância estão em NTFS, exFAT ou
FAT; [troubleshooting.md](troubleshooting.md#o-sistema) tem o conserto de cada um.

## Via Colony

Procure Eidos na [Colony](https://github.com/Project-Colony/Colony) e instale; as
atualizações chegam pelo lançador. A Colony guarda um único arquivo,
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Execute-o e a janela abre.
Chamado por um link chamado `eidos`, o mesmo arquivo é a linha de comando. Ponha
um no seu `PATH` e registre o manipulador de downloads do Nexus:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

A janela não precisa do link: sempre que inicia um jogo ou uma ferramenta, ela
executa o próprio arquivo como a CLI. A prévia de modelos NIF precisa do
`eidos-nif-preview`, que só o pacote de versão, o pacote do Arch e as compilações a
partir do código trazem; sem ele, a prévia avisa, e nada mais é afetado. O
`eidos-linux` também não traz nenhum dos runtimes da Microsoft que o pacote
inclui ([abaixo](#os-runtimes-da-microsoft-no-pacote)): quando uma ferramenta
precisa de um, o Eidos o baixa da Microsoft assim que você inicia a instalação dos
pré-requisitos. Aponte o Steam para o próprio arquivo, como abaixo.

## Um pacote de versão

Baixe `eidos-linux-bundle.tar.gz` e os arquivos `.sig`, `.meta` e `.meta.sig`
dele na [versão mais recente](https://github.com/Project-Colony/Eidos/releases/latest),
e confira-os como mostra a
[Política de assinatura de código](../../README.md#política-de-assinatura-de-código)
do README. O pacote contém `eidos`, `eidos-gui` e `eidos-nif-preview` no nível
superior, com `install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, os ícones em
`assets/brand/png` e os avisos de terceiros em `licenses/` (nifly, omod,
microsoft-directx, microsoft-runtimes e os crates Rust) e a pasta `runtimes/`
descrita abaixo. Ele não traz `eidos-fuse` nem `eidos-launch`. Depois:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

O `install.sh` copia os binários para `~/.local/bin`, ou para `$XDG_BIN_HOME`
quando ela está definida. `--system` coloca em `/usr/local/bin` via sudo,
`--bindir DIR` em qualquer outro lugar, e `--from DIR` pega os binários de outro
diretório. O script também:

- registra o manipulador `nxm://` rodando `eidos nxm --register`, então o passo de
  registro da seção da Colony não é necessário;
- escreve `~/.local/share/applications/eidos.desktop` e os ícones do Eidos em
  `~/.local/share/icons/hicolor`;
- copia os avisos de licença para `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` com `--system`);
- copia `runtimes/` para `~/.local/share/Colony/Eidos/runtimes`, também com
  `--system`, de modo que os runtimes pertencem ao usuário que roda o script;
- termina imprimindo a opção de inicialização do Steam com o caminho absoluto do
  `eidos-gui` que instalou.

`--cap` pede ao sudo que conceda `CAP_SYS_ADMIN` ao `eidos` instalado (veja o
passthrough do FUSE abaixo). `--no-cap` é o padrão e o deixa sem root. Rodar de
novo é a forma prevista de atualizar. O pacote e o `eidos-linux` exigem Linux
x86_64 com glibc 2.39 ou mais recente.

### Os runtimes da Microsoft no pacote

A pasta `runtimes/` do pacote guarda os arquivos da Microsoft que as ferramentas
pedem: `VC_redist.x86.exe` e `VC_redist.x64.exe` (Visual C++ 2015-2022, o
pré-requisito `vcrun2022`), `windowsdesktop-runtime-8.0.12-win-x64.exe` (o .NET 8
Desktop Runtime, para `dotnet8` e `dotnetdesktop8`) e
`dotnet-runtime-10.0.10-win-x64.zip` (o runtime .NET 10, para `dotnet10`). O Eidos
roda ele mesmo os instaladores do Visual C++ e do .NET 8 no prefixo do jogo, sem
winetricks, e descompacta o zip do .NET 10 uma vez em
`~/.local/share/Colony/Eidos/runtimes`, onde toda instância o compartilha (78 MB
descompactado). Cada arquivo só é usado quando o SHA-256 dele bate com o que está
fixado no Eidos. O SDK do .NET 10 de que o Synthesis precisa (`dotnetsdk10`, cerca
de 300 MB) não está no pacote e é sempre baixado.

Um pacote descompactado também roda no lugar: o Eidos procura `runtimes/` ao lado
do próprio binário, depois da pasta de dados acima.

## Arch

```bash
cd packaging && makepkg -si
```

O PKGBUILD compila o `main` atual a partir do código-fonte. Ele instala `eidos`,
`eidos-gui`, `eidos-nif-preview`, `eidos-fuse` e `eidos-launch` em `/usr/bin`, com
um manipulador `nxm://` para todo o sistema, uma entrada de desktop e os ícones.
Depende de `fuse3`, `7zip` e `xdg-utils`. Não traz pasta `runtimes/`, então os
runtimes da Microsoft são baixados quando uma ferramenta precisa deles.
`winetricks` é opcional: o Eidos ainda o usa para os pré-requisitos que ele mesmo
não instala, como `dotnet48` e `xact`.

## A partir do código

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Compilar a partir do código-fonte exige Rust 1.92 ou mais recente, `just`, CMake
3.20+, um compilador C++17 e Python 3 para os testes do programa auxiliar nativo.
`just build` compila o workspace inteiro, então `eidos-fuse` e `eidos-launch`
também são compilados, e `just install` roda `packaging/install.sh` sobre a saída
da compilação, que os copia junto com o resto. Para comandos sem `just`, consulte
as [instruções do programa auxiliar nativo](../../../../../crates/eidos-nif-preview/README.md),
compile Rust com `cargo build --release --locked`, copie o programa auxiliar para
`target/release` e execute `packaging/install.sh --from target/release`. Uma
compilação a partir do código também não tem pasta `runtimes/`, então os runtimes
da Microsoft são baixados quando uma ferramenta precisa deles.

O perfil release usa uma unidade de geração de código, LTO completo e símbolos
removidos. Com rustc 1.99.1 e 3 jobs de compilação, uma compilação a frio de
`eidos` e `eidos-gui` levou 334 s, contra 243 s com LTO thin, e gerou um `eidos` de
20,9 MB e um `eidos-gui` de 49,7 MB. Cada binário embute oito DLLs DirectX
guardadas compactadas com gzip: 7,9 MB em vez de 17,9 MB.
`just profile=dev build` compila sem otimização.

## Depois: apontar o Steam para ele

O Eidos roda *como* o comando de inicialização do seu jogo, e é assim que ele
consegue montar antes de o jogo começar. No Steam, clique direito no jogo ->
Propriedades -> Opções de inicialização, e informe o caminho absoluto da janela do
Eidos seguido de `%command%`:

```
~/.local/bin/eidos-gui %command%
```

Use o caminho absoluto - o Steam não lê o `PATH` do seu shell. O caminho depende de
como você instalou o Eidos, e o `install.sh` imprime a linha do diretório que usou:

| Instalação | Opção de inicialização |
|---|---|
| Pacote de versão ou `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Pacote do Arch | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Aperte Jogar. O Eidos abre na instância daquele jogo; instale mods, ordene com o
LOOT, clique em Run. Ao sair, a montagem vai embora junto e sua instalação fica
exatamente como estava.

O jogo sempre inicia pela loja dele. Numa janela que você abriu por conta própria,
Run executa `steam -applaunch <appid>`, o Steam inicia o Eidos pela opção de
inicialização, e uma segunda janela do Eidos abre, onde você clica em Run. Enquanto
a opção de inicialização não iniciar o Eidos, Run não inicia nada e a linha de
status mostra a linha a definir. Com Start the game at once when a store opens
Eidos (Preferences, General, Running a game; desligado por padrão), Jogar no Steam
inicia o jogo sem a janela quando o Health não lista nenhum problema. Heroic e
Legendary funcionam do mesmo modo pela configuração de wrapper deles;
[usage.md](usage.md#iniciar-o-jogo) tem os detalhes.

A aba Diagnostics verifica a linha. Um cartão Health mostra a linha a definir
quando a opção de inicialização está vazia, não nomeia nenhum binário do Eidos,
aponta para um arquivo que não existe, não tem `%command%` depois do Eidos, ou
inicia outra cópia do Eidos que não a que está em execução.

### Se você prefere o terminal

```sh
eidos init skyrimse               # criar uma instância (informe uma pasta para torná-la portátil)
eidos install skyrimse mod.7z     # mods Simple / FOMOD / BAIN / root
eidos sort skyrimse               # ordenar a carga com o LOOT
eidos check skyrimse              # as verificações Health que não precisam de janela
eidos play skyrimse -- %command%  # rodar qualquer coisa através da visão combinada
eidos shell skyrimse              # um shell na visão combinada, as escritas vão para o Overwrite
```

Todo comando que aceita um identificador de jogo aceita também a pasta de uma
instância portátil - veja
[usage.md](usage.md#instâncias-global-e-portátil). O passeio completo está
em [usage.md](usage.md).

## Atualizar

A Colony atualiza o `eidos-linux` pelo lançador. Para o pacote de versão, rode o
`install.sh` do novo pacote; para o pacote do Arch, rode `makepkg -si` de novo;
para uma compilação a partir do código, faça pull e rode `just install`. A janela
pergunta ao GitHub pela versão mais recente ao iniciar, no máximo uma vez por dia,
e quando há uma mais nova mostra um link para as notas dela depois de Help na barra
de menus. Ela nunca se baixa nem se substitui sozinha. Preferences, About, Version
desliga a verificação, o modo offline a pula, e uma cópia instalada em `/usr` (o
pacote do Arch) nunca pergunta, porque o gerenciador de pacotes a atualiza.

## Opcional: passthrough do FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` concede a capacidade
opcional; ela não ativa o passthrough. `./install.sh --cap` e `just setcap` a
concedem ao `eidos` que instalam ou compilam. Num ponto de montagem `nosuid` o
kernel ignora capacidades de arquivo, então o `install.sh` pula a concessão com um
aviso e o `just setcap` para com um erro. A capacidade vai no binário `eidos`
separado do pacote de versão, do pacote do Arch ou de uma compilação a partir do
código, nunca no `eidos-linux`, que também é a janela
([SECURITY.md](../../../../../SECURITY.md)). Um binário recompilado ou reinstalado
é um arquivo novo sem a capacidade, então conceda-a de novo depois de cada
atualização, se você a usa.

`EIDOS_FUSE_PASSTHROUGH=1` é a opção separada para ativá-lo durante a execução, e
ele está desligado por padrão. Com o passthrough ligado, uma versão anterior do
daemon impedia o Skyrim SE de carregar qualquer conteúdo de mod. A causa mais
provável é que ela registrava um arquivo de respaldo novo a cada abertura, enquanto
o kernel aceita um só por inode. O daemon agora compartilha um único respaldo entre
todas as aberturas em passthrough de um arquivo, mas isso ainda não foi medido de
novo em um jogo. Até lá, ligue o passthrough só para testá-lo. As medições e a
causa estão em [troubleshooting.md](troubleshooting.md).

## Já deu algo errado?

[troubleshooting.md](troubleshooting.md) cobre as chaves de ambiente, como ler os
contadores de operações, os cartões Health e todo problema que já mordeu alguém até
agora.
