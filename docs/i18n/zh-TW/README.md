<!-- eidos-i18n: source=README.md sha=37e97bafbd2cf43f2101a28b23c1464420a5f9ec -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**從不碰你遊戲資料夾的原生 Linux 模組管理器。**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#安裝)

Eidos 為 Linux 上的 Bethesda 遊戲帶來 Mod Organizer 2 在 Windows 上帶來的東西:一份
每次啟動重建的模組合併檢視。它用 Linux 的掛載命名空間和 FUSE,而不是 Windows API 掛鉤,
管理器本身不需要 Wine。沒有檔案被複製進遊戲資料夾,所以事後沒有東西需要清理。Eidos 也有
Stellar Blade、Stardew Valley、7 Days to Die 和 Crimson Desert 的定義。

Steam 的副本從遊戲的啟動選項執行 Eidos。Heroic 或 Legendary 裡的 GOG 和 Epic 副本則把它
當作包裝器執行。遊戲一律透過擁有該副本的商店啟動;Eidos 從不自己用 Proton 啟動它。

```
Steam ──> eidos-gui %command% ──> [ 私有命名空間 ]
                                  │  模組 ⊕ 遊戲  ──> 遊戲看到的東西
                                  └─ 隨遊戲一同消亡;安裝目錄始終原封不動
```

> **狀態:** Skyrim SE 每天都透過 Eidos 遊玩,有 SKSE、script extender 預載器、
> Creation Club、LOOT 排序過的載入順序和各設定檔獨立的存檔。目前只有一個遊戲家族在實際
> 遊玩中得到驗證。另外十款 Bethesda 遊戲和四款其他遊戲已經接好線,等著測試者。

## 為什麼是 Eidos

合併檢視掛載在一個私有的掛載命名空間裡,只有遊戲的行程樹看得到它。其他程式,包括第二款
遊戲,都看不到它,也不需要為它取得任何授權。遊戲結束或被殺掉,或機器斷電時,命名空間會
隨它的行程一起消失,安裝目錄和先前一樣。

每個設定檔擁有自己的模組清單、外掛順序、INI 與存檔。啟動時,外掛檔案與存檔資料夾會
bind-mount 蓋在遊戲自己的路徑之上,所以遊戲寫在那裡的東西會落進設定檔。切換設定檔就切換
這一切。

Eidos 不需要 setuid 輔助程式、不需要系統常駐程式、不用 `sudo setcap`,也不用改
`/etc/fuse.conf`。FUSE 常駐程式以你的使用者身分為一次啟動執行,並隨遊戲結束。你只要安裝
一個程式,設定一個 Steam 啟動選項。

每次執行之前,Eidos 都會保留一份外掛清單的快照。如果一次當機弄壞了清單,Diagnostics 會把
它和那份快照比對,Restore the pre-session order 會把舊的順序放回去。一次會抹掉你載入順序
的擷取會被拒絕,並說明原因。

