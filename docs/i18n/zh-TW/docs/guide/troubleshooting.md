<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=af0c14c5192ccae592dd1c1661fd02d5242ecd5a -->

# 疑難排解與診斷

環境開關、如何讀操作計數器、Diagnostics 分頁的 Health 卡片、如何找出造成當機的模組、
回報錯誤時該附上什麼,以及已知問題及其歷史。

## 診斷 VFS

當遊戲看見的東西與檔案系統對不上時,有兩個環境變數涵蓋大多數情況:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

細分的那種形式,正是找出已知問題裡 `FOPEN_KEEP_CACHE` 那次當機的東西:四個全關回答的是
「是不是快取的問題?」,只逐一指名才回答「是哪一個」。計數器回答另外一半。passthrough
開著時(預設關閉),一次顯示 `read 0` 的載入,代表每一個位元組都由 `FUSE_PASSTHROUGH`
在核心裡服務掉了,所以你原本打算在讀取路徑上調校的東西,早就不花成本了。

另外還有幾個開關可以進一步縮小故障範圍:

```sh
EIDOS_FUSE_TRACE=open,opendir       # log each open / opendir (1 = every channel)
EIDOS_FUSE_THREADS=1                # one daemon thread (default: CPU count, at most 4)
EIDOS_FUSE_OPENDIR=1                # keep opendir handles even when the kernel can skip them
EIDOS_NO_INDEX=1                    # walk the layers instead of using the layer index
EIDOS_NO_OW_INDEX=1                 # the same for the Overwrite's membership index
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE back on, see Known issues
```

從 Steam 啟動時,把它們放在啟動選項裡命令的前面,例如
`EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`。輸出會進入這次執行的日誌:從視窗
啟動時是實例的 `logs/run-*.log`,從 Steam 啟動時是 Steam 的主控台。

