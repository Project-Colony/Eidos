<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders, DLSS e geração de quadros

O Community Shaders 1.4+ traz seu próprio upscaling (DLSS 4 / FSR 3.1 / XeSS, pelo
pacote separado "Upscaling - Community Shaders") e geração de quadros FSR 3.1. Tudo
isso funciona através do Eidos no Linux - o CS e seus pacotes se instalam como mods
comuns e a união serve suas DLLs como qualquer outra coisa - mas três coisas **não**
são descobríveis de dentro do jogo, e cada uma faz o recurso silenciosamente não
fazer nada. Esta página é a lista delas, aprendida no osso numa instalação real.

## A opção de inicialização de que o DLSS precisa

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

O Proton desliga sua camada NVIDIA NVAPI (dxvk-nvapi) a menos que o jogo esteja na
lista de permissões da Valve, e Skyrim não está. Sem ela o CS não consegue
inicializar o DLSS e cai para o upscaling FSR - quietinho, sem nada na tela dizendo
por quê. Definir a variável não custa nada em máquinas sem NVIDIA, então a opção de
inicialização segura é simplesmente a linha acima. A geração de quadros em si é FSR
3.1 e não precisa de NVAPI; só o upscaler DLSS precisa.

## A geração de quadros exige janela sem borda

A geração de quadros do CS roda sobre um proxy de apresentação D3D12 e recusa de
saída a tela cheia exclusiva. `bFull Screen=1` em `SkyrimPrefs.ini` significa que
ela nunca engata - sem erro, sem mensagem, só a taxa base. O conserto robusto é o
SSE Display Tweaks, que impõe o modo no nível do motor diga o que disserem os INIs:

```ini
[Render]
Fullscreen=false
Borderless=true
```

A janela fica idêntica (sem borda, na resolução nativa); só muda aquilo em que o
motor acredita - e aquilo em que o motor acredita é o que o CS verifica.

Mais duas condições de ativação, com a mesma falha silenciosa:

- **Atualização do monitor de 120 Hz ou mais**, ou defina
  `frameGenerationForceEnable` nas configurações de upscaling do CS. A geração de
  quadros dobra a taxa apresentada, então o CS se recusa a armá-la em telas que não
  conseguem mostrar o resultado.
- **O pacote Upscaling instalado** (sua árvore `Data/Shaders/Upscaling/` guarda as
  DLLs do Streamline e do FidelityFX). O CS sem ele mostra as entradas de menu e
  não consegue habilitar nada.

## O limite de quadros do Reflex pode estrangular a saída

As configurações Reflex do CS carregam um teto de FPS próprio (`reflexFPSLimit`,
com `reflexUseFPSLimit`). Um teto deixado em algum valor antigo - o nosso era 79 de
uma calibragem passada - fica depois da geração de quadros e corta exatamente os
quadros que ela produz: base 60 dobrada para 120, cortada de volta para 79, lê-se
como "a geração de quadros não faz nada". Num monitor de 144 Hz o teto Reflex usual
é ~138. Confira sempre que a saída gerada parecer sumida; é o segundo assassino
silencioso depois da tela cheia exclusiva.

## Interação conhecida: tela preta com SSE Display Tweaks

A combinação FG + Display Tweaks + DXVK tem uma falha conhecida de tela preta.
Conserto, nesta ordem:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Se não bastar, um `dxvk.conf` ao lado do executável do jogo (o diretório `Root/`
   de um mod coloca um lá) com
   `dxvk.enableGraphicsPipelineLibrary = False`

## Lendo os números depois

Quadros gerados existem só do lado da apresentação: o motor continua simulando na
taxa base, o Havok continua batendo na taxa base, e tudo que conta quadros *do
motor* (inclusive os contadores do próprio CS) continua reportando ~60 enquanto a
tela mostra ~120. Isso é comportamento correto, não um contador quebrado - e é por
isso que a geração de quadros é segura para a física onde elevar a taxa do próprio
motor não é. `DXVK_HUD=fps` nas opções de inicialização mostra um contador se você
quiser um na tela.

Uma regra: interpolação no nível do driver (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) e a geração de quadros do CS são tecnologias
concorrentes. Use uma ou outra, nunca as duas.
