<!-- eidos-i18n: source=docs/guide/tools.md sha=bc1c18f7029d1b12bcae46d861626848edfafb5a -->

# Herramientas: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Una herramienta ejecutada a través de Eidos ve **la vista combinada**, dentro del
propio prefijo Proton del juego. Lee lo que leerá el juego - cada mod activado, en
orden de prioridad - y lo que escriba aterriza en el Overwrite, donde un clic lo
convierte en un mod de verdad.

## Las que Eidos encuentra por sí solo

Algunas herramientas tienen nombres lo bastante únicos como para encontrarlas en
vez de declararlas, y xEdit es el caso evidente: `FO4Edit.exe` para Fallout 4,
`SSEEdit.exe` para Skyrim SE, `TES5Edit.exe` para el original, y así - junto con
el gemelo **QuickAutoClean** de cada uno, que es el botón para las ediciones
sucias de las que LOOT no deja de avisar. Eidos las busca, por nombre de archivo,
en:

- la carpeta de instalación del juego, y los árboles `Root/` de los mods
  activados;
- **el `mods/` de esta instancia**, que es donde los usuarios de MO2 instalan
  herramientas;
- la **carpeta de herramientas** que fijas en Preferences (General -> Tools),
  para el directorio compartido entre instancias - `/mnt/Games/Tools` y similares.

