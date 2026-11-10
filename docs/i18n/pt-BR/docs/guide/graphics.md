<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS e geração de quadros

O Community Shaders 1.4+ traz seu próprio upscaling (DLSS 4 / FSR 3.1 / XeSS, pelo
pacote separado "Upscaling - Community Shaders") e geração de quadros FSR 3.1. Tudo
isso funciona através do Eidos no Linux - o CS e seus pacotes se instalam como mods
comuns e a união serve suas DLLs como qualquer outro arquivo - mas três
configurações não são visíveis de dentro do jogo, e cada uma faz o recurso não
fazer nada, sem erro: a variável NVAPI, a tela cheia exclusiva e o teto de FPS do
Reflex.

A aba Diagnostics verifica as duas primeiras, e a linha do ENB mais abaixo, para um
jogo que tenha um prefixo Wine (o prefixo do Proton do Steam, ou o que o Heroic
criou). O teto do Reflex não é verificado. Quando a instância executa o jogo a
partir de uma cópia, como `Stock Game`, as verificações leem os arquivos da cópia,
não os da instalação da loja. `eidos check <instance>` imprime os mesmos cartões
num terminal, sem os botões Fix.

- O cartão NVAPI aparece para uma cópia Steam do Skyrim SE ou do Enderal SE quando
  um driver NVIDIA está carregado (`/proc/driver/nvidia/version` existe), o `Data`
  combinado tem algo em `Shaders/Upscaling`, a versão do Proton do jogo deixa o
  NVAPI desligado por padrão, e nem a opção de inicialização do Steam, nem o
  ambiente do próprio Eidos, nem o `launch-env.ini` do perfil define
  `PROTON_ENABLE_NVAPI=1`.
- O cartão de tela cheia exclusiva só aparece com o pacote Upscaling instalado,
  quando o `SkyrimPrefs.ini` do perfil tem `bFull Screen=1` e
  `SKSE/Plugins/SSEDisplayTweaks.ini` não define ao mesmo tempo `Fullscreen=false`
  e `Borderless=true`. O botão Fix dele grava `bFull Screen=0` e `bBorderless=1` no
  `SkyrimPrefs.ini` do perfil. A verificação lê o `SkyrimPrefs.ini` também para o
  Enderal SE, e um perfil do Enderal SE guarda `EnderalPrefs.ini`, então no Enderal
  SE use o cartão Display abaixo.
- O cartão do ENB aparece para qualquer jogo cujo `enblocal.ini` vencedor na visão
  root não tem `LinuxVersion=true` em `[GLOBAL]`. Ele também tem um botão Fix.

## A opção de inicialização de que o DLSS precisa

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Use o caminho absoluto do seu próprio `eidos-gui` ou `eidos-linux`, como em
[install.md](install.md). O cartão dá a linha a definir: a opção de inicialização
atual do jogo com `PROTON_ENABLE_NVAPI=1` na frente ou, quando não há nenhuma, a
linha montada com o caminho do Eidos em execução.

Sem a camada NVIDIA NVAPI (dxvk-nvapi), o CS não consegue inicializar o DLSS e cai
para o upscaling FSR, sem nada na tela dizendo por quê. Se uma versão do Proton
liga a camada sozinha está escrito no script `proton` dela, e o Eidos lê a linha
`use_nvapi` ali. Uma chave de desativação (`disablenvapi`) significa ligado por
padrão: GE-Proton11-9 e Proton 11.0 trazem essa forma (conferido em 2026-11-09).
Uma chave de ativação (`enablenvapi`) significa desligado, e só essa versão precisa
da variável. Defini-la não custa nada em máquinas sem NVIDIA. A geração de quadros
em si é FSR 3.1 e não precisa de NVAPI; só o upscaler DLSS precisa.

Para defini-la só em um perfil, ponha `env=PROTON_ENABLE_NVAPI=1` no
`launch-env.ini` desse perfil (View -> INI editor, última aba).

## A geração de quadros exige janela sem borda

A geração de quadros do CS roda sobre um proxy de apresentação D3D12 e recusa de
saída a tela cheia exclusiva. Com `bFull Screen=1` em `SkyrimPrefs.ini` ela nunca
engata: sem erro, sem mensagem, e o jogo fica na taxa de quadros base. O SSE
Display Tweaks define o modo no nível do motor, digam o que disserem os INIs:

```ini
[Render]
Fullscreen=false
Borderless=true
```

A janela fica idêntica (sem borda, na resolução nativa). Só muda o modo que o motor
informa, e esse modo é o que o CS verifica.

O cartão Display, em Diagnostics, define o tamanho e o modo de uma vez, para
Skyrim SE, Enderal SE e Fallout 4. Ele mostra o tamanho da maior tela conectada.
Com Borderless window selecionado, Apply grava `iSize W`, `iSize H`,
`bFull Screen=0` e `bBorderless=1` no `SkyrimPrefs.ini` do perfil
(`EnderalPrefs.ini` no Enderal SE, `Fallout4Prefs.ini` no Fallout 4). Quando o SSE
Display Tweaks define a janela, Apply grava também `Resolution`, `Fullscreen=false`
e `Borderless=true` numa cópia do INI dele no Overwrite
(`SKSE/Plugins/SSEDisplayTweaks.ini`), e o arquivo do próprio mod fica como está.
Com Start the game at once when a store opens Eidos ligado (Preferences, General,
Running a game), a janela ainda abre quando o perfil não tem tamanho de janela e o
SSE Display Tweaks não define a janela, para que o tamanho possa ser definido antes
da primeira inicialização.

