<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Herramientas: xEdit, BodySlide, DynDOLOD, FNIS

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
- la **carpeta de herramientas** que fijas en Settings (Tools -> Tools folder),
  para el directorio compartido entre instancias - `/mnt/Games/Tools` y similares.

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
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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

## Las DLL que necesita una herramienta se eligen por su NOMBRE

Esta es la parte sorprendente, así que conviene decirlo claro: **el título que le
das a una herramienta decide qué prerrequisitos de runtime le provisiona Eidos.**
La coincidencia es una subcadena del título, sin distinguir mayúsculas.

| Si el título contiene | Eidos solicita |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
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

### Tres clases de prerrequisito

**Nivel 1 - DLL empaquetadas** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). Eidos
las trae y las copia en el prefijo al lanzar. Nada que hacer, sin red.

**Nivel 2 - verbos de winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Estos escriben claves de registro, el GAC y hosts del CLR,
así que no se pueden copiar como archivos. **Descargan de Microsoft**.

**Nivel 3 - runtimes** (`dotnet10`). Un runtime moderno de .NET son 193 archivos
que viven en su propio directorio y se encuentran a través de `DOTNET_ROOT`: nunca
registrados, nunca instalados en el prefijo, así que ninguno de los otros niveles
puede transportarlo. Eidos lo descarga él mismo, lo comprueba contra una suma de
verificación incrustada en el binario, y lo cachea en
`~/.local/share/Colony/Eidos/runtimes/` - **fuera de cualquier instancia**, porque
78 MB no es por juego ni por perfil.

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
