<!-- eidos-i18n: source=docs/guide/usage.md sha=15b7f6e81560a7045c8b62dd59dde35c8e45d32c -->

# Usar Eidos

Cómo usar Eidos: instancias y copias del juego, la línea de órdenes, las
ejecuciones de prueba, la ventana, las colecciones y actualizaciones de Nexus,
mover una instancia a otra máquina, herramientas y extensiones. Para los
problemas, consulta [troubleshooting.md](troubleshooting.md); para compilar desde
el código fuente y la estructura del repositorio, consulta
[../../../../internals/contributing.md](../../../../internals/contributing.md).

## Instancias: globales y portátiles

Las órdenes que aceptan una instancia aceptan un identificador de juego o la
carpeta de una instancia portátil. `skyrimse` nombra la instancia global, que
Eidos guarda en `~/.local/share/Colony/Eidos/instances/skyrimse`. Una instancia
portátil es una carpeta autónoma donde tú quieras (un segundo disco, una partición
de juegos), y puedes moverla, como las instancias portátiles de MO2:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # crear ahí una instancia portátil
eidos install /mnt/games/EidosSkyrim mod.7z  # toda orden acepta la carpeta
eidos play /mnt/games/EidosSkyrim -- %command%
```

El `eidos-instance.ini` de la carpeta nombra el juego, así que la carpeta no
necesita nada más. `EIDOS_INSTANCE=<folder>` en el entorno redirige un
identificador de juego a esa carpeta, por ejemplo en una opción de lanzamiento de
Steam. Las instancias portátiles que hayas creado o abierto se recuerdan, la más
reciente primero, en `~/.config/Colony/Eidos/instances.ini`: la pantalla de
bienvenida de la ventana las lista, el lanzamiento desde Steam aterriza en la
última que jugaste y el manejador `nxm://` descarga en ella.

