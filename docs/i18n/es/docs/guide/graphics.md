<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS y generación de fotogramas

Community Shaders 1.4+ trae su propio escalado (DLSS 4 / FSR 3.1 / XeSS, mediante
el paquete aparte «Upscaling - Community Shaders») y generación de fotogramas FSR
3.1. Todo ello funciona a través de Eidos en Linux (CS y sus paquetes se instalan
como mods corrientes y la unión sirve sus DLL como cualquier otro archivo), pero
hay tres ajustes que no se ven desde dentro del juego, y cada uno hace que la
función no haga nada sin ningún error: la variable NVAPI, la pantalla completa
exclusiva y el tope de FPS de Reflex.

La pestaña Diagnostics comprueba los dos primeros, y la línea de ENB más abajo,
para un juego que tenga un prefijo de Wine (el prefijo de Proton de Steam, o el
que configuró Heroic). El tope de Reflex no se comprueba. Cuando la instancia
ejecuta el juego desde una copia como `Stock Game`, las comprobaciones leen los
archivos de la copia, no los de la instalación de la tienda.
`eidos check <instance>` imprime las mismas tarjetas en una terminal, sin los
botones Fix.

- La tarjeta NVAPI aparece para una copia de Steam de Skyrim SE o Enderal SE
  cuando hay un controlador NVIDIA cargado (existe `/proc/driver/nvidia/version`),
  el `Data` combinado tiene algo en `Shaders/Upscaling`, la versión de Proton del
  juego deja NVAPI desactivado por defecto, y ni la opción de lanzamiento de
  Steam, ni el entorno propio de Eidos, ni el `launch-env.ini` del perfil definen
  `PROTON_ENABLE_NVAPI=1`.
- La tarjeta de pantalla completa exclusiva aparece solo con el paquete Upscaling
  instalado, cuando el `SkyrimPrefs.ini` del perfil tiene `bFull Screen=1` y
  `SKSE/Plugins/SSEDisplayTweaks.ini` no define a la vez `Fullscreen=false` y
  `Borderless=true`. Su botón Fix escribe `bFull Screen=0` y `bBorderless=1` en el
  `SkyrimPrefs.ini` del perfil. La comprobación lee `SkyrimPrefs.ini` también para
  Enderal SE, y un perfil de Enderal SE contiene `EnderalPrefs.ini`, así que en
  Enderal SE usa en su lugar la tarjeta Display de más abajo.
- La tarjeta de ENB aparece para cualquier juego cuyo `enblocal.ini` ganador en la
  vista raíz no tenga `LinuxVersion=true` en `[GLOBAL]`. También tiene un botón
  Fix.

## La opción de lanzamiento que DLSS necesita

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Usa la ruta absoluta de tu propio `eidos-gui` o `eidos-linux`, como en
[install.md](install.md). La tarjeta da la línea que hay que poner: la opción de
lanzamiento actual del juego con `PROTON_ENABLE_NVAPI=1` delante, o, cuando no hay
ninguna, la línea construida con la ruta del Eidos en ejecución.

Sin la capa NVIDIA NVAPI (dxvk-nvapi), CS no puede inicializar DLSS y recae en el
escalado FSR, sin nada en pantalla que diga por qué. Si una versión de Proton
activa la capa por sí sola está escrito en su script `proton`, y Eidos lee ahí la
línea `use_nvapi`. Un interruptor de exclusión (`disablenvapi`) significa activada
por defecto: GE-Proton11-9 y Proton 11.0 traen esa forma (comprobado el
2026-11-09). Un interruptor de inclusión (`enablenvapi`) significa desactivada, y
solo esa versión necesita la variable. Definirla no cuesta nada en máquinas sin
NVIDIA. La generación de fotogramas en sí es FSR 3.1 y no necesita NVAPI; solo la
necesita el escalador DLSS.

Para fijarla solo en un perfil, pon `env=PROTON_ENABLE_NVAPI=1` en el
`launch-env.ini` de ese perfil (View -> INI editor, última pestaña).

## La generación de fotogramas exige ventana sin bordes

La generación de fotogramas de CS corre sobre un proxy de presentación D3D12 y
rechaza de plano la pantalla completa exclusiva. Con `bFull Screen=1` en
`SkyrimPrefs.ini` nunca se activa: sin error, sin mensaje, y el juego se queda en
su tasa base. SSE Display Tweaks fija el modo a nivel de motor, digan lo que digan
los INI:

```ini
[Render]
Fullscreen=false
Borderless=true
```

La ventana se ve idéntica (sin bordes, a resolución nativa). Solo cambia el modo
que informa el motor, y ese modo es lo que CS comprueba.

La tarjeta Display de Diagnostics fija el tamaño y el modo en un solo paso, para
Skyrim SE, Enderal SE y Fallout 4. Muestra el tamaño de la pantalla conectada más
grande. Con Borderless window seleccionado, Apply escribe `iSize W`, `iSize H`,
`bFull Screen=0` y `bBorderless=1` en el `SkyrimPrefs.ini` del perfil
(`EnderalPrefs.ini` para Enderal SE, `Fallout4Prefs.ini` para Fallout 4). Cuando
SSE Display Tweaks fija la ventana, Apply escribe además `Resolution`,
`Fullscreen=false` y `Borderless=true` en una copia de su INI en el Overwrite
(`SKSE/Plugins/SSEDisplayTweaks.ini`), y el archivo del propio mod queda como
está. Con Start the game at once when a store opens Eidos activado (Preferences,
General, Running a game), la ventana se abre igualmente cuando el perfil aún no
tiene tamaño de ventana y SSE Display Tweaks no fija la ventana, para poder
definir el tamaño antes del primer arranque.

