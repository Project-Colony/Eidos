<!-- eidos-i18n: source=docs/guide/tools.md sha=631027d4686fac9eb78728b23a8024d0c5000909 -->

# 工具:xEdit、BodySlide、DynDOLOD, PGPatcher、FNIS

透過 Eidos 執行的工具看到的是**合併檢視**,就在遊戲自己的 Proton 前綴裡。
它讀到的正是遊戲會讀到的東西 - 每一個啟用的模組,按優先順序 - 而它寫出的
任何東西都落進 Overwrite,在那裡一鍵就能變成真正的模組。

## Eidos 自己找得到的那些

有些工具的名字夠獨特,不必宣告就能被找到,xEdit 就是最明顯的例子:Fallout 4
是 `FO4Edit.exe`,Skyrim SE 是 `SSEEdit.exe`,初代是 `TES5Edit.exe`,依此類推 -
連同各自的 **QuickAutoClean** 雙生版,那就是用來處理 LOOT 一直警告的 dirty
edits 的按鈕。Eidos 會依檔名,在這些地方找它們:

- 遊戲的安裝資料夾,以及已啟用模組的 `Root/` 樹;
- **這個實例的 `mods/`**,MO2 使用者就是把工具裝在那裡;
- 你在 Preferences 裡設定的**工具資料夾**(General -> Tools),用於實例之間
  共用的目錄 - 像 `/mnt/Games/Tools` 之類。