Mover una carpeta portátil a mano conserva todo salvo las entradas de herramientas
que registraste con rutas absolutas a la ubicación anterior. `eidos pack` y
`eidos unpack` ([más abajo](#mover-una-instancia-a-otra-máquina)) las corrigen; un
simple `mv` no. La caché de runtimes de `~/.local/share/Colony/Eidos/runtimes/` la
comparten todas las instancias de la máquina, así que un host .NET de 78 MB no se
copia en cada una.

Hay un sitio que se rechaza: dentro de la carpeta de instalación de un juego, donde
los usuarios de MO2 suelen poner su instancia. Ese árbol pertenece a Steam, y una
actualización, un «verificar integridad» o una desinstalación pueden reescribirlo
o borrarlo junto con tu configuración. Eidos además monta sobre la raíz del juego,
así que una instancia ahí dentro quedaría dentro de su propio destino de montaje.
El asistente, `eidos init`, `eidos play` y `eidos shell` lo rechazan. Pon la
carpeta al lado del juego; una carpeta hermana en el mismo disco es igual de
cómoda.

### Dónde guarda Eidos sus archivos

Eidos guarda sus propios archivos bajo `Colony/Eidos`, la disposición que usa todo
programa de la familia Colony: `~/.config/Colony/Eidos/` para lo que elegiste
(preferencias, tu sesión de Nexus, tu lista de instancias, las definiciones de
juegos y extensiones que escribiste), `~/.local/share/Colony/Eidos/` para las
instancias globales (`instances/`), los runtimes descargados y los registros de
sesión (`logs/`), y `~/.cache/Colony/Eidos/` para las masterlists de LOOT, que se
vuelven a descargar siempre que faltan.

Cuando encuentra archivos con la disposición de un Eidos más antiguo
(`~/.config/eidos/`, `~/.local/share/eidos/` y `~/.local/state/`), el primer
lanzamiento los copia y lo dice en el registro. Una instancia global se traslada
con enlaces duros en lugar de una copia byte a byte, así que sus mods no ocupan
espacio extra, y sus entradas de herramientas se redirigen a la carpeta nueva. No
se cambia ni se borra nada de los directorios antiguos; elimínalos tú mismo cuando
la nueva disposición funcione. Si no se puede hacer una copia (un disco lleno o de
solo lectura, una carpeta de instancia antigua en otra unidad, un juego que aún se
ejecuta desde ella), Eidos sigue usando la ubicación antigua y lo vuelve a intentar
en el siguiente lanzamiento. Una opción de lanzamiento de Steam que todavía nombre
la carpeta antigua de la instancia abre la trasladada.

Los enlaces duros ahorran espacio solo al principio. Un mod o una descarga que
quites o reinstales después conserva su copia antigua en el disco a través de la
carpeta antigua de la instancia, y borrar `~/.local/share/eidos/<game>/` devuelve
ese espacio.

Una instancia portátil nunca se mueve: se queda donde la pusiste.

## La línea de órdenes

```sh
eidos games                                # juegos compatibles instalados aquí, como la lista de MO2
eidos init skyrimse [folder]               # crear una instancia; con una carpeta es portátil
eidos init skyrimse --game-path <folder>   # usar la copia instalada en <folder> cuando hay varias instaladas
eidos install skyrimse mod.7z [name]       # instalar un archivo (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # reinstalar sobre el mod (--merge instala sobre sus archivos)
eidos import skyrimse <mo2-profile>        # adoptar el orden de mods y el estado de plugins de un perfil MO2
eidos sort skyrimse [--dry-run]            # ordenar la carga con LOOT (--update-masterlist la actualiza antes)
eidos play skyrimse                        # mostrar qué se montaría
eidos play skyrimse -- <command>           # ejecutar <command> con los mods montados sobre el juego
eidos shell skyrimse                       # una shell en la vista combinada; lo escrito va al Overwrite
eidos check skyrimse [--json]              # comprobaciones Health sin la ventana
eidos sessions skyrimse                    # las ejecuciones registradas del juego y de herramientas, de la más reciente a la más antigua
eidos sessions skyrimse diff [<id>]        # qué cambió desde la última ejecución que terminó con código 0
eidos stock-game skyrimse                  # qué archivos del juego usa la instancia; congelar una copia o cambiar
eidos tool skyrimse                        # listar las herramientas (add, rm y run: ver Herramientas más abajo)
eidos tool skyrimse run --exe <path>       # ejecutar un exe una vez a través de la vista combinada
eidos tool skyrimse import-mo2 [--write]   # los ejecutables de un ModOrganizer.ini; --write los añade
eidos prereqs skyrimse [--install]         # qué necesitan las herramientas; --install trae lo que falta
eidos collection skyrimse <link>           # instalar una colección de Nexus (--dry-run, --no-optional)
eidos nexus status                         # comprobar la sesión de Nexus guardada
eidos nexus update skyrimse [--apply]      # buscar actualizaciones de mods; --apply las instala
eidos nxm <link>                           # descargar un enlace Mod Manager Download
eidos nxm --register                       # hacer que el botón Mod Manager Download del sitio abra Eidos
eidos export skyrimse -o mods.csv --active # el formato CSV de MO2; --active lista solo los mods activados
eidos pack skyrimse backup.eidos           # toda la instancia en un solo archivo
eidos unpack backup.eidos [folder]         # sin carpeta, una instancia global vuelve a su sitio central
eidos --version                            # imprimir la versión
```

`eidos` sin argumentos imprime la lista de órdenes. La mayoría de las órdenes
imprimen sus propias opciones cuando falta un argumento (`eidos init` y
`eidos import` imprimen en su lugar la lista completa), y `eidos tool --help` lista
las subórdenes de herramientas. `eidos pack` acepta `--dry-run`, `--level`,
`--no-downloads`, `--no-recovery` y `--force`, y `eidos unpack` acepta `--info` y
`--force`; ambos se describen [más abajo](#mover-una-instancia-a-otra-máquina).
`eidos stock-game` se describe en
[Archivos del juego](#archivos-del-juego-la-instalación-de-la-tienda-o-una-copia).

### Ejecutar un juego a través de la vista combinada

`eidos play` monta los mods de la instancia sobre el propio directorio `Data` del
juego dentro de un espacio de nombres privado, y luego ejecuta la orden a través de
esa vista. Los archivos del propio juego se apartan antes con un bind-stash, para
que el demonio siga leyéndolos intactos. Las escrituras (partidas guardadas,
configuraciones regeneradas) aterrizan en la capa `overwrite/` de la instancia; la
instalación del juego y todas las carpetas de mods quedan intactas byte a byte.

Eidos no necesita root. Monta en un espacio de nombres privado de usuario y de
montaje, sin ayudante setuid, sin servicio del sistema y sin nada que conceder. Una
capacidad opcional en el binario `eidos` permite el passthrough de FUSE, que sigue
desactivado salvo que se defina `EIDOS_FUSE_PASSTHROUGH=1`; consulta
[install.md](install.md#opcional-passthrough-de-fuse). Con la capacidad, Eidos usa
un espacio de nombres de montaje simple en lugar de uno de usuario, y los mods se
despliegan igual de un modo u otro.

### Una shell en la vista combinada

`eidos shell <instance>` abre tu `$SHELL` (o `/bin/sh`) en la carpeta del juego tal como la vería el juego: cada mod activado montado sobre `Data`, los mods Root sobre la raíz del juego, y todo lo que se escriba ahí va al Overwrite. `eidos shell <instance> -- <command>` ejecuta en su lugar una sola orden, por ejemplo `eidos shell skyrimse -- ls Data/meshes` o una herramienta nativa como NifSkope. La ejecución retiene el bloqueo de la instancia como una sesión de juego, así que la ventana no puede guardar cambios hasta que salgas, y queda registrada en `eidos sessions` con el nombre `shell`, nunca como un lanzamiento del juego. Ctrl+C en una orden detiene la ejecución como lo hace Stop: Eidos termina la orden, captura la sesión y desmonta. En la shell interactiva Ctrl+C es de la shell, como siempre; escribe `exit` para terminar la ejecución.

### Comprobaciones de salud desde la terminal

`eidos check <instance>` ejecuta las comprobaciones Health que no necesitan
ventana, de modo que pueden ejecutarse por SSH o en el Game Mode y acabar en un
informe de errores:

- el prefijo de Proton y su registro;
- los diagnósticos de plugins y los masters ausentes;
- el script extender y sus plugins;
- una actualización del juego desde el último lanzamiento limpio;
- la opción de lanzamiento de Steam;
- los ajustes gráficos (NVAPI, pantalla completa, ENB);
- el registro de cierre de la última ejecución;
- las líneas `sTestFile` de los INI del perfil;
- los plugins `.esp` y blueprint de Starfield;
- `vm.max_map_count` y el límite de archivos abiertos;
- la copia del juego que ejecuta la instancia, y un `gamePath` del
  `ModOrganizer.ini` de una lista que nombra otra carpeta de juego;
- las reglas de orden de colección que ya no se cumplen;
- los archivos del juego en NTFS, exFAT o FAT;
- las rutas de datos de BodySlide;
- las carpetas `Data` que quedan en `Overwrite/Root`;
- las bibliotecas de fuentes que nombra `Interface/fontconfig.txt` y que ningún mod
  aporta;
- el audio comprimido en los archivos BA2 de los mods de Fallout 4 y Fallout 4 VR.

Algunas tarjetas solo las calcula la ventana, en la pestaña Diagnostics: el orden
de parches, los scripts del extender que reemplaza un mod, los archivos que nada
carga, los requisitos de Nexus, los plugins que LOOT marca como sucios, la versión
de Proton que ejecuta el juego, la tarjeta Display y la salida de herramientas más
antigua que sus entradas.

La orden imprime las mismas líneas de sistema que Copy report, y luego una línea por
hallazgo, con tu carpeta personal mostrada como `~` y tu nombre de usuario como
`<user>`. El código de salida es 0 cuando no se encontró ningún problema (los
consejos solos siguen dando 0), 1 cuando se encontró al menos un problema y 2
cuando Eidos no pudo comprobar: argumentos incorrectos, una instancia que no puede
abrir o un juego que no detecta.

`eidos check <instance> --json` imprime en su lugar un único objeto JSON. Sus campos
de nivel superior son `eidos` (la versión), `game`, `instance`, `system`
(`proton`, `flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`,
`dev_fuse`, `fusermount3`, `max_map_count`, `open_files_hard`) y `findings`.
`max_map_count` y `open_files_hard` son números, o `null` cuando Eidos no pudo
leerlos. Cada hallazgo es un `code`, un `level` (`problem` o `advice`) y sus `args`;
la pestaña Diagnostics muestra la frase de cada código.

### Qué cambió desde la última ejecución buena

Cada ejecución con `eidos play`, `eidos shell` o `eidos tool run`, lanzada desde la ventana, una terminal o Steam, escribe un registro en `<instance>/logs/sessions/`: el perfil, cada mod activado con su versión y sus ids de archivo de Nexus, el orden de los plugins, una suma de comprobación de cada INI del perfil, la versión del ejecutable del juego, la versión de Eidos, el código de salida y las partidas guardadas que creó la ejecución. Se conservan los 50 más recientes, y `logs/` no entra en una instancia empaquetada. `eidos sessions <instance> diff` compara la última ejecución del juego que terminó con código 0 con el estado actual; `diff <id>` elige otro registro de la lista. La lista marca una [ejecución de prueba](#ejecuciones-de-prueba) como `test`, y una ejecución de prueba nunca se elige como la última ejecución buena. Una ejecución que detuviste registra el código de salida 143, así que tampoco se elige. Eidos aún no detecta cierres inesperados aquí, y un juego que falló bajo Proton puede terminar igualmente con código 0: si la ejecución elegida es una de esas, indica una anterior con `diff <id>`.

## La lista de mods

Como en MO2, cada perfil tiene su propio `modlist.txt` (qué mods están activados y
con qué prioridad), mientras que las carpetas de los mods viven en la carpeta
`mods/` compartida de la instancia. Una carpeta que dejas a mano en `mods/` todavía
no está listada: aparece desactivada con la prioridad más alta, al final de la
lista de la ventana como en MO2, de modo que gana sus conflictos en cuanto la
activas.

Como las carpetas son compartidas, quitar o renombrar un mod cambia todos los
perfiles: su línea se elimina, o se renombra, en el `modlist.txt` de cada perfil, y
cada perfil conserva su propio estado y orden para todo lo demás. Un perfil que se
queda sin mods no tiene ningún `modlist.txt`.

La ventana trabaja con su propia copia de la lista de mods y del orden de carga. Si
otro proceso de Eidos escribe cualquiera de las dos mientras está abierta
(`eidos install` o `eidos sort` en una terminal, una colección, una sesión que
lanzó Steam, una segunda ventana), la siguiente edición de la ventana se rechaza en
vez de sobrescribir ese trabajo. Vuelve a cargar desde el disco y dice *Not saved:
another Eidos process changed the mod list. It has been reloaded - redo the
change.* (o lo mismo para el orden de carga); solo se pierde ese clic, así que
vuelve a hacerlo. Rename, New mod, Add separator, Remove y convertir el Overwrite en
un mod hacen la comprobación antes de tocar `mods/`, de modo que una edición
rechazada no cambia nada en el disco. El editor de INI, del mismo modo, se niega a
guardar encima de un archivo que haya cambiado desde que lo abrió.

## Las partidas guardadas pertenecen al perfil

Cada perfil guarda sus propias partidas en `profiles/<name>/saves/`, y durante toda
una ejecución esa carpeta se monta sobre la carpeta de partidas guardadas del juego
en el prefijo de Proton. El primer lanzamiento de un perfil copia en él, una sola
vez, las partidas que ya estaban en el prefijo, de modo que una partida que después
borres del perfil sigue borrada. Tras cada ejecución, las partidas más recientes
del perfil se copian de vuelta al prefijo para Steam Cloud; una partida del prefijo
que Eidos no puso ahí se conserva primero en el perfil como `orphan-<time>-<name>`,
nunca se sobrescribe.

Una partida que llega al prefijo después de esa primera copia sin pasar por Eidos
(una descarga de Steam Cloud desde una Steam Deck, un lanzamiento sin Eidos) queda
oculta mientras las partidas del perfil están montadas. Nada la borra: el
lanzamiento la nombra en el registro de la ejecución (el `logs/run-*.log` de la
instancia para un lanzamiento desde la ventana, la consola de Steam para un
lanzamiento desde Steam), una vez cada vez que cambia ese conjunto de archivos, y la
pestaña Saves cuenta estas partidas en la línea bajo su encabezado. Para jugar una,
pulsa Import en la pestaña Saves. Import copia cada una con su co-save al perfil y
nunca reemplaza una partida del perfil: una partida cuyo nombre el perfil ya usa se
importa como `orphan-<time>-<name>`.

## Ejecuciones de prueba

El botón Test run, junto a Run, inicia el juego igual que Run, pero todo lo que el juego escribe va a `<instance>/test-run/` en lugar de a tu perfil, al Overwrite y al prefijo: partidas nuevas y modificadas, cambios de INI, `plugins.txt`, archivos escritos en Data o en la carpeta del juego, y los registros de `Documents/My Games/<game>`. Puedes probar con los mods del perfil, sin ningún mod, o solo con los mods seleccionados que están activados, más los mods cuyos plugins necesitan como masters.

Cuando el juego se cierra, Eidos muestra esos archivos en cinco grupos (partidas, cambios de INI, estado de los plugins, salida del Overwrite, registros) y te pide que conserves o descartes cada uno. Apply espera hasta que cada grupo que contiene archivos tenga una respuesta. El estado de los plugins no se ofrece cuando la ejecución usó otro conjunto de mods distinto del del perfil. Conservar las partidas nunca borra una partida del perfil, ni siquiera una que el juego borró durante la ejecución. Conservar la salida del Overwrite también quita del Overwrite los archivos que el juego borró, y la lista los cuenta antes. Descartar cualquier cosa pide una segunda pulsación. Decide later cierra la lista y deja los archivos en `test-run/`, y la lista se abre de nuevo la próxima vez que abras la instancia o pulses F5. Si Eidos o el ordenador se detienen durante una ejecución de prueba, los archivos también se quedan ahí y la lista se abre del mismo modo.

Cuando la ventana no la inició la tienda del juego, Test run pasa el juego a esa tienda (Steam, Heroic o Legendary), como hace Run, y la ejecución que inicia la tienda es la ejecución de prueba. Si la tienda no inicia el juego, la petición queda en espera: la siguiente ejecución del juego de ese perfil, desde Run o desde la tienda, es la ejecución de prueba, y Cancel the waiting test run la descarta. Una ejecución de prueba solo cubre las carpetas de arriba: las demás escrituras del juego, como el registro de Wine, van adonde van siempre. Las partidas de una ejecución de prueba llegan a Steam Cloud solo después de que las conserves y juegues una ejecución normal.

### Encontrar el mod detrás de un problema

Find the mod behind a problem, en la pestaña Diagnostics, busca el mod que causa un problema que puedes reproducir, como un cierre inesperado en un lugar concreto. Cada paso es una ejecución de prueba con la mitad de los sospechosos que quedan, los masters que necesitan y los mods que mantienes siempre activos: los mods activados seleccionados en la lista al empezar, y el mod que aporta el script extender. Tras cada ejecución respondes si el problema ocurrió. Not sure reparte los sospechosos de otra forma, y Run it again repite el paso. Responder, o pulsar Run it again, descarta los archivos que escribió el paso, tras una segunda pulsación. Para conservar algunos, pulsa antes See the files: abre la misma lista que tras una ejecución de prueba. Si el juego se cerró de golpe y un registrador de cierres escribió un registro, o si el juego terminó con un código de error, Eidos marca Yes como la respuesta probable. 200 sospechosos llevan unas 8 ejecuciones.

La búsqueda se guarda en `<instance>/bisect.json`, así que puedes cerrar Eidos entre pasos, y pertenece al perfil en el que empezó. Stop the search la descarta tras una segunda pulsación. El resultado nombra el mod, o el conjunto de mods que necesitan los masters unos de otros, y se guarda en `<instance>/logs/bisect-<time>.txt`. Los mods generados (una salida de Nemesis o DynDOLOD, un Bashed Patch) se construyen para la lista completa y pueden fallar con la mitad: mantenlos siempre activos o déjalos desactivados durante toda la búsqueda.

## Listas de Wabbajack

Eidos no tiene instalador de Wabbajack. Instala la lista con CLF3 o Jackify, fuera de Eidos, y elige la carpeta de instalación como instancia portátil en el asistente. Eidos lee la carpeta con la disposición de MO2: `mods/`, `profiles/<name>/modlist.txt` y las listas de plugins. Conserva el `ModOrganizer.ini` que escribió el instalador y nunca lo sobrescribe, así que los ejecutables propios de la lista siguen registrados.

Muchas listas añaden una carpeta `Stock Game` o `Game Root` que contiene una copia modificada del juego, y MO2 ejecuta el juego desde ahí. Cuando el asistente o `eidos init` adopta por primera vez la carpeta de una lista y esa carpeta contiene el ejecutable del juego, la instancia también ejecuta el juego desde ella. El juego sigue iniciándose a través de Steam o de la tienda dueña del juego, así que no hay que mover nada. Eidos encuentra la carpeta por el `gamePath` del `ModOrganizer.ini` de la lista, o por esos dos nombres. Una instancia adoptada con una versión anterior de Eidos conserva la instalación de la tienda hasta que la cambies (consulta la sección siguiente). Cuando ese `gamePath` nombra una carpeta de juego que la instancia no ejecuta, Diagnostics lo dice, porque las herramientas en modo Mod Organizer 2, como PGPatcher, leen el juego desde ahí. CLF3 reconstruye los archivos BSA y DDS con sus propias herramientas, por lo que el resultado puede diferir de una instalación de Wabbajack, y los autores de las listas no le dan soporte.

## Archivos del juego: la instalación de la tienda o una copia

Por defecto, el juego se ejecuta desde su instalación de la tienda, la carpeta donde Steam, Heroic o Legendary lo guarda. Una instancia puede ejecutarse en su lugar desde una copia de esa carpeta dentro de la instancia. Los mods se colocan sobre la copia, y la instalación de la tienda queda solo como el lugar donde Eidos monta el resultado. La tienda sigue iniciando el juego, así que Steam conserva su overlay, su caché de shaders y sus partidas guardadas en la nube.

La tarjeta Game files, arriba de la pestaña Diagnostics, muestra qué archivos usa la instancia, con la versión del juego de la copia y de la instalación de la tienda, y permite cambiar entre ellos. En una terminal, `eidos stock-game <instance>` imprime los mismos datos, y `use <folder>`, `store`, `freeze` y `delete --yes` hacen lo que los botones de la tarjeta. `freeze` imprime el tamaño y el tipo de copia, y `freeze --yes` copia.

**Freeze this version** copia la instalación de la tienda en `<instance>/Stock Game` y ejecuta el juego desde ahí, de modo que una actualización de la tienda ya no rompe los plugins del script extender: la actualización cambia la instalación de la tienda, no la copia. La primera pulsación muestra el tamaño y lo que hace el sistema de archivos con él, y la segunda copia. En un sistema de archivos que puede compartir bloques (reflink), como btrfs o XFS, la copia casi no ocupa espacio hasta que la tienda cambia un archivo. En ext4, o cuando el juego y la instancia están en sistemas de archivos distintos, es una copia completa tan grande como la carpeta del juego. Eidos se niega mientras Steam informa que el juego está ocupado o no está instalado del todo, mientras se ejecuta un juego o una herramienta de la instancia, cuando la instancia ya contiene la copia del juego de una lista, una copia congelada o una sin terminar, y cuando el sistema de archivos tiene menos espacio libre del que necesita una copia completa. Las copias de Heroic y Legendary no se comprueban en busca de una actualización en curso. Stop, un fallo, o una actualización de Steam del juego durante la copia dejan `Stock Game.eidos-tmp`, que la tarjeta ofrece borrar; esa carpeta nunca se usa.

Una copia oculta cada archivo que la tienda añade, cambia o elimina en la carpeta del juego después de hacer la copia: archivos nuevos de Creation Club, un ejecutable nuevo, DLL nuevas. El juego, la lista de plugins, LOOT y las comprobaciones Health leen todos la copia. No oculta lo que pasa fuera de la carpeta del juego: una actualización aún puede instalar runtimes en el prefijo de Proton en el siguiente lanzamiento, y Steam puede pasar el juego a otro Proton.

**Return to the store version** saca la copia del siguiente lanzamiento tras una segunda pulsación. La copia se queda en el disco, y **Delete the copy** la elimina, también tras una segunda pulsación. Delete se rechaza mientras la instancia aún ejecuta la copia. Eidos nunca borra la carpeta `Stock Game` o `Game Root` propia de una lista. Si la carpeta de la copia desaparece, o ya no contiene el ejecutable del juego, los lanzamientos se rechazan y Diagnostics nombra la carpeta, así que la versión de la tienda nunca se ejecuta por sorpresa.

Morrowind, y Oblivion con `bUseMyGamesDirectory=0`, guardan su lista de plugins en la carpeta del juego, y Eidos aún no los ejecuta desde una copia.

## Instalar una colección de Nexus

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # leerla y decir qué pasaría
eidos collection skyrimse rqhcxy --no-optional  # omitir los miembros que marca como opcionales
```

En la ventana, pega el enlace en File -> Open a Nexus collection..., o haz clic en
uno en el sitio. Eidos lista los miembros de la revisión, cada uno comparado con
esta instancia: instalado, descargado o ausente. *Install this collection* ejecuta
toda la receta en un hilo de trabajo, con la barra de progreso que usan los
trabajos de empaquetar y desempaquetar, y la barra se convierte en el informe al
terminar. La ventana y `eidos collection` ejecutan la misma instalación.

Sin enlace, File -> Open a Nexus collection... se abre en la pestaña Browse. Lista
las colecciones publicadas para el juego de la instancia abierta, de 20 en 20,
ordenadas por más descargadas, actualizadas recientemente o más nuevas, y el cuadro
de búsqueda reduce la lista a las colecciones cuyo nombre o texto coincide. Cada
fila da el autor, la última revisión, el número de mods, las descargas y una fecha.
View abre la última revisión de esa colección en la pestaña Link, igual que pegar
su enlace. Las colecciones marcadas como para adultos se omiten salvo que tu cuenta
de Nexus muestre contenido para adultos, las colecciones sin valoración se omiten
siempre, y la línea bajo la lista cuenta ambas. Explorar necesita una cuenta de
Nexus con sesión iniciada, como el resto del panel.

Una colección solo se abre con una instancia de su propio juego. Una colección de
Skyrim abierta mientras hay una instancia de Fallout 4 cargada se rechaza por su
nombre, en lugar de comparar sus miembros con la lista de mods equivocada.

### Lo que Eidos lee de una colección

Casi nada de una colección es visible a través de la API de Nexus. El orden de
instalación, las respuestas que dio el autor al instalador con script de cada mod,
qué mod gana un conflicto de archivos, qué plugins cargan dónde y los parches
binarios están todos en un `collection.json` dentro del archivo de la propia
colección. Descargar los mismos mods e instalarlos con sus opciones por defecto da
un juego distinto.

Eidos descarga el archivo, lee `collection.json` y lo aplica:

| La colección dice | Eidos hace |
| --- | --- |
| `phase` | instala en ese orden, los miembros opcionales al final |
| `choices` | repite las respuestas del autor a cada FOMOD, y lo dice cuando no puede |
| `modRules` | reordena la lista de mods para que el mod correcto gane cada archivo |
| `plugins` / `pluginRules` | fusiona las reglas de LOOT en tu userlist y ordena |
| `INI Tweaks/` | los instala como un mod propio |
| `tools` | te dice qué herramientas espera; no crea nada |

Los miembros se activan a medida que se instalan, al final de la lista de mods en
el orden de instalación, antes de que `modRules` los mueva. Un miembro que quedara
fuera de la lista de mods no cargaría, y el informe no lo mostraría.

El estado se escribe en `<instance>/collections/<slug>-<revision>.state.json`
después de cada miembro. Está junto a la carpeta de esa revisión, nunca dentro de
ella, así que una colección no puede traer un archivo de estado propio, y una
instalación interrumpida en el miembro 147 de 200 continúa desde el 147. Se
escribe con un nombre temporal y se renombra a su sitio, así que una interrupción
deja el registro antiguo o el nuevo y nunca la mitad de ninguno. Si alguna vez no
se puede leer, Eidos se detiene y lo dice en lugar de empezar de nuevo toda la
colección. Ejecuta la misma orden otra vez.

Instalar una revisión más reciente de una colección que ya tienes se hace cargo de
las carpetas de la revisión anterior: un miembro sin cambios se verifica en su
sitio, un miembro cuyo archivo actualizó el autor se reemplaza dentro de su carpeta
antigua, y un miembro que la nueva revisión eliminó se deja instalado y se nombra
en el informe para que lo desactives o quites. Una carpeta que activa otro perfil
nunca se asume, porque la carpeta de mods es compartida y ese perfil cambiaría sin
que lo supieras: la nueva revisión instala en su lugar su propia copia al lado, y el
informe te pide que desactives la antigua en este perfil, nunca que la quites.
Volver a una revisión más antigua recupera sus carpetas del mismo modo, o instala
copias nuevas de las que no puede recuperar.
Siempre que un miembro se reemplaza así y habías cambiado o añadido archivos en su
carpeta, la carpeta antigua se conserva primero al lado como `<name>_backup`, que
nunca se carga en el juego, y el informe la nombra.

Un resto de una revisión anterior que sigue activado impide que la instalación
cuente como la que construyó el autor; el aviso desaparece cuando lo desactivas.
Un miembro que omitiste con `--no-optional` no se asume y se nombra del mismo modo.
Si falla la propia asunción, la ejecución se detiene con *previous revision: its
folders could not be taken over* y el motivo; arregla eso y ejecuta la orden otra
vez.

### Cuentas gratuitas y miembros que descargas tú

Una cuenta gratuita de Nexus no puede descargar los mods miembros. Nexus no da
enlaces de descarga a las cuentas gratuitas a través de la API: cada archivo
necesita el botón «Mod Manager Download» del propio sitio. El archivo de la
colección se descarga con una cuenta gratuita, así que puedes leer toda la receta,
pero cada miembro se informa con la URL exacta de su página para que lo descargues
tú, y reintentar no lo cambia. Descárgalos con el botón del sitio y ejecuta la
orden otra vez: esos miembros se buscan en `downloads/` en cada ejecución, así que
la colección termina a medida que los aportas.

En la ventana, una cuenta gratuita recibe *Fetch the N missing required files, one
by one* en el panel de la colección en lugar de la descarga por lotes. Abre la
página del primer archivo requerido que falta en Nexus, espera a que ese archivo
llegue a `downloads/`, y luego abre la siguiente. Sigues pulsando el botón de
descarga de la página para cada archivo. El panel muestra cuánto lleva (*3 of 12*),
con *Skip* y *Stop*. Cuando llega en su lugar otro archivo del mismo mod, el panel
nombra ambos ids de archivo y sigue esperando el fijado. Los miembros opcionales no
forman parte de esto y conservan su propio botón *Open*.

Eidos también aplica parches binarios (cada uno comprobado con su CRC registrado),
sustituciones de archivos (los archivos nombrados se ocultan con `.mohidden`, como
hace MO2) y miembros cuyos archivos viajan dentro del archivo de la colección
(`bundle`). Los miembros que la colección espera que descargues a mano (`browse`,
`manual`) llevan al informe las instrucciones del propio autor. Un miembro cuyo
nombre ya es un mod tuyo se instala a su lado con un nombre libre, nunca encima, y
el informe dice cuál.

### El informe

Un miembro cuyas respuestas del instalador no se pudieron repetir todas recibe su
propia línea, *installed, but not the way the collection asks*, aparte de los
miembros que se instalaron como los construyó el autor, porque sus archivos en el
disco difieren de los del autor. El informe también lista las reglas de orden que
no nombraban nada de la colección, los mods atrapados en un bucle de reglas
contradictorias, los grupos de LOOT que no existen y cualquier parte del manifiesto
que esta versión de Eidos no entiende.

### Mantener el orden de la colección

Eidos conserva las reglas de cada colección instalada después de la instalación.
Cuando más tarde mueves dos de sus mods contra una regla de antes o después, la
pestaña Diagnostics muestra una tarjeta para esa colección, que nombra el primer
par de ese tipo y cuenta las reglas que ya no se cumplen. Solo cuentan los mods
activados en el perfil actual, y las reglas atrapadas en un bucle se omiten, porque
ningún orden puede cumplirlas. **Restore collection order** devuelve los mods de la
colección a un orden que cumple todas las reglas, conserva tu propio orden donde
las reglas lo permiten, y deja donde están los mods que no son de la colección.
Ctrl+Z lo deshace. `eidos check` lista el mismo hallazgo como
`collection_order_broken`. La comprobación lee la receta de la revisión en
`collections/`, así que no se comprueba una colección cuya receta en caché se
eliminó.

## Mover una instancia a otra máquina

`eidos pack` escribe una instancia entera (cada mod, el orden de carga, todos los
perfiles, el Overwrite con tus partidas dentro y los archivos desde los que se
instaló todo) en un solo archivo. `eidos unpack` la devuelve a su sitio:

```sh
eidos pack skyrimse ~/backup.eidos                  # o da una carpeta: se nombra con el juego y la fecha
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # en la otra máquina
```

`eidos pack --dry-run` lista exactamente qué entraría, qué no y por qué, y cuánto
espacio necesita, sin escribir nada. `eidos unpack --info` hace lo mismo con un
archivo que ya tienes.

Si 7-Zip no pudo leer algo, o el recorrido tuvo que omitir algo que esperarías en
una copia de seguridad (un enlace simbólico, una carpeta que no pudo leer, un
nombre que no se le puede dar a 7-Zip con seguridad), el archivo se escribe igualmente
con el nombre que elegiste, pero `eidos pack` termina con código 1 y dice qué falta,
de modo que un script que ejecute `eidos pack ... && ...` se detiene ahí.

Un archivo `.eidos` es un archivo de 7-Zip con su propia extensión. 7-Zip ya es
necesario para instalar un mod, así que esto no añade ninguna dependencia, y la
copia de seguridad se abre sin Eidos en cualquier compresor que lea 7-Zip. Se
comprime con LZMA2 a `-mx1`, hasta cerca del 43 % del original en una lista con
muchas texturas; `-mx9` tarda mucho más y ahorra unos puntos más. Las texturas y las
mallas ya son formatos comprimidos, así que un diccionario mayor encuentra muy poco
que comprimir. `--level` (0, 1, 3, 5, 7 o 9) elige otro nivel, y `--no-downloads`
deja fuera `downloads/`.

En el nivel 1 por defecto, los archivos van en bloques sólidos de 16 MB como máximo.
Extraer un archivo de una copia de 70 GB decodifica como mucho el bloque en el que
está, nunca todo lo que hay delante, y empaquetar contenido del juego es de 2 a 5
veces más rápido que con cada archivo comprimido por separado (desempaquetar, de 2
a 4 veces). El coste está en los daños: un byte corrupto puede llevarse consigo
todos los archivos de su bloque, hasta 16 MB de archivos vecinos. Los datos de
recuperación descritos más abajo lo cubren. Cualquier otro nivel sigue sin ser
sólido: cada archivo es su propio flujo, y un byte corrupto cuesta ese único
archivo. El nivel 0 almacena los archivos. Desde el nivel 3, LZMA2 ya trabaja en
trozos de 16 MB o más, y los bloques sólidos resultaron más lentos ahí: en el nivel
5, empaquetar mallas tardó 1,4 veces más y desempaquetar texturas 6,8 veces más.

Empaquetar toma el bloqueo de la instancia, así que no puede ejecutarse mientras el
juego u otro Eidos escribe en esa instancia, y nada cambia la instancia mientras se
lee. La copia se escribe con un nombre `.part`, se vuelca al disco y se vuelve a leer
entera con `7z t` antes de recibir el nombre que elegiste. Esa comprobación
decodifica cada bloque y compara cada suma de verificación, el mismo trabajo que
hace un desempaquetado en la otra máquina. Muestra lo que el disco devolvió justo
después de escribir, no que la unidad vaya a conservarlo; para eso están los datos
de recuperación de más abajo. Si falla, el `.part` se borra, no se escribe nada con
tu nombre, y `eidos pack` cita la línea donde 7-Zip encontró el daño. Un empaquetado
interrumpido o fallido deja un archivo `.part` o nada, nunca un archivo `.eidos`
que parezca completo y no lo sea.

Cuando par2cmdline está instalado (`sudo pacman -S par2cmdline` en Arch,
`sudo apt install par2` en Debian y Ubuntu, `sudo dnf install par2cmdline` en
Fedora), `eidos pack` también escribe datos de recuperación PAR2 junto al archivo:
`backup.eidos.par2` y unos pocos archivos `backup.eidos.volNN+MM.par2`, el 5 % del
archivo más un pequeño índice. Guárdalos en la misma carpeta que el archivo
`.eidos`, con los nombres con que se escribieron, y cópialos con él: una copia
movida sin ellos no tiene nada desde lo que repararse, y los datos de recuperación
registran el nombre del archivo, así que una copia renombrada ya no coincide.
`eidos unpack` los encuentra por su nombre y comprueba el archivo contra ellos antes
de extraer nada, incluida una copia a la que le cortaron el final de modo que 7-Zip
no puede abrirla. Si unos pocos bytes se estropearon, par2 repara primero el archivo
y conserva la copia dañada como `backup.eidos.1`, lo que necesita espacio libre del
tamaño del archivo en su carpeta; un daño mayor del que cubre el 5 % detiene el
desempaquetado antes de que se escriba un solo archivo. Los archivos de
recuperación que no coinciden con el archivo, como los de una copia más antigua con
el mismo nombre, no se usan cuando 7-Zip lee el archivo y lo encuentra intacto, y el
desempaquetado lo dice. En una máquina sin par2 el desempaquetado se ejecuta igualmente,
con un aviso de que no se usaron los datos de recuperación. `--no-recovery` empaqueta
sin ellos, y `eidos pack --dry-run` dice si se escribirán. `--force` reemplaza un
archivo existente del mismo nombre.

### Qué no va dentro, y por qué

| Se deja fuera | Por qué |
| --- | --- |
| El prefijo de Proton | Tiene la forma de la máquina (asignaciones de unidades, una vista `Z:` de *este* sistema de archivos) y Eidos lo reconstruye en un minuto con `eidos prereqs <instance> --install`. Llevarlo duplicaría aproximadamente el archivo para enviar algo que la otra máquina tiene que regenerar de todos modos. |
| El juego | Una instancia modifica un juego que no contiene. Instálalo desde Steam en la otra máquina. |
| Herramientas fuera de la instancia | xEdit, DynDOLOD y otras herramientas son binarios de terceros con sus propios instaladores. Se nombran en el archivo, así que al desempaquetar te dice exactamente cuáles faltan en la máquina nueva. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Registros de la máquina que hizo la copia. `prereqs.done` en particular afirmaría que las bibliotecas de runtime ya están instaladas en un prefijo que no existe. |
| `loot/`, salvo `userlist.yaml` | Una caché de masterlist que Eidos vuelve a descargar cuando hace falta. Tus propias reglas de LOOT en `userlist.yaml` no se pueden volver a descargar, así que ese único archivo entra. |
| `.base/`, `.base-root/` | Puntos de montaje vacíos donde se apartan los archivos del propio juego durante una sesión. |
| `test-run/`, `bisect.json` | Una [ejecución de prueba](#ejecuciones-de-prueba) a la espera de su revisión, y una búsqueda del mod detrás de un problema entre dos pasos. Ambas pertenecen a ejecuciones de esta máquina. |
| Archivos a medio escribir | Una descarga en pausa (`*.unfinished`), una escritura atómica en curso (`*.eidos-tmp*`). |
| Enlaces simbólicos | 7-Zip seguiría uno y copiaría lo que señala, lo que en un enlace absoluto significa meter un árbol ajeno en tu copia. Se informa de ellos en su lugar, y la copia cuenta como incompleta: un `mods/` o `downloads/` enlazado a otra unidad no se empaqueta. Para hacer la copia sin un `downloads/` enlazado a propósito, pasa `--no-downloads` (o desmarca *Include downloads/* en la ventana); una exclusión deliberada no cuenta como falta. |

Una copia del juego en la instancia (`Stock Game`) se empaqueta como cualquier otra
carpeta, así que la copia de seguridad de una instancia congelada es más grande en
el tamaño del juego. Una copia sin terminar (`Stock Game.eidos-tmp`) se deja fuera.

Cada una de estas exclusiones se lista con su motivo en `eidos-backup.ini` en la
raíz del archivo, así que leer ese archivo muestra qué se dejó fuera. El mismo
archivo registra dónde vivía la instancia, qué perfil estaba activo, cuánto ocupa
al desempaquetarse y qué herramientas necesitará la máquina nueva.

Los registros de descarga (`downloads/*.meta`) entran con su línea `url=` vaciada.
Esa URL es un enlace firmado de Nexus que deja de funcionar en cuestión de horas y
lleva el `user_id` de la cuenta que descargó el archivo, y una copia de seguridad
puede entregarse a otra persona. El id del mod, el id del archivo y la versión se
quedan, así que la descarga se puede volver a encontrar.

### En la ventana

Ambas están en el menú File: *Pack this instance...* y *Unpack a backup...*.
Desempaquetar también está en la pantalla de bienvenida, bajo la lista de
instancias, de modo que una máquina que todavía no tiene ninguna instancia puede
desempaquetar una.

Antes de escribir nada, el diálogo Pack muestra cuántos archivos, qué tamaño, cuánto
de eso es `downloads/`, más o menos a cuánto llegará el archivo, y qué herramientas
se nombran pero no se incluyen. Su casilla *Write recovery data* está marcada cuando
par2 está instalado, y atenuada con el paquete que hay que instalar cuando no lo
está. El diálogo Unpack lee el manifiesto de la copia en cuanto eliges el archivo,
así que puede decirte qué juego contiene, cuándo se hizo, dónde vivía y cuáles de
sus herramientas faltan aquí, antes de que elijas una carpeta.

Ambas se ejecutan en un hilo de trabajo con una barra de progreso, y la barra se
convierte en el informe al terminar.

### Qué repara el desempaquetado

Una copia simple de la carpeta dejaría cada botón de herramienta apuntando a una
ruta de la máquina antigua. `eidos unpack` reescribe los valores que nombran la raíz
de la instancia antigua, y solo esos: `exe`, `workdir` y las claves `arg` numeradas
de `tools.ini`, e `installationFile` en el `meta.ini` de cada mod. Una herramienta
que nunca estuvo dentro de la instancia (`/opt/xedit/SSEEdit.exe`) se deja
exactamente como estaba y se lista como algo que hay que reinstalar; Eidos no adivina
dónde está esa herramienta en la máquina nueva. También corrige si la instancia se
considera central o portátil, para que las órdenes posteriores la busquen donde está
ahora.

Desempaquetar rechaza una carpeta que no está vacía (pasa `--force` si quieres
sobrescribir), un disco demasiado pequeño para la cifra del propio manifiesto, un
archivo hecho por un Eidos más nuevo que el tuyo, y cualquier archivo que contenga
una entrada que se escribiría fuera de la carpeta que elegiste. Sin carpeta, la
copia de una instancia global vuelve al sitio central de su juego; una portátil
necesita una carpeta.

Después:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # reconstruir el prefijo de Proton
eidos play /mnt/games/EidosSkyrim
```

## La ventana

Inicia `eidos-gui`, o el archivo `eidos-linux` que instala Colony.

El primer lanzamiento abre un asistente como el de MO2: bienvenida, tipo de
instancia (portátil o global), juego, nombre y ubicación, resumen, y luego la
ventana principal. La pantalla de bienvenida también lista cada instancia conocida,
global y portátil, la última usada primero, para abrirla con un clic, de modo que
sirve además de selector de instancias. Apuntar el asistente a una carpeta que ya
contiene una instancia abre esa instancia tal como está en lugar de crear una nueva
encima, y una carpeta que pertenece a otro juego se rechaza.

La ventana principal tiene dos paneles: la lista de mods, y las pestañas Data,
Plugins, Conflicts, Overwrite, Archives, Saves, Downloads y Diagnostics, con un
botón Run, un botón Test run ([Ejecuciones de prueba](#ejecuciones-de-prueba)) y un
selector del objetivo de ejecución. Puedes filtrar la lista de mods, seleccionar,
reordenar, agrupar con separadores, acotar por categoría y hacer clic derecho para
las acciones. Además de enviar arriba y abajo, el menú de un mod tiene los
movimientos dirigidos de MO2: enviar por encima del primer mod en conflicto, por
debajo del último, a una prioridad dada, o al grupo de un separador.

El selector de perfiles cambia de perfil. Haz clic derecho en un perfil para
Rename, Copy to new..., Desktop shortcut, Import from MO2... y borrar, que
requiere dos clics y rechaza el perfil activo o el último. Import from MO2... toma
el orden de mods y el orden de carga de un perfil de MO2. Desktop shortcut escribe
una entrada de escritorio que ejecuta
`eidos tool <instance> run "<game>" --profile <name>`: convierte ese perfil en el
activo y pide a la tienda que inicie el juego (detalles en
[tools.md](tools.md#llegar-a-una-herramienta-ocultar-fijar-y-un-acceso-directo-de-escritorio)).

El menú File también ejecuta winecfg, regedit, el explorador de archivos de Wine y
el desinstalador en el prefijo del juego, y Prefix drive_c abre la unidad `C:` del
prefijo; consulta
[tools.md](tools.md#los-programas-propios-de-wine-en-el-prefijo-del-juego).
*Instances...* lista las instancias que Eidos conoce; Forget quita una de esa lista
y no toca nada en el disco. *Export the mod list...* escribe la lista en el formato
CSV de MO2, como hace `eidos export`.

### Preferencias

Haz clic en Eidos, arriba a la izquierda, para abrir Preferences en lugar de los dos
paneles, y haz clic otra vez (o en Close) para volver. General, Appearance y
Accessibility van primero, como en todo programa de Colony, y luego Mod list, Nexus
y About. Cada cambio se aplica y se guarda al instante; no hay botón Save.

- General: el juego que abre Eidos, el tamaño de la ventana, el idioma de la
  interfaz, la carpeta de herramientas y cómo se ejecuta un juego. Lock the window
  while a game runs está activado por defecto. Start the game at once when a store
  opens Eidos, y la notificación de escritorio que avisa cuando termina un trabajo
  largo mientras la ventana está en segundo plano, están desactivados por defecto.
  La ventana habla inglés y francés; System sigue `LC_ALL`, `LC_MESSAGES` o `LANG`,
  y el cambio se aplica al instante. La línea de órdenes `eidos` siempre responde en
  inglés.
- Appearance: el tema (26 familias, 59 paletas, el pergamino de Eidos por defecto),
  el color de acento, el tamaño del texto, los efectos y una vista previa en vivo.
- Accessibility: alto contraste, una tipografía apta para la dislexia, menos
  movimiento y un tamaño de texto de lectura que se multiplica con el de Appearance.
- Mod list: la velocidad del desplazamiento automático mientras se arrastra un mod
  hacia el borde superior o inferior de la lista.
- Nexus: inicio de sesión, la carpeta de descargas, el manejador `nxm://` y su botón
  Register, los servidores preferidos, Install downloads when they finish, y
  Offline mode.
- About: dónde viven los archivos, los atajos de teclado, la versión y la
  comprobación de versiones.

El aspecto y el idioma de la interfaz viven en
`~/.config/Colony/Eidos/preferences.json`. Los demás ajustes, que también lee la
línea de órdenes (el juego por defecto, la carpeta de herramientas, los servidores
de Nexus, Offline mode y el resto), viven en `settings.ini` a su lado, y la sesión
de Nexus en `nexus.ini`. El tema que un Eidos más antiguo guardaba en
`settings.ini` se traslada la primera vez.

About -> Version contiene la comprobación de versiones, activada por defecto. Al
arrancar, como mucho una vez al día, Eidos pregunta a GitHub por la última versión
de `Project-Colony/Eidos`. Cuando es más nueva que la copia en ejecución, la barra
de menús muestra `Eidos <version> is available: release notes` después de Help, y el
enlace abre la página de esa versión con su registro de cambios. Eidos nunca
descarga ni se sustituye a sí mismo. La comprobación se omite en modo sin conexión.
Está desactivada para una copia instalada bajo `/usr` por un gestor de paquetes, que
la actualiza; `/usr/local`, donde `install.sh --system` pone Eidos, cuenta como una
instalación propia.

### Columnas, ordenación y agrupación

La lista muestra cuatro columnas por defecto (Category, Content, Version y Flags,
las que muestra MO2) y ofrece ocho: Category, Content, Version, Author, Installed,
Nexus id, Game, Flags. Márcalas en el menú View. Con las ocho activadas, a la
columna del nombre le queda poco espacio.

Haz clic en un encabezado para ordenar por él, otra vez para invertir, y una tercera
vez para volver al orden de carga. La lista solo se puede arrastrar en el orden de
carga: una posición de soltado se dirige a la lista real, y bajo una ordenación el
lugar de una fila en pantalla no es su lugar en esa lista. Mientras hay una
ordenación activa, las tiras de inserción no se dibujan y un arrastre se rechaza,
como en MO2. El menú View lo dice y ofrece el camino de vuelta.

El menú View también puede agrupar toda la lista, por categoría o por origen (de
Nexus, o instalado a mano). Los encabezados de grupo no son separadores: no se
pueden renombrar, colorear ni mover. Se pliegan, y un encabezado plegado conserva su
recuento. Los separadores se ocultan mientras hay una ordenación o una agrupación
activa, porque un separador encabeza las filas que le siguen en el orden de carga, y
ambas cambian ese orden en pantalla.

### Ratón y teclado

Doble clic en un mod para Information, Ctrl+doble clic para su carpeta,
Mayús+doble clic para su página de Nexus. Ctrl+F pone el cursor en el cuadro de
filtro. Escribir una letra salta al siguiente mod que empieza por ella, y pulsarla
otra vez recorre el resto en lugar de quedarse en el primero. Ninguna cae en una
fila que el filtro, un separador plegado o un grupo plegado oculte, así que el
siguiente Espacio nunca activa un mod que no ves.

«Collapse others» en el menú de un separador pliega todos los grupos salvo ese.
Durante un arrastre, mantener el puntero quieto sobre un grupo plegado lo abre, para
poder soltar el mod dentro; pasar por encima no lo abre.

Ctrl+Z deshace el último cambio en la lista de mods o en el orden de carga: una
casilla, un movimiento, una desactivación por lotes, una ordenación de LOOT.
Ctrl+Mayús+Z lo rehace. La ventana conserva 50 pasos y los olvida con F5, al final
de una ejecución del juego o de una herramienta, al cambiar de perfil y cuando otro
proceso de Eidos cambia la lista. Instalar, renombrar o quitar un mod cambia las
carpetas en el disco, así que se rechaza un deshacer que pase por uno de esos.

### Filtrar la lista

El botón Filters abre una fila por criterio: activado, en conflicto, actualización
disponible, sin categoría, respaldado, seguido, tiene un id de Nexus, tiene archivos
ocultos, sin datos de juego válidos, para otro juego, tiene notas, y una fila por
tipo de contenido (plugins, archivos BSA o BA2, texturas, mallas, scripts, plugins
del script extender, archivos de interfaz, sonido, FaceGen). Un clic cicla una fila
entre off, only y except. La fila de arriba alterna entre cumplir todos los
criterios activados y cumplir cualquiera de ellos. El cuadro de nombre y la lista de
categorías siempre acotan más el resultado. La fila de archivos ocultos solo mira
los mods activados.

### Lo que la lista te dice de un mod

Dos indicadores orientativos, ambos un glifo con la explicación al pasar el puntero.
Sin datos de juego válidos significa que nada en lo alto del mod se parece a algo
que este juego cargue; puede que haya que subir sus carpetas un nivel, o puede que no
sea un mod para este juego. Otro juego significa que el `meta.ini` del propio mod
nombra uno distinto. Ninguno bloquea nada: el mod se despliega igualmente. «Mark as
valid» en el menú de la fila silencia cualquiera de los dos mediante la clave
`validated=` del propio MO2, de modo que un mod marcado como válido en un gestor no
se señala en el otro.

La comprobación de estructura señala un mod solo cuando está segura: un árbol
`Root/`, una carpeta ilegible y una carpeta vacía cuentan todos como datos de juego
válidos.

### Desactivar un mod que usan tus partidas guardadas u otros mods

Antes de desactivar o quitar un mod, Eidos lee las listas de plugins de las tres
partidas más recientes del perfil. Cuando una de ellas usa un plugin de ese mod, una
barra sobre la lista nombra los plugins y espera Disable anyway o Cancel (Escape
también cancela). Remove añade los mismos nombres a su mensaje de «haz clic otra
vez». Solo se leen partidas de Skyrim y Fallout 4; los demás juegos se desactivan sin
preguntar.

La pestaña General de Information de un mod lista los mods que necesita y los mods
que lo necesitan, leídos de los masters de los plugins activados. Desactivar un mod
que otros necesitan muestra la misma barra con Disable them too, que también
desactiva cada mod que lo necesita y los mods que necesitan a esos, o Keep them
enabled. La tarjeta de masters ausentes de la pestaña Diagnostics tiene Enable what
provides them: activa el mod que contiene cada master ausente y marca un plugin
master que está presente pero sin marcar.

### Hacer copia de un mod antes de tocarlo

«Back up this mod» copia su carpeta aparte como `<name>_backup` (luego `_backup2`,
y así sucesivamente; una copia nunca reemplaza la anterior). La copia no es un mod:
su casilla no hace nada y no añade nada a la vista combinada, porque marcarla
desplegaría dos copias de un mismo mod una sobre otra. «Restore this backup over the
mod» la devuelve a su sitio, en dos clics; el contenido actual se aparta primero y
solo se descarta cuando la copia ha tenido éxito.

**Backups** en la barra de herramientas conserva puntos de restauración de la lista de
mods del perfil, su orden de carga (`plugins.txt` y `loadorder.txt` juntos) y sus INI
del juego, 10 de cada, en `<profile>/backups/`. Back up now añade uno, y Restore
guarda antes el estado actual, así que una elección equivocada se puede deshacer.
Eidos también conserva un punto de restauración de INI antes de cada ejecución de
BethINI (consulta [tools.md](tools.md#bethini-pie)).

### Las pestañas

**Data** es un árbol de la vista combinada, que se expande de un nivel en un nivel:
abrir una carpeta lee esa carpeta en cada capa que la tiene, nunca todos los mods
activados a la vez. Se construye a partir de la misma pila de capas que sirve el
montaje, así que se respetan los whiteouts y los archivos ocultos y la pestaña
muestra lo que verá el juego. Fíltrala por nombre, redúcela a los archivos en
disputa, ordena qué hay dónde con las columnas Size y Modified, y haz Reveal de
cualquier fila en un gestor de archivos. Al pasar el puntero sobre un archivo se
listan los demás mods que lo traen. El `meta.ini`, el `readme.txt`, el `fomod/` y el
`Root/` de primer nivel de un mod se dejan fuera de Data, en esta pestaña y en el
montaje: los tres primeros son para el gestor de mods, y `Root/` va en cambio a la
carpeta del juego. Un archivo que el juego o una herramienta escribe bajo uno de
esos nombres aterriza en el Overwrite y sigue siendo visible.

Una fila `.exe` en Data tiene Add tool, que abre el editor Executables con ese archivo rellenado (no se guarda nada hasta que pulses Save), y Run, que lo ejecuta una vez a través de la vista combinada como una herramienta registrada ([Herramientas](#herramientas)). Save tree escribe todo el árbol Data combinado en un archivo de texto, una línea por archivo con el mod que lo aporta, con tu carpeta personal y tu nombre de usuario eliminados, para un informe de errores.

**Plugins** es el orden de carga de ESP/ESM/ESL: activa, reordena a mano u ordena con
LOOT y lee el informe posterior a la ordenación, cuyos enlaces de consejos se abren
en tu navegador. Su encabezado cuenta los plugins activados en cada espacio de
índices frente al límite del juego, por ejemplo `full 231/254, light 512/4096`, y se
pone naranja cuando uno está lleno. El cuadro de filtro acota la lista por nombre.
Espacio, Ctrl+A, los rangos con Mayús y Activate all shown y Deactivate all shown
del menú de la fila actúan entonces solo sobre las filas mostradas, y un arrastre se
rechaza hasta que se borra el filtro. Al seleccionar un plugin se tiñen sus masters
en la lista de plugins y el mod que lo trae en la lista de mods. Los plugins que
LOOT marca como sucios reciben una tarjeta en Diagnostics que puede limpiarlos con
xEdit ([tools.md](tools.md#limpiar-un-plugin-que-loot-marca-como-sucio)).

Tras una ordenación de LOOT, un plugin completo cuyos registros nuevos caben todos
en el rango light lo dice al pasar el puntero, y el menú de su fila tiene Flag as
light. Eidos escribe una copia con la marca light en un mod nuevo llamado
`<plugin> (light)` al final de la lista de mods (la prioridad más alta) y deja el
original intacto, así que desactivar ese mod lo deshace. Se niega cuando una de las
tres partidas más recientes del perfil carga el plugin como plugin completo. Solo se
leen partidas de Skyrim y Fallout 4, así que en Starfield no se comprueba ninguna
partida. LOOT comprueba solo los rangos de registros: mira en xEdit un plugin que
añada celdas o worldspaces antes de marcarlo.

Al pasar el puntero sobre un plugin se muestra de dónde viene, su autor, la versión
de su cabecera y de formulario, los primeros 300 caracteres de su descripción, sus
masters y, tras una ordenación de LOOT, los mensajes de LOOT. En Skyrim, Skyrim SE,
Skyrim VR y Enderal SE el juego también lee un INI con el nombre de un plugin
cargado; la columna INI marca esos plugins, y un clic abre ese archivo en solo
lectura.

**Conflicts** explica los ganadores y perdedores por archivo. **Overwrite** convierte
lo que escribió el juego en un mod de verdad en un solo paso.

**Archives** lista cada BSA o BA2 que traen los mods activados, el mod del que viene
y por qué se carga: un plugin activo del mismo nombre, o que lo registra el INI. Uno
que nada carga se marca como peso muerto. Cuando otro mod trae un archivo del mismo
nombre que gana el conflicto, la fila nombra a ese mod. Extract descomprime un
archivo en un mod nuevo con su nombre sin la extensión (`Foo.ba2` da
`Foo (extracted)`) y deja el archivo en su mod; los archivos sueltos ganan a los
archivos de cualquier BSA o BA2, sea cual sea el orden de los mods.

**Saves** lee la cabecera de cada partida (personaje, nivel, lugar, tiempo jugado),
compara la lista de plugins guardada en ella con la actual, y tiene un botón que
activa los mods que necesita. Las partidas que llegaron al prefijo sin Eidos se
describen en
[Las partidas guardadas pertenecen al perfil](#las-partidas-guardadas-pertenecen-al-perfil).

**Downloads** y **Diagnostics** tienen sus propias secciones más abajo.

### Información del mod

«Information...» abre un diálogo por mod con las pestañas General, Conflicts,
Filetree, INI Tweaks, Notes, Nexus files y Nexus page. Desde el filetree (y desde el
árbol Data) cualquier archivo se puede ocultar: se renombra a `<name>.mohidden`, lo
que lo saca de la vista combinada sin borrarlo, de modo que se pueden suprimir unos
pocos archivos sueltos de un mod sin cambiar prioridades. El filetree también hace las
operaciones corrientes con archivos: nueva carpeta, renombrar, borrar, abrir. Cada
una rechaza una ruta que no sea una ruta simple dentro de ese mod: sin `..`, sin ruta
absoluta y sin enlace simbólico por el camino, porque seguir uno podría borrar un
archivo fuera de la carpeta del mod. Rename cambia solo la última parte de la ruta,
así que nunca mueve un archivo, y rechaza un nombre que ya está ocupado. Delete
requiere dos clics y no se puede deshacer.

**View** en cualquier fila del filetree o del árbol Data muestra una vista previa
del archivo: imágenes, texto, texturas DDS (elige el mip, la capa, la cara y el
canal) y modelos NIF estáticos con sus texturas difusas. La vista previa de NIF
necesita `eidos-nif-preview` junto a Eidos o en el `PATH`; el paquete de versión, el
paquete de Arch y las compilaciones desde el código fuente lo traen, `eidos-linux`
no. El texto se lee hasta 64 KB y dice dónde se detuvo.

**INI Tweaks** lista los fragmentos que un mod trae en su carpeta `INI Tweaks/`; los
activados se fusionan en el INI del juego del perfil al lanzar, por orden de
prioridad, y se vuelven a quitar cuando se capturan los INI de la ejecución, así que
desactivar un ajuste quita sus valores.

La pestaña **Nexus files** lista los archivos del mod en Nexus, el más reciente
primero, y marca el instalado aquí y el que lo reemplaza. Esa segunda fila tiene un
botón **Update**; consulta [Actualizaciones de mods](#actualizaciones-de-mods).

La pestaña **Nexus page** muestra la descripción del mod y, el más reciente primero,
el registro de cambios de cada versión de archivo de su página. Eidos lee ambos la
primera vez que abres la pestaña, desde la API GraphQL de Nexus, que no cuenta contra
el presupuesto de peticiones por hora. Los conserva hasta que Eidos se cierra o
pulsas **Refresh**. Se muestran como texto plano: se quita el formato BBCode y HTML,
la dirección de un enlace se escribe tras su texto, y las imágenes y vídeos se
omiten. **Endorse** y **Track**, sobre el texto, hacen lo que el menú de clic derecho
del mod. La página de un mod para adultos permanece oculta salvo que tu cuenta de
Nexus muestre contenido para adultos, y una página sin valoración permanece oculta en
todos los casos.

### Descargas

Una descarga se puede arrastrar desde la lista Downloads a una posición de la lista
de mods para instalarla con esa prioridad, y los archivos o carpetas soltados sobre
la ventana desde un gestor de archivos también se instalan. Soltar archivos necesita
una sesión X11 o XWayland: winit los implementa solo para X11. Las descargas se
pueden pausar y reanudar: pausar detiene la transferencia y conserva el archivo
parcial, y Resume pide a Nexus un enlace nuevo y continúa desde donde se detuvo.

La pestaña Downloads conserva los archivos que descargaste. Fíltrala por nombre (también
por el nombre amistoso del mod, así que «skyui» encuentra `SkyUI_5_2_SE-12604-5-2SE.7z`),
ordénala por más reciente, nombre, tamaño o estado, y oculta un archivo con el que ya
terminaste: el archivo se queda y solo desaparece la fila. «Show hidden» los trae de
vuelta, y el mismo botón los vuelve a mostrar. «Remove N installed» borra los archivos
de los mods que ya instalaste, en dos clics, y solo los que muestra el filtro.

Preferences -> Nexus -> **Install downloads when they finish** está desactivado por
defecto. Activado, una descarga de Nexus que termina mientras la ventana está abierta
se instala sin pasar por la pestaña Downloads. Es la instalación habitual: un FOMOD
abre su instalador, y un nombre que ya está en la lista de mods pregunta qué hacer.
Los archivos que estaban en `downloads/` antes de abrir la ventana o antes de activar
la opción se dejan en paz, y también una descarga cuya instalación cancelaste.
Mientras hay una colección abierta en el panel de colecciones, ninguna descarga se
instala por sí sola, y una que termina entretanto tampoco se instala por sí sola
después; la colección instala sus miembros. El archivo que espera un Update pendiente
lo instala el Update. Solo las descargas de Nexus se instalan así, y una que el filtro
de Downloads oculta espera hasta que se borre el filtro. Mientras hay un arrastre en
curso, o mientras el juego u otro Eidos retiene la instancia, la instalación espera.

### Actualizaciones de mods

El botón de actualización cerca del extremo derecho de la barra de herramientas
comprueba cada mod que tiene un id de Nexus. Cuando la página del mod muestra la
versión que tienes instalada, no se pregunta nada más. Si no, Eidos lee la lista de
archivos del mod (una petición más, conservada durante el resto de la comprobación)
y sigue la cadena de actualizaciones desde el id de archivo que registra el
`meta.ini` del mod: los autores marcan una subida nueva como sucesora de una
anterior, y MO2 lee la misma cadena. Un archivo más nuevo en esa cadena es la
actualización. Cuando la cadena no nombra ninguno y el archivo instalado sigue listado
en la página, el mod cuenta como actual, aunque la página muestre otra versión. La
versión de la página solo decide cuando no hay ningún id de archivo registrado,
cuando el archivo instalado se eliminó o se archivó, o cuando Nexus no envió la lista
de archivos.

Cada mod usa el dominio de Nexus del juego que nombra su propio `meta.ini`
(`gameName`), no el de la instancia: la comprobación de actualizaciones, Endorse,
Track y Visit on Nexus van todos allí. Enderal SE también acepta mods de
`skyrimspecialedition`, así que un mod de Skyrim SE instalado en una instancia de
Enderal SE se comprueba en su página de Skyrim SE. Un enlace Mod Manager Download va a
la instancia que usaste por última vez cuando su juego acepta el dominio del enlace. Si
no, va a una instancia del juego cuyo dominio propio es ese, antes que a una de un juego
que solo lo acepta. `EIDOS_INSTANCE=<folder>` anula ambas cosas.

Track y Untrack en el menú de clic derecho de un mod añaden el mod a tus mods seguidos
en Nexus o lo quitan, de modo que las notificaciones del sitio siguen lo que sigues
aquí. Endorse y Abstain en el mismo menú respaldan el mod en Nexus o retiran el
respaldo; el botón de respaldo junto al botón de actualización respalda el mod
seleccionado. Cada comprobación de actualizaciones también lee de Nexus tus respaldos y
tus mods seguidos (dos peticiones) y ajusta los indicadores Endorsed y Tracked de cada
mod para que coincidan, en el dominio de Nexus del propio mod. Cuando los dos
discrepan, gana Nexus. Si no se puede leer alguna de las dos listas, la comprobación lo
dice y deja esos indicadores como estaban.

El botón **Update** de la pestaña Nexus files de «Information...» instala el archivo
más nuevo. Con una cuenta premium Eidos descarga el archivo él mismo. Con una cuenta
gratuita abre la página del archivo, donde Mod Manager Download devuelve el archivo, y
la instalación empieza cuando el archivo llega a `downloads/`. A partir de ahí es la
instalación habitual: un FOMOD se abre con las respuestas con las que se instaló el
mod, y luego el aviso ofrece Replace con «Keep a backup of the existing mod» marcado,
sea lo que sea lo que elegiste en el último aviso. Replace conserva la prioridad y el
estado de activación del mod.

Antes de que un Replace cambie la carpeta de un mod, Eidos compara la carpeta antigua
con la nueva por nombre de archivo y tamaño, y lee las cabeceras de los plugins de
ambas. Esto cubre Update, una actualización de colección y Replace elegido en el aviso
de instalación. La línea de estado cuenta entonces los archivos añadidos, quitados y
cambiados de tamaño, y nombra cada master que la nueva versión necesita y la antigua
no. La pestaña **General** de «Information...» termina con una sección **Last update**
que lista las versiones, los plugins añadidos o quitados, los masters añadidos o
quitados, los plugins cuya marca ESM, ESL o ESH cambió, los mods con los que este
comparte archivos ahora o ya no, y los archivos. Un archivo reescrito con el mismo
tamaño no se lista. El informe se guarda en `logs/updates/<mod>.json` en la instancia,
que `eidos pack` deja fuera; una instalación nueva o un Merge en la carpeta lo elimina.
`eidos install --replace` imprime los mismos recuentos en una línea `changes:`.

Una carpeta que contiene varios archivos de Nexus, como un archivo principal con un
archivo opcional fusionado en él, se rechaza: reemplazarla con el nuevo archivo
principal descartaría los demás. Actualiza esas a mano.

`eidos nexus update skyrimse --apply` ejecuta la misma comprobación y, con una cuenta
premium, descarga cada archivo nuevo y lo instala sobre su carpeta con
`--replace --backup`. Con una cuenta gratuita imprime en su lugar `OPEN <url>` para
cada página de archivo. Las carpetas que fusionan varios archivos se omiten con el
motivo. Un mod FOMOD sin respuestas registradas, como uno instalado antes de que Eidos
las guardara, no se reinstala con los valores por defecto del instalador: esa
actualización falla con un mensaje que pide actualizar el mod una vez desde la
ventana, lo que registra las respuestas. Un FOMOD cuyo instalador no tiene nada que
elegir se instala como de costumbre. La orden termina con código 1 si alguna
actualización falló.

### Actualizaciones de colecciones

Cuando ejecutas Check for updates, Eidos también pregunta a Nexus por la última revisión de cada colección instalada en esta instancia. Una línea sobre la lista de mods nombra cada colección con una revisión más nueva y cuenta los mods añadidos (+), actualizados (~) y eliminados (-); pasa el puntero para ver sus nombres. Los recuentos faltan cuando la receta de la revisión instalada ya no está en caché. **Update** abre la revisión más nueva en el panel de colecciones, donde *Install this collection* la ejecuta. La lista se reconstruye en cada comprobación y no se guarda.

### Ajustes de Nexus

Preferences -> Nexus -> **Offline** impide que Eidos contacte con Nexus en absoluto, y
omite la comprobación diaria de versiones en GitHub. Las comprobaciones de
actualizaciones, el inicio de sesión, las descargas y las colecciones lo dicen en lugar
de fallar con un error de conexión. Está desactivado por defecto.

**Preferred servers** ordena los nodos de CDN que prefiere una descarga, el mejor
primero. Solo una cuenta premium recibe alguna vez más de un espejo entre los que
elegir, así que para todos los demás Nexus elige y esto no cambia nada. Cuando no se
ofrece ninguno de los servidores que nombraste, la descarga se hace igualmente, desde
el primer nodo que ofrece Nexus.

### Categorías

Las categorías se pueden asignar a un mod o a una selección entera, el catálogo se
puede editar desde el mismo diálogo, y la lista oficial de categorías del juego se
puede traer de Nexus. Ambos archivos del catálogo son los propios de MO2
(`categories.dat` y `nexuscatmap.dat`), así que una instancia compartida conserva un
solo catálogo. Un mod descargado de Nexus se categoriza al instalarse, una vez que
Fetch from Nexus ha asignado el catálogo.

### Editor de INI, registro y extensiones

**View -> INI editor** edita los INI del juego del perfil, la copia que conserva
Eidos; la copia del prefijo de Proton se sobrescribe en cada lanzamiento. Su última
pestaña es el `launch-env.ini` del perfil
([Variables y wrappers por perfil](#variables-y-wrappers-por-perfil)).
**View -> Log** lee los registros de sesión. **View -> Extensions** lista tus propios
complementos; consulta [extensions.md](extensions.md).

Cuando un lanzamiento encuentra que el INI principal del juego (`Skyrim.ini`,
`Fallout4.ini`, `Oblivion.ini`...) no está ni en el perfil ni en el prefijo, Eidos
copia primero el valor por defecto de la carpeta del juego (`Skyrim_Default.ini`,
`Fallout4_Default.ini`...) en el perfil, como hace MO2. Cuando la instancia ejecuta
una copia del juego, el valor por defecto sale de la copia. El prefijo y la carpeta
del juego no se cambian.

### Instalar mods

El instalador maneja archivos Simple, FOMOD, paquetes BAIN de Wrye Bash (marca los
subpaquetes, que se fusionan en orden) y paquetes OMOD de Oblivion. Un script de OMOD
se ejecuta cuando usa los comandos de OBMM que Eidos admite o es uno de los tres
instaladores en C# que Eidos conoce; Eidos no ejecuta ningún otro código C# ni Visual
Basic y lo dice. Cuando ninguna heurística reconoce la estructura, un selector manual
muestra el árbol del archivo y te deja señalar la raíz de datos.

Reinstalar o actualizar un mod FOMOD abre el asistente con las respuestas de su última
instalación marcadas, en el primer paso cuyas opciones cambiaron desde entonces, con
los cambios listados. **Reinstall with new choices** empieza en cambio desde los
valores por defecto del instalador. `eidos install --replace` o `--merge` reutiliza
las respuestas guardadas y se niega, listando los cambios, cuando ya no encajan.

### Diagnostics

La pestaña Diagnostics ejecuta comprobaciones de salud en vivo. Informa de masters
ausentes, plugins que LOOT marca como sucios, archivos que ningún plugin activo
cargará, si la lista de mods sigue coincidiendo con la carpeta de mods, si el
passthrough de FUSE está activado y, cuando se ha pedido, si está presente la
capacidad de lanzamiento que necesita. Tras una ejecución muestra lo que dice el
propio registro del script extender sobre cada una de las DLL de sus plugins, para que
veas si tus plugins de SKSE cargaron. También informa de una actualización del juego
desde el último lanzamiento limpio, un script extender ausente o que no coincide con
la versión del juego (con la página de descarga del extender y un botón de instalación
para el archivo que descargues), una opción de lanzamiento de Steam que no inicia
Eidos con `%command%`, NVAPI desactivado para DLSS, pantalla completa exclusiva que
impide la generación de fotogramas, un `enblocal.ini` de ENB sin `LinuxVersion=true`,
las declaraciones de plugins de los plugins de F4SE y SFSE, y el registro de cierre de
la última sesión con los mods que nombra.

Las comprobaciones de los INI y plugins del perfil buscan líneas `sTestFile` en
`[General]` de los INI personalizados de Fallout 4, Fallout 4 VR y Starfield, que
hacen que el juego ignore `plugins.txt` (Fix las quita de la copia del perfil y
conserva el archivo anterior como `<ini>.pre-fix`; solo para Fallout 4, Fallout 4 VR y
Starfield), plugins `.esp` activados en Starfield, y plugins blueprint a los que
faltan la marca blueprint, el prefijo `BlueprintShips-` o la extensión `.esm`, o cuyo
plugin principal no está activado. Las comprobaciones del sistema señalan un
`vm.max_map_count` o un límite duro de archivos abiertos por debajo de lo que necesita
Proton, y un juego, prefijo o instancia en NTFS, exFAT o FAT. Las comprobaciones de
mods nombran la ruta de datos de BodySlide cuando no apunta al `Data` del juego, las
carpetas `Data` que una herramienta escribió en `Overwrite/Root`, un parche cuyos
archivos sueltos pierden frente al mod que parchea, y un mod que reemplaza los scripts
Papyrus del script extender.

Check for updates también lee los requisitos de Nexus de cada mod. Diagnostics lista un mod requerido que falta o está desactivado, con Ignore para un requisito cumplido de otro modo.

La pestaña también muestra qué versión de Proton ejecuta el juego y avisa antes de que un Proton más antiguo ejecute un prefijo que preparó uno más nuevo; Rebuild aparta un prefijo roto para que Steam cree uno nuevo ([troubleshooting.md](troubleshooting.md#qué-proton-ejecuta-el-juego-y-un-prefijo-roto)). Otras tarjetas señalan un prefijo que nunca se preparó o cuyo registro está obsoleto, un programa distinto de Eidos que recibe los enlaces Mod Manager Download (Register lo arregla), la salida de una herramienta más antigua que sus entradas (con un botón que vuelve a ejecutar la herramienta), y una copia limpiada o light de un plugin cuyo original cambió después. Cada tarjeta se explica en [troubleshooting.md](troubleshooting.md#tarjetas-health). **Copy report** pone en el portapapeles un informe del sistema y de cada hallazgo, con tu carpeta personal y tu nombre de usuario ocultos, para un informe de errores.

Para Skyrim SE, Enderal SE y Fallout 4, Diagnostics también tiene una tarjeta Display. Muestra el tamaño y el modo de ventana definidos en el `SkyrimPrefs.ini` del perfil (`EnderalPrefs.ini`, `Fallout4Prefs.ini`), o en la copia del prefijo cuando el perfil aún no tiene ninguno. Cuando ninguno define un tamaño, sus campos parten de la pantalla conectada más grande que figura en `/sys/class/drm`, así que una Steam Deck lee 1280x800. El ancho y el alto son números enteros de 320 a 16384. Apply escribe `iSize W`, `iSize H`, `bFull Screen` y `bBorderless` en la copia del perfil, después de adoptar la copia del prefijo si el perfil no tiene una. Cuando SSE Display Tweaks fija la ventana, Apply escribe el mismo tamaño y modo en su copia en el Overwrite; el archivo del propio mod no se cambia.

La tarjeta Game files, arriba de la pestaña, se describe en [Archivos del juego](#archivos-del-juego-la-instalación-de-la-tienda-o-una-copia), la tarjeta de orden de colección en [Mantener el orden de la colección](#mantener-el-orden-de-la-colección), y Find the mod behind a problem en [Ejecuciones de prueba](#encontrar-el-mod-detrás-de-un-problema).

### Lanzar el juego

Para lanzar el juego a través de la ventana, pon como opción de lanzamiento de Steam
del juego la ruta absoluta del programa, porque Steam no lee el `PATH` de tu shell
([install.md](install.md#después-apuntar-steam-hacia-él)):

```
~/.local/bin/eidos-gui %command%
```

Desde Colony, la ruta es `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eidos se
abre en la instancia del juego, la que usaste por última vez, de modo que una
instancia portátil se vuelve a encontrar igual que la global; haz clic en Run para
lanzarlo a través de la vista combinada.

Con Start the game at once when a store opens Eidos activado (Preferences, General,
Running a game; desactivado por defecto), Eidos se salta ese clic: comprueba Health y
arranca el juego sin mostrar su ventana. La ventana se abre como de costumbre cuando
Health lista un problema, cuando la lista de mods necesita atención, cuando una
herramienta configurada para ejecutarse antes de jugar tiene una salida desactualizada
o, para Skyrim SE, Enderal SE y Fallout 4, cuando el perfil aún no tiene tamaño de
ventana (la ventana se abre entonces en Diagnostics). Para obtener la ventana en un
solo lanzamiento, pon `EIDOS_WINDOW=1` antes de la orden:
`EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

En una ventana que no abrió ninguna tienda, Run no arranca el juego por sí mismo, y
Eidos nunca arranca un juego directamente a través de Proton o Wine. Run pide a la
tienda que posee la copia que lo arranque, y la tienda arranca Eidos:

- Steam ejecuta `steam -applaunch <appid>` cuando la opción de lanzamiento de arriba
  está definida y el juego tiene un prefijo de Proton, que Steam crea la primera vez
  que arranca el juego. Steam arranca primero si está cerrado.
- Heroic (una copia de GOG o Epic, nativa o Flatpak) recibe un enlace
  `heroic://launch` cuando Eidos es un wrapper en los ajustes del juego en Heroic, con
  la ruta absoluta de `eidos-gui` (o `eidos-linux`) como ejecutable y sin argumentos.
  Heroic arranca primero si está cerrado.
- Legendary por sí solo ejecuta `legendary launch <app>` cuando
  `~/.config/legendary/config.ini` tiene `wrapper =` y la ruta absoluta de
  `eidos-gui` (o `eidos-linux`) bajo la sección `[<app>]` del juego, o bajo
  `[default]`.

El cargador del script extender, el lanzador y el ejecutable del juego en la lista Run
se entregan todos a la tienda de este modo, y también
`eidos tool <instance> run <entry>` para uno de ellos en una terminal. Eidos no puede
entregar algunas copias a su tienda, como una copia de GOG fuera de Heroic: Run dice
entonces que arranques el juego desde esa tienda.

La entrega registra esta instancia como la que usaste por última vez, y la tienda abre
entonces una segunda ventana de Eidos sobre ella. Con Start the game at once desactivado,
haces clic en Run ahí; con él activado, esa ventana arranca el juego sin mostrarse,
salvo que uno de los casos de arriba la abra. La primera ventana no sigue esa
ejecución, así que pulsa F5 en ella después de jugar para recargar sus listas. Cuando
falta el ajuste de la tienda, o el cargador del script extender no está junto al
ejecutable del juego, Run no arranca nada y muestra qué definir y dónde, con la ruta
real del binario en ejecución.

Mientras un juego o una herramienta se ejecuta, Run pasa a ser Stop. Dos pulsaciones
terminan todos los programas que inició la ejecución, y Eidos captura entonces la
sesión y desmonta como tras cualquier salida; una ejecución detenida nunca cuenta como
un lanzamiento limpio
([troubleshooting.md](troubleshooting.md#un-juego-o-una-herramienta-que-no-se-cierra)).

El `%command%` de Steam para los títulos de Bethesda suele apuntar a
`<Game>Launcher.exe`. Eidos nunca lo ejecuta: el lanzador es una aplicación de ajustes
aparte que reexamina `Data` y reescribe `plugins.txt`, deshaciendo el orden de carga
recién desplegado. Eidos ejecuta en su lugar el cargador del script extender si hay
uno instalado, y el binario del juego en caso contrario, y lo dice cuando tiene que
recurrir al binario del juego.

Eidos examina las DLL de los mods activados en busca de una importación de
`d3dcompiler_47.dll`, copia la DLL de Microsoft en el prefijo y solo entonces fija
`WINEDLLOVERRIDES=d3dcompiler_47=n,b`; no necesitas fijarlo tú mismo.

### Variables y wrappers por perfil

Un perfil puede llevar sus propias variables de entorno y órdenes wrapper en `profiles/<name>/launch-env.ini`, una entrada por línea:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Se aplican a cada `eidos play` de ese perfil (el juego arrancado por el `%command%` de Steam, o por Run en una ventana que abrió una tienda) y a cada ejecución de herramienta. `eidos shell` recibe las variables y ningún wrapper. La primera línea `wrapper=` es la más externa. Una línea `wrapper=` que contiene `%command%` recibe la orden en ese lugar, como en las opciones de lanzamiento de Steam; una línea sin él recibe la orden al final. Los wrappers se ejecutan dentro del espacio de nombres privado de Eidos, así que ven los mods.

Una variable aquí reemplaza el valor que Eidos fija para el mismo nombre, y la línea `env=` propia de una herramienta en `tools.ini` reemplaza a ambos. `WINEDLLOVERRIDES`, en cambio, se añade después de los reemplazos que Eidos fija para ENB, ReShade y las DLL incluidas, de modo que una DLL nombrada en ambos toma el ajuste del perfil y las demás conservan el suyo.

View -> INI editor abre el archivo como su última pestaña, para cada juego. Las líneas en blanco y las que empiezan por `#` o `;` se omiten. Al guardar se listan los números de línea que Eidos ignora: las líneas que no son ni `env=NAME=value` ni `wrapper=command`, y una línea `wrapper=` cuya primera palabra es `NAME=value`, como permiten las opciones de lanzamiento de Steam; pon esa variable en una línea `env=` en su lugar. `eidos play <instance>` sin orden lista las variables que recibe una ejecución y los wrappers. Copy report en Diagnostics nombra las variables y los programas wrapper, sin sus valores.

## Herramientas

xEdit, BodySlide, DynDOLOD y otras herramientas se ejecutan a través de la vista
combinada dentro del prefijo de Wine del juego (el prefijo de Proton de Steam, o el que
configuró Heroic para una copia de GOG o Epic):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # ejecutar un exe una vez sin registrarlo
eidos prereqs skyrimse            # lo que necesitan las herramientas registradas, y su estado
eidos prereqs skyrimse --install  # traer lo que falte
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen y Synthesis se encuentran
sin registrarlos. Pandora, Nemesis, FNIS, BodySlide y Synthesis llegan con un mod de
captura ya definido, así que su salida va a un mod propio (detalles en
[tools.md](tools.md#enviar-la-salida-de-una-herramienta-a-su-propio-mod)).

El título que le das a una herramienta decide qué runtimes y DLL le proporciona Eidos:
`BodySlide` recibe sus bibliotecas de DirectX, `BS` no recibe nada. En la ventana, el
diálogo Executables muestra el estado real de cada prerrequisito bajo el campo, y los
que faltan son botones.

El paquete de versión trae los instaladores de vcrun2022 (x86 y x64), el .NET 8 Desktop
Runtime 8.0.12 y el runtime de .NET 10 10.0.10 en `runtimes/`, e `install.sh` los copia
a `~/.local/share/Colony/Eidos/runtimes/`, así que `eidos prereqs <instance> --install`
los instala sin descargar nada. Eidos ejecuta en silencio los instaladores de vcrun2022 y
de .NET 8 de Microsoft en el prefijo del juego, sin winetricks. `eidos-linux` y las
compilaciones desde el código fuente no traen carpeta `runtimes/`, así que descargan de
Microsoft los mismos archivos fijados. El SDK de .NET 10 que necesita Synthesis (unos
300 MB) siempre es una descarga.

La tabla, los tres niveles de prerrequisitos, por qué DynDOLOD necesita un runtime de
.NET que winetricks no puede instalar, y por qué una herramienta instalada como mod se
lanza desde la ruta combinada en lugar de desde su propia carpeta, están en
[tools.md](tools.md).

## Extensiones

Eidos se puede extender sin recompilarlo: un manifiesto TOML en
`~/.config/Colony/Eidos/addons/` añade una herramienta a la lista Extensions o una
comprobación a la pestaña Diagnostics. No se carga nada dentro de Eidos: una extensión
es un programa que él ejecuta. Consulta [extensions.md](extensions.md).

## Probar la prueba de concepto

El script no necesita ningún juego. Muestra la vista combinada, el copy-on-write, una
carpeta de juego que se queda intacta, y una fusión que existe solo dentro de su espacio
de nombres, usando solo OverlayFS sin privilegios en un espacio de nombres de usuario
(Linux 5.11 o posterior):

```sh
./scripts/poc-overlay.sh
```
