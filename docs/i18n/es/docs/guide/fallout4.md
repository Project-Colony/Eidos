<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Fallout 4 a través de Eidos

Con Eidos, Fallout 4 no necesita ninguna opción de lanzamiento especial, ningún
ejecutable renombrado ni ningún script envoltorio. Usa la misma opción de
lanzamiento que todos los demás juegos.

## La opción de lanzamiento

```
~/.local/bin/eidos-gui %command%
```

El objetivo de lanzamiento de Steam para Fallout 4 es `Fallout4Launcher.exe`, nunca
`Fallout4.exe`, así que conseguir que el script extender siquiera se ejecute es en
realidad la pregunta «cómo hago que Steam arranque otro programa». Las respuestas
habituales reescriben `%command%` en bash:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

o copian `f4se_loader.exe` sobre `Fallout4Launcher.exe`, que Steam restaura en
cada actualización del juego, y después juegas sin F4SE sin que nada lo diga.

Eidos hace el cambio él mismo, a partir del descriptor del juego. Cuando la
ventana arranca el juego, sustituye `Fallout4Launcher.exe` por `f4se_loader.exe`
si el cargador está en la carpeta del juego, en el `Root/` de un mod activado o en
`Overwrite/Root/`. Sin el cargador ejecuta `Fallout4.exe`, y la línea de estado
dice
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
Si no encuentra ninguno de los dos archivos, ejecuta el lanzador sin cambios y
también lo dice. Con `eidos play fallout4 -- %command%` como opción de
lanzamiento, la CLI hace el cambio solo si existe el cargador, y si no ejecuta
`Fallout4Launcher.exe`.

Hay una segunda razón para evitar el lanzador: reexamina `Data` y reescribe
`plugins.txt`, deshaciendo el orden de carga recién desplegado.

Cuando abres la ventana tú mismo y pulsas Run, Eidos le pide a Steam que arranque
el juego (`steam -applaunch 377160`), y Steam arranca Eidos mediante la opción de
lanzamiento de arriba. Si `f4se_loader.exe` no está junto al ejecutable del juego
en la instancia, no lo hace, y la línea de estado te pide instalar antes el script
extender. Fallout 4 VR funciona igual, con `Fallout4VRLauncher.exe` y
`f4sevr_loader.exe`.

## De qué se encarga Eidos por ti

