<!-- eidos-i18n: source=docs/guide/install.md sha=bc05db1de00bb7c80224f065c3f730f3adc542dc -->

# 安裝 Eidos

共有四種安裝方式。Colony 只安裝一個檔案 `eidos-linux`,它就是整個程式;其他方式則並排提供 `eidos`(命令列)、`eidos-gui` 和靜態 NIF 預覽輔助程式,以及 `nxm://` 處理器,讓 Nexus 上的「Mod Manager Download」按鈕將下載內容送到你的實例。

## 你需要先具備

| | |
|---|---|
| **帶 FUSE 的 Linux** | PATH 中要有 `fusermount3`。任何目前的發行版都內建。 |
| **一款用 Proton 啟動過一次的遊戲** | Steam 只在首次啟動時建立遊戲的 Wine 前綴,而 Eidos 在其中運作。 |
| **`7z`** | 用來安裝模組壓縮檔。多數發行版裡叫 `p7zip`。 |
| **`par2`**(選用) | 為 `eidos pack` 的備份寫出 PAR2 修復資料。Arch 與 Fedora 上是 `par2cmdline`,Debian 與 Ubuntu 上是 `par2`。 |

不需要 root,不需要常駐程式,不需要改 `/etc/fuse.conf`,也不需要把你加進任何群組。
Eidos 掛載在屬於遊戲行程的私有命名空間裡。

## 透過 Colony

在 [Colony](https://github.com/Project-Colony/Colony) 裡搜尋 **Eidos** 並安裝;
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
模型預覽需要 `eidos-nif-preview`,只有發行套件與從原始碼建置的版本才帶有它;少了
它,預覽會說明這一點,其他一切不受影響。讓 Steam 直接指向這個檔案本身,做法如下。

## 發行套件

從[最新發行版](https://github.com/Project-Colony/Eidos/releases/latest)下載
`eidos-linux-bundle.tar.gz` 及其 `.sig`、`.meta` 與 `.meta.sig`,並依照 README 的
[程式碼簽章政策](../../README.md#程式碼簽章政策)驗證它們。接著:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

預設安裝到 `~/.local/bin`。`--system` 放到 `/usr/local/bin`,`--bindir DIR` 放到別處。
重新執行它就是受支援的升級方式。發行套件與 `eidos-linux` 都需要 x86_64 Linux,
以及 glibc 2.39 或更新版本。

## Arch

```bash
cd packaging && makepkg -si
```

這份 PKGBUILD 會從原始碼建置目前的 `main`。

## 從原始碼建置

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

從原始碼建置需要 Rust 1.89 或更新版本、`just`、CMake 3.20 或更新版本、C++17 編譯器,以及用於原生輔助程式測試的 Python 3。不使用 `just` 時,請參閱[原生輔助程式說明](../../../../../crates/eidos-nif-preview/README.md),使用 `cargo build --release --locked` 建置 Rust 程式,將輔助程式複製到 `target/release`,然後執行 `packaging/install.sh --from target/release`。

## 接著:讓 Steam 指向它

Eidos 是*作為*你遊戲的啟動命令執行的,這正是它能在遊戲啟動前完成掛載的原因。
在 Steam 裡右鍵點遊戲 -> 內容 -> 啟動選項:

```
~/.local/bin/eidos-gui %command%
```

透過 Colony 安裝時,路徑是 `~/.local/share/Colony/apps/Eidos/eidos-linux`。按下
開始遊戲。Eidos 會在該遊戲的實例上開啟;安裝模組、用 LOOT 排序、點 Run。
離開時掛載隨之消失,你的安裝目錄與原先分毫不差。

請使用絕對路徑 - Steam 不會讀取你 shell 的 `PATH`。

### 如果你偏好終端機

```sh
eidos init skyrimse               # 建立實例(給出資料夾即為可攜實例)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root 模組
eidos sort skyrimse               # 用 LOOT 排序載入順序
eidos play skyrimse -- %command%  # 讓任何程式穿過合併檢視執行
```

凡是接受遊戲 id 的命令,同樣接受可攜實例的資料夾 -
見 [usage.md](usage.md#實例全域與可攜)。完整導覽在 [usage.md](usage.md)。

## 選用:FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 僅授予選用權限,不會開啟 passthrough。
它要加在發行套件或原始碼建置中獨立的 `eidos` 執行檔上,絕不能加在 `eidos-linux` 上,
因為那個檔案同時也是視窗([SECURITY.md](../../../../../SECURITY.md))。
執行時需另行設定 `EIDOS_FUSE_PASSTHROUGH=1` 才會啟用。
它**預設關閉,而且你幾乎肯定應該讓它保持關閉**:在 Skyrim SE 上實測,它會讓遊戲
打不開自己的封裝檔與外掛,於是模組靜悄悄地不載入。這個開關的存在是為了重新測試
該機制,而不是因為推薦使用。

細節以及支撐該決定的實測數據,見
[troubleshooting.md](troubleshooting.md)。

## 已經出問題了?

[troubleshooting.md](troubleshooting.md) 說明了環境開關、如何讀
操作計數器,以及迄今為止咬過人的每一個問題。
