<!-- eidos-i18n: source=README.md sha=37e97bafbd2cf43f2101a28b23c1464420a5f9ec -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Un gestor de mods nativo de Linux que nunca toca la carpeta de tu juego.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#instalación)

Eidos da a los juegos de Bethesda en Linux lo que Mod Organizer 2 les da en
Windows: una vista combinada de tus mods, construida en cada lanzamiento. Usa
espacios de nombres de montaje de Linux y FUSE en lugar del enganche de la API de
Windows, y el gestor se ejecuta sin Wine. No se copia nada en la carpeta del
juego, así que después no hay nada que limpiar. Eidos también tiene definiciones
para Stellar Blade, Stardew Valley, 7 Days to Die y Crimson Desert.

Las copias de Steam ejecutan Eidos desde la opción de lanzamiento del juego. Las
copias de GOG y Epic en Heroic o Legendary lo ejecutan como wrapper. El juego
siempre arranca a través de la tienda que posee la copia; Eidos nunca lo arranca
él mismo con Proton.

```
Steam ──> eidos-gui %command% ──> [ espacio de nombres privado ]
                                  │  mods ⊕ juego  ──> lo que el juego ve
                                  └─ muere con el juego; la instalación queda intacta
```

> **Estado:** Skyrim SE se juega a través de Eidos a diario, con SKSE,
> precargadores del script extender, Creation Club, órdenes de carga ordenados
> con LOOT y partidas por perfil. Hasta ahora hay una familia de juegos probada en
> juego real. Otros diez juegos de Bethesda y cuatro más están cableados y
> esperan probadores.

## Por qué Eidos

La vista combinada se monta en un espacio de nombres de montaje privado que solo
puede ver el árbol de procesos del juego. Los demás programas, incluido un segundo
juego, nunca la ven y no necesitan ningún permiso para ella. Cuando el juego
termina o se mata, o el equipo pierde la corriente, el espacio de nombres
desaparece con sus procesos y la instalación queda como estaba.

Cada perfil es dueño de su lista de mods, su orden de plugins, sus INI y sus
partidas. Al lanzar, los archivos de plugins y la carpeta de partidas se montan
con bind sobre las rutas propias del juego, así que lo que el juego escribe ahí
aterriza en el perfil. Cambiar de perfil lo cambia todo.

Eidos no necesita ayudante setuid, ni demonio del sistema, ni `sudo setcap`, ni
editar `/etc/fuse.conf`. El demonio FUSE se ejecuta como tu usuario durante un
lanzamiento y termina con el juego. Instalas un programa y fijas una opción de
lanzamiento de Steam.

Antes de cada ejecución, Eidos guarda una instantánea de la lista de plugins. Si
un cierre inesperado daña la lista, Diagnostics la compara con esa instantánea, y
Restore the pre-session order devuelve el orden anterior. Una captura que borraría
tu orden de carga se rechaza, con el motivo.

