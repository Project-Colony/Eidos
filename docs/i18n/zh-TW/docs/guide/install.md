<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# 安裝 Eidos

三條路。它們都會給你同樣的兩個執行檔 - `eidos`(命令列)與 `eidos-gui` -
以及 `nxm://` 處理器,讓 Nexus 上的「Mod Manager Download」按鈕直接落進你的實例。

## 你需要先具備

| | |
|---|---|
| **帶 FUSE 的 Linux** | PATH 中要有 `fusermount3`。任何目前的發行版都內建。 |
| **一款用 Proton 啟動過一次的遊戲** | Steam 只在首次啟動時建立遊戲的 Wine 前綴,而 Eidos 在其中運作。 |
| **`7z`** | 用來安裝模組壓縮檔。多數發行版裡叫 `p7zip`。 |

不需要 root,不需要常駐程式,不需要改 `/etc/fuse.conf`,也不需要把你加進任何群組。
Eidos 掛載在屬於遊戲行程的私有命名空間裡。

## Arch

```bash
cd packaging && makepkg -si
```

## 發行壓縮檔

```bash
./install.sh
```

預設安裝到 `~/.local/bin`。`--system` 放到 `/usr/local/bin`,`--bindir DIR` 放到別處。
重新執行它就是受支援的升級方式。

## 從原始碼建置

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## 接著:讓 Steam 指向它

Eidos 是*作為*你遊戲的啟動命令執行的,這正是它能在遊戲啟動前完成掛載的原因。
在 Steam 裡右鍵點遊戲 -> 內容 -> 啟動選項:

```
~/.local/bin/eidos-gui %command%
```

按下開始遊戲。Eidos 會在該遊戲的實例上開啟;安裝模組、用 LOOT 排序、點 Run。
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
見 [usage.zh-TW.md](usage.md),完整導覽也在那裡。

## 選用:FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 會開啟核心 FUSE passthrough。
它**預設關閉,而且你幾乎肯定應該讓它保持關閉**:在 Skyrim SE 上實測,它會讓遊戲
打不開自己的封裝檔與外掛,於是模組靜悄悄地不載入。這個開關的存在是為了重新測試
該機制,而不是因為推薦使用。

細節以及支撐該決定的實測數據,見
[troubleshooting.zh-TW.md](troubleshooting.md)。

## 已經出問題了?

[troubleshooting.zh-TW.md](troubleshooting.md) 說明了環境開關、如何讀
操作計數器,以及迄今為止咬過人的每一個問題。