要看合併檢視本身,`eidos shell <instance>` 會在遊戲所見的遊戲資料夾裡開一個 shell,
`eidos shell <instance> -- ls Data/meshes` 則在那裡執行一條指令;寫入的內容都進入
Overwrite([usage.md](usage.md#合併檢視中的-shell))。Data 分頁的 Save tree 按鈕會把整個
合併後的 `Data` 樹寫成文字檔,每行一個檔案並附上它來自哪個模組,你的家目錄會被隱藏。

## 手動掛載一個聯集

`eidos-fuse` 隨 Arch 套件和從原始碼建置的版本一起提供:`just build` 會建置它,有它的時候
`just install` 或 `install.sh` 會把它複製過去。發行套件和 `eidos-linux` 不帶它。它的用法行:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

衝突時第一個 `--layer` 勝出;最後一個是你原封不動的遊戲資料。掛載只需要
`/dev/fuse` 與 `fusermount3`(不需要 overlayfs,不需要 Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

寫入會落在 `--overwrite <dir>`(省略時是一個暫存目錄),所以即使在這裡,各層本身
仍然原封不動。

## 為什麼 passthrough 預設關閉

Passthrough 把真正的後端檔案交給核心,於是讀取完全跳過這個常駐程式。下面的 A/B 是
用較早版本的常駐程式測的,早於後文已知問題「FUSE passthrough 讓遊戲載入不了任何模
組內容」所描述的變更。這個變更避開了最可能的原因,但還沒在遊戲裡重新測量過,所以在
重新測量之前 passthrough 預設維持關閉。在 Skyrim SE 1.6.1170、proton-cachyos
11.0、核心 7.1.4、同一份 82 個外掛的載入順序上做 A/B 實測,唯一的變因是執行檔有沒
有帶上那個能力:

| passthrough | `NtCreateFile` 以 `STATUS_ACCESS_VIOLATION` 失敗的次數 |
|-------------|--------------------------------------------------------|
| 開啟        | 152 - 75 個 `.bsa`、65 個 `.esl`、10 個 `.esm`、2 個 `.esp` |
| 關閉        | 0                                                      |

開著時,遊戲當時一個自己的封裝檔或外掛都打不開,在遊戲裡呈現出來就是模組不存在 -
沒有錯誤,沒有一行日誌。關掉時,同一份載入順序走到了實際遊玩,外掛、封裝檔與
Papyrus 腳本都活著。

這個失敗當時從常駐程式內部是看不見的,這正是它難找的原因:常駐程式自己的 `open` 每
次都成功,核心也從未拒絕後端檔案(以 `EIDOS_FUSE_TRACE=open` 追完一整場失敗的執行
驗證過:零筆 `open FAILED`,零筆 `passthrough refused`)。錯誤是在常駐程式回覆
`opened_passthrough` 之後才產生的,所以常駐程式這一側再怎麼記錄都看不到。它也不挑
副檔名 - 封裝檔與外掛一視同仁,也就是遊戲整場執行期間一直開著的那些檔案。

`EIDOS_FUSE_PASSTHROUGH=1` 會把它重新開啟,用來衡量它帶來什麼,或重新測試這個
機制。啟動器與 Diagnostics 分頁裡的 capability 警告,只有在你主動要求它時才會出現。

## 層索引真的有在用嗎?

`LayerStack::new` 要嘛為唯讀各層建出完整對照表,要嘛一個也不建。沒有對照表時,
每一次查詢都跟以前一樣逐層走過:答案相同,讀一次存檔大約要三倍的時間,其他地方看不
出任何差別。

索引關閉時,掛載會在啟動時說明一次原因:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

原因會指出一個無法讀取的目錄、一個超過 64 層目錄深的層、超過 4,000,000 筆項目,
或 `EIDOS_NO_INDEX`。這一行寫進這次執行的日誌:從視窗啟動時是實例的
`logs/run-*.log`,從 Steam 啟動時是 Steam 的主控台。設定 `EIDOS_FUSE_STATS=1` 後,
卸載時的輸出裡還有一行 `layer index:`,一般掛載上 `unindexed` 不為零,就表示索引
當時是關閉的。

下面的範例離線檢查同一件事,針對實例的資料夾,不需要掛載:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` 會在有索引與沒索引的情況下解析真實路徑,並比對目錄掃描的結果。
`index_agrees` 檢查兩者在一個真實實例的每一條路徑與每一次列表上,給出的是完全相同
的答案。`listing_cost` 衡量合併子項對照表在 `readdir` 上省下了什麼。

`EIDOS_NO_INDEX=1` 會強制走訪,用在兩種答案之間的差異正是你要除錯的對象時。

## 用 CLI 啟動遊戲

要透過 `eidos` CLI 而不是視窗啟動遊戲,把它的 Steam 啟動選項設成 `eidos` 的絕對路徑,
例如:

```
~/.local/bin/eidos play skyrimse -- %command%
```

不需要加上 `WINEDLLOVERRIDES` 前綴。當某個已啟用的模組匯入 `d3dcompiler_47` 時,
Eidos 會把自己隨附的那份部署進前綴、強制它以原生方式載入,並把這項設定與模組自帶的
任何 DLL override 合併(ENB/ReShade/`.asi` 載入器)。

Eidos 從不自己透過 Proton 啟動遊戲。對於會啟動遊戲的項目(script extender 的載入器、
啟動器或遊戲執行檔),`eidos tool <instance> run <title>` 會請擁有該副本的商店來啟動,
就像在沒有商店開啟過的視窗裡按 Run 一樣,並在商店的設定不會啟動 Eidos 時印出該設定什麼。
`--print` 會印出商店命令而不執行它。

## 無法關閉的遊戲或工具

遊戲或工具執行時,Run 按鈕會變成 Stop,鎖定卡片上也有一個 Stop 按鈕。按兩次。第二次按下
會結束這次執行啟動的所有程式:先送 SIGTERM,10 秒後對剩下的送 SIGKILL。遊戲沒有儲存的
內容會遺失。之後 Eidos 做的事與任何結束之後相同:INI 和外掛的變更回到設定檔,工具的輸出
被記錄,模組被卸載。不論程式回傳什麼,這次執行都以結束碼 143 記錄,所以
`eidos sessions <instance> diff` 絕不會把它當作最近一次正常執行。Unlock 不一樣:它把視窗
交還給你,程式繼續執行。在終端機裡對 `eidos` 行程執行 `kill <pid>`(pid 顯示在鎖定卡片
上)與 Stop 效果相同。

## 找出是什麼弄壞了

要找出是哪個變動弄壞了遊戲:

- `eidos sessions <instance> diff` 列出自上一次以結束碼 0 結束的遊戲執行以來變了什麼:
  模組、版本、外掛順序、INI 和遊戲執行檔
  ([usage.md](usage.md#自上次正常執行以來的變化))。
- Run 旁邊的 Test run 可以用設定檔的模組、不帶模組,或只帶選中的模組及它們所需的 master
  來玩,並把遊戲寫入的一切送到 `<instance>/test-run/`。執行之後,你對每一組檔案決定保留
  還是丟棄:存檔、INI 變更、外掛狀態、Overwrite 輸出和日誌
  ([usage.md](usage.md#測試執行))。
- Diagnostics 分頁上的 Find the mod behind a problem 每次用剩餘嫌疑對象的一半做測試執行,
  並詢問問題是否發生。200 個嫌疑對象大約要 8 次執行,而且搜尋在關閉 Eidos 後仍然保留
  ([usage.md](usage.md#找出造成問題的模組))。

被停止的執行絕不會在那個搜尋裡預先填上 Yes,因為它以結束碼 143 記錄(見上)。

## Health 卡片

Diagnostics 分頁會為它找到的每個問題顯示一張 Health 卡片,附上成因,有些還帶一個能修好
它的按鈕。啟動選項卡片在 [install.md](install.md) 裡說明,NVAPI、全螢幕和 ENB 卡片在
[graphics.md](graphics.md),Proton 和前綴卡片在下一節。`eidos check <instance>` 會印出
不需要視窗的結果([usage.md](usage.md#從終端機執行健康檢查) 列出了它們),其中有前綴、
登錄檔和 `S:` 磁碟機卡片。只有視窗會顯示的有:Proton 卡片及其 Flatpak、未知版本和較舊
版本的變體、修補程式順序、擴充器腳本、沒有外掛載入的封裝檔、Nexus 需求、過時的外掛副本
和 Display 卡片。

### 系統

- 「vm.max_map_count is <value>」在低於 1048576 時出現,Arch Linux、Fedora 39 及更新版本
  和 Ubuntu 24.04 及更新版本的預設值就是它。Proton 下的某些遊戲會映射更多記憶體區域,然後
  不報錯地當機。把 `vm.max_map_count = 1048576` 放進 `/etc/sysctl.d` 底下的檔案,例如
  `80-gamecompatibility.conf`,然後執行 `sudo sysctl --system`。
- 「The open files limit is <value>」在硬性上限低於 524288 時出現,Proton 的 esync 需要這個
  值;低於它時遊戲可能無法啟動或當機。systemd 240 及更新版本會設定它。較舊的環境請在
  `/etc/systemd/system.conf` 和 `/etc/systemd/user.conf` 裡設定
  `DefaultLimitNOFILE=524288`,然後重新登入。
- 「Game files on a Windows file system」在遊戲資料夾、前綴或實例根目錄位於 `ntfs`、
  `ntfs3`、`fuseblk`(ntfs-3g、exfat-fuse)、`exfat` 或 `vfat` 上時,會指出是哪一個。
  Proton 需要符號連結、Unix 權限和區分大小寫的名稱。請把遊戲、它的前綴和實例搬到
  ext4、btrfs 或 xfs 這類 Linux 檔案系統上。

### 遊戲檔案

分頁頂端的 Game files 卡片顯示實例執行的是商店安裝,還是實例內的遊戲副本
([usage.md](usage.md#遊戲檔案商店安裝還是副本))。在三種情況下啟動會被拒絕,並附一張說明
原因的卡片:

- 「The game copy '<folder>' is missing」:實例執行的副本已經不見了。把它放回來,或回到
  商店版本。
- 「'<folder>' has no <binary>」:實例所執行的資料夾裡沒有遊戲執行檔,所以它不是遊戲的副本。
- 「<game> cannot run from a copy yet」:Morrowind,以及 `bUseMyGamesDirectory=0` 的
  Oblivion,把外掛清單放在遊戲資料夾裡。

當清單自帶的、不是 Eidos 寫的 `ModOrganizer.ini` 有一個 `gamePath`,不同於實例所執行的遊戲
檔案時,會出現「ModOrganizer.ini points tools at another game folder」。以 Mod Organizer 2
模式執行的工具(例如 PGPatcher)會從那裡讀遊戲。請修改該檔案裡的 `gamePath`,或移除該檔案,
讓 Eidos 寫自己的。

### 載入順序

- 「sTestFile lines make the game ignore plugins.txt」出現在 Fallout 4、Fallout 4 VR 和
  Starfield,當設定檔的 `Fallout4Custom.ini` 或 `StarfieldCustom.ini` 在 `[General]` 裡有
  `sTestFile1` 到 `sTestFile10` 行時。只要設了其中一個,遊戲就會載入那些外掛,Eidos 寫出的
  載入順序就沒有作用。Fix 會從設定檔的副本移除這些行,並把先前的檔案保留為
  `<ini>.pre-fix`。
- Starfield:沒有 master 旗標的已啟用 `.esp` 外掛會有一張卡片,因為 Starfield 無法把它們排在
  `.esm` 和帶 master 旗標的外掛之間,並且整個工作階段都讓它們的部分記錄保持載入。作者提供
  `.esm` 版本可以避免這兩個問題。
- Starfield:「Invalid blueprint plugins」列出具有部分但並非全部 blueprint 旗標、名稱以
  `BlueprintShips-` 開頭且副檔名為 `.esm` 的外掛。「Blueprint plugins without their main
  plugin」列出主外掛未啟用的那些:`BlueprintShips-Example.esm` 需要 `Example.esm`、
  `.esp` 或 `.esl`。
- 「<patch> loses <n> file(s) to <master>」出現在某個模組裡的外掛需要另一個模組的 master,而
  那個模組優先順序更高,所以兩者都帶的檔案是它的副本勝出。修補通常放在它所修補的模組之下,
  卡片上的 Move 按鈕就會把它放到那裡。如果該修補本來就要輸掉這些檔案,請保持現有順序。
- 「Out-of-date copy of <plugin>」出現在某個模組持有某外掛的已清理或已加輕量旗標的副本,
  而原始外掛後來改變了(模組或遊戲更新)。遊戲載入的是舊副本。請隱藏它,然後重新清理或
  標記新的外掛。

### 模組清單

- 「Order rules of the collection <name> that no longer hold」出現在已安裝合集的模組不再處於
  其規則要求的順序時。Restore collection order 只把該合集的模組移回去,Ctrl+Z 可以還原。
- 「<mod> requires <mod>, which is not installed」(或「which is disabled」)來自 Check for
  updates 讀取的 Nexus 需求。Open on Nexus 前往所需模組的頁面,Download main file 在付費
  帳號上下載它,Ignore 則隱藏另一個模組已滿足的需求。DLC 需求只是建議:Eidos 會找與該 DLC
  同名的外掛。
- 「fontconfig.txt names a font library that is missing」(Skyrim 引擎)會指出沒有任何層和任何
  封裝檔持有的字型,以及其 `Interface/fontconfig.txt` 勝出的模組。在安裝該字型或移除該行
  之前,選單文字可能顯示成方框。
- 「BA2 archive stores audio compressed」(Fallout 4 和 Fallout 4 VR)統計某模組的 BA2 以
  壓縮方式存放的 `.wav` 和 `.xwm` 檔案數。遊戲播放它們時可能當機。請用不壓縮音訊的方式
  重新封裝。

### script extender 與遊戲更新

- 「<loader> is not installed」出現在已安裝 script extender 外掛(在 `SKSE/Plugins`、
  `F4SE/Plugins` 等處),但載入器不在任何已啟用的模組裡,也不在遊戲資料夾裡時。遊戲會在
  沒有它們的情況下啟動。
- 「The script extender does not match game version <version>」出現在已安裝載入器,但缺少與
  執行中遊戲版本對應的執行階段 DLL 時,例如 `skse64_1_6_1170.dll`。載入器會報錯停止。這
  通常發生在遊戲更新之後。這張卡片和上面那張都有指向擴充器網站的 Open the download page
  按鈕,以及把載入器放進該模組 `Root/` 資料夾的 Install the downloaded archive 按鈕。
- 「<game> changed since the last clean launch」出現在遊戲執行檔(Steam 副本則是它的 build id)
  自上一次經由 Eidos 啟動且未報錯結束以來改變時。Steam 更新若沒有改動執行檔則不會觸發它。
  為舊版本建置的 script extender 外掛可能不再載入,它下面的卡片會指出宣告了這一點的外掛。
  想保住清單建置時所用的遊戲版本,可以用 Game files 卡片上的 Freeze this version 把它凍結進
  實例,或在 `freeze` 單獨顯示大小之後用 `eidos stock-game <instance> freeze --yes`:之後
  商店更新改變的是商店安裝,不是副本。
- 「<mod> replaces <n> <extender> script(s)」出現在優先順序高於 script extender 的某個模組帶有
  該擴充器自身腳本的散裝 `.pex` 副本時。舊副本會破壞它所宣告的函式,而遊戲不會顯示任何
  錯誤。把擴充器移到那個模組之下(兩者都是模組時卡片的 Move 按鈕能做到),或在該模組裡隱藏
  那些檔案。當擴充器的腳本散放在遊戲的 `Data` 裡時,請把擴充器安裝為模組,這樣才能排序。
  只檢查散裝檔案,不檢查封裝檔。

### 工具輸出

- 「Overwrite/Root holds meshes」(或 textures)表示某個資料路徑指向遊戲資料夾的工具把這些
  資料夾寫到了那裡,而遊戲從不讀取那裡。請修正工具的路徑,刪掉這些資料夾,再執行一次工具。
  Open folder 按鈕會開啟 `Overwrite/Root`。
- 「BodySlide's GameDataPath is not the game's Data folder」(或 `OutputDataPath`):見
  [工具](tools.md#工具自己的設定仍然是它自己的)。

### 上一次工作階段中的當機

「The game crashed in the last session」出現在崩潰記錄器在上次啟動時寫了一份記錄時。卡片會
指出例外,以及記錄中由你的模組或 Overwrite 安裝的 DLL 和外掛。請把它們當作線索:崩潰記錄
裡點名的模組,往往只是當機出現的地方,而不是原因。當記錄沒有指名你的模組中的任何檔案時,
卡片會這麼說。Open the log 開啟記錄,Copy the log (home path hidden) 則在隱藏你的家目錄後
複製它。

### 視窗大小

對於 Skyrim SE、Enderal SE 和 Fallout 4,Display 卡片顯示設定檔啟動遊戲所用的視窗大小與
模式,當設定檔的 Prefs INI 和前綴都沒有設定時,會寫「No window size set for this profile」。
Apply 把大小與模式寫進設定檔的副本,而當由 SSE Display Tweaks 設定視窗時,也寫進 Overwrite
中它的 INI 的一份副本([usage.md](usage.md#診斷))。

## 遊戲由哪個 Proton 執行,以及損壞的前綴

Diagnostics 分頁有一張 Proton 卡片:Eidos 和 Steam 為該遊戲使用的版本、Steam 自己的設定,
以及最後執行該前綴的版本。如果最後準備前綴的 Proton 比所選的更新,卡片會變成建議,因為在
較新的前綴上用較舊的 Proton 可能會重設其中的 Wine 檔案,或者無法啟動。請在 Steam 中遊戲的
內容、相容性裡選擇一個至少同樣新的版本,或重建前綴。

其他 Proton 與前綴卡片:

- 「Proton comes from the Flatpak Steam install」:那個 Proton 把它的執行階段和 steamclient
  程式庫放在 Flatpak 沙箱裡,所以從主機執行它可能失敗。Eidos 不會透過 `flatpak run` 啟動
  它,那會把遊戲放進沙箱,放到合併檢視之外。請在 `~/.steam/root/compatibilitytools.d/`
  裡安裝一個 Proton,並為遊戲選用它。
- 「Eidos cannot tell which Proton runs this game」:Steam 的 `config.vdf` 沒有為它指定相容性
  工具,而前綴也沒有記錄任何已安裝的版本。在 Steam 裡選好一個之前(遊戲的內容、相容性),
  工具不會啟動。
- 「<game> has no Proton prefix yet」:先透過 Steam 啟動遊戲一次,然後按 F5。如果 Steam 沒有
  建立前綴,請檢查 `steamapps/compatdata` 底下該遊戲的資料夾是否可寫;一個由 root 擁有的空
  資料夾會不出聲地擋住它。
- 「Windows tools cannot find <game>」:前綴中安裝路徑的記錄有誤或缺失,而 xEdit、DynDOLOD、
  Wrye Bash 和 Creation Kit 會讀它。Eidos 會在你下次執行工具時寫入正確的路徑。
- 「The prefix's S: drive points outside the Steam library」:`S:` 既不符合函式庫根目錄(Eidos
  為工具指向的位置),也不符合 `<library>/steamapps`(Steam 為遊戲指向的位置)。函式庫可能
  搬過了,或前綴來自另一台機器。前綴裡存的每一個以 `S:` 為基準的路徑,在下一次經由 Eidos
  或 Steam 啟動、再次指向它之前,都解析不到任何地方。
- 「<store> installation: started through its store」是 GOG 或 Epic 副本的卡片。當 Eidos 是
  遊戲在 Heroic 或 Legendary 中的包裝器時,Run 會請它們啟動遊戲,而工具使用 Heroic 為該
  副本記錄的 Wine 或 Proton 版本執行。

File 選單會在遊戲的前綴裡執行 winecfg、regedit、Wine 檔案總管和解除安裝程式,使用執行該
遊戲工具的 Proton([tools.md](tools.md#wine-自己的程式在遊戲前綴裡))。在前綴有這些程式之前,
它們那幾列保持灰色。

Proton 卡片上的 Rebuild prefix 按鈕用於已經無法使用的前綴。按兩次。Eidos 不刪除任何東西:
它在同一資料夾裡把 `steamapps/compatdata/<appid>` 重新命名為
`<appid>.eidos-backup-<date>-<time>`,下次你從 Steam 啟動遊戲時,Steam 會建立一個新前綴。
只要有遊戲、工具或任何 Wine 程式在使用該前綴,它就會拒絕。在新前綴存在之前,視窗裡的 Run
無法把遊戲交給 Steam,會改為顯示啟動選項,所以請在 Steam 裡按 Play。只存在舊前綴裡的存檔
和設定會留在重新命名後的資料夾中:把需要的複製回來,不再需要時請自己刪除那個資料夾。

## 回報錯誤時要附上什麼

- Diagnostics 分頁上的 Copy report 會複製 Eidos 版本、遊戲、Proton 版本、Steam 是否為
  Flatpak、passthrough 狀態、工作階段類型、發行版、核心、`vm.max_map_count`、開啟檔案數
  上限、`/dev/fuse` 和 `fusermount3` 是否存在、設定檔啟動變數和包裝程式的名稱,以及每一張
  Health 卡片。你的家目錄顯示為 `~`,登入名稱顯示為 `<user>`。`eidos check <instance>` 在
  終端機裡印出相同的標頭,`--json` 則以 JSON 給出。
- 執行日誌 `<instance>/logs/run-*.log` 以 Eidos 執行的確切命令開頭。File, Eidos logs 會開啟
  Eidos 自己日誌的資料夾,`~/.local/share/Colony/Eidos/logs`。
- 當機卡片上的 Copy the log (home path hidden) 會複製崩潰記錄。
- Data 分頁上的 Save tree 會儲存合併後的 `Data` 樹,並附上每個檔案的來源。

## 已知問題

### DLSS 或影格生成靜悄悄地毫無作用

三種各自獨立的成因,每一種都沒有任何錯誤訊息:啟動選項裡沒有啟用 NVAPI、獨佔
全螢幕,或是一個過時的 Reflex FPS 上限。完整的檢查清單在
[graphics.md](graphics.md)。

### Flatpak 版 Steam 下 Run 沒有反應

在沒有商店開啟過的視窗裡,Run 透過執行 `steam -applaunch <appid>` 請 Steam 啟動遊戲。
Flatpak 版 Steam 不會在主機的 `PATH` 上放 `steam`,所以狀態列會顯示「Could not start
Steam」和錯誤訊息。請改在 Steam 裡按 Play。

### 來自另一台裝置的存檔沒有出現

不經 Eidos 就到達前綴的存檔,例如從 Steam Deck 下載的 Steam Cloud 存檔,在設定檔的存檔被
掛載期間會保持隱藏。沒有東西刪除它。Saves 分頁在標題下統計這些存檔,Import 會把它們複製
進設定檔,而不會取代設定檔裡的存檔([usage.md](usage.md#存檔屬於設定檔))。

### 一個把同一個目錄寫成兩種拼法的模組,弄丟了第二種拼法底下的一切

已修正。ext4
把 `meshes/` 與 `Meshes/` 當成兩回事;合併檢視不能這樣,而真實的模組兩種都會出貨 -
XP32 Maximum Skeleton 的動畫與 FNIS 行為檔在大寫的那個底下,`character assets` 在
另一個底下。

解析器對每一個路徑元件都取大小寫完全相符的那個,然後就此定案:它進了 `meshes/`,
在裡面找不到路徑剩下的部分,於是放棄了整個層。另一種拼法底下的每一個檔案對遊戲來說
都不存在 - 沒有錯誤,沒有日誌,任何診斷裡都沒有。在一個真實的 50 層實例上,那是
74 個檔案。

現在一個相符的元件只是候選,不是定案;大小寫完全相符的仍然優先嘗試,只有當剩下的
部分在它底下失敗時,掃描才會去找摺疊後相等的同層目錄。列表在高一層的目錄上有同樣的
毛病,現在每一層裡摺疊後相等的目錄都會讀。

值得知道它的形狀:路徑索引從來沒有這個 bug,因為它會走訪自己找到的每一個目錄。它
一直默默回傳後備路徑拿不到的檔案,而這是反過來的 - 後備路徑才是那個應該永遠不會錯
的答案。

### DynDOLOD 的 LODGen 死掉,只留下一份空的日誌

已由 `dotnet10` 修正;見 [tools.md](tools.md)。每一個世界的 `LODGen_SSE_<world>_log.txt`
裡只有一段版本橫幅、一行 `.NET Version:`,再無其他,而對話框只說「failed to generate
object LOD for one or more worlds」。成因是 Wine 的 Mono 代替 .NET Framework 回應,而且
裝再多次 .NET Framework 都修不好 - 每次前綴更新,Proton 都會把 `mscoree.dll` 換成一個指向
它自己樹裡的 symlink。發行套件附帶 .NET 10 執行階段;其他安裝方式會下載一次。

### Wine 分辨不出這個掛載會摺疊大小寫

已修正。

Wine 的 `get_dir_case_sensitivity` 會先用 `FS_IOC_GETFLAGS` 詢問目錄,再去嗅
CIOPFS 留在它所服務目錄裡的那個標記。兩者都沒有,Wine 就假設大小寫敏感,而每一次拼
法沒有逐位元組相符的查找,都會退回去讀整個目錄來找不分大小寫的相符項。Bethesda 的
遊戲要的是 `data/ccbgssse001-fish.bsa`,而檔案叫 `ccBGSSSE001-Fish.bsa`,於是它幾
乎在每一個素材上都被觸發:八秒內 4471 次標記探測與 2236 次整個目錄重讀,九十秒內
195796 次對 `Data` 的列舉。Skyrim SE 從來沒走到主選單 - 它停在 240 MB 常駐記憶體,
而常駐程式燒掉了一顆核心的 92%。

Eidos 從一開始就在 `resolve_read` 裡摺疊大小寫。這整份成本只是因為從來沒說出來。
現在 `lookup` 會回應 `.ciopfs`;`readdir` 仍然不把它列出來。`ioctl` 也會對目錄回
應 `FS_IOC_GETFLAGS` 並帶上 casefold 旗標,目前的 Wine 會在尋找標記之前先問這個。

有兩件事讓它從單純的慢變成致命。成本隨目錄大小增長,所以裝上 Anniversary 內容
(`Data` 從 37 個檔案變成 177 個)就把它推過了頭。而且 `opendir` 會急切地建出合併
後的列表,當 Wine 打開一個目錄只是為了 `stat` 裡面那個標記時,那完全是浪費 - 現在
快照是在第一次 `readdir` 時才拍。

之後:主選單、2.1 GB 常駐記憶體、常駐程式 0% CPU。

找出它的是 `EIDOS_FUSE_TRACE=opendir`,而且它有出貨。操作計數器只說有多少次;一個
目錄被列舉 195796 次,在總數裡是看不出來的。

### 遊戲把 `plugins.txt` 重寫成空的

很可能和上面的大小寫摺疊是同一回事 - 一個它在任何合理時間內都
列舉不完的 `Data`,於是它認定那裡什麼都沒有,並把這個結論存了下來。尚未證實,值得
重新查一次。無論如何,擷取防護(任何規模下,會把啟用集合整個清空的擷取一律拒絕)
意味著它再也弄不壞設定檔了。

### `FOPEN_KEEP_CACHE` 是關的

已修正,而且值得知道原因。它會在主選單之後幾秒讓
Skyrim SE 因空指標解參考而當機,可穩定重現,而且一個模組都沒裝;另外三個核心側的快
取被逐一二分排除,只有這一個有影響。那次二分是在 `FUSE_PASSTHROUGH` 開著的情況下
做的,而核心會讓帶有 `FOPEN_KEEP_CACHE` 的 passthrough 開啟以 EIO 失敗,所以開著這
個旗標時,每一次 passthrough 開啟都失敗了。讓遊戲當機的或許正是這些失敗的開啟,而
在 passthrough 關閉時單獨測試這個旗標,從來沒做過。Passthrough 回覆現在不再帶它。
當時測出來失去它是零成本,但 passthrough 開著時常駐程式服務的讀取是零次
(`EIDOS_FUSE_STATS` 在一次完整載入中回報 `read 0`),所以在 passthrough 預設關閉的
現在(見下),真正的成本未經測量。要單獨測試的話用 `EIDOS_FUSE_KEEP_CACHE=1` 重新開
啟。

### FUSE passthrough 讓遊戲載入不了任何模組內容

常駐程式現在避開了最可能的原因(見下),但這還沒在遊戲裡重新測量過,所以 passthrough
預設維持關閉;`EIDOS_FUSE_PASSTHROUGH=1` 可以開啟它。這次變更之前,在核心 7.1.4 上,
passthrough 開著時,Skyrim SE 有 152 個自己的檔案(75 個 `.bsa`、65 個 `.esl`、10
個 `.esm`、2 個 `.esp`)以 `STATUS_ACCESS_VIOLATION` 開啟失敗,關掉時是 0 個 - 於
是沒有任何模組內容會載入,而且悄無聲息。核心是在常駐程式回覆 `opened_passthrough`
之後才拋出這個錯誤,所以常駐程式自己的日誌顯示的是一場乾淨的執行(零次開啟失敗,零
次後端檔案被拒)。

核心為每個 inode 只綁定一個後端檔案(`fs/fuse/iomode.c`)。如果一個 inode 已經有後
端檔案,再帶來第二個後端檔案 ID 的開啟會以 EIO 失敗;對已有後端檔案的 inode 做一般
開啟(不帶 `FOPEN_PASSTHROUGH`)也一樣。帶有 `FOPEN_KEEP_CACHE` 的 passthrough 開
啟同樣以 EIO 失敗。以前的常駐程式每次開啟都註冊一個新的 ID,所以遊戲已經開著的檔
案被第二次開啟時就會失敗。Wine 把 EIO 回報為 `STATUS_ACCESS_VIOLATION`,這正是那
152 次開啟失敗背後的錯誤。這條規則是從核心原始碼讀出來的,而且與那 152 次失敗吻合,
所以它是最可能的原因。

現在常駐程式每個 inode 只註冊一個後端檔案,由該 inode 的所有 passthrough 開啟共用,
而且從不在 passthrough 開啟上設定 `FOPEN_KEEP_CACHE`。Passthrough 只用於遊戲層和
模組層中大小達到 1 MiB 或以 `.dll`、`.exe`、`.asi` 結尾的檔案的唯讀開啟。這個後
端檔案一旦註冊,該檔案的每一次唯讀開啟都使用它;只要該檔案還有一般開啟,新的讀取者
拿到的也是一般開啟。Overwrite 裡的檔案從不走 passthrough,投射外掛時間戳記的掛載
則完全不使用它。當 passthrough 讀取者還占著一個檔案時,以寫入方式開啟它會在任何
copy-up 之前得到 ETXTBSY(在 Wine 中是 `STATUS_SHARING_VIOLATION`),因此 `O_TRUNC`
開啟不會在 Overwrite 裡留下一個遮住模組檔案的空副本。

### Mod Manager Download 按鈕沒有反應

Preferences, Nexus, Downloads 會顯示由哪個程式接收 `nxm://` 連結。如果該程式不是 Eidos、
已不存在或尚未設定,會出現一張 Health 卡片。Register 按鈕會寫入 `eidos-nxm.desktop`,並將它
設為 `x-scheme-handler/nxm` 的預設處理程式。在終端機中,`eidos nxm --register` 的效果相同。

如果狀態已顯示 Eidos,按鈕仍然沒有反應,可能是瀏覽器沿用了之前的選擇。請在瀏覽器設定中清除
nxm 連結的已儲存選擇,讓它重新詢問。Flatpak 版瀏覽器無法直接啟動主機上的程式,而是透過桌面
入口轉交連結,入口讀取的是在主機上註冊的處理程式。
