<!-- eidos-i18n: source=docs/guide/graphics.md sha=ee3cee732aafbf91d8085d03a74f7d4e0cfa224d -->

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

## Mods de texturas mezclados: PGPatcher

Community Shaders sabe representar parallax, complex material y PBR, pero solo
donde una malla esta preparada para ello. Historicamente eso significaba
instalar mallas preparcheadas y luego un mod para volver a apagar el efecto
alli donde faltaban las texturas: cobertura limitada a las mallas que uno tenia,
y CPU gastada en tiempo de ejecucion deshaciendo una decision que nunca debio
tomarse.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) le da la
vuelta. Usted instala las texturas que quiera, de cualquier tipo de shader, y el
reescribe mallas y plugins para que cada superficie use el correcto - incluidos
los registros de texturas alternativas de los plugins, que el metodo antiguo
dejaba fuera. Es la diferencia entre un paquete de texturas que se ve y uno que
solo se carga, y cuenta sobre todo en una instalacion mezclada, que es lo que es
cualquier orden de carga real.

Dos cosas antes de ejecutarlo en Linux:

- necesita el argumento **`--ignore-mo2vfscheck`** o sale al instante
  quejandose de MO2. Eidos lo aporta - vea
  [Herramientas](tools.md#por-que-pgpatcher-necesita---ignore-mo2vfscheck) para saber que es la opcion y por que la
  comprobacion falla aqui;
- edita mallas, asi que se ejecuta **despues** de instalar todos los mods de
  texturas y **antes** de DynDOLOD, que debe ver las mallas finales. Cambiar sus
  texturas mas tarde significa ejecutar ambos otra vez, en ese orden.

Su salida va al Overwrite como la de cualquier herramienta, donde un clic la
convierte en un mod. Dele a ese mod prioridad alta: esta hecho para ganar.

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