`PGPatcher.exe` se encuentra de la misma manera en los juegos Creation Engine, y llega con un argumento sin el cual no puede funcionar - vea [mas abajo](#por-que-pgpatcher-necesita---ignore-mo2vfscheck).

La lista es por juego, así que a una instancia de Skyrim nunca se le ofrece el
editor de Fallout. La búsqueda se detiene cuatro niveles abajo, porque un conjunto
de mods son cientos de miles de archivos y esto se ejecuta cada vez que se
construye la lista de herramientas, y no sigue enlaces simbólicos. Una herramienta
encontrada así se configura exactamente igual que una que escribas tú: sus
runtimes salen de su nombre, por la misma regla que todo lo que viene abajo.

Si una herramienta está en otro sitio, o quieres argumentos distintos, añádela a
mano - una entrada de usuario con el mismo título anula cualquier cosa encontrada
automáticamente.

## Añadir una

En la interfaz: **Tools -> Executables**, luego Add. Desde la línea de órdenes:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # listar lo que está registrado
eidos tool skyrimse run BodySlide         # ejecutarlo a través de la vista combinada
eidos tool skyrimse run BodySlide --print # mostrar la orden sin ejecutarla
```

El script extender, el binario del juego y el lanzador se detectan
automáticamente; sólo hay que registrar las herramientas adicionales.

### Apúntala al archivo real, esté donde esté

Registra el ejecutable donde de verdad está. Si la herramienta se instaló como
mod, eso es dentro de la carpeta del mod:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(esa es la ruta de la instancia global - para una instancia portátil la misma
regla se aplica bajo su propia carpeta, `<instance>/mods/...`; ojo, una ruta
absoluta como esta es lo único que no sobrevive a MOVER después una carpeta
portátil).

Eidos reescribe esa ruta a la combinada antes de lanzar, de modo que la
herramienta se ejecuta desde `<game>/Data/CalienteTools/BodySlide/` y ve ahí
también los archivos de todos los demás mods. Esto importa más de lo que parece:
BodySlide trae un directorio `SliderSets` **vacío**, y cada cuerpo que puede
construir viene de CBBE y de los mods de ropa. Lanzado desde su propia carpeta de
mod no encuentra nada y parece roto.

MO2 hace la misma reescritura, por la misma razón - su propio comentario nombra a
FNIS.

Una herramienta dentro de un mod **desactivado** no se puede reescribir, porque
sus archivos tampoco están en la vista. Eidos lo dice y la ejecuta desde su propia
carpeta en vez de fingir.

## Enviar la salida de una herramienta a su propio mod

Un generador - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - escribe cientos de
archivos. Por defecto aterrizan en el Overwrite con todo lo demás. Fija **Capture
output into** en el editor de Executables y la salida de esta ejecución va a ese
mod en su lugar:

```
Tools -> Executables -> (tu herramienta) -> Capture output into: FNIS Output
```

El mod se crea si no existe. Sólo se mueven los archivos que produjo ESTA
ejecución; lo que ya estaba en el Overwrite se queda ahí, así que dos herramientas
con destino de captura no se roban la salida entre ellas. Una ejecución que no
escribió nada no deja atrás un mod vacío.

Se hace después de la ejecución y no apuntando la capa de escritura al mod, que es
como lo hace MO2. Apuntar la capa de escritura a un mod lo ascendería a la máxima
prioridad durante toda la ejecución - dando la vuelta a cada conflicto en el que
está y devolviéndolos después - y escribiría directamente a través de los propios
archivos del mod sin copy-up. La captura llega al mismo estado final sin ninguna
de las dos cosas.

Si el mod de destino está desactivado, la salida se escribe igualmente pero el
juego no la verá, así que la herramienta regeneraría los mismos archivos en la
siguiente ejecución. Eidos avisa cuando es el caso.

### Cuando su salida está desactualizada

Eidos anota con qué mods, orden de plugins, perfil y ejecutable se generó la salida de una herramienta. Cuando uno de ellos cambia, la pestaña Diagnostics lo indica, y la tarjeta trae un botón **Run <tool> again** que ejecuta la herramienta con su configuración actual.

Para un generador cuya salida desactualizada rompe el juego, como Pandora o Nemesis, añada `check_before_play=1` a su entrada en `tools.ini`, o active **Ask before playing when its output is out of date** en el diálogo Executables. Run pregunta entonces primero: ejecutar la herramienta o jugar de todos modos. Jugar de todos modos se recuerda hasta que los mods, los plugins o el perfil vuelvan a cambiar, o hasta que se cierre la ventana. Eidos nunca inicia la herramienta por sí mismo.

## Las DLL que necesita una herramienta se eligen por su NOMBRE

Esta es la parte sorprendente, así que conviene decirlo claro: **el título que le
das a una herramienta decide qué prerrequisitos de runtime le provisiona Eidos.**
La coincidencia es una subcadena del título, sin distinguir mayúsculas.

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

Así que una herramienta registrada como **`BodySlide`** recibe sus DLL de
DirectX; el mismo ejecutable registrado como **`BS`** no recibe nada y puede
fallar al arrancar con un error que no dice nada sobre DLL. Nombra las
herramientas como el programa.

La lista está en `default_prereqs` (`crates/eidos-instance/src/tools.rs`), y el
campo `Prereqs` del diálogo de Executables es editable - la detección es un valor
por defecto, no una regla.

El titulo decide los **argumentos** de la misma manera, para la unica
herramienta que necesita uno que nadie podria adivinar: un titulo que contenga
`pgpatcher` (o `parallaxgen`, su nombre anterior) recibe
`--ignore-mo2vfscheck`. Por que, y por que no es opcional,
[mas abajo](#por-que-pgpatcher-necesita---ignore-mo2vfscheck).

### Tres clases de prerrequisito

**Nivel 1 - DLL empaquetadas** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). Eidos las trae y las copia en el prefijo al lanzar. Nada que
hacer, sin red.

**Nivel 2 - verbos de winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Estos escriben claves de registro, el GAC y hosts del CLR,
así que no se pueden copiar como archivos. **Descargan de Microsoft**.

**Nivel 3 - runtimes** (`dotnet10`, `dotnetsdk10`). Un runtime moderno de .NET son 193 archivos
que viven en su propio directorio y se encuentran a través de `DOTNET_ROOT`: nunca
registrados, nunca instalados en el prefijo, así que ninguno de los otros niveles
puede transportarlo. Eidos lo descarga él mismo, lo comprueba contra una suma de
verificación incrustada en el binario, y lo cachea en
`~/.local/share/Colony/Eidos/runtimes/` - **fuera de cualquier instancia**, porque
78 MB no es por juego ni por perfil.

`dotnetsdk10` es el SDK de .NET 10: una descarga de 301 MB que ocupa 808 MB
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
Una entrada de Synthesis en `tools.ini` que
todavía lleva el antiguo valor por defecto, `dotnet8`, `vcrun2022`, pasa a
`dotnetsdk10`, `vcrun2022` cuando Eidos la lee.

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

Nada de los niveles 2 o 3 se ejecuta en silencio:

```sh
eidos prereqs skyrimse            # mostrar lo que necesitan las herramientas registradas, y su estado
eidos prereqs skyrimse --install  # traer lo que falta (descarga)
```

En la interfaz los mismos estados están bajo el campo Prereqs, y los que faltan
son botones. Un verbo que no es ni empaquetado, ni un runtime, ni un verbo
conocido de winetricks se reporta como probable errata en vez de ofrecerse como
descarga.

### Por qué DynDOLOD necesita `dotnet10`

DynDOLOD no construye el LOD de objetos él mismo: llama a LODGen, y trae tres.
`LODGenx64.exe` apunta a .NET Framework 4.8, que bajo Proton se enruta al Mono de
Wine - cuyo inicializador de `System.Uri` llama a un método que Mono no
implementa. Muere antes de su primera línea de trabajo, dejando un log con un
banner de versión y nada más, y un diálogo de DynDOLOD que sólo dice «failed for
one or more worlds».

Instalar el .NET Framework de verdad no lo arregla: Proton reemplaza `mscoree.dll`
- el cargador que lo encontraría - por un enlace simbólico dentro de su propio
árbol, y lo rehace en cada actualización del prefijo.

La compilación que funciona es `LODGenx64Win10.exe`, que apunta a .NET moderno y
nunca toca `mscoree`. Apunta `DOTNET_ROOT` a un runtime de .NET 10 y funciona. Eso
es lo que provisiona `dotnet10`, y Eidos fija la variable al lanzar cualquier
herramienta que lo declare.

Eidos ejecuta el `winetricks` del sistema contra el propio `wine` de Proton y el
prefijo del juego, lo que esquiva el contenedor pressure-vessel de Steam y el
desajuste protontricks + Proton-GE. Una herramienta que declara un verbo de
Nivel 2 no instalado se lanza igualmente, con un aviso que nombra el verbo y la
orden para arreglarlo - el usuario puede tenerlo de otra parte.

### Por que PGPatcher necesita `--ignore-mo2vfscheck`

PGPatcher reescribe mallas y plugins para que los mods de texturas que tenga -
vanilla, parallax, complex material, PBR - usen el shader correcto en cada
superficie. Se niega a ejecutarse fuera de un gestor de mods, con razon:
parchear una carpeta de juego desnuda seria destructivo.

El problema es COMO lo comprueba: busca **usvfs**, la capa de enganche de DLL
que MO2 inyecta en el juego bajo Windows. Eidos no tiene ninguna y nunca la
tendra - la vista fusionada es un montaje FUSE en el que Wine entra sin saber
que existe - asi que la comprobacion no encuentra nada y la herramienta sale de
inmediato con

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

nombrando un programa que la usuaria no esta ejecutando, antes de hacer ningun
trabajo. La respuesta de su propio autor a los dos informes de Linux
([#716][pg716] y [#725][pg725], ambos de Fluorine, el port a Linux de MO2, que
usa FUSE por la misma razon) es una opcion que salta la comprobacion.

Por eso Eidos la aporta. Un PGPatcher encontrado en un pool de mods llega con el
argumento ya puesto, y escribir `PGPatcher` como titulo en el dialogo
Executables rellena el campo Arguments igual. Lo que usted escriba siempre gana:
el rellenado solo toca un campo que dejo vacio, igual que los prerrequisitos de
arriba.

La lista esta en `default_args` (`crates/eidos-instance/src/tools.rs`), junto a
`default_prereqs`. Tiene una entrada, y una basta para justificar una funcion:
nadie puede adivinar una opcion, y el fallo que evita nombra al programa
equivocado.

Su ejecucion no exige nada, algo raro en una herramienta .NET: incluye el suyo
(`dotnetlib/`, con un `runtimeconfig.json` que declara `includedFrameworks`) y
entrega las DLL del CRT de Microsoft junto a su ejecutable. Deje Prereqs vacio.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher y PGLite en modo Mod Organizer 2

Con Mod Manager en `None`, PGPatcher ve una sola carpeta `Data` combinada y no
puede saber de qué mod viene un mesh o una textura, así que no tiene conflictos
que mostrar ni orden de mods que seguir. Su modo `Mod Organizer 2` lee la
instancia directamente: el `modlist.txt` del perfil, los mods activados en su
orden de prioridad y los archivos de la carpeta de cada mod. PGLite, la versión
dentro del juego, lee la instancia de la misma forma. Una instancia de Eidos ya
tiene la estructura de MO2 (`mods/`, `profiles/<name>/modlist.txt`,
`overwrite/`), por lo que este modo funciona con ella.

En el lanzador de PGPatcher:

1. Ponga **Mod Manager** en `Mod Organizer 2`.
2. Ponga **MO2 Options -> Instance Location** en la carpeta de la instancia de
   Eidos, escrita como ruta de Windows en la unidad `Z:` de Wine, que es `/`. Una
   instancia portable en `/mnt/Games/Eidos-Skyrim` es
   `Z:\mnt\Games\Eidos-Skyrim`; una global es
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Mantenga `--ignore-mo2vfscheck` en los argumentos de la herramienta. En este
   modo PGPatcher busca usvfs y se detiene si no lo encuentra.

PGPatcher solo acepta la carpeta si contiene un `modorganizer.ini`, así que
Eidos escribe `ModOrganizer.ini` en la raíz de la instancia cada vez que inicia
el juego o una herramienta, para el perfil que se inicia. El archivo solo se
reescribe cuando su contenido cambia. Contiene:

- en `[General]`: `gameName` (el nombre del juego en MO2, así que `New Vegas` para Fallout: New Vegas), `gamePath` (la carpeta de instalación del juego
  como ruta `Z:`) y `selected_profile`;
- en `[Settings]`: `mod_directory`, `profiles_directory` y
  `overwrite_directory` como rutas `Z:` absolutas, más `download_directory`
  cuando la instancia tiene una carpeta `downloads/`.

Su primera línea es un comentario que dice que lo escribió Eidos. Un
`ModOrganizer.ini` sin esa línea, con cualquier combinación de mayúsculas en el
nombre, nunca se modifica ni se borra: una instancia de MO2 que abrió en Eidos
conserva el suyo, y PGPatcher lee ese archivo tal cual. MO2 descarta los
comentarios al guardar su configuración, así que un archivo sobre el que MO2 ha
guardado pasa a ser de MO2. Un archivo vacío cuenta como de Eidos.

Un archivo así nombra el perfil que MO2 seleccionó por última vez, y PGPatcher
usa ese perfil, no el que inicia Eidos. Cuando los dos difieren, Eidos registra
una advertencia al lanzar. Seleccione el mismo perfil en MO2, o cambie el nombre
del archivo o quítelo para que Eidos escriba el suyo.

Eidos escribe su archivo solo cuando inicia algo, de modo que entre dos
lanzamientos nombra el perfil del último. Una herramienta que Eidos inicia
encuentra ahí su propio perfil.

PGPatcher se detiene con un error cuando el perfil no tiene ningún mod activado
y cuando su carpeta de salida es la carpeta de un mod activado, igual que bajo
MO2.

## La ruta del juego en el prefijo

Las herramientas de Windows encuentran su juego leyendo
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, una clave que escribe
el propio instalador del juego - y que Steam bajo Proton nunca ejecuta. Sin ella
xEdit, Wrye Bash y DynDOLOD abren sobre una ruta vacía. Eidos la escribe antes de
ejecutar una herramienta: idempotente, aditiva, y omitida si el prefijo no está
inicializado o está en uso.

## Llegar a una herramienta: ocultar, fijar y un acceso directo de escritorio

Los valores por defecto de un juego incluyen herramientas que quizá nunca uses, y
un selector que lista ocho entradas para llegar a la segunda es un selector que
nadie lee. En el diálogo de Executables:

- **Pin to top** pone una entrada a la cabeza de la lista Run.
- **Hide from picker** saca una sin borrarla.
- **Desktop shortcut** escribe un `.desktop` en
  `~/.local/share/applications` - donde le corresponde a un lanzador en un
  sistema freedesktop, así que aparece en tu menú de aplicaciones y en una
  búsqueda, no en el escritorio. Ejecuta `eidos tool <instance> run <title>`
  directamente, lo que significa que la herramienta sale **a través de la vista
  combinada con el perfil de esta instancia** sin que la ventana de Eidos esté
  abierta siquiera.

Ocultar y fijar tienen que ver con cómo se *llega* a una herramienta y no con lo
que ejecuta, así que se aplican a los valores por defecto de cada juego igual que
a tus propias entradas.

## Una herramienta que es su propia aplicación de Steam

El Creation Kit es una aplicación de Steam aparte y quiere su propio AppID; unas
cuantas herramientas de modding distribuidas por Steam son igual. Fija **Steam
AppID** en la entrada y Eidos la lanza bajo ese id en vez del del juego.

En Windows esto significa un lanzador distinto. Aquí son dos variables de entorno
en la ejecución que ya se estaba construyendo - `SteamAppId` y `SteamGameId`, las
dos, porque Proton lee una y las propias bibliotecas de Steam leen la otra, y una
herramienta que las ve discrepar falla de forma rara en vez de clara.
`eidos tool ... --print` muestra exactamente lo que recibiría la ejecución real.

## Variables de entorno y herramientas sin Proton

Tres claves de `<instance>/tools.ini` todavía no tienen campo en el diálogo de
Executables, así que añádelas a mano en la sección de la herramienta. Cada línea
`env=NAME=value` fija una variable solo para las ejecuciones de esa herramienta:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos aplica estas líneas después de todo lo que fija él mismo, así que un valor
puesto aquí reemplaza el suyo. Eso incluye el `WINEDLLOVERRIDES` que construye a
partir de las DLL que traen tus mods: una herramienta que lo fije debe listar en
él cada DLL que necesite. Un nombre se compone de letras, dígitos y `_`, y no
empieza por un dígito. Una línea con cualquier otro nombre se ignora.

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
la carpeta que contiene `exe`, aquí la del ejecutor, así que fija `workdir=`.
Solo `exe` se reescribe a su ruta combinada: para un programa de Windows dentro
de un mod, da su ruta bajo la carpeta `Data` del juego, donde la vista combinada
lo muestra. Una herramienta nativa se salta los pasos de Proton: la clave de
registro de arriba, la caché de shaders de Steam, las raíces .NET vacías y la
retirada del overlay de Steam de `LD_PRELOAD`.

En una copia de GOG o Epic instalada con Heroic, las herramientas no necesitan
esa entrada si los ajustes del juego en Heroic (`GamesConfig/<appName>.json`)
indican una versión de Wine o Proton: Eidos las ejecuta con esa versión en el
prefijo de Heroic del juego, y `eidos prereqs <instance> --install` se la pasa a
winetricks. Una copia instalada solo con Legendary no tiene ajustes de Heroic.

`slr=1` inicia la herramienta, y la importación al registro que la precede,
dentro del contenedor de Steam Linux Runtime que indica el Proton de la
herramienta (`require_tool_appid` en su `toolmanifest.vdf`), igual que Steam
inicia el juego. Ese runtime debe estar instalado en una biblioteca de Steam; si
no lo está, la ejecución se detiene e indica la aplicación de Steam que hay que
instalar. `eidos prereqs` sigue ejecutando winetricks en el host, y una
herramienta con `runner=native` ignora la clave. El contenedor ve tu carpeta
personal y la biblioteca de Steam del juego. Una herramienta, un mod o un
runtime en cualquier otro sitio, en otro disco por ejemplo, necesita
`env=STEAM_COMPAT_MOUNTS=/esa/ruta` (varias rutas separadas por `:`). Un Proton
instalado en `/usr/share/steam/compatibilitytools.d` probablemente no sirva con
`slr=1`, porque `/usr` dentro del contenedor es el del runtime. Todavía no se ha
probado ninguna herramienta dentro del contenedor.

`eidos tool <instance> run <title> --print` muestra la orden final y cada
variable que la ejecución fija sobre el entorno heredado, incluidas estas
líneas.

## Los ajustes propios de una herramienta siguen siendo suyos

Eidos pone una herramienta en el sitio correcto con las DLL correctas. Lo que la
herramienta haga luego con su configuración es entre tú y la herramienta, y el
fallo suele ser silencioso.

El ejemplo resuelto, porque si no cuesta una hora: el **Game Data Path** de
BodySlide (Settings) debe apuntar al directorio `Data` del juego, no a la carpeta
del juego que está encima. Fijado un nivel demasiado arriba, un batch build
informa «All sets processed successfully» y escribe 1439 mallas donde el juego
nunca las buscará. Eidos las recoge - aterrizan en `Overwrite/Root/` en vez de en
tu instalación - pero nada está mal desde el punto de vista del juego salvo que
tus cuerpos no están construidos.

La salida de las herramientas va en el Overwrite. Cuando una ejecución produce
algo que merece la pena conservar, **Overwrite -> Create mod...** lo convierte en
un mod corriente que se puede ordenar, desactivar y quitar como cualquier otro.
