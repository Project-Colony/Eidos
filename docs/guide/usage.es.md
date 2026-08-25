<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# Usar Eidos

El manual práctico: la línea de órdenes, la interfaz gráfica, la opción de
lanzamiento de Steam, la compilación desde el código fuente y el script de prueba
de concepto. Para qué hacer cuando algo parece ir mal, ver
[troubleshooting.es.md](troubleshooting.es.md).

## Usarlo (CLI)

```sh
eidos games                       # juegos compatibles instalados aquí (como la lista de MO2)
eidos init skyrimse               # crear una instancia de modding
# ...deja cada mod como una carpeta en <instance>/mods/ (la instancia global vive
#    en ~/.local/share/eidos/skyrimse; `eidos init` imprime la tuya)...
eidos install skyrimse mod.7z     # o instala un archivo descargado (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adoptar el orden y el estado de plugins de un perfil MO2 existente
eidos sort skyrimse               # ordenar la carga de plugins con LOOT
eidos play skyrimse               # mostrar qué se montaría
eidos play skyrimse -- <command>  # ejecutar <command> con los mods montados sobre el juego
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` y `eidos export`
completan el conjunto; ejecuta `eidos` sin argumentos para la lista completa.

### Instancias: globales y portátiles

Toda orden de arriba se dirige a una instancia. `skyrimse` nombra la **global** -
guardada de forma centralizada en `~/.local/share/eidos/skyrimse`, gestionada por
Eidos. El otro tipo es **portátil**: una carpeta autónoma donde tú quieras (un
segundo disco, una partición de juegos), movible y aislada, exactamente como las
instancias portátiles de MO2. Allí donde una orden acepta un identificador de
juego acepta también la carpeta de una instancia portátil:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # crear ahí una instancia portátil
eidos install /mnt/games/EidosSkyrim mod.7z  # toda orden acepta la carpeta
eidos play /mnt/games/EidosSkyrim -- %command%
```

La carpeta se describe a sí misma (su `eidos-instance.ini` nombra el juego), así
que no hace falta nada más - y `EIDOS_INSTANCE=<folder>` en el entorno redirige un
identificador de juego a esa carpeta, lo que resulta cómodo en las opciones de
lanzamiento de Steam. Las instancias portátiles que hayas creado o abierto se
recuerdan (la más reciente primero) en `~/.config/Colony/Eidos/instances.ini`; la
pantalla de bienvenida de la interfaz las lista para abrirlas con un clic, el
lanzamiento desde Steam aterriza en la última que jugaste y el manejador `nxm://`
descarga en ella. Dos salvedades que conviene conocer: mover una carpeta portátil
conserva todo salvo las entradas de herramientas que registraste con rutas
absolutas a la ubicación anterior (vuelve a añadirlas), y la caché compartida de
runtimes (`~/.local/share/Colony/Eidos/runtimes/`) se queda deliberadamente global
a la máquina - un host .NET de 78 MB no va por instancia.

Eidos guarda sus propios archivos bajo `Colony/Eidos`, la disposición que usa todo
programa de la familia Colony: `~/.config/Colony/Eidos/` para lo que elegiste
(preferencias, tu sesión de Nexus, tu lista de instancias, las definiciones de
juegos y extensiones que escribiste), `~/.local/state/Colony/Eidos/logs/` para los
registros de sesión, y `~/.local/share/Colony/Eidos/` para lo que Eidos descargó.
Un Eidos más antiguo los guardaba en `~/.config/eidos/` y `~/.local/state/eidos/`;
el primer lanzamiento tras actualizar los **copia** y lo dice en el registro. Los
directorios antiguos quedan exactamente como estaban - no se borra nada, así que
una mala actualización no puede costarte un inicio de sesión - y puedes
eliminarlos tú mismo cuando estés conforme.

Tus mods no forman parte de eso. Una instancia global sigue viviendo en
`~/.local/share/eidos/<game>/`, y una portátil donde la pusieras, porque esas
rutas están escritas en tu lista de instancias y quizá en una opción de
lanzamiento de Steam: moverlas rompería un enlace del que Eidos no controla los
dos extremos.

Hay un sitio que se rechaza sin más: **dentro de la carpeta de instalación de un
juego** (el reflejo del veterano de MO2). Ese árbol pertenece a Steam - una
actualización, un «verificar integridad» o una desinstalación pueden reescribirlo
o borrarlo, llevándose por delante toda tu configuración - y Eidos monta sobre la
raíz del juego, así que una instancia ahí dentro quedaría dentro de su propio
destino de montaje. El asistente, `eidos init` y `eidos play` dicen que no; pon la
carpeta AL LADO del juego (una hermana en el mismo disco te da la misma
comodidad).

