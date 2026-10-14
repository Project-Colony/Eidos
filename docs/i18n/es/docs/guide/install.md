<!-- eidos-i18n: source=docs/guide/install.md sha=40f02a87874c44292179b6dff322d0f623c307ff -->

# Instalar Eidos

Cuatro formas de entrar. Colony instala un solo archivo, `eidos-linux`, que es el programa completo; las demás proporcionan `eidos` (la línea de órdenes), `eidos-gui` y el programa auxiliar de vista previa estática de NIF uno junto a otro, además del manejador `nxm://` que hace que el botón «Mod Manager Download» de Nexus llegue a tu instancia.

## Lo que necesitas primero

| | |
|---|---|
| **Linux con FUSE** | `fusermount3` en tu PATH. Toda distribución actual lo incluye. |
| **Un juego con Proton, lanzado una vez** | Steam sólo crea el prefijo Wine del juego en el primer lanzamiento, y Eidos trabaja dentro de él. |
| **`7z`** | Para instalar archivos de mods. `p7zip` en la mayoría de distribuciones. |

Sin root, sin demonio, sin editar `/etc/fuse.conf` y sin nada que añadir a tus
grupos. Eidos monta dentro de un espacio de nombres privado que pertenece al
proceso del juego.

## Mediante Colony

Busca **Eidos** en [Colony](https://github.com/Project-Colony/Colony) e
instálalo; las actualizaciones llegan a través del lanzador. Colony guarda un
solo archivo, `~/.local/share/Colony/apps/Eidos/eidos-linux`. Ejecútalo y se
abre la ventana. Llamado a través de un enlace llamado `eidos`, el mismo archivo
es la línea de órdenes. Pon uno en tu `PATH` y registra el manejador de
descargas de Nexus:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

La ventana no necesita ningún enlace: cada vez que lanza un juego o una
herramienta, ejecuta su propio archivo como línea de órdenes. La vista previa de
modelos NIF necesita `eidos-nif-preview`, que sólo incluyen el paquete de la
versión y las compilaciones desde el código fuente; sin él, la vista previa lo
indica y nada más se ve afectado. Apunta Steam al propio archivo, como se
explica más abajo.

## Un paquete de versión

Descarga `eidos-linux-bundle.tar.gz` y sus `.sig`, `.meta` y `.meta.sig` desde
la [última versión](https://github.com/Project-Colony/Eidos/releases/latest), y
compruébalos como muestra la
[Política de firma de código](../../README.md#política-de-firma-de-código) del
README. Luego:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Instala en `~/.local/bin` por defecto. `--system` lo pone en `/usr/local/bin`,
`--bindir DIR` en cualquier otro sitio. Volver a ejecutarlo es la forma prevista
de actualizar. El paquete y `eidos-linux` necesitan Linux x86_64 con glibc 2.39
o posterior.

## Arch

```bash
cd packaging && makepkg -si
```

El PKGBUILD compila la rama `main` actual desde el código fuente.

## Desde el código fuente

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Compilar desde el código fuente requiere Rust 1.89 o posterior, `just`, CMake 3.20+, un compilador C++17 y Python 3 para las pruebas del programa auxiliar nativo. Para usar órdenes sin `just`, consulta las [instrucciones del programa auxiliar nativo](../../../../../crates/eidos-nif-preview/README.md), compila Rust con `cargo build --release --locked`, copia el programa auxiliar a `target/release` y ejecuta `packaging/install.sh --from target/release`.

## Después: apuntar Steam hacia él

Eidos se ejecuta *como* la orden de lanzamiento de tu juego, que es como llega a
montar antes de que el juego arranque. En Steam, clic derecho en el juego ->
Propiedades -> Opciones de lanzamiento:

```
~/.local/bin/eidos-gui %command%
```

Desde Colony, la ruta es `~/.local/share/Colony/apps/Eidos/eidos-linux`.
Pulsa Jugar. Eidos se abre en la instancia de ese juego; instala mods, ordena con
LOOT, pulsa Run. Al salir, el montaje se va con él y tu instalación queda
exactamente como estaba.

Usa la ruta absoluta - Steam no lee el `PATH` de tu shell.

### Si prefieres la terminal

```sh
eidos init skyrimse               # crear una instancia (añade una carpeta para hacerla portátil)
eidos install skyrimse mod.7z     # mods Simple / FOMOD / BAIN / root
eidos sort skyrimse               # ordenar la carga con LOOT
eidos play skyrimse -- %command%  # ejecutar lo que sea a través de la vista combinada
```

Toda orden que acepta un identificador de juego acepta también la carpeta de una
instancia portátil - ver [usage.es.md](usage.md#instancias-globales-y-portátiles).
El recorrido completo está en [usage.es.md](usage.md).

## Opcional: passthrough de FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` concede la capacidad opcional, pero no activa el passthrough.
Se aplica al binario `eidos` separado del paquete o de una compilación desde el
código fuente, nunca a `eidos-linux`, que también es la ventana
([SECURITY.md](../../../../../SECURITY.md)).
`EIDOS_FUSE_PASSTHROUGH=1` es el interruptor independiente durante la ejecución. Está **desactivado por defecto y casi con seguridad lo quieres así**:
medido en Skyrim SE, impide que el juego abra sus propios archivos y complementos,
de modo que los mods no se cargan, en silencio. El interruptor existe para volver
a probar el mecanismo, no porque se recomiende.

Los detalles, y las mediciones detrás de esa decisión, en
[troubleshooting.es.md](troubleshooting.md).

## ¿Algo va mal ya?

[troubleshooting.es.md](troubleshooting.md) cubre los interruptores de
entorno, cómo leer los contadores de operaciones y todos los problemas que han
mordido a alguien hasta ahora.
