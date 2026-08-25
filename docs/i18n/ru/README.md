<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Нативный менеджер модов для Linux, который никогда не трогает вашу игру.**

</div>

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
`~/.local/share/eidos` - или портативные: самодостаточная папка где угодно
(второй диск, раздел с играми), перемещаемая и изолированная, как в MO2.
Портативные инстанции запоминаются между сессиями; GUI, запуск из Steam и любая
команда CLI следуют за той, которой вы пользовались последней, а любая команда
принимает папку везде, где принимает идентификатор игры. Подробности в
[usage.ru.md](docs/guide/usage.md#инстанции-глобальные-и-портативные).

**Профили.** Порядок модов, состояние плагинов, INI-файлы и сохранения - у
каждого профиля свои. Сохранения разбираются, сравниваются с вашими текущими
плагинами - с кнопкой, включающей то, что нужно сохранению, - и
синхронизируются обратно для Steam Cloud после каждой сессии.

**Nexus.** Подключите аккаунт - и кнопка сайта «Mod Manager Download» попадает
прямо в вашу инстанцию, с проверкой обновлений для того, что у вас установлено,
с автором каждого мода и ссылкой на его профиль. Ссылка на **коллекцию**
показывает её состав, сопоставленный с вашей инстанцией - установлено, скачано,
отсутствует, - то есть чтение коллекции, а не её установка, и панель объясняет
почему. Вкладка Downloads - это библиотека архивов: фильтровать, сортировать,
скрывать без удаления и вычищать уже установленные. Переключатель **офлайн**
останавливает всё это.

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

**Где он хранит свои файлы.** `~/.config/Colony/Eidos/` - для того, что выбрали
вы: настройки, ваша сессия Nexus, список инстанций, написанные вами описания игр
и дополнений, - а логи в `~/.local/state/Colony/Eidos/`. Такую раскладку
использует каждая программа семейства Colony. Более старый Eidos держал всё это
в `~/.config/eidos/`; первый запуск после обновления копирует их, пишет об этом
в лог и оставляет старый каталог ровно таким, каким он был.

## Сравнение

| | Eidos | MO2 через Wine | Fluorine-Manager | Limo / развёртывание ссылками |
|---|---|---|---|---|
| Менеджер работает нативно | ✅ | ❌ Windows-приложение в Wine | ✅ (порт на Qt) | ✅ |
| Папка игры не тронута | ✅ всегда | ✅ | ✅ | ❌ в неё пишутся ссылки |
| Монтирование видно | только игре | только игре | **всей системе** | н/д |
| Нужна уборка после падения | нет, по устройству | нет | восстановление зависшего монтирования | ручная отмена развёртывания |
| Root-моды (ENB, предзагрузчики) | ✅ нативно | нужен плагин | нужен плагин | частично |
| Нужны привилегии | нет | нет | правка `/etc/fuse.conf` | нет |

## Насколько он быстр

| | было | стало |
|---|---|---|
| загрузка сохранения | ~20 секунд | **6-7 секунд** |
| чтений каталогов за одну сессию | 5,6 миллиона | 465 тысяч |

Смена ячеек мгновенна. Выигрыш получился оттого, что модам стали задавать
меньше вопросов: поиск одного файла раньше опрашивал все пятьдесят по очереди, а
перечисление одной папки делало это пятьдесят раз подряд. Больше ни то, ни
другое так не делает. Измерено на реальной инстанции в обычной игре, а не на
синтетическом тесте.

## Начало работы

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Затем поставьте в параметрах запуска игры в Steam
`~/.local/bin/eidos-gui %command%` и нажмите «Играть».

Пакеты Arch и архивы релизов, что нужно установить заранее, и путь через
командную строку: **[docs/guide/install.ru.md](docs/guide/install.md)**.

## Параметры запуска Steam

Большинству конфигураций хватает базовой строки:

```
~/.local/bin/eidos-gui %command%
```

Всё остальное - переменные окружения, выставленные перед ней, и они свободно
комбинируются:

| Вам нужно... | Поставьте впереди |
|---|---|
| DLSS с Community Shaders | `PROTON_ENABLE_NVAPI=1` - без неё DLSS молча никогда не инициализируется; полный список - [guide/graphics.ru.md](docs/guide/graphics.md) |
| счётчик кадров на экране | `DXVK_HUD=fps` |
| интерполяция кадров на уровне драйвера, без модов (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - никогда вместе с собственной генерацией кадров Community Shaders |
| подробные логи для отчёта об ошибке | `EIDOS_LOG=debug` (логи сессии попадают в `~/.local/state/Colony/Eidos/logs/`) |
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
[guide/troubleshooting.ru.md](docs/guide/troubleshooting.md).

## Куда дальше

| Если вы хотите... | |
|---|---|
| установить его | [guide/install.ru.md](docs/guide/install.md) |
| освоить командную строку и GUI | [guide/usage.ru.md](docs/guide/usage.md) |
| настроить xEdit, BodySlide или DynDOLOD | [guide/tools.ru.md](docs/guide/tools.md) |
| играть в Fallout 4 (F4SE, версии, падение из-за обломков на NVIDIA) | [guide/fallout4.ru.md](docs/guide/fallout4.md) |
| заставить работать DLSS / генерацию кадров (Community Shaders) | [guide/graphics.ru.md](docs/guide/graphics.md) |
| починить то, что выглядит неправильно | [guide/troubleshooting.ru.md](docs/guide/troubleshooting.md) |
| узнать, почему он быстрый, и проверить это самому | [internals/performance.md](../../internals/performance.md) |
| понять, как он устроен внутри | [internals/architecture.md](../../internals/architecture.md) |
| собрать его, протестировать, внести вклад | [internals/contributing.md](../../internals/contributing.md) |
| узнать, зачем он вообще существует | [project/landscape.md](../../project/landscape.md) |

Язык - это один каталог: `docs/i18n/ru/` повторяет структуру корня репозитория,
благодаря чему ссылка между двумя переведёнными страницами совпадает со ссылкой
между их английскими оригиналами.

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

Добавить язык - это четыре файла и строка в этой таблице; шаги описаны в
[`docs/internals/contributing.md`](../../internals/contributing.md).

## Поддерживаемые игры

**Skyrim SE/AE** - проверен в реальной игре. **Fallout 4** тоже подключён от
начала до конца (F4SE подставляется автоматически, инвалидация архивов, порядок
загрузки со звёздочками, LOOT, сохранения `.fos`) - см.
[guide/fallout4.ru.md](docs/guide/fallout4.md). Подключены по общему
дескриптору игры и ждут тестировщиков: Skyrim LE, Skyrim VR, Enderal SE,
Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion и Morrowind
(последние два монтируются и управляют модами; их списки плагинов,
упорядоченные по меткам времени, пока не управляются).

Добавить семейство - это одна строка дескриптора:
[internals/adding-games.md](../../internals/adding-games.md).

## Предшественники и благодарности

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) и
  [usvfs](https://github.com/ModOrganizer2/usvfs) - семантика, которую
  воспроизводит Eidos, и кодовая база, по которой изучалась его совместимость
- [LOOT](https://loot.github.io/) - движок сортировки, через libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) и другие менеджеры для Linux -
  доказательство того, что есть сообщество, которое хочет решения этой задачи

## Лицензия

GPL-3.0-or-later. Управление модами принадлежит всем.
