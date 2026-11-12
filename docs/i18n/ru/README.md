<!-- eidos-i18n: source=README.md sha=31c66f68ad4e47e896eeb2256fd8f89c68cfdbef -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Нативный менеджер модов для Linux, который никогда не трогает вашу папку игры.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#установка)

Eidos даёт играм Bethesda под Linux то же, что Mod Organizer 2 даёт им под
Windows: объединённый вид ваших модов, собираемый заново при каждом запуске. Он
использует пространства имён монтирования Linux и FUSE вместо перехвата Windows
API, а сам менеджер работает без Wine. В папку игры ничего не копируется, так что
потом нечего очищать. В Eidos есть также описания для Stellar Blade, Stardew
Valley, 7 Days to Die и Crimson Desert.

Копии Steam запускают Eidos через параметр запуска игры. Копии GOG и Epic в Heroic
или Legendary запускают его как обёртку. Игра всегда стартует через магазин,
которому принадлежит копия; сам Eidos её через Proton никогда не запускает.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Состояние:** в Skyrim SE играют через Eidos каждый день, с SKSE,
> предзагрузчиками script extender, Creation Club, порядками загрузки,
> отсортированными LOOT, и сохранениями по профилям. Пока одно семейство игр
> проверено в реальной игре. Ещё десять игр Bethesda и четыре других подключены и
> ждут тестировщиков.

## Почему Eidos

Объединённый вид монтируется в приватном пространстве имён монтирования, которое
видит только дерево процессов игры. Остальные программы, включая вторую игру, его
не видят, и разрешение на него им не нужно. Когда игра завершается, её убивают или
у машины пропадает питание, пространство имён уходит вместе со своими процессами, а
установка остаётся такой, какой была.

Каждый профиль владеет своим списком модов, порядком плагинов, INI и
сохранениями. При запуске файлы плагинов и папка сохранений монтируются через bind
поверх собственных путей игры, так что то, что игра там пишет, попадает в профиль.
Смена профиля меняет всё это сразу.

Eidos не нужны ни setuid-помощник, ни системный демон, ни `sudo setcap`, ни правка
`/etc/fuse.conf`. Демон FUSE работает от вашего пользователя на время одного запуска
и завершается вместе с игрой. Вы устанавливаете одну программу и задаёте один
параметр запуска Steam.

Перед каждым запуском Eidos сохраняет снимок списка плагинов. Если сбой испортил
список, Diagnostics сравнивает его с этим снимком, а Restore the pre-session order
возвращает прежний порядок. Захват, который стёр бы ваш порядок загрузки,
отклоняется с указанием причины.