`PGPatcher.exe` 在 Creation Engine 系列遊戲中以同樣的方式被找到，並且帶著一個缺了就無法運作的參數 - 見[下文](#為什麼-pgpatcher-需要---ignore-mo2vfscheck)。

清單是按遊戲分的,所以 Skyrim 實例永遠不會被提供 Fallout 的編輯器。搜尋在第
四層就停下,因為一個模組池有數十萬個檔案,而每次建立工具清單時都會跑一次;
它也不跟隨 symlink。這樣找到的工具,設定方式和你自己輸入的完全一樣:它的
執行階段依名字而定,規則與下面所有內容相同。

如果工具在別的地方,或你想用不同的參數,就手動加 - 標題相同的使用者項目會
覆蓋任何自動找到的結果。

## 清理 LOOT 標記為 dirty 的外掛

LOOT 排序之後，Diagnostics 分頁會列出 LOOT 標記為 dirty（需要清理）的外掛，適用於同時有 xEdit 和 LOOT 清理資料的遊戲（Skyrim LE、SE 和 VR，Enderal SE，Fallout 3，New Vegas，Fallout 4 和 4 VR，Oblivion，Starfield）。如果 Eidos 找到了 xEdit（在 Executables 裡登記過，或放在實例中 Eidos 尋找工具的任何位置），卡片上會有 Clean all，每個 dirty 外掛的列選單裡會有 Clean with QuickAutoClean；否則卡片會提示在 Executables 裡加入 xEdit。Eidos 不會下載 xEdit。它透過合併檢視，用 `-qac -autoexit -autoload`，一次對一個外掛執行 xEdit 的 QuickAutoClean 模式。xEdit 把清理後的外掛存回它載入的那個外掛上，在合併檢視裡這是 Overwrite 中的一個新檔案；Eidos 把它移進名為 `Cleaned plugins` 的模組，這個模組建立時是啟用狀態，位於清單底部，在所有衝突中勝出。原模組裡的外掛和遊戲 Data 資料夾裡的外掛永遠不會被寫入。要復原一次清理，停用 `Cleaned plugins`，或隱藏其中的一個檔案。

xEdit 的備份、快取和記錄檔放在合併檢視之外的 `<instance>/logs/qac-<time>/`，狀態列會回報從這份記錄檔讀到的數量。清理成功後 Eidos 會刪除這個資料夾，失敗則保留。Eidos 會把每個副本所依據的外掛的大小和日期記錄在 `Cleaned plugins/meta.ini` 裡。 該外掛變更後，Health 分頁會回報這個副本已過期，並提供隱藏它的選項。

## 加一個進去

在 GUI 裡:**Tools -> Executables**,然後按 Add。在命令列裡:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` 會把該次執行的輸出收進 `mods/<mod>`，不論工具本身如何設定。

script extender、遊戲主程式與啟動器都會自動偵測;只有額外的工具需要註冊。

### 指向真正的檔案,不管它在哪

執行檔實際在哪裡,就註冊在哪裡。如果工具是以模組方式安裝的,那就在模組資料夾
裡面:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(那是全域實例的路徑 - 可攜實例套用同樣的規則,只是在它自己的資料夾底下,
`<instance>/mods/...`;要注意,像這樣的絕對路徑,是日後「移動」可攜資料夾時
唯一撐不過去的東西。)

Eidos 在啟動前會把那個路徑改寫成合併後的路徑,所以工具是從
`<game>/Data/CalienteTools/BodySlide/` 執行,也能在那裡看到其他每一個模組的
檔案。這件事比聽起來重要:BodySlide 附的 `SliderSets` 目錄是**空的**,它能建出
的每一具身體都來自 CBBE 與服裝模組。從它自己的模組資料夾啟動,它什麼都找不到,
看起來就像壞了。

MO2 也做同樣的改寫,理由相同 - 它自己的註解點名了 FNIS。

位於**已停用**模組裡的工具無法被改寫,因為它的檔案同樣不在檢視裡。Eidos 會直
說,並從它自己的資料夾執行,而不是假裝沒事。

## 來自 Mod Organizer 2 清單的工具

以 Eidos 執行個體開啟的 Wabbajack 清單或其他 MO2 執行個體會保留自己的 `ModOrganizer.ini`，其中設定的可執行檔可以複製到 `tools.ini`：

```sh
eidos tool /path/to/list import-mo2           # 顯示將要新增的內容
eidos tool /path/to/list import-mo2 --write   # 新增
```

在視窗中，「可執行檔」對話方塊有 **從 ModOrganizer.ini 匯入** 按鈕，每個項目都有可取消勾選的核取方塊。按下儲存之前不會寫入任何內容。

`Z:` 上的路徑是 Linux 路徑，維持原樣。其他磁碟機代號上的路徑如果包含清單資料夾的名稱，就會移到清單資料夾底下（`D:\Lists\MyList\tools\SSEEdit.exe` 變成 `<list>/tools/SSEEdit.exe`），可執行檔、工作目錄與引數都照此處理。指令碼擴充器的載入器與遊戲主程式會變成單純的檔名，因此和內建項目一樣，從 Eidos 掛載檢視的遊戲資料夾執行。指向其他位置的項目（Windows 系統程式、清單之外的工具）會連同原因列為已略過，`tools.ini` 中已有的標題則永遠不會被取代。

## 把工具的輸出送進它自己的模組

產生器 - FNIS、Nemesis、BodySlide、DynDOLOD、Synthesis - 會寫出好幾百個檔案。
預設情況下它們和其他東西一起落進 Overwrite。在 Executables 編輯器裡設定
**Capture output into**,這一次執行的輸出就改為進入那個模組:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

模組不存在就會被建立。只有「這一次」執行產生的檔案會被搬走;原本就在 Overwrite
裡的東西留在原地,所以兩個各有擷取目標的工具不會互相搶走輸出。什麼都沒寫的執行
不會留下一個空模組。

這是在執行結束之後才做,而不是把寫入層指向那個模組 - MO2 是後者的做法。把寫入
層指向一個模組,會讓它在整段執行期間被提升到最高優先權 - 把它牽涉到的每一個
衝突都翻一次,結束後再翻回來 - 而且會直接寫穿模組自己的檔案,沒有 copy-up。
擷取達到同樣的最終狀態,而兩者都不需要。

如果目標模組是停用的,輸出仍然會寫出去,但遊戲看不到,於是工具下次執行時又會
重新產生同樣的檔案。遇到這種情況 Eidos 會提出警告。

### 輸出過期時

Eidos 會記錄工具的輸出是由哪些模組、外掛順序、設定檔和執行檔產生的。其中任何一項改變,Diagnostics 分頁都會提示,卡片上還有一個 **Run <tool> again** 按鈕,依目前設定重新執行該工具。

對於輸出過期就會弄壞遊戲的產生器(例如 Pandora 或 Nemesis),可以在 `tools.ini` 裡它的項目下加上 `check_before_play=1`,或在 Executables 對話框中為它開啟 **Ask before playing when its output is out of date**。之後 Run 會先詢問:執行該工具,還是照常遊玩。選擇照常遊玩後,會記住到模組、外掛或設定檔再次改變,或視窗關閉為止。Eidos 從不自己啟動該工具。

## 工具需要哪些 DLL,是由它的「名字」決定的

這是最出人意料的部分,值得直說:**你給工具取的標題,決定了 Eidos 會替它準備
哪些執行階段前置需求。**比對方式是對標題做不分大小寫的子字串比對。

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

所以註冊成 **`BodySlide`** 的工具會拿到它的 DirectX DLL;同一個執行檔註冊成
**`BS`** 就什麼都拿不到,可能啟動失敗,而錯誤訊息完全沒提到 DLL。工具就照程式
的名字命名。

這份清單在 `default_prereqs`(`crates/eidos-instance/src/tools.rs`)裡,而
Executables 對話框裡的 `Prereqs` 欄位可以編輯 - 偵測結果是預設值,不是規定。

標題同樣決定**參數**，只為那一個需要誰也猜不到的參數的工具：標題中含
`pgpatcher`（或它的舊名 `parallaxgen`）就會得到 `--ignore-mo2vfscheck`。原因，
以及它為何不是選用項，見[下文](#為什麼-pgpatcher-需要---ignore-mo2vfscheck)。

### 三種前置需求

**第一層 - 隨附的 DLL**(`d3dx9_43`、`d3dcompiler_47`、`d3dcompiler_43`、
`d3dx11_43`)。Eidos 自己帶著它們,並在啟動時複製進前綴。不必做任何事,不用網路。

**第二層 - winetricks verb**(`vcrun2022`、`dotnet8`、`dotnetdesktop8`、
`dotnet48`、`xact`...)。它們會寫入登錄機碼、GAC 與 CLR host,所以沒辦法用複製
檔案解決。它們會**從 Microsoft 下載**。

**第三層 - 執行階段**(`dotnet10`, `dotnetsdk10`)。現代的 .NET 執行階段是 193 個檔案,住在
自己的目錄裡,透過 `DOTNET_ROOT` 被找到:從不註冊,也完全不安裝進前綴,所以
另外兩層都載不動它。Eidos 自己下載,對照編進二進位檔裡的 checksum 檢查,並快取
在 `~/.local/share/Colony/Eidos/runtimes/` - **在任何實例之外**,因為 78 MB 不是
每個遊戲一份,也不是每個 profile 一份。

`dotnetsdk10` 是 .NET 10 SDK,下載 301 MB,解壓縮後 808 MB。
Synthesis 編譯 patcher 需要它,而且建置與執行 patcher 時呼叫的是不帶路徑的 `dotnet`,
所以宣告了 `dotnetsdk10` 的工具還會在 `WINEPATH` 上拿到 SDK 資料夾,外加 `NUGET_CERT_REVOCATION_MODE=offline`、`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` 與 `DOTNET_CLI_TELEMETRY_OPTOUT=1`。
建置 patcher 會留下仍在執行的 MSBuild 節點與 VBCSCompiler 伺服器,在 Wine 下它們會讓前綴的 wineserver 一直存活,所以該工具還會拿到 `MSBUILDDISABLENODEREUSE=1`、`DOTNET_CLI_USE_MSBUILD_SERVER=0` 與 `UseSharedCompilation=false`。
`DOTNET_ROLL_FORWARD=Major` 讓為 .NET 8 建置的 patcher(Mutagen 0.54 之前)在找不到 .NET 8 執行階段時,改用 SDK 附帶的 .NET 10 執行階段執行。
`tools.ini` 中仍寫著舊預設值 `dotnet8`, `vcrun2022` 的 Synthesis 項目,會在 Eidos 讀取時改成 `dotnetsdk10`, `vcrun2022`。

NuGet 會對照 Windows 根憑證存放區檢查它還原的每個套件的簽章,而 Wine 前綴裡沒有 SDK 以 `sdk/10.0.403/trustedroots/codesignctl.pem` 與 `timestampctl.pem` 附帶的程式碼簽署根憑證與時間戳記根憑證。
沒有它們,還原 `Mutagen.Bethesda.Synthesis` 0.36.6 會以 `NU3018` 與 `NU3028` 失敗。
因此,在執行宣告了 `dotnetsdk10` 的工具之前,只要 SDK 已經下載,Eidos 就會把這兩個檔案中的 385 個憑證加入遊戲前綴裡目前使用者的根憑證存放區,用的是寫入遊戲路徑的同一次 `regedit` 匯入(見下文)。
如果其中某個憑證遺失,Eidos 會重新匯入。
此後,該前綴裡的每個 Windows 程式都信任這些根憑證。
有了它們,在全新的前綴與空的 NuGet 快取下,同樣的還原與建置在系統 Wine 11.21 與 GE-Proton11-9 上都能成功。

第二層與第三層都不會靜悄悄地執行:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

在 GUI 裡,同樣的狀態就在 Prereqs 欄位底下,缺少的那些是按鈕。既不是隨附的、
也不是執行階段、又不是已知 winetricks verb 的項目,會被回報為可能的拼字錯誤,
而不是提供下載。

### 為什麼 DynDOLOD 需要 `dotnet10`

DynDOLOD 自己不建 object LOD:它把工作丟給 LODGen,而且附了三個版本。
`LODGenx64.exe` 的目標是 .NET Framework 4.8,在 Proton 底下會被導向 Wine 的
Mono - 而它的 `System.Uri` 初始化程序會呼叫一個 Mono 沒有實作的方法。它在做
第一行正事之前就掛掉,留下一份只有版本橫幅、其他什麼都沒有的記錄檔,以及一個
只寫著「failed for one or more worlds」的 DynDOLOD 對話框。

安裝真正的 .NET Framework 也修不好:Proton 把 `mscoree.dll` - 也就是本該找到它
的載入器 - 換成指向自己目錄樹的 symlink,而且每次前綴更新都會重做一遍。

能用的版本是 `LODGenx64Win10.exe`,它的目標是現代 .NET,完全不碰 `mscoree`。把
`DOTNET_ROOT` 指向一份 .NET 10 執行階段,它就跑得起來。`dotnet10` 準備的就是
這個,而 Eidos 在啟動任何宣告了它的工具時會設好這個變數。

Eidos 是用系統的 `winetricks`,搭配 Proton 自己的 `wine` 與遊戲前綴來執行的,
這樣就繞開了 Steam 的 pressure-vessel 容器,以及 protontricks 與 Proton-GE
不相容的問題。宣告了未安裝的第二層 verb 的工具照樣會啟動,只是帶一則警告,
點名該 verb 以及修好它的命令 - 使用者也可能已經從別處裝了。

### 為什麼 PGPatcher 需要 `--ignore-mo2vfscheck`

PGPatcher 會重寫網格與插件，讓你安裝的貼圖模組——原版、視差、complex
material、PBR——在每個表面上都用對的著色器。它拒絕在模組管理器之外執行，這是對
的：對一個未經管理的遊戲目錄打補丁是破壞性的。

問題出在它「怎麼」檢查：它去找 **usvfs**，也就是 MO2 在 Windows 下注入遊戲的
DLL 掛鉤層。Eidos 沒有，也永遠不會有——合併檢視是一個 FUSE 掛載，Wine 走進去時
根本不知道它存在——所以檢查什麼也找不到，工具在做任何事之前就立刻結束：

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

而且它點名的還是使用者根本沒在執行的程式。作者本人對兩份 Linux 回報
（[#716][pg716] 與 [#725][pg725]，都來自 Fluorine，即 MO2 的 Linux 分支，基於
同樣的原因採用 FUSE）給出的答案，就是一個跳過該檢查的旗標。

因此 Eidos 會替你預填。在模組池中被找到的 PGPatcher 出現時參數已經設好；在
Executables 對話框的標題輸入 `PGPatcher`，Arguments 欄也會以同樣方式被填上。
你輸入的內容永遠優先：預填只會填你留空的欄位，與上面的前置條件規則一致。

清單在 `default_args`（`crates/eidos-instance/src/tools.rs`），緊鄰
`default_prereqs`。它只有一條，但一條就足以讓它成為一個函式：旗標誰也猜不到，
而它所避免的失敗點名的還是別的程式。

執行時它什麼都不需要，這對 .NET 工具來說少見：它自帶執行環境（`dotnetlib/`，
附有宣告 `includedFrameworks` 的 `runtimeconfig.json`），並把 Microsoft CRT 的
DLL 放在執行檔旁邊。Prereqs 留空即可。

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### Mod Organizer 2 模式下的 PGPatcher 與 PGLite

Mod Manager 設為 `None` 時，PGPatcher 只能看到一個合併後的 `Data` 資料夾，無法
得知某個網格或材質來自哪個模組，因此既沒有衝突可顯示，也沒有模組順序可遵循。
它的 `Mod Organizer 2` 模式直接讀取實例本身：設定檔的 `modlist.txt`、依優先順序
排列的已啟用模組，以及每個模組資料夾裡的檔案。遊戲內版本 PGLite 也以同樣方式
讀取實例。Eidos 實例本來就是 MO2 的配置（`mods/`、`profiles/<name>/modlist.txt`、
`overwrite/`），所以這個模式可以直接用於它。

在 PGPatcher 的啟動器中：

1. 把 **Mod Manager** 設為 `Mod Organizer 2`。
2. 把 **MO2 Options -> Instance Location** 設為 Eidos 實例資料夾，寫成 Wine
   `Z:` 磁碟（即 `/`）上的 Windows 路徑。位於 `/mnt/Games/Eidos-Skyrim` 的可攜
   實例寫作 `Z:\mnt\Games\Eidos-Skyrim`；全域實例寫作
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`。
3. 在工具的參數中保留 `--ignore-mo2vfscheck`。在這個模式下，PGPatcher 會尋找
   usvfs，找不到就停止。

PGPatcher 只接受含有 `modorganizer.ini` 的資料夾，所以 Eidos 每次啟動遊戲或
工具時，都會為所啟動的設定檔在實例根目錄寫入 `ModOrganizer.ini`。只有內容
變更時才會重寫。檔案包含：

- `[General]` 中：`gameName`（MO2 中的遊戲名稱，因此 Fallout: New Vegas 寫作 `New Vegas`）、`gamePath`（遊戲安裝資料夾，以 `Z:` 路徑表示）與
  `selected_profile`；
- `[Settings]` 中：以絕對 `Z:` 路徑表示的 `mod_directory`、`profiles_directory`
  與 `overwrite_directory`，實例有 `downloads/` 資料夾時還有
  `download_directory`。

檔案第一行是一條註解，說明它由 Eidos 寫入。沒有這一行的 `ModOrganizer.ini`，
無論檔名大小寫如何，Eidos 都不會修改或刪除：你在 Eidos 中開啟的 MO2 實例保留
它自己的檔案，PGPatcher 照原樣讀取。MO2 儲存設定時會丟棄註解，所以被 MO2 覆寫
儲存過的檔案從此歸 MO2 所有。空檔案算作 Eidos 的檔案。

這樣的檔案記錄的是 MO2 最後選擇的設定檔，PGPatcher 使用這個設定檔，而不是 Eidos
啟動的那個。兩者不同時，Eidos 會在啟動時記錄一則警告。請在 MO2 中選擇同一個設定
檔，或者重新命名或刪除該檔案，讓 Eidos 寫入自己的檔案。

Eidos 只在啟動某個東西時寫它自己的檔案，所以在兩次啟動之間，它記錄的是上一次啟動的
設定檔。由 Eidos 啟動的工具能在其中找到自己的設定檔。

當設定檔中沒有任何已啟用的模組，或輸出資料夾就是某個已啟用模組的資料夾時，
PGPatcher 會報錯並停止，和在 MO2 下一樣。

## 前綴裡的遊戲路徑

Windows 工具是靠讀 `HKLM\Software\Bethesda Softworks\<game>` 的
`installed path` 來找遊戲的,那把機碼由遊戲自己的安裝程式寫入 - 而 Steam 在
Proton 底下從來不跑那個安裝程式。沒有它,xEdit、Wrye Bash 與 DynDOLOD 開起來
就是一個空路徑。Eidos 會在執行工具前寫好它:冪等、只增不減,若前綴尚未初始化
或正在使用中則跳過。

## 找到工具:隱藏、置頂,以及桌面捷徑

一款遊戲的預設項目裡有些工具你可能從來不用,而一個要列八個項目才輪到第二個的
選單,是沒人會讀的選單。在 Executables 對話框裡:

- **Pin to top** 把項目放到 Run 清單的最前面。
- **Hide from picker** 把項目從選單裡拿掉,但不刪除它。
- **Desktop shortcut** 會把一個 `.desktop` 寫進
  `~/.local/share/applications` - 在 freedesktop 系統上啟動器本來就該待的
  地方,所以它會出現在你的應用程式選單與搜尋裡,而不是出現在桌面上。它直接
  執行 `eidos tool <instance> run <title>`,也就是說,即使 Eidos 視窗根本沒開
  著,工具也是**帶著這個實例的 profile、穿過合併檢視**啟動的。

隱藏與置頂關乎的是一個工具*怎麼被找到*,而不是它執行什麼,所以它們對每款遊戲
的預設項目和你自己的項目一樣有效。

script extender 的 loader、啟動器與遊戲主程式沒有 Desktop shortcut 按鈕。它們
會啟動遊戲,而遊戲一律透過它的商店啟動,這件事商店本身的捷徑已經在做。用
Eidos 1.22 或更早版本替其中某一項建立的捷徑仍然能用:和在終端機裡輸入的
`eidos tool <instance> run <title>` 一樣,它會請商店啟動遊戲,就像 Run 在不是
由商店開啟的視窗裡做的那樣。商店的設定缺少時,這個指令什麼也不啟動,並印出要
設定什麼。捷徑不會顯示任何訊息,所以要看它,就在終端機裡執行同一個指令。

每個設定檔都可以有自己的遊戲捷徑。在設定檔上按右鍵並選擇 **Desktop shortcut**：
這個項目會執行 `eidos tool <instance> run "<game>" --profile <name>`，它會像在視
窗裡選取該設定檔那樣把它設為目前的設定檔，然後請商店啟動遊戲。商店開啟的 Eidos
視窗顯示的就是這個設定檔。`--profile` 適用於任何 `eidos tool ... run`，會在工具
啟動前選取設定檔；與 `--print` 一起使用時，它什麼也不選。

## 本身就是獨立 Steam 應用程式的工具

Creation Kit 是獨立的 Steam 應用程式,要用自己的 AppID;少數幾個透過 Steam
發行的模組工具也一樣。在項目上設定 **Steam AppID**,Eidos 就會用那個 id 而不是
遊戲的 id 來啟動它。

在 Windows 上這代表換一個啟動器。在這裡,它只是本來就要組出來的那次執行上的
兩個環境變數 - `SteamAppId` 與 `SteamGameId`,兩個都要,因為 Proton 讀其中
一個,而 Steam 自己的函式庫讀另一個,工具若看到兩者不一致,失敗的方式會很古怪
而不是乾脆。`eidos tool ... --print` 會原原本本顯示真正執行時會拿到什麼。

## 環境變數,以及不經過 Proton 的工具

`<instance>/tools.ini` 裡有三個鍵在 Executables 對話框中還沒有對應的欄位,
需要手動寫進工具所在的區段。每一行 `env=NAME=value` 設定一個只對這個工具的
執行生效的變數:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos 在設定完它自己的所有變數之後才套用這些行,所以這裡寫的值會取代 Eidos
自己的值。這也包括 Eidos 依模組附帶的 DLL 組出來的 `WINEDLLOVERRIDES`:
設定了它的工具,必須把自己需要的每個 DLL 都列進去。變數名稱由字母、數字和 `_`
組成,不以數字開頭。名稱不合規則的行會被忽略。

`runner=native` 不經過 Proton,直接執行 `exe` 本身。合併檢視照常掛載,
`arg0=`、`arg1=` 這些行就是它的參數。Linux 程式,或 Steam 不管理、Heroic
也沒有記錄執行器的副本上的工具,都用這種方式:`exe` 是那個副本的 Wine,Windows
程式是第一個參數,前綴由 `env=` 指定。

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` 必須是絕對路徑。工作目錄預設是 `exe` 所在的資料夾,在這裡就是執行器的
資料夾,所以要設定 `workdir=`。只有 `exe` 會被改寫成合併後的路徑:模組裡的
Windows 程式,要寫它在遊戲 `Data` 資料夾下的路徑,合併檢視就是在那裡顯示它的。
原生工具會跳過 Proton 的步驟:上面講的登錄機碼、Steam 的著色器快取、空的 .NET
根目錄,以及從 `LD_PRELOAD` 中拿掉 Steam 覆蓋層。

用 Heroic 安裝的 GOG 或 Epic 副本,如果遊戲的 Heroic 設定
(`GamesConfig/<appName>.json`)指定了 Wine 或 Proton 版本,工具就不需要這樣的
項目:Eidos 用那個版本在遊戲的 Heroic 前綴裡執行它們,
`eidos prereqs <instance> --install` 也把它交給 winetricks。只用 Legendary
安裝的副本沒有 Heroic 設定。

`slr=1` 會讓工具及其之前的登錄檔匯入,在工具的 Proton 版本所指定的 Steam Linux Runtime 容器裡啟動(見其 `toolmanifest.vdf` 中的 `require_tool_appid`),與 Steam 啟動遊戲的方式相同。
這個執行階段必須裝在某個 Steam 媒體庫中;沒有裝的話,執行會停下,並指出需要安裝的 Steam 應用程式。
`eidos prereqs` 仍在主機上執行 winetricks,`runner=native` 的工具會忽略這個鍵。
容器能看到你的家目錄與遊戲所在的 Steam 媒體庫。
放在其他地方(例如另一顆磁碟上)的工具、mod 或執行階段,需要 `env=STEAM_COMPAT_MOUNTS=/那個/路徑`(多個路徑以 `:` 分隔)。
裝在 `/usr/share/steam/compatibilitytools.d` 下的 Proton 很可能無法搭配 `slr=1` 使用,因為容器裡的 `/usr` 是執行階段自己的。
目前還沒有在容器裡實際執行過工具。

`eidos tool <instance> run <title> --print` 會顯示最終的命令,以及這次執行在繼承的
環境之上設定的每一個變數,包括這些行。

## 工具自己的設定,終究還是它自己的事

Eidos 把工具擺在對的位置,配上對的 DLL。工具接下來拿它的設定做什麼,是你和
工具之間的事,而且失敗通常是無聲的。

這裡給一個實例,因為不講的話會浪費你一小時:BodySlide 的 **Game Data Path**
(Settings)必須指向遊戲的 `Data` 目錄,而不是它上面那層遊戲資料夾。設高一層,
批次建置會回報「All sets processed successfully」,然後把 1439 個 mesh 寫到
遊戲永遠不會去找的地方。Eidos 會接住它們 - 它們落在 `Overwrite/Root/` 而不是
你的安裝目錄裡 - 但從遊戲的角度看一切正常,只不過你的身體模型沒建出來。

工具的輸出本來就該待在 Overwrite。當一次執行產生了值得留下的東西,
**Overwrite -> Create mod...** 會把它變成一個普通模組,可以像其他模組一樣
排序、停用和移除。
