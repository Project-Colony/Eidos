<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Fallout 4 através do Eidos

Com o Eidos, o Fallout 4 não precisa de nenhuma opção de inicialização especial,
nenhum executável renomeado e nenhum script embrulhador. Ele usa a mesma opção de
inicialização de todos os outros jogos.

## A opção de inicialização

```
~/.local/bin/eidos-gui %command%
```

O alvo de inicialização do Steam para Fallout 4 é `Fallout4Launcher.exe`, nunca
`Fallout4.exe`, então fazer o script extender rodar é na verdade a pergunta "como
faço o Steam iniciar outro programa". As respostas usuais reescrevem `%command%` em
bash:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

ou copiam `f4se_loader.exe` por cima de `Fallout4Launcher.exe`, que o Steam restaura
a cada atualização do jogo - depois disso você está jogando sem F4SE e nada avisa.

O Eidos faz a troca sozinho, a partir do descritor do jogo. Quando a janela inicia o
jogo, ela substitui `Fallout4Launcher.exe` por `f4se_loader.exe` se o loader estiver
na pasta do jogo, no `Root/` de um mod ativado ou em `Overwrite/Root/`. Sem o
loader, ela executa `Fallout4.exe`, e a linha de status diz
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
Quando nenhum dos dois arquivos é encontrado ali, ela executa o lançador sem
alteração e também avisa. Com `eidos play fallout4 -- %command%` como opção de
inicialização, a CLI só troca quando o loader existe e, caso contrário, executa
`Fallout4Launcher.exe`.

Há um segundo motivo para evitar o lançador: ele revarre `Data` e reescreve
`plugins.txt`, desfazendo a ordem de carga recém-implantada.

Quando você abre a janela por conta própria e clica em Run, o Eidos pede ao Steam
que inicie o jogo (`steam -applaunch 377160`), e o Steam inicia o Eidos pela opção
de inicialização acima. Se `f4se_loader.exe` não estiver ao lado do executável do
jogo na instância, ele não inicia, e a linha de status pede que você instale antes
o script extender. O Fallout 4 VR funciona do mesmo modo, com
`Fallout4VRLauncher.exe` e `f4sevr_loader.exe`.

## Do que o Eidos cuida por você

