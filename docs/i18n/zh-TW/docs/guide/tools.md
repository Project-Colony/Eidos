<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# 工具:xEdit、BodySlide、DynDOLOD、PGPatcher、FNIS

透過 Eidos 執行的工具看到的是合併檢視,就在遊戲自己的 Wine 前綴裡(Steam 的 Proton
前綴,或 Heroic 設好的前綴)。它讀到的正是遊戲會讀到的東西(每一個啟用的模組,按優先
順序),而它寫出的任何東西都落進 Overwrite,在那裡一鍵就能變成真正的模組。
`runner=native` 的工具不經過 Proton 執行;見
[下文](#環境變數以及不經過-proton-的工具)。

## Eidos 自己找得到的那些

有些工具的名字夠獨特,不必宣告就能被找到,xEdit 就是最明顯的例子:Fallout 4
是 `FO4Edit.exe`,Skyrim SE 是 `SSEEdit.exe`,初代是 `TES5Edit.exe`,依此類推。
每一個都附帶一個 QuickAutoClean 項目,用來處理 LOOT 警告的 dirty edits。
`PGPatcher.exe` 在除 Morrowind 之外的每一款 Bethesda 遊戲上都以同樣的方式被找到,
並且帶著一個缺了就無法運作的參數(見
[下文](#為什麼-pgpatcher-需要---ignore-mo2vfscheck))。Eidos 依檔名,在這些地方找它們:

- `Overwrite/Root/`、實例執行所用的遊戲資料夾(商店安裝,或它的遊戲副本),以及已啟用
  模組的 `Root/` 樹;
- 這個實例的 `mods/`,MO2 使用者就是把工具裝在那裡;
- 你在 Preferences 裡設定的工具資料夾(General -> Tools),用於實例之間共用的目錄,
  像 `/mnt/Games/Tools` 之類。

常用的產生器也以同樣的方式尋找,各自只在其支援的遊戲中出現:Pandora、Nemesis、FNIS、
BodySlide、DynDOLOD、TexGen、xLODGen 和 Synthesis。BethINI Pie(`Bethini.exe`)也是,
出現在與 PGPatcher 相同的遊戲上;見 [BethINI Pie](#bethini-pie)。在 Stardew Valley 上,
SMAPI(`StardewModdingAPI.exe`)也以同樣方式被找到。它同時是那款遊戲的 script extender
載入器,所以它透過商店啟動遊戲,如
[找到工具](#找到工具隱藏置頂以及桌面捷徑)所述。

清單是按遊戲分的,所以 Skyrim 實例永遠不會被提供 Fallout 的編輯器。搜尋在第四層就停下,
因為一個模組池有數十萬個檔案,而每次建立工具清單時都會跑一次;它也不跟隨 symlink。這樣
找到的工具,設定方式和你自己輸入的完全一樣:它的前置需求、參數(PGPatcher 的
`--ignore-mo2vfscheck`)和擷取模組都依它的標題而定,規則見下文。

如果工具在別的地方,或你想用不同的參數,就手動加。標題相同的使用者項目會覆蓋任何自動
找到的結果。

## 清理 LOOT 標記為 dirty 的外掛

LOOT 排序之後,Diagnostics 分頁會列出 LOOT 標記為 dirty(需要清理)的外掛,適用於同時有
xEdit 和 LOOT 清理資料的遊戲(Skyrim LE、SE 和 VR,Enderal SE,Fallout 3,New Vegas,
Fallout 4 和 4 VR,Oblivion,Starfield)。如果 Eidos 找到了 xEdit(在 Executables 裡登記
過,或放在實例中 Eidos 尋找工具的任何位置),卡片上會有 Clean all,每個 dirty 外掛的列
選單裡會有 Clean with QuickAutoClean;否則卡片會提示在 Executables 裡加入 xEdit。Eidos
不會下載 xEdit。它透過合併檢視,用 `-qac -autoexit -autoload`,一次對一個外掛執行 xEdit
的 QuickAutoClean 模式。xEdit 把清理後的外掛存回它載入的那個外掛上,在合併檢視裡這是
Overwrite 中的一個新檔案;Eidos 把它移進名為 `Cleaned plugins` 的模組,這個模組建立時是
啟用狀態,位於清單底部,在所有衝突中勝出。原模組裡的外掛和遊戲 Data 資料夾裡的外掛永遠
不會被寫入。要復原一次清理,停用 `Cleaned plugins`,或隱藏其中的一個檔案。位於 Overwrite
裡的外掛不會被清理,因為 xEdit 會自己重寫那個檔案;請先把 Overwrite 變成模組。

xEdit 的備份、快取和記錄檔放在合併檢視之外的 `<instance>/logs/qac-<time>/`,狀態列會回報
從這份記錄檔讀到的數量。清理成功後 Eidos 會刪除這個資料夾,失敗則保留。Eidos 會把每個
副本所依據的外掛的大小和日期記錄在 `Cleaned plugins/meta.ini` 裡。該外掛變更後,
Diagnostics 分頁會顯示 Out-of-date copy of <plugin>,並附一個隱藏舊副本的按鈕。

## 加一個進去

在 GUI 裡:選單列的 Tools,或工具列上的 Executables,然後按 Add。在命令列裡:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse rm BodySlide          # remove an entry you added
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # run an exe once, without registering it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` 會把該次執行的輸出收進 `mods/<mod>`,不論工具本身如何設定。`rm` 只會
移除你加的項目;每款遊戲的預設項目可以隱藏,不能移除。

用 `--exe` 執行的 exe,如果它的檔名(不分大小寫)是 Eidos 為該遊戲所認識的,就會得到那個
工具的標題,連同下面的前置需求、參數和擷取模組。`GenerateFNISforUsers.exe` 就是 FNIS,
所以上面那行 `--exe` 會要求 `dotnet48` 並擷取到 `FNIS Output`。其他任何 exe 都以檔名作為
標題。在 Data 分頁裡,`.exe` 那一列有 Add tool(開啟 Executables 對話框並填好該項目)和
Run(像 `--exe` 一樣把該檔案執行一次)。兩者替檔案取標題的方式相同。

對於不是 Windows 工具的指令,`eidos shell skyrimse` 會在遊戲資料夾裡開啟你的 `$SHELL`
(未設定時用 `/bin/sh`),並掛載好合併檢視。shell 寫入的內容進入 Overwrite,離開 shell
就卸載檢視。`eidos shell skyrimse -- <command>` 則只執行一條指令。在互動式 shell 裡按
Ctrl+C 會停止你輸入的指令,shell 保持開啟。shell 不經過 Proton 執行;Windows 程式要走
`eidos tool`。

在視窗裡,Stop 可以結束卡住的工具執行:按兩次,Eidos 會結束這次執行的程式,然後像任何
結束之後一樣處理它的檔案。開啟 Notify the desktop when a long job ends(Preferences,
General, Running a game)時,視窗在背景時結束的工具執行會發出桌面通知。

script extender、遊戲主程式與啟動器都會自動偵測;只有額外的工具需要註冊。在從遊戲副本
執行的實例上,它們是在副本裡偵測的;見[遊戲副本上的工具](#遊戲副本上的工具)。

### 指向真正的檔案,不管它在哪

執行檔實際在哪裡,就註冊在哪裡。如果工具是以模組方式安裝的,那就在模組資料夾裡面:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

那是全域實例的路徑。可攜實例套用同樣的規則,只是在它自己的資料夾底下,
`<instance>/mods/...`。像這樣的絕對路徑,是日後移動可攜資料夾時唯一撐不過去的東西。

Eidos 在啟動前會把那個路徑改寫成合併後的路徑,所以工具是從
`<game>/Data/CalienteTools/BodySlide/` 執行,也能在那裡看到其他每一個模組的檔案。
BodySlide 附的 `SliderSets` 目錄是空的,它能建出的每一具身體都來自 CBBE 與服裝模組。
從它自己的模組資料夾啟動,它什麼都找不到,看起來就像壞了。

MO2 也做同樣的改寫,理由相同:它自己的註解點名了 FNIS。

位於已停用模組裡的工具無法被改寫,因為它的檔案同樣不在檢視裡。Eidos 會直說,並從它自己
的資料夾執行。

## 來自 Mod Organizer 2 清單的工具

以 Eidos 實例開啟的 Wabbajack 清單或其他 MO2 實例會保留自己的 `ModOrganizer.ini`,其中
設定的可執行檔可以複製到 `tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # show what would be added
eidos tool /path/to/list import-mo2 --write   # add them
```

在視窗中,Executables 對話框有 Import from ModOrganizer.ini,每個項目都有可取消勾選的
核取方塊。Add selected 會把勾選的項目放進清單,按下 Save 之前不會儲存任何內容。只有不是
Eidos 寫的 `ModOrganizer.ini` 才算;Eidos 寫的那份見
[PGPatcher 與 PGLite](#mod-organizer-2-模式下的-pgpatcher-與-pglite)。

`Z:` 上的路徑是 Linux 路徑,維持原樣。其他磁碟機代號上的路徑如果包含清單資料夾的名稱,
就會移到清單資料夾底下(`D:\Lists\MyList\tools\SSEEdit.exe` 變成
`<list>/tools/SSEEdit.exe`),可執行檔、工作目錄與引數都照此處理。script extender 的
載入器、啟動器與遊戲主程式會變成單純的檔名,因此和內建項目一樣,從 Eidos 掛載檢視的
遊戲資料夾執行。MO2 的 `hide` 變成 Hide from picker,`steamAppID` 變成 Steam AppID。
指向其他位置的項目(Windows 系統程式、清單之外的工具)會連同原因列為已略過,`tools.ini`
中已有的標題則永遠不會被取代。

匯入的項目保留 MO2 的標題和引數,Eidos 不會依標題填入任何東西:沒有前置需求、沒有額外
參數,也沒有擷取模組。請在 Executables 對話框中設定它們。在 Windows 上製作的清單裡的
PGPatcher 項目通常缺少 `--ignore-mo2vfscheck`;請加上(見
[下文](#為什麼-pgpatcher-需要---ignore-mo2vfscheck))。

指向清單 `Stock Game` 資料夾內的匯入路徑,只有在實例從那份副本執行時,才會在執行時移到
商店安裝上;見下一節。

## 遊戲副本上的工具

當實例從遊戲副本執行時(清單的 `Stock Game` 或 `Game Root` 資料夾,或凍結的副本;見
[遊戲檔案](usage.md#遊戲檔案商店安裝還是副本)),工具看到的是副本的檔案,而不是商店
安裝的檔案。合併檢視仍然掛載在商店安裝上:root 檢視把副本放在最底下,Data 檢視讀的是
副本的 `Data`。Eidos 會在副本而不是商店安裝裡尋找 script extender、啟動器、遊戲主程式
和已知的工具。

指向副本內的工具路徑、引數或工作目錄,會在執行前移到商店安裝上。它可以寫成 Linux 路徑
或 Wine 的 `Z:` 路徑:`-D:"Z:\...\Stock Game\Data"` 會變成商店安裝底下的同一條路徑。
從清單的 `ModOrganizer.ini` 匯入的項目需要這一點。當實例從商店安裝執行時不會移動任何
東西,所以這樣的項目讀到的是不含你的模組的原始副本。

前綴登錄檔裡遊戲的 `installed path`,以及 Eidos 所寫的 `ModOrganizer.ini` 裡的
`gamePath`,仍然指名商店安裝,也就是合併檢視所在的位置。如果副本資料夾不見了,或裡面
沒有遊戲執行檔,工具執行會像遊戲啟動一樣停下,訊息會指名 `eidos stock-game <instance> store`
以回到商店安裝。

`eidos stock-game <instance>` 顯示實例執行的是哪份遊戲檔案,以及它可以切換到哪些副本。
`use <folder>` 和 `store` 用來切換,`freeze --yes` 則把商店安裝複製到實例的
`Stock Game` 資料夾並從它執行。Diagnostics 分頁裡的 Game files 卡片做的是同一件事;見
[遊戲檔案](usage.md#遊戲檔案商店安裝還是副本)。

## 把工具的輸出送進它自己的模組

產生器 - FNIS、Nemesis、BodySlide、DynDOLOD、Synthesis - 會寫出好幾百個檔案。
預設情況下它們和其他東西一起落進 Overwrite。在 Executables 編輯器裡設定
Capture output into,這一次執行的輸出就改為進入那個模組:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora、Nemesis、FNIS、BodySlide 和 Synthesis 加入時已設好擷取模組:`Pandora Output`、
`Nemesis Output`、`FNIS Output`、`BodySlide Output` 和 `Synthesis Output`。DynDOLOD、
TexGen 和 xLODGen 沒有預設,因為它們寫入你指定的輸出資料夾。

模組不存在就會被建立。只有這一次執行產生的檔案會被搬走;原本就在 Overwrite
裡的東西留在原地,所以兩個各有擷取目標的工具不會互相搶走輸出。什麼都沒寫的執行
不會留下一個空模組。

Eidos 在執行結束之後才搬檔案。MO2 則是把寫入層指向那個模組,這會讓它在整段執行期間被
提升到最高優先權:它牽涉到的每一個衝突都翻一次,結束後再翻回來。寫入也會直接寫穿模組
自己的檔案,沒有 copy-up。擷取達到同樣的最終狀態,而兩者都不需要。

如果目標模組是停用的,輸出仍然會寫出去,但遊戲看不到,於是工具下次執行時又會
重新產生同樣的檔案。遇到這種情況 Eidos 會提出警告。

### 輸出過期時

Eidos 會記錄工具的輸出是由哪些模組、外掛順序和設定檔產生的、工具執行檔的大小和日期,以及
`tools.ini` 裡該項目的路徑和引數。其中任何一項改變,Diagnostics 分頁都會顯示
Generated output inputs changed,卡片上還有一個 Run <tool> again 按鈕,依目前設定重新
執行該工具。

對於輸出過期就會弄壞遊戲的產生器(例如 Pandora 或 Nemesis),可以在 `tools.ini` 裡它的
項目下加上 `check_before_play=1`,或在 Executables 對話框中為它開啟 Ask before playing
when its output is out of date。之後 Run 會先詢問:執行該工具,還是照常遊玩。選擇
Play anyway 後,會記住到模組、外掛或設定檔再次改變,或視窗關閉為止。Eidos 從不自己啟動
該工具。開啟 Start the game at once when a store opens Eidos 時,這樣的詢問會讓視窗保持
開啟,而不是直接啟動遊戲。

## 前置需求與參數由標題決定

你給工具取的標題,決定了 Eidos 會替它準備哪些執行階段前置需求。比對方式是對標題做不分
大小寫的子字串比對。

| 標題若含有 | Eidos 會請求 |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| 其他任何情況 | 無 |

所以註冊成 `BodySlide` 的工具會拿到它的 DirectX DLL;同一個執行檔註冊成 `BS` 就什麼都
拿不到,可能啟動失敗,而錯誤訊息完全沒提到 DLL。工具就照程式的名字命名。

這份清單在 `default_prereqs`(`crates/eidos-instance/src/tools.rs`)裡,而
Executables 對話框裡的 `Prereqs` 欄位可以編輯:標題只填入預設值。

標題同樣決定參數,只為那一個需要誰也猜不到的參數的工具:標題中含 `pgpatcher`(或它的
舊名 `parallaxgen`)就會得到 `--ignore-mo2vfscheck`。原因,以及它為何不是選用項,見
[下文](#為什麼-pgpatcher-需要---ignore-mo2vfscheck)。擷取模組也由標題決定(見
[上文](#把工具的輸出送進它自己的模組))。

### 三種前置需求

第一類是隨附的 DLL:`d3dx9_43`、`d3dcompiler_47`、`d3dcompiler_43` 和 `d3dx11_43`。
Eidos 把它們帶在自己的執行檔裡,並在啟動時複製進前綴。在 64 位元前綴(Proton 的預設)
裡,64 位元版本放進 `system32`,32 位元版本放進 `syswow64`。不必做任何事,不用網路。
每個版本都以 gzip 壓縮存放,所以八個檔案在每個執行檔裡佔 7.9 MB,而不是 17.9 MB,而且
只有正在複製的那一個會被解開(最多 4.7 MB)。

第二類是安裝程式:`vcrun2022`、`dotnet6`、`dotnet7`、`dotnet8`、`dotnetdesktop6`、
`dotnetdesktop8`、`dotnet48`、`xact` 和 `xact_x64`。它們會寫入登錄機碼、GAC 與 CLR host,
所以沒辦法用複製檔案解決。其中三個由 Eidos 直接安裝,不經過 winetricks:`vcrun2022`
執行 Microsoft 的 `VC_redist.x86.exe` 與 `VC_redist.x64.exe`,`dotnet8` 與
`dotnetdesktop8` 執行 .NET 8.0.12 Desktop Runtime 安裝程式,它也包含 .NET 執行階段。
發行套件(`eidos-linux-bundle.tar.gz`)在 `runtimes/` 資料夾裡附上這些安裝程式,
`install.sh` 會把它們複製到執行它的使用者的 `~/.local/share/Colony/Eidos/runtimes/`,
加 `--system` 時也一樣。Eidos 先在那裡找每個檔案,再找 `eidos` 執行檔旁邊的 `runtimes/`
資料夾(原地執行的解開套件)。兩份副本都沒有時(單一檔案 `eidos-linux`、從原始碼建置,
或系統安裝的另一位使用者),Eidos 會從 Microsoft 下載
(`download.visualstudio.microsoft.com`)到 `~/.local/share/Colony/Eidos/runtimes/` 並
保留在那裡。不論哪種情況,SHA-256 與編進 Eidos 的值不符的檔案都會被拒絕,檔案會在遊戲
前綴裡以無視窗方式(`/install /quiet /norestart`)執行,用的是執行該遊戲工具的那個
Proton 或 Wine 版本。和 winetricks 一樣,`vcrun2022` 會先在前綴的 DLL 覆寫設定裡把
Visual C++ 的 DLL 設成 `native,builtin`。之後,對於 Proton 版本,Eidos 會檢查前綴裡的
C++ 執行階段是不是一半來自 Microsoft、一半來自 Proton。若是,這次安裝就會失敗:訊息會
點名來自不同版本的 DLL 配對,並說明改用 winetricks 安裝 `vcrun2022`,因為它自己會取代
`msvcp140.dll`。只會安裝 64 位元的 .NET 8;32 位元的 .NET 8 工具仍然需要
`winetricks dotnetdesktop8`。

其他六個 verb 則透過系統的 `winetricks`,它必須在 `PATH` 上。Eidos 用執行該遊戲工具的
Proton 或 Wine 版本的 `wine` 來執行它,針對遊戲前綴,這樣就繞開了 Steam 的
pressure-vessel 容器,以及 protontricks 與 Proton-GE 不相容的問題。有些 winetricks verb
需要 `cabextract`,缺少時 Eidos 會警告。宣告了未安裝的第二類 verb 的工具照樣會啟動,
只是帶一則警告,點名該 verb 以及修好它的命令,因為你可能已經從別處裝了。

當前綴自己的 `winetricks.log` 列出某個第二類 verb 時(不論是誰執行了 winetricks 或
protontricks),或 Eidos 已把它安裝進同一個前綴時,該 verb 就算完成。Eidos 把自己的安裝
記錄在實例的 `prereqs.done` 裡,並與前綴的路徑和 inode 綁定,所以重建後的前綴會把它的
verb 重新顯示為待處理。Rebuild prefix(Steam 副本在 Diagnostics 裡的操作)會把
`compatdata/<appid>` 重新命名為 `<appid>.eidos-backup-<time>`,不刪除任何東西;Steam 在
下次啟動遊戲時會建立新前綴。

第三類是執行階段,`dotnet10` 和 `dotnetsdk10`。現代的 .NET 執行階段是 193 個檔案,住在
自己的目錄裡,透過 `DOTNET_ROOT` 被找到。它從不註冊,也完全不安裝進前綴,所以另外兩類
都載不動它。Eidos 把它放在任何實例之外,因為 78 MB 不是每個遊戲一份,也不是每個設定檔
一份:每個執行階段解開到 `~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/`
(`dotnet10-10.0.10`、`dotnetsdk10-10.0.403`)。Eidos 在與第二類安裝程式相同的兩個資料夾
裡,依下載檔名尋找執行階段的 zip,並解開發行套件附帶的那份(`dotnet-runtime-10.0.10-win-x64.zip`,
只供 `dotnet10` 使用),或把它下載下來。套件不附帶 SDK 的 `dotnet-sdk-10.0.403-win-x64.zip`,
但手動放在那裡的副本會被使用,並省下下載。任何一個檔案的 SHA-256 與編進 Eidos 的值不符
都會被拒絕。它用 `7z` 解開 zip,也就是它用於模組壓縮檔的同一個 7-Zip。

`dotnetsdk10` 是 .NET 10 SDK,下載 301 MB,解壓縮後 808 MB。
Synthesis 編譯 patcher 需要它,而且建置與執行 patcher 時呼叫的是不帶路徑的 `dotnet`,
所以宣告了 `dotnetsdk10` 的工具還會在 `WINEPATH` 上拿到 SDK 資料夾,外加
`NUGET_CERT_REVOCATION_MODE=offline`、`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` 與
`DOTNET_CLI_TELEMETRY_OPTOUT=1`。建置 patcher 會留下仍在執行的 MSBuild 節點與
VBCSCompiler 伺服器,在 Wine 下它們會讓前綴的 wineserver 一直存活,所以該工具還會拿到
`MSBUILDDISABLENODEREUSE=1`、`DOTNET_CLI_USE_MSBUILD_SERVER=0` 與
`UseSharedCompilation=false`。`DOTNET_ROLL_FORWARD=Major` 讓為 .NET 8 建置的 patcher
(Mutagen 0.54 之前)在找不到 .NET 8 執行階段時,改用 SDK 附帶的 .NET 10 執行階段執行。
`tools.ini` 中仍寫著舊預設值 `dotnet8`, `vcrun2022` 的 Synthesis 項目,會在 Eidos 讀取
時改成 `dotnetsdk10`, `vcrun2022`。

NuGet 會對照 Windows 根憑證存放區檢查它還原的每個套件的簽章,而 Wine 前綴裡沒有 SDK 以
`sdk/10.0.403/trustedroots/codesignctl.pem` 與 `timestampctl.pem` 附帶的程式碼簽署根憑證
與時間戳記根憑證。沒有它們,還原 `Mutagen.Bethesda.Synthesis` 0.36.6 會以 `NU3018` 與
`NU3028` 失敗。因此,在執行宣告了 `dotnetsdk10` 的工具之前,只要 SDK 已經下載,Eidos
就會把這兩個檔案中的 385 個憑證加入遊戲前綴裡目前使用者的根憑證存放區,用的是寫入遊戲
路徑的同一次 `regedit` 匯入(見下文)。如果其中某個憑證遺失,Eidos 會重新匯入。此後,
該前綴裡的每個 Windows 程式都信任這些根憑證。有了它們,在全新的前綴與空的 NuGet 快取下,
同樣的還原與建置在系統 Wine 11.21 與 GE-Proton11-9 上都能成功。

在你提出要求之前,Eidos 不會安裝第二類或第三類的任何東西:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # install what is missing (the bundle's copies first, then downloads)
```

在 GUI 裡,同樣的狀態就在 Prereqs 欄位底下,缺少的那些是按鈕。既不是隨附的、
也不是執行階段、又不是第二類 verb 的項目,會被回報為可能的拼字錯誤,
而不是提供下載。

`eidos prereqs <instance> --install` 在遊戲、Steam 或某個工具仍占用前綴時會拒絕。第三類
執行階段在前綴存在之前就能安裝。Eidos 自己執行的安裝程式需要前綴已經存在:在商店啟動過
遊戲一次之前,它們會以「start the game once through its store, then install again」停下。
第二類 verb 一次安裝一個,每次成功都會被記錄,所以失敗之後同一條命令可以接著做。對於
沒有 Heroic 設定的 GOG 或 Epic 副本,`--install` 會複製隨附的 DLL,然後在第二類 verb
之前停下:它會要你使用該執行器自己的 winetricks,並在下次檢查時讀取前綴的
`winetricks.log`。

### 為什麼 DynDOLOD 需要 `dotnet10`

DynDOLOD 自己不建 object LOD:它把工作丟給 LODGen,而且附了三個版本。
`LODGenx64.exe` 的目標是 .NET Framework 4.8,在 Proton 底下會被導向 Wine 的
Mono - 而 Mono 的 `System.Uri` 初始化程序會呼叫一個 Mono 沒有實作的方法。LODGen 在做
第一行正事之前就掛掉,留下一份只有版本橫幅、其他什麼都沒有的記錄檔,以及一個
只寫著「failed for one or more worlds」的 DynDOLOD 對話框。

安裝真正的 .NET Framework 也修不好:Proton 把 `mscoree.dll` - 也就是本該找到它
的載入器 - 換成指向自己目錄樹的 symlink,而且每次前綴更新都會重做一遍。

能用的版本是 `LODGenx64Win10.exe`,它的目標是現代 .NET,完全不碰 `mscoree`。把
`DOTNET_ROOT` 指向一份 .NET 10 執行階段,它就跑得起來。`dotnet10` 準備的就是
這個,而 Eidos 在啟動任何宣告了它的工具時會設好這個變數。

### 為什麼 Pandora 需要 `dotnet10`

Pandora 是 Avalonia 應用程式,不是 WPF,自 Pandora 4.1.0 起它的目標就是 `net10.0`,所以
光是 .NET 10 執行階段就能執行它。找不到執行階段時,它會說要安裝 .NET Desktop Runtime。
那是 .NET 啟動器對任何找不到執行階段的 GUI 程式顯示的通用文字,而 `dotnetdesktop8`
修不好它。

Eidos 1.21.0 和更早版本給 Pandora 的是 `dotnetdesktop8`,而項目會保留它的 `prereqs=` 行。
當 Eidos 讀到一個仍然只列 `dotnetdesktop8` 的 Pandora 項目,而它執行檔旁邊的
`.runtimeconfig.json` 指名 `net10.0` 時,該項目就改為 `dotnet10`。Pandora 2.x 的目標是
.NET 8,保留它原有的設定。

### 為什麼 PGPatcher 需要 `--ignore-mo2vfscheck`

PGPatcher 會重寫網格與外掛,讓你安裝的貼圖模組(原版、視差、complex material、PBR)在
每個表面上都用對的著色器。它拒絕在模組管理器之外執行,因為對一個未經管理的遊戲資料夾打
補丁是破壞性的。

PGPatcher 會檢查 usvfs,也就是 MO2 在 Windows 下注入遊戲的 DLL 掛鉤層。Eidos 沒有:
合併檢視是一個 FUSE 掛載,Wine 走進去時根本不知道它存在。所以檢查什麼也找不到,工具在
做任何事之前就立刻結束:

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

而且它點名的還是使用者根本沒在執行的程式。作者本人對兩份 Linux 回報
([#716][pg716] 與 [#725][pg725],都來自 Fluorine,即 MO2 的 Linux 分支,基於同樣的原因
採用 FUSE)給出的答案,就是一個跳過該檢查的旗標。

因此 Eidos 會替你預填。在模組池中被找到的 PGPatcher 出現時參數已經設好;在 Executables
對話框的標題輸入 `PGPatcher`,Arguments 欄也會以同樣方式被填上。你輸入的內容永遠優先:
預填只會填你留空的欄位,與上面的前置需求規則一致。

清單在 `default_args`(`crates/eidos-instance/src/tools.rs`),緊鄰 `default_prereqs`。

執行時它什麼都不需要,這對 .NET 工具來說少見:它自帶執行環境(`dotnetlib/`,附有宣告
`includedFrameworks` 的 `runtimeconfig.json`),並把 Microsoft CRT 的 DLL 放在執行檔旁邊。
Prereqs 留空即可。

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### Mod Organizer 2 模式下的 PGPatcher 與 PGLite

Mod Manager 設為 `None` 時,PGPatcher 只能看到一個合併後的 `Data` 資料夾,無法得知某個
網格或貼圖來自哪個模組,因此既沒有衝突可顯示,也沒有模組順序可遵循。它的
`Mod Organizer 2` 模式直接讀取實例本身:設定檔的 `modlist.txt`、依優先順序排列的已啟用
模組,以及每個模組資料夾裡的檔案。遊戲內版本 PGLite 也以同樣方式讀取實例。Eidos 實例
本來就是 MO2 的配置(`mods/`、`profiles/<name>/modlist.txt`、`overwrite/`),所以這個
模式可以直接用於它。

在 PGPatcher 的啟動器中:

1. 把 Mod Manager 設為 `Mod Organizer 2`。
2. 把 MO2 Options -> Instance Location 設為 Eidos 實例資料夾,寫成 Wine `Z:` 磁碟
   (即 `/`)上的 Windows 路徑。位於 `/mnt/Games/Eidos-Skyrim` 的可攜實例寫作
   `Z:\mnt\Games\Eidos-Skyrim`;全域實例寫作
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`。
3. 在工具的參數中保留 `--ignore-mo2vfscheck`。在這個模式下,PGPatcher 會尋找 usvfs,
   找不到就停止。

PGPatcher 只接受含有 `modorganizer.ini` 的資料夾,所以 Eidos 每次啟動遊戲或工具時,都會
為所啟動的設定檔在實例根目錄寫入 `ModOrganizer.ini`。只有內容變更時才會重寫。檔案包含:

- `[General]` 中:`gameName`(MO2 中的遊戲名稱,因此 Fallout: New Vegas 寫作
  `New Vegas`)、`gamePath`(遊戲商店安裝資料夾,以 `Z:` 路徑表示,從遊戲副本執行的
  實例上也一樣)與 `selected_profile`;
- `[Settings]` 中:以絕對 `Z:` 路徑表示的 `mod_directory`、`profiles_directory` 與
  `overwrite_directory`,實例有 `downloads/` 資料夾時還有 `download_directory`。

檔案第一行是一條註解,說明它由 Eidos 寫入。沒有這一行的 `ModOrganizer.ini`,無論檔名
如何拼寫,Eidos 都不會修改或刪除:你在 Eidos 中開啟的 MO2 實例保留它自己的檔案,
PGPatcher 照原樣讀取。MO2 儲存設定時會丟棄註解,所以被 MO2 覆寫儲存過的檔案從此歸 MO2
所有。空檔案算作 Eidos 的檔案。

這樣的檔案記錄的是 MO2 最後選擇的設定檔,PGPatcher 使用這個設定檔,而不是 Eidos 啟動的
那個。兩者不同時,Eidos 會在啟動時記錄一則警告。請在 MO2 中選擇同一個設定檔,或者重新
命名或刪除該檔案,讓 Eidos 寫入自己的檔案。

Mod Organizer 2 模式下的工具也從該檔案的 `gamePath` 讀取遊戲。當它指名的既不是實例所
執行的遊戲資料夾,也不是商店安裝時,Diagnostics 分頁會顯示
ModOrganizer.ini points tools at another game folder,附上該路徑和這個實例所執行的遊戲
資料夾。請修改檔案裡的 `gamePath`,或移除該檔案,讓 Eidos 寫入自己的檔案。

Eidos 只在啟動某個東西時寫它自己的檔案,所以在兩次啟動之間,它記錄的是上一次啟動的
設定檔。由 Eidos 啟動的工具能在其中找到自己的設定檔。

當設定檔中沒有任何已啟用的模組,或輸出資料夾就是某個已啟用模組的資料夾時,PGPatcher
會報錯並停止,和在 MO2 下一樣。

## Wine 自己的程式在遊戲前綴裡

File 選單會在遊戲的前綴中執行 Wine 的四個程式:winecfg、regedit、Wine 檔案總管和解除
安裝程式。它們和其他工具一樣啟動,使用執行遊戲工具的 Proton 或 Wine 版本,並掛載好
模組,視窗會等它們結束。Prefix drive_c 會在檔案管理員中開啟前綴的 `C:` 磁碟。前綴裡還
沒有某個程式時,對應的列顯示為灰色,商店第一次啟動遊戲後就會有。在終端機裡,
`eidos tool <instance> run winecfg` 做的是同一件事,`regedit`、`explorer` 和
`uninstaller` 也是,只要清單裡沒有工具用了那個標題。

你在 regedit 裡對遊戲的 `installed path` 做的修改,會在下次執行工具時被改回來;見下一節。

## 前綴裡的遊戲路徑

Windows 工具是靠讀 `HKLM\Software\Bethesda Softworks\<game>` 的 `installed path` 來找
遊戲的,那把機碼由遊戲自己的安裝程式寫入。Steam 在 Proton 底下從來不跑那個安裝程式。
沒有它,xEdit、Wrye Bash 與 DynDOLOD 開起來就是一個空路徑。

執行工具之前,Eidos 會在遊戲前綴裡用 `regedit` 匯入來寫入該機碼,除非先前某次匯入已經
寫了同一條路徑而且前綴仍保有它。該機碼指名的是商店安裝,從遊戲副本執行的實例上也一樣,
因為合併檢視就掛載在那裡。對於宣告了 `dotnetsdk10` 的工具,同一次匯入還會加入 NuGet
簽署根憑證(見[上文](#三種前置需求))。`runner=native` 的工具會跳過這一步。前綴尚不存在,
或另一個程式占用它時,工具照樣會執行,Eidos 會印出警告,指出該從哪個商店啟動一次遊戲,
或該關閉哪個程式。當機碼指名的資料夾不是商店安裝時,Diagnostics 分頁會顯示
Windows tools cannot find <game>,直到下次工具執行再次寫入它。

## 找到工具:隱藏、置頂,以及桌面捷徑

一款遊戲的預設項目裡有些工具你可能從來不用。在 Executables 對話框裡:

- Pin to top 把項目放到 Run 清單的最前面。
- Hide from picker 把項目從選單裡拿掉,但不刪除它。
- Desktop shortcut 會把一個 `.desktop` 寫進 `~/.local/share/applications`,所以它會出現
  在你的應用程式選單與搜尋裡,而不是出現在桌面上。它直接執行
  `eidos tool <instance> run <title>`,也就是說,即使 Eidos 視窗根本沒開著,工具也是帶著
  這個實例的設定檔、穿過合併檢視啟動的。

隱藏與置頂只改變工具怎麼被找到,所以它們對每款遊戲的預設項目和你自己的項目一樣有效。

script extender 的載入器、啟動器與遊戲主程式沒有 Desktop shortcut 按鈕。它們會啟動遊戲,
而遊戲一律透過它的商店啟動,這件事商店本身的捷徑已經在做。用 Eidos 1.22 或更早版本替
其中某一項建立的捷徑仍然能用:和在終端機裡輸入的 `eidos tool <instance> run <title>`
一樣,它會請商店啟動遊戲,就像 Run 在不是由商店開啟的視窗裡做的那樣。商店的設定缺少時,
這個指令什麼也不啟動,並印出要設定什麼。捷徑不會顯示任何訊息,所以要看它,就在終端機裡
執行同一個指令。

只要項目的執行檔檔名(不分大小寫)是 script extender 的載入器、啟動器或遊戲主程式,
Eidos 都會這樣把它交給商店:預設項目、你加的項目、從 MO2 匯入的項目,或 `--exe` 執行。
它會先把這個實例標記為最近使用的一個,讓商店開啟的視窗顯示這個實例,然後執行
`steam -applaunch <id>`,Heroic 則是
`xdg-open heroic://launch?appName=<app>&runner=<runner>`,或 `legendary launch <app>`。
加上 `--print` 時,它印出那條商店命令而不啟動任何東西。`steam` 命令必須在 `PATH` 上:
Flatpak 版 Steam 不會把它放在那裡,所以啟動會失敗,訊息會要你從 Steam 啟動遊戲。

當商店已啟動了行程時,項目像任何工具一樣直接穿過合併檢視執行:`SteamGameId`、
`SteamAppId`、`HEROIC_APP_NAME`、`HEROIC_APP_RUNNER` 或 `EIDOS_STORE_STARTED` 其中之一
已設定。沒有其中任何一個時,Eidos 不會自己用 Proton 啟動遊戲。當實例沒有 script extender
載入器(在它所執行的遊戲資料夾、已啟用模組的 `Root/` 或 `Overwrite/Root/` 裡)時,什麼都
不會啟動,訊息會要你先安裝 script extender。對於 Heroic 之外的 GOG 副本,或 Eidos 無法
轉交的商店 id,訊息會要你從那個商店啟動遊戲。

設定檔可以有自己的遊戲捷徑。在設定檔上按右鍵並選擇 Desktop shortcut:這個項目會執行
`eidos tool <instance> run "<game>" --profile <name>`,它會像在視窗裡選取該設定檔那樣
把它設為目前的設定檔,然後請商店啟動遊戲。商店開啟的 Eidos 視窗顯示的就是這個設定檔。
`--profile` 適用於任何 `eidos tool ... run`,會在工具啟動前選取設定檔;與 `--print` 一起
使用時,它什麼也不選。

## 本身就是獨立 Steam 應用程式的工具

Creation Kit 是獨立的 Steam 應用程式,要用自己的 AppID;少數幾個透過 Steam 發行的模組
工具也一樣。在項目上設定 Steam AppID,Eidos 就會用那個 id 而不是遊戲的 id 來啟動它。

在 Windows 上這代表換一個啟動器。在這裡,它只是同一次執行上的兩個環境變數,
`SteamAppId` 與 `SteamGameId`。Eidos 兩個都設,因為 Proton 讀其中一個,而 Steam 自己的
函式庫讀另一個,工具若看到兩者不一致,失敗時不會有清楚的錯誤。`eidos tool ... --print`
會原原本本顯示真正執行時會拿到什麼。

## 環境變數,以及不經過 Proton 的工具

`<instance>/tools.ini` 裡有三個鍵在 Executables 對話框中還沒有對應的欄位,需要手動寫進
工具所在的區段。每一行 `env=NAME=value` 設定一個只對這個工具的執行生效的變數:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos 在設定完它自己的所有變數之後,以及設定檔 `launch-env.ini` 裡的變數之後,才套用這些
行,所以這裡寫的值會取代兩者。這也包括 Eidos 依模組附帶的 DLL 組出來的
`WINEDLLOVERRIDES`:設定了它的工具,必須把自己需要的每個 DLL 都列進去。設定檔的包裝器
會包在工具的命令外面,就像包在遊戲的命令外面一樣;見
[每個設定檔的變數與包裝器](usage.md#每個設定檔的變數與包裝器)。變數名稱由字母、數字和
`_` 組成,不以數字開頭。名稱不合規則的行會被忽略。

`runner=native` 不經過 Proton,直接執行 `exe` 本身。合併檢視照常掛載,`arg0=`、`arg1=`
這些行就是它的參數。Linux 程式,或 Steam 不管理、Heroic 也沒有記錄執行器的副本上的
工具,都用這種方式:`exe` 是那個副本的 Wine,Windows 程式是第一個參數,前綴由 `env=`
指定。

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` 必須是絕對路徑。工作目錄預設是 `exe` 所在的資料夾,在這裡就是執行器的資料夾,所以
要設定 `workdir=`。只有 `exe` 會被改寫成合併後的路徑:模組裡的 Windows 程式,要寫它在
遊戲 `Data` 資料夾下的路徑,合併檢視就是在那裡顯示它的。原生工具會跳過 Proton 的步驟:
上面講的登錄機碼、Steam 的著色器快取、空的 .NET 根目錄,以及從 `LD_PRELOAD` 中拿掉
Steam 覆蓋層。

用 Heroic 安裝的 GOG 或 Epic 副本,如果遊戲的 Heroic 設定(`GamesConfig/<appName>.json`)
指定了 Wine 或 Proton 版本,工具就不需要這樣的項目:Eidos 用那個版本在遊戲的 Heroic
前綴裡執行它們,`eidos prereqs <instance> --install` 也把它用於安裝程式與 winetricks。
只用 Legendary 安裝的副本沒有 Heroic 設定。

`slr=1` 會讓工具及其之前的登錄檔匯入,在工具的 Proton 版本所指定的 Steam Linux Runtime
容器裡啟動(見其 `toolmanifest.vdf` 中的 `require_tool_appid`),與 Steam 啟動遊戲的方式
相同。這個執行階段必須裝在某個 Steam 媒體庫中;沒有裝的話,執行會停下,並指出需要安裝的
Steam 應用程式。`eidos prereqs` 仍在主機上執行它的安裝程式與 winetricks,
`runner=native` 的工具會忽略這個鍵。容器能看到你的家目錄與遊戲所在的 Steam 媒體庫。
放在其他地方(例如另一顆磁碟上)的工具、模組或執行階段,需要
`env=STEAM_COMPAT_MOUNTS=/那個/路徑`(多個路徑以 `:` 分隔)。裝在
`/usr/share/steam/compatibilitytools.d` 下的 Proton 很可能無法搭配 `slr=1` 使用,因為
容器裡的 `/usr` 是執行階段自己的。目前還沒有在容器裡實際執行過工具。

`eidos tool <instance> run <title> --print` 會顯示最終的命令,以及這次執行在繼承的環境
之上設定的每一個變數,包括這些行。對於會啟動遊戲的項目,它改印出商店命令(見
[找到工具](#找到工具隱藏置頂以及桌面捷徑))。

## BethINI Pie

Eidos 不附帶 BethINI Pie。把從它的 Nexus 頁面下載的版本放進此實例的 `mods/` 或你的工具
資料夾,`Bethini.exe` 就會出現在工具清單中,所在遊戲與 PGPatcher 相同。透過 Eidos 執行
時,它編輯的是設定檔自己的遊戲 INI 副本。每次執行標題含有 `bethini` 的工具之前,Eidos
會保留這些 INI 的一份還原點;工具列上的 Backups,然後 INIs,可以還原其中一份。副本建立
時尚不存在的 INI 維持原樣。對話框為每一種還原點保留最新的 10 份,而還原其中一份時會先
備份目前的 INI,所以選錯了也能復原。

## 工具自己的設定仍然是它自己的

Eidos 不檢查大多數工具設定;錯誤的設定通常會不報錯地失敗。

BodySlide 的 Game Data Path(Settings)必須指向遊戲的 `Data` 目錄,而不是它上面那層遊戲
資料夾。設高一層,批次建置會回報「All sets processed successfully」,然後把 1439 個
網格寫到遊戲永遠不會去找的地方。Eidos 會在 `Overwrite/Root/` 裡接住它們,所以你的遊戲
安裝不受影響,而在遊戲裡唯一的徵兆就是你的身體模型沒建出來。

在執行 BodySlide 或 Outfit Studio 之前,如果勝出的 `CalienteTools/BodySlide/Config.xml`
裡的 `GameDataPath` 或 `OutputDataPath` 在執行期間沒有通到遊戲的 `Data` 資料夾,狀態列
會發出警告,Diagnostics 分頁也會把同一個警告顯示為一張卡片,並給出該設定的值。
`eidos check <instance>` 也會回報它。`S:` 路徑依照 Eidos 為工具對應 `S:` 的方式來讀,也
就是 Steam 媒體庫資料夾。位於實例 `mods/` 資料夾內、用於輸出模組的 `OutputDataPath` 會
被接受。當 `Overwrite/Root/` 裡有 `meshes` 或 `textures` 資料夾時,Diagnostics 也會顯示
一張卡片。

工具的輸出本來就該待在 Overwrite。當一次執行產生了值得留下的東西,
Overwrite -> Create mod... 會把它變成一個普通模組,可以像其他模組一樣排序、停用和移除。