Mais duas condições de ativação, com a mesma falha silenciosa:

- A tela atualiza a 120 Hz ou mais, ou `frameGenerationForceEnable` está definido
  nas configurações de upscaling do CS. A geração de quadros dobra a taxa
  apresentada, então o CS se recusa a armá-la em telas que não conseguem mostrar o
  resultado.
- O pacote Upscaling está instalado (sua árvore `Data/Shaders/Upscaling/` guarda as
  DLLs do Streamline e do FidelityFX). O CS sem ele mostra as entradas de menu e
  não consegue habilitar nada.

## O limite de quadros do Reflex pode estrangular a saída

As configurações Reflex do CS carregam um teto de FPS próprio (`reflexFPSLimit`,
com `reflexUseFPSLimit`). Um teto deixado num valor antigo - o nosso era 79, de uma
calibragem passada - fica depois da geração de quadros e corta exatamente os quadros
que ela produz: base 60 dobrada para 120, cortada de volta para 79, lê-se como "a
geração de quadros não faz nada". Num monitor de 144 Hz o teto Reflex usual é ~138.
Confira sempre que os quadros gerados parecerem sumidos.

## Interação conhecida: tela preta com SSE Display Tweaks

A combinação FG + Display Tweaks + DXVK tem uma falha conhecida de tela preta.
Conserto, nesta ordem:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Se não bastar, um `dxvk.conf` ao lado do executável do jogo (o diretório `Root/`
   de um mod coloca um lá) com
   `dxvk.enableGraphicsPipelineLibrary = False`

## Mods de textura misturados: PGPatcher

O Community Shaders sabe renderizar parallax, complex material e PBR, mas só onde
uma malha está preparada para isso. Historicamente isso significava instalar malhas
já patcheadas e depois um mod para desligar o efeito onde faltassem texturas:
cobertura limitada às malhas que você tinha, e CPU gasta em tempo de execução
desfazendo uma escolha feita na instalação.

O [PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) inverte
isso. Você instala as texturas que quiser, de qualquer tipo de shader, e ele
reescreve malhas e plugins para que cada superfície use a certa - inclusive os
registros de textura alternativa nos plugins, que o método antigo deixava de fora.
Sem ele, uma textura que precisa de um shader que a malha não pede carrega, mas não
renderiza como deveria, o que acontece mais numa instalação que mistura tipos de
textura.

Duas coisas antes de rodar no Linux:

- ele precisa do argumento `--ignore-mo2vfscheck` ou sai na hora reclamando do MO2.
  O Eidos fornece - veja
  [Ferramentas](tools.md#por-que-o-pgpatcher-precisa-de---ignore-mo2vfscheck) para
  saber o que é a flag e por que a verificação falha aqui;
- ele edita malhas, então roda depois de instalar todos os mods de textura e antes
  do DynDOLOD, que precisa ver as malhas finais. Mudar suas texturas depois
  significa rodar os dois de novo, nessa ordem.

A saída dele não vai para o Overwrite. O PGPatcher escreve só na Output location
definida no launcher dele, e recusa uma pasta dentro de `Data`. Aponte esse local
para uma pasta só dele, como uma nova pasta `PGPatcher Output` dentro do `mods/` da
instância. Mantenha esse mod desativado enquanto o PGPatcher roda, porque o
PGPatcher esvazia a pasta antes de escrevê-la de novo e o Eidos espera que os mods
ativados fiquem inalterados enquanto a visão está montada. Depois ative-o com
prioridade alta: ele foi feito para vencer.

Não dê à entrada do PGPatcher um mod em Capture output into e depois use esse mod
como Output location. O Eidos move as pastas `cfg/` e `log/` do próprio PGPatcher
para esse mod depois de cada execução, e o PGPatcher então para com
"Output directory has non-PGPatcher related files".

## ENB no Proton: LinuxVersion

O Jackify escreve `LinuxVersion=true` na seção `[GLOBAL]` do `enblocal.ini` de cada
ENB que instala no Proton. A documentação do próprio ENB não descreve essa chave. Um
cartão Health na aba Diagnostics avisa sobre um `enblocal.ini` sem a linha, e o
botão Fix dele grava em `overwrite/Root/` uma cópia do `enblocal.ini` vencedor com a
linha definida, de modo que o mod do ENB mantém seu arquivo.

Outro cartão avisa quando um ENB está na pasta do jogo (um `enblocal.ini`, um
`enbseries.ini` ou uma pasta `enbseries/`) enquanto um mod ativado traz o Community
Shaders (`SKSE/Plugins/CommunityShaders.dll`). Os dois podem rodar juntos. Se a
imagem parecer errada, desligue um deles no INI dele.

## Lendo os números depois

Quadros gerados existem só do lado da apresentação: o motor continua simulando na
taxa base, o Havok continua batendo na taxa base, e tudo que conta quadros *do
motor* (inclusive os contadores do próprio CS) continua reportando ~60 enquanto a
tela mostra ~120. O contador está certo, e é por isso que a geração de quadros é
segura para a física onde elevar a taxa do próprio motor não é.
`DXVK_HUD=fps` nas opções de inicialização mostra um contador se você quiser um na
tela.

Uma regra: interpolação no nível do driver (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) e a geração de quadros do CS são tecnologias
concorrentes. Use uma ou outra, nunca as duas.