| Área | Qué hace Eidos |
|---|---|
| Invalidación de archivos | `Fallout4Custom.ini` recibe `[Archive]` `bInvalidateOlderFiles=1` y un `sResourceDataDirsFinal=` vacío, las dos claves que permiten que los archivos sueltos fuera de `Data` se vean siquiera. Escrito en el perfil, no en la carpeta del juego. |
| INI | `Fallout4.ini`, `Fallout4Prefs.ini` y `Fallout4Custom.ini` pertenecen al perfil. BethINI Pie ejecutado a través de Eidos edita las copias del perfil, y Backups -> INIs devuelve la copia que Eidos guarda antes de cada una de sus ejecuciones ([tools.md](tools.md#bethini-pie)). |
| Orden de carga | `plugins.txt` en el formato de asterisco que usa Fallout 4 (`*` marca activo), con `Fallout4.ccc` respetado para los complementos implícitos de Creation Club |
| LOOT | La ordenación funciona igual que en Skyrim: `eidos sort <instance>` obtiene la masterlist de `fallout4` |
| Partidas | Las partidas `.fos` y sus cosaves `.f4se` se listan, copian y guardan por perfil; el panel de detalle lee la tabla de complementos de la propia partida, así que una partida que necesita un complemento que has desactivado lo dice antes de cargarla. Desactivar o quitar un mod cuyo complemento usa una de las tres partidas más recientes del perfil pide confirmación antes. |
| Mods root | Todo lo que un mod trae junto al ejecutable (el propio F4SE, ENB, un `dxvk.conf`) aterriza ahí por el mismo mecanismo `Root/` que usa Skyrim |

## La cuestión de las versiones

Fallout 4 ya no es el juego congelado que fue entre 2019 y 2024. A agosto de 2026
hay tres ramas vivas, y una DLL de mod compilada para una no cargará en otra:

| Rama | Versión | F4SE |
|---|---|---|
| Clásica («old-gen») | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 a 1.11.240 | 0.7.4 a 0.7.9 |

Dos consecuencias que conviene conocer antes de montar una lista de mods:

- Carpetas `Creations/` y `Mods/` en la raíz del juego significan que estás en la
  línea 1.11.x. El panel de detalle de una partida en Eidos también muestra la
  compilación que la escribió: Fallout lo graba en la partida, y Eidos lo muestra
  como «Game build».
- F4SE suele aparecer uno o dos días después de una actualización de Bethesda,
  pero *Address Library for F4SE Plugins*, por donde la mayoría de los mods DLL
  resuelven sus desplazamientos, va a su propio ritmo. Entre ambos, los mods DLL
  no cargan. Los mods sin DLL (texturas, mallas, complementos) no se ven
  afectados, así que un parche recién salido es mal día para empezar una lista de
  mods.

Cuando tu montaje funcione, congela la versión del juego en la instancia: Freeze
this version en la tarjeta Game files de la pestaña Diagnostics, o
`eidos stock-game fallout4 freeze` para ver el tamaño y
`eidos stock-game fallout4 freeze --yes` para copiar. Eidos copia la instalación
de la tienda en `<instance>/Stock Game` y ejecuta el juego desde esa copia, que
sigue arrancando Steam. Un parche posterior cambia la instalación de la tienda y
no la copia, así que las DLL que instalaste conservan su versión del juego. En
btrfs o XFS la copia comparte los bloques de la tienda y casi no ocupa espacio
hasta que Steam cambia un archivo; en ext4 es una copia completa, tan grande como
la carpeta del juego. Eidos se niega mientras Steam informe de que el juego está
ocupado o no está instalado del todo. Los detalles están en
[usage.md](usage.md#archivos-del-juego-la-instalación-de-la-tienda-o-una-copia).

Sin copia, desactiva las actualizaciones automáticas de Steam para Fallout 4
(Propiedades, Actualizaciones, «Actualizar este juego solo al iniciarlo»), o el
siguiente parche romperá todas las DLL que instalaste.

## Nota de hardware: los restos de armas provocan cierres en NVIDIA

El efecto de restos de armas de Fallout 4 corre sobre NVIDIA FleX, un derivado de
PhysX que NVIDIA dejó de mantener tras la generación Pascal. En cualquier tarjeta
Turing o posterior (GTX 16, RTX 20 hasta RTX 50) cierra el juego. Es un fallo del
juego, nada que ver con Linux, Proton ni Eidos.

Dos arreglos, cualquiera sirve: desactiva «Weapon Debris» en los ajustes del juego,
o instala *Weapon Debris Crash Fix* (Nexus 48078), que desactiva la colisión de los
fragmentos en lugar del efecto.

## Tarjetas Health para Fallout 4

La pestaña Diagnostics muestra una tarjeta Health por cada uno de estos casos:

- Hay complementos de F4SE instalados en `F4SE/Plugins`, pero `f4se_loader.exe`
  no está en ningún mod activado ni en la carpeta del juego. El juego arranca sin
  ellos.
- `f4se_loader.exe` está instalado, pero falta el `f4se_<version>.dll` de la
  versión del juego en ejecución (`f4se_1_10_984.dll` para 1.10.984). El
  cargador se detiene con un error; suele ocurrir tras una actualización del
  juego.
- El ejecutable del juego o su compilación de Steam cambió desde el último
  lanzamiento a través de Eidos que terminó sin error. Los complementos de F4SE
  compilados para la versión anterior pueden dejar de cargar.
- Un mod con prioridad más alta que F4SE trae copias `.pex` sueltas de los
  scripts del propio F4SE. Una copia antigua rompe las funciones de F4SE que
  declara, y el juego no muestra ningún error.
- El `Fallout4Custom.ini` del perfil tiene líneas `sTestFile1` a `sTestFile10`
  en `[General]`. Mientras haya una definida, el juego carga esos complementos e
  ignora `plugins.txt`. Fix las quita y guarda el archivo anterior como
  `Fallout4Custom.ini.pre-fix`.
- El archivo BA2 de un mod guarda audio `.wav` o `.xwm` comprimido. El juego
  puede cerrarse al reproducir ese sonido. Vuelve a empaquetar el archivo con el
  audio sin comprimir. Fallout 4 VR recibe la misma tarjeta.
- Un registrador de cierres escribió un registro durante el último lanzamiento:
  el `crash-*.log` de Buffout 4 en `Documents/My Games/Fallout4/F4SE`, o un
  `.log` en `F4SE/Crashlogs`. La tarjeta nombra la excepción y las DLL y
  complementos del registro que instalan tus mods.
- La tarjeta Display muestra el tamaño y el modo de ventana definidos en el
  `Fallout4Prefs.ini` del perfil, y Apply escribe ahí `iSize W`, `iSize H`,
  `bFull Screen` y `bBorderless`.
- Un requisito de Nexus nombra un DLC. Eidos busca un complemento con el nombre
  del DLC, y los complementos de DLC de Fallout 4 tienen otros nombres
  (`DLCCoast.esm` para Far Harbor), así que esta tarjeta es un consejo:
  comprueba a mano y pulsa Ignore.

Las dos tarjetas del cargador tienen un botón Open the download page para
<https://f4se.silverlock.org/> y un botón Install the downloaded archive, que
pone el cargador en el `Root/` del mod. `eidos check fallout4` imprime los mismos
hallazgos salvo la tarjeta de `.pex`, la tarjeta Display y las tarjetas de
requisitos de Nexus, que solo calcula la ventana.

## Si algo parece ir mal

La lista general está en [troubleshooting.md](troubleshooting.md); la primera
pregunta propia de Fallout es siempre *qué ejecutable arrancó realmente*. Cuando
la ventana arranca el juego, escribe la orden de lanzamiento completa en el
registro de ejecución de la instancia, así que:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Si nombra `f4se_loader.exe`, el cambio ocurrió. Si nombra `Fallout4.exe`, F4SE no
está instalado donde Eidos pueda encontrarlo: le corresponde estar junto al
ejecutable del juego, lo que en un montaje gestionado significa el directorio
`Root/` de un mod (o la propia carpeta del juego, instalado a mano). Si nombra
`Fallout4Launcher.exe`, Eidos no encontró ni el cargador ni `Fallout4.exe` en la
vista raíz.

Cuando arranca el ejecutable correcto y el juego sigue cerrándose, Test run y
Find the mod behind a problem lo reducen a un solo mod sin tocar las partidas del
perfil ([usage.md](usage.md#ejecuciones-de-prueba)).
