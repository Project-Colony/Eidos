<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=c365b34dbc69a832b1c9305498c94eb223b567a1 -->

# Solución de problemas y diagnóstico

Los interruptores de entorno, cómo leer los contadores de operaciones, las
tarjetas Health de la pestaña Diagnostics, cómo encontrar el mod detrás de un
cierre inesperado, qué adjuntar a un informe de errores y los problemas
conocidos con su historia.

## Diagnosticar el VFS

Dos variables de entorno cubren la mayoría de los casos en que el juego ve algo
con lo que el sistema de archivos no está de acuerdo:

```sh
EIDOS_FUSE_STATS=1                  # contadores de operaciones, volcados al desmontar
EIDOS_FUSE_NO_CACHE=1               # todas las cachés del lado del núcleo apagadas
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # o nómbralas una a una
```

La forma granular es lo que encontró el cierre de `FOPEN_KEEP_CACHE` descrito en
los problemas conocidos: apagar las cuatro responde «¿es la caché?», y solo
nombrarlas responde «¿cuál?». Los contadores responden a la otra mitad. Con el
passthrough activado (está desactivado por defecto), una carga que muestra
`read 0` es una en la que `FUSE_PASSTHROUGH` sirvió cada byte dentro del núcleo,
así que todo lo que ibas a afinar en la ruta de lectura ya es gratis.

Unos cuantos interruptores más acotan un fallo todavía más:

```sh
EIDOS_FUSE_TRACE=open,opendir       # registra cada open / opendir (1 = todos los canales)
EIDOS_FUSE_THREADS=1                # un solo hilo del demonio (por defecto: número de CPU, como mucho 4)
EIDOS_FUSE_OPENDIR=1                # conserva los manejadores de opendir aunque el núcleo pueda omitirlos
EIDOS_NO_INDEX=1                    # recorre las capas en lugar de usar el índice de capas
EIDOS_NO_OW_INDEX=1                 # lo mismo para el índice de pertenencia del Overwrite
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE activado otra vez, ver Problemas conocidos
```

Para un lanzamiento desde Steam, ponlos antes de la orden en la opción de
lanzamiento, por ejemplo `EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`.
La salida va al registro de la ejecución: el `logs/run-*.log` de la instancia
para un lanzamiento desde la ventana, la consola de Steam para un lanzamiento
desde Steam.

