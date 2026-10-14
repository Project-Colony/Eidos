<!-- eidos-i18n: source=README.md sha=c4073ef7daa93b14edfa3d1e00794a2cdbdf8066 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**從不碰你遊戲的原生 Linux 模組管理器。**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#安裝)

Eidos 為 Linux 上的 Bethesda 遊戲帶來 Mod Organizer 2 在 Windows 上帶來的東西 -
一份虛擬的、每次啟動重建的模組合併檢視 - 建立在 Linux 原生機制上,而不是 Windows
API 掛鉤。管理器不需要 Wine。沒有檔案被複製進遊戲目錄。沒有清理程序,因為根本沒有
東西需要清理。

```
Steam ──> eidos-gui %command% ──> [ 私有命名空間 ]
                                  │  模組 ⊕ 遊戲  ──> 遊戲看到的東西
                                  └─ 隨遊戲一同消亡;安裝目錄始終原封不動
```

> **狀態:** Skyrim SE 每天都透過 Eidos 遊玩 - SKSE、script extender 預載器、
> Creation Club、LOOT 排序過的載入順序、各設定檔獨立的存檔,全部都在內。目前有
> 一個遊戲家族在實際遊玩中得到驗證;另外十個已經接好線,等著測試者。

## 為什麼是 Eidos

- 🔒 **只有你的遊戲看得見的掛載。** 合併檢視活在一個私有的掛載命名空間裡:你的
  檔案管理器、你的備份工作、另一款遊戲 - 沒有一個看得到它,也沒有一個需要為它
  取得授權。殺掉遊戲、直接拔電源:命名空間隨行程樹一同消亡,而你的安裝目錄和
  先前分毫不差。殘留物*從構造上*就不存在。
- 🧾 **真相只有一份。** 你的設定檔擁有自己的模組清單、外掛順序、INI 與存檔。外掛
  檔案與存檔目錄在啟動時 bind-mount 蓋在遊戲自己的路徑之上,所以連遊戲自己寫出的
  東西也落進你的設定檔。切換設定檔就切換一切。
- 🐧 **完全免 root。** 沒有 setuid 輔助程式,沒有常駐程式,不用 `sudo setcap`,
  不用改 `/etc/fuse.conf`。一個執行檔,一個 Steam 啟動選項。
- 🛡️ **有憑有據的防護。** 一次弄壞你外掛清單的當機,會被對照工作階段前的快照
  標示出來,並附一鍵還原。一次會抹掉你載入順序的擷取會被拒絕,並說明原因。

### 它和其他方案的比較

| | Eidos | 透過 Wine 的 MO2 | Fluorine-Manager | Limo / 連結式部署器 |
|---|---|---|---|---|
| 管理器原生執行 | ✅ | ❌ Wine 裡的 Windows 程式 | ✅(Qt 移植) | ✅ |
| 遊戲目錄不受動 | ✅ 永遠 | ✅ | ✅ | ❌ 連結被寫進去 |
| 掛載對誰可見 | 只有遊戲 | 只有遊戲 | **整個系統** | n/a |
| 當機後需要清理 | 沒有,設計使然 | 沒有 | 陳舊掛載的復原 | 手動取消部署 |
| root 模組(ENB、預載器) | ✅ 原生 | 需要外掛程式 | 需要外掛程式 | 部分 |
| 需要的特權 | 沒有 | 沒有 | 改 `/etc/fuse.conf` | 沒有 |

### 它有多快

| | 之前 | 現在 |
|---|---|---|
| 載入一份存檔 | ~20 秒 | **6-7 秒** |
| 一次工作階段裡的目錄讀取 | 560 萬 | 46.5 萬 |

Cell 切換是即時的。這份收益來自少問你的模組幾個問題:以前找一個檔案要把五十個模組
逐一問過,列一個資料夾則要重複做五十遍。現在兩者都不做了。這是在一個正常遊玩的
真實實例上量出來的,不是在跑分程式上。

