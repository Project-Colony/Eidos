<!-- eidos-i18n: source=README.md sha=37e97bafbd2cf43f2101a28b23c1464420a5f9ec -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Нативний менеджер модів для Linux, який ніколи не торкається теки вашої гри.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#встановлення)

Eidos дає іграм Bethesda під Linux те, що Mod Organizer 2 дає їм під Windows:
об'єднаний вигляд ваших модів, що будується при кожному запуску. Він
використовує mount namespace і FUSE у Linux замість перехоплення Windows API, а
сам менеджер працює без Wine. Нічого не копіюється в теку гри, тож нічого й
очищати після цього. Eidos має також визначення для Stellar Blade, Stardew
Valley, 7 Days to Die і Crimson Desert.

Копії зі Steam запускають Eidos через параметр запуску гри. Копії з GOG і Epic у
Heroic чи Legendary запускають його як обгортку. Гра завжди стартує через
магазин, якому належить копія; Eidos ніколи не запускає її через Proton сам.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Стан:** у Skyrim SE грають через Eidos щодня, зі SKSE, preloader'ами
> script extender, Creation Club, порядками завантаження, відсортованими LOOT, і
> збереженнями на кожен профіль. Поки що одне сімейство ігор перевірене реальною
> грою. Ще десять ігор Bethesda та чотири інші під'єднані й чекають на
> тестувальників.

## Чому Eidos

Об'єднаний вигляд монтується в приватному mount namespace, який бачить лише
дерево процесів гри. Інші програми, друга гра теж, ніколи його не бачать і не
потребують на нього дозволу. Коли гра завершується чи її вбито, або коли машина
втрачає живлення, namespace зникає разом зі своїми процесами, а інсталяція
лишається такою, якою була.

Кожен профіль володіє своїм списком модів, порядком плагінів, INI та
збереженнями. При запуску файли плагінів і тека збережень bind-монтуються поверх
власних шляхів гри, тож те, що гра там пише, потрапляє в профіль. Зміна профілю
змінює все це.

Eidos не потребує ні setuid-помічника, ні системного демона, ні `sudo setcap`, ні
редагування `/etc/fuse.conf`. Демон FUSE працює від вашого імені на один запуск і
завершується разом з грою. Ви встановлюєте одну програму й задаєте один параметр
запуску Steam.

Перед кожним запуском Eidos зберігає знімок списку плагінів. Якщо збій
пошкоджує список, Diagnostics порівнює його зі знімком, а Restore the pre-session
order повертає старий порядок. Захоплення, що стерло б ваш порядок завантаження,
відхиляється з поясненням причини.

