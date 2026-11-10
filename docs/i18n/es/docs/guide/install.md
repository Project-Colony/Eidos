<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Instalar Eidos

Cuatro formas de entrar. Colony instala un solo archivo, `eidos-linux`, que es el
programa completo. El paquete de versión, el paquete de Arch y una compilación
desde el código fuente te dan `eidos` (la línea de órdenes), `eidos-gui` y el
programa auxiliar estático de vista previa de NIF `eidos-nif-preview` uno junto a
otro. El paquete de Arch y la compilación desde el código fuente instalan además
`eidos-fuse` y `eidos-launch`, las dos líneas de órdenes de más bajo nivel. Cada
vía puede registrar el manejador `nxm://` que hace que el botón «Mod Manager
Download» de Nexus llegue a tu instancia.

## Lo que necesitas primero

| Requisito | Por qué |
|---|---|
| Linux con FUSE | `fusermount3` en tu PATH. Toda distribución actual lo incluye. |
| Un juego con Proton, lanzado una vez | Steam solo crea el prefijo Wine del juego en el primer lanzamiento, y Eidos trabaja dentro de él. Eidos también detecta las copias de GOG y Epic de Skyrim Special Edition, las copias de Epic de Fallout 3 y Fallout: New Vegas, y la copia de GOG de Stardew Valley. Esas se ejecutan mediante Heroic o Legendary con Eidos como wrapper del juego allí: consulta [usage.md](usage.md#lanzar-el-juego). |
| Un binario de 7-Zip: `7z`, `7zz` o `7za` | Para instalar archivos de mods. `7zip` en Arch, `p7zip` en la mayoría de las demás distribuciones. |
| `par2` (opcional) | Datos de recuperación PAR2 para las copias de `eidos pack`. `par2cmdline` en Arch y Fedora, `par2` en Debian y Ubuntu. |

Sin root, sin demonio, sin editar `/etc/fuse.conf` y sin nada que añadir a tus
grupos. Eidos monta dentro de un espacio de nombres privado que pertenece al
proceso del juego.

Algunos ajustes del sistema solo se notan como cierres inesperados. La pestaña
Diagnostics comprueba `vm.max_map_count` (1048576 o más), el límite duro de
archivos abiertos (524288 o más) y si el juego, su prefijo o la instancia están
en NTFS, exFAT o FAT; [troubleshooting.md](troubleshooting.md#el-sistema) tiene el
arreglo para cada caso.

## Mediante Colony

Busca Eidos en [Colony](https://github.com/Project-Colony/Colony) e instálalo; las
actualizaciones llegan a través del lanzador. Colony guarda un solo archivo,
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Ejecútalo y se abre la ventana.
Llamado a través de un enlace llamado `eidos`, el mismo archivo es la línea de
órdenes. Pon uno en tu `PATH` y registra el manejador de descargas de Nexus:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

La ventana no necesita ningún enlace: cada vez que lanza un juego o una
herramienta, ejecuta su propio archivo como línea de órdenes. La vista previa de
modelos NIF necesita `eidos-nif-preview`, que solo incluyen el paquete de versión,
el paquete de Arch y las compilaciones desde el código fuente; sin él, la vista
previa lo indica y nada más se ve afectado. `eidos-linux` tampoco trae ninguno de
los runtimes de Microsoft que incluye el paquete
([más abajo](#los-runtimes-de-microsoft-del-paquete)): cuando una herramienta
necesita uno, Eidos lo descarga de Microsoft en cuanto inicias la instalación de
prerrequisitos. Apunta Steam al propio archivo, como se explica más abajo.

## Un paquete de versión

Descarga `eidos-linux-bundle.tar.gz` y sus `.sig`, `.meta` y `.meta.sig` desde
la [última versión](https://github.com/Project-Colony/Eidos/releases/latest), y
compruébalos como muestra la
[Política de firma de código](../../README.md#política-de-firma-de-código) del
README. El paquete contiene `eidos`, `eidos-gui` y `eidos-nif-preview` en el
nivel superior, junto con `install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, los
iconos en `assets/brand/png`, los avisos de terceros en `licenses/` (nifly, omod,
microsoft-directx, microsoft-runtimes y los crates de Rust) y la carpeta
`runtimes/` que se describe más abajo. No incluye `eidos-fuse` ni `eidos-launch`.
Luego:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` copia los binarios en `~/.local/bin`, o en `$XDG_BIN_HOME` cuando
está definida. `--system` los pone en `/usr/local/bin` mediante sudo, `--bindir
DIR` en cualquier otro sitio, y `--from DIR` toma los binarios de otro
directorio. El script además:

- registra el manejador `nxm://` ejecutando `eidos nxm --register`, así que el
  paso de registro de la sección de Colony no hace falta;
- escribe `~/.local/share/applications/eidos.desktop` y los iconos de Eidos en
  `~/.local/share/icons/hicolor`;
- copia los avisos de licencia a `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` con `--system`);
- copia `runtimes/` a `~/.local/share/Colony/Eidos/runtimes`, también con
  `--system`, de modo que los runtimes pertenecen al usuario que ejecuta el
  script;
- termina imprimiendo la opción de lanzamiento de Steam con la ruta absoluta del
  `eidos-gui` que instaló.

`--cap` pide a sudo que conceda `CAP_SYS_ADMIN` al `eidos` instalado (consulta el
passthrough de FUSE más abajo). `--no-cap` es el valor por defecto y lo deja sin
privilegios. Volver a ejecutar el script es la forma prevista de actualizar. El
paquete y `eidos-linux` necesitan Linux x86_64 con glibc 2.39 o posterior.

### Los runtimes de Microsoft del paquete

La carpeta `runtimes/` del paquete contiene los archivos de Microsoft que piden
las herramientas: `VC_redist.x86.exe` y `VC_redist.x64.exe` (Visual C++ 2015-2022,
el prerrequisito `vcrun2022`), `windowsdesktop-runtime-8.0.12-win-x64.exe` (el
.NET 8 Desktop Runtime, para `dotnet8` y `dotnetdesktop8`) y
`dotnet-runtime-10.0.10-win-x64.zip` (el runtime de .NET 10, para `dotnet10`).
Eidos ejecuta él mismo los instaladores de Visual C++ y de .NET 8 en el prefijo
del juego, sin winetricks, y descomprime una sola vez el zip de .NET 10 en
`~/.local/share/Colony/Eidos/runtimes`, donde lo comparten todas las instancias
(78 MB descomprimido). Cada archivo se usa solo cuando su SHA-256 coincide con el
que Eidos tiene fijado. El SDK de .NET 10 que necesita Synthesis (`dotnetsdk10`,
unos 300 MB) no está en el paquete y siempre se descarga.

Un paquete descomprimido también funciona en el sitio: Eidos busca `runtimes/`
junto a su propio binario después de la carpeta de datos de arriba.

## Arch

```bash
cd packaging && makepkg -si
```

El PKGBUILD compila la rama `main` actual desde el código fuente. Instala `eidos`,
`eidos-gui`, `eidos-nif-preview`, `eidos-fuse` y `eidos-launch` en `/usr/bin`, con
un manejador `nxm://` para todo el sistema, una entrada de escritorio y los
iconos. Depende de `fuse3`, `7zip` y `xdg-utils`. No trae carpeta `runtimes/`, así
que los runtimes de Microsoft se descargan cuando una herramienta los necesita.
`winetricks` es opcional: Eidos lo sigue usando para los prerrequisitos que no
instala por sí mismo, como `dotnet48` y `xact`.

## Desde el código fuente

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Compilar desde el código fuente requiere Rust 1.92 o posterior, `just`, CMake
3.20+, un compilador C++17 y Python 3 para las pruebas del programa auxiliar
nativo. `just build` compila todo el workspace, así que `eidos-fuse` y
`eidos-launch` también se compilan, y `just install` ejecuta `packaging/install.sh`
sobre el resultado, que los copia junto con el resto. Para usar órdenes sin
`just`, consulta las
[instrucciones del programa auxiliar nativo](../../../../../crates/eidos-nif-preview/README.md),
compila Rust con `cargo build --release --locked`, copia el programa auxiliar a
`target/release` y ejecuta `packaging/install.sh --from target/release`. Una
compilación desde el código fuente tampoco trae carpeta `runtimes/`, así que los
runtimes de Microsoft se descargan cuando una herramienta los necesita.

El perfil release usa una unidad de generación de código, LTO completo y símbolos
eliminados. Con rustc 1.99.1 y 3 trabajos de compilación, una compilación en frío
de `eidos` y `eidos-gui` tardó 334 s, frente a 243 s con LTO ligero, y dio un
`eidos` de 20,9 MB y un `eidos-gui` de 49,7 MB. Cada binario incorpora ocho DLL de
DirectX guardadas comprimidas con gzip: 7,9 MB en lugar de 17,9 MB.
`just profile=dev build` compila sin optimización.

## Después: apuntar Steam hacia él

Eidos se ejecuta *como* la orden de lanzamiento de tu juego, que es como llega a
montar antes de que el juego arranque. En Steam, clic derecho en el juego ->
Propiedades -> Opciones de lanzamiento, e introduce la ruta absoluta de la ventana
de Eidos seguida de `%command%`:

```
~/.local/bin/eidos-gui %command%
```

Usa la ruta absoluta: Steam no lee el `PATH` de tu shell. La ruta depende de cómo
instalaste Eidos, e `install.sh` imprime la línea del directorio que usó:

| Instalación | Opción de lanzamiento |
|---|---|
| Paquete de versión o `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Paquete de Arch | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Pulsa Jugar. Eidos se abre en la instancia de ese juego; instala mods, ordena con
LOOT, pulsa Run. Al salir, el montaje se va con él y tu instalación queda
exactamente como estaba.

El juego siempre arranca a través de su tienda. En una ventana que abriste tú
mismo, Run ejecuta `steam -applaunch <appid>`, Steam arranca Eidos mediante la
opción de lanzamiento y se abre una segunda ventana de Eidos donde pulsas Run.
Mientras la opción de lanzamiento no arranque Eidos, Run no arranca nada y la
línea de estado muestra la línea que hay que poner. Con Start the game at once
when a store opens Eidos (Preferences, General, Running a game; desactivado por
defecto), Jugar en Steam arranca el juego sin la ventana cuando Health no lista
ningún problema. Heroic y Legendary funcionan igual mediante su ajuste de
wrapper; [usage.md](usage.md#lanzar-el-juego) tiene los detalles.

La pestaña Diagnostics comprueba la línea. Una tarjeta Health muestra la línea que
hay que poner cuando la opción de lanzamiento está vacía, no nombra ningún
binario de Eidos, apunta a un archivo que no existe, no tiene `%command%` después
de Eidos, o arranca otra copia de Eidos distinta de la que se está ejecutando.

### Si prefieres la terminal

```sh
eidos init skyrimse               # crear una instancia (añade una carpeta para hacerla portátil)
eidos install skyrimse mod.7z     # mods Simple / FOMOD / BAIN / root
eidos sort skyrimse               # ordenar la carga con LOOT
eidos check skyrimse              # las comprobaciones Health que no necesitan ventana
eidos play skyrimse -- %command%  # ejecutar lo que sea a través de la vista combinada
eidos shell skyrimse              # una shell en la vista combinada, lo escrito va al Overwrite
```

Toda orden que acepta un identificador de juego acepta también la carpeta de una
instancia portátil; consulta
[usage.md](usage.md#instancias-globales-y-portátiles). El recorrido completo está
en [usage.md](usage.md).

## Actualizar

Colony actualiza `eidos-linux` a través de su lanzador. Con el paquete de versión,
ejecuta el `install.sh` del paquete nuevo; con el paquete de Arch, ejecuta
`makepkg -si` otra vez; con una compilación desde el código fuente, haz pull y
ejecuta `just install`. Al arrancar, la ventana pregunta a GitHub por la última
versión, como mucho una vez al día, y cuando hay una más nueva muestra un enlace a
sus notas de versión después de Help en la barra de menús. Nunca descarga ni se
sustituye a sí misma. Preferences, About, Version desactiva la comprobación, el
modo sin conexión la omite, y una copia instalada bajo `/usr` (el paquete de Arch)
nunca pregunta, porque la actualiza el gestor de paquetes.

## Opcional: passthrough de FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` concede la capacidad
opcional; no activa el passthrough. `./install.sh --cap` y `just setcap` la
conceden al `eidos` que instalan o compilan. En un montaje `nosuid` el núcleo
ignora las capacidades de archivo, así que `install.sh` omite la concesión con un
aviso y `just setcap` se detiene con un error. La capacidad va en el binario
`eidos` separado del paquete de versión, del paquete de Arch o de una compilación
desde el código fuente, nunca en `eidos-linux`, que también es la ventana
([SECURITY.md](../../../../../SECURITY.md)). Un binario recompilado o reinstalado
es un archivo nuevo sin la capacidad, así que vuelve a concederla tras cada
actualización si la usas.

`EIDOS_FUSE_PASSTHROUGH=1` es el interruptor independiente durante la ejecución, y
está desactivado por defecto. Con el passthrough activo, una versión anterior del
demonio impedía que Skyrim SE cargara contenido de mods. La causa más probable es
que registraba un archivo de respaldo nuevo en cada apertura, mientras que el
núcleo acepta uno solo por inodo. El demonio ahora comparte un respaldo entre
todas las aperturas en passthrough de un archivo, pero eso aún no se ha vuelto a
medir en un juego. Hasta entonces, activa el passthrough solo para probarlo. Las
mediciones y la causa están en [troubleshooting.md](troubleshooting.md).

## ¿Algo va mal ya?

[troubleshooting.md](troubleshooting.md) cubre los interruptores de entorno, cómo
leer los contadores de operaciones, las tarjetas Health y todos los problemas que
han mordido a alguien hasta ahora.