Test run 會用設定檔的模組、不帶模組,或只帶選取的模組及其 master 來啟動遊戲。那次執行的
存檔、INI 變更、外掛狀態、Overwrite 輸出,以及其他 My Games 檔案(包括日誌),都進入
`<instance>/test-run/`。遊戲結束時,Eidos 列出這五組檔案,你逐組決定保留或丟棄。其他寫入,
例如 Wine 登錄檔,照常寫到原處。Diagnostics 裡的 Find the mod behind a problem 每次用
一半的嫌疑模組加上它們需要的 master 進行測試執行。每次執行之後你回答問題是否出現,Eidos
就縮小嫌疑範圍,直到點出那個模組,或一組彼此需要對方 master 的模組;200 個嫌疑對象大約要
8 次執行。開始之前在清單裡選取的模組,在每一步都保持開啟。細節見
[usage.md](docs/guide/usage.md#測試執行)和
[usage.md](docs/guide/usage.md#找出造成問題的模組)。

### 它和其他方案的比較

| | Eidos | 透過 Wine 的 MO2 | Fluorine-Manager | Limo / 連結式部署器 |
|---|---|---|---|---|
| 管理器原生執行 | 是 | 否,Wine 裡的 Windows 程式 | 是(Qt 移植) | 是 |
| 遊戲目錄不受動 | 是,永遠 | 是 | 是 | 否,連結被寫進去 |
| 掛載對誰可見 | 只有遊戲 | 只有遊戲 | 整個系統 | 不適用 |
| 當機後需要清理 | 沒有 | 沒有 | 陳舊掛載的復原 | 手動取消部署 |
| root 模組(ENB、預載器) | 是,原生 | 需要外掛程式 | 需要外掛程式 | 部分 |
| 需要的特權 | 沒有 | 沒有 | 改 `/etc/fuse.conf` | 沒有 |

### 它有多快

| | 之前 | 現在 |
|---|---|---|
| 載入一份存檔 | ~20 秒 | 6-7 秒 |
| 一次工作階段裡的目錄讀取 | 560 萬 | 33.5 萬 |

Cell 切換是即時的。以前找一個檔案要把五十個模組逐一問過,列一個資料夾則要重複做五十遍。
現在唯讀的各層在掛載時只建一次索引,所以找一個檔案是一次雜湊查詢,資料夾合併後的列表也
是在掛載時算好一次。這些數字來自一個正常遊玩的真實實例,不是跑分程式:
[internals/performance.md](../../internals/performance.md)。

### 前人成果與致謝

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) 與
  [usvfs](https://github.com/ModOrganizer2/usvfs):Eidos 重現的語意,以及它據以研究對等
  程度的那份程式碼庫
- [LOOT](https://loot.github.io/):排序引擎,透過 libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager)、
  [Limo](https://github.com/limo-app/limo) 與其他 Linux 管理器,它們證明有一個社群想看到
  這個問題被解決

## 它做些什麼

### 模組

Eidos 安裝單純的壓縮檔、FOMOD 精靈、Wrye Bash BAIN 套件、帶有受支援 OBMM 腳本的 OMOD
容器,其餘的則靠手動選擇器。Root 模組(script extender 預載器、ENB、Engine Fixes)原生
運作,不需要 Root Builder 外掛程式,也沒有任何東西被複製進你的安裝目錄。你可以隱藏單一
檔案、用分隔線分組模組、把選取的項目移到指定目標、為每個模組保留註記與類別,還可以匯入
MO2 設定檔。

這份清單和 MO2 的一樣運作:八個選用欄位以及依其中任一個排序、依類別或依來源分組、雙擊
手勢、鍵入即跳、每個模組專屬的備份(在你還原之前它們什麼也不做),以及對某個模組的旗標:
這個遊戲不會載入它的版面,或者它是為另一款遊戲下載的。篩選器可以依 Nexus 狀態(已推薦、
已追蹤、有更新)、註記、隱藏檔案和內容類型(外掛、封裝檔、貼圖、網格、腳本、音效等等)
挑選模組。檔案樹可以新增資料夾、改名、刪除、開啟檔案,並且不啟動任何東西就能預覽圖片與
文字。

Ctrl+Z 和 Ctrl+Shift+Z 復原和重做對模組清單與載入順序的變更。停用或移除一個被設定檔最新
三個存檔用到的模組時,會先詢問並指出那些外掛(Skyrim 和 Fallout 4 的存檔)。重新安裝或
更新 FOMOD 模組時,精靈會以上次安裝的答案開啟。模組資訊會列出它需要的模組和需要它的
模組,依據已啟用外掛的 master 讀取。

### 外掛

載入順序內建 LOOT 排序、和遊戲自己算出來一樣的模組索引、缺少 master 的警告,以及以未受
管理的列的身分顯示的 DLC 與 Creation Club 內容。標題列統計每個索引空間(full、medium、
light)已用的欄位數。滑鼠停在外掛上會顯示它的作者、說明和標頭版本。在 Skyrim 引擎上,
遊戲會載入與之同名 INI 的外掛會被標出來。

被 LOOT 標記為 dirty 的外掛,由 xEdit 的 QuickAutoClean 清理進一個 `Cleaned plugins`
模組,絕不就地修改。Flag as light 會把外掛的 light 副本寫進新模組,而當最新的存檔之一把
該外掛當作完整外掛載入時會拒絕(Skyrim 和 Fallout 4 的存檔)。

### 實例

實例是全域的,放在 `~/.local/share/Colony/Eidos/instances/` 底下,或是可攜的:一個自我
完備的資料夾,放在你想要的任何地方(第二顆硬碟、遊戲分割區),可移動、彼此隔離,和 MO2
的一樣。可攜實例會跨工作階段被記住。GUI、Steam 啟動與每一個 CLI 命令都跟隨你上次用的
那一個,而凡是接受遊戲 id 的命令,同樣接受那個資料夾。細節見
[usage.md](docs/guide/usage.md#實例全域與可攜)。

實例也可以從實例內的遊戲資料夾副本執行遊戲。Eidos 第一次開啟 Wabbajack 清單時,會執行
清單自己的副本:它的 `ModOrganizer.ini` 指名的資料夾,否則是 `Stock Game` 或
`Game Root` 資料夾。Diagnostics 裡 Game files 卡片上的 Freeze this version 會把商店安裝
複製到 `<instance>/Stock Game`,所以商店更新不再改變遊戲所執行的檔案。當兩個資料夾位於
同一個能共用資料區塊的檔案系統上(例如 btrfs 或 XFS)時,副本與商店安裝共用區塊
(reflink)。在 ext4 上,或跨兩個檔案系統時,它是完整複製,並會先顯示大小和可用空間。
遊戲仍由商店啟動。Return to the store version 和 Delete the copy 會復原它,終端機裡的
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
做同樣的事。Morrowind,以及 `bUseMyGamesDirectory=0` 的 Oblivion,無法從副本執行。細節見
[usage.md](docs/guide/usage.md#遊戲檔案商店安裝還是副本)。

### 設定檔

每個設定檔各有自己的模組順序、外掛狀態、INI 與存檔。存檔會被解析、和你目前的外掛比對
(還附一個按鈕啟用存檔需要的東西),並在每次工作階段之後複製回前綴,給 Steam Cloud 用。
Steam Cloud 從另一台裝置帶來的存檔,或沒有透過 Eidos 的啟動留在前綴裡的存檔,會列在
Saves 分頁上。Import 會把它們複製進設定檔,而設定檔已經用了那個名稱的存檔,會以新的
`orphan-` 名稱匯入,而不是取代它。

設定檔可以為它的遊戲與工具執行帶上啟動變數與包裝器(`env=NAME=value` 和
`wrapper=command`,每行一項,在 INI editor 裡)。對於 Skyrim SE、Enderal SE 和
Fallout 4,Diagnostics 裡的 Display 卡片會在設定檔的 INI 副本裡設定視窗大小與模式。
Backups 保留設定檔 INI 的還原點,Eidos 也會在每次 BethINI Pie 執行之前保留一個。設定檔
的桌面捷徑透過它的商店啟動遊戲。`eidos sessions` 列出已記錄的執行,並把其中一次與目前
狀態比較。

### Nexus

連接帳號後,可以接收「Mod Manager Download」連結、檢查更新,以及查看作者及其個人資料。
每個模組都在它的檔案來源的 Nexus 網域上檢查、推薦和連結,所以裝在 Enderal SE 實例裡的
Skyrim SE 模組仍留在 Skyrim SE 網站上。更新會沿著作者在 Nexus 上設定的後繼鏈,用新檔案
取代模組的資料夾(會提供舊資料夾的備份),保留它的優先順序與啟用狀態,並重播安裝時儲存
的 FOMOD 答案。在終端機裡,`eidos nexus update <instance> --apply` 在 premium 帳號上做
同樣的事,在免費帳號上則印出檔案頁面。Track 和 Endorse 在模組選單裡,每次檢查更新都會
把它們與你的 Nexus 帳號同步。模組資訊有一個 Nexus page 分頁,以純文字顯示它的描述與
更新日誌,它的 General 分頁最後則列出上一次更新改變了什麼:新增、移除和大小改變的檔案,
以及新增或移除的 master。Install downloads when they finish(Preferences, Nexus,預設
關閉)會在視窗開著時,於下載一完成就安裝它。

合集會按照配方安裝,支援中斷後繼續,並明確回報不支援的步驟。比對時會核對確切的來源
檔案;不完整的舊中繼資料仍標示為未驗證。合集面板會瀏覽和搜尋為某款遊戲發布的合集,
顯示已安裝的合集何時有較新的修訂版,並帶免費帳號一頁一頁走完缺少的檔案。當一次拖曳
破壞了合集的順序規則時,Diagnostics 會說出來,Restore collection order 會把它的模組放回
去。Downloads 分頁可以篩選、排序、隱藏和刪除壓縮檔。Offline mode(Preferences, Nexus)
會停止存取 Nexus,以及每天一次的版本檢查。

### 工具

xEdit、BodySlide、DynDOLOD 與同類的模組工具,是穿過合併檢視、在遊戲自己的 Wine 前綴
(Steam 的 Proton 前綴,或 Heroic 設好的前綴)裡執行的。它們看得到你的模組,它們的輸出
落進 Overwrite,一鍵就把那些輸出變成真正的模組。對於 Skyrim SE,Eidos 會在遊戲資料夾、
某個模組裡,或你擺在遊戲旁邊的工具資料夾裡,找到 SSEEdit、SSEEdit QuickAutoClean、
PGPatcher、Pandora、Nemesis、FNIS、BodySlide、DynDOLOD、TexGen、xLODGen、Synthesis 和
BethINI Pie,每一個都帶著它需要的執行階段。其他遊戲有各自的清單。Pandora、Nemesis、
FNIS、BodySlide 和 Synthesis 把輸出寫進一個有名字的模組(`Nemesis Output` 等等)。

發行套件附帶 Microsoft 的 Visual C++ 2015-2022 和 .NET 8 Desktop Runtime 安裝程式,以及
.NET 10 執行階段。Eidos 用這些安裝程式安裝 `vcrun2022` 和 .NET 8,不經過 winetricks;
沒有套件時,它從 Microsoft 下載同樣的檔案,每個都對照編進 Eidos 的 SHA-256 檢查。
Synthesis 需要 .NET 10 SDK,那是一個 300 MB 的下載,套件不附帶。其他前置需求
(.NET Framework 4.8、XACT 和較舊的 .NET 版本)仍然透過 winetricks。在你要求安裝工具的
前置需求之前,不會下載任何東西。

把你會用的釘起來,把你不用的藏起來,當某個工具本身就是一個 Steam 應用程式時給它自己的
Steam AppID,並寫出一個 `.desktop` 捷徑,讓它完全不必打開 Eidos 就能穿過合併檢視啟動。
`eidos tool <game> import-mo2` 會列出清單資料夾的 `ModOrganizer.ini` 裡的可執行檔,
`--write` 會把它們加進來。Data 分頁裡的 exe 可以執行一次或加為工具,
`eidos tool <instance> run --exe <path>` 則在終端機裡執行一個。開啟 Ask before playing
後,產生器的輸出過期時,Run 會先詢問。

Stop 會結束執行中的遊戲或工具。File 選單會在遊戲的前綴裡執行 winecfg、regedit、Wine
檔案總管和 Windows 解除安裝程式。Diagnostics 裡的 Rebuild prefix 會把前綴重新命名,讓
Steam 建立新的,不刪除任何東西。`eidos shell <instance>` 在合併檢視裡開一個 shell,或
執行 `--` 後面給的命令,它寫入的內容進入 Overwrite。

視窗在背景時,Eidos 可以在工具執行、合集安裝、打包或解包,或最後一個執行中的下載結束
時,以及 LOOT 排序失敗時,發出桌面通知。Preferences 裡的 Notify the desktop when a long
job ends 會開啟這項功能;預設關閉。

### 診斷

Diagnostics 分頁列出缺少的 master、無主的封裝檔、模組清單偏移、損壞的外掛組合,以及在一次
執行之後,script extender 自己的日誌說實際載入了什麼。它也會回報:

- 缺少或與遊戲版本不符的 script extender
- 自上次正常啟動以來改變了的遊戲
- 沒有啟動 Eidos 或 Eidos 後面沒有 `%command%` 的 Steam 啟動選項
- 比上次準備前綴的版本更舊的 Proton,或來自 Flatpak 版 Steam 的 Proton
- DLSS 缺少 `PROTON_ENABLE_NVAPI`,以及會讓 Community Shaders 的影格生成失效的獨佔
  全螢幕
- 沒有 `LinuxVersion=true` 的 `enblocal.ini`
- 上次工作階段的當機日誌和其中點名的模組
- 缺少或已停用的 Nexus 需求
- 是另一個程式或檔案不存在的 `nxm://` 處理器
- 會讓遊戲忽略 `plugins.txt` 的 `sTestFile` 行
- Starfield 中已啟用的 `.esp` 外掛,以及無效或沒有配對的 blueprint 外掛
- 散裝檔案輸給它所修補的模組的修補程式
- 某個模組的 script extender Papyrus 腳本副本壓過了擴充器自己的腳本
- 過低的 `vm.max_map_count` 或開啟檔案數上限
- 位於 Windows 檔案系統上的遊戲檔案
- 不是遊戲 Data 資料夾的 BodySlide 資料路徑,以及 `Overwrite/Root` 裡應該放在 Data 的
  資料夾
- 指向另一個遊戲資料夾的清單 `ModOrganizer.ini`
- 不見了或沒有遊戲執行檔的遊戲副本

`eidos check <instance> [--json]` 執行不需要視窗的檢查,發現問題時以 1 結束。Copy report
會複製一份把你的家目錄寫成 `~`、使用者名稱寫成 `<user>` 的報告,Data 分頁上的 Save tree
則寫出合併後的 Data 樹,附上每個檔案的來源,以同樣方式遮蔽。

關於替換保護、BSA/BA2 內部檔案衝突、SKSE 靜態診斷、產生紀錄及其限制,請參閱[完整性檢查與產生檔案](../../internals/integrity.md)(英文)。

### 它把自己的檔案放在哪裡

Eidos 使用 Colony 家族每個程式都用的配置:`~/.config/Colony/Eidos/` 放你選的東西(偏好
設定、你的 Nexus 工作階段、你的實例清單、你自己寫的遊戲與附加元件定義),
`~/.local/share/Colony/Eidos/` 放全域實例、工具執行階段(下載的,或由 `install.sh` 從
套件複製來的)與工作階段日誌,`~/.cache/Colony/Eidos/` 放 LOOT 主清單。在實例裡面,
`Stock Game` 放凍結的遊戲副本,`test-run/` 放一次等待檢視的測試執行。

舊版 Eidos 把這些放在 `~/.config/eidos/`、`~/.local/share/eidos/` 與
`~/.local/state/`。升級後的第一次啟動會把它們搬過來,在日誌裡說明,並讓舊目錄原封不動
留著。搬移完成之前,Eidos 會繼續使用舊檔案。詳情見
[internals/paths.md](../../internals/paths.md)(英文)。

### 支援的遊戲

Skyrim SE/AE 在實際遊玩中得到驗證。Fallout 4 也已經接好整條線(自動換上 F4SE、封裝檔
失效、星號載入順序、LOOT、`.fos` 存檔);見 [guide/fallout4.md](docs/guide/fallout4.md)。
已接好線、正在找測試者的有:Skyrim LE、Skyrim VR、Enderal SE、Fallout 4 VR、Fallout 3、
Fallout NV、Starfield、Oblivion 與 Morrowind。Fallout 3、Fallout NV、Oblivion 與
Morrowind 依檔案日期排列外掛:Eidos 每個設定檔保存一份順序,並在掛載內顯示這些日期,不
改動磁碟上的檔案。

另外四款遊戲也已接好線,同樣在找測試者:Stellar Blade(`~mods` 裡的 Unreal `.pak` 模組)、
Stardew Valley(SMAPI 模組,SMAPI 需另外安裝)、7 Days to Die(遊戲旁邊的 `Mods/` 資料夾)
和 Crimson Desert。Crimson Desert 只接受程式碼模組:`bin64` 裡的 ASI 外掛、ReShade 和
載入器 DLL。它的資料模組放在 PAZ 群組裡,目前不支援。

一款遊戲就是一個 TOML 檔案,你自己的可以放進 `~/.config/Colony/Eidos/games/`,不必重新
建置:[internals/adding-games.md](../../internals/adding-games.md)(英文)。

## 安裝

### 透過 Colony(推薦)

在 [Colony](https://github.com/Project-Colony/Colony) 裡搜尋 **Eidos** 並安裝。
更新會透過啟動器送達。Colony 只安裝一個檔案 `eidos-linux`:它是視窗,而透過名為
`eidos` 的連結呼叫時,它也是 `eidos` 命令列。這個檔案既不帶 NIF 模型預覽輔助程式,也不帶
下方套件附的 Microsoft 執行階段。少了輔助程式,預覽會說明這一點,其他一切照常運作;而當你
安裝工具的前置需求時,Eidos 會從 Microsoft 下載執行階段。安裝之後該做什麼,以及如何讓
Steam 指向它:[docs/guide/install.md](docs/guide/install.md#透過-colony)。

### 直接下載執行檔

從[最新發行版](https://github.com/Project-Colony/Eidos/releases/latest)取得一個檔案:

| 檔案 | 它是什麼 |
|---|---|
| `eidos-linux` | 整個程式合為一個檔案,和 Colony 安裝的一樣 |
| `eidos-linux-bundle.tar.gz` | `eidos`、`eidos-gui`、NIF 預覽輔助程式、Visual C++ 2015-2022 和 .NET 8 Desktop Runtime 安裝程式、.NET 10 執行階段、圖示、授權聲明,以及一個會註冊桌面項目與 Nexus 下載處理器的安裝程式 |

先依照[程式碼簽章政策](#程式碼簽章政策)驗證下載的檔案,然後:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

安裝程式不需要 sudo,會把執行檔放進 `~/.local/bin`,並把套件的 `runtimes/` 複製到
`~/.local/share/Colony/Eidos/runtimes`。`--system` 與 `--bindir DIR` 可以把執行檔放到
別處,`--from DIR` 則從另一個資料夾取執行檔。`--cap` 請 sudo 授予 `CAP_SYS_ADMIN`,只有
實驗性的 FUSE passthrough 需要它,而 passthrough 仍保持關閉;`--no-cap` 是預設。兩種檔案
都需要 x86_64 Linux、glibc 2.39 或更新版本、FUSE(`fusermount3`),以及用來處理模組壓縮檔
的 7-Zip。

Eidos 不會自己更新。視窗在啟動時每天一次向 GitHub 詢問最新發行版,有更新版本時,會在
選單列的 Help 之後顯示指向其發行說明的連結。Preferences 可以關掉這項檢查,而安裝在
`/usr` 底下、`/usr/local` 之外的副本(由套件管理員更新)從不詢問。

### 從原始碼建置

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

需要 Rust 1.92 或更新版本(CI 檢查的 `rust-version`;CI 與發行版以 1.99.1 建置),
另加 `just`、CMake 3.20 或更新版本,以及供 NIF 預覽輔助程式使用的 C++17 編譯器。
`just build` 使用 release 設定檔:一個 codegen unit 和 fat LTO。用 3 個建置工作,
`eidos` 與 `eidos-gui` 的冷建置這樣花了 334 秒,thin LTO 則是 243 秒,而 fat LTO 讓
`eidos` 小 1.2 MB、`eidos-gui` 小 3.8 MB(測量數據在 `Cargo.toml`)。
`just profile=dev build` 編譯得比較快。Arch 套件可從 `packaging/PKGBUILD` 建置。前置需求
以及不用 `just` 的命令:[docs/guide/install.md](docs/guide/install.md)。

### Steam 啟動選項

Eidos 是作為遊戲的啟動命令執行的,這正是它能在遊戲啟動前完成掛載的原因。大多數設定需要
的就只有這條基本行,並使用你安裝位置的絕對路徑(Steam 不會讀取你的 `PATH`):

```
~/.local/bin/eidos-gui %command%
```

透過 Colony 安裝時,這個路徑是 `~/.local/share/Colony/apps/Eidos/eidos-linux`。接著按下
Play:Eidos 會在該遊戲的實例上開啟,而 Run 會讓遊戲穿過合併檢視啟動。

在你自己打開的視窗裡,Run 會請 Steam 啟動遊戲(`steam -applaunch <appid>`),所以啟動
選項仍然適用,你在 Steam 開啟的視窗裡按 Run。Heroic 會收到 `heroic://launch` 連結,
Legendary 則執行 `legendary launch <app>`,前提是 Eidos 已在那裡被設為遊戲的包裝器。商店
的設定缺少時,Run 什麼也不啟動,並說明要設定什麼:
[guide/usage.md](docs/guide/usage.md#啟動遊戲)。開啟 Start the game at once when a store
opens Eidos(Preferences,預設關閉)後,Eidos 會執行 Diagnostics 檢查,並在沒有任何
需要注意的事時,不顯示視窗就啟動遊戲。

其餘的一切都是疊在它前面的環境變數,而且可以自由組合:

| 你想要... | 放在前面 |
|---|---|
| 搭配 Community Shaders 的 DLSS | `PROTON_ENABLE_NVAPI=1`。沒有它,DLSS 永遠不會初始化,也不會說什麼;完整檢查清單在 [guide/graphics.md](docs/guide/graphics.md) |
| 畫面上的 FPS 計數器 | `DXVK_HUD=fps` |
| 驅動層級的影格插補,零模組(RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`,絕不要和 Community Shaders 自己的影格生成一起用 |
| 給錯誤回報用的詳細日誌 | `EIDOS_LOG=debug`(工作階段日誌落在 `~/.local/share/Colony/Eidos/logs/`) |
| 掛載端每個工作階段的 I/O 報告 | `EIDOS_FUSE_STATS=1` |
| 不同的 FUSE 工作執行緒數量 | `EIDOS_FUSE_THREADS=8`(預設 4;追查併發問題時,`1` 是第一個該試的) |
| 把這次啟動釘在某個可攜實例上 | `EIDOS_INSTANCE=/path/to/folder`。沒有它,Eidos 會開啟你上次用的實例 |
| 在 Start the game at once 開啟時,為某一次啟動顯示視窗 | `EIDOS_WINDOW=1` |

現代模組化設定(Community Shaders、DLSS、影格生成)該留下的那一行,就是最終的命令,
不是範例:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

在確認設定可用的期間把 `DXVK_HUD=fps` 加在前面,確認之後就拿掉。

同樣的變數也可以放進設定檔的啟動變數,而不是 Steam 那一行,這樣它們按設定檔保存,也會
套用到工具執行:
[guide/usage.md](docs/guide/usage.md#每個設定檔的變數與包裝器)。

更深入的診斷開關(`EIDOS_FUSE_TRACE`、快取與索引的二分排查開關,以及
`EIDOS_FUSE_PASSTHROUGH` 為什麼預設關閉)在
[guide/troubleshooting.md](docs/guide/troubleshooting.md)。

## 文件

一切都在 [docs/](../../) 裡,並在 [docs/README.md](docs/README.md) 依讀者分類索引:
使用者指南(安裝、CLI 與 GUI、xEdit 與 DynDOLOD 之類的工具、圖形、疑難排解)、
Eidos 內部如何運作,以及它為什麼存在。安全政策以及如何回報漏洞:
[SECURITY.md](../../../SECURITY.md)。

## 語言

玩家需要的頁面都有翻譯。英文是準的:當翻譯和它有出入時,以英文檔案為準。

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

其餘的一切是英文。`docs/internals/` 與 `docs/project/` 的讀者同時也在讀 Rust 程式碼,
`CHANGELOG.md` 是產生出來的,`SECURITY.md` 也沒有翻譯。翻譯前兩者會為一群並不需要它們的
讀者,多出超過 19,000 個字要維持同步。

每一份翻譯都帶著它所依據的英文檔案的雜湊值,而當這個雜湊值和英文不再相符時 CI 會失敗,見
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh)。接著翻譯會被更新,並用
`scripts/i18n-check.sh --fix` 重新蓋章。過期的翻譯是被更新,而不是被刪除。

一種語言就是一個目錄。`docs/i18n/<lang>/` 鏡像了儲存庫根目錄的結構,所以
`docs/i18n/de/docs/guide/install.md` 就是德文版的 `docs/guide/install.md`。正因如此,兩個
譯文頁面之間的連結與它們英文原文之間的連結是同一串字元,而要撤下一種語言也只需要一次
`rm -r`。

## 程式碼簽章政策

每一個發行檔案都由 Project Colony 組織在 CI 中簽章,從不在開發者的機器上簽。在發行
頁面上,每個檔案旁邊都有:

| 檔案 | 它是什麼 |
|---|---|
| `<asset>.sig` | 以組織的發行金鑰對該檔案做的 ed25519 簽章 |
| `<asset>.meta` | 三行內容,把該檔案綁定到它的檔名、它的 sha256 與發行版本 |
| `<asset>.meta.sig` | 對 `.meta` 做的 ed25519 簽章 |

私鑰是組織的機密,只由共用的
[sign-and-publish 工作流程](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
在一個不建置任何東西的 job 中使用;編譯 Eidos 的那些 job 從來看不到它。
Colony 在安裝或更新 Eidos 之前會檢查這三個檔案,並拒絕比已安裝版本更舊的發行版。
要用 OpenSSL 3 自己驗證下載的檔案(同樣的命令適用於每一個檔案):

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

發行版如何建置、簽章與發布:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)(英文)。

## 隱私

Eidos 不傳送任何遙測、分析資料或當機報告。除了這張表裡列出的伺服器之外,它不連線到
任何伺服器,而且只在表中所寫的動作時才連線。Offline mode(Preferences, Nexus)會停止
所有與 Nexus 的連線,以及每天一次的版本檢查。

| 資料 | 儲存或傳送 | 在哪裡,以及為什麼 |
|---|---|---|
| 偏好設定、實例清單、遊戲與附加元件定義 | 儲存 | `~/.config/Colony/Eidos/` |
| 實例、工具執行階段、工作階段日誌 | 儲存 | `~/.local/share/Colony/Eidos/`,或你為可攜實例選擇的資料夾 |
| LOOT 主清單 | 儲存 | `~/.cache/Colony/Eidos/loot/` |
| 版本檢查 | 傳送 | 啟動時,每天最多一次,對 `api.github.com/repos/Project-Colony/Eidos/releases/latest` 發出一個請求。Eidos 只讀發行標籤,不下載任何東西。它預設開啟,可在 Preferences 關閉,在 Offline mode 以及安裝在 `/usr` 底下、`/usr/local` 之外(由套件管理員更新)的副本上會略過。[GitHub 隱私權聲明](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Nexus 登入 | 傳送 | 登入時會在你的瀏覽器開啟 `users.nexusmods.com`,它會把結果交回給 Eidos 只為這次登入在 `127.0.0.1` 上開啟的監聽程式。接著 Eidos 在 `users.nexusmods.com/oauth/token` 把它換成權杖,並取得 Nexus 用來簽署權杖的公開金鑰。[Nexus Mods 隱私權政策](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus 憑證 | 儲存 | 存取權杖與更新權杖,存放在 `~/.config/Colony/Eidos/nexus.ini`,權限模式 0600。舊版 Eidos 寫在那裡的 `api_key=` 行會保留但永遠不會被讀取;你想刪就刪。 |
| Nexus 請求 | 傳送 | `api.nexusmods.com`,在你使用 Nexus 功能時:你的帳號名稱與 premium 狀態、模組與檔案細節、更新檢查、合集與下載連結。每次檢查更新都會讀取你的推薦與追蹤模組,並在你按 Endorse 或 Track 時寫入。模組需求、描述與更新日誌,以及合集搜尋,會送到 `api.nexusmods.com/v2/graphql`。每個請求都帶著你的存取權杖,並標明 Eidos 及其版本。 |
| Nexus 下載 | 傳送 | 你要求的檔案,來自下載連結所指名的 Nexus CDN 主機,存進實例的下載資料夾,每個檔案旁邊附一個 MO2 格式的 `.meta`。 |
| `nxm://` 連結 | 儲存 | `eidos nxm --register`(`install.sh` 會替你執行)會寫出 `~/.local/share/applications/eidos-nxm.desktop`,並透過 `xdg-mime` 把它設為 `nxm://` 處理器,讓網站上的 Mod Manager Download 按鈕把連結交給 Eidos。Arch 套件會在全系統安裝同樣的項目。 |
| LOOT 主清單 | 傳送 | `raw.githubusercontent.com/loot/...`,在某款遊戲第一次以 LOOT 排序時,以及你要求更新時。不會送出任何屬於你的資料。[GitHub 隱私權聲明](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| 工具執行階段 | 傳送 | 只在你安裝某個工具的前置需求時,而且只針對發行套件沒有附上的東西(單一檔案 `eidos-linux` 與從原始碼建置都不附任何東西):來自 `builds.dotnet.microsoft.com` 的 .NET 10 執行階段,以及來自 `download.visualstudio.microsoft.com` 的 Visual C++ 與 .NET 8 安裝程式,若與編進 Eidos 的 SHA-256 不符就拒絕;.NET 10 SDK,一律來自 `builds.dotnet.microsoft.com`,以同樣方式檢查;以及用於其他前置需求(.NET Framework 4.8、XACT、.NET 6 和 7)的 `winetricks`,它會從 Microsoft 與 winetricks 使用的其他主機下載它要安裝的東西。[Microsoft 隱私權聲明](https://www.microsoft.com/en-us/privacy/privacystatement) |
| 工作階段日誌 | 儲存 | `~/.local/share/Colony/Eidos/logs/`,權限模式 0600:每個實例十個工作階段,30 天後清除。你的家目錄寫成 `~`,使用者名稱寫成 `<user>`,所以日誌可以原樣貼進 issue。除非你自己送出,它們永遠不會離開你的機器。 |
| 執行紀錄 | 儲存 | `<instance>/logs/sessions/`:每次遊戲或工具執行的設定檔、已啟用的模組及其版本與 Nexus 檔案 id、外掛順序、每個設定檔 INI 的檢查碼、遊戲與 Eidos 版本、結束碼,以及這次執行建立的存檔。保留最新的 50 筆,`eidos sessions` 會讀取它們,`eidos pack` 不會打包它們。它們留在你的機器上。 |
| 更新報告 | 儲存 | `<instance>/logs/updates/<mod>.json`:上一次更新或 Replace 在該模組資料夾裡改變了什麼。`eidos pack` 不會打包它們。 |
| 捷徑 | 儲存 | `install.sh` 會在 `~/.local/share` 底下寫出 `eidos.desktop` 及其圖示;你建立的工具或設定檔捷徑會放到 `~/.local/share/applications`。 |
| 桌面通知 | 皆否 | 預設關閉。開啟時,它們透過 D-Bus(`org.freedesktop.Notifications`)送到你桌面的通知服務,並指出結束的工作。 |
| 你開啟的連結 | 皆否 | 模組頁面與其他連結會透過 `xdg-open` 在你自己的瀏覽器中開啟。 |
| 附加元件 | 皆否 | 你安裝的擴充以獨立程式的身分在你的機器上執行;它們做什麼由它們自己負責。 |

## 授權

Eidos 的原創程式碼採用 GPL-3.0-or-later 授權;見 [LICENSE](../../../LICENSE)。以下部分
採用不含「or later」選項的 GPL-3.0:

- `eidos-install` 中的 OMOD 腳本轉譯;請參閱其
  [上游作者致謝](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- 建立在內附 nifly 函式庫之上的 `eidos-nif-preview` 輔助程式;請參閱其
  [元件授權與作者致謝](../../../crates/eidos-nif-preview/UPSTREAM.md);
- 載入順序程式碼所連結的 `esplugin` 與 `libloadorder` crate,它們宣告的授權是
  `GPL-3.0`。

執行檔中的其他 Rust crate 保留各自的授權(MIT、Apache-2.0、BSD 及類似授權)。發行
套件在 `licenses/crates/` 中附上它們的授權聲明,並附有每個 crate 及其授權的索引。

執行檔也內嵌了八個正版的 Microsoft DirectX 可轉散發 DLL(`d3dcompiler_47`、
`d3dcompiler_43`、`d3dx9_43` 與 `d3dx11_43`,32 位元與 64 位元各一),以 gzip 壓縮存放
(7.9 MB,而非 17.9 MB),當某個模組或工具需要它們時,Eidos 會把它們解開並部署進遊戲的
前綴。它們仍受 Microsoft 可轉散發條款約束;見它們的
[來源說明](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md)。

發行套件還在 `runtimes/` 底下附上 Microsoft 的 Visual C++ 2015-2022 安裝程式、
.NET 8 Desktop Runtime 安裝程式和 .NET 10 執行階段 zip,與 Microsoft 發布的一致。它們的
條款在 `licenses/microsoft-runtimes/`(`vc_redist.txt`、`dotnet.txt`)。