Test run запускає гру з модами профілю, без модів або лише з вибраними модами та
їхніми майстрами. Збереження, зміни INI, стан плагінів, вивід Overwrite та інші
файли My Games цього запуску, журнали теж, потрапляють у `<instance>/test-run/`.
Коли гра завершується, Eidos перелічує ці п'ять груп, і ви залишаєте або
викидаєте кожну. Інші записи, як-от реєстр Wine, йдуть туди, куди йшли завжди.
Find the mod behind a problem на вкладці Diagnostics запускає тестові запуски з
половиною підозрюваних модів щоразу плюс майстри, яких вони потребують. Після
кожного запуску ви відповідаєте, чи проблема була, і Eidos звужує підозрюваних,
доки не назве мод або моди, що потребують майстрів одне одного; 200 підозрюваних
потребують близько 8 запусків. Моди, вибрані у списку до початку, лишаються
ввімкненими на кожному кроці. Подробиці в
[usage.md](docs/guide/usage.md#тестові-запуски) та
[usage.md](docs/guide/usage.md#як-знайти-мод-що-спричиняє-проблему).

### Як він порівнюється

| | Eidos | MO2 через Wine | Fluorine-Manager | Limo / link-деплоєри |
|---|---|---|---|---|
| Менеджер працює нативно | так | ні, Windows-застосунок у Wine | так (порт на Qt) | так |
| Тека гри не зачеплена | так, завжди | так | так | ні, у неї пишуться посилання |
| Монтування видиме для | лише гри | лише гри | усієї системи | н/д |
| Потрібне очищення після збою | ні | ні | відновлення після застряглого монтування | ручне un-deploy |
| Root-моди (ENB, preloader'и) | так, нативно | потрібен плагін | потрібен плагін | частково |
| Потрібні привілеї | немає | немає | редагування `/etc/fuse.conf` | немає |

### Наскільки він швидкий

| | до | тепер |
|---|---|---|
| завантаження збереження | ~20 секунд | 6-7 секунд |
| читань тек за одну сесію | 5,6 мільйона | 335 тисяч |

Переходи між cell миттєві. Пошук одного файлу раніше опитував усі п'ятдесят модів
по черзі, а перелік однієї теки робив це п'ятдесят разів поспіль. Шари лише для
читання тепер індексуються один раз при монтуванні, тож пошук файлу - це одне
звернення до хеша, а об'єднаний перелік теки теж обчислюється один раз при
монтуванні. Числа взято з реального екземпляра у звичайній грі, а не з
бенчмарку: [internals/performance.md](../../internals/performance.md).

### Попередні напрацювання та подяки

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) та
  [usvfs](https://github.com/ModOrganizer2/usvfs): семантика, яку Eidos
  відтворює, і код, за яким вивчалася відповідність
- [LOOT](https://loot.github.io/): рушій сортування, через libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) та інші менеджери під Linux, які
  показують, що є спільнота, яка хоче це розв'язати

## Що він робить

### Моди

Eidos встановлює прості архіви, майстри FOMOD, пакунки Wrye Bash BAIN, контейнери
OMOD з підтримуваними скриптами OBMM і все інше через ручний вибір. Root-моди
(preloader'и script extender, ENB, Engine Fixes) працюють нативно, без плагіна
Root Builder і без нічого, скопійованого у вашу інсталяцію. Можна приховувати
окремі файли, групувати моди роздільниками, переміщувати вибране до цілі, вести
нотатки й категорії на кожен мод та імпортувати профіль MO2.

Список працює як в MO2: вісім необов'язкових стовпців і сортування за будь-яким із
них, групування за категорією чи за джерелом, жести подвійним клацанням, перехід
набором тексту, резервні копії на кожен мод, які нічого не роблять, доки ви їх не
відновите, і позначки для мода, чию розкладку ця гра не завантажить або який
завантажено для іншої гри. Фільтри вибирають моди за станом Nexus (схвалені,
відстежувані, є оновлення), нотатками, прихованими файлами й видом вмісту
(плагіни, архіви, текстури, меші, скрипти, звук та інше). Дерево файлів створює
теки, перейменовує, видаляє й відкриває файли та показує зображення й текст,
нічого не запускаючи.

Ctrl+Z і Ctrl+Shift+Z скасовують і повторюють зміни списку модів та порядку
завантаження. Вимкнення чи видалення мода, який використовує одне з трьох
найновіших збережень профілю, спершу питає й називає плагіни (збереження Skyrim і
Fallout 4). Перевстановлення чи оновлення мода FOMOD відкриває майстер з
відповідями його останнього встановлення. Information мода перелічує моди, яких він
потребує, і моди, які потребують його, зчитані з майстрів увімкнених плагінів.

### Плагіни

Порядок завантаження має вбудоване сортування LOOT, індекси модів такі, як їх
обчислює гра, попередження про відсутні майстри, а ваші DLC та вміст Creation Club
показані як некеровані рядки. Заголовок рахує використані слоти в кожному просторі
індексів (full, medium, light). Наведення на плагін показує його автора, опис і
версію заголовка. У рушіях Skyrim плагін, чий однойменний INI завантажує гра,
позначається.

Плагіни, які LOOT позначає як брудні, чистяться через QuickAutoClean від xEdit у
мод `Cleaned plugins`, ніколи на місці. Flag as light, into a new mod пише
light-копію плагіна й відмовляє, коли одне з найновіших збережень завантажує цей
плагін як повний (збереження Skyrim і Fallout 4).

### Екземпляри

Екземпляр буває глобальним, що лежить у `~/.local/share/Colony/Eidos/instances/`,
або портативним: самодостатня тека будь-де, де забажаєте (другий диск, ігровий
розділ), переносна й ізольована, як у MO2. Портативні екземпляри запам'ятовуються
між сесіями. GUI, запуск зі Steam і кожна команда CLI йдуть за тим, яким ви
користувалися востаннє, а будь-яка команда приймає теку скрізь, де приймає
ідентифікатор гри. Подробиці в
[usage.md](docs/guide/usage.md#екземпляри-глобальні-та-портативні).

Екземпляр може також запускати гру з копії теки гри всередині екземпляра. Коли
Eidos уперше відкриває список Wabbajack, він запускає власну копію списку: теку,
яку називає його `ModOrganizer.ini`, а інакше теку `Stock Game` чи `Game Root`.
Freeze this version на картці Game files у Diagnostics копіює інсталяцію з магазину
в `<instance>/Stock Game`, тож оновлення магазину більше не змінює файли, з яких
працює гра. Коли обидві теки на одній файловій системі, що вміє ділити блоки, як-от
btrfs чи XFS, копія ділить їх з інсталяцією з магазину (reflink). На ext4 або між
двома файловими системами це повна копія, а розмір і вільне місце показуються
спершу. Магазин і далі запускає гру. Return to the store version і Delete the copy
скасовують це, а
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
робить те саме в терміналі. Morrowind, а також Oblivion з
`bUseMyGamesDirectory=0` не можуть працювати з копії. Подробиці в
[usage.md](docs/guide/usage.md#файли-гри-встановлення-з-магазину-або-копія).

### Профілі

Кожен профіль має власний порядок модів, стан плагінів, INI та збереження.
Збереження розбираються й звіряються з вашими поточними плагінами, з кнопкою, що
вмикає те, чого збереження потребує, і копіюються назад у префікс для Steam Cloud
після кожної сесії. Збереження, які Steam Cloud приніс з іншого пристрою або які
запуск без Eidos залишив у префіксі, перелічуються на вкладці Saves. Import
копіює їх у профіль, а збереження, чию назву профіль уже використовує, приходить
під новою назвою з `orphan-` замість заміни наявного.

Профіль може нести змінні запуску й обгортки (`env=NAME=value` та
`wrapper=command`, по одному на рядок, у редакторі INI) для запусків своєї гри та
інструментів. Для Skyrim SE, Enderal SE і Fallout 4 картка Display у Diagnostics
задає розмір і режим вікна в копіях INI профілю. Backups зберігає точки
відновлення INI профілю, а Eidos робить одну перед кожним запуском BethINI Pie.
Ярлик робочого столу профілю запускає гру через її магазин. `eidos sessions`
перелічує записані запуски й порівнює один із поточним станом.

### Nexus

Під'єднайте обліковий запис, щоб отримувати посилання Mod Manager Download,
перевіряти оновлення та переглядати авторів і їхні профілі. Кожен мод
перевіряється, схвалюється й пов'язується на домені Nexus, звідки прийшов його
файл, тож мод Skyrim SE в екземплярі Enderal SE лишається на сайті Skyrim SE.
Оновлення йде за ланцюгом наступників, який автор задав на Nexus, замінює теку
мода новим файлом (пропонується резервна копія старої), зберігає його пріоритет і
стан ввімкнення та відтворює відповіді FOMOD, збережені при встановленні.
`eidos nexus update <instance> --apply` робить те саме в терміналі на
premium-обліковому записі, а на безкоштовному друкує сторінки файлів. Track і
Endorse є в меню мода, а кожна перевірка оновлень синхронізує їх з вашим обліковим
записом Nexus. Information мода має вкладку Nexus page з його описом і журналом
змін простим текстом, а його вкладка General закінчується тим, що змінило останнє
оновлення: додані, видалені й змінені в розмірі файли та додані чи видалені
майстри. Install downloads when they finish (Preferences, Nexus, типово вимкнено)
встановлює завантаження, щойно воно завершується, поки вікно відкрите.

Колекції встановлюються за своїми рецептами з можливістю відновити перерваний
процес і явними звітами про непідтримувані кроки. Зіставлення перевіряє точні
вихідні файли; неповні старі метадані залишаються неперевіреними. Панель колекцій
переглядає й шукає колекції, опубліковані для гри, показує, коли встановлена
колекція має новішу ревізію, і веде безкоштовний обліковий запис через відсутні
файли по одній сторінці. Коли перетягування ламає правила порядку колекції,
Diagnostics каже про це, а Restore collection order повертає її моди на місце.
Вкладка Downloads дозволяє фільтрувати, сортувати, приховувати й видаляти архіви.
Offline mode (Preferences, Nexus) припиняє доступ до Nexus і щоденну перевірку
випусків.

### Інструменти

xEdit, BodySlide, DynDOLOD та інші інструменти модингу працюють через об'єднаний
вигляд усередині Wine-префікса гри (Proton-префікса Steam або того, який створив
Heroic). Вони бачать ваші моди, їхній вивід потрапляє в Overwrite, а один клац
перетворює його на справжній мод. Для Skyrim SE Eidos знаходить SSEEdit, SSEEdit
QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen,
xLODGen, Synthesis і BethINI Pie у теці гри, всередині мода або в теці
інструментів, яку ви тримаєте поруч з іграми, кожен із потрібними йому
середовищами виконання. Інші ігри мають власний список. Pandora, Nemesis, FNIS,
BodySlide і Synthesis пишуть свій вивід в іменований мод (`Nemesis Output` тощо).

Пакет випуску постачає інсталятори Visual C++ 2015-2022 і .NET 8 Desktop Runtime
від Microsoft та середовище виконання .NET 10. Eidos встановлює `vcrun2022` і
.NET 8 цими інсталяторами, без winetricks, а без пакета завантажує ті самі файли
від Microsoft, кожен із яких перевіряється за SHA-256, вбудованим в Eidos.
Synthesis отримує .NET 10 SDK, завантаження на 300 МБ, яке пакет не постачає.
Інші передумови (.NET Framework 4.8, XACT і старіші версії .NET) і далі йдуть
через winetricks. Нічого не завантажується, доки ви не попросите встановити
передумови інструмента.

Закріплюйте інструменти, якими користуєтеся, ховайте ті, якими ні, дайте
інструментові власний Steam AppID, коли він є окремим застосунком Steam, і
запишіть ярлик `.desktop`, який запускає його через об'єднаний вигляд, не
відкриваючи Eidos. `eidos tool <game> import-mo2` перелічує виконувані файли з
`ModOrganizer.ini` теки списку, а `--write` додає їх. Exe на вкладці Data можна
запустити один раз або додати як інструмент, а `eidos tool <instance> run --exe
<path>` запускає його з терміналу. З Ask before playing Run спершу питає, коли вивід
генератора застарів.

Stop завершує запущену гру чи інструмент. Меню File запускає winecfg, regedit,
файловий провідник Wine і програму видалення Windows у префіксі гри. Rebuild
prefix у Diagnostics перейменовує префікс, щоб Steam створив новий, і нічого не
видаляє. `eidos shell <instance>` відкриває оболонку в об'єднаному вигляді або
виконує команду, наведену після `--`, а те, що вона пише, потрапляє в Overwrite.

Поки вікно у фоні, Eidos може надіслати сповіщення робочого столу, коли
завершується запуск інструмента, встановлення колекції, пакування чи
розпакування або останнє завантаження, що виконувалося, і коли сортування LOOT
не вдається. Notify the desktop when a long job ends у Preferences вмикає це;
типово вимкнено.

### Діагностика

Вкладка Diagnostics перелічує відсутні майстри, осиротілі архіви, розходження
списку модів і пошкоджені набори плагінів, а після запуску - що власний лог script
extender каже про справді завантажене. Вона також повідомляє:

- про script extender, якого немає або який не відповідає версії гри
- про гру, що змінилася з останнього чистого запуску
- про параметр запуску Steam, що не стартує Eidos або не має `%command%` після нього
- про Proton, старіший за той, що останнім налаштовував префікс, або з Flatpak
  Steam
- про відсутній `PROTON_ENABLE_NVAPI` для DLSS і ексклюзивний повноекранний режим,
  що зупиняє генерацію кадрів Community Shaders
- про `enblocal.ini` без `LinuxVersion=true`
- про лог збою останньої сесії та моди, які він називає
- про вимоги Nexus, яких немає або які вимкнені
- про обробник `nxm://`, що є іншою програмою або відсутнім файлом
- про рядки `sTestFile`, які змушують гру ігнорувати `plugins.txt`
- про увімкнені `.esp` у Starfield та некоректні чи непарні blueprint-додатки
- про патч, чиї вільні файли програють моду, який він патчить
- про мод, чиї копії скриптів Papyrus script extender перемагають власні скрипти
  розширювача
- про низький `vm.max_map_count` чи межу відкритих файлів
- про файли гри на файловій системі Windows
- про шляхи даних BodySlide, що не є текою Data гри, і теки в `Overwrite/Root`,
  яким місце в Data
- про `ModOrganizer.ini` списку, що скеровує інструменти на іншу теку гри
- про копію гри, якої немає або в якій немає виконуваного файлу гри

`eidos check <instance> [--json]` виконує перевірки, яким не потрібне вікно, і
завершується з кодом 1, коли знаходить проблему. Copy report копіює звіт із вашою
домашньою текою як `~` і вашим іменем користувача як `<user>`, а Save tree на
вкладці Data записує об'єднане дерево Data з джерелом кожного файла, приховане
так само.

Про захист під час заміни, конфлікти файлів у BSA/BA2, статичну діагностику SKSE
та записи генерації разом з їхніми обмеженнями див.
[Перевірки цілісності та створені файли](../../internals/integrity.md)
(англійською).

### Де він тримає власні файли

Eidos використовує розкладку, якою користується кожна програма сімейства Colony:
`~/.config/Colony/Eidos/` - для того, що ви обрали (налаштування, ваша сесія
Nexus, список екземплярів, написані вами визначення ігор і доповнень),
`~/.local/share/Colony/Eidos/` - для глобальних екземплярів, середовищ виконання
інструментів (завантажених або скопійованих із пакета через `install.sh`) і логів
сесій, і `~/.cache/Colony/Eidos/` - для мастер-списків LOOT. Усередині
екземпляра `Stock Game` містить заморожену копію гри, а `test-run/` - тестовий
запуск, що чекає на розгляд.

Старіший Eidos тримав свої файли в `~/.config/eidos/`, `~/.local/share/eidos/` і
`~/.local/state/`. Перший запуск після оновлення переносить їх, пише про це в
лог і лишає старі теки такими, якими вони були. Доки перенесення не завершено,
Eidos і далі користується старими файлами. Подробиці в
[internals/paths.md](../../internals/paths.md) (англійською).

### Підтримувані ігри

Skyrim SE/AE перевірений реальною грою. Fallout 4 теж під'єднаний від початку до
кінця (F4SE підставляється автоматично, інвалідація архівів, порядок завантаження
із зірочкою, LOOT, збереження `.fos`); див.
[guide/fallout4.md](docs/guide/fallout4.md). Під'єднані й шукають тестувальників:
Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV, Starfield,
Oblivion і Morrowind. Fallout 3, Fallout NV, Oblivion і Morrowind упорядковують
плагіни за датами файлів: Eidos тримає один порядок на профіль і показує ці дати
всередині монтування, не змінюючи файли на диску.

Ще чотири гри під'єднані й теж шукають тестувальників: Stellar Blade (моди Unreal
`.pak` у `~mods`), Stardew Valley (моди SMAPI, зі SMAPI, встановленим окремо),
7 Days to Die (теки `Mods/` поруч із грою) і Crimson Desert. Crimson Desert
приймає лише кодові моди: плагіни ASI, ReShade і DLL-завантажувачі в `bin64`. Його
моди даних, що живуть у групах PAZ, не підтримуються.

Гра - це один файл TOML, а ваш власний можна покласти в
`~/.config/Colony/Eidos/games/` без перезбирання:
[internals/adding-games.md](../../internals/adding-games.md).

## Встановлення

### Через Colony (рекомендовано)

Знайдіть **Eidos** у [Colony](https://github.com/Project-Colony/Colony) і
встановіть його. Оновлення приходять через лаунчер. Colony встановлює один файл,
`eidos-linux`: це вікно, а також командний рядок `eidos`, коли його викликають
через посилання з назвою `eidos`. Цей файл не містить ні допоміжної програми
перегляду моделей NIF, ні середовищ виконання Microsoft з описаного нижче пакета.
Без допоміжної програми перегляд про це повідомляє, а все інше працює, і Eidos
завантажує середовище виконання від Microsoft, коли ви встановлюєте передумови
інструмента. Що робити після встановлення і як скерувати на нього Steam:
[docs/guide/install.md](docs/guide/install.md#через-colony).

### Пряме завантаження бінарних файлів

Візьміть файл з [останнього випуску](https://github.com/Project-Colony/Eidos/releases/latest):

| Файл | Що це |
|---|---|
| `eidos-linux` | уся програма в одному файлі, так, як її встановлює Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, допоміжна програма перегляду NIF, інсталятори Visual C++ 2015-2022 і .NET 8 Desktop Runtime та середовище виконання .NET 10, іконки, повідомлення про ліцензії та інсталятор, що реєструє запис робочого столу й обробник завантажень Nexus |

Спершу перевірте завантаження, як показано в розділі
[Політика підписування коду](#політика-підписування-коду), потім:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Інсталятор кладе бінарні файли в `~/.local/bin` без sudo і копіює `runtimes/`
пакета до `~/.local/share/Colony/Eidos/runtimes`. `--system` і `--bindir DIR`
кладуть бінарні файли деінде, а `--from DIR` бере їх з іншої теки. `--cap` просить
sudo надати `CAP_SYS_ADMIN`, потрібний лише експериментальному FUSE passthrough, а
passthrough лишається вимкненим; `--no-cap` - типове значення. Обом файлам потрібен
Linux x86_64 з glibc 2.39 або новішою, FUSE (`fusermount3`) і 7-Zip для архівів
модів.

Eidos не оновлює себе сам. Раз на добу при старті вікно питає GitHub про останній
випуск, і коли виходить новіший, показує посилання на його нотатки про випуск
після Help у рядку меню. Preferences це вимикає, а копія під `/usr` поза
`/usr/local`, яку оновлює менеджер пакетів, ніколи не питає.

### Збирання з джерельних кодів

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Потрібні Rust 1.92 або новіший (`rust-version`, який перевіряє CI; CI і випуски
збираються з 1.99.1), а також `just`, CMake 3.20+ і компілятор C++17 для
допоміжної програми перегляду NIF. `just build` використовує профіль release: одну
кодогенеративну одиницю й fat LTO. Холодна збірка `eidos` та `eidos-gui` з 3
завданнями збирання тривала так 334 секунди, а з thin LTO - 243 секунди, і fat LTO
робить `eidos` меншим на 1,2 МБ, а `eidos-gui` - на 3,8 МБ (виміри в
`Cargo.toml`). `just profile=dev build` компілює швидше. Пакунок Arch збирається з
`packaging/PKGBUILD`. Передумови й команди без `just`:
[docs/guide/install.md](docs/guide/install.md).

### Параметри запуску Steam

Eidos виконується як команда запуску гри - саме так він монтує до старту гри.
Базового рядка досить для більшості конфігурацій, з абсолютним шляхом до вашої
інсталяції (Steam не читає ваш `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Для Colony цей шлях - `~/.local/share/Colony/apps/Eidos/eidos-linux`. Потім
натисніть Play: Eidos відкриється на екземплярі цієї гри, а Run запускає гру
через об'єднаний вигляд.

У вікні, яке ви відкрили самі, Run просить Steam запустити гру
(`steam -applaunch <appid>`), тож параметр запуску й далі діє, а ви натискаєте Run
у вікні, яке відкриває Steam. Heroic отримує посилання `heroic://launch`, а
Legendary виконує `legendary launch <app>`, коли Eidos там задано обгорткою гри.
Коли налаштування магазину немає, Run нічого не запускає й каже, що задати:
[guide/usage.md](docs/guide/usage.md#запуск-гри). З Start the game at once when a
store opens Eidos (Preferences, типово вимкнено) Eidos виконує перевірки
Diagnostics і запускає гру, не показуючи свого вікна, коли нічого не потребує
уваги.

Усе інше - це змінні середовища, складені перед ним, і вони вільно поєднуються:

| Ви хочете... | Поставте попереду |
|---|---|
| DLSS із Community Shaders | `PROTON_ENABLE_NVAPI=1`. Без цього DLSS мовчки ніколи не ініціалізується; повний перелік - у [guide/graphics.md](docs/guide/graphics.md) |
| лічильник FPS на екрані | `DXVK_HUD=fps` |
| інтерполяцію кадрів на рівні драйвера, нуль модів (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, ніколи разом із власною генерацією кадрів Community Shaders |
| докладні логи для звіту про ваду | `EIDOS_LOG=debug` (логи сесій потрапляють у `~/.local/share/Colony/Eidos/logs/`) |
| звіт про I/O монтування за сесію | `EIDOS_FUSE_STATS=1` |
| іншу кількість робітників FUSE | `EIDOS_FUSE_THREADS=8` (типово 4; `1` - перше, що варто спробувати, шукаючи ваду конкурентності) |
| закріпити цей запуск за одним портативним екземпляром | `EIDOS_INSTANCE=/path/to/folder`. Без цього Eidos відкриває екземпляр, яким ви користувалися востаннє |
| вікно для одного запуску, поки ввімкнено Start the game at once | `EIDOS_WINDOW=1` |

Рядок, який варто лишити для сучасної модифікованої збірки (Community Shaders,
DLSS, генерація кадрів), - це остаточна команда, а не приклад:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Додайте `DXVK_HUD=fps` попереду, поки перевіряєте, що збірка працює, і приберіть,
щойно вона запрацює.

Ті самі змінні можна задати в змінних запуску профілю замість рядка Steam, що
тримає їх на кожен профіль і застосовує також до запусків інструментів:
[guide/usage.md](docs/guide/usage.md#змінні-та-обгортки-для-профілю).

Глибші діагностичні перемикачі (`EIDOS_FUSE_TRACE`, перемикачі бісекції кешу та
індексу, чому `EIDOS_FUSE_PASSTHROUGH` типово вимкнений) - у
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Документація

Усе лежить у [docs/](../../../docs/), упорядковане за тим, хто читає, у
[docs/README.md](docs/README.md): посібник користувача (встановлення, CLI і GUI,
інструменти на кшталт xEdit і DynDOLOD, графіка, усунення несправностей), як
Eidos влаштований усередині і навіщо він існує. Політика безпеки і як повідомити
про вразливість: [SECURITY.md](../../../SECURITY.md).

## Мова

Сторінки, потрібні гравцеві, перекладені. Англійська - еталон: якщо переклад із
нею розходиться, правий англійський файл.

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

**Усе інше англійською.** `docs/internals/` і `docs/project/` читають ті, хто
читає й сам Rust, `CHANGELOG.md` генерується, а `SECURITY.md` теж не
перекладається. Переклад перших двох додав би понад 19 000 слів, які треба тримати
синхронними, для читачів, яким вони не потрібні.

Кожен переклад несе хеш англійського файлу, з якого його зроблено, і CI падає,
доки цей хеш не збігається з англійським; див.
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Тоді переклад
оновлюють і ставлять новий хеш через `scripts/i18n-check.sh --fix`. Застарілий
переклад оновлюється, а не видаляється.

Мова - це одна тека. `docs/i18n/<lang>/` повторює структуру кореня репозиторію, тож
`docs/i18n/de/docs/guide/install.md` - це німецький `docs/guide/install.md`. Саме
тому посилання між двома перекладеними сторінками є тією самою стрічкою, що й
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
названої там дії. Режим Offline (Preferences, Nexus) припиняє будь-які
з'єднання з Nexus і щоденну перевірку випусків.

| Дані | Зберігаються чи надсилаються | Куди і навіщо |
|---|---|---|
| Налаштування, список екземплярів, визначення ігор і доповнень | зберігаються | `~/.config/Colony/Eidos/` |
| Екземпляри, середовища виконання інструментів, логи сесій | зберігаються | `~/.local/share/Colony/Eidos/` або тека, яку ви обрали для портативного екземпляра |
| Мастер-списки LOOT | зберігаються | `~/.cache/Colony/Eidos/loot/` |
| Перевірка випусків | надсилається | При старті, щонайбільше раз на добу, один запит до `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos читає лише тег випуску й нічого не завантажує. Вона типово ввімкнена й вимикається в Preferences, а в режимі Offline та для копії, встановленої під `/usr` поза `/usr/local`, яку оновлює менеджер пакетів, пропускається. [Заява GitHub про конфіденційність](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Вхід у Nexus | надсилається | Вхід відкриває `users.nexusmods.com` у вашому браузері, який повертає результат слухачеві, що його Eidos відкриває на `127.0.0.1` лише для цього входу. Потім Eidos обмінює його на токени на `users.nexusmods.com/oauth/token` і отримує відкриті ключі, якими Nexus їх підписує. [Політика конфіденційності Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Облікові дані Nexus | зберігаються | Токени доступу та оновлення, у `~/.config/Colony/Eidos/nexus.ini` з правами 0600. Рядок `api_key=`, записаний туди старішим Eidos, зберігається, але ніколи не читається; за бажання видаліть його. |
| Запити до Nexus | надсилаються | `api.nexusmods.com`, коли ви користуєтеся функцією Nexus: назва вашого облікового запису і статус premium, відомості про моди та файли, перевірки оновлень, колекції та посилання на завантаження. Ваші схвалення й відстежувані моди читаються при кожній перевірці оновлень і записуються, коли ви натискаєте Endorse чи Track. Вимоги модів, описи й журнали змін, а також пошук колекцій ідуть на `api.nexusmods.com/v2/graphql`. Кожен запит несе ваш токен доступу і називає Eidos та його версію. |
| Завантаження з Nexus | надсилаються | Файли, які ви запитуєте, з хоста CDN Nexus, названого в посиланні на завантаження; зберігаються в теці завантажень екземпляра з `.meta` у форматі MO2 поруч із кожним. |
| Посилання `nxm://` | зберігаються | `eidos nxm --register`, який `install.sh` запускає за вас, пише `~/.local/share/applications/eidos-nxm.desktop` і робить його обробником `nxm://` через `xdg-mime`, тож кнопка Mod Manager Download на сайті передає своє посилання Eidos. Пакунок Arch встановлює той самий запис для всієї системи. |
| Мастер-списки LOOT | надсилаються | `raw.githubusercontent.com/loot/...`, під час першого сортування LOOT для гри і щоразу, коли ви просите оновлення. Нічого вашого не надсилається. [Заява GitHub про конфіденційність](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Середовища виконання інструментів | надсилаються | Лише коли ви встановлюєте передумови інструмента, і лише те, чого не було в пакеті випуску (окремий файл `eidos-linux` і збірка з джерельного коду не містять нічого): середовище виконання .NET 10 з `builds.dotnet.microsoft.com` та інсталятори Visual C++ і .NET 8 з `download.visualstudio.microsoft.com`, кожен з яких відхиляється, якщо не збігається з SHA-256, вбудованим в Eidos; .NET 10 SDK, завжди з `builds.dotnet.microsoft.com` і з тією самою перевіркою; і `winetricks` для решти передумов (.NET Framework 4.8, XACT, .NET 6 і 7), який завантажує те, що встановлює, з Microsoft та інших хостів, якими користується winetricks. [Заява Microsoft про конфіденційність](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Логи сесій | зберігаються | `~/.local/share/Colony/Eidos/logs/`, права 0600: десять сесій на екземпляр, прибираються через 30 днів. Ваша домашня тека записується як `~`, а ім'я користувача як `<user>`, тож лог можна вставити в issue як є. Вони ніколи не залишають вашу машину, якщо ви самі їх не надішлете. |
| Записи запусків | зберігаються | `<instance>/logs/sessions/`: для кожного запуску гри чи інструмента профіль, увімкнені моди з їхніми версіями та ідентифікаторами файлів Nexus, порядок плагінів, контрольна сума кожного INI профілю, версії гри й Eidos, код виходу та збереження, створені запуском. Зберігається 50 найновіших, `eidos sessions` їх читає, а `eidos pack` їх не включає. Вони лишаються на вашій машині. |
| Звіти про оновлення | зберігаються | `<instance>/logs/updates/<mod>.json`: що останнє оновлення чи Replace змінило в теці цього мода. `eidos pack` їх не включає. |
| Ярлики | зберігаються | `install.sh` пише `eidos.desktop` і його іконки в `~/.local/share`; ярлик інструмента чи профілю, який ви створюєте, потрапляє в `~/.local/share/applications`. |
| Сповіщення робочого столу | ні те, ні інше | Типово вимкнені. Коли ввімкнені, вони йдуть до служби сповіщень вашого робочого столу через D-Bus (`org.freedesktop.Notifications`) і називають завдання, що завершилося. |
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

Бінарні файли також вбудовують вісім справжніх розповсюджуваних DLL Microsoft
DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` і `d3dx11_43`, 32- і
64-бітні), збережених стиснутими gzip (7,9 МБ замість 17,9 МБ), які Eidos
розпаковує й розгортає в префікс гри, коли вони потрібні моду чи інструменту.
Вони лишаються під умовами Microsoft для розповсюджуваних компонентів; див. їхнє
[походження](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Пакет випуску також містить у `runtimes/` інсталятори Visual C++ 2015-2022 від
Microsoft, інсталятор .NET 8 Desktop Runtime і zip середовища виконання .NET 10 у
тому вигляді, як їх публікує Microsoft. Їхні умови лежать у
`licenses/microsoft-runtimes/` (`vc_redist.txt`, `dotnet.txt`).
