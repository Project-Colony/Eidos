<!-- eidos-i18n: source=README.md sha=9c6df24d2b5b403d78e0fd5afa489427e7bf9d71 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Нативный менеджер модов для Linux, который никогда не трогает вашу игру.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#установка)

Eidos даёт играм Bethesda под Linux то же, что Mod Organizer 2 даёт им под
Windows - виртуальный объединённый вид ваших модов, собираемый заново при
каждом запуске, - построенный на примитивах Linux, а не на перехвате Windows
API. Никакого Wine для самого менеджера. Ни одного файла, скопированного в
папку игры. Никакой процедуры очистки, потому что очищать нечего.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Состояние:** в Skyrim SE играют через Eidos каждый день - SKSE,
> предзагрузчики script extender, Creation Club, порядки загрузки,
> отсортированные LOOT, сохранения по профилям, всё это. Пока одно семейство
> игр проверено в реальной игре; ещё десять подключены и ждут тестировщиков.

## Почему Eidos

- 🔒 **Монтирование, которое видит только ваша игра.** Объединённый вид живёт в
  приватном пространстве имён монтирования: ваш файловый менеджер, ваша задача
  резервного копирования, вторая игра - никто из них его не видит, никому из
  них не нужно разрешение на него. Убейте игру, выдерните шнур: пространство
  имён умирает вместе с деревом процессов, а ваша установка ровно такая, какой
  была. Остатков не бывает *по построению*.
- 🧾 **Одна копия истины.** Профилю принадлежат его список модов, порядок
  плагинов, INI-файлы и сохранения. Файлы плагинов и папка сохранений
  монтируются через bind поверх собственных путей игры при запуске, поэтому
  даже то, что пишет сама игра, попадает в ваш профиль. Смена профиля меняет
  всё разом.
- 🐧 **Полностью без root.** Ни setuid-хелпера, ни демона, ни `sudo setcap`, ни
  правок `/etc/fuse.conf`. Один исполняемый файл, один параметр запуска Steam.
- 🛡️ **Защита с доказательствами.** Падение, испортившее список плагинов,
  отмечается сравнением со снимком, снятым перед сессией, и восстанавливается
  одним щелчком. Захват, который стёр бы ваш порядок загрузки, отклоняется с
  объяснением причины.

### Сравнение

| | Eidos | MO2 через Wine | Fluorine-Manager | Limo / развёртывание ссылками |
|---|---|---|---|---|
| Менеджер работает нативно | ✅ | ❌ Windows-приложение в Wine | ✅ (порт на Qt) | ✅ |
| Папка игры не тронута | ✅ всегда | ✅ | ✅ | ❌ в неё пишутся ссылки |
| Монтирование видно | только игре | только игре | **всей системе** | н/д |
| Нужна уборка после падения | нет, по устройству | нет | восстановление зависшего монтирования | ручная отмена развёртывания |
| Root-моды (ENB, предзагрузчики) | ✅ нативно | нужен плагин | нужен плагин | частично |
| Нужны привилегии | нет | нет | правка `/etc/fuse.conf` | нет |

### Насколько он быстр

| | было | стало |
|---|---|---|
| загрузка сохранения | ~20 секунд | **6-7 секунд** |
| чтений каталогов за одну сессию | 5,6 миллиона | 465 тысяч |

Смена ячеек мгновенна. Выигрыш получился оттого, что модам стали задавать
меньше вопросов: поиск одного файла раньше опрашивал все пятьдесят по очереди, а
перечисление одной папки делало это пятьдесят раз подряд. Больше ни то, ни
другое так не делает. Измерено на реальной инстанции в обычной игре, а не на
синтетическом тесте.