### 前人成果與致謝

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) 與
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos 重現的語意,以及它據以
  研究對等程度的那份程式碼庫
- [LOOT](https://loot.github.io/) - 排序引擎,透過 libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager)、
  [Limo](https://github.com/limo-app/limo) 與其他 Linux 管理器 - 證明有一個社群
  想看到這個問題被解決

## 它做些什麼

**模組。** 單純的壓縮檔、FOMOD 精靈、Wrye Bash BAIN 套件,其餘的則有一個手動選擇器 -
以及**原生支援 root 模組**(script extender 預載器、ENB、Engine Fixes),不需要
Root Builder 外掛程式,也沒有任何東西被複製進你的安裝目錄。隱藏單一檔案、用分隔線
分組、指定目標的移動、每個模組專屬的註記與類別,還有一個 MO2 設定檔匯入器。

這份清單就是 MO2 的那一份,連同它的習慣:八個選用欄位以及依其中任一個排序、依類別
或依來源分組、雙擊手勢、鍵入即跳、每個模組專屬的備份 - 在你還原之前它們都是惰性的 -
以及對某個模組的提示性旗標:這個遊戲不會載入它的版面,或者它是為另一款遊戲下載的。
它的檔案樹會做那些一般操作 - 新資料夾、改名、刪除、開啟 - 並且不啟動任何東西就能
預覽圖片與文字。

**外掛。** 內建 LOOT 排序的載入順序、和遊戲自己算出來一樣的模組索引、缺少 master
的警告,以及以未受管理的列的身分顯示的 DLC 與 Creation Club 內容。

**實例。** 全域 - 集中管理在 `~/.local/share/Colony/Eidos/instances/` 底下 - 或可攜:一個自我完備的
資料夾,放在你想要的任何地方(第二顆硬碟、遊戲分割區),可移動、彼此隔離,和 MO2
的一樣。可攜實例會跨工作階段被記住;GUI、Steam 啟動與每一個 CLI 命令都跟隨你上次
用的那一個,而凡是接受遊戲 id 的命令,同樣接受那個資料夾。細節見
[usage.md](docs/guide/usage.md#實例全域與可攜)。

**設定檔。** 每個設定檔各有自己的模組順序、外掛狀態、INI 與存檔。存檔會被解析、和你
目前的外掛比對 - 還附一個按鈕啟用存檔需要的東西 - 並在每次工作階段之後同步回去給
Steam Cloud。

**Nexus。** 連接帳號後，可以接收「Mod Manager Download」連結、檢查更新，以及查看作者及其個人資料。合集會按照配方安裝，支援中斷後繼續，並明確回報不支援的步驟。比對時會核對確切的來源檔案；不完整的舊中繼資料仍標示為未驗證。下載分頁可以篩選、排序、隱藏和刪除壓縮檔。**離線**開關會停止存取 Nexus。

**工具。** xEdit、BodySlide、DynDOLOD 與同類程式是*穿過合併檢視*、在遊戲自己的
Proton 前綴裡執行的 - 它們看得到你的模組,它們的輸出落進 Overwrite,一鍵就變成
真正的模組。每個工具需要什麼執行階段,都會在你要求時取得,所以一個缺少的 DLL 是
一個按鈕,而不是一個下午。xEdit 和它的 QuickAutoClean 雙生版會替你找出來 - 在遊戲
資料夾裡、在某個模組裡,或在你擺在遊戲旁邊的工具資料夾裡 - 而且已經選好了正確的
執行階段。把你會用的釘起來,把你不用的藏起來,當某個工具本身就是一個 Steam
應用程式時給它自己的
Steam AppID,並寫出一個 `.desktop` 捷徑,讓它完全不必打開 Eidos 就能穿過合併檢視
啟動。

**診斷。** 缺少的 master、無主的壓縮檔、模組清單偏移、損壞的外掛組合 - 以及在一次
執行之後,script extender 自己的日誌說實際載入了什麼。

**它把自己的檔案放在哪裡。** 這是 Colony 家族每個程式都用的配置:`~/.config/Colony/Eidos/` 放你選的東西 - 偏好設定、你的 Nexus 工作階段、你的實例清單、你自己寫的遊戲與附加元件定義 - `~/.local/share/Colony/Eidos/` 放全域實例、下載的執行環境與工作階段日誌,`~/.cache/Colony/Eidos/` 放 LOOT 主清單。舊版 Eidos 把這些放在 `~/.config/eidos/`、`~/.local/share/eidos/` 與 `~/.local/state/`;升級後的第一次啟動會把它們搬過來,在日誌裡說明,並讓舊目錄原封不動留著。搬移完成之前,Eidos 會繼續使用舊檔案。詳情見 [internals/paths.md](../../internals/paths.md)（英文）。

關於替換保護、BSA/BA2 內部檔案衝突、SKSE 靜態診斷、產生紀錄及其限制，請參閱[完整性檢查與產生檔案](../../../docs/internals/integrity.md)（英文）。

### 支援的遊戲

**Skyrim SE/AE** - 在實際遊玩中得到驗證。**Fallout 4** 也已經接好整條線(自動換上
F4SE、封裝檔失效、星號載入順序、LOOT、`.fos` 存檔)- 見
[guide/fallout4.md](docs/guide/fallout4.md)。依共用的遊戲描述符接好線、
正在找測試者的有:Skyrim LE、Skyrim VR、Enderal SE、Fallout 3、Fallout NV、
Fallout 4(+ VR)、Starfield、Oblivion 與 Morrowind(最後兩款會掛載並管理模組;
它們以時間戳排序的外掛清單還沒有被管理)。

加一個家族就是一列描述符:
[internals/adding-games.md](../../internals/adding-games.md)(英文)。

## 安裝

### 透過 Colony(推薦)

在 [Colony](https://github.com/Project-Colony/Colony) 裡搜尋 **Eidos** 並安裝。
更新會透過啟動器送達。Colony 只安裝一個檔案 `eidos-linux`:它是視窗,而透過名為
`eidos` 的連結呼叫時,它也是 `eidos` 命令列。NIF 模型預覽需要一個只有下方套件才帶有
的輔助程式;少了它,預覽會說明這一點,其他一切照常運作。安裝之後該做什麼,以及如何
讓 Steam 指向它:[docs/guide/install.md](docs/guide/install.md#透過-colony)。

### 直接下載執行檔

從[最新發行版](https://github.com/Project-Colony/Eidos/releases/latest)取得一個檔案:

| 檔案 | 它是什麼 |
|---|---|
| `eidos-linux` | 整個程式合為一個檔案,和 Colony 安裝的一樣 |
| `eidos-linux-bundle.tar.gz` | `eidos`、`eidos-gui`、NIF 預覽輔助程式、圖示,以及一個會註冊桌面項目與 Nexus 下載處理器的安裝程式 |

先依照[程式碼簽章政策](#程式碼簽章政策)驗證下載的檔案,然後:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

安裝程式不需要 sudo,會把一切放進 `~/.local/bin`;`--system` 與 `--bindir DIR`
可以選擇其他位置。兩種檔案都需要 x86_64 Linux、glibc 2.39 或更新版本、FUSE
(`fusermount3`),以及用來處理模組壓縮檔的 7-Zip。

### 從原始碼建置

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

需要 Rust 1.89 或更新版本(CI 檢查的 `rust-version`;CI 與發行版以 1.99.0 建置),
另加 `just`、CMake 3.20 或更新版本,以及供 NIF 預覽輔助程式使用的 C++17 編譯器。
Arch 套件可從 `packaging/PKGBUILD` 建置。前置需求以及不用 `just` 的命令:
[docs/guide/install.md](docs/guide/install.md)。

### Steam 啟動選項

Eidos 是*作為*遊戲的啟動命令執行的,這正是它能在遊戲啟動前完成掛載的原因。大多數
設定需要的就只有這條基本行,並使用你安裝位置的絕對路徑(Steam 不會讀取你的 `PATH`):

```
~/.local/bin/eidos-gui %command%
```

透過 Colony 安裝時,這個路徑是 `~/.local/share/Colony/apps/Eidos/eidos-linux`。接著
按下開始遊戲:Eidos 會在該遊戲的實例上開啟,而 Run 會讓遊戲穿過合併檢視啟動。

其餘的一切都是疊在它前面的環境變數,而且可以自由組合:

| 你想要... | 放在前面 |
|---|---|
| 搭配 Community Shaders 的 DLSS | `PROTON_ENABLE_NVAPI=1` - 沒有它,DLSS 會靜悄悄地永遠不初始化;完整檢查清單在 [guide/graphics.md](docs/guide/graphics.md) |
| 畫面上的 FPS 計數器 | `DXVK_HUD=fps` |
| 驅動層級的影格插補,零模組(RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - 絕不要和 Community Shaders 自己的影格生成一起用 |
| 給錯誤回報用的詳細日誌 | `EIDOS_LOG=debug`(工作階段日誌落在 `~/.local/share/Colony/Eidos/logs/`) |
| 掛載端每個工作階段的 I/O 報告 | `EIDOS_FUSE_STATS=1` |
| 不同的 FUSE 工作執行緒數量 | `EIDOS_FUSE_THREADS=8`(預設 4;追查併發問題時,`1` 是第一個該試的) |
| 把這次啟動釘在某個可攜實例上 | `EIDOS_INSTANCE=/path/to/folder` - 沒有它,Eidos 會開啟你上次用的那個實例,那通常正是你要的 |

現代模組化設定(Community Shaders、DLSS、影格生成)該留下的那一行 - 這就是最終的
命令,不是範例:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

在確認設定可用的期間把 `DXVK_HUD=fps` 加在前面,確認之後就拿掉。

更深入的診斷開關(`EIDOS_FUSE_TRACE`、快取與索引的二分排查開關,以及
`EIDOS_FUSE_PASSTHROUGH` 為什麼預設關閉)住在
[guide/troubleshooting.md](docs/guide/troubleshooting.md)。

## 文件

一切都在 [docs/](../../) 裡,並在 [docs/README.md](docs/README.md) 依讀者分類索引:
使用者指南(安裝、CLI 與 GUI、xEdit 與 DynDOLOD 之類的工具、圖形、疑難排解)、
Eidos 內部如何運作,以及它為什麼存在。安全政策以及如何回報漏洞:
[SECURITY.md](../../../SECURITY.md)。

## 語言

玩家需要的頁面都有翻譯。**英文是準的**:當翻譯和它有出入時,以英文檔案為準。

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

**其餘的一切是刻意用英文,不是漏掉。** `docs/internals/` 與 `docs/project/` 的讀者
同時也在讀 Rust 程式碼,而 `CHANGELOG.md` 是產生出來的。翻譯它們等於為一群並不
需要它們的讀者,多出 18,536 個字要維持誠實。

每一份翻譯都帶著它所依據的英文檔案的雜湊值,而當英文往前走時 CI 會失敗 - 見
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh)。無法被帶回到最新狀態的翻譯會被
**刪除**,而不是留在原地:一個過期的頁面看起來仍然權威,卻發出上個月的命令,對
讀者來說比被送去看英文更糟。

一種語言就是一個目錄。`docs/i18n/<lang>/` 鏡像了儲存庫根目錄的結構,所以
`docs/i18n/de/docs/guide/install.md` 就是德文版的 `docs/guide/install.md` - 正因如此,
兩個譯文頁面之間的連結與它們英文原文之間的連結是同一串字元,而要撤下一種語言也
只需要一次 `rm -r`。

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
任何伺服器,而且只在表中所寫的動作時才連線。**離線**模式(Preferences -> Nexus)會
停止所有與 Nexus 的連線。

| 資料 | 儲存或傳送 | 在哪裡,以及為什麼 |
|---|---|---|
| 偏好設定、實例清單、遊戲與附加元件定義 | 儲存 | `~/.config/Colony/Eidos/` |
| 實例、下載的執行環境、工作階段日誌 | 儲存 | `~/.local/share/Colony/Eidos/`,或你為可攜實例選擇的資料夾 |
| LOOT 主清單 | 儲存 | `~/.cache/Colony/Eidos/loot/` |
| Nexus 登入 | 傳送 | 登入時會在你的瀏覽器開啟 `users.nexusmods.com`,它會把結果交回給 Eidos 只為這次登入在 `127.0.0.1` 上開啟的監聽程式。接著 Eidos 在 `users.nexusmods.com/oauth/token` 把它換成權杖,並取得 Nexus 用來簽署權杖的公開金鑰。[Nexus Mods 隱私權政策](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus 憑證 | 儲存 | 存取權杖與更新權杖,存放在 `~/.config/Colony/Eidos/nexus.ini`,權限模式 0600。舊版 Eidos 寫在那裡的 `api_key=` 行會保留但永遠不會被讀取;你想刪就刪。 |
| Nexus 請求 | 傳送 | `api.nexusmods.com`,在你使用 Nexus 功能時:你的帳號名稱與 premium 狀態、模組與檔案細節、更新檢查、合集與下載連結。每個請求都帶著你的存取權杖,並標明 Eidos 及其版本。 |
| Nexus 下載 | 傳送 | 你要求的檔案,來自下載連結所指名的 Nexus CDN 主機,存進實例的下載資料夾,每個檔案旁邊附一個 MO2 格式的 `.meta`。 |
| `nxm://` 連結 | 儲存 | `eidos nxm --register`(`install.sh` 會替你執行)會寫出 `~/.local/share/applications/eidos-nxm.desktop`,並透過 `xdg-mime` 把它設為 `nxm://` 處理器,讓網站上的 Mod Manager Download 按鈕把連結交給 Eidos。Arch 套件會在全系統安裝同樣的項目。 |
| LOOT 主清單 | 傳送 | `raw.githubusercontent.com/loot/...`,在某款遊戲第一次以 LOOT 排序時,以及你要求更新時。不會送出任何屬於你的資料。[GitHub 隱私權聲明](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| 工具執行環境 | 傳送 | 只在你安裝某個工具的前置需求,或某個模組需要它時:來自 `builds.dotnet.microsoft.com` 的 .NET 執行環境(若與編進 Eidos 的 SHA-256 不符就拒絕),以及 `winetricks`,它會從 Microsoft 與 winetricks 使用的其他主機下載它要安裝進遊戲 Proton 前綴的東西(Visual C++ 與 .NET 可轉散發套件)。[Microsoft 隱私權聲明](https://www.microsoft.com/en-us/privacy/privacystatement) |
| 工作階段日誌 | 儲存 | `~/.local/share/Colony/Eidos/logs/`,權限模式 0600:每個實例十個工作階段,30 天後清除。你的家目錄寫成 `~`,使用者名稱寫成 `<user>`,所以日誌可以原樣貼進 issue。除非你自己送出,它們永遠不會離開你的機器。 |
| 捷徑 | 儲存 | `install.sh` 會在 `~/.local/share` 底下寫出 `eidos.desktop` 及其圖示;你建立的工具捷徑會放到 `~/.local/share/applications`。 |
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
`d3dcompiler_43`、`d3dx9_43` 與 `d3dx11_43`,32 位元與 64 位元各一),當某個模組或
工具需要它們時,Eidos 會把它們部署進遊戲的前綴。它們仍受 Microsoft 可轉散發條款
約束;見它們的[來源說明](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md)。

模組管理屬於所有人。
