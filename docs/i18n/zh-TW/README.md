<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**從不碰你遊戲的原生 Linux 模組管理器。**

</div>

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

**實例。** 全域 - 集中管理在 `~/.local/share/eidos` 底下 - 或可攜:一個自我完備的
資料夾,放在你想要的任何地方(第二顆硬碟、遊戲分割區),可移動、彼此隔離,和 MO2
的一樣。可攜實例會跨工作階段被記住;GUI、Steam 啟動與每一個 CLI 命令都跟隨你上次
用的那一個,而凡是接受遊戲 id 的命令,同樣接受那個資料夾。細節見
[usage.zh-TW.md](docs/guide/usage.md#實例全域與可攜)。

**設定檔。** 每個設定檔各有自己的模組順序、外掛狀態、INI 與存檔。存檔會被解析、和你
目前的外掛比對 - 還附一個按鈕啟用存檔需要的東西 - 並在每次工作階段之後同步回去給
Steam Cloud。

**Nexus。** 連上一個帳號,網站的「Mod Manager Download」按鈕就直接落進你的實例,
並會對照你已安裝的東西做更新檢查,顯示每個模組是誰做的,以及一條連到其個人頁面的
連結。一條**合集**連結會列出它的成員,並和你的實例對照 - 已安裝、已下載、缺少 -
那是在讀取一份合集,而不是安裝一份,面板上也說明了原因。Downloads 分頁是一座壓縮檔
圖書館:篩選、排序、不刪除就隱藏,以及清掉那些已經安裝過的。一個**離線**開關會把
這一切全部停下。

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

**它把自己的檔案放在哪裡。** `~/.config/Colony/Eidos/` 放你選的東西 - 偏好設定、
你的 Nexus 工作階段、你的實例清單、你自己寫的遊戲與附加元件定義 - 日誌則在
`~/.local/state/Colony/Eidos/` 底下。這是 Colony 家族每個程式都用的配置。舊版 Eidos
把這些放在 `~/.config/eidos/`;升級後的第一次啟動會把它們複製過來,在日誌裡說明,
並讓舊目錄原封不動留著。

## 它和其他方案的比較

| | Eidos | 透過 Wine 的 MO2 | Fluorine-Manager | Limo / 連結式部署器 |
|---|---|---|---|---|
| 管理器原生執行 | ✅ | ❌ Wine 裡的 Windows 程式 | ✅(Qt 移植) | ✅ |
| 遊戲目錄不受動 | ✅ 永遠 | ✅ | ✅ | ❌ 連結被寫進去 |
| 掛載對誰可見 | 只有遊戲 | 只有遊戲 | **整個系統** | n/a |
| 當機後需要清理 | 沒有,設計使然 | 沒有 | 陳舊掛載的復原 | 手動取消部署 |
| root 模組(ENB、預載器) | ✅ 原生 | 需要外掛程式 | 需要外掛程式 | 部分 |
| 需要的特權 | 沒有 | 沒有 | 改 `/etc/fuse.conf` | 沒有 |

## 它有多快

| | 之前 | 現在 |
|---|---|---|
| 載入一份存檔 | ~20 秒 | **6-7 秒** |
| 一次工作階段裡的目錄讀取 | 560 萬 | 46.5 萬 |

Cell 切換是即時的。這份收益來自少問你的模組幾個問題:以前找一個檔案要把五十個模組
逐一問過,列一個資料夾則要重複做五十遍。現在兩者都不做了。這是在一個正常遊玩的
真實實例上量出來的,不是在跑分程式上。

## 開始上手

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

接著把你遊戲的 Steam 啟動選項設成 `~/.local/bin/eidos-gui %command%`,然後按下
開始遊戲。

Arch 套件與發行壓縮檔、你需要先裝好的東西,以及命令列這條路:
**[docs/guide/install.zh-TW.md](docs/guide/install.md)**。

## Steam 啟動選項

大多數設定需要的就只有這條基本行:

```
~/.local/bin/eidos-gui %command%
```

其餘的一切都是疊在它前面的環境變數,而且可以自由組合:

| 你想要... | 放在前面 |
|---|---|
| 搭配 Community Shaders 的 DLSS | `PROTON_ENABLE_NVAPI=1` - 沒有它,DLSS 會靜悄悄地永遠不初始化;完整檢查清單在 [guide/graphics.zh-TW.md](docs/guide/graphics.md) |
| 畫面上的 FPS 計數器 | `DXVK_HUD=fps` |
| 驅動層級的影格插補,零模組(RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - 絕不要和 Community Shaders 自己的影格生成一起用 |
| 給錯誤回報用的詳細日誌 | `EIDOS_LOG=debug`(工作階段日誌落在 `~/.local/state/Colony/Eidos/logs/`) |
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
[guide/troubleshooting.zh-TW.md](docs/guide/troubleshooting.md)。

## 接下來去哪裡

| 如果你想... | |
|---|---|
| 安裝它 | [guide/install.zh-TW.md](docs/guide/install.md) |
| 學會 CLI 與 GUI | [guide/usage.zh-TW.md](docs/guide/usage.md) |
| 設定 xEdit、BodySlide 或 DynDOLOD | [guide/tools.zh-TW.md](docs/guide/tools.md) |
| 玩 Fallout 4(F4SE、版本、NVIDIA 武器碎片當機) | [guide/fallout4.zh-TW.md](docs/guide/fallout4.md) |
| 讓 DLSS / 影格生成運作起來(Community Shaders) | [guide/graphics.zh-TW.md](docs/guide/graphics.md) |
| 修好某個看起來不對勁的東西 | [guide/troubleshooting.zh-TW.md](docs/guide/troubleshooting.md) |
| 知道它為什麼快,並自己驗證 | [internals/performance.md](../../internals/performance.md) |
| 理解它內部如何運作 | [internals/architecture.md](../../internals/architecture.md) |
| 建置它、測試它、貢獻 | [internals/contributing.md](../../internals/contributing.md) |
| 知道它究竟為什麼存在 | [project/landscape.md](../../project/landscape.md) |

一種語言就是一個目錄:`docs/i18n/zh-TW/` 鏡像了儲存庫根目錄的結構,因此兩個譯文
頁面之間的連結與它們英文原文之間的連結是同一串字元。

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
需要它們的讀者,多出 17,678 個字要維持誠實。

每一份翻譯都帶著它所依據的英文檔案的雜湊值,而當英文往前走時 CI 會失敗 - 見
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh)。無法被帶回到最新狀態的翻譯會被
**刪除**,而不是留在原地:一個過期的頁面看起來仍然權威,卻發出上個月的命令,對
讀者來說比被送去看英文更糟。

加一種語言就是四個檔案加上這張表裡的一列;
[`docs/internals/contributing.md`](../../internals/contributing.md) 寫了步驟。

## 支援的遊戲

**Skyrim SE/AE** - 在實際遊玩中得到驗證。**Fallout 4** 也已經接好整條線(自動換上
F4SE、封裝檔失效、星號載入順序、LOOT、`.fos` 存檔)- 見
[guide/fallout4.zh-TW.md](docs/guide/fallout4.md)。依共用的遊戲描述符接好線、
正在找測試者的有:Skyrim LE、Skyrim VR、Enderal SE、Fallout 3、Fallout NV、
Fallout 4(+ VR)、Starfield、Oblivion 與 Morrowind(最後兩款會掛載並管理模組;
它們以時間戳排序的外掛清單還沒有被管理)。

加一個家族就是一列描述符:
[internals/adding-games.md](../../internals/adding-games.md)。

## 前人成果與致謝

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) 與
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos 重現的語意,以及它據以
  研究對等程度的那份程式碼庫
- [LOOT](https://loot.github.io/) - 排序引擎,透過 libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager)、
  [Limo](https://github.com/limo-app/limo) 與其他 Linux 管理器 - 證明有一個社群
  想看到這個問題被解決

## 授權

GPL-3.0-or-later。模組管理屬於所有人。