Para mirar la propia vista combinada, `eidos shell <instance>` abre una shell en
la carpeta del juego tal como la ve el juego, y
`eidos shell <instance> -- ls Data/meshes` ejecuta allí una sola orden; lo que se
escriba va al Overwrite
([usage.md](usage.md#una-shell-en-la-vista-combinada)). El botón Save tree de la
pestaña Data escribe todo el árbol `Data` combinado en un archivo de texto, un
archivo por línea con el mod del que viene, con tu carpeta personal oculta.

## Montar una unión a mano

`eidos-fuse` viene con el paquete de Arch y con las compilaciones desde el código
fuente: `just build` lo compila, y `just install` o `install.sh` lo copian cuando
está. El paquete de versión y `eidos-linux` no lo traen. Su línea de uso:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

La primera `--layer` gana en caso de conflicto; la última son tus datos de juego
intactos. El montaje solo necesita `/dev/fuse` y `fusermount3` (sin overlayfs, sin
Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... lee y escribe a través de /mnt/point ...
fusermount3 -u /mnt/point
```

Las escrituras aterrizan en `--overwrite <dir>` (un directorio temporal si se
omite), de modo que las propias capas siguen intactas incluso aquí.

## Por qué el passthrough está desactivado por defecto

El passthrough entrega al núcleo el archivo de respaldo real, de forma que las
lecturas se saltan este demonio por completo. El A/B de abajo se midió con una
versión anterior del demonio, antes del cambio descrito más abajo en el problema
conocido «El passthrough de FUSE impide que el juego cargue contenido de mods».
Ese cambio evita la causa más probable, pero aún no se ha vuelto a medir en un
juego, así que el passthrough sigue desactivado por defecto hasta que se mida.
Medido en A/B sobre Skyrim SE 1.6.1170, proton-cachyos 11.0, núcleo 7.1.4, el
mismo orden de carga de 82 complementos, con la única variable de si el binario
llevaba la capacidad:

| passthrough | fallos de `NtCreateFile` con `STATUS_ACCESS_VIOLATION` |
|-------------|---------------------------------------------------------|
| activado    | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| desactivado | 0                                                       |

Con él activado, el juego no abría ninguno de sus propios archivos ni
complementos, lo que en el juego se manifestaba como mods que no estaban, sin
error y sin línea de registro. Desactivado, el mismo orden de carga llegaba a la
partida con sus complementos, archivos y scripts de Papyrus activos.

El fallo era invisible desde dentro del demonio: su propio `open` tenía éxito
siempre y el núcleo nunca rechazaba un archivo de respaldo (verificado a lo largo
de una sesión completa fallida con `EIDOS_FUSE_TRACE=open`: cero `open FAILED`,
cero `passthrough refused`). El error se producía después de que el demonio
hubiera respondido `opened_passthrough`, así que ningún registro del lado del
demonio podía verlo. Tampoco dependía de la extensión: golpeaba archivos y
complementos por igual, es decir, los ficheros que el juego mantiene abiertos
durante toda su ejecución.

`EIDOS_FUSE_PASSTHROUGH=1` lo vuelve a activar, para medir lo que aporta o para
volver a probar el mecanismo. Los avisos de capacidad en el lanzador y en la
pestaña Diagnostics solo aparecen cuando lo has pedido.

## ¿Se está usando de verdad el índice de capas?

`LayerStack::new` construye un mapa completo de las capas de solo lectura o
ninguno. Sin mapa, cada consulta recorre las capas como antes: las respuestas son
las mismas, cargar una partida tarda unas tres veces más y nada más cambia a la
vista.

Cuando el índice está desactivado, el montaje dice por qué, una vez, al arrancar:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

El motivo nombra un directorio ilegible, una capa con más de 64 niveles de
directorios, más de 4.000.000 de entradas o `EIDOS_NO_INDEX`. La línea va al
registro de la ejecución: el `logs/run-*.log` de la instancia para un lanzamiento
desde la ventana, la consola de Steam para un lanzamiento desde Steam. Con
`EIDOS_FUSE_STATS=1`, el volcado al desmontar también tiene una línea
`layer index:`, y un contador `unindexed` distinto de cero en un montaje normal
significa que el índice estaba desactivado.

Los ejemplos de abajo comprueban lo mismo sin conexión, sobre las carpetas de una
instancia, sin montar nada:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` resuelve rutas reales con y sin el índice y compara los recorridos
de directorio. `index_agrees` comprueba que ambos respondan LO MISMO, en cada ruta
y cada listado de una instancia real. `listing_cost` mide lo que el mapa de hijos
combinados ahorra en `readdir`.

`EIDOS_NO_INDEX=1` fuerza el recorrido, para cuando lo que se depura es justamente
la diferencia entre las dos respuestas.

## Iniciar el juego con la CLI

Para lanzar el juego mediante la CLI `eidos` en lugar de la ventana, pon como
opción de lanzamiento de Steam la ruta absoluta de `eidos`, por ejemplo:

```
~/.local/bin/eidos play skyrimse -- %command%
```

No hace falta ningún prefijo `WINEDLLOVERRIDES`. Cuando un mod activado importa
`d3dcompiler_47`, Eidos despliega su copia incluida en el prefijo, la fuerza como
nativa y la fusiona con cualquier anulación de DLL que traiga un mod (cargadores
ENB/ReShade/`.asi`).

Eidos nunca arranca el juego por sí mismo a través de Proton.
`eidos tool <instance> run <title>` con una entrada que arranca el juego (el
cargador del script extender, el lanzador o el binario del juego) pide a la
tienda que posee la copia que lo arranque, como hace Run en una ventana que no
abrió ninguna tienda, e imprime qué hay que definir cuando el ajuste de la tienda
no arranca Eidos. `--print` imprime la orden de la tienda en lugar de ejecutarla.

## Un juego o una herramienta que no se cierra

Mientras un juego o una herramienta se ejecuta, el botón Run pasa a ser Stop, y la
tarjeta de bloqueo también tiene un botón Stop. Púlsalo dos veces. La segunda
pulsación termina todos los programas que inició la ejecución: primero SIGTERM y,
a los 10 segundos, SIGKILL para lo que quede. Lo que el juego no hubiera guardado
se pierde. Después, Eidos hace lo mismo que tras cualquier salida: los cambios de
INI y de plugins vuelven al perfil, se registra la salida de las herramientas y
los mods se desmontan. La ejecución se registra con el código de salida 143, sea
cual sea lo que devolvió el programa, de modo que `eidos sessions <instance> diff`
nunca la toma como la última ejecución buena. Unlock es distinto: devuelve la
ventana y deja el programa en marcha. Desde una terminal, `kill <pid>` sobre el
proceso `eidos` (el pid aparece en la tarjeta de bloqueo) hace lo mismo que Stop.

## Averiguar qué se rompió

Para encontrar qué cambio rompió el juego:

- `eidos sessions <instance> diff` lista lo que cambió desde la última ejecución
  del juego que terminó con código 0: mods, versiones, orden de plugins, INI y el
  ejecutable del juego
  ([usage.md](usage.md#qué-cambió-desde-la-última-ejecución-buena)).
- Test run, junto a Run, juega con los mods del perfil, sin mods, o con los mods
  seleccionados que están activados, más los masters que necesitan, y envía todo lo que escribe el juego
  a `<instance>/test-run/`. Después de la ejecución conservas o descartas cada
  grupo de archivos: partidas, cambios de INI, estado de plugins, salida del
  Overwrite y registros ([usage.md](usage.md#ejecuciones-de-prueba)).
- Find the mod behind a problem, en la pestaña Diagnostics, hace ejecuciones de
  prueba con la mitad de los sospechosos restantes cada vez y pregunta si el
  problema ocurrió. 200 sospechosos llevan unas 8 ejecuciones, y la búsqueda
  sobrevive al cierre de Eidos
  ([usage.md](usage.md#encontrar-el-mod-detrás-de-un-problema)).

Una ejecución detenida nunca rellena de antemano un Yes en esa búsqueda, porque se
registra con el código de salida 143 (ver arriba).

## Tarjetas Health

La pestaña Diagnostics muestra una tarjeta Health por cada problema que encuentra,
con su causa y, en algunas, un botón que lo arregla. Las tarjetas de la opción de
lanzamiento se describen en [install.md](install.md), las de NVAPI, pantalla
completa y ENB en [graphics.md](graphics.md), y las de Proton y el prefijo en la
sección siguiente. `eidos check <instance>` imprime los hallazgos que no necesitan
ventana ([usage.md](usage.md#comprobaciones-de-salud-desde-la-terminal) los
lista), entre ellos las tarjetas del prefijo, del registro y de la unidad `S:`.
Solo la ventana muestra la tarjeta Proton con sus variantes de Flatpak, versión
desconocida y versión más antigua, el orden de parches, los scripts del extender,
los archivos que ningún plugin carga, los requisitos de Nexus, las copias
obsoletas de plugins y la tarjeta Display.

### El sistema

- «vm.max_map_count is <value>» aparece por debajo de 1048576, el valor por
  defecto en Arch Linux, Fedora 39 y posteriores, y Ubuntu 24.04 y posteriores.
  Algunos juegos bajo Proton mapean más regiones de memoria y entonces se cierran
  sin error. Pon `vm.max_map_count = 1048576` en un archivo bajo
  `/etc/sysctl.d`, por ejemplo `80-gamecompatibility.conf`, y ejecuta
  `sudo sysctl --system`.
- «The open files limit is <value>» aparece cuando el límite duro está por debajo
  de 524288, que necesita el esync de Proton; por debajo, los juegos pueden no
  arrancar o cerrarse. systemd 240 y posteriores lo fijan. En una instalación más
  antigua, pon `DefaultLimitNOFILE=524288` en `/etc/systemd/system.conf` y en
  `/etc/systemd/user.conf`, y vuelve a iniciar sesión.
- «Game files on a Windows file system» nombra la carpeta del juego, el prefijo o
  la raíz de la instancia cuando está en `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g,
  exfat-fuse), `exfat` o `vfat`. Proton necesita enlaces simbólicos, permisos
  Unix y nombres que distingan mayúsculas. Mueve el juego, su prefijo y la
  instancia a un sistema de archivos de Linux como ext4, btrfs o xfs.

### Los archivos del juego

La tarjeta Game files, arriba en la pestaña, muestra si la instancia ejecuta la
instalación de la tienda o una copia del juego dentro de la instancia
([usage.md](usage.md#archivos-del-juego-la-instalación-de-la-tienda-o-una-copia)).
Los lanzamientos se rechazan, con una tarjeta que dice por qué, en tres casos:

- «The game copy '<folder>' is missing»: la instancia ejecuta una copia que ya no
  existe. Devuélvela a su sitio, o vuelve a la versión de la tienda.
- «'<folder>' has no <binary>»: la carpeta que ejecuta la instancia no contiene
  ningún ejecutable del juego, así que no es una copia del juego.
- «<game> cannot run from a copy yet»: Morrowind, y Oblivion con
  `bUseMyGamesDirectory=0`, guardan su lista de plugins en la carpeta del juego.

«ModOrganizer.ini points tools at another game folder» aparece cuando el
`ModOrganizer.ini` propio de una lista, no escrito por Eidos, tiene un `gamePath`
distinto de los archivos del juego que ejecuta la instancia. Las herramientas en
modo Mod Organizer 2, como PGPatcher, leen el juego desde ahí. Cambia `gamePath`
en ese archivo, o quita el archivo para que Eidos escriba el suyo.

### El orden de carga

- «sTestFile lines make the game ignore plugins.txt» aparece para Fallout 4,
  Fallout 4 VR y Starfield cuando el `Fallout4Custom.ini` o el
  `StarfieldCustom.ini` del perfil tiene líneas `sTestFile1` a `sTestFile10` en
  `[General]`. Mientras haya una definida, el juego carga esos plugins y el orden
  de carga que escribe Eidos no tiene efecto. Fix quita las líneas de la copia del
  perfil y guarda el archivo anterior como `<ini>.pre-fix`.
- Starfield: los plugins `.esp` activados sin la marca de master reciben una
  tarjeta, porque Starfield no puede ordenarlos entre los `.esm` y los plugins con
  marca de master y mantiene cargados durante toda la sesión algunos de sus
  registros. Una versión `.esm` del autor evita ambas cosas.
- Starfield: «Invalid blueprint plugins» lista los plugins que tienen algunas,
  pero no todas, de estas tres cosas: la marca blueprint, un nombre que empieza
  por `BlueprintShips-` y la extensión `.esm`. «Blueprint plugins without their
  main plugin» lista los que tienen el plugin principal sin activar:
  `BlueprintShips-Example.esm` necesita `Example.esm`, `.esp` o `.esl`.
- «<patch> loses <n> file(s) to <master>» aparece cuando un plugin de un mod
  necesita un master de otro mod, y ese otro mod tiene la prioridad más alta, de
  modo que ganan sus copias de los archivos que ambos traen. Un parche suele ir
  por debajo del mod que parchea, y el botón Move de la tarjeta lo coloca ahí.
  Mantén el orden si el parche debe perder esos archivos.
- «Out-of-date copy of <plugin>» aparece cuando un mod contiene una copia limpiada
  o con marca light de un plugin cuyo original cambió después (una actualización
  de un mod o del juego). El juego carga la copia antigua. Ocúltala y limpia o
  marca de nuevo el plugin nuevo.

### La lista de mods

- «Order rules of the collection <name> that no longer hold» aparece cuando los
  mods de una colección instalada ya no están en el orden que piden sus reglas.
  Restore collection order solo devuelve a su sitio los mods de esa colección, y
  Ctrl+Z lo deshace.
- «<mod> requires <mod>, which is not installed» (o «which is disabled») viene de
  los requisitos de Nexus que lee Check for updates. Open on Nexus abre la página
  del mod requerido, Download main file lo descarga con una cuenta premium, e
  Ignore oculta un requisito que otro mod ya cumple. Un requisito de DLC es solo
  un consejo: Eidos busca un plugin con el nombre del DLC.
- «fontconfig.txt names a font library that is missing» (motores de Skyrim) nombra
  las fuentes que no tiene ninguna capa ni ningún archivo, y el mod cuyo
  `Interface/fontconfig.txt` gana. El texto de los menús puede verse como
  cuadrados hasta que se instale la fuente o se quite la línea.
- «BA2 archive stores audio compressed» (Fallout 4 y Fallout 4 VR) cuenta los
  archivos `.wav` y `.xwm` que el BA2 de un mod guarda comprimidos. El juego puede
  cerrarse al reproducirlos. Vuelve a empaquetar el archivo con el audio sin
  comprimir.

### El script extender y las actualizaciones del juego

- «<loader> is not installed» aparece cuando hay plugins del script extender
  instalados (en `SKSE/Plugins`, `F4SE/Plugins` y así sucesivamente) pero el
  cargador no está en ningún mod activado ni en la carpeta del juego. El juego
  arranca sin ellos.
- «The script extender does not match game version <version>» aparece cuando el
  cargador está instalado pero falta la DLL de runtime de la versión del juego en
  ejecución, por ejemplo `skse64_1_6_1170.dll`. El cargador se detiene con un
  error. Suele ocurrir tras una actualización del juego. Esta tarjeta y la de
  arriba tienen un botón Open the download page para el sitio del extender y un
  botón Install the downloaded archive, que pone el cargador en la carpeta
  `Root/` del mod.
- «<game> changed since the last clean launch» aparece cuando el ejecutable del
  juego, o en una copia de Steam su build id, cambió desde el último lanzamiento
  a través de Eidos que terminó sin error. Una actualización de Steam que deja el
  ejecutable como estaba no la dispara. Los plugins del script extender
  compilados para la versión anterior pueden dejar de cargar, y las tarjetas de
  debajo nombran los que lo declaran. Para conservar la versión del juego sobre
  la que se montó una lista, congélala en la instancia con Freeze this version en
  la tarjeta Game files, o con `eidos stock-game <instance> freeze --yes` después
  de que `freeze` a secas haya mostrado el tamaño: las actualizaciones posteriores
  de la tienda cambian la instalación de la tienda, no la copia.
- «<mod> replaces <n> <extender> script(s)» aparece cuando un mod con prioridad
  más alta que el script extender trae copias `.pex` sueltas de los scripts del
  propio extender. Una copia antigua rompe las funciones que declara, y el juego
  no muestra ningún error. Mueve el extender por debajo de ese mod (el botón Move
  de la tarjeta lo hace cuando ambos son mods), u oculta esos archivos en el mod.
  Cuando los scripts del extender están sueltos en el `Data` del juego, instala el
  extender como un mod para poder ordenarlo. Los archivos comprimidos no se
  comprueban, solo los sueltos.

### La salida de las herramientas

- «Overwrite/Root holds meshes» (o texturas) significa que una herramienta cuya
  ruta de datos apuntaba a la carpeta del juego escribió allí esas carpetas, donde
  el juego nunca las lee. Corrige la ruta de la herramienta, borra las carpetas y
  vuelve a ejecutarla. El botón Open folder abre `Overwrite/Root`.
- «BodySlide's GameDataPath is not the game's Data folder» (o `OutputDataPath`):
  consulta [Herramientas](tools.md#los-ajustes-propios-de-una-herramienta-siguen-siendo-suyos).

### Un cierre inesperado en la última sesión

«The game crashed in the last session» aparece cuando un registrador de cierres
escribió un registro durante el último lanzamiento. La tarjeta nombra la excepción
y las DLL y plugins del registro que instalan tus mods o el Overwrite. Tómalos
como pistas: un mod nombrado en un registro de cierre suele ser donde se vio el
cierre, no su causa. Cuando el registro no nombra ningún archivo de tus mods, la
tarjeta lo dice. Open the log lo abre, y Copy the log (home path hidden) lo copia
con tu carpeta personal oculta.

### El tamaño de la ventana

Para Skyrim SE, Enderal SE y Fallout 4, una tarjeta Display muestra el tamaño y
el modo de ventana con los que el perfil arranca el juego, y dice «No window size
set for this profile» cuando ni el INI de preferencias del perfil ni el del
prefijo definen uno. Apply escribe el tamaño y el modo en la copia del perfil, y
en una copia del INI de SSE Display Tweaks en el Overwrite cuando ese mod fija la
ventana ([usage.md](usage.md#diagnostics)).

## Qué Proton ejecuta el juego, y un prefijo roto

La pestaña Diagnostics tiene una tarjeta Proton: la versión que usan Eidos y Steam
para el juego, el ajuste propio de Steam y la versión que ejecutó el prefijo por
última vez. Cuando un Proton más reciente que el seleccionado fue el último en
preparar el prefijo, la tarjeta pasa a ser un consejo, porque un Proton más
antiguo sobre un prefijo más reciente puede restablecer sus archivos de Wine o no
arrancar. Elige en Steam una versión al menos igual de reciente, en Propiedades
del juego, Compatibilidad, o reconstruye el prefijo.

Las demás tarjetas de Proton y del prefijo:

- «Proton comes from the Flatpak Steam install»: ese Proton guarda su runtime y
  sus bibliotecas steamclient dentro del sandbox de Flatpak, así que ejecutarlo
  desde el anfitrión puede fallar. Eidos no lo arranca mediante `flatpak run`, lo
  que dejaría el juego en el sandbox, fuera de la vista combinada. Instala un
  Proton en `~/.steam/root/compatibilitytools.d/` y selecciónalo para el juego.
- «Eidos cannot tell which Proton runs this game»: el `config.vdf` de Steam no
  nombra ninguna herramienta de compatibilidad para él, y el prefijo no registra
  ninguna versión instalada. Las herramientas no arrancan hasta que se elija una
  en Steam (Propiedades del juego, Compatibilidad).
- «<game> has no Proton prefix yet»: lanza el juego una vez desde Steam y pulsa
  F5. Si Steam no crea el prefijo, comprueba que la carpeta del juego bajo
  `steamapps/compatdata` sea escribible; una vacía que pertenezca a root lo
  bloquea sin mensaje.
- «Windows tools cannot find <game>»: el registro del prefijo con la ruta de
  instalación es incorrecto o falta, y xEdit, DynDOLOD, Wrye Bash y el Creation
  Kit lo leen. Eidos escribe la ruta correcta la próxima vez que ejecutes una
  herramienta.
- «The prefix's S: drive points outside the Steam library»: `S:` no coincide ni
  con la raíz de la biblioteca, adonde Eidos la apunta para una herramienta, ni
  con `<library>/steamapps`, adonde Steam la apunta para el juego. Probablemente
  la biblioteca se movió, o el prefijo vino de otra máquina. Toda ruta relativa a
  `S:` guardada en el prefijo no lleva a ninguna parte hasta que el siguiente
  lanzamiento a través de Eidos o de Steam la apunte otra vez.
- «<store> installation: started through its store» es la tarjeta de una copia de
  GOG o Epic. Run pide a Heroic o Legendary que arranquen el juego cuando Eidos es
  el wrapper del juego allí, y las herramientas se ejecutan con la versión de Wine
  o Proton que Heroic registra para la copia.

El menú File ejecuta winecfg, regedit, el explorador de archivos de Wine y el
desinstalador en el prefijo del juego, con el Proton que ejecuta las herramientas
del juego ([tools.md](tools.md#los-programas-propios-de-wine-en-el-prefijo-del-juego)).
Sus filas siguen en gris hasta que el prefijo tiene esos programas.

El botón Rebuild prefix de la tarjeta Proton es para un prefijo que ya no
funciona. Púlsalo dos veces. Eidos no borra nada: renombra
`steamapps/compatdata/<appid>` a `<appid>.eidos-backup-<date>-<time>` en la misma
carpeta, y Steam crea un prefijo nuevo la próxima vez que inicies el juego desde
Steam. Se niega mientras un juego, una herramienta o cualquier programa de Wine
use el prefijo. Hasta que exista el prefijo nuevo, Run en la ventana no puede
entregar el juego a Steam y muestra en su lugar la opción de lanzamiento, así que
pulsa Jugar en Steam. Las partidas guardadas y los ajustes que solo estén en el
prefijo antiguo se quedan en la carpeta renombrada: copia de vuelta lo que
necesites y borra esa carpeta tú mismo cuando ya no la quieras.

## Qué adjuntar a un informe de errores

- Copy report, en la pestaña Diagnostics, copia la versión de Eidos, el juego, la
  versión de Proton, si Steam es el de Flatpak, el estado del passthrough, el tipo
  de sesión, la distribución, el núcleo, `vm.max_map_count`, el límite de archivos
  abiertos, si `/dev/fuse` y `fusermount3` están presentes, los nombres de las
  variables de lanzamiento y de los programas wrapper del perfil, y cada tarjeta
  Health. Tu carpeta personal aparece como `~` y tu nombre de usuario como
  `<user>`. `eidos check <instance>` imprime la misma cabecera en una terminal, y
  `--json` la da como JSON.
- El registro de la ejecución, `<instance>/logs/run-*.log`, empieza con la orden
  exacta que ejecutó Eidos. File, Eidos logs abre la carpeta de los registros
  propios de Eidos, `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden), en la tarjeta de cierre inesperado, copia el
  registro del cierre.
- Save tree, en la pestaña Data, guarda el árbol `Data` combinado con el origen de
  cada archivo.

## Problemas conocidos

### DLSS o la generación de fotogramas no hace nada, en silencio

Tres causas distintas, cada una sin mensaje de error alguno: NVAPI no habilitado en
las opciones de lanzamiento, pantalla completa exclusiva, o un tope de FPS de Reflex
obsoleto. La lista completa está en [graphics.md](graphics.md).

### Run no hace nada con el Steam de Flatpak

En una ventana que no abrió ninguna tienda, Run pide a Steam que arranque el juego
ejecutando `steam -applaunch <appid>`. El Steam de Flatpak no pone ningún `steam`
en el `PATH` del anfitrión, así que la línea de estado dice «Could not start
Steam» con el error. Pulsa Jugar en Steam en su lugar.

### Una partida guardada de otro dispositivo no aparece

Una partida que llega al prefijo sin pasar por Eidos, como una descarga de Steam
Cloud desde una Steam Deck, sigue oculta mientras las partidas del perfil están
montadas. Nada la borra. La pestaña Saves cuenta estas partidas bajo su
encabezado, e Import las copia al perfil sin reemplazar una partida del perfil
([usage.md](usage.md#las-partidas-guardadas-pertenecen-al-perfil)).

### Un mod que escribe un directorio de dos maneras perdía todo lo que había bajo la segunda

Corregido. ext4 distingue `meshes/` de `Meshes/`; la vista combinada no debe
hacerlo, y hay mods reales que traen ambas: XP32 Maximum Skeleton tiene sus
animaciones y su archivo de comportamiento FNIS bajo la versión con mayúscula, y
sus `character assets` bajo la otra.

El resolutor tomaba la coincidencia exacta de mayúsculas para cada componente de la
ruta y se comprometía con ella: entraba en `meshes/`, no encontraba allí el resto de
la ruta y abandonaba LA CAPA ENTERA. Todo archivo bajo la otra grafía era invisible
para el juego: sin error, sin registro, sin nada en ningún diagnóstico. En una
instancia real de 50 capas fueron 74 archivos.

Un componente que coincide es ahora un candidato, no una decisión; la coincidencia
exacta se sigue probando primero, y solo cuando el resto falla debajo el recorrido
busca hermanos equivalentes ignorando mayúsculas. Los listados tenían el mismo fallo
un directorio más arriba y ahora leen cada directorio equivalente por capa.

El índice de rutas nunca tuvo este fallo, porque recorre cada directorio que
encuentra. Así que durante un tiempo el índice devolvía archivos que el recorrido
de respaldo no, aunque el respaldo es la respuesta pensada para no equivocarse
nunca.

### El LODGen de DynDOLOD muere dejando un registro vacío

Corregido con `dotnet10`; consulta [tools.md](tools.md). El
`LODGen_SSE_<world>_log.txt` de cada mundo contiene un cartel de versión, una
línea `.NET Version:` y nada más, y un diálogo dice solo «failed to generate
object LOD for one or more worlds». La causa es el Mono de Wine respondiendo por
.NET Framework, y ninguna cantidad de .NET Framework instalado lo arregla: Proton
reemplaza `mscoree.dll` por un enlace simbólico a su propio árbol en cada
actualización del prefijo. El paquete de versión trae el runtime de .NET 10; las
demás instalaciones lo descargan una vez.

### Wine no podía saber que el montaje pliega mayúsculas y minúsculas

Corregido.

`get_dir_case_sensitivity` de Wine pregunta primero al directorio con
`FS_IOC_GETFLAGS` y después olfatea el marcador que CIOPFS deja en los
directorios que sirve. Si faltan ambos, Wine supone que SÍ distingue, y toda
búsqueda cuya grafía no coincida byte a byte recae en leer el directorio ENTERO
para encontrar una coincidencia sin distinción de mayúsculas. Los juegos de
Bethesda piden `data/ccbgssse001-fish.bsa` cuando el archivo es
`ccBGSSSE001-Fish.bsa`, así que saltaba en casi todos los recursos: 4471 sondeos
del marcador y 2236 relecturas completas de directorio en ocho segundos, y
195796 enumeraciones de `Data` en noventa. Skyrim SE nunca llegaba a su menú
principal: se quedaba en 240 MB residentes mientras el demonio quemaba el 92 %
de un núcleo.

Eidos plegaba mayúsculas en `resolve_read` desde el principio. Todo el coste
venía de no decirlo nunca. Ahora `lookup` responde `.ciopfs`; `readdir` sigue
sin listarlo. `ioctl` también responde `FS_IOC_GETFLAGS` en los directorios con
el indicador de casefold, que el Wine actual pregunta antes de buscar el
marcador.

Dos cosas lo volvieron mortal en lugar de meramente lento. El coste escala con el
tamaño del directorio, así que instalar el contenido Anniversary (`Data` de 37 a 177
archivos) lo desbordó. Y `opendir` construía con avidez el listado combinado, lo que
es puro desperdicio cuando Wine abre un directorio solo para hacer `stat` a ese
marcador dentro: la instantánea se toma ahora en el primer `readdir`.

Después: el menú principal, 2,1 GB residentes, demonio al 0 % de CPU.

`EIDOS_FUSE_TRACE=opendir` es lo que lo encontró, y se distribuye. Los contadores de
operaciones dicen cuántas; 195796 enumeraciones de un solo directorio son invisibles
dentro de un total.

### El juego reescribió `plugins.txt` vacío

Muy probablemente lo mismo que el plegado de mayúsculas de arriba: un `Data` que no
podía enumerar en un tiempo razonable, de donde concluía que allí no había nada y
guardaba eso. No demostrado, y merece revisarse. En cualquier caso, la guarda de
captura (una captura que vacía por completo el conjunto activo se rechaza sea cual
sea su tamaño) significa que ya no puede dañar el perfil.

### `FOPEN_KEEP_CACHE` está apagado

Corregido. Hacía que Skyrim SE se cerrara por desreferencia nula segundos después
del menú principal, de forma determinista, sin un solo mod instalado; las otras
tres cachés del lado del núcleo se descartaron una a una por bisección y solo esta
importaba. Esa bisección se hizo con `FUSE_PASSTHROUGH` activo, y el núcleo hace
fallar con EIO una apertura en passthrough que lleva `FOPEN_KEEP_CACHE`, así que
con la bandera activa fallaba cada apertura en passthrough. Puede que fueran esas
aperturas fallidas las que cerraban el juego, y la bandera por sí sola, con el
passthrough apagado, nunca se ha probado. Las respuestas en passthrough ya no la
llevan. Su pérdida se midió entonces como gratuita, pero con el passthrough activo
el demonio sirve *cero* lecturas (`EIDOS_FUSE_STATS` informó `read 0` en una carga
completa), así que con el passthrough apagado por defecto el coste real está sin
medir. Reactívalo con `EIDOS_FUSE_KEEP_CACHE=1` para probarlo por sí solo.

### El passthrough de FUSE impide que el juego cargue contenido de mods

El demonio evita ahora la causa más probable (abajo), pero eso aún no se ha
vuelto a medir en un juego, así que el passthrough sigue apagado por defecto;
`EIDOS_FUSE_PASSTHROUGH=1` lo activa. Antes del cambio, con el passthrough activo,
Skyrim SE no lograba abrir 152 de sus propios archivos, con
`STATUS_ACCESS_VIOLATION`, así que no se cargaba ningún contenido de mods, en
silencio. La medición y por qué los registros del propio demonio mostraban una
ejecución limpia están en «Por qué el passthrough está desactivado por defecto»
más arriba.

El núcleo asocia un único archivo de respaldo a cada inodo (`fs/fuse/iomode.c`).
Una apertura que trae un segundo identificador de respaldo para un inodo que ya
tiene uno falla con EIO, igual que una apertura normal (sin `FOPEN_PASSTHROUGH`)
de un inodo que tiene archivo de respaldo. Una apertura en passthrough que lleva
`FOPEN_KEEP_CACHE` también falla con EIO. El demonio registraba un identificador
nuevo en cada apertura, así que fallaba la segunda apertura de un archivo que el
juego ya tenía abierto. Wine informa EIO como `STATUS_ACCESS_VIOLATION`, el
error detrás de las 152 aperturas fallidas. Esta regla sale del código fuente
del núcleo y coincide con esos 152 fallos, lo que la convierte en la causa más
probable.

El demonio registra ahora un único archivo de respaldo por inodo, compartido por
todas sus aperturas en passthrough, y nunca pone `FOPEN_KEEP_CACHE` en una
apertura en passthrough. El passthrough solo cubre aperturas de solo lectura de
archivos de las capas del juego y de los mods que midan 1 MiB o más o terminen
en `.dll`, `.exe` o `.asi`. Una vez registrado ese archivo de respaldo, toda
apertura de solo lectura del archivo lo usa, y mientras el archivo tenga
aperturas normales, los lectores nuevos también reciben aperturas normales. Un
archivo del Overwrite nunca pasa por el passthrough, y un montaje que proyecta
las marcas de tiempo de los complementos no lo usa en absoluto. Una apertura
para escritura de un archivo que retienen lectores en passthrough recibe ETXTBSY
(en Wine: `STATUS_SHARING_VIOLATION`) antes de cualquier copy-up, así que una
apertura con `O_TRUNC` no puede dejar en el Overwrite una copia vacía que oculte
el archivo del mod.

### El botón Mod Manager Download no hace nada

Preferences, Nexus, Downloads muestra qué programa recibe los enlaces `nxm://`.
Aparece una tarjeta Health cuando ese programa no es Eidos, no existe o no hay
ninguno definido. El botón Register escribe `eidos-nxm.desktop` y lo fija como
gestor predeterminado de `x-scheme-handler/nxm`. Desde una terminal,
`eidos nxm --register` hace lo mismo.

Si el estado ya nombra Eidos y el botón sigue sin hacer nada, el navegador puede
estar usando una elección anterior. Borra la elección guardada para los enlaces
nxm en los ajustes del navegador para que vuelva a preguntar. Un navegador
Flatpak no puede iniciar programas del anfitrión y entrega el enlace mediante el
portal de escritorio, que lee el gestor registrado en el anfitrión.