`play` monta los mods de la instancia sobre el propio directorio `Data` del juego
(mediante un bind-stash, para que el demonio siga leyendo los archivos intactos)
dentro de un espacio de nombres privado, y luego ejecuta la orden a través de esa
vista. Las escrituras (partidas guardadas, configuraciones regeneradas) aterrizan
en la capa `overwrite/` de la instancia; la instalación del juego y todas las
fuentes de los mods quedan intactas byte a byte.

### No hace falta ningún paso privilegiado

Eidos funciona por completo sin root. Monta en un espacio de nombres privado de
usuario + montaje, así que no hay ayudante setuid, ni demonio, ni nada que
conceder.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` es **opcional** y controla
exactamente una cosa: el passthrough FUSE del núcleo, que está desactivado por
defecto porque rompe el juego (abajo). Con esa capacidad Eidos toma un espacio de
nombres de montaje simple en vez de uno de usuario; los mods se despliegan igual
de un modo u otro.


Por qué desapareció el viejo consejo de `setcap` - y por qué el passthrough FUSE
se entrega desactivado - se explica en
[troubleshooting.es.md](troubleshooting.es.md#por-qué-el-passthrough-está-desactivado-por-defecto).

## GUI

```sh
cargo run -p eidos-gui
```

Un asistente de primer lanzamiento al estilo de MO2 con el aspecto pergamino /
burdeos de Colony: bienvenida -> tipo de instancia (portátil / global) -> juego ->
nombre y ubicación -> resumen -> crear -> pantalla principal. La pantalla de
bienvenida lista además toda instancia existente conocida (global y portátil, la
más reciente primero) para abrirla con un clic - hace también de conmutador de
instancias - y apuntar el asistente a una carpeta que ya contiene una instancia la
ADOPTA tal cual en vez de crear encima (rechazándola sin más si la carpeta
pertenece a otro juego).

La ventana principal de dos paneles también está hecha: un selector de perfiles
(cambiar, o crear uno nuevo copiando el actual), una lista de mods que filtras,
seleccionas, reordenas, agrupas con separadores, acotas por categoría y sobre la
que haces clic derecho para las acciones, más las pestañas Data / Plugins /
Conflicts / Overwrite / Saves / Downloads / Diagnostics y un botón Run con un
selector de destino de ejecución.

Reordenar no es sólo enviar arriba o abajo: los movimientos dirigidos de MO2
también están aquí - enviar por encima del primer mod en conflicto, por debajo del
último, a una prioridad explícita, o dentro del grupo de un separador. Todos pasan
por un único ayudante de movimiento compartido, así que el error de uno que sale
de quitar filas antes de reinsertarlas existe en un sitio en vez de en cinco.

### Columnas, ordenación y agrupación

La lista dibuja cuatro columnas de serie y ofrece ocho: Category, Content,
Version, Author, Installed, Nexus id, Game, Flags. Márcalas en el menú View. El
valor por defecto no son las ocho a propósito - una lista con todas las columnas
visibles no deja sitio para el NOMBRE, que es la columna que realmente estás
leyendo.

Haz clic en cualquier encabezado para ordenar por él. Otro clic invierte, y un
tercero vuelve al **orden de carga**, lo que importa más de lo que parece: el
orden de carga es el único orden en el que se puede arrastrar la lista, porque un
hueco de inserción se dirige a la lista real mientras que una fila ordenada está
en otro sitio completamente distinto. Mientras hay una ordenación activa, las
franjas de inserción no se dibujan y un arrastre se rechaza en vez de aterrizar
donde nadie apuntaba - lo mismo que hace MO2, y por la misma razón. El menú View
lo dice y ofrece la vuelta atrás.

El menú View puede también **agrupar** la lista entera, por categoría o por origen
(desde Nexus, o instalado a mano). Los encabezados de grupo no son separadores: no
hay nada detrás de ellos que renombrar, colorear o mover, se pliegan, y el
recuento se queda en el encabezado al plegarse. Los separadores desaparecen de la
lista bajo una ordenación o una agrupación - un separador encabeza las filas que
le siguen en el orden de carga, y ambas las han movido.

### Ratón y teclado

Doble clic en un mod para Information, Ctrl+doble clic para su carpeta,
Shift+doble clic para su página de Nexus. Ctrl+F pone el cursor en la caja de
filtro. Teclear una letra salta al siguiente mod que empiece por ella, y pulsarla
otra vez recorre el resto en vez de quedarse en el primero. Ninguno de ellos puede
aterrizar en una fila que oculten el filtro, un separador plegado o un grupo
plegado - mover un resaltado que no puedes ver es como el siguiente Space acaba
activando un mod que no estabas mirando.

«Collapse others» en el menú de un separador pliega todos los grupos salvo ése.
Durante un arrastre, detenerse sobre un grupo plegado lo abre, de modo que se
puede soltar un mod dentro sin abandonar antes el arrastre - detenerse, no pasar
rozando.

### Lo que la lista te dice de un mod

Dos marcas informativas, ambas un glifo con la explicación al pasar el ratón. **No
valid game data** significa que nada en la raíz del mod parece algo que este juego
cargue; puede que haya que subir sus carpetas un nivel, o puede que no sea un mod
para este juego. **Another game** significa que el propio `meta.ini` del mod
nombra otro. Ninguna bloquea nada - el mod se despliega igual - y «Mark as valid»
en el menú de la fila silencia cualquiera de las dos, mediante la propia clave
`validated=` de MO2, de modo que un mod por el que has respondido en un gestor
llega callado al otro.

La comprobación de la disposición es deliberadamente generosa: un árbol `Root/`
cuenta, una carpeta ilegible cuenta, una vacía cuenta. Un aviso equivocado en una
lista de quinientas filas es peor que uno ausente.

### Hacer copia de un mod antes de tocarlo

«Back up this mod» copia su carpeta aparte como `<name>_backup` (luego `_backup2`,
y así - una copia nunca reemplaza a la anterior). La copia es **inerte**: no es un
mod, su casilla no hace nada y no aporta nada a la vista combinada, porque
marcarla desplegaría dos copias de un mismo mod una sobre otra. «Restore this
backup over the mod» la devuelve a su sitio, en dos clics; el contenido actual se
aparta primero y sólo se descarta una vez que la copia ha salido bien.

**Data** es un árbol real de la vista combinada, expandido un nivel cada vez, de
modo que abrir un nodo cuesta una lectura de directorio por cada capa que lo tiene
en vez de un recorrido recursivo de todos los mods activos. Lo responde la MISMA
pila de capas desde la que sirve el montaje, así que se respetan los whiteouts y
los archivos ocultos y la pestaña no puede contradecir lo que verá el juego.
Fíltralo por nombre, acótalo sólo a los archivos en disputa, aclara qué está dónde
con las columnas Size y Modified, y usa Reveal en cualquier fila para abrirla en
un gestor de archivos. **Plugins** es el orden de carga de ESP/ESM/ESL (activar,
reordenar a mano, u ordenar con LOOT y leer el informe posterior, cuyos enlaces de
consejo se abren en tu navegador). **Conflicts** explica los ganadores y
perdedores archivo por archivo. **Overwrite** convierte lo que el juego escribió
en un mod real en un solo paso. **Saves** analiza la cabecera de cada partida
guardada - personaje, nivel, lugar, tiempo jugado - y compara la lista de plugins
grabada en ella con la tuya actual, con un botón que activa los mods que necesita,
porque nombrarlos y dejarte a ti el resto es la mitad aburrida.

«Information...» abre un diálogo por mod: general, conflictos, árbol de archivos,
ajustes INI, notas. Desde el árbol de archivos (y desde el árbol Data) cualquier
archivo puede **ocultarse** - se renombra a `<name>.mohidden`, lo que lo saca de
la vista virtual sin borrarlo, de modo que las tres mallas sueltas de un mod se
pueden suprimir sin tocar prioridades. El árbol de archivos hace también las
operaciones de archivo corrientes: nueva carpeta, renombrar, borrar, abrir. Todas
pasan por un único resolutor que rechaza cualquier cosa que no sea una ruta simple
dentro de ese mod - nada de `..`, ninguna ruta absoluta y ningún componente que
sea un enlace simbólico, ya que seguir uno pondría un borrado completamente fuera
de la carpeta del mod. Renombrar reemplaza sólo el último componente, así que
nunca puede convertirse en un movimiento, y rechaza un nombre ya ocupado en vez de
reemplazar ese archivo en silencio. Borrar cuesta dos clics; es la única acción
aquí que otro clic no puede deshacer.

**View** en cualquier fila del árbol de archivos o del árbol Data previsualiza el
archivo: imágenes y texto. DDS o NIF no - ésos necesitan un decodificador de
bloques y un renderizador que este árbol no tiene - pero lo dicen en vez de
mostrar una caja vacía, y apuntan a Reveal. El texto se lee hasta 64 KB y avisa de
dónde se detuvo, porque una previsualización es un vistazo y un registro de
Papyrus puede pesar cien megabytes. **INI Tweaks** lista los fragmentos que un mod
trae en su carpeta `INI Tweaks/`; los activados se fusionan en el INI del juego
del perfil al lanzar, por orden de prioridad, y se retiran de nuevo cuando se
capturan los INI de la ejecución - de lo contrario un ajuste se convierte en
silencio en una opción y desactivarlo no haría nada.

Una descarga se puede **arrastrar desde la lista Downloads hasta una posición de
la lista de mods** para instalarla con esa prioridad, y los archivos o carpetas
soltados sobre la ventana desde un gestor de archivos también se instalan (esa
mitad necesita una sesión X11 o XWayland - winit implementa el soltado de archivos
sólo para X11). Las propias descargas se pueden pausar y reanudar: pausar detiene
la transferencia y conserva lo parcial, y Resume vuelve a resolver un enlace nuevo
y continúa desde donde se detuvo.

La pestaña Downloads es una **biblioteca** de archivos, no una cola de
transferencias. Fíltrala por nombre (también por el nombre legible del mod, así
«skyui» encuentra `SkyUI_5_2_SE-12604-5-2SE.7z`), ordena por más reciente, nombre,
tamaño o estado, y **oculta** un archivo con el que hayas terminado - lo que
conserva el fichero y sólo quita la fila, así que guardar un libro no es quemarlo.
«Show hidden» los trae de vuelta, y el mismo botón los vuelve a mostrar. «Remove N
installed» borra los archivos de los mods que ya has instalado, en dos clics, y
sólo los que están **en pantalla**: el filtro es tu forma de decir a cuáles te
referías.

### Colecciones de Nexus

Pega un enlace de colección - o haz clic en uno en el sitio - y Eidos lista los
miembros de la revisión, cada uno cruzado con esta instancia: instalado,
descargado o ausente. **Lee** una colección; no la instala, y el panel lo dice.
Cuatro cosas hacen que aquí un instalador sea deshonesto y no sólo difícil: los
miembros son archivos corrientes de Nexus que necesitan una clave por archivo que
sólo una cuenta premium puede acuñar fuera del propio botón del sitio; una
instalación completa son tres llamadas a la API por miembro contra un presupuesto
que este cliente se niega a gastar de más; las fases, las reglas y las respuestas
FOMOD reproducidas del manifiesto no se pudieron verificar contra una colección
Bethesda real publicada, y adivinar produce un orden de carga que parece correcto
y no lo es. Leer cuesta una petición y es exacto.

Una colección sólo se puede leer contra **su propio juego**. Abre una colección de
Skyrim con una instancia de Fallout 4 cargada y se rechaza por nombre en vez de
cruzar los miembros con la lista de mods equivocada, donde cada «instalado» y cada
«ausente» serían ruido con forma de respuesta.

### Modo sin conexión

**Settings -> Nexus -> Offline** hace que Eidos no contacte con Nexus en absoluto.
Las comprobaciones de actualización, el inicio de sesión, las descargas y las
colecciones lo dicen en vez de fallar con un error de conexión. Está desactivado
salvo que lo actives - un archivo de configuración escrito por un Eidos más
antiguo no tiene esa clave, y leer una ausente como «activada» cortaría la red a
todo el que actualice.

**Preferred servers** ordena los nodos CDN que prefiere una descarga, el mejor
primero. Sólo a una cuenta premium se le ofrece más de un espejo entre los que
elegir, así que para todos los demás elige Nexus y esto no cambia nada. Es una
ordenación, no un filtro: si hoy no se ofrece ninguno de los que nombraste, la
descarga se hace igual, desde el nodo que Nexus ofreciera primero.

**Categories** son editables, no sólo se muestran: asígnalas a un mod o a toda una
selección, edita el propio catálogo desde el mismo diálogo, y trae de Nexus la
lista oficial de categorías del juego. Los dos archivos de catálogo son los
propios de MO2 (`categories.dat` y `nexuscatmap.dat`), así que una instancia
compartida mantiene un solo catálogo.

**View -> INI editor** edita los INI del juego del perfil - la copia que persiste,
no la enterrada en el prefijo de Proton que se sobrescribe en cada lanzamiento.
**View -> Log** lee los registros de sesión. **View -> Extensions** lista tus
propias extensiones; ver [extensions.es.md](extensions.es.md).

La instalación acepta todo: las rutas Simple y FOMOD, más los paquetes **BAIN** de
Wrye Bash (marca los subpaquetes, que se fusionan por orden) y un selector
**manual** que muestra el árbol del archivo y te deja señalar la raíz de datos
cuando ninguna heurística reconoce la disposición. No se rechaza ningún archivo.

**Diagnostics** ejecuta comprobaciones de salud en vivo: la capacidad de
lanzamiento ante todo, los masters ausentes (el predictor de fallos más fiable que
existe), los archivos que ningún plugin activo cargará, si la lista de mods sigue
coincidiendo con la carpeta de mods y - tras una ejecución - qué dice el propio
registro del script extender sobre cada una de sus DLL de plugin, lo que convierte
«¿se cargaron mis plugins de SKSE?» de una inferencia en una evidencia.

Para lanzar el juego a través de la interfaz gráfica, pon la opción de lanzamiento
de Steam del juego con la ruta absoluta del binario (Steam no ve `~/.cargo/bin` en
el PATH):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos se abre en la instancia de ese juego - la última que usaste, así que una
instancia portátil se vuelve a encontrar igual que la global; pulsa Run para
lanzarlo a través de la vista combinada. (El botón Run muestra esta línea exacta,
con la ruta real del binario en ejecución, si lo pulsas fuera de Steam.)

El `%command%` de Steam para los títulos de Bethesda suele apuntar a
`<Game>Launcher.exe`. Eidos nunca lo ejecuta: el launcher es una aplicación de
opciones aparte que vuelve a escanear `Data` y reescribe `plugins.txt`,
deshaciendo el orden de carga que se acababa de desplegar. Pone en su lugar el
cargador del script extender si hay uno instalado, y el binario del juego en caso
contrario, y lo dice cuando tiene que recurrir a esa alternativa - un juego que
arranca con todos los mods de SKSE inertes es peor que uno que no arranca.

Instrucciones más antiguas forzaban aquí `WINEDLLOVERRIDES="d3dcompiler_47=n"`. Ya
no hace falta y nunca fue del todo correcto: una anulación a *native* sólo ayuda
si un `d3dcompiler_47.dll` auténtico ya está en el prefijo. Ahora Eidos escanea
las importaciones de DLL de los mods activos, despliega él mismo la DLL real de
Microsoft y sólo entonces fija la anulación.

## Probar la prueba de concepto

No hace falta ningún juego. Demuestra la unión + copy-on-write + cero
modificaciones + alcance por espacio de nombres usando sólo OverlayFS sin
privilegios en un espacio de nombres de usuario (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Herramientas

xEdit, BodySlide, DynDOLOD y compañía se ejecutan a través de la vista combinada
dentro del prefijo de Proton del juego:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # lo que necesitan las herramientas registradas, y su estado
eidos prereqs skyrimse --install  # obtener lo que falte
```

Una cosa que conviene saber antes de nombrar una herramienta: **el título decide
qué DLL de runtime le provisiona Eidos** - `BodySlide` recibe sus bibliotecas
DirectX, `BS` no recibe nada. En la interfaz gráfica el diálogo Executables
muestra el estado real de cada requisito bajo el campo, y los que faltan son
botones.

La tabla, los tres niveles de requisitos, por qué DynDOLOD necesita un runtime
.NET que winetricks no puede instalar y por qué una herramienta instalada como mod
se lanza desde la ruta combinada y no desde su propia carpeta están en
[tools.es.md](tools.es.md).

La compilación desde el código fuente y la disposición del repositorio están en
[../internals/contributing.md](../internals/contributing.md).

## Extensiones

Eidos se puede extender sin recompilarlo: un manifiesto TOML en
`~/.config/Colony/Eidos/addons/` añade una herramienta a la lista Extensions o una
comprobación a la pestaña Health. No se carga nada dentro de Eidos - una extensión
es un programa que ejecuta. Ver [extensions.es.md](extensions.es.md).
