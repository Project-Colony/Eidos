<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Нативний менеджер модів для Linux, який ніколи не торкається вашої гри.**

</div>

Eidos дає іграм Bethesda під Linux те, що Mod Organizer 2 дає їм під Windows -
віртуальний об'єднаний вигляд ваших модів, що будується при кожному запуску, -
зроблений із примітивів Linux, а не з перехоплення Windows API. Жодного Wine для
самого менеджера. Жодних файлів, скопійованих у теку гри. Жодного шляху
очищення, бо очищати нічого.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Стан:** у Skyrim SE грають через Eidos щодня - SKSE, preloader'и script
> extender, Creation Club, відсортовані LOOT порядки завантаження, збереження на
> кожен профіль, усе це. Поки що одне сімейство ігор перевірене реальною грою;
> ще десять під'єднані й чекають на тестувальників.

## Чому Eidos

- 🔒 **Монтування, яке бачить лише ваша гра.** Об'єднаний вигляд живе у
  приватному mount namespace: ваш файловий менеджер, ваше резервне копіювання,
  друга гра - жоден із них його не бачить і жодному не потрібен дозвіл на нього.
  Вбийте гру, висмикніть живлення: namespace помирає разом із деревом процесів, а
  ваша інсталяція точно така, якою була. Залишків немає *за побудовою*.
- 🧾 **Одна копія істини.** Ваш профіль володіє своїм списком модів, порядком
  плагінів, INI та збереженнями. Файли плагінів і тека збережень при запуску
  bind-монтуються поверх власних шляхів гри, тож навіть власні записи гри
  потрапляють у ваш профіль. Зміна профілю змінює все.
- 🐧 **Повністю без root.** Ані setuid-помічника, ані демона, ані `sudo setcap`,
  ані редагування `/etc/fuse.conf`. Один виконуваний файл, один параметр запуску
  Steam.
- 🛡️ **Запобіжники з доказами.** Збій, що понівечив ваш список плагінів,
  позначається за знімком, зробленим до сесії, з відновленням в один клац.
  Захоплення, що стерло б ваш порядок завантаження, відхиляється з поясненням
  причини.

## Що він робить

**Моди.** Прості архіви, майстри FOMOD, пакунки Wrye Bash BAIN, ручний вибір для
решти - і **root-моди нативно** (preloader'и script extender, ENB, Engine
Fixes), без плагіна Root Builder і без жодного файлу, скопійованого у вашу
інсталяцію. Приховування окремих файлів, групування роздільниками, точкові
переміщення, нотатки й категорії на кожен мод, а також імпортер профілів MO2.

Список - як в MO2, з його звичками: вісім необов'язкових стовпців і сортування за
будь-яким із них, групування за категорією чи за джерелом, жести подвійним
клацанням, перехід набором тексту, резервні копії на кожен мод, які нічого не
роблять, доки ви їх не відновите, і попереджувальні позначки для мода, чию
розкладку ця гра не завантажить або який завантажено для іншої.
Його дерево файлів виконує звичайні операції - нова тека, перейменувати,
видалити, відкрити - і показує зображення й текст, нічого не запускаючи.

**Плагіни.** Порядок завантаження з вбудованим сортуванням LOOT, індекси модів
такі, як їх обчислює гра, попередження про відсутні master-файли, а ваші DLC та
вміст Creation Club показані як некеровані рядки, якими вони і є.

