<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders, DLSS y generación de fotogramas

Community Shaders 1.4+ trae su propio escalado (DLSS 4 / FSR 3.1 / XeSS, mediante
el paquete aparte «Upscaling - Community Shaders») y generación de fotogramas FSR
3.1. Todo ello funciona a través de Eidos en Linux - CS y sus paquetes se instalan
como mods corrientes y la unión sirve sus DLL como cualquier otra cosa - pero hay
tres cosas que **no** se pueden descubrir desde dentro del juego, y cada una hace
que la función no haga nada, en silencio. Esta página es la lista, aprendida a las
malas en una instalación real.

## La opción de lanzamiento que DLSS necesita

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton desactiva su capa NVIDIA NVAPI (dxvk-nvapi) salvo que el juego esté en la
lista blanca de Valve, y Skyrim no lo está. Sin ella CS no puede inicializar DLSS
y recae en el escalado FSR - calladamente, sin nada en pantalla que diga por qué.
Definir la variable no cuesta nada en máquinas sin NVIDIA, así que la opción de
lanzamiento segura es sencillamente la línea de arriba. La generación de
fotogramas en sí es FSR 3.1 y no necesita NVAPI; sólo lo necesita el escalador
DLSS.

## La generación de fotogramas exige ventana sin bordes

La generación de fotogramas de CS corre sobre un proxy de presentación D3D12 y
rechaza de plano la pantalla completa exclusiva. `bFull Screen=1` en
`SkyrimPrefs.ini` significa que nunca se activa - sin error, sin mensaje, sólo la
tasa base. El arreglo robusto es SSE Display Tweaks, que impone el modo a nivel de
motor digan lo que digan los INI:

```ini
[Render]
Fullscreen=false
Borderless=true
```

La ventana se ve idéntica (sin bordes, a resolución nativa); sólo cambia lo que el
motor cree - y lo que el motor cree es lo que CS comprueba.

Dos condiciones más de activación, con el mismo fallo silencioso:

- **Refresco de pantalla de 120 Hz o más**, o activa `frameGenerationForceEnable`
  en los ajustes de escalado de CS. La generación de fotogramas duplica la tasa
  presentada, así que CS se niega a armarla en pantallas que no pueden mostrar el
  resultado.
- **El paquete Upscaling instalado** (su árbol `Data/Shaders/Upscaling/` contiene
  las DLL de Streamline y FidelityFX). CS sin él muestra las entradas de menú y no
  puede habilitar nada.

## El límite de fotogramas de Reflex puede estrangular la salida

Los ajustes Reflex de CS llevan su propio tope de FPS (`reflexFPSLimit`, con
`reflexUseFPSLimit`). Un tope que quedó en un valor anterior - el nuestro estaba en
79 de un ajuste viejo - se sitúa después de la generación de fotogramas y recorta
exactamente los que ésta produce: 60 de base duplicados a 120, recortados de vuelta
a 79, se lee como «la generación de fotogramas no hace nada». En una pantalla de
144 Hz el tope Reflex habitual ronda 138. Compruébalo siempre que la salida
generada parezca faltar; es el segundo asesino silencioso tras la pantalla completa
exclusiva.

## Interacción conocida: pantalla negra con SSE Display Tweaks

La combinación FG + Display Tweaks + DXVK tiene un fallo conocido de pantalla
negra. Arreglo, por orden:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Si no basta, un `dxvk.conf` junto al ejecutable del juego (el directorio
   `Root/` de un mod coloca uno ahí) con
   `dxvk.enableGraphicsPipelineLibrary = False`

## Leer los números después

Los fotogramas generados existen sólo del lado de la presentación: el motor sigue
simulando a la tasa base, Havok sigue latiendo a la tasa base, y todo lo que cuenta
fotogramas *del motor* (los contadores de CS incluidos) sigue informando ~60
mientras la pantalla muestra ~120. Eso es comportamiento correcto, no un contador
roto - y es por lo que la generación de fotogramas es segura para la física allí
donde subir la tasa del propio motor no lo es. `DXVK_HUD=fps` en las opciones de
lanzamiento muestra un contador si lo quieres en pantalla.

Una regla: la interpolación a nivel de controlador (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) y la generación de fotogramas de CS son
tecnologías competidoras. Usa una u otra, nunca ambas.
