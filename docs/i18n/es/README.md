<!-- eidos-i18n: source=README.md sha=9c6df24d2b5b403d78e0fd5afa489427e7bf9d71 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**El gestor de mods nativo de Linux que nunca toca tu juego.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#instalación)

Eidos da a los juegos de Bethesda en Linux lo que Mod Organizer 2 les da en
Windows - una vista combinada de tus mods, virtual y rehecha en cada
lanzamiento - construida sobre primitivas de Linux en vez de sobre el enganche
de la API de Windows. Nada de Wine para el gestor. Ningún archivo copiado en el directorio
del juego. Ningún procedimiento de limpieza, porque no hay nada que limpiar.

```
Steam ──> eidos-gui %command% ──> [ espacio de nombres privado ]
                                  │  mods ⊕ juego  ──> lo que el juego ve
                                  └─ muere con el juego; la instalación queda intacta
```

> **Estado:** Skyrim SE se juega a través de Eidos a diario - SKSE,
> precargadores de script extender, Creation Club, órdenes de carga ordenados
> con LOOT, partidas por perfil, todo. Una familia de juegos probada en juego
> real hasta ahora; otras diez están cableadas y esperando probadores.

## Por qué Eidos

- 🔒 **Un montaje que sólo tu juego ve.** La vista combinada vive en un espacio
  de nombres de montaje privado: tu gestor de archivos, tu copia de seguridad,
  un segundo juego - ninguno de ellos lo ve, ninguno necesita permiso para él.
  Mata el juego, corta la corriente: el espacio de nombres muere con el árbol de
  procesos y tu instalación queda exactamente como estaba. No hay residuo *por
  construcción*.
- 🧾 **Una sola copia de la verdad.** Tu perfil es dueño de su lista de mods, su
  orden de plugins, sus INI y sus partidas. Los archivos de plugins y el
  directorio de partidas se montan con bind sobre las rutas propias del juego al
  lanzar, así que incluso las escrituras del propio juego aterrizan en tu perfil.
  Cambiar de perfil lo cambia todo.
- 🐧 **Totalmente sin root.** Sin ayudante setuid, sin demonio, sin
  `sudo setcap`, sin editar `/etc/fuse.conf`. Un binario, una opción de
  lanzamiento de Steam.
- 🛡️ **Salvaguardas con justificante.** Un cierre inesperado que destroza tu
  lista de plugins se señala frente a una instantánea previa a la sesión, con
  restauración en un clic. Una captura que borraría tu orden de carga se rechaza
  y dice por qué.

### Cómo se compara

| | Eidos | MO2 con Wine | Fluorine-Manager | Limo / desplegadores por enlaces |
|---|---|---|---|---|
| El gestor se ejecuta de forma nativa | ✅ | ❌ aplicación Windows en Wine | ✅ (port a Qt) | ✅ |
| Directorio del juego intacto | ✅ siempre | ✅ | ✅ | ❌ se escriben enlaces dentro |
| El montaje lo ve | sólo el juego | sólo el juego | **todo el sistema** | n/a |
| Limpieza necesaria tras un cierre inesperado | ninguna, por diseño | ninguna | recuperación de montajes obsoletos | retirada manual |
| Mods root (ENB, precargadores) | ✅ nativo | requiere complemento | requiere complemento | parcial |
| Privilegios necesarios | ninguno | ninguno | editar `/etc/fuse.conf` | ninguno |

### Su rapidez

| | antes | ahora |
|---|---|---|
| cargar una partida | ~20 segundos | **6-7 segundos** |
| lecturas de directorio en una sesión | 5,6 millones | 465 mil |

Los cambios de celda son inmediatos. La ganancia vino de hacerles menos
preguntas a tus mods: encontrar un archivo interrogaba antes a los cincuenta por
turno, y listar una carpeta lo hacía cincuenta veces seguidas. Ninguna de las dos
cosas lo hace ya. Medido en una instancia real jugada con normalidad, no en un
banco de pruebas.