**Екземпляри.** Глобальні - керовані централізовано в `~/.local/share/eidos` -
або портативні: самодостатня тека будь-де, де забажаєте (другий диск, ігровий
розділ), переносна й ізольована, як у MO2. Портативні екземпляри
запам'ятовуються між сесіями; GUI, запуск зі Steam і кожна команда CLI йдуть за
тим, яким ви користувалися востаннє, а будь-яка команда приймає теку скрізь, де
приймає ідентифікатор гри. Подробиці в [usage.uk.md](docs/guide/usage.md#екземпляри-глобальні-та-портативні).

**Профілі.** Порядок модів, стан плагінів, INI та збереження на кожен профіль.
Збереження розбираються, звіряються з вашими поточними плагінами - з кнопкою, що
вмикає те, чого збереження потребує, - і після кожної сесії синхронізуються назад
для Steam Cloud.

**Nexus.** Під'єднайте акаунт - і кнопка «Mod Manager Download» на сайті
потрапляє просто до вашого екземпляра, разом із перевіркою оновлень щодо того, що
у вас встановлено, авторством кожного мода й посиланням на його профіль.
Посилання на **колекцію** перелічує її учасників, зіставлених із вашим
екземпляром - встановлені, завантажені, відсутні, - що є читанням колекції, а не
її встановленням, і панель пояснює чому. Вкладка Downloads - це бібліотека
архівів: фільтрувати, сортувати, приховувати без видалення й вичищати вже
встановлені. Перемикач **offline** зупиняє все це.

**Інструменти.** xEdit, BodySlide, DynDOLOD і решта запускаються *через
об'єднаний вигляд* усередині Proton-префікса гри - вони бачать ваші моди, їхній
результат потрапляє в Overwrite, і один клац перетворює його на справжній мод.
Потрібний кожному runtime звантажується на вимогу, тож відсутня DLL - це кнопка,
а не змарнований день. xEdit і його близнюк QuickAutoClean знаходяться за вас -
у теці гри, всередині мода або в теці інструментів, яку ви тримаєте поруч з
іграми, - з уже обраними правильними runtime. Закріплюйте ті, якими
користуєтеся, ховайте ті, якими ні, дайте інструментові власний
Steam AppID, коли він є окремим застосунком Steam, і запишіть ярлик `.desktop`,
який запускає його через об'єднаний вигляд, взагалі не відкриваючи Eidos.

**Діагностика.** Відсутні master-файли, осиротілі архіви, розходження списку
модів, пошкоджені набори плагінів - і, після запуску, те, що власний лог script
extender каже про справді завантажене.

**Де він тримає власні файли.** `~/.config/Colony/Eidos/` - для того, що ви
обрали: налаштування, ваша сесія Nexus, список екземплярів, написані вами
визначення ігор і доповнень, - а логи в `~/.local/state/Colony/Eidos/`.
Розкладка, якою користується кожна програма сімейства Colony. Старіший Eidos
тримав це в `~/.config/eidos/`; перший запуск після оновлення копіює їх туди,
пише про це в лог і лишає стару теку точно такою, якою вона була.

## Порівняння

| | Eidos | MO2 через Wine | Fluorine-Manager | Limo / link-деплоєри |
|---|---|---|---|---|
| Менеджер працює нативно | ✅ | ❌ Windows-застосунок у Wine | ✅ (порт на Qt) | ✅ |
| Тека гри не зачеплена | ✅ завжди | ✅ | ✅ | ❌ у неї пишуться посилання |
| Монтування видиме для | лише гри | лише гри | **усієї системи** | н/д |
| Потрібне очищення після збою | не потрібне, за побудовою | не потрібне | відновлення після застряглого монтування | ручне un-deploy |
| Root-моди (ENB, preloader'и) | ✅ нативно | потрібен плагін | потрібен плагін | частково |
| Потрібні привілеї | немає | немає | редагування `/etc/fuse.conf` | немає |

## Наскільки він швидкий

| | до | тепер |
|---|---|---|
| завантаження збереження | ~20 секунд | **6-7 секунд** |
| читань тек за одну сесію | 5,6 мільйона | 465 тисяч |

Переходи між cell миттєві. Виграш прийшов від того, що вашим модам ставиться
менше питань: пошук одного файлу раніше опитував усі п'ятдесят по черзі, а
перелік однієї теки робив це п'ятдесят разів поспіль. Тепер ні те, ні інше цього
не робить. Виміряно на реальному екземплярі у звичайній грі, а не на бенчмарку.

## Початок

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Далі вкажіть у параметрах запуску вашої гри в Steam
`~/.local/bin/eidos-gui %command%` і натисніть «Грати».

Пакунки Arch і архіви випусків, що потрібно встановити спершу, і шлях через CLI:
**[docs/guide/install.uk.md](docs/guide/install.md)**.

## Параметри запуску Steam

Базового рядка досить для більшості конфігурацій:

```
~/.local/bin/eidos-gui %command%
```

Усе інше - це змінні середовища, складені перед ним, і вони вільно поєднуються:

| Ви хочете... | Поставте попереду |
|---|---|
| DLSS із Community Shaders | `PROTON_ENABLE_NVAPI=1` - без цього DLSS мовчки ніколи не ініціалізується; повний перелік - у [guide/graphics.uk.md](docs/guide/graphics.md) |
| лічильник FPS на екрані | `DXVK_HUD=fps` |
| інтерполяцію кадрів на рівні драйвера, нуль модів (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - ніколи разом із власною генерацією кадрів Community Shaders |
| докладні логи для звіту про ваду | `EIDOS_LOG=debug` (логи сесій потрапляють у `~/.local/state/Colony/Eidos/logs/`) |
| звіт про I/O монтування за сесію | `EIDOS_FUSE_STATS=1` |
| іншу кількість робітників FUSE | `EIDOS_FUSE_THREADS=8` (типово 4; `1` - перше, що варто спробувати, шукаючи ваду конкурентності) |
| закріпити цей запуск за одним портативним екземпляром | `EIDOS_INSTANCE=/path/to/folder` - без цього Eidos відкриває екземпляр, яким ви користувалися востаннє, а це зазвичай і є те, чого ви хочете |

Рядок, який варто лишити для сучасної модифікованої збірки (Community Shaders,
DLSS, генерація кадрів), - це остаточна команда, а не приклад:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Додайте `DXVK_HUD=fps` попереду, поки перевіряєте, що збірка працює, і приберіть,
щойно вона запрацює.

Глибші діагностичні перемикачі (`EIDOS_FUSE_TRACE`, перемикачі бісекції кешу та
індексу, чому `EIDOS_FUSE_PASSTHROUGH` типово вимкнений) - у
[guide/troubleshooting.uk.md](docs/guide/troubleshooting.md).

## Куди далі

| Якщо ви хочете... | |
|---|---|
| встановити його | [guide/install.uk.md](docs/guide/install.md) |
| вивчити CLI та GUI | [guide/usage.uk.md](docs/guide/usage.md) |
| налаштувати xEdit, BodySlide чи DynDOLOD | [guide/tools.uk.md](docs/guide/tools.md) |
| грати у Fallout 4 (F4SE, версії, збій NVIDIA з уламками) | [guide/fallout4.uk.md](docs/guide/fallout4.md) |
| змусити працювати DLSS / генерацію кадрів (Community Shaders) | [guide/graphics.uk.md](docs/guide/graphics.md) |
| полагодити те, що виглядає неправильно | [guide/troubleshooting.uk.md](docs/guide/troubleshooting.md) |
| дізнатися, чому він швидкий, і перевірити самому | [internals/performance.md](../../internals/performance.md) |
| зрозуміти, як він влаштований усередині | [internals/architecture.md](../../internals/architecture.md) |
| зібрати, протестувати, зробити внесок | [internals/contributing.md](../../internals/contributing.md) |
| дізнатися, навіщо він узагалі існує | [project/landscape.md](../../project/landscape.md) |

Мова - це одна тека: `docs/i18n/uk/` повторює структуру кореня репозиторію, тож
посилання між двома перекладеними сторінками збігається з посиланням між їхніми
англійськими оригіналами.

## Мова

Сторінки, потрібні гравцеві, перекладені. **Англійська - канонічна**: якщо
переклад із нею розходиться, правий англійський файл.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

**Усе інше англійською навмисно, а не через недогляд.** `docs/internals/` і
`docs/project/` читають ті, хто читає й сам Rust, а `CHANGELOG.md` генерується.
Їхній переклад - це ще 17 678 слів, які треба тримати чесними, для аудиторії,
якій вони не потрібні.

Кожен переклад несе хеш англійського файлу, з якого його зроблено, і CI падає,
коли англійський іде вперед - див. [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
Переклад, який неможливо повернути в актуальний стан, **видаляється**, а не
лишається на місці: застаріла сторінка так само виглядає авторитетно й видає
команди минулого місяця, а це для читача гірше, ніж бути відісланим до
англійської.

Додати мову - це чотири файли й рядок у цій таблиці; кроки є в
[`docs/internals/contributing.md`](../../internals/contributing.md).

## Підтримувані ігри

**Skyrim SE/AE** - перевірений реальною грою. **Fallout 4** теж під'єднаний від
початку до кінця (F4SE підставляється автоматично, інвалідація архівів, порядок
завантаження із зірочкою, LOOT, збереження `.fos`) - див. [guide/fallout4.uk.md](docs/guide/fallout4.md). Під'єднані через спільний дескриптор гри й
шукають тестувальників: Skyrim LE, Skyrim VR, Enderal SE, Fallout 3, Fallout NV,
Fallout 4 (+ VR), Starfield, Oblivion і Morrowind (останні дві монтують і керують
модами; їхні впорядковані за часовими мітками списки плагінів поки не керовані).

Додати сімейство - це один рядок дескриптора:
[internals/adding-games.md](../../internals/adding-games.md).

## Попередні напрацювання та подяки

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) та
  [usvfs](https://github.com/ModOrganizer2/usvfs) - семантика, яку Eidos
  відтворює, і код, за яким вивчалася відповідність
- [LOOT](https://loot.github.io/) - рушій сортування, через libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) та інші менеджери під Linux -
  доказ, що є спільнота, яка хоче це розв'язати

## Ліцензія

GPL-3.0-or-later. Керування модами належить усім.
