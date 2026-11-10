<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# 安裝 Eidos

共有四種安裝方式。Colony 只安裝一個檔案 `eidos-linux`,它就是整個程式。發行套件、Arch
套件和從原始碼建置的版本則並排提供 `eidos`(命令列)、`eidos-gui` 和靜態 NIF 預覽輔助
程式 `eidos-nif-preview`。Arch 套件和從原始碼建置的版本還會安裝 `eidos-fuse` 和
`eidos-launch` 這兩個較底層的 CLI。每種方式都能註冊 `nxm://` 處理器,讓 Nexus 上的
「Mod Manager Download」按鈕將下載內容送到你的實例。

## 你需要先具備

| 需求 | 原因 |
|---|---|
| 帶 FUSE 的 Linux | PATH 中要有 `fusermount3`。任何目前的發行版都內建。 |
| 一款用 Proton 啟動過一次的遊戲 | Steam 只在首次啟動時建立遊戲的 Wine 前綴,而 Eidos 在其中運作。Eidos 也能偵測 Skyrim Special Edition 的 GOG 和 Epic 版本、Fallout 3 和 Fallout: New Vegas 的 Epic 版本,以及 Stardew Valley 的 GOG 版本。這些遊戲透過 Heroic 或 Legendary 執行,Eidos 作為遊戲在那裡的包裝器:見 [usage.md](usage.md#啟動遊戲)。 |
| 7-Zip 執行檔:`7z`、`7zz` 或 `7za` | 用來安裝模組壓縮檔。Arch 上是 `7zip`,多數其他發行版是 `p7zip`。 |
| `par2`(選用) | 為 `eidos pack` 的備份寫出 PAR2 修復資料。Arch 與 Fedora 上是 `par2cmdline`,Debian 與 Ubuntu 上是 `par2`。 |

不需要 root,不需要常駐程式,不需要改 `/etc/fuse.conf`,也不需要把你加進任何群組。
Eidos 掛載在屬於遊戲行程的私有命名空間裡。

有些系統設定只會以當機的形式表現出來。Diagnostics 分頁會檢查 `vm.max_map_count`
(1048576 或更大)、硬性的開啟檔案數上限(524288 或更大),以及遊戲、它的前綴或實例是否
位於 NTFS、exFAT 或 FAT 上;每一項的修法見
[troubleshooting.md](troubleshooting.md#系統)。

## 透過 Colony

在 [Colony](https://github.com/Project-Colony/Colony) 裡搜尋 Eidos 並安裝;
更新會透過啟動器送達。Colony 只保留一個檔案,
`~/.local/share/Colony/apps/Eidos/eidos-linux`。執行它就會開啟視窗。
透過名為 `eidos` 的連結呼叫時,同一個檔案就是命令列。在你的 `PATH` 上放一個這樣的
連結,並註冊 Nexus 下載處理器:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

視窗本身不需要連結:每當它啟動遊戲或工具時,都會把自己的檔案當作 CLI 執行。NIF
模型預覽需要 `eidos-nif-preview`,只有發行套件、Arch 套件與從原始碼建置的版本才帶有
它;少了它,預覽會說明這一點,其他一切不受影響。`eidos-linux` 也不帶發行套件附的任何
Microsoft 執行階段([見下](#發行套件中的-microsoft-執行階段)):工具需要其中之一時,
Eidos 會在你開始安裝先決條件後從 Microsoft 下載。讓 Steam 直接指向這個檔案本身,做法
如下。

## 發行套件

從[最新發行版](https://github.com/Project-Colony/Eidos/releases/latest)下載
`eidos-linux-bundle.tar.gz` 及其 `.sig`、`.meta` 與 `.meta.sig`,並依照 README 的
[程式碼簽章政策](../../README.md#程式碼簽章政策)驗證它們。套件的最上層有 `eidos`、
`eidos-gui` 和 `eidos-nif-preview`,以及 `install.sh`、`README.md`、`LICENSE`、
`SECURITY.md`、`assets/brand/png` 裡的圖示、`licenses/` 裡的第三方聲明(nifly、omod、
microsoft-directx、microsoft-runtimes 和 Rust crate),以及下面介紹的 `runtimes/` 資料夾。
它不含 `eidos-fuse` 或 `eidos-launch`。接著:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` 把執行檔複製到 `~/.local/bin`,若設了 `$XDG_BIN_HOME` 則複製到那裡。
`--system` 透過 sudo 放到 `/usr/local/bin`,`--bindir DIR` 放到別處,`--from DIR` 從另
一個目錄取執行檔。這個腳本還會:

- 執行 `eidos nxm --register` 註冊 `nxm://` 處理器,所以 Colony 一節裡的註冊步驟不需要;
- 寫入 `~/.local/share/applications/eidos.desktop`,並把 Eidos 圖示放到
  `~/.local/share/icons/hicolor` 底下;
- 把授權聲明複製到 `~/.local/share/licenses/eidos`(加 `--system` 時是
  `/usr/local/share/licenses/eidos`);
- 把 `runtimes/` 複製到 `~/.local/share/Colony/Eidos/runtimes`,加 `--system` 時也一樣,
  所以這些執行階段屬於執行腳本的使用者;
- 最後印出 Steam 啟動選項,其中帶有它所安裝的 `eidos-gui` 的絕對路徑。

`--cap` 請 sudo 把 `CAP_SYS_ADMIN` 授予所安裝的 `eidos`(見下面的 FUSE passthrough)。
`--no-cap` 是預設,保持無需 root。重新執行這個腳本就是受支援的升級方式。發行套件與
`eidos-linux` 都需要 x86_64 Linux,以及 glibc 2.39 或更新版本。

### 發行套件中的 Microsoft 執行階段

套件的 `runtimes/` 資料夾放著工具會要求的 Microsoft 檔案:`VC_redist.x86.exe` 和
`VC_redist.x64.exe`(Visual C++ 2015-2022,即 `vcrun2022` 先決條件)、
`windowsdesktop-runtime-8.0.12-win-x64.exe`(.NET 8 Desktop Runtime,供 `dotnet8` 和
`dotnetdesktop8` 使用),以及 `dotnet-runtime-10.0.10-win-x64.zip`(.NET 10 執行階段,供
`dotnet10` 使用)。Eidos 自己在遊戲的前綴裡執行 Visual C++ 和 .NET 8 的安裝程式,不經
winetricks,並把 .NET 10 的 zip 一次解開到 `~/.local/share/Colony/Eidos/runtimes`,所有
實例共用(解開後 78 MB)。每個檔案只有在其 SHA-256 與 Eidos 內固定的值相符時才會使用。
Synthesis 需要的 .NET 10 SDK(`dotnetsdk10`,約 300 MB)不在套件裡,一律下載。

解開後的套件也能原地執行:Eidos 在上述資料資料夾之後,會在自己執行檔旁邊找 `runtimes/`。

## Arch

```bash
cd packaging && makepkg -si
```

這份 PKGBUILD 會從原始碼建置目前的 `main`。它把 `eidos`、`eidos-gui`、
`eidos-nif-preview`、`eidos-fuse` 和 `eidos-launch` 安裝到 `/usr/bin`,並附帶系統層級的
`nxm://` 處理器、桌面項目和圖示。它相依於 `fuse3`、`7zip` 和 `xdg-utils`。它不帶
`runtimes/` 資料夾,所以 Microsoft 執行階段在工具需要時才下載。`winetricks` 為選用:
Eidos 仍會用它處理自己不安裝的先決條件,例如 `dotnet48` 和 `xact`。

## 從原始碼建置

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

從原始碼建置需要 Rust 1.92 或更新版本、`just`、CMake 3.20 或更新版本、C++17 編譯器,以及用於原生輔助程式測試的 Python 3。`just build` 建置整個工作區,所以 `eidos-fuse` 和 `eidos-launch` 也會建置,`just install` 對建置輸出執行 `packaging/install.sh`,把它們和其餘檔案一起複製。不使用 `just` 時,請參閱[原生輔助程式說明](../../../../../crates/eidos-nif-preview/README.md),使用 `cargo build --release --locked` 建置 Rust 程式,將輔助程式複製到 `target/release`,然後執行 `packaging/install.sh --from target/release`。從原始碼建置的版本同樣沒有 `runtimes/` 資料夾,所以 Microsoft 執行階段在工具需要時才下載。

release 設定檔使用一個 codegen unit、fat LTO 和去除符號。用 rustc 1.99.1 和 3 個建置工作,
`eidos` 與 `eidos-gui` 的冷建置花了 334 秒,thin LTO 則是 243 秒,產出 20.9 MB 的 `eidos`
和 49.7 MB 的 `eidos-gui`。每個執行檔內嵌八個以 gzip 壓縮存放的 DirectX DLL:7.9 MB,
而非 17.9 MB。`just profile=dev build` 不做最佳化直接建置。

## 接著:讓 Steam 指向它

Eidos 是*作為*你遊戲的啟動命令執行的,這正是它能在遊戲啟動前完成掛載的原因。
在 Steam 裡右鍵點遊戲 -> 內容 -> 啟動選項,輸入 Eidos 視窗的絕對路徑,後面接
`%command%`:

```
~/.local/bin/eidos-gui %command%
```

請使用絕對路徑 - Steam 不會讀取你 shell 的 `PATH`。路徑取決於你怎麼安裝 Eidos,
`install.sh` 會印出它所用目錄對應的那一行:

| 安裝方式 | 啟動選項 |
|---|---|
| 發行套件或 `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Arch 套件 | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

按下 Play。Eidos 會在該遊戲的實例上開啟;安裝模組、用 LOOT 排序、點 Run。
離開時掛載隨之消失,你的安裝目錄與原先分毫不差。

遊戲一律透過它的商店啟動。在你自己打開的視窗裡,Run 會執行 `steam -applaunch <appid>`,
Steam 透過啟動選項啟動 Eidos,然後開啟第二個 Eidos 視窗,你在那裡按 Run。在啟動選項
啟動 Eidos 之前,Run 什麼也不啟動,狀態列會顯示該設定的那一行。開啟 Start the game at
once when a store opens Eidos(Preferences, General, Running a game;預設關閉)後,在
Health 沒有列出問題時,在 Steam 按 Play 會不開視窗直接啟動遊戲。Heroic 和 Legendary
透過它們的包裝器設定以同樣方式運作;細節見 [usage.md](usage.md#啟動遊戲)。

Diagnostics 分頁會檢查這一行。啟動選項為空、沒有指名 Eidos 執行檔、指向不存在的檔案、
Eidos 後面沒有 `%command%`,或啟動的是與正在執行的不同的另一份 Eidos 時,Health 卡片
會顯示該設定的那一行。

### 如果你偏好終端機

```sh
eidos init skyrimse               # 建立實例(給出資料夾即為可攜實例)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root 模組
eidos sort skyrimse               # 用 LOOT 排序載入順序
eidos check skyrimse              # 不需要視窗的 Health 檢查
eidos play skyrimse -- %command%  # 讓任何程式穿過合併檢視執行
eidos shell skyrimse              # 合併檢視裡的 shell,寫入的內容進入 Overwrite
```

凡是接受遊戲 id 的命令,同樣接受可攜實例的資料夾 -
見 [usage.md](usage.md#實例全域與可攜)。完整導覽在 [usage.md](usage.md)。

## 更新

Colony 透過它的啟動器更新 `eidos-linux`。發行套件請執行新套件的 `install.sh`;Arch 套件
再執行一次 `makepkg -si`;從原始碼建置的版本則拉取後執行 `just install`。視窗在啟動時向
GitHub 詢問最新發行版,每天最多一次,有更新版本時,會在選單列的 Help 之後顯示指向其
發行說明的連結。它絕不會自行下載或替換自己。Preferences, About, Version 可以關掉這項
檢查,離線模式會略過它,安裝在 `/usr` 底下的副本(Arch 套件)從不詢問,因為由套件管理員
更新它。

## 選用:FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 僅授予選用權限,不會開啟
passthrough。`./install.sh --cap` 和 `just setcap` 會把它授予它們所安裝或建置的
`eidos`。在 `nosuid` 掛載上,核心會忽略檔案權限,所以 `install.sh` 會帶著警告略過授予,
`just setcap` 則以錯誤停止。權限要加在發行套件、Arch 套件或原始碼建置中獨立的 `eidos`
執行檔上,絕不能加在 `eidos-linux` 上,因為那個檔案同時也是視窗
([SECURITY.md](../../../../../SECURITY.md))。重新建置或重新安裝的執行檔是沒有該權限的
新檔案,如果你要用它,每次升級後都要再授予一次。

`EIDOS_FUSE_PASSTHROUGH=1` 是另一個執行時開關,預設關閉。passthrough 開著時,較早版本的
常駐程式會讓 Skyrim SE 無法載入任何模組內容。最可能的原因是它每次開啟都註冊一個新的
後端檔案,而核心每個 inode 只接受一個。常駐程式現在讓一個檔案的所有 passthrough 開啟
共用同一個後端,但這還沒在遊戲裡重新測量過。在那之前,只為了測試才開啟 passthrough。
實測數據和原因見 [troubleshooting.md](troubleshooting.md)。

## 已經出問題了?

[troubleshooting.md](troubleshooting.md) 說明了環境開關、如何讀
操作計數器、Health 卡片,以及迄今為止咬過人的每一個問題。