Dos condiciones de activación más, con el mismo fallo silencioso:

- La pantalla se refresca a 120 Hz o más, o `frameGenerationForceEnable` está
  activado en los ajustes de escalado de CS. La generación de fotogramas duplica
  la tasa presentada, así que CS se niega a armarla en pantallas que no pueden
  mostrar el resultado.
- El paquete Upscaling está instalado (su árbol `Data/Shaders/Upscaling/` contiene
  las DLL de Streamline y FidelityFX). CS sin él muestra las entradas de menú y no
  puede habilitar nada.

## El límite de fotogramas de Reflex puede estrangular la salida

Los ajustes Reflex de CS llevan su propio tope de FPS (`reflexFPSLimit`, con
`reflexUseFPSLimit`). Un tope que quedó en un valor anterior (el nuestro estaba en
79 de un ajuste viejo) se sitúa después de la generación de fotogramas y recorta
exactamente los que esta produce: 60 de base duplicados a 120, recortados de vuelta
a 79, se lee como «la generación de fotogramas no hace nada». En una pantalla de
144 Hz el tope Reflex habitual ronda 138. Compruébalo siempre que parezcan faltar
fotogramas generados.

## Interacción conocida: pantalla negra con SSE Display Tweaks

La combinación FG + Display Tweaks + DXVK tiene un fallo conocido de pantalla
negra. Arreglo, por orden:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Si no basta, un `dxvk.conf` junto al ejecutable del juego (el directorio
   `Root/` de un mod coloca uno ahí) con
   `dxvk.enableGraphicsPipelineLibrary = False`

## Mods de texturas mezclados: PGPatcher

Community Shaders sabe representar parallax, complex material y PBR, pero solo
donde una malla está preparada para ello. Históricamente eso significaba
instalar mallas parcheadas de antemano y luego un mod para volver a apagar el
efecto allí donde faltaban las texturas: cobertura limitada a las mallas que uno
tenía, y CPU gastada en tiempo de ejecución deshaciendo una decisión tomada en la
instalación.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) le da la
vuelta. Instalas las texturas que quieras, de cualquier tipo de shader, y él
reescribe mallas y plugins para que cada superficie use la correcta, incluidos
los registros de texturas alternativas de los plugins, que el método antiguo
dejaba sin cubrir. Sin él, una textura que necesita un shader que su malla no
pide se carga pero no se ve como debería, y eso ocurre sobre todo en una
instalación que mezcla tipos de textura.

Dos cosas antes de ejecutarlo en Linux:

- necesita el argumento `--ignore-mo2vfscheck` o sale al instante quejándose de
  MO2. Eidos lo aporta; consulta
  [Herramientas](tools.md#por-qué-pgpatcher-necesita---ignore-mo2vfscheck) para
  saber qué es la opción y por qué la comprobación falla aquí;
- edita mallas, así que se ejecuta después de instalar todos los mods de texturas
  y antes de DynDOLOD, que debe ver las mallas finales. Cambiar tus texturas más
  tarde significa ejecutar ambos otra vez, en ese orden.

Su salida no va al Overwrite. PGPatcher solo escribe en la Output location
configurada en su lanzador, y rechaza una carpeta dentro de `Data`. Apunta esa
ubicación a una carpeta propia, por ejemplo una nueva carpeta `PGPatcher Output`
dentro de `mods/` de la instancia. Mantén ese mod desactivado mientras PGPatcher
se ejecuta, porque PGPatcher vacía la carpeta antes de volver a escribirla y Eidos
espera que los mods activados no cambien mientras la vista está montada. Actívalo
después con prioridad alta: está hecho para ganar.

No des a la entrada de PGPatcher un mod en Capture output into y uses luego ese
mod como Output location. Eidos mueve las carpetas `cfg/` y `log/` propias de
PGPatcher a ese mod después de cada ejecución, y PGPatcher se detiene entonces con
«Output directory has non-PGPatcher related files».

## ENB bajo Proton: LinuxVersion

Jackify escribe `LinuxVersion=true` en la sección `[GLOBAL]` de `enblocal.ini` para
cada ENB que instala bajo Proton. La documentación propia de ENB no describe esa
clave. Una tarjeta Health de la pestaña Diagnostics avisa de un `enblocal.ini` sin
esa línea, y su botón Fix escribe en `overwrite/Root/` una copia del
`enblocal.ini` ganador con la línea puesta, de modo que el mod de ENB conserva su
archivo.

Otra tarjeta avisa cuando hay un ENB en la carpeta del juego (un `enblocal.ini`,
un `enbseries.ini` o una carpeta `enbseries/`) mientras un mod activado trae
Community Shaders (`SKSE/Plugins/CommunityShaders.dll`). Los dos pueden funcionar
juntos. Si la imagen se ve mal, apaga uno de ellos en su INI.

## Leer los números después

Los fotogramas generados existen solo del lado de la presentación: el motor sigue
simulando a la tasa base, Havok sigue latiendo a la tasa base, y todo lo que cuenta
fotogramas *del motor* (los contadores de CS incluidos) sigue informando ~60
mientras la pantalla muestra ~120. El contador es correcto, y por eso la
generación de fotogramas es segura para la física allí donde subir la tasa del
propio motor no lo es. `DXVK_HUD=fps` en las opciones de lanzamiento muestra un
contador si lo quieres en pantalla.

Una regla: la interpolación a nivel de controlador (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) y la generación de fotogramas de CS son
tecnologías competidoras. Usa una u otra, nunca ambas.
