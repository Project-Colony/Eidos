<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Extensiones

Una extensión añade una entrada a Eidos sin formar parte de Eidos. Es un
manifiesto TOML que nombra un programa y, como mucho, ese programa.

Los manifiestos viven en `~/.config/Colony/Eidos/addons/`, un `.toml` por
extensión. Abre la carpeta desde View -> Extensions -> Open folder y pulsa Reload, sin
reiniciar.

`id`, `kind` y `exec` son obligatorios; `name` toma el valor del id si falta.
`exec` nombra el programa. Un nombre sin `/` se busca en `PATH`. Si el programa
no existe, Eidos omite la extensión y su fila en View -> Extensions dice por qué
(`<exec> is not on PATH` o `<exec> is not there`). `games` limita la extensión a
esos identificadores de juego; sin él, la extensión vale para todos los juegos, y
en otro juego su fila dice "not for this game".

Un manifiesto que Eidos rechaza aparece primero en la lista, con el motivo: un
archivo que no se puede leer, que no es TOML válido o al que le falta un campo
obligatorio, un `id` vacío o con un espacio, un `exec` vacío, un `id` que un
manifiesto anterior ya tomó (los archivos se leen por orden de nombre), un `kind`
que Eidos no conoce, o un instalador, vista previa o información de partidas
guardadas sin `protocol = 1` y un nombre de archivo en `extensions`.

## Por qué no se carga nada dentro de Eidos

Mod Organizer 2 carga sus complementos como bibliotecas compartidas y aloja los de
Python mediante Qt. Ninguna de las dos cosas se traslada. Rust no tiene una ABI
estable, así que una biblioteca compartida construida con otro compilador - u otra
bandera de optimización, u otro conjunto de características de una dependencia
común - es comportamiento indefinido, y nada lo señala como un desajuste de versiones. Y los widgets
de Eidos son genéricos en tiempo de compilación, de modo que una biblioteca no
podría construir uno para devolverlo aunque la ABI fuese estable.

Una extensión es un programa que Eidos ejecuta con el acceso al sistema de archivos de tu cuenta. Instala solo programas auxiliares en los que confíes. El aislamiento de procesos y la comprobación de respuestas no proporcionan un entorno aislado del sistema operativo.

## Una herramienta

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # omitir para todos los juegos
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Aparece en **View -> Extensions** con un botón Run y arranca desacoplada: Eidos
no la espera, y la línea de estado indica su pid. Su salida se descarta. Sin
`workdir`, arranca en la carpeta desde la que se lanzó el propio Eidos.

## Una comprobación

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Se ejecuta en cada refresco, en la raíz de la instancia salvo que `workdir` diga
otra cosa, e imprime un hallazgo por línea:

```
level<TAB>title<TAB>detail
```

donde `level` es `problem`, `advice` u `ok`. El detalle es opcional; una línea sin título se descarta. Todo lo que
no empiece por un nivel conocido se ignora, de modo que la salida de progreso y
los avisos sueltos no pueden levantar una fila que parezca una de las
comprobaciones propias de Eidos. Los hallazgos aparecen en la pestaña Diagnostics como
`<name> - <title>`, después de las tarjetas de prefijo y de Nexus y antes de las
demás tarjetas Health.

Una comprobación dispone de tres segundos. La que se pasa se detiene y se informa
como un problema contra sí misma - se ejecuta en el mismo refresco que sigue a
cada clic, así que una colgada congelaría la ventana. El reloj corre hasta que la
comprobación ha terminado y sus tuberías de salida están cerradas, de modo que un
proceso hijo en segundo plano que aún las mantenga abiertas cuenta contra los tres
segundos. Una comprobación que termina con error y no imprime nada se informa del
mismo modo, con la primera línea de su salida de error, y también la que imprime
más de 1 MiB en cualquiera de las dos salidas. En ambos casos Eidos no vuelve a
ejecutarla hasta que pulses Reload, y su fila dice "It will not be retried until
you press Reload." Una comprobación no lee nada de su entrada estándar.

## Marcadores

Tanto `args` como `workdir` expanden estos:

| Marcador        | Qué es                                       |
| --------------- | -------------------------------------------- |
| `{instance}`    | la raíz de la instancia                      |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | el nombre del perfil activo                  |
| `{profile_dir}` | el directorio del perfil activo              |
| `{game}`        | el identificador del juego, p. ej. `skyrimse`|
| `{game_name}`   | el nombre visible del juego                  |
| `{install}`     | la carpeta del juego que usa la instancia    |
| `{data}`        | el directorio `Data` de esa carpeta          |

`{install}` es la instalación de la tienda, o la copia del juego cuando la
instancia usa una (un `Stock Game` congelado o la carpeta de juego propia de una
lista; consulta [usage.md](usage.md#archivos-del-juego-la-instalación-de-la-tienda-o-una-copia)).
`{data}` la sigue.

Una comprobación recibe además `{addon_dir}`, la carpeta que contiene su
manifiesto, igual que las extensiones del protocolo 1 de más abajo. Una
herramienta no.

Un marcador desconocido nunca se vacía, porque eso convertiría `--out {typo}` en
`--out --next-flag`. Eidos se niega a ejecutar una herramienta o una comprobación
cuyos marcadores no se pueden resolver todos, y dice cuáles faltan: en la línea de
estado si es una herramienta, en la fila de la comprobación en la pestaña
Diagnostics si es una comprobación.

## Lo que una extensión no puede hacer

Una extensión no puede hacer llamadas de vuelta a Eidos, registrarse desde un archivo comprimido ni dibujar sus propios controles. Los instaladores estructurados del protocolo 1 pueden devolver planes de archivos comprobados y preguntas con opciones; Eidos controla la revisión, la preparación temporal y la publicación. FOMOD y BAIN siguen siendo instaladores nativos. Las extensiones de vista previa e información de partidas guardadas usan el mismo anfitrión con respuestas limitadas. Consulta la [especificación del protocolo y los ejemplos ejecutables](../../../../guide/examples/extensions/README.md) para conocer la prioridad de coincidencias, las respuestas CLI registradas, los límites y las comprobaciones de identidad al repetir una instalación.