### Trabajos previos y agradecimientos

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) y
  [usvfs](https://github.com/ModOrganizer2/usvfs) - la semántica que Eidos
  reproduce, y el código con el que se estudió su paridad
- [LOOT](https://loot.github.io/) - el motor de ordenación, vía libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) y los demás gestores de Linux -
  prueba de que hay una comunidad que quiere esto resuelto

## Qué hace

**Mods.** Archivos simples, asistentes FOMOD, paquetes BAIN de Wrye Bash, un
selector manual para el resto - y **los mods root de forma nativa**
(precargadores de script extender, ENB, Engine Fixes), sin el complemento Root
Builder y sin copiar nada en tu instalación. Oculta archivos sueltos, agrupa con
separadores, movimientos dirigidos, notas y categorías por mod, y un importador
de perfiles MO2.

La lista es la de MO2, con sus costumbres: ocho columnas opcionales y ordenación
por cualquiera de ellas, agrupación por categoría o por origen, gestos de doble
clic, escribir para saltar, copias de seguridad por mod inertes hasta que las
restauras, y avisos consultivos para un mod cuya disposición este juego no
cargará o que se descargó para otro. Su árbol de archivos hace las operaciones
ordinarias - carpeta nueva, renombrar, borrar, abrir - y previsualiza imágenes y
texto sin lanzar nada.

**Plugins.** El orden de carga con la ordenación de LOOT integrada, los índices
de mods tal como los calcula el juego, avisos de masters ausentes, y tus DLC y
contenidos de Creation Club mostrados como las filas no gestionadas que son.

**Instancias.** Globales - gestionadas de forma centralizada bajo
`~/.local/share/Colony/Eidos/instances/` - o portátiles: una carpeta autónoma donde quieras (un
segundo disco, una partición de juegos), movible y aislada, como las de MO2. Las
instancias portátiles se recuerdan entre sesiones; la GUI, el lanzamiento de
Steam y toda orden de la línea de órdenes siguen la que usaste por última vez, y
cualquier orden acepta la carpeta allí donde acepta un identificador de juego.
Los detalles en [usage.es.md](docs/guide/usage.md#instancias-globales-y-portátiles).

**Perfiles.** Orden de mods, estado de plugins, INI y partidas por perfil. Las
partidas se analizan, se comparan con tus plugins actuales - con un botón que
activa lo que una partida necesita - y se resincronizan para Steam Cloud después
de cada sesión.

**Nexus.** Conecta una cuenta para recibir enlaces de «Mod Manager Download», comprobar actualizaciones y consultar autores y sus perfiles. Las colecciones se instalan a partir de sus recetas, con progreso reanudable e informes explícitos sobre los pasos no compatibles. La comparación verifica los archivos de origen exactos; los metadatos antiguos incompletos quedan sin verificar. La pestaña Descargas permite filtrar, ordenar, ocultar y eliminar archivos. Un interruptor **sin conexión** detiene el acceso a Nexus.

**Herramientas.** xEdit, BodySlide, DynDOLOD y compañía se ejecutan *a través de
la vista combinada* dentro del prefijo Proton del juego - ven tus mods, su
salida aterriza en Overwrite, y un clic la convierte en un mod real. El runtime
que cada una necesita se obtiene a petición, así que una DLL ausente es un botón
y no una tarde entera. xEdit y su gemelo QuickAutoClean se encuentran por ti - en
la carpeta del juego, dentro de un mod, o en el directorio de herramientas que
guardas junto a tus juegos - con los runtimes correctos ya elegidos. Fija las que
usas, oculta las que no, da a una herramienta su propio
AppID de Steam cuando es su propia aplicación de Steam, y escribe un acceso
directo `.desktop` que la lance a través de la vista combinada sin abrir Eidos en
absoluto.

**Diagnóstico.** Masters ausentes, archivos huérfanos, deriva de la lista de
mods, conjuntos de plugins dañados - y, tras una ejecución, lo que el propio
registro del script extender dice que se cargó realmente.

**Dónde guarda sus propios archivos.** La disposición que usa todo programa de
la familia Colony: `~/.config/Colony/Eidos/` para lo que has elegido -
preferencias, tu sesión de Nexus, tu lista de instancias, las definiciones de
juegos y de extensiones que hayas escrito - `~/.local/share/Colony/Eidos/` para
las instancias globales, los runtimes descargados y los registros de sesión, y
`~/.cache/Colony/Eidos/` para las masterlists de LOOT. Un Eidos más antiguo los
guardaba en `~/.config/eidos/`, `~/.local/share/eidos/` y `~/.local/state/`; el
primer lanzamiento tras actualizar los traslada, lo dice en el registro, y deja
los directorios antiguos exactamente como estaban. Mientras un traslado no haya
terminado, Eidos sigue usando los archivos antiguos. Detalles en
[internals/paths.md](../../internals/paths.md) (en inglés).

Consulta [Comprobaciones de integridad y archivos generados](../../../docs/internals/integrity.md) (en inglés) para conocer la protección al reemplazar mods, los conflictos de archivos BSA/BA2, los diagnósticos estáticos de SKSE y los registros de generación, con sus límites.

### Juegos compatibles

**Skyrim SE/AE** - probado en juego real. **Fallout 4** también está cableado de
punta a punta (F4SE sustituido automáticamente, invalidación de archivos, orden
de carga con asteriscos, LOOT, partidas `.fos`) - ver
[guide/fallout4.es.md](docs/guide/fallout4.md). Cableados según el descriptor
de juego compartido y a la espera de probadores: Skyrim LE, Skyrim VR, Enderal
SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion y Morrowind
(estos dos últimos montan y gestionan mods; sus listas de plugins ordenadas por
marca de tiempo todavía no se gestionan).

Añadir una familia es una fila de descriptor:
[internals/adding-games.md](../../internals/adding-games.md).

## Instalación

### Mediante Colony (recomendado)

Busca **Eidos** en [Colony](https://github.com/Project-Colony/Colony) e
instálalo. Las actualizaciones llegan a través del lanzador. Colony instala un
solo archivo, `eidos-linux`: la ventana, y también la línea de órdenes `eidos`
cuando se llama a través de un enlace llamado `eidos`. La vista previa de
modelos NIF necesita un programa auxiliar que sólo incluye el paquete de más
abajo; sin él, la vista previa lo indica y todo lo demás funciona. Qué hacer
después de instalarlo, y cómo apuntar Steam hacia él:
[docs/guide/install.es.md](docs/guide/install.md#mediante-colony).

### Descarga directa del binario

Descarga un archivo de la [última versión](https://github.com/Project-Colony/Eidos/releases/latest):

| Archivo | Qué es |
|---|---|
| `eidos-linux` | el programa completo en un solo archivo, tal como lo instala Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, el programa auxiliar de vista previa NIF, iconos y un instalador que registra la entrada de escritorio y el manejador de descargas de Nexus |

Comprueba primero la descarga, como muestra la
[Política de firma de código](#política-de-firma-de-código), y luego:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

El instalador lo pone todo en `~/.local/bin` sin sudo; `--system` y
`--bindir DIR` eligen otro sitio. Ambos archivos necesitan Linux x86_64 con
glibc 2.39 o posterior, FUSE (`fusermount3`) y 7-Zip para los archivos de mods.

### Compilar desde el código fuente

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Requiere Rust 1.89 o posterior (la `rust-version` que comprueba CI; CI y las
versiones publicadas compilan con 1.99.0), además de `just`, CMake 3.20+ y un
compilador C++17 para el programa auxiliar de vista previa NIF. Un paquete de
Arch se compila desde `packaging/PKGBUILD`. Requisitos previos y las órdenes sin
`just`: [docs/guide/install.es.md](docs/guide/install.md).

### Opciones de lanzamiento de Steam

Eidos se ejecuta *como* la orden de lanzamiento del juego, que es como llega a
montar antes de que el juego arranque. La línea base es todo lo que necesitan la
mayoría de las instalaciones, con la ruta absoluta de tu instalación (Steam no
lee tu `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Desde Colony, esa ruta es `~/.local/share/Colony/apps/Eidos/eidos-linux`. Luego
pulsa Jugar: Eidos se abre en la instancia de ese juego, y Run inicia el juego a
través de la vista combinada.

Todo lo demás son variables de entorno apiladas por delante, y se combinan
libremente:

| Quieres... | Pon delante |
|---|---|
| DLSS con Community Shaders | `PROTON_ENABLE_NVAPI=1` - sin ella DLSS nunca se inicializa, en silencio; la lista completa está en [guide/graphics.es.md](docs/guide/graphics.md) |
| un contador de FPS en pantalla | `DXVK_HUD=fps` |
| interpolación de fotogramas a nivel de controlador, cero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nunca junto con la generación de fotogramas de Community Shaders |
| registros detallados para un informe de fallo | `EIDOS_LOG=debug` (los registros de sesión aterrizan en `~/.local/share/Colony/Eidos/logs/`) |
| un informe de E/S por sesión desde el montaje | `EIDOS_FUSE_STATS=1` |
| otro número de hilos de FUSE | `EIDOS_FUSE_THREADS=8` (4 por defecto; `1` es lo primero que probar cuando persigues un fallo de concurrencia) |
| este lanzamiento fijado a una instancia portátil | `EIDOS_INSTANCE=/path/to/folder` - sin ella Eidos abre la instancia que usaste por última vez, que suele ser lo que quieres |

La línea que conservar para una instalación moderna con mods (Community Shaders,
DLSS, generación de fotogramas) - esta es la orden final, no un ejemplo:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Añade `DXVK_HUD=fps` delante mientras compruebas que la instalación funciona, y
quítalo cuando lo haga.

Los interruptores de diagnóstico más profundos (`EIDOS_FUSE_TRACE`, los
conmutadores de bisección de caché e índice, por qué `EIDOS_FUSE_PASSTHROUGH`
está desactivado por defecto) están en
[guide/troubleshooting.es.md](docs/guide/troubleshooting.md).

## Documentación

Todo está en [docs/](../../), indexado según quién lee en
[docs/README.md](docs/README.md): la guía de usuario (instalación, la línea de
órdenes y la GUI, herramientas como xEdit y DynDOLOD, gráficos, solución de
problemas), cómo funciona Eidos por dentro, y por qué existe. Política de
seguridad y cómo informar de una vulnerabilidad: [SECURITY.md](../../../SECURITY.md).

## Idioma

Las páginas que un jugador necesita están traducidas. **El inglés es canónico**:
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

**Todo lo demás está en inglés a propósito, no por omisión.** `docs/internals/` y
`docs/project/` los leen personas que además leen el Rust, y `CHANGELOG.md` se
genera. Traducirlos serían 17.678 palabras más que mantener honestas para un
público que no las necesita.

Cada traducción lleva el hash del archivo en inglés del que se hizo, y CI falla
cuando el inglés avanza - ver [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
Una traducción que no se puede volver a poner al día se **borra**, no se deja
donde está: una página caduca sigue pareciendo autoritativa y reparte las órdenes

Un idioma es un solo directorio. `docs/i18n/<lang>/` refleja la raíz del
repositorio, así que `docs/i18n/de/docs/guide/install.md` es el
`docs/guide/install.md` en alemán - lo que hace que un enlace entre dos páginas
traducidas sea EXACTAMENTE la misma cadena que el enlace entre sus originales en
inglés, y que retirar un idioma sea un solo `rm -r`.

## Política de firma de código

Cada archivo de una versión lo firma la organización Project Colony en CI, nunca
en la máquina de un desarrollador. Junto a cada archivo, en la página de la
versión:

| Archivo | Qué es |
|---|---|
| `<asset>.sig` | una firma ed25519 del archivo, hecha con la clave de publicación de la organización |
| `<asset>.meta` | tres líneas que vinculan el archivo a su nombre, su sha256 y la versión publicada |
| `<asset>.meta.sig` | una firma ed25519 del `.meta` |

La clave privada es un secreto de la organización, que sólo usa el
[flujo de trabajo sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
compartido, en un trabajo que no compila nada; los trabajos que compilan Eidos
nunca la ven. Colony comprueba los tres archivos antes de instalar o actualizar
Eidos, y rechaza una versión más antigua que la instalada. Para comprobar tú
mismo una descarga con OpenSSL 3 (las mismas órdenes sirven para cualquier
archivo):

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

Eidos no envía telemetría, ni analíticas, ni informes de fallos. No se conecta a
ningún servidor salvo los de esta tabla, y sólo para la acción que allí se
indica. El modo **sin conexión** (Preferencias -> Nexus) corta toda conexión con
Nexus.

| Datos | Guardados o enviados | Dónde, y por qué |
|---|---|---|
| Preferencias, lista de instancias, definiciones de juegos y de extensiones | guardados | `~/.config/Colony/Eidos/` |
| Instancias, runtimes descargados, registros de sesión | guardados | `~/.local/share/Colony/Eidos/`, o la carpeta que elegiste para una instancia portátil |
| Masterlists de LOOT | guardadas | `~/.cache/Colony/Eidos/loot/` |
| Inicio de sesión en Nexus | enviado | Iniciar sesión abre `users.nexusmods.com` en tu navegador, que devuelve el resultado a un receptor que Eidos abre en `127.0.0.1` sólo para ese inicio de sesión. Eidos lo canjea después por tokens en `users.nexusmods.com/oauth/token` y obtiene las claves públicas con las que Nexus los firma. [Política de privacidad de Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Credenciales de Nexus | guardadas | Los tokens de acceso y de renovación, en `~/.config/Colony/Eidos/nexus.ini` con modo 0600. Una línea `api_key=` escrita allí por un Eidos más antiguo se conserva pero nunca se lee; bórrala si quieres. |
| Peticiones a Nexus | enviadas | `api.nexusmods.com`, cuando usas una función de Nexus: tu nombre de cuenta y tu estado premium, los detalles de mods y archivos, las comprobaciones de actualizaciones, las colecciones y los enlaces de descarga. Cada petición lleva tu token de acceso y nombra a Eidos y su versión. |
| Descargas de Nexus | enviadas | Los archivos que pides, desde el servidor CDN de Nexus que indica el enlace de descarga, guardados en la carpeta de descargas de la instancia con un `.meta` en formato MO2 junto a cada uno. |
| Enlaces `nxm://` | guardados | `eidos nxm --register`, que `install.sh` ejecuta por ti, escribe `~/.local/share/applications/eidos-nxm.desktop` y lo convierte en el manejador de `nxm://` mediante `xdg-mime`, para que el botón «Mod Manager Download» del sitio entregue su enlace a Eidos. El paquete de Arch instala la misma entrada para todo el sistema. |
| Masterlists de LOOT | enviadas | `raw.githubusercontent.com/loot/...`, en la primera ordenación con LOOT de un juego y cada vez que pides una actualización. No se envía nada tuyo. [Declaración de privacidad de GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtimes de herramientas | enviados | Sólo cuando instalas un requisito previo de una herramienta o un mod lo necesita: el runtime de .NET desde `builds.dotnet.microsoft.com`, rechazado salvo que coincida con el SHA-256 integrado en Eidos, y `winetricks`, que descarga lo que instala en el prefijo Proton del juego (redistribuibles de Visual C++ y .NET) desde Microsoft y los demás servidores que usa winetricks. [Declaración de privacidad de Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Registros de sesión | guardados | `~/.local/share/Colony/Eidos/logs/`, modo 0600: diez sesiones por instancia, borradas pasados 30 días. Tu carpeta personal se escribe como `~` y tu nombre de usuario como `<user>`, así que un registro se puede pegar tal cual en una incidencia. Nunca salen de tu máquina salvo que los envíes tú. |
| Accesos directos | guardados | `install.sh` escribe `eidos.desktop` y sus iconos bajo `~/.local/share`; un acceso directo de herramienta que crees va a `~/.local/share/applications`. |
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
DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` y `d3dx11_43`, de 32 y
64 bits), que Eidos despliega en el prefijo del juego cuando un mod o una
herramienta las necesita. Siguen bajo los términos de redistribución de
Microsoft; consulta su [procedencia](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

La gestión de mods es de todos.
