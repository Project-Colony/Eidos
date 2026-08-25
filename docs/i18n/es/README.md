<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**El gestor de mods nativo de Linux que nunca toca tu juego.**

</div>

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
`~/.local/share/eidos` - o portátiles: una carpeta autónoma donde quieras (un
segundo disco, una partición de juegos), movible y aislada, como las de MO2. Las
instancias portátiles se recuerdan entre sesiones; la GUI, el lanzamiento de
Steam y toda orden de la línea de órdenes siguen la que usaste por última vez, y
cualquier orden acepta la carpeta allí donde acepta un identificador de juego.
Los detalles en [usage.es.md](docs/guide/usage.md#instancias-globales-y-portátiles).

**Perfiles.** Orden de mods, estado de plugins, INI y partidas por perfil. Las
partidas se analizan, se comparan con tus plugins actuales - con un botón que
activa lo que una partida necesita - y se resincronizan para Steam Cloud después
de cada sesión.

**Nexus.** Conecta una cuenta y el botón «Mod Manager Download» del sitio
aterriza directamente en tu instancia, con comprobación de actualizaciones
frente a lo que tienes instalado, quién hizo cada mod y un enlace a su perfil. Un
enlace de **colección** lista sus miembros cruzados con tu instancia -
instalados, descargados, ausentes - lo que es leer una colección en vez de
instalarla, y el panel dice por qué. La pestaña Downloads es una biblioteca de
archivos: filtrar, ordenar, ocultar sin borrar, y purgar los que ya están
instalados. Un interruptor **sin conexión** lo detiene todo.

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

**Dónde guarda sus propios archivos.** `~/.config/Colony/Eidos/` para lo que has
elegido - preferencias, tu sesión de Nexus, tu lista de instancias, las
definiciones de juegos y de extensiones que hayas escrito - con los registros
bajo `~/.local/state/Colony/Eidos/`. La disposición que usa todo programa de la
familia Colony. Un Eidos más antiguo los guardaba en `~/.config/eidos/`; el
primer lanzamiento tras actualizar los copia, lo dice en el registro, y deja el
directorio antiguo exactamente como estaba.

## Cómo se compara

| | Eidos | MO2 con Wine | Fluorine-Manager | Limo / desplegadores por enlaces |
|---|---|---|---|---|
| El gestor se ejecuta de forma nativa | ✅ | ❌ aplicación Windows en Wine | ✅ (port a Qt) | ✅ |
| Directorio del juego intacto | ✅ siempre | ✅ | ✅ | ❌ se escriben enlaces dentro |
| El montaje lo ve | sólo el juego | sólo el juego | **todo el sistema** | n/a |
| Limpieza necesaria tras un cierre inesperado | ninguna, por diseño | ninguna | recuperación de montajes obsoletos | retirada manual |
| Mods root (ENB, precargadores) | ✅ nativo | requiere complemento | requiere complemento | parcial |
| Privilegios necesarios | ninguno | ninguno | editar `/etc/fuse.conf` | ninguno |

## Su rapidez

| | antes | ahora |
|---|---|---|
| cargar una partida | ~20 segundos | **6-7 segundos** |
| lecturas de directorio en una sesión | 5,6 millones | 465 mil |

Los cambios de celda son inmediatos. La ganancia vino de hacerles menos
preguntas a tus mods: encontrar un archivo interrogaba antes a los cincuenta por
turno, y listar una carpeta lo hacía cincuenta veces seguidas. Ninguna de las dos
cosas lo hace ya. Medido en una instancia real jugada con normalidad, no en un
banco de pruebas.

## Empezar

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Luego pon la opción de lanzamiento de Steam de tu juego a
`~/.local/bin/eidos-gui %command%` y pulsa Jugar.

Paquetes de Arch y archivos de versión, lo que necesitas instalado primero, y la
vía de la línea de órdenes: **[docs/guide/install.es.md](docs/guide/install.md)**.

## Opciones de lanzamiento de Steam

La línea base es todo lo que necesitan la mayoría de las instalaciones:

```
~/.local/bin/eidos-gui %command%
```

Todo lo demás son variables de entorno apiladas por delante, y se combinan
libremente:

| Quieres... | Pon delante |
|---|---|
| DLSS con Community Shaders | `PROTON_ENABLE_NVAPI=1` - sin ella DLSS nunca se inicializa, en silencio; la lista completa está en [guide/graphics.es.md](docs/guide/graphics.md) |
| un contador de FPS en pantalla | `DXVK_HUD=fps` |
| interpolación de fotogramas a nivel de controlador, cero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nunca junto con la generación de fotogramas de Community Shaders |
| registros detallados para un informe de fallo | `EIDOS_LOG=debug` (los registros de sesión aterrizan en `~/.local/state/Colony/Eidos/logs/`) |
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

## A dónde ir ahora

| Si quieres... | |
|---|---|
| instalarlo | [guide/install.es.md](docs/guide/install.md) |
| aprender la línea de órdenes y la GUI | [guide/usage.es.md](docs/guide/usage.md) |
| configurar xEdit, BodySlide o DynDOLOD | [guide/tools.es.md](docs/guide/tools.md) |
| jugar a Fallout 4 (F4SE, versiones, el cierre por restos en NVIDIA) | [guide/fallout4.es.md](docs/guide/fallout4.md) |
| hacer funcionar DLSS / la generación de fotogramas (Community Shaders) | [guide/graphics.es.md](docs/guide/graphics.md) |
| arreglar algo que parece ir mal | [guide/troubleshooting.es.md](docs/guide/troubleshooting.md) |
| saber por qué es rápido, y comprobarlo tú mismo | [internals/performance.md](../../internals/performance.md) |
| entender cómo funciona por dentro | [internals/architecture.md](../../internals/architecture.md) |
| compilarlo, probarlo, contribuir | [internals/contributing.md](../../internals/contributing.md) |
| saber por qué existe siquiera | [project/landscape.md](../../project/landscape.md) |

Un idioma es un solo directorio: `docs/i18n/es/` refleja la raíz del repositorio,
lo que hace que un enlace entre dos páginas traducidas sea idéntico al enlace
entre sus originales en inglés.

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
del mes pasado, lo que para el lector es peor que ser enviado al inglés.

Añadir un idioma son cuatro archivos y una fila en esta tabla;
[`docs/internals/contributing.md`](../../internals/contributing.md) tiene los pasos.

## Juegos compatibles

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

## Trabajos previos y agradecimientos

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) y
  [usvfs](https://github.com/ModOrganizer2/usvfs) - la semántica que Eidos
  reproduce, y el código con el que se estudió su paridad
- [LOOT](https://loot.github.io/) - el motor de ordenación, vía libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) y los demás gestores de Linux -
  prueba de que hay una comunidad que quiere esto resuelto

## Licencia

GPL-3.0-or-later. La gestión de mods es de todos.