Test run запускает игру с модами профиля, без модов или только с выбранными модами,
которые включены, плюс их мастерами. Сохранения, изменения INI, состояние плагинов, вывод в Overwrite и
остальные файлы My Games этого запуска, включая журналы, попадают в
`<instance>/test-run/`. Когда игра завершается, Eidos перечисляет эти пять групп, и
вы оставляете или выбрасываете каждую. Остальные записи, например в реестр Wine,
идут туда, куда всегда. Find the mod behind a problem в Diagnostics выполняет
тестовые запуски, каждый раз с половиной подозреваемых модов и нужными им
мастерами. После каждого запуска вы отвечаете, случилась ли проблема, а Eidos
сужает круг подозреваемых, пока не назовёт мод или моды, которым нужны мастера
друг друга; 200 подозреваемых требуют примерно 8 запусков. Включённые моды, выбранные в списке
до начала, остаются включёнными на каждом шаге. Подробности в
[Тестовые запуски](docs/guide/usage.md#тестовые-запуски) и
[Поиск мода, вызывающего проблему](docs/guide/usage.md#поиск-мода-вызывающего-проблему).

### Сравнение

| | Eidos | MO2 через Wine | Fluorine-Manager | Limo / развёртывание ссылками |
|---|---|---|---|---|
| Менеджер работает нативно | да | нет, приложение Windows в Wine | да (порт на Qt) | да |
| Папка игры не тронута | да, всегда | да | да | нет, в неё пишутся ссылки |
| Монтирование видно | только игре | только игре | всей системе | н/д |
| Нужна уборка после падения | нет | нет | восстановление зависшего монтирования | ручная отмена развёртывания |
| Root-моды (ENB, предзагрузчики) | да, нативно | нужен плагин | нужен плагин | частично |
| Нужны привилегии | нет | нет | правка `/etc/fuse.conf` | нет |

### Насколько он быстр

| | было | стало |
|---|---|---|
| загрузка сохранения | ~20 секунд | 6-7 секунд |
| чтений каталогов за одну сессию | 5,6 миллиона | 335 тысяч |

Смена ячеек мгновенна. Поиск одного файла раньше опрашивал по очереди каждый из
пятидесяти модов, а перечисление одной папки делало это пятьдесят раз подряд.
Теперь слои только для чтения индексируются один раз при монтировании, так что
поиск файла - это один поиск по хешу, а объединённый список содержимого папки тоже
вычисляется один раз при монтировании. Цифры получены на реальной инстанции в
обычной игре, а не на синтетическом тесте:
[internals/performance.md](../../internals/performance.md).

### Предшественники и благодарности

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) и
  [usvfs](https://github.com/ModOrganizer2/usvfs): семантика, которую воспроизводит
  Eidos, и кодовая база, по которой изучалась его совместимость
- [LOOT](https://loot.github.io/): движок сортировки, через libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) и другие менеджеры для Linux, которые
  показывают, что есть сообщество, желающее решения этой задачи

## Что он делает

### Моды

Eidos устанавливает простые архивы, мастера FOMOD, пакеты Wrye Bash BAIN,
контейнеры OMOD с поддерживаемыми скриптами OBMM и всё остальное через ручной
выбор. Root-моды (предзагрузчики script extender, ENB, Engine Fixes) работают
нативно, без плагина Root Builder и без копирования чего-либо в вашу установку.
Можно скрывать отдельные файлы, группировать моды разделителями, перемещать
выделение на целевую позицию, вести заметки и категории по каждому моду и
импортировать профиль MO2.

Список работает как в MO2: восемь необязательных столбцов и сортировка по любому из
них, группировка по категории или по источнику, жесты двойного щелчка, переход по
набору первой буквы, резервные копии отдельных модов, которые ничего не делают,
пока вы не восстановите их, и флаги для мода, чью структуру эта игра не загрузит
или который скачан для другой игры. Фильтры отбирают моды по состоянию на Nexus
(одобрен, отслеживается, есть обновление), заметкам, скрытым файлам и виду
содержимого (плагины, архивы, текстуры, меши, скрипты, звук и другое). Дерево
файлов создаёт папки, переименовывает, удаляет и открывает файлы, а также
показывает изображения и текст без запуска чего-либо.

Ctrl+Z и Ctrl+Shift+Z отменяют и возвращают изменения списка модов и порядка
загрузки. Отключение или удаление мода, который использует одно из трёх новейших
сохранений профиля, сначала спрашивает и называет плагины (сохранения Skyrim и
Fallout 4). Переустановка или обновление мода FOMOD открывает мастер с ответами
его последней установки. Информация о моде перечисляет моды, которые ему нужны, и
моды, которым нужен он, по мастерам включённых плагинов.

### Плагины

В порядок загрузки встроена сортировка LOOT, индексы модов в том виде, как их
считает игра, предупреждения об отсутствующих мастерах, а ваш контент DLC и
Creation Club показан как неуправляемые строки. Заголовок считает занятые слоты в
каждом пространстве индексов (full, medium, light). Наведение на плагин показывает
его автора, описание и версию заголовка. В движках Skyrim плагин, чей одноимённый
INI загружает игра, помечается.

Плагины, которые LOOT помечает как грязные, очищаются через QuickAutoClean из xEdit
в мод `Cleaned plugins`, никогда не на месте. Flag as light в новый мод записывает
лёгкую копию плагина и отказывается, когда одно из новейших сохранений загружает
этот плагин как полный (сохранения Skyrim и Fallout 4).

### Инстанции

Инстанция бывает глобальной, хранящейся в `~/.local/share/Colony/Eidos/instances/`,
или портативной: самодостаточная папка там, где вы захотите (второй диск, раздел
для игр), переносимая и изолированная, как в MO2. Портативные инстанции
запоминаются между сеансами. GUI, запуск из Steam и каждая команда CLI следуют за
той, которой вы пользовались последней, а любая команда принимает папку везде, где
принимает идентификатор игры. Подробности в
[usage.md](docs/guide/usage.md#инстанции-глобальные-и-портативные).

Инстанция может также запускать игру из копии папки игры внутри инстанции. Когда
Eidos впервые открывает список Wabbajack, он запускает собственную копию списка:
папку, которую называет его `ModOrganizer.ini`, иначе папку `Stock Game` или
`Game Root`. Freeze this version на карточке Game files в Diagnostics копирует
установку из магазина в `<instance>/Stock Game`, так что обновление магазина больше
не меняет файлы, которые запускает игра. Когда обе папки лежат на одной файловой
системе, умеющей делить блоки, такой как btrfs или XFS, копия делит их с установкой
из магазина (reflink). На ext4 или между двумя файловыми системами это полная копия,
а размер и свободное место показываются заранее. Игру по-прежнему запускает
магазин. Return to the store version и Delete the copy отменяют это, а
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
делает то же в терминале. Morrowind и Oblivion с `bUseMyGamesDirectory=0` нельзя
запускать из копии. Подробности в
[usage.md](docs/guide/usage.md#файлы-игры-установка-из-магазина-или-копия).

### Профили

У каждого профиля свой порядок модов, состояние плагинов, INI и сохранения.
Сохранения разбираются и сравниваются с вашими текущими плагинами, с кнопкой,
которая включает то, что нужно сохранению, и после каждой сессии копируются
обратно в префикс для Steam Cloud. Сохранения, которые Steam Cloud принёс с другого
устройства или которые запуск без Eidos оставил в префиксе, перечислены во вкладке
Saves. Import копирует их в профиль, а сохранение, чьё имя профиль уже использует,
приходит под новым именем с `orphan-` вместо замены.

Профиль может нести переменные запуска и обёртки (`env=NAME=value` и
`wrapper=command`, по одной на строку, в редакторе INI) для запусков своей игры и
инструментов. Для Skyrim SE, Enderal SE и Fallout 4 карточка Display в Diagnostics
задаёт размер и режим окна в копиях INI профиля. Backups хранит точки
восстановления INI профиля, а перед каждым запуском BethINI Pie Eidos делает одну.
Ярлык рабочего стола профиля запускает игру через её магазин. `eidos sessions`
перечисляет записанные запуски и сравнивает один из них с текущим состоянием.

### Nexus

Подключите учётную запись, чтобы получать ссылки Mod Manager Download, проверять
обновления и видеть авторов и их профили. Каждый мод проверяется, одобряется и
ссылается на домен Nexus, с которого пришёл его файл, так что мод Skyrim SE в
инстанции Enderal SE остаётся на сайте Skyrim SE. Обновление следует по цепочке
преемников, заданной автором на Nexus, заменяет папку мода новым файлом (резервная
копия старой предлагается), сохраняет его приоритет и состояние включённости и
воспроизводит ответы FOMOD, сохранённые при установке.
`eidos nexus update <instance> --apply` делает то же в терминале на премиум-
аккаунте, а на бесплатном печатает страницы файлов. Track и Endorse есть в меню
мода, а каждая проверка обновлений синхронизирует их с вашей учётной записью
Nexus. У информации о моде есть вкладка Nexus page с его описанием и списком
изменений в виде обычного текста, а вкладка General заканчивается тем, что
изменило последнее обновление: добавленные, удалённые и изменившие размер файлы и
добавленные или удалённые мастера. Install downloads when they finish (Preferences,
Nexus, по умолчанию выключено) устанавливает загрузку, как только она завершается,
пока окно открыто.

Коллекции устанавливаются по своим рецептам с возобновляемым ходом выполнения и
явными отчётами о неподдерживаемых шагах. Сопоставление проверяет точные исходные
файлы; неполные старые метаданные остаются непроверенными. Панель коллекций
просматривает и ищет коллекции, опубликованные для игры, показывает, когда у
установленной коллекции есть более новая ревизия, и проводит бесплатную учётную
запись по недостающим файлам по одной странице. Когда перетаскивание нарушает
правила порядка коллекции, Diagnostics сообщает об этом, а Restore collection order
возвращает её моды на место. Вкладка Downloads фильтрует, сортирует, скрывает и
удаляет архивы. Offline mode (Preferences, Nexus) останавливает доступ к Nexus и
ежедневную проверку релиза.

### Инструменты

xEdit, BodySlide, DynDOLOD и другие инструменты для модов запускаются через
объединённый вид внутри Wine-префикса игры (Proton-префикса Steam или префикса,
который настроил Heroic). Они видят ваши моды, их вывод попадает в Overwrite, а один
щелчок превращает этот вывод в настоящий мод. Для Skyrim SE Eidos находит SSEEdit,
SSEEdit QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS, BodySlide, DynDOLOD,
TexGen, xLODGen, Synthesis и BethINI Pie в папке игры, внутри мода или в папке
инструментов, которую вы держите рядом с играми, каждый со средами выполнения,
которые ему нужны. У других игр свой список. Pandora, Nemesis, FNIS, BodySlide и
Synthesis пишут вывод в именованный мод (`Nemesis Output` и так далее).

Набор релиза поставляет установщики Visual C++ 2015-2022 и .NET 8 Desktop Runtime от
Microsoft и среду .NET 10. Eidos устанавливает `vcrun2022` и .NET 8 этими
установщиками, без winetricks, а без набора скачивает те же файлы с сайта Microsoft,
проверяя каждый по SHA-256, встроенному в Eidos. Synthesis получает .NET 10 SDK,
загрузку в 300 МБ, которую набор не поставляет. Остальные prerequisites (.NET
Framework 4.8, XACT и более старые версии .NET) по-прежнему идут через winetricks.
Ничего не скачивается, пока вы не попросите установить prerequisites инструмента.

Закрепляйте инструменты, которыми пользуетесь, скрывайте те, которыми нет, задавайте
инструменту собственный Steam AppID, если он сам является приложением Steam, и
записывайте ярлык `.desktop`, запускающий его через объединённый вид без открытия
Eidos. `eidos tool <game> import-mo2` перечисляет исполняемые файлы из
`ModOrganizer.ini` в папке списка, а `--write` их добавляет. Exe во вкладке Data
можно запустить один раз или добавить как инструмент, а
`eidos tool <instance> run --exe <path>` запускает его из терминала. С Ask before
playing Run сначала спрашивает, когда вывод генератора устарел.

Stop завершает запущенную игру или инструмент. Меню File запускает winecfg, regedit,
файловый проводник Wine и деинсталлятор Windows в префиксе игры. Rebuild prefix в
Diagnostics переименовывает префикс, чтобы Steam создал новый, и ничего не
удаляет. `eidos shell <instance>` открывает оболочку в объединённом виде или
выполняет команду, заданную после `--`, а всё, что она пишет, попадает в Overwrite.

Пока окно в фоне, Eidos может отправить уведомление рабочего стола, когда
заканчивается запуск инструмента, установка коллекции, pack или unpack или последняя
запущенная загрузка, и когда не удаётся сортировка LOOT. Notify the desktop when a
long job ends в Preferences включает это; по умолчанию выключено.

### Диагностика

Вкладка Diagnostics перечисляет отсутствующие мастера, осиротевшие архивы, дрейф
списка модов и повреждённые наборы плагинов, а после запуска - что загрузилось по
собственному журналу расширителя скриптов. Она также сообщает о:

- расширителе скриптов, который отсутствует или не соответствует версии игры
- игре, изменившейся с последнего чистого запуска
- параметре запуска Steam, который не запускает Eidos или не содержит `%command%`
  после него
- Proton старее того, что последним настраивал префикс, или Proton из Flatpak Steam
- отсутствующем `PROTON_ENABLE_NVAPI` для DLSS и эксклюзивном полноэкранном
  режиме, который останавливает генерацию кадров Community Shaders
- `enblocal.ini` без `LinuxVersion=true`
- логе краха последней сессии и названных в нём модах
- требованиях Nexus, которые отсутствуют или отключены
- обработчике `nxm://`, которым служит другая программа или отсутствующий файл
- строках `sTestFile`, из-за которых игра игнорирует `plugins.txt`
- включённых плагинах `.esp` в Starfield и неверных или непарных плагинах blueprint
- патче, чьи свободные файлы проигрывают моду, который он патчит
- моде, чьи копии скриптов Papyrus расширителя скриптов побеждают собственные
  скрипты расширителя
- низком `vm.max_map_count` или лимите открытых файлов
- файлах игры на файловой системе Windows
- путях данных BodySlide, которые не указывают на папку игры Data, и папках в
  `Overwrite/Root`, которым место в Data
- `ModOrganizer.ini` списка, который направляет инструменты в другую папку игры
- копии игры, которой нет или в которой нет исполняемого файла игры

`eidos check <instance> [--json]` выполняет проверки, которым не нужно окно, и
завершается с кодом 1, когда находит проблему. Copy report копирует отчёт, где
домашняя папка записана как `~`, а имя пользователя как `<user>`, а Save tree во
вкладке Data записывает объединённое дерево Data с источником каждого файла,
скрытым так же.

В разделе [Проверки целостности и созданные файлы](../../../docs/internals/integrity.md)
(на английском) описаны защита при замене, конфликты файлов внутри BSA/BA2,
статическая диагностика SKSE и квитанции генераторов, включая их ограничения.

### Где он хранит свои файлы

Eidos использует структуру, которой пользуются все программы семейства Colony:
`~/.config/Colony/Eidos/` для того, что вы выбрали (настройки, ваша сессия Nexus,
список инстанций, определения игр и дополнений, которые вы написали),
`~/.local/share/Colony/Eidos/` для глобальных инстанций, сред выполнения
инструментов (скачанных или скопированных из набора релиза через `install.sh`) и
журналов сессий, и `~/.cache/Colony/Eidos/` для masterlist LOOT. Внутри инстанции
`Stock Game` хранит замороженную копию игры, а `test-run/` хранит тестовый запуск,
ожидающий разбора.

Более старый Eidos хранил свои файлы в `~/.config/eidos/`, `~/.local/share/eidos/`
и `~/.local/state/`. Первый запуск после обновления переносит их, сообщает об этом
в журнале и оставляет старые каталоги как были. Пока перенос не завершён, Eidos
продолжает использовать старые файлы. Подробности в
[internals/paths.md](../../internals/paths.md) (на английском).

### Поддерживаемые игры

Skyrim SE/AE проверен в реальной игре. Fallout 4 тоже подключён от начала до конца
(F4SE подставляется автоматически, инвалидация архивов, порядок загрузки со
звёздочками, LOOT, сохранения `.fos`); см. [guide/fallout4.md](docs/guide/fallout4.md).
Подключены и ищут тестировщиков: Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR,
Fallout 3, Fallout NV, Starfield, Oblivion и Morrowind. Fallout 3, Fallout NV,
Oblivion и Morrowind упорядочивают плагины по датам файлов: Eidos хранит один порядок
на профиль и показывает эти даты внутри монтирования, не меняя файлы на диске.

Ещё четыре игры подключены и тоже ищут тестировщиков: Stellar Blade (моды Unreal
`.pak` в `~mods`), Stardew Valley (моды SMAPI, SMAPI устанавливается отдельно),
7 Days to Die (папки `Mods/` рядом с игрой) и Crimson Desert. Crimson Desert
принимает только моды с кодом: плагины ASI, ReShade и DLL-загрузчики в `bin64`. Его
моды данных, которые лежат в группах PAZ, не поддерживаются.

Игра - это один файл TOML, и ваш можно положить в `~/.config/Colony/Eidos/games/`
без пересборки: [internals/adding-games.md](../../internals/adding-games.md).

## Установка

### Через Colony (рекомендуется)

Найдите Eidos в [Colony](https://github.com/Project-Colony/Colony) и установите его.
Обновления приходят через лаунчер. Colony устанавливает один файл, `eidos-linux`:
окно, а также командную строку `eidos`, когда он вызван через ссылку с именем
`eidos`. В этом файле нет ни вспомогательной программы предпросмотра моделей NIF, ни
сред выполнения Microsoft из набора ниже. Без вспомогательной программы
предпросмотр сообщает об этом, а всё остальное работает, а среду выполнения Eidos
скачивает с сайта Microsoft, когда вы устанавливаете prerequisites инструмента. Что
делать после установки и как указать на него Steam:
[docs/guide/install.md](docs/guide/install.md#через-colony).

### Прямая загрузка бинарных файлов

Возьмите файл из [последнего релиза](https://github.com/Project-Colony/Eidos/releases/latest):

| Файл | Что это |
|---|---|
| `eidos-linux` | вся программа в одном файле, как её устанавливает Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, вспомогательная программа предпросмотра NIF, установщики Visual C++ 2015-2022 и .NET 8 Desktop Runtime и среда .NET 10, значки, уведомления о лицензиях и установщик, регистрирующий запись рабочего стола и обработчик загрузок Nexus |

Сначала проверьте загрузку, как показано в разделе
[Политика подписи кода](#политика-подписи-кода), затем:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Установщик кладёт бинарные файлы в `~/.local/bin` без sudo и копирует `runtimes/` из
набора в `~/.local/share/Colony/Eidos/runtimes`. `--system` и `--bindir DIR` кладут
бинарные файлы в другое место, а `--from DIR` берёт их из другой папки. `--cap`
просит sudo выдать `CAP_SYS_ADMIN`, который нужен только экспериментальному FUSE
passthrough, и passthrough остаётся выключенным; `--no-cap` действует по умолчанию.
Обоим файлам нужен Linux x86_64 с glibc 2.39 или новее, FUSE (`fusermount3`) и 7-Zip
для архивов модов.

Eidos сам не обновляется. Раз в сутки при запуске окно спрашивает у GitHub последний
релиз, а когда вышел более новый, показывает ссылку на его заметки о выпуске после
Help в строке меню. Preferences отключает это, а копия в `/usr` вне `/usr/local`,
которую обновляет менеджер пакетов, никогда не спрашивает.

### Сборка из исходников

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Нужны Rust 1.92 или новее (`rust-version`, который проверяет CI; CI и релизы
собираются на 1.99.1), а также `just`, CMake 3.20+ и компилятор C++17 для
вспомогательной программы предпросмотра NIF. `just build` использует профиль
release: один codegen unit и fat LTO. Холодная сборка `eidos` и `eidos-gui` на 3
задачах сборки заняла так 334 секунды, а с thin LTO 243 секунды, зато fat LTO делает
`eidos` на 1,2 МБ, а `eidos-gui` на 3,8 МБ меньше (измерения в `Cargo.toml`).
`just profile=dev build` компилирует быстрее. Пакет Arch собирается из
`packaging/PKGBUILD`. Prerequisites и команды без `just`:
[docs/guide/install.md](docs/guide/install.md).

### Параметры запуска Steam

Eidos запускается как команда запуска игры, и именно так он успевает смонтировать
вид до её старта. Базовой строки большинству настроек достаточно, с абсолютным
путём вашей установки (Steam не читает ваш `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Из Colony этот путь - `~/.local/share/Colony/apps/Eidos/eidos-linux`. Затем нажмите
Play: Eidos откроется на инстанции этой игры, а Run запустит игру через объединённый
вид.

В окне, которое вы открыли сами, Run просит Steam запустить игру
(`steam -applaunch <appid>`), так что параметр запуска по-прежнему действует, а Run
вы нажимаете в окне, которое открывает Steam. Heroic получает ссылку
`heroic://launch`, а Legendary выполняет `legendary launch <app>`, когда Eidos задан
там обёрткой игры. Когда настройки магазина нет, Run ничего не запускает и говорит,
что задать: [guide/usage.md](docs/guide/usage.md#запуск-игры). При включённом Start
the game at once when a store opens Eidos (Preferences, по умолчанию выключено) Eidos
выполняет проверки Diagnostics и запускает игру, не показывая окна, когда ничто не
требует внимания.

Всё остальное - переменные окружения, ставящиеся перед ним, и они свободно
сочетаются:

| Вы хотите... | Поставьте перед |
|---|---|
| DLSS с Community Shaders | `PROTON_ENABLE_NVAPI=1`. Без него DLSS никогда не инициализируется и ничего не говорит; полный список проверок - [guide/graphics.md](docs/guide/graphics.md) |
| счётчик FPS на экране | `DXVK_HUD=fps` |
| интерполяция кадров на уровне драйвера, без модов (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, никогда вместе с собственной генерацией кадров Community Shaders |
| подробные журналы для отчёта об ошибке | `EIDOS_LOG=debug` (журналы сессий попадают в `~/.local/share/Colony/Eidos/logs/`) |
| отчёт ввода-вывода монтирования по сессии | `EIDOS_FUSE_STATS=1` |
| другое число рабочих потоков FUSE | `EIDOS_FUSE_THREADS=8` (по умолчанию 4; `1` - первое, что стоит попробовать при поиске ошибки конкурентности) |
| этот запуск, привязанный к одной портативной инстанции | `EIDOS_INSTANCE=/path/to/folder`. Без него Eidos открывает инстанцию, которой вы пользовались последней |
| окно для одного запуска при включённом Start the game at once | `EIDOS_WINDOW=1` |

Строка, которую стоит сохранить для современной настройки с модами (Community Shaders,
DLSS, генерация кадров), - это итоговая команда, а не пример:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Добавьте `DXVK_HUD=fps` перед ней, пока проверяете, что настройка работает, и уберите,
когда заработает.

Те же переменные можно положить в переменные запуска профиля вместо строки Steam, что
хранит их по профилям и применяет и к запускам инструментов:
[guide/usage.md](docs/guide/usage.md#переменные-и-обёртки-по-профилям).

Более глубокие диагностические переключатели (`EIDOS_FUSE_TRACE`, переключатели
отладки кэша и индекса, причина, по которой `EIDOS_FUSE_PASSTHROUGH` по умолчанию
выключен) описаны в [guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Документация

Всё лежит в [docs/](docs/), с указателем по тому, кто читает, в
[docs/README.md](docs/README.md): руководство пользователя (установка,
командная строка и GUI, инструменты вроде xEdit и DynDOLOD, графика, решение
проблем), как Eidos устроен внутри и зачем он существует. Политика безопасности
и как сообщить об уязвимости: [SECURITY.md](../../../SECURITY.md).

## Язык

Страницы, нужные игроку, переведены. Каноничен английский: когда перевод с ним
расходится, прав английский файл.

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

Всё остальное на английском. `docs/internals/` и `docs/project/` читают те, кто читает
и сам Rust, `CHANGELOG.md` генерируется, а `SECURITY.md` тоже не переведён. Перевод
первых двух добавил бы более 28 000 слов, которые пришлось бы держать в актуальном
состоянии ради читателей, которым они не нужны.

Каждый перевод несёт хеш английского файла, с которого он сделан, и CI падает, пока
этот хеш не совпадает с английским; см.
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Затем перевод обновляется
и перештамповывается командой `scripts/i18n-check.sh --fix`. Устаревший перевод
обновляется, а не удаляется.

Язык - это один каталог. `docs/i18n/<lang>/` повторяет структуру корня репозитория,
так что `docs/i18n/de/docs/guide/install.md` - это немецкий `docs/guide/install.md`.
Именно поэтому ссылка между двумя переведёнными страницами - та же строка, что и
ссылка между их английскими оригиналами, а отказ от языка - это один `rm -r`.

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

Eidos не отправляет телеметрию, аналитику и отчёты о сбоях. Он подключается только к
серверам из этой таблицы и только для названного там действия. Режим Offline
(Preferences, Nexus) прекращает любые подключения к Nexus и ежедневную проверку
релиза.

| Данные | Хранятся или отправляются | Где и зачем |
|---|---|---|
| Настройки, список инстанций, описания игр и дополнений | хранятся | `~/.config/Colony/Eidos/` |
| Инстанции, среды выполнения инструментов, журналы сессий | хранятся | `~/.local/share/Colony/Eidos/` или папка, которую вы выбрали для портативной инстанции |
| Masterlist LOOT | хранятся | `~/.cache/Colony/Eidos/loot/` |
| Проверка релиза | отправляется | При запуске, не чаще раза в сутки, один запрос к `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos читает только тег релиза и ничего не скачивает. По умолчанию включена, отключается в Preferences, пропускается в режиме Offline и для копии, установленной в `/usr` вне `/usr/local`, которую обновляет менеджер пакетов. [Заявление GitHub о конфиденциальности](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Вход в Nexus | отправляется | Вход открывает `users.nexusmods.com` в вашем браузере, который передаёт результат обратно слушателю, открытому Eidos на `127.0.0.1` только на время этого входа. Затем Eidos обменивает его на токены по адресу `users.nexusmods.com/oauth/token` и получает открытые ключи, которыми Nexus их подписывает. [Политика конфиденциальности Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Учётные данные Nexus | хранятся | Токены доступа и обновления, в `~/.config/Colony/Eidos/nexus.ini` с правами 0600. Строка `api_key=`, записанная туда более старым Eidos, сохраняется, но никогда не читается; при желании удалите её. |
| Запросы к Nexus | отправляются | `api.nexusmods.com`, когда вы пользуетесь функцией Nexus: имя вашей учётной записи и статус premium, сведения о модах и файлах, проверки обновлений, коллекции и ссылки на загрузку. Ваши одобрения и отслеживаемые моды читаются при каждой проверке обновлений и записываются, когда вы нажимаете Endorse или Track. Требования модов, описания и списки изменений, а также поиск коллекций идут на `api.nexusmods.com/v2/graphql`. Каждый запрос несёт ваш токен доступа и называет Eidos и его версию. |
| Загрузки с Nexus | отправляются | Файлы, которые вы запросили, с узла CDN Nexus, указанного в ссылке на загрузку; сохраняются в папке загрузок инстанции с файлом `.meta` в формате MO2 рядом с каждым. |
| Ссылки `nxm://` | хранятся | `eidos nxm --register`, которую `install.sh` запускает за вас, записывает `~/.local/share/applications/eidos-nxm.desktop` и делает его обработчиком `nxm://` через `xdg-mime`, так что кнопка Mod Manager Download на сайте передаёт свою ссылку Eidos. Пакет Arch устанавливает тот же ярлык для всей системы. |
| Masterlist LOOT | отправляются | `raw.githubusercontent.com/loot/...`, при первой сортировке LOOT для игры и всякий раз, когда вы запрашиваете обновление. Ничего вашего не отправляется. [Заявление GitHub о конфиденциальности](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Среды выполнения инструментов | отправляются | Только когда вы устанавливаете prerequisites инструмента и только то, чего не поставил набор релиза (отдельный файл `eidos-linux` и сборка из исходников не поставляют ничего): среда .NET 10 с `builds.dotnet.microsoft.com` и установщики Visual C++ и .NET 8 с `download.visualstudio.microsoft.com`, каждый отклоняется, если не совпадает с SHA-256, встроенным в Eidos; .NET 10 SDK, всегда с `builds.dotnet.microsoft.com` и проверяемый так же; и `winetricks` для остальных prerequisites (.NET Framework 4.8, XACT, .NET 6 и 7), который скачивает то, что устанавливает, с серверов Microsoft и других узлов, которыми пользуется winetricks. [Заявление Microsoft о конфиденциальности](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Журналы сессий | хранятся | `~/.local/share/Colony/Eidos/logs/`, права 0600: десять сессий на инстанцию, удаляются через 30 дней. Ваша домашняя папка записывается как `~`, а имя пользователя как `<user>`, так что журнал можно вставить в issue как есть. Они никогда не покидают вашу машину, если вы сами их не отправите. |
| Записи запусков | хранятся | `<instance>/logs/sessions/`: для каждого запуска игры или инструмента - профиль, включённые моды с их версиями и идентификаторами файлов Nexus, порядок плагинов, контрольная сумма каждого INI профиля, версии игры и Eidos, код выхода и сохранения, которые создал запуск. Хранятся 50 новейших, их читает `eidos sessions`, а `eidos pack` их исключает. Они остаются на вашей машине. |
| Отчёты об обновлениях | хранятся | `<instance>/logs/updates/<mod>.json`: что последнее обновление или Replace изменило в папке этого мода. `eidos pack` их исключает. |
| Ярлыки | хранятся | `install.sh` записывает `eidos.desktop` и его значки в `~/.local/share`; созданный вами ярлык инструмента или профиля попадает в `~/.local/share/applications`. |
| Уведомления рабочего стола | ни то, ни другое | По умолчанию выключены. Когда включены, идут в службу уведомлений вашего рабочего стола по D-Bus (`org.freedesktop.Notifications`) и называют завершившуюся задачу. |
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
64-разрядные), хранящихся в сжатом gzip виде (7,9 МБ вместо 17,9 МБ), которые Eidos
распаковывает и разворачивает в префикс игры, когда они нужны моду или инструменту.
На них по-прежнему распространяются условия распространения Microsoft; см. их
[происхождение](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Набор релиза также несёт в `runtimes/` установщики Visual C++ 2015-2022, установщик
.NET 8 Desktop Runtime и zip среды .NET 10 в том виде, в каком их публикует Microsoft.
Их условия лежат в `licenses/microsoft-runtimes/` (`vc_redist.txt`, `dotnet.txt`).
