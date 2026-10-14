<!-- eidos-i18n: source=README.md sha=9c6df24d2b5b403d78e0fd5afa489427e7bf9d71 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Нативний менеджер модів для Linux, який ніколи не торкається вашої гри.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#встановлення)

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

### Як він порівнюється

| | Eidos | MO2 через Wine | Fluorine-Manager | Limo / link-деплоєри |
|---|---|---|---|---|
| Менеджер працює нативно | ✅ | ❌ Windows-застосунок у Wine | ✅ (порт на Qt) | ✅ |
| Тека гри не зачеплена | ✅ завжди | ✅ | ✅ | ❌ у неї пишуться посилання |
| Монтування видиме для | лише гри | лише гри | **усієї системи** | н/д |
| Потрібне очищення після збою | не потрібне, за побудовою | не потрібне | відновлення після застряглого монтування | ручне un-deploy |
| Root-моди (ENB, preloader'и) | ✅ нативно | потрібен плагін | потрібен плагін | частково |
| Потрібні привілеї | немає | немає | редагування `/etc/fuse.conf` | немає |

### Наскільки він швидкий

| | до | тепер |
|---|---|---|
| завантаження збереження | ~20 секунд | **6-7 секунд** |
| читань тек за одну сесію | 5,6 мільйона | 465 тисяч |

Переходи між cell миттєві. Виграш прийшов від того, що вашим модам ставиться
менше питань: пошук одного файлу раніше опитував усі п'ятдесят по черзі, а
перелік однієї теки робив це п'ятдесят разів поспіль. Тепер ні те, ні інше цього
не робить. Виміряно на реальному екземплярі у звичайній грі, а не на бенчмарку.

### Попередні напрацювання та подяки

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) та
  [usvfs](https://github.com/ModOrganizer2/usvfs) - семантика, яку Eidos
  відтворює, і код, за яким вивчалася відповідність
- [LOOT](https://loot.github.io/) - рушій сортування, через libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) та інші менеджери під Linux -
  доказ, що є спільнота, яка хоче це розв'язати

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

**Екземпляри.** Глобальні - керовані централізовано в `~/.local/share/Colony/Eidos/instances/` -
або портативні: самодостатня тека будь-де, де забажаєте (другий диск, ігровий
розділ), переносна й ізольована, як у MO2. Портативні екземпляри
запам'ятовуються між сесіями; GUI, запуск зі Steam і кожна команда CLI йдуть за
тим, яким ви користувалися востаннє, а будь-яка команда приймає теку скрізь, де
приймає ідентифікатор гри. Подробиці в [usage.uk.md](docs/guide/usage.md#екземпляри-глобальні-та-портативні).

**Профілі.** Порядок модів, стан плагінів, INI та збереження на кожен профіль.
Збереження розбираються, звіряються з вашими поточними плагінами - з кнопкою, що
вмикає те, чого збереження потребує, - і після кожної сесії синхронізуються назад
для Steam Cloud.

**Nexus.** Під'єднайте обліковий запис, щоб отримувати посилання «Mod Manager Download», перевіряти оновлення та переглядати авторів і їхні профілі. Колекції встановлюються за своїми рецептами з можливістю відновити перерваний процес і явними звітами про непідтримувані кроки. Зіставлення перевіряє точні вихідні файли; неповні старі метадані залишаються неперевіреними. Вкладка завантажень дозволяє фільтрувати, сортувати, приховувати й видаляти архіви. Перемикач **офлайн** припиняє доступ до Nexus.

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

**Де він тримає власні файли.** Розкладка, якою користується кожна програма
сімейства Colony: `~/.config/Colony/Eidos/` - для того, що ви обрали:
налаштування, ваша сесія Nexus, список екземплярів, написані вами визначення
ігор і доповнень; `~/.local/share/Colony/Eidos/` - для глобальних екземплярів,
завантажених середовищ виконання і логів сесій; `~/.cache/Colony/Eidos/` - для
мастер-списків LOOT. Старіший Eidos тримав це в `~/.config/eidos/`,
`~/.local/share/eidos/` і `~/.local/state/`; перший запуск після оновлення
переносить їх, пише про це в лог і лишає старі теки точно такими, якими вони
були. Доки перенесення не завершено, Eidos і далі користується старими файлами.
Подробиці в [internals/paths.md](../../internals/paths.md) (англійською).

У розділі [Перевірки цілісності та створені файли](../../../docs/internals/integrity.md) (англійською) описано захист під час заміни, конфлікти файлів у BSA/BA2, статичну діагностику SKSE та записи генерації разом з їхніми обмеженнями.

### Підтримувані ігри

**Skyrim SE/AE** - перевірений реальною грою. **Fallout 4** теж під'єднаний від
початку до кінця (F4SE підставляється автоматично, інвалідація архівів, порядок
завантаження із зірочкою, LOOT, збереження `.fos`) - див. [guide/fallout4.uk.md](docs/guide/fallout4.md). Під'єднані через спільний дескриптор гри й
шукають тестувальників: Skyrim LE, Skyrim VR, Enderal SE, Fallout 3, Fallout NV,
Fallout 4 (+ VR), Starfield, Oblivion і Morrowind (останні дві монтують і керують
модами; їхні впорядковані за часовими мітками списки плагінів поки не керовані).

Додати сімейство - це один рядок дескриптора:
[internals/adding-games.md](../../internals/adding-games.md).

## Встановлення

### Через Colony (рекомендовано)

Знайдіть **Eidos** у [Colony](https://github.com/Project-Colony/Colony) і
встановіть його. Оновлення приходять через лаунчер. Colony встановлює один файл,
`eidos-linux`: це вікно, а також командний рядок `eidos`, коли його викликають
через посилання з назвою `eidos`. Попередньому перегляду моделей NIF потрібна
допоміжна програма, яку містить лише описаний нижче пакет; без неї перегляд про
це повідомляє, а все інше працює. Що робити після встановлення і як скерувати на
нього Steam: [docs/guide/install.md](docs/guide/install.md#через-colony).

### Пряме завантаження бінарних файлів

Візьміть файл з [останнього випуску](https://github.com/Project-Colony/Eidos/releases/latest):

| Файл | Що це |
|---|---|
| `eidos-linux` | уся програма в одному файлі, так, як її встановлює Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, допоміжна програма перегляду NIF, іконки та інсталятор, що реєструє запис робочого столу й обробник завантажень Nexus |

Спершу перевірте завантаження, як показано в розділі
[Політика підписування коду](#політика-підписування-коду), потім:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Інсталятор кладе все в `~/.local/bin` без sudo; `--system` і `--bindir DIR`
обирають інше місце. Обом файлам потрібен Linux x86_64 з glibc 2.39 або новішою,
FUSE (`fusermount3`) і 7-Zip для архівів модів.

### Збирання з джерельних кодів

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Потрібні Rust 1.89 або новіший (`rust-version`, який перевіряє CI; CI і випуски
збираються з 1.99.0), а також `just`, CMake 3.20+ і компілятор C++17 для
допоміжної програми перегляду NIF. Пакунок Arch збирається з `packaging/PKGBUILD`.
Передумови й команди без `just`: [docs/guide/install.md](docs/guide/install.md).

### Параметри запуску Steam

Eidos виконується *як* команда запуску гри - саме так він монтує до старту гри.
Базового рядка досить для більшості конфігурацій, з абсолютним шляхом до вашої
інсталяції (Steam не читає ваш `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Для Colony цей шлях - `~/.local/share/Colony/apps/Eidos/eidos-linux`. Потім
натисніть «Грати»: Eidos відкриється на екземплярі цієї гри, а Run запускає гру
через об'єднаний вигляд.

Усе інше - це змінні середовища, складені перед ним, і вони вільно поєднуються:

| Ви хочете... | Поставте попереду |
|---|---|
| DLSS із Community Shaders | `PROTON_ENABLE_NVAPI=1` - без цього DLSS мовчки ніколи не ініціалізується; повний перелік - у [guide/graphics.uk.md](docs/guide/graphics.md) |
| лічильник FPS на екрані | `DXVK_HUD=fps` |
| інтерполяцію кадрів на рівні драйвера, нуль модів (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - ніколи разом із власною генерацією кадрів Community Shaders |
| докладні логи для звіту про ваду | `EIDOS_LOG=debug` (логи сесій потрапляють у `~/.local/share/Colony/Eidos/logs/`) |
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

## Документація

Усе лежить у [docs/](../../../docs/), упорядковане за тим, хто читає, у
[docs/README.md](docs/README.md): посібник користувача (встановлення, CLI і GUI,
інструменти на кшталт xEdit і DynDOLOD, графіка, усунення несправностей), як
Eidos влаштований усередині і навіщо він існує. Політика безпеки і як повідомити
про вразливість: [SECURITY.md](../../../SECURITY.md).

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

Мова - це одна тека. `docs/i18n/<lang>/` повторює структуру кореня репозиторію, тож
`docs/i18n/de/docs/guide/install.md` - це німецький `docs/guide/install.md`. Саме
тому посилання між двома перекладеними сторінками є ТІЄЮ САМОЮ стрічкою, що й
посилання між їхніми англійськими оригіналами, і саме тому прибрати мову - це
один `rm -r`.

## Політика підписування коду

Кожен файл випуску підписує організація Project Colony у CI, ніколи не на машині
розробника. Поруч із кожним файлом на сторінці випуску:

| Файл | Що це |
|---|---|
| `<asset>.sig` | підпис ed25519 над файлом, зроблений ключем випусків організації |
| `<asset>.meta` | три рядки, що прив'язують файл до його імені, його sha256 і версії випуску |
| `<asset>.meta.sig` | підпис ed25519 над `.meta` |

Закритий ключ - це секрет організації, яким користується лише спільний
[робочий процес sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
у завданні, яке нічого не збирає; завдання, що компілюють Eidos, ніколи його не
бачать. Colony перевіряє всі три файли, перш ніж встановити чи оновити Eidos, і
відмовляється від випуску, старішого за встановлений. Щоб перевірити завантаження
самостійно за допомогою OpenSSL 3 (ті самі команди працюють для кожного файлу):

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

Як випуски збираються, підписуються і публікуються:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Конфіденційність

Eidos не надсилає ні телеметрії, ні аналітики, ні звітів про збої. Він не
з'єднується з жодним сервером, окрім наведених у цій таблиці, і лише для
названої там дії. Режим **Offline** (Preferences -> Nexus) припиняє будь-які
з'єднання з Nexus.

| Дані | Зберігаються чи надсилаються | Куди і навіщо |
|---|---|---|
| Налаштування, список екземплярів, визначення ігор і доповнень | зберігаються | `~/.config/Colony/Eidos/` |
| Екземпляри, завантажені середовища виконання, логи сесій | зберігаються | `~/.local/share/Colony/Eidos/` або тека, яку ви обрали для портативного екземпляра |
| Мастер-списки LOOT | зберігаються | `~/.cache/Colony/Eidos/loot/` |
| Вхід у Nexus | надсилається | Вхід відкриває `users.nexusmods.com` у вашому браузері, який повертає результат слухачеві, що його Eidos відкриває на `127.0.0.1` лише для цього входу. Потім Eidos обмінює його на токени на `users.nexusmods.com/oauth/token` і отримує відкриті ключі, якими Nexus їх підписує. [Політика конфіденційності Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Облікові дані Nexus | зберігаються | Токени доступу та оновлення, у `~/.config/Colony/Eidos/nexus.ini` з правами 0600. Рядок `api_key=`, записаний туди старішим Eidos, зберігається, але ніколи не читається; за бажання видаліть його. |
| Запити до Nexus | надсилаються | `api.nexusmods.com`, коли ви користуєтеся функцією Nexus: назва вашого облікового запису і статус premium, відомості про моди та файли, перевірки оновлень, колекції та посилання на завантаження. Кожен запит несе ваш токен доступу і називає Eidos та його версію. |
| Завантаження з Nexus | надсилаються | Файли, які ви запитуєте, з хоста CDN Nexus, названого в посиланні на завантаження; зберігаються в теці завантажень екземпляра з `.meta` у форматі MO2 поруч із кожним. |
| Посилання `nxm://` | зберігаються | `eidos nxm --register`, який `install.sh` запускає за вас, пише `~/.local/share/applications/eidos-nxm.desktop` і робить його обробником `nxm://` через `xdg-mime`, тож кнопка Mod Manager Download на сайті передає своє посилання Eidos. Пакунок Arch встановлює той самий запис для всієї системи. |
| Мастер-списки LOOT | надсилаються | `raw.githubusercontent.com/loot/...`, під час першого сортування LOOT для гри і щоразу, коли ви просите оновлення. Нічого вашого не надсилається. [Заява GitHub про конфіденційність](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Середовища виконання інструментів | надсилаються | Лише коли ви встановлюєте передумову інструмента або вона потрібна моду: середовище виконання .NET з `builds.dotnet.microsoft.com`, яке відхиляється, якщо не збігається з SHA-256, вбудованим в Eidos, і `winetricks`, який завантажує те, що встановлює в Proton-префікс гри (розповсюджувані пакети Visual C++ і .NET), з Microsoft та інших хостів, якими користується winetricks. [Заява Microsoft про конфіденційність](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Логи сесій | зберігаються | `~/.local/share/Colony/Eidos/logs/`, права 0600: десять сесій на екземпляр, прибираються через 30 днів. Ваша домашня тека записується як `~`, а ім'я користувача як `<user>`, тож лог можна вставити в issue як є. Вони ніколи не залишають вашу машину, якщо ви самі їх не надішлете. |
| Ярлики | зберігаються | `install.sh` пише `eidos.desktop` і його іконки в `~/.local/share`; ярлик інструмента, який ви створюєте, потрапляє в `~/.local/share/applications`. |
| Посилання, які ви відкриваєте | ні те, ні інше | Сторінки модів та інші посилання відкриваються у вашому власному браузері через `xdg-open`. |
| Доповнення | ні те, ні інше | Встановлені вами розширення працюють як окремі програми на вашій машині; що вони роблять - їхня справа. |

## Ліцензія

Оригінальний код Eidos поширюється за GPL-3.0-or-later; див. [LICENSE](../../../LICENSE).
Ці частини поширюються за GPL-3.0 без варіанта «or later»:

- переклади скриптів OMOD в `eidos-install`; див.
  [відомості про первинних авторів](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- допоміжна програма `eidos-nif-preview`, побудована на вбудованій бібліотеці nifly;
  див. [ліцензії та відомості про авторів її компонентів](../../../crates/eidos-nif-preview/UPSTREAM.md);
- крейти `esplugin` і `libloadorder`, з якими лінкується код порядку
  завантаження і які оголошують `GPL-3.0`.

Інші крейти Rust у бінарних файлах зберігають власні ліцензії (MIT, Apache-2.0,
BSD і подібні). Пакет випуску містить їхні повідомлення в `licenses/crates/`
разом з індексом кожного крейта та його ліцензії.

Бінарні файли також містять вісім справжніх розповсюджуваних DLL Microsoft
DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` і `d3dx11_43`, 32- і
64-бітні), які Eidos розгортає в префікс гри, коли вони потрібні моду чи
інструменту. Вони лишаються під умовами Microsoft для розповсюджуваних
компонентів; див. їхнє [походження](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Керування модами належить усім.
