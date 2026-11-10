<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Herramientas: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Una herramienta ejecutada a través de Eidos ve la vista combinada, dentro del
prefijo de Wine del juego (el prefijo de Proton de Steam, o el que configuró
Heroic). Lee lo que leerá el juego (cada mod activado, en orden de prioridad) y lo
que escriba aterriza en el Overwrite, donde un clic lo convierte en un mod de
verdad. Una herramienta `runner=native` se ejecuta sin Proton; consulta
[más abajo](#variables-de-entorno-y-herramientas-sin-proton).

## Las que Eidos encuentra por sí solo

Algunas herramientas tienen nombres lo bastante únicos como para encontrarlas en
vez de declararlas, y xEdit es el caso evidente: `FO4Edit.exe` para Fallout 4,
`SSEEdit.exe` para Skyrim SE, `TES5Edit.exe` para el original, y así
sucesivamente. Cada uno viene con una entrada QuickAutoClean, que limpia las
ediciones sucias de las que avisa LOOT. `PGPatcher.exe` se encuentra del mismo
modo en todos los juegos de Bethesda salvo Morrowind, y llega con un argumento sin
el cual no puede funcionar (consulta
[más abajo](#por-qué-pgpatcher-necesita---ignore-mo2vfscheck)). Eidos las busca,
por nombre de archivo, en:

- `Overwrite/Root/`, la carpeta del juego desde la que se ejecuta la instancia (la
  instalación de la tienda, o su copia del juego) y los árboles `Root/` de los
  mods activados;
- el `mods/` de esta instancia, que es donde los usuarios de MO2 instalan
  herramientas;
- la carpeta de herramientas que fijas en Preferences (General -> Tools), para un
  directorio compartido entre instancias, como `/mnt/Games/Tools`.

Los generadores habituales se encuentran del mismo modo, cada uno en los juegos que admite: Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen y Synthesis. También BethINI Pie (`Bethini.exe`), en los mismos juegos que PGPatcher; consulta [BethINI Pie](#bethini-pie). En Stardew Valley, SMAPI (`StardewModdingAPI.exe`) se encuentra del mismo modo. Es además el cargador del script extender de ese juego, así que arranca el juego a través de la tienda, como se describe en [Llegar a una herramienta](#llegar-a-una-herramienta-ocultar-fijar-y-un-acceso-directo-de-escritorio).

La lista es por juego, así que a una instancia de Skyrim nunca se le ofrece el
editor de Fallout. La búsqueda se detiene cuatro niveles abajo, porque un conjunto
de mods son cientos de miles de archivos y esto se ejecuta cada vez que se
construye la lista de herramientas, y no sigue enlaces simbólicos. Una herramienta
encontrada así se configura exactamente igual que una que escribas tú: sus
prerrequisitos, sus argumentos (el `--ignore-mo2vfscheck` de PGPatcher) y su mod de
captura salen de su título, por las reglas de más abajo.

Si una herramienta está en otro sitio, o quieres argumentos distintos, añádela a
mano. Una entrada de usuario con el mismo título anula cualquier cosa encontrada
automáticamente.

## Limpiar un plugin que LOOT marca como sucio

Tras una ordenación de LOOT, la pestaña Diagnostics lista los plugins que LOOT
marca como sucios, en los juegos que tienen a la vez un xEdit y datos de
limpieza de LOOT (Skyrim LE, SE y VR, Enderal SE, Fallout 3, New Vegas, Fallout
4 y 4 VR, Oblivion, Starfield). Cuando Eidos encuentra xEdit, registrado en
Executables o en cualquier sitio de la instancia donde busca herramientas, la
tarjeta tiene Clean all y cada plugin sucio tiene Clean with QuickAutoClean en
el menú de su fila; si no, la tarjeta te indica que añadas xEdit en Executables.
Eidos no descarga xEdit. Ejecuta el modo QuickAutoClean de xEdit sobre un plugin
cada vez, a través de la vista combinada, con `-qac -autoexit -autoload`. xEdit
guarda el plugin limpio encima del que cargó, lo que en la vista combinada es un
archivo nuevo en el Overwrite; Eidos lo mueve a un mod llamado `Cleaned
plugins`, que se crea activado al final de la lista, donde gana todos los
conflictos. El plugin en su propio mod, o en la carpeta Data del juego, nunca se
escribe. Para deshacer una limpieza, desactiva `Cleaned plugins` u oculta un
archivo dentro de él. Un plugin que está en el Overwrite no se limpia, porque
xEdit reescribiría ese mismo archivo; convierte primero el Overwrite en un mod.

Las copias de seguridad, la caché y el registro de xEdit van a
`<instance>/logs/qac-<time>/`, fuera de la vista combinada, y la línea de estado
muestra las cifras de ese registro. Eidos borra esa carpeta tras una limpieza
que salió bien y la conserva tras una que falló. Eidos anota en `Cleaned
plugins/meta.ini` el tamaño y la fecha del plugin del que salió cada copia.
Cuando ese plugin cambia, la pestaña Diagnostics muestra **Out-of-date copy of
<plugin>**, con un botón que oculta la copia antigua.

## Añadir una

En la interfaz: Tools en la barra de menús, o Executables en la barra de
herramientas, y luego Add. Desde la línea de órdenes:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # listar lo que está registrado
eidos tool skyrimse run BodySlide         # ejecutarlo a través de la vista combinada
eidos tool skyrimse run BodySlide --print # mostrar la orden sin ejecutarla
eidos tool skyrimse rm BodySlide          # quitar una entrada que añadiste
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # ejecutar un exe una vez, sin registrarlo
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` captura la salida de esa ejecución en `mods/<mod>`, sea cual sea el ajuste propio de la herramienta. `rm` quita solo las entradas que añadiste; una entrada por defecto de un juego se puede ocultar, no quitar.

Un exe ejecutado con `--exe` cuyo nombre de archivo es uno que Eidos conoce para el juego, sin distinguir mayúsculas, recibe el título de esa herramienta, y con él los prerrequisitos, argumentos y mod de captura de más abajo. `GenerateFNISforUsers.exe` es FNIS, así que la línea `--exe` de arriba pide `dotnet48` y captura en `FNIS Output`. Cualquier otro exe recibe como título su nombre de archivo. En la pestaña Data, una fila `.exe` tiene Add tool, que abre el diálogo Executables con la entrada rellenada, y Run, que ejecuta el archivo una vez como hace `--exe`. Ambos titulan el archivo del mismo modo.

Para una orden que no es una herramienta de Windows, `eidos shell skyrimse` abre tu
`$SHELL` (`/bin/sh` si no está definida) en la carpeta del juego con la vista
combinada montada. Lo que escriba la shell aterriza en el Overwrite, y al salir de
la shell se desmonta la vista. `eidos shell skyrimse -- <command>` ejecuta una sola
orden en su lugar. Ctrl+C en la shell interactiva detiene la orden que escribiste y
deja la shell abierta. La shell se ejecuta sin Proton; un programa de Windows pasa
por `eidos tool`.

En la ventana, Stop termina una ejecución de herramienta que se cuelga: púlsalo
dos veces, y Eidos termina los programas de la ejecución y luego trata sus
archivos como tras cualquier salida. Con Notify the desktop when a long job ends
activado (Preferences, General, Running a game), una ejecución de herramienta que
termina mientras la ventana está en segundo plano envía una notificación de
escritorio.

El script extender, el binario del juego y el lanzador se detectan
automáticamente; solo hay que registrar las herramientas adicionales. En una
instancia que se ejecuta desde una copia del juego se detectan en la copia;
consulta [Herramientas en una copia del juego](#herramientas-en-una-copia-del-juego).

### Apúntala al archivo real, esté donde esté

Registra el ejecutable donde de verdad está. Si la herramienta se instaló como
mod, eso es dentro de la carpeta del mod:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

Esa es la ruta de la instancia global. Para una instancia portátil la misma regla
se aplica bajo su propia carpeta, `<instance>/mods/...`. Una ruta absoluta como
esta es lo único que no sobrevive a mover después una carpeta portátil.

Eidos reescribe esa ruta a la combinada antes de lanzar, de modo que la
herramienta se ejecuta desde `<game>/Data/CalienteTools/BodySlide/` y ve ahí
también los archivos de todos los demás mods. BodySlide trae un directorio
`SliderSets` vacío, y cada cuerpo que puede construir viene de CBBE y de los mods
de ropa. Lanzado desde su propia carpeta de mod no encuentra nada y parece roto.

MO2 hace la misma reescritura, por la misma razón: su propio comentario nombra a
FNIS.

Una herramienta dentro de un mod desactivado no se puede reescribir, porque sus
archivos tampoco están en la vista. Eidos lo dice y la ejecuta desde su propia
carpeta.

## Herramientas de una lista de Mod Organizer 2

Una lista de Wabbajack u otra instancia de MO2 abierta como instancia de Eidos conserva su `ModOrganizer.ini`, y los ejecutables configurados ahí se pueden copiar a `tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # muestra lo que se añadiría
eidos tool /path/to/list import-mo2 --write   # las añade
```

En la ventana, el diálogo Executables tiene Import from ModOrganizer.ini, con una casilla que se puede desmarcar en cada entrada. Add selected pone las entradas marcadas en la lista, y no se guarda nada hasta que pulses Save. Solo cuenta un `ModOrganizer.ini` que Eidos no escribió; para el que escribe Eidos, consulta [PGPatcher y PGLite](#pgpatcher-y-pglite-en-modo-mod-organizer-2).

Las rutas en `Z:` son rutas de Linux y se quedan como están. Una ruta en otra letra de unidad pasa a la carpeta de la lista cuando nombra esa carpeta (`D:\Lists\MyList\tools\SSEEdit.exe` pasa a ser `<list>/tools/SSEEdit.exe`), tanto en el ejecutable como en el directorio de trabajo y en los argumentos. El cargador del script extender, el lanzador y el binario del juego pasan a ser nombres simples, así que se ejecutan desde la carpeta del juego sobre la que Eidos monta la vista, como las entradas integradas. El `hide` de MO2 pasa a ser Hide from picker, y `steamAppID` pasa a ser Steam AppID. Las entradas que apuntan a otro sitio (un programa del sistema de Windows, una herramienta fuera de la lista) se muestran como omitidas con el motivo, y un título que ya esté en `tools.ini` nunca se reemplaza.

Una entrada importada conserva el título y los argumentos de MO2, y Eidos no rellena nada a partir de su título: ni prerrequisitos, ni argumento extra, ni mod de captura. Defínelos en el diálogo Executables. A una entrada de PGPatcher de una lista hecha en Windows suele faltarle `--ignore-mo2vfscheck`; añádelo (consulta [más abajo](#por-qué-pgpatcher-necesita---ignore-mo2vfscheck)).

Una ruta importada que apunta a la carpeta `Stock Game` de la lista se traslada a la instalación de la tienda en el momento de la ejecución solo cuando la instancia se ejecuta desde esa copia; consulta la sección siguiente.

## Herramientas en una copia del juego

Cuando la instancia se ejecuta desde una copia del juego (la carpeta `Stock Game`
o `Game Root` de una lista, o una copia congelada; consulta
[Archivos del juego](usage.md#archivos-del-juego-la-instalación-de-la-tienda-o-una-copia)),
una herramienta ve los archivos de la copia, no los de la instalación de la tienda.
La vista combinada sigue montada sobre la instalación de la tienda: la vista raíz
tiene la copia en el fondo, y la vista Data lee el `Data` de la copia. Eidos busca
el script extender, el lanzador, el binario del juego y las herramientas conocidas
en la copia en lugar de en la instalación de la tienda.

Una ruta de herramienta, un argumento o un directorio de trabajo que apunta a la
copia se traslada a la instalación de la tienda antes de la ejecución. Puede
escribirse como ruta de Linux o como ruta `Z:` de Wine:
`-D:"Z:\...\Stock Game\Data"` pasa a ser la misma ruta bajo la instalación de la
tienda. Las entradas importadas del `ModOrganizer.ini` de una lista lo necesitan.
Cuando la instancia se ejecuta desde la instalación de la tienda no se traslada
nada, así que una entrada así lee la copia desnuda sin tus mods.

El `installed path` del juego en el registro del prefijo y el `gamePath` del
`ModOrganizer.ini` que escribe Eidos siguen nombrando la instalación de la tienda,
donde está la vista combinada. Si falta la carpeta de la copia, o no contiene
ningún binario del juego, la ejecución de la herramienta se detiene, como un
lanzamiento del juego, y el mensaje nombra `eidos stock-game <instance> store`
para volver a la instalación de la tienda.

`eidos stock-game <instance>` muestra qué archivos del juego ejecuta la instancia
y a qué copias podría cambiar. `use <folder>` y `store` los cambian, y
`freeze --yes` copia la instalación de la tienda en la carpeta `Stock Game` de la
instancia y se ejecuta desde ella. La tarjeta Game files de la pestaña
Diagnostics hace lo mismo; consulta
[Archivos del juego](usage.md#archivos-del-juego-la-instalación-de-la-tienda-o-una-copia).

## Enviar la salida de una herramienta a su propio mod

Un generador (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) escribe cientos de
archivos. Por defecto aterrizan en el Overwrite con todo lo demás. Fija Capture
output into en el editor de Executables y la salida de esta ejecución va a ese
mod en su lugar:

```
Executables -> (tu herramienta) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide y Synthesis llegan con un mod de captura ya definido: `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` y `Synthesis Output`. DynDOLOD, TexGen y xLODGen llegan sin él, porque escriben en la carpeta de salida que les indiques.

El mod se crea si no existe. Solo se mueven los archivos que produjo esta
ejecución; lo que ya estaba en el Overwrite se queda ahí, así que dos herramientas
con destino de captura no se roban la salida entre ellas. Una ejecución que no
escribió nada no deja atrás un mod vacío.

Eidos mueve los archivos después de la ejecución. MO2, en cambio, apunta la capa de
escritura al mod, lo que asciende ese mod a la máxima prioridad durante toda la
ejecución: cada conflicto en el que está se invierte, y se invierte de nuevo
después. Las escrituras también pasan directamente por los archivos del propio mod,
sin copy-up. La captura llega al mismo estado final sin ninguna de las dos cosas.

Si el mod de destino está desactivado, la salida se escribe igualmente pero el
juego no la verá, así que la herramienta regeneraría los mismos archivos en la
siguiente ejecución. Eidos avisa cuando es el caso.

### Cuando su salida está desactualizada

Eidos anota con qué mods, orden de plugins y perfil se generó la salida de una herramienta, el tamaño y la fecha del ejecutable de la herramienta, y la ruta y los argumentos de la entrada en `tools.ini`. Cuando uno de ellos cambia, la pestaña Diagnostics muestra **Generated output inputs changed**, y la tarjeta trae un botón **Run <tool> again** que ejecuta la herramienta con su configuración actual.

Para un generador cuya salida desactualizada rompe el juego, como Pandora o Nemesis, añade `check_before_play=1` a su entrada en `tools.ini`, o activa Ask before playing when its output is out of date para ella en el diálogo Executables. Run pregunta entonces primero: ejecutar la herramienta o jugar de todos modos. Play anyway se recuerda hasta que los mods, los plugins o el perfil vuelvan a cambiar, o hasta que se cierre la ventana. Eidos nunca inicia la herramienta por sí mismo. Con Start the game at once when a store opens Eidos activado, una pregunta así mantiene abierta la ventana en lugar de arrancar el juego.

## Los prerrequisitos y los argumentos salen del título

El título que le das a una herramienta decide qué prerrequisitos de runtime le
provisiona Eidos. La coincidencia es una subcadena del título, sin distinguir
mayúsculas.

| Si el título contiene | Eidos solicita |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| cualquier otra cosa | nada |

Así que una herramienta registrada como `BodySlide` recibe sus DLL de DirectX; el
mismo ejecutable registrado como `BS` no recibe nada y puede fallar al arrancar con
un error que no dice nada sobre DLL. Nombra las herramientas como el programa.

La lista está en `default_prereqs` (`crates/eidos-instance/src/tools.rs`), y el
campo `Prereqs` del diálogo Executables es editable: el título solo rellena el
valor por defecto.

El título decide los argumentos del mismo modo, para la única herramienta que
necesita uno que nadie podría adivinar: un título que contenga `pgpatcher` (o
`parallaxgen`, su nombre anterior) recibe `--ignore-mo2vfscheck`. Por qué, y por
qué no es opcional, está
[más abajo](#por-qué-pgpatcher-necesita---ignore-mo2vfscheck). El mod de captura
también sale del título (consulta
[más arriba](#enviar-la-salida-de-una-herramienta-a-su-propio-mod)).

### Tres clases de prerrequisito

El nivel 1 son las DLL empaquetadas: `d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`
y `d3dx11_43`. Eidos las lleva dentro de sus binarios y las copia en el prefijo al
lanzar. En un prefijo de 64 bits, el valor por defecto de Proton, la compilación de
64 bits va a `system32` y la de 32 bits a `syswow64`. Nada que hacer, sin red.
Cada compilación se guarda comprimida con gzip, así que los ocho archivos ocupan
7,9 MB de cada binario en lugar de 17,9 MB, y solo se descomprime la que se está
copiando (4,7 MB como máximo).

El nivel 2 son los instaladores: `vcrun2022`, `dotnet6`, `dotnet7`, `dotnet8`,
`dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` y `xact_x64`. Escriben
claves de registro, el GAC y hosts del CLR, así que no se pueden copiar como
archivos. Eidos instala tres de ellos sin winetricks: `vcrun2022` ejecuta
`VC_redist.x86.exe` y `VC_redist.x64.exe` de Microsoft, y `dotnet8` y
`dotnetdesktop8` ejecutan el instalador del .NET 8.0.12 Desktop Runtime, que
también incluye el runtime de .NET. El paquete de versión
(`eidos-linux-bundle.tar.gz`) trae estos instaladores en su carpeta `runtimes/`, e
`install.sh` los copia a `~/.local/share/Colony/Eidos/runtimes/` para el usuario
que lo ejecuta, también con `--system`. Eidos busca cada archivo primero ahí, y
luego en una carpeta `runtimes/` junto al binario `eidos` (un paquete
descomprimido ejecutado en el sitio). Sin ninguna de las dos copias (el archivo
único `eidos-linux`, una compilación desde el código fuente, u otro usuario de una
instalación para todo el sistema) Eidos los descarga de Microsoft
(`download.visualstudio.microsoft.com`) en `~/.local/share/Colony/Eidos/runtimes/`
y los conserva ahí. En ambos casos un archivo se rechaza salvo que su SHA-256
coincida con el integrado en Eidos, y se ejecuta sin ventana
(`/install /quiet /norestart`) en el prefijo del juego, con la versión de Proton o
Wine que ejecuta las herramientas del juego. Igual que winetricks, `vcrun2022`
pone primero las DLL de Visual C++ en `native,builtin` en los reemplazos de DLL
del prefijo. Después, con una versión de Proton, Eidos comprueba que el runtime de
C++ del prefijo no sea mitad de Microsoft y mitad de Proton. Si lo es, la
instalación falla: el mensaje nombra los pares de DLL que vienen de compilaciones
distintas y dice que instales `vcrun2022` con winetricks en su lugar, que
reemplaza `msvcp140.dll` por sí mismo. Solo se instala el .NET 8 de 64 bits; una
herramienta .NET 8 de 32 bits sigue necesitando `winetricks dotnetdesktop8`.

Los otros seis verbos pasan por el `winetricks` del sistema, que debe estar en el
`PATH`. Eidos lo ejecuta con el `wine` de la versión de Proton o Wine que ejecuta
las herramientas del juego, contra el prefijo del juego, lo que esquiva el
contenedor pressure-vessel de Steam y el desajuste protontricks + Proton-GE.
Algunos verbos de winetricks necesitan `cabextract`, y Eidos avisa cuando falta.
Una herramienta que declara un verbo de nivel 2 no instalado se lanza igualmente,
con un aviso que nombra el verbo y la orden para arreglarlo, porque puede que lo
hayas instalado de otra forma.

Un verbo de nivel 2 cuenta como hecho cuando el propio `winetricks.log` del prefijo
lo lista, lo haya ejecutado quien lo haya ejecutado (winetricks o protontricks), o
cuando Eidos lo instaló en ese mismo prefijo. Eidos registra sus propias
instalaciones en `prereqs.done` en la instancia, ligadas a la ruta y al inodo del
prefijo, de modo que un prefijo reconstruido vuelve a mostrar sus verbos como
pendientes. **Rebuild prefix**, una acción de Diagnostics para una copia de Steam,
renombra `compatdata/<appid>` a `<appid>.eidos-backup-<time>` y no borra nada;
Steam crea un prefijo nuevo la próxima vez que inicia el juego.

El nivel 3 son los runtimes, `dotnet10` y `dotnetsdk10`. Un runtime moderno de .NET
son 193 archivos que viven en su propio directorio y se encuentran a través de
`DOTNET_ROOT`. Nunca se registra y nunca se instala en el prefijo, así que ninguno
de los otros niveles puede transportarlo. Eidos lo guarda fuera de cualquier
instancia, porque 78 MB no es por juego ni por perfil: cada runtime se descomprime
en `~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/` (`dotnet10-10.0.10`,
`dotnetsdk10-10.0.403`). Eidos busca el zip del runtime con su nombre de descarga
en las mismas dos carpetas que los instaladores del nivel 2, y descomprime la copia
que trae el paquete de versión (`dotnet-runtime-10.0.10-win-x64.zip`, solo para
`dotnet10`) o la descarga. El paquete no trae el
`dotnet-sdk-10.0.403-win-x64.zip` del SDK, pero una copia puesta ahí a mano se usa
y ahorra la descarga. Eidos rechaza cualquiera de los dos archivos salvo que su
SHA-256 coincida con el integrado en Eidos. Descomprime el zip con `7z`, el mismo
7-Zip que usa para los archivos de mods.

`dotnetsdk10` es el SDK de .NET 10, una descarga de 301 MB que ocupa 808 MB
descomprimida. Synthesis lo necesita para compilar sus patchers, y llama a un
`dotnet` sin ruta para construirlos y ejecutarlos, así que una herramienta que
declara `dotnetsdk10` recibe también la carpeta del SDK en `WINEPATH`, además de
`NUGET_CERT_REVOCATION_MODE=offline`, `DOTNET_GENERATE_ASPNET_CERTIFICATE=false`
y `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
Compilar un patcher dejaría en marcha nodos de MSBuild y el servidor
VBCSCompiler, y bajo Wine estos mantienen vivo el wineserver del prefijo, así
que la herramienta recibe también `MSBUILDDISABLENODEREUSE=1`,
`DOTNET_CLI_USE_MSBUILD_SERVER=0` y `UseSharedCompilation=false`.
`DOTNET_ROLL_FORWARD=Major` permite que un patcher compilado para .NET 8
(Mutagen anterior a 0.54) se ejecute con el runtime .NET 10 del SDK cuando no se
encuentra ningún runtime de .NET 8.
Una entrada de Synthesis en `tools.ini` que todavía lleva el antiguo valor por
defecto, `dotnet8`, `vcrun2022`, pasa a `dotnetsdk10`, `vcrun2022` cuando Eidos la
lee.

NuGet comprueba la firma de cada paquete que restaura contra el almacén de
raíces de Windows, y a un prefijo de Wine le faltan las raíces de firma de
código y de marca de tiempo que el SDK incluye como
`sdk/10.0.403/trustedroots/codesignctl.pem` y `timestampctl.pem`. Sin ellas,
restaurar `Mutagen.Bethesda.Synthesis` 0.36.6 falla con `NU3018` y `NU3028`. Por
eso, antes de ejecutar una herramienta que declara `dotnetsdk10`, y una vez
descargado el SDK, Eidos añade los 385 certificados de ambos archivos al almacén
de raíces del usuario actual en el prefijo del juego, en la misma importación
con `regedit` que escribe la ruta del juego (ver más abajo). Si falta alguno,
los vuelve a importar. A partir de ahí, todos los programas de Windows de ese
prefijo confían en estas raíces. Con ellas, en un prefijo nuevo y con la caché
de NuGet vacía, la misma restauración y la compilación funcionan con el Wine
11.21 del sistema y con GE-Proton11-9.

Eidos no instala nada de los niveles 2 o 3 hasta que se lo pides:

```sh
eidos prereqs skyrimse            # mostrar lo que necesitan las herramientas registradas, y su estado
eidos prereqs skyrimse --install  # instalar lo que falta (primero las copias del paquete, luego descargas)
```

En la interfaz los mismos estados están bajo el campo Prereqs, y los que faltan
son botones. Un verbo que no es ni empaquetado, ni un runtime, ni un verbo de
nivel 2 se reporta como probable errata en vez de ofrecerse como descarga.

`eidos prereqs <instance> --install` se niega mientras el juego, Steam o una
herramienta sigan usando el prefijo. Los runtimes del nivel 3 se instalan incluso
antes de que exista el prefijo. Los instaladores que ejecuta Eidos necesitan un
prefijo que exista: antes de que la tienda haya arrancado el juego una vez, se
detienen con «start the game once through its store, then install again». Los
verbos del nivel 2 se instalan de uno en uno y cada éxito queda registrado, así
que tras un fallo la misma orden continúa donde se quedó. Para una copia de GOG o
Epic sin ajustes de Heroic, `--install` copia las DLL empaquetadas y luego se
detiene antes de los verbos del nivel 2: dice que uses el winetricks de ese
runner, y lee el `winetricks.log` del prefijo en la siguiente comprobación.

### Por qué DynDOLOD necesita `dotnet10`

DynDOLOD no construye el LOD de objetos él mismo: llama a LODGen, y trae tres.
`LODGenx64.exe` apunta a .NET Framework 4.8, que bajo Proton se enruta al Mono de
Wine, y el inicializador de `System.Uri` de Mono llama a un método que Mono no
implementa. LODGen muere antes de su primera línea de trabajo, dejando un registro
con un cartel de versión y nada más, y un diálogo de DynDOLOD que solo dice
«failed for one or more worlds».

Instalar el .NET Framework de verdad no lo arregla: Proton reemplaza
`mscoree.dll`, el cargador que lo encontraría, por un enlace simbólico dentro de
su propio árbol, y lo rehace en cada actualización del prefijo.

La compilación que funciona es `LODGenx64Win10.exe`, que apunta a .NET moderno y
nunca toca `mscoree`. Apunta `DOTNET_ROOT` a un runtime de .NET 10 y funciona. Eso
es lo que provisiona `dotnet10`, y Eidos fija la variable al lanzar cualquier
herramienta que lo declare.

### Por qué Pandora necesita `dotnet10`

Pandora es una aplicación Avalonia, no WPF, y apunta a `net10.0` desde Pandora
4.1.0, así que el runtime de .NET 10 por sí solo la ejecuta. Cuando no encuentra
ningún runtime, dice que instales el .NET Desktop Runtime. Es el texto genérico que
muestra el lanzador de .NET para cualquier programa con interfaz gráfica que no
encuentra runtime, y `dotnetdesktop8` no lo arregla.

Eidos 1.21.0 y anteriores daban a Pandora `dotnetdesktop8`, y una entrada conserva
su línea `prereqs=`. Cuando Eidos lee una entrada de Pandora que aún lista solo
`dotnetdesktop8`, y el `.runtimeconfig.json` junto a su ejecutable nombra
`net10.0`, la entrada recibe `dotnet10` en su lugar. Pandora 2.x apunta a .NET 8 y
conserva lo que tiene.

### Por qué PGPatcher necesita `--ignore-mo2vfscheck`

PGPatcher reescribe mallas y plugins para que los mods de texturas que tengas
(vanilla, parallax, complex material, PBR) usen el shader correcto en cada
superficie. Se niega a ejecutarse fuera de un gestor de mods, porque parchear una
carpeta de juego desnuda sería destructivo.

PGPatcher busca usvfs, la capa de enganche de DLL que MO2 inyecta en el juego.
Eidos no tiene ninguna: la vista combinada es un montaje FUSE en el que Wine entra
sin saber que existe. Así que la comprobación no encuentra nada y la herramienta
sale de inmediato con

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

nombrando un programa que el usuario no está ejecutando, antes de hacer ningún
trabajo. La respuesta de su propio autor a los dos informes de Linux
([#716][pg716] y [#725][pg725], ambos de Fluorine, el fork de MO2 para Linux, que
usa FUSE por la misma razón) es una opción que salta la comprobación.

Por eso Eidos la aporta. Un PGPatcher encontrado en un conjunto de mods llega con
el argumento ya puesto, y escribir `PGPatcher` como título en el diálogo
Executables rellena el campo Arguments igual. Lo que escribas siempre gana: el
rellenado solo toca un campo que dejaste vacío, igual que los prerrequisitos de
arriba.

La lista está en `default_args` (`crates/eidos-instance/src/tools.rs`), junto a
`default_prereqs`.

Su runtime no necesita nada, algo raro en una herramienta .NET: incluye el suyo
(`dotnetlib/`, con un `runtimeconfig.json` que declara `includedFrameworks`) y
entrega las DLL del CRT de Microsoft junto a su ejecutable. Deja Prereqs vacío.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher y PGLite en modo Mod Organizer 2

Con Mod Manager en `None`, PGPatcher ve una sola carpeta `Data` combinada y no
puede saber de qué mod viene una malla o una textura, así que no tiene conflictos
que mostrar ni orden de mods que seguir. Su modo `Mod Organizer 2` lee la
instancia directamente: el `modlist.txt` del perfil, los mods activados en su
orden de prioridad y los archivos de la carpeta de cada mod. PGLite, la versión
dentro del juego, lee la instancia de la misma forma. Una instancia de Eidos ya
tiene la estructura de MO2 (`mods/`, `profiles/<name>/modlist.txt`,
`overwrite/`), por lo que este modo funciona con ella.

En el lanzador de PGPatcher:

1. Pon Mod Manager en `Mod Organizer 2`.
2. Pon MO2 Options -> Instance Location en la carpeta de la instancia de Eidos,
   escrita como ruta de Windows en la unidad `Z:` de Wine, que es `/`. Una
   instancia portátil en `/mnt/Games/Eidos-Skyrim` es
   `Z:\mnt\Games\Eidos-Skyrim`; una global es
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Mantén `--ignore-mo2vfscheck` en los argumentos de la herramienta. En este modo
   PGPatcher busca usvfs y se detiene si no lo encuentra.

PGPatcher solo acepta la carpeta si contiene un `modorganizer.ini`, así que
Eidos escribe `ModOrganizer.ini` en la raíz de la instancia cada vez que inicia
el juego o una herramienta, para el perfil que se inicia. El archivo solo se
reescribe cuando su contenido cambia. Contiene:

- en `[General]`: `gameName` (el nombre del juego en MO2, así que `New Vegas` para
  Fallout: New Vegas), `gamePath` (la carpeta de instalación del juego en la
  tienda como ruta `Z:`, también en una instancia que se ejecuta desde una copia
  del juego) y `selected_profile`;
- en `[Settings]`: `mod_directory`, `profiles_directory` y
  `overwrite_directory` como rutas `Z:` absolutas, más `download_directory`
  cuando la instancia tiene una carpeta `downloads/`.

Su primera línea es un comentario que dice que lo escribió Eidos. Un
`ModOrganizer.ini` sin esa línea, con cualquier combinación de mayúsculas en el
nombre, nunca se modifica ni se borra: una instancia de MO2 que abriste en Eidos
conserva el suyo, y PGPatcher lee ese archivo tal cual. MO2 descarta los
comentarios al guardar su configuración, así que un archivo sobre el que MO2 ha
guardado pasa a ser de MO2. Un archivo vacío cuenta como de Eidos.

Un archivo así nombra el perfil que MO2 seleccionó por última vez, y PGPatcher
usa ese perfil, no el que inicia Eidos. Cuando los dos difieren, Eidos registra
una advertencia al lanzar. Selecciona el mismo perfil en MO2, o cambia el nombre
del archivo o quítalo para que Eidos escriba el suyo.

Las herramientas en modo Mod Organizer 2 también leen el juego del `gamePath` de
ese archivo. Cuando no nombra ni la carpeta del juego desde la que se ejecuta la
instancia ni la instalación de la tienda, la pestaña Diagnostics muestra
**ModOrganizer.ini points tools at another game folder**, con esa ruta y la
carpeta del juego que ejecuta esta instancia. Cambia `gamePath` en el archivo, o
quita el archivo para que Eidos escriba el suyo.

Eidos escribe su archivo solo cuando inicia algo, de modo que entre dos
lanzamientos nombra el perfil del último. Una herramienta que Eidos inicia
encuentra ahí su propio perfil.

PGPatcher se detiene con un error cuando el perfil no tiene ningún mod activado
y cuando su carpeta de salida es la carpeta de un mod activado, igual que bajo
MO2.

## Los programas propios de Wine en el prefijo del juego

El menú File ejecuta cuatro programas de Wine en el prefijo del juego: winecfg, regedit, el explorador de archivos de Wine y el desinstalador. Arrancan como cualquier herramienta, con la versión de Proton o Wine que ejecuta las herramientas del juego, con los mods montados, y la ventana espera a que terminen. **Prefix drive_c** abre la unidad `C:` del prefijo en tu gestor de archivos. Una fila aparece atenuada hasta que el prefijo tiene ese programa, algo que ocurre la primera vez que la tienda inicia el juego. Desde una terminal, `eidos tool <instance> run winecfg` hace lo mismo, y también `regedit`, `explorer` y `uninstaller`, cuando ninguna herramienta de la lista tiene ese título.

Un cambio hecho en regedit al `installed path` del juego se restablece en la siguiente ejecución de una herramienta; consulta la sección siguiente.

## La ruta del juego en el prefijo

Las herramientas de Windows encuentran su juego leyendo
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, una clave que escribe
el propio instalador del juego. Steam bajo Proton nunca ejecuta ese instalador.
Sin la clave, xEdit, Wrye Bash y DynDOLOD abren sobre una ruta vacía.

Antes de ejecutar una herramienta, Eidos escribe la clave con una importación de
`regedit` en el prefijo del juego, salvo que una importación anterior ya escribiera
la misma ruta y el prefijo todavía la contenga. La clave nombra la instalación de
la tienda, también en una instancia que se ejecuta desde una copia del juego,
porque ahí es donde está montada la vista combinada. Para una herramienta que
declara `dotnetsdk10`, la misma importación añade las raíces de firma de NuGet
(consulta [más arriba](#tres-clases-de-prerrequisito)). Una herramienta
`runner=native` se salta este paso. Cuando el prefijo todavía no existe, o lo
tiene otro programa, la herramienta se ejecuta igualmente, y Eidos imprime un
aviso que nombra la tienda desde la que arrancar el juego una vez, o el programa
que hay que cerrar. Cuando la clave nombra una carpeta distinta de la instalación
de la tienda, la pestaña Diagnostics muestra **Windows tools cannot find <game>**
hasta que la siguiente ejecución de una herramienta la escriba de nuevo.

## Llegar a una herramienta: ocultar, fijar y un acceso directo de escritorio

Los valores por defecto de un juego incluyen herramientas que quizá nunca uses. En
el diálogo Executables:

- Pin to top pone una entrada a la cabeza de la lista Run.
- Hide from picker saca una sin borrarla.
- Desktop shortcut escribe un `.desktop` en `~/.local/share/applications`, así que
  aparece en tu menú de aplicaciones y en una búsqueda, no en el escritorio.
  Ejecuta `eidos tool <instance> run <title>` directamente, de modo que la
  herramienta sale a través de la vista combinada con el perfil de esta instancia
  sin que la ventana de Eidos esté abierta.

Ocultar y fijar cambian solo cómo se llega a una herramienta, así que se aplican a
los valores por defecto de cada juego igual que a tus propias entradas.

El cargador del script extender, el lanzador y el binario del juego no tienen
botón Desktop shortcut. Inician el juego, y el juego siempre se inicia a través de
su tienda, cosa que ya hace el acceso directo de la propia tienda. Un acceso
directo creado para uno de ellos con Eidos 1.22 o anterior sigue funcionando:
igual que `eidos tool <instance> run <title>` escrito en una terminal, pide a la
tienda que inicie el juego, como hace Run en una ventana que no abrió ninguna
tienda. Cuando falta el ajuste de la tienda, la orden no inicia nada y muestra
qué poner. Un acceso directo no muestra ningún mensaje, así que ejecuta la misma
orden en una terminal para leerlo.

Eidos entrega así cualquier entrada a la tienda cuando su ejecutable tiene el
nombre de archivo del cargador del script extender, del lanzador o del binario del
juego, sin distinguir mayúsculas: una entrada por defecto, una que añadiste, una
importada de MO2 o una ejecución con `--exe`. Primero marca esta instancia como la
última usada, para que la ventana que abre la tienda muestre esta instancia, y
luego ejecuta `steam -applaunch <id>`,
`xdg-open heroic://launch?appName=<app>&runner=<runner>` para Heroic, o
`legendary launch <app>`. Con `--print` imprime esa orden de la tienda y no inicia
nada. La orden `steam` debe estar en el `PATH`: el Steam de Flatpak no la pone
ahí, así que el arranque falla y el mensaje dice que inicies el juego desde Steam.

La entrada se ejecuta directamente a través de la vista combinada, como cualquier
herramienta, cuando una tienda inició el proceso: está definida una de
`SteamGameId`, `SteamAppId`, `HEROIC_APP_NAME`, `HEROIC_APP_RUNNER` o
`EIDOS_STORE_STARTED`. Sin ninguna de ellas, Eidos no arranca el juego con Proton
por sí mismo. Cuando la instancia no tiene cargador del script extender (en la
carpeta del juego desde la que se ejecuta, en el `Root/` de un mod activado o en
`Overwrite/Root/`), no se inicia nada y la línea dice que instales primero el
script extender. Para una copia de GOG fuera de Heroic, o un id de tienda que
Eidos no puede pasar, dice que inicies el juego desde esa tienda.

Un perfil puede tener su propio acceso directo para el juego. Haz clic derecho en
el perfil y elige Desktop shortcut: la entrada ejecuta
`eidos tool <instance> run "<game>" --profile <name>`, que convierte ese perfil en
el activo, igual que elegirlo en la ventana, y luego pide a la tienda que inicie
el juego. La ventana de Eidos que abre la tienda muestra ese perfil. `--profile`
funciona con cualquier `eidos tool ... run` y selecciona el perfil antes de que
arranque la herramienta; con `--print` no selecciona nada.

## Una herramienta que es su propia aplicación de Steam

El Creation Kit es una aplicación de Steam aparte y quiere su propio AppID; unas
cuantas herramientas de modding distribuidas por Steam son igual. Fija Steam
AppID en la entrada y Eidos la lanza bajo ese id en vez del del juego.

En Windows esto significa un lanzador distinto. Aquí son dos variables de entorno
en la misma ejecución, `SteamAppId` y `SteamGameId`. Eidos fija las dos, porque
Proton lee una y las propias bibliotecas de Steam leen la otra, y una herramienta
que las ve discrepar falla sin un error claro. `eidos tool ... --print` muestra
exactamente lo que recibiría la ejecución real.

## Variables de entorno y herramientas sin Proton

Tres claves de `<instance>/tools.ini` todavía no tienen campo en el diálogo
Executables, así que añádelas a mano en la sección de la herramienta. Cada línea
`env=NAME=value` fija una variable solo para las ejecuciones de esa herramienta:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos aplica estas líneas después de todo lo que fija él mismo y después de las
variables del `launch-env.ini` del perfil, así que un valor puesto aquí reemplaza
a ambos. Eso incluye el `WINEDLLOVERRIDES` que Eidos construye a partir de las DLL
que traen tus mods: una herramienta que lo fije debe listar en él cada DLL que
necesite. Los wrappers del perfil rodean la orden de una herramienta igual que la
del juego; consulta
[Variables y wrappers por perfil](usage.md#variables-y-wrappers-por-perfil).
Un nombre se compone de letras, dígitos y `_`, y no empieza por un dígito. Una
línea con cualquier otro nombre se ignora.

`runner=native` ejecuta `exe` directamente, sin Proton. La vista combinada se
monta como para cualquier herramienta, y las líneas `arg0=`, `arg1=` son sus
argumentos. Úsalo para un programa de Linux, o para una herramienta de una copia
que Steam no gestiona y para la que Heroic no registra ningún runner: `exe` es
el Wine de esa copia, el programa de Windows es el primer argumento y el prefijo
sale de `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` tiene que ser una ruta absoluta. El directorio de trabajo es por defecto
la carpeta que contiene `exe`, aquí la del runner, así que fija `workdir=`.
Solo `exe` se reescribe a su ruta combinada: para un programa de Windows dentro
de un mod, da su ruta bajo la carpeta `Data` del juego, donde la vista combinada
lo muestra. Una herramienta nativa se salta los pasos de Proton: la clave de
registro de arriba, la caché de shaders de Steam, las raíces .NET vacías y la
retirada del overlay de Steam de `LD_PRELOAD`.

En una copia de GOG o Epic instalada con Heroic, las herramientas no necesitan
esa entrada si los ajustes del juego en Heroic (`GamesConfig/<appName>.json`)
indican una versión de Wine o Proton: Eidos las ejecuta con esa versión en el
prefijo de Heroic del juego, y `eidos prereqs <instance> --install` la usa para
los instaladores y para winetricks. Una copia instalada solo con Legendary no
tiene ajustes de Heroic.

`slr=1` inicia la herramienta, y la importación al registro que la precede,
dentro del contenedor de Steam Linux Runtime que indica el Proton de la
herramienta (`require_tool_appid` en su `toolmanifest.vdf`), igual que Steam
inicia el juego. Ese runtime debe estar instalado en una biblioteca de Steam; si
no lo está, la ejecución se detiene e indica la aplicación de Steam que hay que
instalar. `eidos prereqs` sigue ejecutando sus instaladores y winetricks en el
host, y una herramienta con `runner=native` ignora la clave. El contenedor ve tu
carpeta personal y la biblioteca de Steam del juego. Una herramienta, un mod o
un runtime en cualquier otro sitio, en otro disco por ejemplo, necesita
`env=STEAM_COMPAT_MOUNTS=/esa/ruta` (varias rutas separadas por `:`). Un Proton
instalado en `/usr/share/steam/compatibilitytools.d` probablemente no sirva con
`slr=1`, porque `/usr` dentro del contenedor es el del runtime. Todavía no se ha
probado ninguna herramienta dentro del contenedor.

`eidos tool <instance> run <title> --print` muestra la orden final y cada
variable que la ejecución fija sobre el entorno heredado, incluidas estas
líneas. Para una entrada que arranca el juego imprime en su lugar la orden de la
tienda (consulta
[Llegar a una herramienta](#llegar-a-una-herramienta-ocultar-fijar-y-un-acceso-directo-de-escritorio)).

## BethINI Pie

Eidos no incluye BethINI Pie. Coloca la versión que descargaste de su página de Nexus en el `mods/` de esta instancia o en tu carpeta de herramientas, y `Bethini.exe` aparecerá en la lista de herramientas, en los mismos juegos que PGPatcher. Ejecutado a través de Eidos, edita la copia de los INI del juego que tiene el perfil. Antes de cada ejecución de una herramienta cuyo título contenga `bethini`, Eidos guarda un punto de restauración de esos INI; Backups en la barra de herramientas, y luego INIs, restaura uno. Un INI que no existía cuando se hizo la copia se deja como está. El diálogo conserva los 10 puntos de restauración más recientes de cada tipo, y restaurar uno guarda antes una copia de los INI tal como están, así que una elección equivocada se puede deshacer.

## Los ajustes propios de una herramienta siguen siendo suyos

Eidos no comprueba la mayoría de los ajustes de las herramientas; uno incorrecto
suele fallar sin error.

El Game Data Path de BodySlide (Settings) debe apuntar al directorio `Data` del
juego, no a la carpeta del juego que está encima. Fijado un nivel demasiado
arriba, un batch build informa «All sets processed successfully» y escribe 1439
mallas donde el juego nunca las buscará. Eidos las recoge en `Overwrite/Root/`, así
que tu instalación del juego queda intacta, y la única señal en el juego es que tus
cuerpos no están construidos.

Antes de ejecutar BodySlide u Outfit Studio, la línea de estado avisa cuando
`GameDataPath` u `OutputDataPath` en el `CalienteTools/BodySlide/Config.xml` que
gana no lleva a la carpeta `Data` del juego durante la ejecución, y la pestaña
Diagnostics muestra el mismo aviso como una tarjeta con el valor que hay que
poner. `eidos check <instance>` también lo informa. Una ruta en `S:` se lee como
Eidos asigna `S:` para una herramienta, a la carpeta de la biblioteca de Steam. Se
acepta un `OutputDataPath` dentro de la carpeta `mods/` de la instancia, para un
mod de salida. Diagnostics también muestra una tarjeta cuando `Overwrite/Root/`
contiene una carpeta `meshes` o `textures`.

La salida de las herramientas va en el Overwrite. Cuando una ejecución produce
algo que merece la pena conservar, Overwrite -> Create mod... lo convierte en
un mod corriente que se puede ordenar, desactivar y quitar como cualquier otro.