| Área | O que o Eidos faz |
|---|---|
| Invalidação de arquivos | `Fallout4Custom.ini` recebe `[Archive]` `bInvalidateOlderFiles=1` e um `sResourceDataDirsFinal=` vazio, as duas chaves que permitem que arquivos soltos fora de `Data` sejam sequer vistos. Escrito no perfil, não na pasta do jogo. |
| INIs | `Fallout4.ini`, `Fallout4Prefs.ini` e `Fallout4Custom.ini` pertencem ao perfil. O BethINI Pie executado pelo Eidos edita as cópias do perfil, e Backups -> INIs devolve a cópia que o Eidos guarda antes de cada execução dele ([tools.md](tools.md#bethini-pie)). |
| Ordem de carga | `plugins.txt` no formato de asterisco que o Fallout 4 usa (`*` marca ativo), com `Fallout4.ccc` respeitado para os plugins implícitos do Creation Club |
| LOOT | A ordenação funciona igual à do Skyrim - `eidos sort <instance>` busca a masterlist de `fallout4` |
| Saves | Saves `.fos` e seus cosaves `.f4se` são listados, copiados e mantidos por perfil; o painel de detalhes lê a tabela de plugins do próprio save, então um save que precisa de um plugin desativado avisa antes de você carregá-lo. Desativar ou remover um mod cujo plugin um dos três saves mais recentes do perfil usa pergunta antes. |
| Mods root | Tudo que um mod traz ao lado do executável (o próprio F4SE, ENB, um `dxvk.conf`) cai lá pelo mesmo mecanismo `Root/` que o Skyrim usa |

## A questão das versões

Fallout 4 não é mais o jogo congelado que foi entre 2019 e 2024. Em agosto de 2026
há três ramos vivos, e uma DLL de mod construída para um não carrega em outro:

| Ramo | Versão | F4SE |
|---|---|---|
| Clássico ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 a 1.11.240 | 0.7.4 a 0.7.9 |

Duas consequências que vale conhecer antes de montar uma lista de mods:

- Pastas `Creations/` e `Mods/` na raiz do jogo significam que você está na linha
  1.11.x. O painel de detalhes de um save no Eidos também mostra a build que o
  escreveu: o Fallout grava isso no save, e o Eidos exibe como "Game build".
- O F4SE costuma sair um ou dois dias depois de uma atualização da Bethesda, mas a
  *Address Library for F4SE Plugins* - por onde a maioria dos mods DLL resolve seus
  deslocamentos - segue o próprio calendário. Entre os dois, os mods DLL não
  carregam. Mods sem DLL (texturas, malhas, plugins) não são afetados, então um
  patch recém-saído é um mau dia para começar uma lista de mods.

Assim que sua pilha funcionar, congele a versão do jogo na instância: Freeze this
version no cartão Game files da aba Diagnostics, ou
`eidos stock-game fallout4 freeze` para ver o tamanho e
`eidos stock-game fallout4 freeze --yes` para copiar. O Eidos copia a instalação da
loja para `<instance>/Stock Game` e executa o jogo a partir dessa cópia, ainda
iniciado pelo Steam. Um patch posterior altera a instalação da loja e não a cópia,
então as DLLs que você instalou mantêm a versão do jogo delas. Em btrfs ou XFS a
cópia compartilha os blocos da loja e quase não ocupa espaço até o Steam alterar um
arquivo; em ext4 é uma cópia completa, do tamanho da pasta do jogo. O Eidos recusa
enquanto o Steam informa que o jogo está ocupado ou não totalmente instalado. Os
detalhes estão em
[usage.md](usage.md#arquivos-do-jogo-a-instalação-da-loja-ou-uma-cópia).

Sem uma cópia, desligue as atualizações automáticas do Steam para Fallout 4
(Propriedades, Atualizações, "Só atualizar este jogo quando eu o iniciar"), ou o
próximo patch quebrará toda DLL que você instalou.

## Nota de hardware: destroços de armas travam em NVIDIA

O efeito de destroços de armas do Fallout 4 roda sobre o NVIDIA FleX, um derivado do
PhysX que a NVIDIA parou de dar suporte depois da geração Pascal. Em qualquer placa
Turing ou mais nova - GTX 16, RTX 20 até RTX 50 - ele derruba o jogo. É um bug do
jogo, sem relação com Linux, Proton ou Eidos.

Dois consertos, qualquer um serve: desligue "Weapon Debris" nas configurações do
jogo, ou instale o *Weapon Debris Crash Fix* (Nexus 48078), que desativa a colisão
dos fragmentos em vez do efeito.

## Cartões Health para Fallout 4

A aba Diagnostics mostra um cartão Health para cada um destes casos:

- Plugins F4SE estão instalados em `F4SE/Plugins`, mas `f4se_loader.exe` não está
  em nenhum mod ativado nem na pasta do jogo. O jogo inicia sem eles.
- `f4se_loader.exe` está instalado, mas o `f4se_<version>.dll` da versão do jogo em
  execução (`f4se_1_10_984.dll` para 1.10.984) não está. O loader para com um erro;
  isso costuma acontecer depois de uma atualização do jogo.
- O executável do jogo ou sua build do Steam mudou desde a última inicialização
  pelo Eidos que terminou sem erro. Plugins F4SE construídos para a versão antiga
  podem deixar de carregar.
- Um mod com prioridade maior que a do F4SE tem cópias soltas `.pex` dos próprios
  scripts do F4SE. Uma cópia antiga quebra as funções do F4SE que ela declara, e o
  jogo não mostra erro.
- O `Fallout4Custom.ini` do perfil tem linhas `sTestFile1` a `sTestFile10` em
  `[General]`. Enquanto uma estiver definida, o jogo carrega esses plugins e ignora
  `plugins.txt`. Fix as remove e guarda o arquivo anterior como
  `Fallout4Custom.ini.pre-fix`.
- Um arquivo BA2 de um mod guarda áudio `.wav` ou `.xwm` comprimido. O jogo pode
  travar ao tocar esse som. Reempacote o arquivo com o áudio sem compressão. O
  Fallout 4 VR recebe o mesmo cartão.
- Um registrador de falhas escreveu um log na última inicialização: o
  `crash-*.log` do Buffout 4 em `Documents/My Games/Fallout4/F4SE`, ou um `.log` em
  `F4SE/Crashlogs`. O cartão nomeia a exceção e as DLLs e plugins do log que seus
  mods instalam.
- O cartão Display mostra o tamanho e o modo de janela definidos no
  `Fallout4Prefs.ini` do perfil, e Apply escreve ali `iSize W`, `iSize H`,
  `bFull Screen` e `bBorderless`.
- Um requisito do Nexus nomeia uma DLC. O Eidos procura um plugin com o nome da DLC,
  e os plugins de DLC do Fallout 4 têm outros nomes (`DLCCoast.esm` para Far
  Harbor), então este cartão é um aviso: confira à mão e clique em Ignore.

Os dois cartões do loader têm um botão Open the download page para
<https://f4se.silverlock.org/> e um botão Install the downloaded archive, que coloca
o loader no `Root/` do mod. `eidos check fallout4` imprime os mesmos achados, exceto
o cartão `.pex`, o cartão Display e os cartões de requisitos do Nexus, que só a
janela calcula.

## Se algo parece errado

A lista geral está em [troubleshooting.md](troubleshooting.md); a
primeira pergunta específica do Fallout é sempre *qual executável realmente
iniciou*. Quando a janela inicia o jogo, o Eidos escreve o comando de inicialização
completo no log de execução da instância, então:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Se nomear `f4se_loader.exe`, a troca aconteceu. Se nomear `Fallout4.exe`, o F4SE não
está instalado onde o Eidos consegue achar - o lugar dele é ao lado do executável do
jogo, o que numa instalação gerenciada significa o diretório `Root/` de um mod (ou a
própria pasta do jogo, instalado à mão). Se nomear `Fallout4Launcher.exe`, o Eidos
não achou nem o loader nem `Fallout4.exe` na visão root.

Quando o executável certo inicia e o jogo ainda trava, Test run e Find the mod
behind a problem isolam um único mod sem tocar nos saves do perfil
([usage.md](usage.md#execuções-de-teste)).
