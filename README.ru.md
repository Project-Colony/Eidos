<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
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
[usage.ru.md](docs/guide/usage.ru.md#инстанции-глобальные-и-портативные).

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
командную строку: **[docs/guide/install.ru.md](docs/guide/install.ru.md)**.

## Параметры запуска Steam

Большинству конфигураций хватает базовой строки:

```
~/.local/bin/eidos-gui %command%
```

Всё остальное - переменные окружения, выставленные перед ней, и они свободно
комбинируются:

| Вам нужно... | Поставьте впереди |
|---|---|
| DLSS с Community Shaders | `PROTON_ENABLE_NVAPI=1` - без неё DLSS молча никогда не инициализируется; полный список - [guide/graphics.ru.md](docs/guide/graphics.ru.md) |
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
[guide/troubleshooting.ru.md](docs/guide/troubleshooting.ru.md).

## Куда дальше

| Если вы хотите... | |
|---|---|
| установить его | [guide/install.ru.md](docs/guide/install.ru.md) |
| освоить командную строку и GUI | [guide/usage.ru.md](docs/guide/usage.ru.md) |
| настроить xEdit, BodySlide или DynDOLOD | [guide/tools.ru.md](docs/guide/tools.ru.md) |
| играть в Fallout 4 (F4SE, версии, падение из-за обломков на NVIDIA) | [guide/fallout4.ru.md](docs/guide/fallout4.ru.md) |
| заставить работать DLSS / генерацию кадров (Community Shaders) | [guide/graphics.ru.md](docs/guide/graphics.ru.md) |
| починить то, что выглядит неправильно | [guide/troubleshooting.ru.md](docs/guide/troubleshooting.ru.md) |
| узнать, почему он быстрый, и проверить это самому | [internals/performance.md](docs/internals/performance.md) |
| понять, как он устроен внутри | [internals/architecture.md](docs/internals/architecture.md) |
| собрать его, протестировать, внести вклад | [internals/contributing.md](docs/internals/contributing.md) |
| узнать, зачем он вообще существует | [project/landscape.md](docs/project/landscape.md) |

Полный указатель - в [docs/README.ru.md](docs/README.ru.md); политика
безопасности и порядок сообщения об уязвимости - в [SECURITY.md](SECURITY.md).

## Язык

Страницы, нужные игроку, переведены. **Канонична английская версия**: когда
перевод с ней расходится, прав английский файл.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**Всё остальное на английском намеренно, а не по недосмотру.** `docs/internals/`
и `docs/project/` читают те, кто читает и сам Rust, а `CHANGELOG.md`
генерируется. Их перевод - это ещё 17 678 слов, которые пришлось бы держать
честными ради аудитории, которой они не нужны.

Каждый перевод несёт хеш английского файла, с которого он сделан, и CI падает,
когда английский уходит вперёд, - см. [`scripts/i18n-check.sh`](scripts/i18n-check.sh).
Перевод, который не удаётся привести в актуальное состояние, **удаляется**, а не
оставляется на месте: устаревшая страница по-прежнему выглядит авторитетно и
раздаёт команды месячной давности, что для читателя хуже, чем отправка к
английской версии.

Добавить язык - это четыре файла и строка в этой таблице; шаги описаны в
[`docs/internals/contributing.md`](docs/internals/contributing.md).

## Поддерживаемые игры

**Skyrim SE/AE** - проверен в реальной игре. **Fallout 4** тоже подключён от
начала до конца (F4SE подставляется автоматически, инвалидация архивов, порядок
загрузки со звёздочками, LOOT, сохранения `.fos`) - см.
[guide/fallout4.ru.md](docs/guide/fallout4.ru.md). Подключены по общему
дескриптору игры и ждут тестировщиков: Skyrim LE, Skyrim VR, Enderal SE,
Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion и Morrowind
(последние два монтируются и управляют модами; их списки плагинов,
упорядоченные по меткам времени, пока не управляются).

Добавить семейство - это одна строка дескриптора:
[internals/adding-games.md](docs/internals/adding-games.md).

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
