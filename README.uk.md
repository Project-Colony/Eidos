<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
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
приймає ідентифікатор гри. Подробиці в [usage.uk.md](docs/guide/usage.uk.md#екземпляри-глобальні-та-портативні).

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
**[docs/guide/install.uk.md](docs/guide/install.uk.md)**.

## Параметри запуску Steam

Базового рядка досить для більшості конфігурацій:

```
~/.local/bin/eidos-gui %command%
```

Усе інше - це змінні середовища, складені перед ним, і вони вільно поєднуються:

| Ви хочете... | Поставте попереду |
|---|---|
| DLSS із Community Shaders | `PROTON_ENABLE_NVAPI=1` - без цього DLSS мовчки ніколи не ініціалізується; повний перелік - у [guide/graphics.uk.md](docs/guide/graphics.uk.md) |
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
[guide/troubleshooting.uk.md](docs/guide/troubleshooting.uk.md).

## Куди далі

| Якщо ви хочете... | |
|---|---|
| встановити його | [guide/install.uk.md](docs/guide/install.uk.md) |
| вивчити CLI та GUI | [guide/usage.uk.md](docs/guide/usage.uk.md) |
| налаштувати xEdit, BodySlide чи DynDOLOD | [guide/tools.uk.md](docs/guide/tools.uk.md) |
| грати у Fallout 4 (F4SE, версії, збій NVIDIA з уламками) | [guide/fallout4.uk.md](docs/guide/fallout4.uk.md) |
| змусити працювати DLSS / генерацію кадрів (Community Shaders) | [guide/graphics.uk.md](docs/guide/graphics.uk.md) |
| полагодити те, що виглядає неправильно | [guide/troubleshooting.uk.md](docs/guide/troubleshooting.uk.md) |
| дізнатися, чому він швидкий, і перевірити самому | [internals/performance.md](docs/internals/performance.md) |
| зрозуміти, як він влаштований усередині | [internals/architecture.md](docs/internals/architecture.md) |
| зібрати, протестувати, зробити внесок | [internals/contributing.md](docs/internals/contributing.md) |
| дізнатися, навіщо він узагалі існує | [project/landscape.md](docs/project/landscape.md) |

Повний покажчик - у [docs/README.uk.md](docs/README.uk.md); політика безпеки й
те, як повідомити про вразливість, - у [SECURITY.md](SECURITY.md).

## Мова

Сторінки, потрібні гравцеві, перекладені. **Англійська - канонічна**: якщо
переклад із нею розходиться, правий англійський файл.

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


**Усе інше англійською навмисно, а не через недогляд.** `docs/internals/` і
`docs/project/` читають ті, хто читає й сам Rust, а `CHANGELOG.md` генерується.
Їхній переклад - це ще 17 678 слів, які треба тримати чесними, для аудиторії,
якій вони не потрібні.

Кожен переклад несе хеш англійського файлу, з якого його зроблено, і CI падає,
коли англійський іде вперед - див. [`scripts/i18n-check.sh`](scripts/i18n-check.sh).
Переклад, який неможливо повернути в актуальний стан, **видаляється**, а не
лишається на місці: застаріла сторінка так само виглядає авторитетно й видає
команди минулого місяця, а це для читача гірше, ніж бути відісланим до
англійської.

Додати мову - це чотири файли й рядок у цій таблиці; кроки є в
[`docs/internals/contributing.md`](docs/internals/contributing.md).

## Підтримувані ігри

**Skyrim SE/AE** - перевірений реальною грою. **Fallout 4** теж під'єднаний від
початку до кінця (F4SE підставляється автоматично, інвалідація архівів, порядок
завантаження із зірочкою, LOOT, збереження `.fos`) - див. [guide/fallout4.uk.md](docs/guide/fallout4.uk.md). Під'єднані через спільний дескриптор гри й
шукають тестувальників: Skyrim LE, Skyrim VR, Enderal SE, Fallout 3, Fallout NV,
Fallout 4 (+ VR), Starfield, Oblivion і Morrowind (останні дві монтують і керують
модами; їхні впорядковані за часовими мітками списки плагінів поки не керовані).

Додати сімейство - це один рядок дескриптора:
[internals/adding-games.md](docs/internals/adding-games.md).

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