Test run inicia el juego con los mods del perfil, sin mods, o solo con los mods
seleccionados y sus masters. Las partidas, los cambios de INI, el estado de los
plugins, la salida del Overwrite y los demás archivos de My Games de esa ejecución,
registros incluidos, van a `<instance>/test-run/`. Cuando el juego termina, Eidos
lista esos cinco grupos y conservas o descartas cada uno. Las demás escrituras,
como el registro de Wine, van adonde van siempre. Find the mod behind a problem,
en Diagnostics, ejecuta ejecuciones de prueba con la mitad de los mods sospechosos
cada vez, más los masters que necesitan. Después de cada ejecución respondes si el
problema ocurrió, y Eidos va reduciendo los sospechosos hasta nombrar el mod, o los
mods que necesitan los masters unos de otros; 200 sospechosos llevan unas 8
ejecuciones. Los mods seleccionados en la lista antes de empezar siguen activos en
cada paso. Detalles en
[usage.md](docs/guide/usage.md#ejecuciones-de-prueba) y
[usage.md](docs/guide/usage.md#encontrar-el-mod-detrás-de-un-problema).

### Cómo se compara

| | Eidos | MO2 con Wine | Fluorine-Manager | Limo / desplegadores por enlaces |
|---|---|---|---|---|
| El gestor se ejecuta de forma nativa | sí | no, una aplicación de Windows en Wine | sí (port a Qt) | sí |
| Carpeta del juego intacta | sí, siempre | sí | sí | no, se escriben enlaces en ella |
| Montaje visible para | solo el juego | solo el juego | todo el sistema | no aplica |
| Limpieza tras un cierre inesperado | ninguna | ninguna | recuperación de montajes obsoletos | desdespliegue manual |
| Mods root (ENB, precargadores) | sí, de forma nativa | requiere un plugin | requiere un plugin | parcial |
| Privilegios necesarios | ninguno | ninguno | edición de `/etc/fuse.conf` | ninguno |

### Su rapidez

| | antes | ahora |
|---|---|---|
| cargar una partida | ~20 segundos | 6-7 segundos |
| escaneos de directorio en una sesión | 5,6 millones | 335 mil |

Los cambios de celda son inmediatos. Encontrar un archivo solía preguntar a cada
uno de cincuenta mods por turno, y listar una carpeta lo hacía cincuenta veces más.
Las capas de solo lectura ahora se indexan una vez al montar, así que encontrar un
archivo es una búsqueda hash, y el listado combinado de una carpeta también se
calcula una vez al montar. Las cifras salen de una instancia real jugada con
normalidad, no de un benchmark:
[internals/performance.md](../../internals/performance.md).

### Trabajos previos y agradecimientos

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) y
  [usvfs](https://github.com/ModOrganizer2/usvfs): la semántica que reproduce
  Eidos, y la base de código frente a la que se estudió su paridad
- [LOOT](https://loot.github.io/): el motor de ordenación, mediante libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) y los demás gestores de Linux, que
  muestran que hay una comunidad que quiere ver esto resuelto

## Qué hace

### Mods

Eidos instala archivos simples, asistentes FOMOD, paquetes BAIN de Wrye Bash,
contenedores OMOD con scripts de OBMM compatibles, y cualquier otra cosa mediante
un selector manual. Los mods root (precargadores del script extender, ENB, Engine
Fixes) funcionan de forma nativa, sin plugin Root Builder y sin copiar nada en tu
instalación. Puedes ocultar archivos sueltos, agrupar mods con separadores, mover
una selección a un destino, guardar notas y categorías por mod, e importar un
perfil de MO2.

La lista funciona como la de MO2: ocho columnas opcionales y ordenación por
cualquiera de ellas, agrupación por categoría o por origen, gestos de doble clic,
salto al escribir, copias de seguridad por mod que no hacen nada hasta que las
restauras, e indicadores para un mod cuya estructura este juego no cargará o que se
descargó para otro juego. Los filtros eligen mods por estado en Nexus (respaldado,
seguido, actualización disponible), notas, archivos ocultos y tipo de contenido
(plugins, archivos, texturas, mallas, scripts, sonido y más). El árbol de archivos
crea carpetas, renombra, borra y abre archivos, y muestra una vista previa de
imágenes y texto sin lanzar nada.

Ctrl+Z y Ctrl+Mayús+Z deshacen y rehacen los cambios en la lista de mods y en el
orden de carga. Desactivar o quitar un mod que usa una de las tres partidas más
recientes del perfil pide confirmación antes y nombra los plugins (partidas de
Skyrim y Fallout 4). Reinstalar o actualizar un mod FOMOD abre el asistente con las
respuestas de su última instalación. La información de un mod lista los mods que
necesita y los mods que lo necesitan, leídos de los masters de los plugins
activados.

### Plugins

El orden de carga trae la ordenación de LOOT integrada, los índices de mod tal como
los calcula el juego, avisos de masters ausentes, y tu contenido de DLC y Creation
Club mostrado como filas no gestionadas. El encabezado cuenta los espacios usados en
cada espacio de índices (full, medium, light). Al pasar el puntero sobre un plugin se
muestran su autor, su descripción y la versión de su cabecera. En los motores de
Skyrim se marca un plugin cuyo INI del mismo nombre carga el juego.

Los plugins que LOOT marca como sucios los limpia el QuickAutoClean de xEdit en un
mod `Cleaned plugins`, nunca en su sitio. Flag as light, en un mod nuevo, escribe una
copia light de un plugin, y se niega cuando una de las partidas más recientes carga
ese plugin como plugin completo (partidas de Skyrim y Fallout 4).

### Instancias

Una instancia es global, guardada bajo `~/.local/share/Colony/Eidos/instances/`, o
portátil: una carpeta autónoma donde quieras (un segundo disco, una partición de
juegos), movible y aislada como las de MO2. Las instancias portátiles se recuerdan
entre sesiones. La interfaz, el lanzamiento desde Steam y cada orden de la CLI siguen
a la que usaste por última vez, y cualquier orden acepta la carpeta allí donde acepta
un identificador de juego. Detalles en
[usage.md](docs/guide/usage.md#instancias-globales-y-portátiles).

Una instancia también puede ejecutar el juego desde una copia de la carpeta del juego
dentro de la instancia. Cuando Eidos abre por primera vez una lista de Wabbajack,
ejecuta la copia propia de la lista: la carpeta que nombra su `ModOrganizer.ini`, o si
no una carpeta `Stock Game` o `Game Root`. Freeze this version, en la tarjeta Game
files de Diagnostics, copia la instalación de la tienda en `<instance>/Stock Game`,
de modo que una actualización de la tienda ya no cambia los archivos con los que se
ejecuta el juego. Cuando ambas carpetas están en un mismo sistema de archivos que
puede compartir bloques, como btrfs o XFS, la copia los comparte con la instalación
de la tienda (reflink). En ext4, o entre dos sistemas de archivos, es una copia
completa, y se muestran antes el tamaño y el espacio libre. La tienda sigue iniciando
el juego. Return to the store version y Delete the copy lo deshacen, y
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
hace lo mismo en una terminal. Morrowind, y Oblivion con
`bUseMyGamesDirectory=0`, no pueden ejecutarse desde una copia. Detalles en
[usage.md](docs/guide/usage.md#archivos-del-juego-la-instalación-de-la-tienda-o-una-copia).

### Perfiles

Cada perfil tiene su propio orden de mods, estado de plugins, INI y partidas. Las
partidas se analizan y se comparan con tus plugins actuales, con un botón que activa
lo que una partida necesita, y se copian de vuelta al prefijo para Steam Cloud tras
cada sesión. Las partidas que Steam Cloud trajo de otro dispositivo, o que un
lanzamiento sin Eidos dejó en el prefijo, se listan en la pestaña Saves. Import las
copia al perfil, y una partida cuyo nombre el perfil ya usa entra con un nombre
`orphan-` nuevo en lugar de reemplazarla.

Un perfil puede llevar variables de lanzamiento y wrappers (`env=NAME=value` y
`wrapper=command`, uno por línea, en el editor de INI) para sus ejecuciones del juego
y de herramientas. Para Skyrim SE, Enderal SE y Fallout 4, una tarjeta Display en
Diagnostics fija el tamaño y el modo de ventana en las copias de INI del perfil.
Backups conserva puntos de restauración de los INI del perfil, y Eidos toma uno antes
de cada ejecución de BethINI Pie. El acceso directo de escritorio de un perfil
arranca el juego a través de su tienda. `eidos sessions` lista las ejecuciones
registradas y compara una con el estado actual.

### Nexus

Conecta una cuenta para recibir enlaces Mod Manager Download, buscar actualizaciones
y ver autores y sus perfiles. Cada mod se comprueba, se respalda y se enlaza en el
dominio de Nexus de donde vino su archivo, así que un mod de Skyrim SE en una
instancia de Enderal SE se queda en el sitio de Skyrim SE. Una actualización sigue la
cadena de sucesores que el autor fijó en Nexus, reemplaza la carpeta del mod con el
archivo nuevo (se ofrece una copia de seguridad de la antigua), conserva su prioridad
y su estado de activación, y repite las respuestas de FOMOD guardadas en la
instalación. `eidos nexus update <instance> --apply` hace lo mismo en una terminal con
una cuenta premium, e imprime las páginas de los archivos con una gratuita. Track y
Endorse están en el menú del mod, y cada comprobación de actualizaciones los
sincroniza con tu cuenta de Nexus. La información de un mod tiene una pestaña Nexus
page con su descripción y su registro de cambios como texto plano, y su pestaña
General termina con lo que cambió la última actualización: los archivos añadidos,
quitados y de otro tamaño, y los masters añadidos o quitados. Install downloads when
they finish (Preferences, Nexus, desactivado por defecto) instala una descarga en
cuanto termina mientras la ventana está abierta.

Las colecciones se instalan a partir de sus recetas con progreso reanudable e
informes explícitos de los pasos no admitidos. La comparación comprueba los archivos
de origen exactos; los metadatos antiguos incompletos quedan sin verificar. El panel
de colecciones explora y busca las colecciones publicadas para un juego, muestra
cuándo una colección instalada tiene una revisión más nueva, y guía a una cuenta
gratuita por los archivos que faltan, una página cada vez. Cuando un arrastre rompe
las reglas de orden de una colección, Diagnostics lo dice, y Restore collection order
devuelve sus mods a su sitio. La pestaña Downloads filtra, ordena, oculta y quita
archivos. Offline mode (Preferences, Nexus) detiene el acceso a Nexus y la
comprobación diaria de versiones.

### Herramientas

xEdit, BodySlide, DynDOLOD y otras herramientas de modding se ejecutan a través de la
vista combinada dentro del prefijo de Wine del juego (el prefijo de Proton de Steam, o
el que configuró Heroic). Ven tus mods, su salida aterriza en el Overwrite, y un clic
convierte esa salida en un mod de verdad. Para Skyrim SE, Eidos encuentra SSEEdit,
SSEEdit QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS, BodySlide, DynDOLOD,
TexGen, xLODGen, Synthesis y BethINI Pie en la carpeta del juego, dentro de un mod, o
en la carpeta de herramientas que guardas junto a tus juegos, cada una con los
runtimes que necesita. Los demás juegos tienen su propia lista. Pandora, Nemesis,
FNIS, BodySlide y Synthesis escriben su salida en un mod con nombre (`Nemesis Output`
y así sucesivamente).

El paquete de versión trae los instaladores de Visual C++ 2015-2022 y .NET 8 Desktop
Runtime de Microsoft y el runtime de .NET 10. Eidos instala `vcrun2022` y .NET 8 con
esos instaladores, sin winetricks, y sin el paquete descarga los mismos archivos de
Microsoft, cada uno comprobado con el SHA-256 integrado en Eidos. Synthesis recibe el
SDK de .NET 10, una descarga de 300 MB que el paquete no trae. Los demás
prerrequisitos (.NET Framework 4.8, XACT y versiones antiguas de .NET) siguen pasando
por winetricks. No se descarga nada hasta que pides instalar los prerrequisitos de una
herramienta.

Fija las herramientas que usas, oculta las que no, dale a una herramienta su propio
AppID de Steam cuando es su propia aplicación de Steam, y escribe un acceso directo
`.desktop` que la lanza a través de la vista combinada sin abrir Eidos.
`eidos tool <game> import-mo2` lista los ejecutables del `ModOrganizer.ini` de la
carpeta de una lista, y `--write` los añade. Un exe en la pestaña Data se puede
ejecutar una vez o añadir como herramienta, y `eidos tool <instance> run --exe <path>`
ejecuta uno desde una terminal. Con Ask before playing, Run pregunta antes cuando la
salida de un generador está desactualizada.

Stop termina un juego o una herramienta en ejecución. El menú File ejecuta winecfg,
regedit, el explorador de archivos de Wine y el desinstalador de Windows en el
prefijo del juego. Rebuild prefix, en Diagnostics, renombra el prefijo para que Steam
cree uno nuevo, y no borra nada. `eidos shell <instance>` abre una shell en la vista
combinada, o ejecuta la orden dada después de `--`, y lo que escribe aterriza en el
Overwrite.

Mientras la ventana está en segundo plano, Eidos puede enviar una notificación de
escritorio cuando termina la ejecución de una herramienta, la instalación de una
colección, un empaquetado o desempaquetado, o la última descarga en curso, y cuando
falla una ordenación de LOOT. Notify the desktop when a long job ends, en
Preferences, lo activa; está desactivado por defecto.

### Diagnostics

La pestaña Diagnostics lista masters ausentes, archivos huérfanos, desajustes de la
lista de mods y conjuntos de plugins dañados, y, tras una ejecución, lo que dice el
propio registro del script extender que se cargó. También informa de:

- un script extender ausente o que no coincide con la versión del juego
- un juego que cambió desde el último lanzamiento limpio
- una opción de lanzamiento de Steam que no inicia Eidos o no tiene `%command%`
  después
- un Proton más antiguo que el que preparó el prefijo por última vez, o uno del
  Steam de Flatpak
- `PROTON_ENABLE_NVAPI` ausente para DLSS, y pantalla completa exclusiva, que
  impide la generación de fotogramas de Community Shaders
- un `enblocal.ini` sin `LinuxVersion=true`
- el registro de cierre de la última sesión y los mods que nombra
- requisitos de Nexus ausentes o desactivados
- un manejador `nxm://` que es otro programa o un archivo ausente
- líneas `sTestFile`, que hacen que el juego ignore `plugins.txt`
- plugins `.esp` activados en Starfield, y plugins blueprint no válidos o sin pareja
- un parche cuyos archivos sueltos pierden frente al mod que parchea
- un mod cuyas copias de los scripts Papyrus del script extender ganan a las del
  propio extender
- un `vm.max_map_count` o un límite de archivos abiertos bajos
- archivos del juego en un sistema de archivos de Windows
- rutas de datos de BodySlide que no son la carpeta Data del juego, y carpetas en
  `Overwrite/Root` que pertenecen a Data
- un `ModOrganizer.ini` de una lista que apunta las herramientas a otra carpeta de
  juego
- una copia del juego que falta o no tiene ejecutable del juego

`eidos check <instance> [--json]` ejecuta las comprobaciones que no necesitan
ventana y termina con 1 cuando encuentra un problema. Copy report copia un informe con
tu carpeta personal escrita como `~` y tu nombre de usuario como `<user>`, y Save tree
en la pestaña Data escribe el árbol Data combinado con el origen de cada archivo,
censurado del mismo modo.

Consulta [Integrity checks and generated output](../../internals/integrity.md) para la
protección contra reemplazos, los conflictos entre miembros de BSA/BA2, los
diagnósticos estáticos de SKSE y los recibos de los generadores, con sus límites.

### Dónde guarda sus propios archivos

Eidos usa la disposición que usa todo programa de la familia Colony:
`~/.config/Colony/Eidos/` para lo que elegiste (preferencias, tu sesión de Nexus, tu
lista de instancias, las definiciones de juegos y extensiones que escribiste),
`~/.local/share/Colony/Eidos/` para las instancias globales, los runtimes de
herramientas (descargados, o copiados del paquete por `install.sh`) y los registros de
sesión, y `~/.cache/Colony/Eidos/` para las masterlists de LOOT. Dentro de una
instancia, `Stock Game` contiene una copia congelada del juego y `test-run/` contiene
una ejecución de prueba a la espera de revisión.

Un Eidos más antiguo guardaba sus archivos en `~/.config/eidos/`,
`~/.local/share/eidos/` y `~/.local/state/`. El primer lanzamiento tras actualizar los
traslada, lo dice en el registro y deja los directorios antiguos como estaban. Hasta
que un traslado se haya completado, Eidos sigue usando los archivos antiguos.
Detalles en [internals/paths.md](../../internals/paths.md).

### Juegos compatibles

Skyrim SE/AE está probado en juego real. Fallout 4 también está cableado de punta a
punta (F4SE sustituido automáticamente, invalidación de archivos, orden de carga con
asteriscos, LOOT, partidas `.fos`); consulta
[guide/fallout4.md](docs/guide/fallout4.md). Cableados y en busca de probadores:
Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV, Starfield,
Oblivion y Morrowind. Fallout 3, Fallout NV, Oblivion y Morrowind ordenan los plugins
por las fechas de los archivos: Eidos guarda un orden por perfil y muestra esas
fechas dentro del montaje, sin cambiar los archivos del disco.

Otros cuatro juegos están cableados y también buscan probadores: Stellar Blade (mods
`.pak` de Unreal en `~mods`), Stardew Valley (mods de SMAPI, con SMAPI instalado por
separado), 7 Days to Die (carpetas `Mods/` junto al juego) y Crimson Desert. Crimson
Desert admite solo mods de código: plugins ASI, ReShade y DLL de cargadores en
`bin64`. Sus mods de datos, que viven en grupos PAZ, no se admiten.

Un juego es un archivo TOML, y el tuyo puede ir en `~/.config/Colony/Eidos/games/` sin
recompilar: [internals/adding-games.md](../../internals/adding-games.md).

## Instalación

### Mediante Colony (recomendado)

Busca **Eidos** en [Colony](https://github.com/Project-Colony/Colony) e instálalo.
Las actualizaciones llegan a través del lanzador. Colony instala un solo archivo,
`eidos-linux`: la ventana, y también la línea de órdenes `eidos` cuando se llama a
través de un enlace llamado `eidos`. Ese archivo no trae ni el programa auxiliar de
vista previa de modelos NIF ni los runtimes de Microsoft del paquete de más abajo. Sin
el programa auxiliar la vista previa lo indica y todo lo demás funciona, y Eidos
descarga un runtime de Microsoft cuando instalas los prerrequisitos de una
herramienta. Qué hacer después de instalarlo, y cómo apuntar Steam hacia él:
[docs/guide/install.md](docs/guide/install.md#mediante-colony).

### Descarga directa del binario

Descarga un archivo de la [última versión](https://github.com/Project-Colony/Eidos/releases/latest):

| Archivo | Qué es |
|---|---|
| `eidos-linux` | el programa completo en un solo archivo, tal como lo instala Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, el programa auxiliar de vista previa NIF, los instaladores de Visual C++ 2015-2022 y .NET 8 Desktop Runtime y el runtime de .NET 10, iconos, avisos de licencia y un instalador que registra la entrada de escritorio y el manejador de descargas de Nexus |

Comprueba primero la descarga, como muestra la
[Política de firma de código](#política-de-firma-de-código), y luego:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

El instalador pone los binarios en `~/.local/bin` sin sudo y copia los `runtimes/` del
paquete a `~/.local/share/Colony/Eidos/runtimes`. `--system` y `--bindir DIR` ponen
los binarios en otro sitio, y `--from DIR` los toma de otra carpeta. `--cap` pide a
sudo `CAP_SYS_ADMIN`, que solo necesita el passthrough experimental de FUSE, y el
passthrough sigue desactivado; `--no-cap` es el valor por defecto. Ambos archivos
necesitan Linux x86_64 con glibc 2.39 o posterior, FUSE (`fusermount3`) y 7-Zip para
los archivos de mods.

Eidos no se actualiza solo. Una vez al día al arrancar, la ventana pregunta a GitHub
por la última versión, y cuando hay una más nueva muestra un enlace a sus notas de
versión después de Help en la barra de menús. Preferences lo desactiva, y una copia
bajo `/usr` fuera de `/usr/local`, que actualiza un gestor de paquetes, nunca
pregunta.

### Compilar desde el código fuente

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Requiere Rust 1.92 o posterior (la `rust-version` que comprueba CI; CI y las
versiones publicadas compilan con 1.99.1), además de `just`, CMake 3.20+ y un
compilador C++17 para el programa auxiliar de vista previa NIF. `just build` usa el
perfil release: una unidad de generación de código y LTO completo. Una compilación en
frío de `eidos` y `eidos-gui` con 3 trabajos de compilación tardó 334 segundos así y
243 segundos con LTO ligero, y el LTO completo hace `eidos` 1,2 MB más pequeño y
`eidos-gui` 3,8 MB más pequeño (mediciones en `Cargo.toml`).
`just profile=dev build` compila más rápido. Un paquete de Arch se compila desde
`packaging/PKGBUILD`. Requisitos previos y las órdenes sin `just`:
[docs/guide/install.md](docs/guide/install.md).

### Opciones de lanzamiento de Steam

Eidos se ejecuta como la orden de lanzamiento del juego, que es como llega a montar
antes de que el juego arranque. La línea base es todo lo que necesitan la mayoría de
las instalaciones, con la ruta absoluta de tu instalación (Steam no lee tu `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Desde Colony esa ruta es `~/.local/share/Colony/apps/Eidos/eidos-linux`. Luego pulsa
Jugar: Eidos se abre en la instancia de ese juego, y Run inicia el juego a través de
la vista combinada.

En una ventana que abriste tú mismo, Run pide a Steam que arranque el juego
(`steam -applaunch <appid>`), así que la opción de lanzamiento sigue aplicándose, y
pulsas Run en la ventana que abre Steam. Heroic recibe un enlace `heroic://launch`, y
Legendary ejecuta `legendary launch <app>`, una vez que Eidos está fijado allí como
wrapper del juego. Cuando falta el ajuste de la tienda, Run no inicia nada y dice qué
hay que fijar: [guide/usage.md](docs/guide/usage.md#lanzar-el-juego). Con Start the
game at once when a store opens Eidos (Preferences, desactivado por defecto), Eidos
ejecuta las comprobaciones de Diagnostics y arranca el juego sin mostrar su ventana
cuando nada necesita atención.

Todo lo demás son variables de entorno apiladas por delante, y se combinan
libremente:

| Quieres... | Pon delante |
|---|---|
| DLSS con Community Shaders | `PROTON_ENABLE_NVAPI=1`. Sin ella DLSS nunca se inicializa y no dice nada; la lista completa está en [guide/graphics.md](docs/guide/graphics.md) |
| un contador de FPS en pantalla | `DXVK_HUD=fps` |
| interpolación de fotogramas a nivel de controlador, cero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, nunca junto con la generación de fotogramas de Community Shaders |
| registros detallados para un informe de errores | `EIDOS_LOG=debug` (los registros de sesión aterrizan en `~/.local/share/Colony/Eidos/logs/`) |
| un informe de E/S por sesión desde el montaje | `EIDOS_FUSE_STATS=1` |
| otro número de hilos de FUSE | `EIDOS_FUSE_THREADS=8` (4 por defecto; `1` es lo primero que probar cuando persigues un fallo de concurrencia) |
| este lanzamiento fijado a una instancia portátil | `EIDOS_INSTANCE=/path/to/folder`. Sin ella Eidos abre la instancia que usaste por última vez |
| la ventana en un solo lanzamiento con Start the game at once activado | `EIDOS_WINDOW=1` |

La línea que conservar para una instalación moderna con mods (Community Shaders,
DLSS, generación de fotogramas) es la orden final, no un ejemplo:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Añade `DXVK_HUD=fps` delante mientras compruebas que la instalación funciona, y
quítalo cuando lo haga.

Las mismas variables pueden ir en las variables de lanzamiento del perfil en lugar de
en la línea de Steam, lo que las mantiene por perfil y las aplica también a las
ejecuciones de herramientas:
[guide/usage.md](docs/guide/usage.md#variables-y-wrappers-por-perfil).

Los interruptores de diagnóstico más profundos (`EIDOS_FUSE_TRACE`, los
conmutadores de bisección de caché e índice, por qué `EIDOS_FUSE_PASSTHROUGH` está
desactivado por defecto) están en
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Documentación

Todo está en [docs/](../../), indexado según quién lee en
[docs/README.md](docs/README.md): la guía de usuario (instalación, la línea de
órdenes y la interfaz gráfica, herramientas como xEdit y DynDOLOD, gráficos, solución
de problemas), cómo funciona Eidos por dentro, y por qué existe. Política de
seguridad y cómo informar de una vulnerabilidad: [SECURITY.md](../../../SECURITY.md).

## Idioma

Las páginas que un jugador necesita están traducidas. El inglés es la referencia:
cuando una traducción no concuerda con él, el archivo en inglés es el que tiene
razón.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

Todo lo demás está en inglés. `docs/internals/` y `docs/project/` los leen personas
que además leen el Rust, `CHANGELOG.md` se genera, y `SECURITY.md` tampoco está
traducido. Traducir los dos primeros añadiría más de 19.000 palabras que mantener
al día para lectores que no las necesitan.

Cada traducción lleva el hash del archivo en inglés del que se hizo, y CI falla
mientras ese hash ya no coincida con el inglés; consulta
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Entonces la traducción se
actualiza y se vuelve a sellar con `scripts/i18n-check.sh --fix`. Una traducción
desactualizada se actualiza, no se elimina.

Un idioma es un solo directorio. `docs/i18n/<lang>/` refleja la raíz del
repositorio, así que `docs/i18n/de/docs/guide/install.md` es el
`docs/guide/install.md` en alemán. Por eso un enlace entre dos páginas traducidas es
la misma cadena que el enlace entre sus originales en inglés, y por eso retirar un
idioma es un solo `rm -r`.

## Política de firma de código

Cada archivo de una versión lo firma la organización Project Colony en CI, nunca en
la máquina de un desarrollador. Junto a cada archivo, en la página de la versión:

| Archivo | Qué es |
|---|---|
| `<asset>.sig` | una firma ed25519 del archivo, hecha con la clave de publicación de la organización |
| `<asset>.meta` | tres líneas que vinculan el archivo a su nombre, su sha256 y la versión publicada |
| `<asset>.meta.sig` | una firma ed25519 del `.meta` |

La clave privada es un secreto de la organización, que solo usa el
[flujo de trabajo sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
compartido, en un trabajo que no compila nada; los trabajos que compilan Eidos nunca
la ven. Colony comprueba los tres archivos antes de instalar o actualizar Eidos, y
rechaza una versión más antigua que la instalada. Para comprobar tú mismo una descarga
con OpenSSL 3 (las mismas órdenes sirven para cualquier archivo):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

Cómo se compilan, firman y publican las versiones:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Privacidad

Eidos no envía telemetría, ni analíticas, ni informes de fallos. Solo se conecta a los
servidores de esta tabla, y solo para la acción que allí se indica. Offline mode
(Preferences, Nexus) detiene toda conexión con Nexus y la comprobación diaria de
versiones.

| Datos | Guardados o enviados | Dónde, y por qué |
|---|---|---|
| Preferencias, lista de instancias, definiciones de juegos y de extensiones | guardados | `~/.config/Colony/Eidos/` |
| Instancias, runtimes de herramientas, registros de sesión | guardados | `~/.local/share/Colony/Eidos/`, o la carpeta que elegiste para una instancia portátil |
| Masterlists de LOOT | guardadas | `~/.cache/Colony/Eidos/loot/` |
| Comprobación de versiones | enviada | Al arrancar, como mucho una vez al día, una petición a `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos lee solo la etiqueta de la versión y no descarga nada. Está activada por defecto y se desactiva en Preferences, y se omite en Offline mode y para una copia instalada bajo `/usr` fuera de `/usr/local`, que actualiza un gestor de paquetes. [Declaración de privacidad de GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Inicio de sesión en Nexus | enviado | Iniciar sesión abre `users.nexusmods.com` en tu navegador, que devuelve el resultado a un receptor que Eidos abre en `127.0.0.1` solo para ese inicio de sesión. Eidos lo canjea después por tokens en `users.nexusmods.com/oauth/token` y obtiene las claves públicas con las que Nexus los firma. [Política de privacidad de Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Credenciales de Nexus | guardadas | Los tokens de acceso y de renovación, en `~/.config/Colony/Eidos/nexus.ini` con modo 0600. Una línea `api_key=` escrita allí por un Eidos más antiguo se conserva pero nunca se lee; bórrala si quieres. |
| Peticiones a Nexus | enviadas | `api.nexusmods.com`, cuando usas una función de Nexus: tu nombre de cuenta y tu estado premium, los detalles de mods y archivos, las comprobaciones de actualizaciones, las colecciones y los enlaces de descarga. Tus respaldos y tus mods seguidos se leen en cada comprobación de actualizaciones y se escriben cuando pulsas Endorse o Track. Los requisitos de los mods, las descripciones y los registros de cambios, y la búsqueda de colecciones, van a `api.nexusmods.com/v2/graphql`. Cada petición lleva tu token de acceso y nombra a Eidos y su versión. |
| Descargas de Nexus | enviadas | Los archivos que pides, desde el servidor CDN de Nexus que indica el enlace de descarga, guardados en la carpeta de descargas de la instancia con un `.meta` en formato MO2 junto a cada uno. |
| Enlaces `nxm://` | guardados | `eidos nxm --register`, que `install.sh` ejecuta por ti, escribe `~/.local/share/applications/eidos-nxm.desktop` y lo convierte en el manejador de `nxm://` mediante `xdg-mime`, para que el botón Mod Manager Download del sitio entregue su enlace a Eidos. El paquete de Arch instala la misma entrada para todo el sistema. |
| Masterlists de LOOT | enviadas | `raw.githubusercontent.com/loot/...`, en la primera ordenación con LOOT de un juego y cada vez que pides una actualización. No se envía nada tuyo. [Declaración de privacidad de GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtimes de herramientas | enviados | Solo cuando instalas los prerrequisitos de una herramienta, y solo para lo que el paquete de versión no traía (el archivo único `eidos-linux` y una compilación desde el código fuente no traen nada): el runtime de .NET 10 desde `builds.dotnet.microsoft.com` y los instaladores de Visual C++ y .NET 8 desde `download.visualstudio.microsoft.com`, cada uno rechazado salvo que coincida con el SHA-256 integrado en Eidos; el SDK de .NET 10, siempre desde `builds.dotnet.microsoft.com` y comprobado del mismo modo; y `winetricks` para los demás prerrequisitos (.NET Framework 4.8, XACT, .NET 6 y 7), que descarga lo que instala desde Microsoft y los demás servidores que usa winetricks. [Declaración de privacidad de Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Registros de sesión | guardados | `~/.local/share/Colony/Eidos/logs/`, modo 0600: diez sesiones por instancia, borradas pasados 30 días. Tu carpeta personal se escribe como `~` y tu nombre de usuario como `<user>`, así que un registro se puede pegar tal cual en una incidencia. Nunca salen de tu máquina salvo que los envíes tú. |
| Registros de ejecución | guardados | `<instance>/logs/sessions/`: por cada ejecución del juego o de una herramienta, el perfil, los mods activados con sus versiones y sus ids de archivo de Nexus, el orden de plugins, una suma de comprobación de cada INI del perfil, las versiones del juego y de Eidos, el código de salida y las partidas que creó la ejecución. Se conservan los 50 más recientes, `eidos sessions` los lee, y `eidos pack` los deja fuera. Se quedan en tu máquina. |
| Informes de actualización | guardados | `<instance>/logs/updates/<mod>.json`: lo que cambió en la carpeta de ese mod la última actualización o Replace. `eidos pack` los deja fuera. |
| Accesos directos | guardados | `install.sh` escribe `eidos.desktop` y sus iconos bajo `~/.local/share`; un acceso directo de herramienta o de perfil que crees va a `~/.local/share/applications`. |
| Notificaciones de escritorio | ninguno de los dos | Desactivadas por defecto. Cuando están activadas, van al servicio de notificaciones de tu escritorio mediante D-Bus (`org.freedesktop.Notifications`) y nombran el trabajo que terminó. |
| Enlaces que abres | ninguno de los dos | Las páginas de mods y otros enlaces se abren en tu propio navegador mediante `xdg-open`. |
| Extensiones | ninguno de los dos | Las extensiones que instalas se ejecutan como programas propios en tu máquina; lo que hagan es cosa suya. |

## Licencia

El código original de Eidos está bajo GPL-3.0-or-later; consulta
[LICENSE](../../../LICENSE). Estas partes están bajo GPL-3.0 sin la opción «or
later»:

- las traducciones de scripts OMOD de `eidos-install`; consulta sus
  [créditos originales](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- el programa auxiliar `eidos-nif-preview`, construido sobre la biblioteca nifly
  incluida; consulta las [licencias y los créditos de sus componentes](../../../crates/eidos-nif-preview/UPSTREAM.md);
- los crates `esplugin` y `libloadorder` que enlaza el código del orden de carga,
  que declaran `GPL-3.0`.

Los demás crates de Rust de los binarios conservan sus propias licencias (MIT,
Apache-2.0, BSD y similares). El paquete de la versión incluye sus avisos en
`licenses/crates/`, con un índice de cada crate y su licencia.

Los binarios incluyen además ocho DLL redistribuibles auténticas de Microsoft
DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` y `d3dx11_43`, de 32 y 64
bits), guardadas comprimidas con gzip (7,9 MB en lugar de 17,9 MB), que Eidos
descomprime y despliega en el prefijo del juego cuando un mod o una herramienta las
necesita. Siguen bajo los términos de redistribución de Microsoft; consulta su
[procedencia](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

El paquete de la versión trae además, bajo `runtimes/`, los instaladores de Visual C++
2015-2022 de Microsoft, el instalador de .NET 8 Desktop Runtime y el zip del runtime
de .NET 10, tal como los publica Microsoft. Sus términos están en
`licenses/microsoft-runtimes/` (`vc_redist.txt`, `dotnet.txt`).