### Предшественники и благодарности

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) и
  [usvfs](https://github.com/ModOrganizer2/usvfs) - семантика, которую
  воспроизводит Eidos, и кодовая база, по которой изучалась его совместимость
- [LOOT](https://loot.github.io/) - движок сортировки, через libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) и другие менеджеры для Linux -
  доказательство того, что есть сообщество, которое хочет решения этой задачи

## Что он делает

**Моды.** Простые архивы, мастера FOMOD, пакеты BAIN от Wrye Bash, ручной выбор
для всего остального - и **root-моды нативно** (предзагрузчики script extender,
ENB, Engine Fixes), без плагина Root Builder и без единого файла, скопированного
в вашу установку. Скрытие отдельных файлов, группировка разделителями, точечные
перемещения, заметки и категории по модам, импорт профилей MO2.

Список - это список MO2 со всеми его привычками: восемь необязательных столбцов
и сортировка по любому из них, группировка по категории или по источнику, жесты
двойным щелчком, переход набором текста, резервные копии по модам, которые
ничего не делают, пока вы их не восстановите, и предупреждающие флаги для мода,
чью раскладку эта игра не загрузит или который скачан для другой игры. Его
дерево файлов выполняет обычные операции - новая папка, переименовать, удалить,
открыть - и показывает изображения и текст, ничего не запуская.

**Плагины.** Порядок загрузки со встроенной сортировкой LOOT, индексы модов -
такие, какими их считает игра, предупреждения о недостающих мастер-файлах, а
ваши DLC и контент Creation Club показаны теми неуправляемыми строками, какими
они и являются.

**Инстанции.** Глобальные - управляемые централизованно в
`~/.local/share/Colony/Eidos/instances/` - или портативные: самодостаточная папка где угодно
(второй диск, раздел с играми), перемещаемая и изолированная, как в MO2.
Портативные инстанции запоминаются между сессиями; GUI, запуск из Steam и любая
команда CLI следуют за той, которой вы пользовались последней, а любая команда
принимает папку везде, где принимает идентификатор игры. Подробности в
[usage.md](docs/guide/usage.md#инстанции-глобальные-и-портативные).

**Профили.** Порядок модов, состояние плагинов, INI-файлы и сохранения - у
каждого профиля свои. Сохранения разбираются, сравниваются с вашими текущими
плагинами - с кнопкой, включающей то, что нужно сохранению, - и
синхронизируются обратно для Steam Cloud после каждой сессии.

**Nexus.** Подключите аккаунт, чтобы получать ссылки «Mod Manager Download», проверять обновления и просматривать авторов и их профили. Коллекции устанавливаются по своим рецептам с возможностью продолжить прерванный процесс и явными отчётами о неподдерживаемых шагах. Сопоставление проверяет точные исходные файлы; неполные старые метаданные остаются непроверенными. Вкладка загрузок позволяет фильтровать, сортировать, скрывать и удалять архивы. Переключатель **офлайн** отключает доступ к Nexus.

**Инструменты.** xEdit, BodySlide, DynDOLOD и им подобные работают *через
объединённый вид* внутри Proton-префикса игры - они видят ваши моды, их вывод
попадает в Overwrite, и один щелчок превращает его в настоящий мод. Нужный
каждому из них рантайм скачивается по запросу, так что недостающая DLL - это
кнопка, а не полдня возни. xEdit и его двойник QuickAutoClean находятся за вас -
в папке игры, внутри мода или в каталоге инструментов, который вы держите рядом
с играми, - уже с выбранными нужными рантаймами. Закрепляйте те, которыми
пользуетесь, скрывайте те, которыми нет, задавайте инструменту собственный
Steam AppID, если он сам по себе приложение Steam, и создавайте ярлык
`.desktop`, который запускает его через объединённый вид, вообще не открывая
Eidos.

**Диагностика.** Недостающие мастер-файлы, осиротевшие архивы, расхождение
списка модов, повреждённые наборы плагинов - и, после запуска, то, что о
действительно загруженном говорит собственный лог script extender.

**Где он хранит свои файлы.** Такую раскладку использует каждая программа
семейства Colony: `~/.config/Colony/Eidos/` - для того, что выбрали вы:
настройки, ваша сессия Nexus, список инстанций, написанные вами описания игр и
дополнений; `~/.local/share/Colony/Eidos/` - для глобальных инстанций, скачанных
сред выполнения и логов сессий; `~/.cache/Colony/Eidos/` - для мастер-списков
LOOT. Более старый Eidos держал всё это в `~/.config/eidos/`,
`~/.local/share/eidos/` и `~/.local/state/`; первый запуск после обновления
переносит их, пишет об этом в лог и оставляет старые каталоги ровно такими,
какими они были. Пока перенос не завершён, Eidos продолжает пользоваться старыми
файлами. Подробности в [internals/paths.md](../../internals/paths.md) (на
английском).

В разделе [Проверки целостности и созданные файлы](../../../docs/internals/integrity.md) (на английском) описаны защита при замене, конфликты файлов внутри BSA/BA2, статическая диагностика SKSE и записи о генерации, включая их ограничения.

### Поддерживаемые игры

**Skyrim SE/AE** - проверен в реальной игре. **Fallout 4** тоже подключён от
начала до конца (F4SE подставляется автоматически, инвалидация архивов, порядок
загрузки со звёздочками, LOOT, сохранения `.fos`) - см.
[guide/fallout4.md](docs/guide/fallout4.md). Подключены по общему
дескриптору игры и ждут тестировщиков: Skyrim LE, Skyrim VR, Enderal SE,
Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion и Morrowind
(последние два монтируются и управляют модами; их списки плагинов,
упорядоченные по меткам времени, пока не управляются).

Добавить семейство - это одна строка дескриптора:
[internals/adding-games.md](../../internals/adding-games.md).

## Установка

### Через Colony (рекомендуется)

Найдите **Eidos** в [Colony](https://github.com/Project-Colony/Colony) и
установите его. Обновления приходят через лаунчер. Colony устанавливает один
файл, `eidos-linux`: это окно, а также командная строка `eidos`, когда файл
вызывается через ссылку с именем `eidos`. Предпросмотру моделей NIF нужна
вспомогательная программа, которая есть только в описанном ниже наборе; без неё
предпросмотр сообщает об этом, а всё остальное работает. Что делать после
установки и как указать на него Steam:
[docs/guide/install.md](docs/guide/install.md#через-colony).

### Прямая загрузка бинарных файлов

Возьмите файл из [последнего релиза](https://github.com/Project-Colony/Eidos/releases/latest):

| Файл | Что это |
|---|---|
| `eidos-linux` | вся программа в одном файле, в том виде, в каком её устанавливает Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, вспомогательная программа предпросмотра NIF, значки и установщик, который регистрирует ярлык рабочего стола и обработчик загрузок Nexus |

Сначала проверьте загруженный файл, как показано в разделе
[Политика подписи кода](#политика-подписи-кода), затем:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Установщик кладёт всё в `~/.local/bin` без sudo; `--system` и `--bindir DIR`
выбирают другое место. Обоим файлам нужен Linux x86_64 с glibc 2.39 или новее,
FUSE (`fusermount3`) и 7-Zip для архивов модов.

### Сборка из исходников

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Нужен Rust 1.89 или новее (это `rust-version`, который проверяет CI; CI и
релизы собираются на 1.99.0), а также `just`, CMake 3.20+ и компилятор C++17
для вспомогательной программы предпросмотра NIF. Пакет Arch собирается из
`packaging/PKGBUILD`. Что нужно заранее и команды без `just`:
[docs/guide/install.md](docs/guide/install.md).

### Параметры запуска Steam

Eidos запускается *как* команда запуска игры - именно так он успевает
смонтировать вид до её старта. Большинству конфигураций хватает базовой строки
с абсолютным путём к вашей установке (Steam не читает ваш `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Из Colony этот путь - `~/.local/share/Colony/apps/Eidos/eidos-linux`. Затем
нажмите «Играть»: Eidos откроется на инстанции этой игры, а Run запустит игру
через объединённый вид.

Всё остальное - переменные окружения, выставленные перед ней, и они свободно
комбинируются:

| Вам нужно... | Поставьте впереди |
|---|---|
| DLSS с Community Shaders | `PROTON_ENABLE_NVAPI=1` - без неё DLSS молча никогда не инициализируется; полный список - [guide/graphics.md](docs/guide/graphics.md) |
| счётчик кадров на экране | `DXVK_HUD=fps` |
| интерполяция кадров на уровне драйвера, без модов (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - никогда вместе с собственной генерацией кадров Community Shaders |
| подробные логи для отчёта об ошибке | `EIDOS_LOG=debug` (логи сессии попадают в `~/.local/share/Colony/Eidos/logs/`) |
| отчёт по вводу-выводу монтирования за сессию | `EIDOS_FUSE_STATS=1` |
| другое число рабочих потоков FUSE | `EIDOS_FUSE_THREADS=8` (по умолчанию 4; `1` - первое, что стоит попробовать при поиске бага с параллелизмом) |
| закрепить этот запуск за одной портативной инстанцией | `EIDOS_INSTANCE=/path/to/folder` - без неё Eidos открывает инстанцию, которой вы пользовались последней, а это обычно то, что нужно |

Строка, которую стоит оставить для современной сборки с модами (Community
Shaders, DLSS, генерация кадров), - это окончательная команда, а не пример:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Добавьте впереди `DXVK_HUD=fps`, пока проверяете, что всё работает, и уберите,
когда убедитесь.

Более глубокие диагностические переключатели (`EIDOS_FUSE_TRACE`, бисекция кеша
и индекса, почему `EIDOS_FUSE_PASSTHROUGH` выключен по умолчанию) живут в
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Документация

Всё лежит в [docs/](docs/), с указателем по тому, кто читает, в
[docs/README.md](docs/README.md): руководство пользователя (установка,
командная строка и GUI, инструменты вроде xEdit и DynDOLOD, графика, решение
проблем), как Eidos устроен внутри и зачем он существует. Политика безопасности
и как сообщить об уязвимости: [SECURITY.md](../../../SECURITY.md).

## Язык

Страницы, нужные игроку, переведены. **Канонична английская версия**: когда
перевод с ней расходится, прав английский файл.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
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

**Всё остальное на английском намеренно, а не по недосмотру.** `docs/internals/`
и `docs/project/` читают те, кто читает и сам Rust, а `CHANGELOG.md`
генерируется. Их перевод - это ещё 17 678 слов, которые пришлось бы держать
честными ради аудитории, которой они не нужны.

Каждый перевод несёт хеш английского файла, с которого он сделан, и CI падает,
когда английский уходит вперёд, - см. [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
Перевод, который не удаётся привести в актуальное состояние, **удаляется**, а не
оставляется на месте: устаревшая страница по-прежнему выглядит авторитетно и
раздаёт команды месячной давности, что для читателя хуже, чем отправка к
английской версии.

Язык - это один каталог. `docs/i18n/<lang>/` повторяет структуру корня
репозитория, так что `docs/i18n/de/docs/guide/install.md` - это немецкий
`docs/guide/install.md`. Именно благодаря этому ссылка между двумя переведёнными
страницами - ТА ЖЕ строка, что и ссылка между их английскими оригиналами, а
отказ от языка - это один `rm -r`.

## Политика подписи кода

Каждый файл релиза подписывается организацией Project Colony в CI и никогда - на
машине разработчика. Рядом с каждым файлом на странице релиза:

| Файл | Что это |
|---|---|
| `<asset>.sig` | подпись ed25519 над файлом, сделанная ключом релизов организации |
| `<asset>.meta` | три строки, связывающие файл с его именем, его sha256 и версией релиза |
| `<asset>.meta.sig` | подпись ed25519 над `.meta` |

Закрытый ключ - секрет организации; им пользуется только общий
[рабочий процесс sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
в задании, которое ничего не собирает; задания, компилирующие Eidos, его никогда
не видят. Colony проверяет все три файла, прежде чем устанавливать или обновлять
Eidos, и отказывается от релиза старше установленного. Чтобы проверить загрузку
самостоятельно с помощью OpenSSL 3 (те же команды работают для любого файла):

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

Как релизы собираются, подписываются и публикуются (на английском):
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Конфиденциальность

Eidos не отправляет телеметрию, аналитику и отчёты о сбоях. Он не подключается
ни к каким серверам, кроме перечисленных в этой таблице, и только для указанного
там действия. Режим **Offline** (Preferences -> Nexus) прекращает любые
подключения к Nexus.

| Данные | Хранятся или отправляются | Где и зачем |
|---|---|---|
| Настройки, список инстанций, описания игр и дополнений | хранятся | `~/.config/Colony/Eidos/` |
| Инстанции, скачанные среды выполнения, логи сессий | хранятся | `~/.local/share/Colony/Eidos/` или папка, которую вы выбрали для портативной инстанции |
| Мастер-списки LOOT | хранятся | `~/.cache/Colony/Eidos/loot/` |
| Вход в Nexus | отправляется | Вход открывает `users.nexusmods.com` в вашем браузере, который передаёт результат обратно слушателю, открытому Eidos на `127.0.0.1` только на время этого входа. Затем Eidos обменивает его на токены по адресу `users.nexusmods.com/oauth/token` и получает открытые ключи, которыми Nexus их подписывает. [Политика конфиденциальности Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Учётные данные Nexus | хранятся | Токены доступа и обновления, в `~/.config/Colony/Eidos/nexus.ini` с правами 0600. Строка `api_key=`, записанная туда более старым Eidos, сохраняется, но никогда не читается; при желании удалите её. |
| Запросы к Nexus | отправляются | `api.nexusmods.com`, когда вы пользуетесь функцией Nexus: имя вашего аккаунта и статус premium, сведения о модах и файлах, проверки обновлений, коллекции и ссылки на загрузку. Каждый запрос несёт ваш токен доступа и называет Eidos и его версию. |
| Загрузки с Nexus | отправляются | Файлы, которые вы запросили, с узла CDN Nexus, указанного в ссылке на загрузку; сохраняются в папке загрузок инстанции с файлом `.meta` в формате MO2 рядом с каждым. |
| Ссылки `nxm://` | хранятся | `eidos nxm --register`, которую `install.sh` запускает за вас, записывает `~/.local/share/applications/eidos-nxm.desktop` и делает его обработчиком `nxm://` через `xdg-mime`, так что кнопка Mod Manager Download на сайте передаёт свою ссылку Eidos. Пакет Arch устанавливает тот же ярлык для всей системы. |
| Мастер-списки LOOT | отправляются | `raw.githubusercontent.com/loot/...`, при первой сортировке LOOT для игры и всякий раз, когда вы запрашиваете обновление. Ничего вашего не отправляется. [Заявление GitHub о конфиденциальности](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Среды выполнения инструментов | отправляются | Только когда вы устанавливаете зависимость инструмента или она нужна моду: среда выполнения .NET с `builds.dotnet.microsoft.com`, отклоняемая, если она не совпадает с SHA-256, встроенным в Eidos, и `winetricks`, который скачивает то, что устанавливает в Proton-префикс игры (распространяемые пакеты Visual C++ и .NET), с серверов Microsoft и других узлов, которыми пользуется winetricks. [Заявление Microsoft о конфиденциальности](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Логи сессий | хранятся | `~/.local/share/Colony/Eidos/logs/`, права 0600: десять сессий на инстанцию, удаляются через 30 дней. Ваша домашняя папка записывается как `~`, а имя пользователя как `<user>`, так что лог можно вставить в issue как есть. Они никогда не покидают вашу машину, если вы сами их не отправите. |
| Ярлыки | хранятся | `install.sh` записывает `eidos.desktop` и его значки в `~/.local/share`; созданный вами ярлык инструмента попадает в `~/.local/share/applications`. |
| Открываемые ссылки | ни то, ни другое | Страницы модов и другие ссылки открываются в вашем собственном браузере через `xdg-open`. |
| Дополнения | ни то, ни другое | Установленные вами расширения работают как самостоятельные программы на вашей машине; что они делают - их дело. |

## Лицензия

Исходный код самого Eidos распространяется под GPL-3.0-or-later; см.
[LICENSE](../../../LICENSE). Следующие части распространяются под GPL-3.0 без
варианта «or later»:

- переводы скриптов OMOD в `eidos-install`; см.
  [сведения об исходных авторах](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- вспомогательная программа `eidos-nif-preview`, построенная на встроенной
  библиотеке nifly; см. [лицензии и сведения об авторах её компонентов](../../../crates/eidos-nif-preview/UPSTREAM.md);
- крейты `esplugin` и `libloadorder`, с которыми компонуется код порядка
  загрузки и которые объявляют `GPL-3.0`.

Остальные крейты Rust в бинарных файлах сохраняют собственные лицензии (MIT,
Apache-2.0, BSD и подобные). Набор релиза несёт их уведомления в
`licenses/crates/` вместе с указателем всех крейтов и их лицензий.

Бинарные файлы также содержат восемь подлинных распространяемых DLL Microsoft
DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` и `d3dx11_43`, 32- и
64-битные), которые Eidos разворачивает в префикс игры, когда они нужны моду
или инструменту. На них по-прежнему распространяются условия распространения
Microsoft; см. их [происхождение](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Управление модами принадлежит всем.
