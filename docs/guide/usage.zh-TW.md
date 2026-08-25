<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# 使用 Eidos

實務手冊:CLI、GUI、Steam 啟動選項、從原始碼建置,以及概念驗證腳本。
東西看起來不對勁時該怎麼辦,見 [troubleshooting.zh-TW.md](troubleshooting.zh-TW.md)。

## 使用它(CLI)

```sh
eidos games                       # supported games installed here (like MO2's list)
eidos init skyrimse               # create a modding instance
# ...drop each mod as a folder into <instance>/mods/ (the global instance lives
#    at ~/.local/share/eidos/skyrimse; `eidos init` prints yours)...
eidos install skyrimse mod.7z     # or install a downloaded archive (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adopt an existing MO2 profile's order + plugin state
eidos sort skyrimse               # LOOT-sort the plugin load order
eidos play skyrimse               # show what would be mounted
eidos play skyrimse -- <command>  # run <command> with the mods mounted over the game
```

`eidos tool`、`eidos prereqs`、`eidos nexus`、`eidos nxm` 與 `eidos export` 把整套
補齊;不帶參數執行 `eidos` 就會列出完整清單。

### 實例:全域與可攜

上面每一個命令都指向某個實例。`skyrimse` 指的是**全域**的那一個 - 集中存放在
`~/.local/share/eidos/skyrimse`,由 Eidos 管理。另一種是**可攜**的:一個自我完備
的資料夾,放在你想要的任何地方(第二顆硬碟、遊戲分割區),可移動、彼此隔離,
和 MO2 的可攜實例完全一樣。凡是接受遊戲 id 的命令,同樣接受可攜實例的資料夾:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # create a portable instance there
eidos install /mnt/games/EidosSkyrim mod.7z  # every command accepts the folder
eidos play /mnt/games/EidosSkyrim -- %command%
```

那個資料夾會自我描述(它的 `eidos-instance.ini` 指明了遊戲),所以不需要別的東西 -
而環境中的 `EIDOS_INSTANCE=<folder>` 會把一個遊戲 id 重新導向到該資料夾,這在
Steam 啟動選項裡很方便。你建立過或開啟過的可攜實例會被記在
`~/.config/Colony/Eidos/instances.ini`(最近使用的在前);GUI 的歡迎畫面會列出
它們,一鍵開啟,Steam 啟動會落在你上次玩的那一個,`nxm://` 處理器也會下載進去。
有兩點但書值得知道:移動可攜資料夾後,除了你以舊位置的絕對路徑註冊的工具項目之外
一切都保留(那些要重新加),而共用的執行階段快取
(`~/.local/share/Colony/Eidos/runtimes/`)刻意維持在整台機器共用 - 一個 78 MB 的
.NET host 不該每個實例一份。

Eidos 把自己的檔案放在 `Colony/Eidos` 底下,那是 Colony 家族每個程式都用的配置:
`~/.config/Colony/Eidos/` 放你選的東西(偏好設定、你的 Nexus 工作階段、你的實例
清單、你自己寫的遊戲與附加元件定義),`~/.local/state/Colony/Eidos/logs/` 放工作
階段日誌,`~/.local/share/Colony/Eidos/` 放 Eidos 下載的東西。舊版 Eidos 把這些
放在 `~/.config/eidos/` 與 `~/.local/state/eidos/`;升級後的第一次啟動會把它們
**複製**過來,並在日誌裡說明。舊目錄會原封不動留著 - 什麼都不刪,所以一次糟糕的
升級不會讓你賠掉登入狀態 - 等你確認沒問題,可以自己移除。

你的模組不屬於那一部分。全域實例仍然在 `~/.local/share/eidos/<game>/`,可攜實例則
在你放的地方,因為那些路徑寫進了你的實例清單,也可能寫進了 Steam 啟動選項:移動
它們會弄斷一條 Eidos 並不同時擁有兩端的連結。

有一個位置會被直接拒絕:**遊戲安裝資料夾裡面**(MO2 老手的反射動作)。那棵樹歸
Steam 管 - 一次更新、一次「驗證檔案完整性」或一次解除安裝,都可能改寫或刪除它,
連你整套設定一起帶走 - 而且 Eidos 掛載在遊戲根目錄之上,所以放在那裡的實例會坐在
自己的掛載目標裡面。精靈、`eidos init` 與 `eidos play` 都會說不;請把資料夾放在
遊戲「旁邊」(同一顆硬碟上的同層目錄同樣方便)。

`play` 會在一個私有命名空間裡,把實例的模組掛載到遊戲自己的 `Data` 目錄之上
(透過 bind-stash,所以常駐程式仍然讀得到原封不動的檔案),然後讓命令穿過那個
檢視執行。寫入(存檔、重新產生的設定)落在實例的 `overwrite/` 層;遊戲安裝目錄與
每一份模組來源都逐位元組保持原封不動。

### 不需要任何特權步驟

Eidos 完全不需要 root 就能執行。它掛載在私有的 user + mount 命名空間裡,所以沒有
setuid 輔助程式、沒有常駐程式,也沒有什麼權限要授予。

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 是**選用的**,而且只管一件事:
核心 FUSE passthrough,它預設關閉,因為它會弄壞遊戲(見下)。帶著這個 capability
時,Eidos 取用的是單純的 mount 命名空間而不是 user 命名空間;兩種方式部署模組的
結果一模一樣。


舊的 `setcap` 建議為什麼消失了 - 以及 FUSE passthrough 為什麼出貨時就關著 - 在
[troubleshooting.zh-TW.md](troubleshooting.zh-TW.md#為什麼-passthrough-預設關閉)
有說明。

## GUI

```sh
cargo run -p eidos-gui
```

一個 MO2 風格的首次啟動精靈,採用 Colony 的羊皮紙 / 酒紅配色:歡迎 -> 實例類型
(可攜 / 全域)-> 遊戲 -> 名稱與位置 -> 摘要 -> 建立 -> 主畫面。歡迎畫面也會列出
每一個已知的既有實例(全域與可攜,最近使用的在前),一鍵開啟 - 它兼任實例切換器 -
而把精靈指向一個已經放著實例的資料夾,會「原樣沿用」它,而不是覆蓋著建立
(若該資料夾屬於另一款遊戲,就直接拒絕)。

雙欄主視窗也已經做好:一個設定檔選擇器(切換,或複製目前這份建立新的)、一份可以
篩選、選取、重新排序、用分隔線分組、依類別縮小範圍並右鍵操作的模組清單,加上
Data / Plugins / Conflicts / Overwrite / Saves / Downloads / Diagnostics 分頁,以及
一個帶執行目標選擇器的 Run 按鈕。

重新排序不只有移到最上/最下:MO2 那些指定目標的移動也都在 - 移到第一個衝突模組
之上、最後一個之下、移到指定的優先順序,或移進某個分隔線的群組。它們全都走同一個
共用的移動輔助函式,所以「先移除列、再重新插入」造成的 off-by-one 只存在一個
地方,而不是五個。

### 欄位、排序與分組

清單開箱畫出四個欄位,總共提供八個:Category、Content、Version、Author、
Installed、Nexus id、Game、Flags。在 View 選單裡勾選它們。預設不是八個全開,這是
刻意的 - 每個欄位都顯示的清單,就沒有空間留給「名稱」了,而那正是你真正在讀的
那一欄。

點任一欄標題就依它排序。再點一次反向,第三次點回到**載入順序**,這件事比聽起來
重要:載入順序是清單唯一能拖曳的順序,因為插入間隙定位的是真實的清單,而排序後的
一列根本在別的地方。排序開著的時候,不會畫出插入條,拖曳會被拒絕,而不是落在沒有
人瞄準的地方 - MO2 也是這麼做,理由相同。View 選單會說明這一點,並提供回去的
辦法。

View 選單也可以把整份清單**分組**,依類別或依來源(來自 Nexus,或手動安裝)。群組
標頭不是分隔線:它們背後沒有東西可以改名、上色或移動,它們會摺疊,摺疊時計數留在
標頭上。套上排序或分組時,分隔線就從清單裡消失 - 分隔線帶領的是載入順序中跟在它
後面的那些列,而這兩者都把那些列移走了。

### 滑鼠與鍵盤

雙擊模組開啟 Information,Ctrl+雙擊開它的資料夾,Shift+雙擊開它的 Nexus 頁面。
Ctrl+F 把游標放進篩選框。按下一個字母會跳到下一個以它開頭的模組,再按一次會繼續
走完其餘的,而不是卡在第一個。它們都不會落在被篩選、被摺疊的分隔線或被摺疊的群組
藏起來的列上 - 把一個你看不見的反白移過去,正是下一次按 Space 會切換到一個你根本
沒在看的模組的原因。

分隔線選單上的「Collapse others」會把除了那一個以外的每個群組都摺起來。拖曳過程
中,停在一個摺疊的群組上會把它展開,所以模組可以直接丟進去,不必先放棄這次拖曳 -
是停住,不是掠過。

### 清單會告訴你關於一個模組的什麼事

兩個提示性旗標,都是一個字符,滑鼠停留就有說明。**No valid game data** 表示模組
最上層沒有任何東西看起來像這個遊戲會載入的內容;它可能需要把資料夾往上移一層,也
可能根本不是這個遊戲的模組。**Another game** 表示模組自己的 `meta.ini` 指名了另一
款遊戲。兩者都不會擋住任何事 - 模組照樣部署 - 而列選單上的「Mark as valid」可以
讓其中任一個閉嘴,靠的是 MO2 自己的 `validated=` 鍵,所以你在一個管理器裡背書過的
模組,到另一個裡也是安靜的。

這個版面檢查刻意放寬:一棵 `Root/` 樹算數,一個讀不到的資料夾算數,一個空的也
算數。在一份五百列的清單上,一個錯誤的警告比一個漏掉的警告更糟。

### 動它之前先備份一個模組

「Back up this mod」會把它的資料夾另存一份為 `<name>_backup`(接著是 `_backup2`,
依此類推 - 備份永遠不會取代上一份)。這份複本是**惰性的**:它不是模組,它的核取
方塊什麼也不做,對合併檢視也毫無貢獻,因為勾選它等於把同一個模組的兩份複本疊著
部署。「Restore this backup over the mod」會把它放回去,兩次點擊;目前的內容會先被
移到一旁,等複製成功之後才丟棄。

**Data** 是合併檢視的一棵真正的樹,一次展開一層,所以打開一個節點的成本,是每個
擁有它的層各讀一次目錄,而不是遞迴走過每一個啟用的模組。回答它的是掛載本身服務所
用的「同一套」層堆疊,所以 whiteout 與隱藏檔案都被遵守,這個分頁不可能和遊戲將會
看到的東西說法不一。可以依名稱篩選、縮小到只看有爭議的檔案、用 Size 與 Modified
欄位理清什麼在哪裡,還能把任一列在檔案管理器裡 Reveal 出來。**Plugins** 是
ESP/ESM/ESL 的載入順序(切換、手動重新排序,或用 LOOT 排序並閱讀排序後的報告,
報告裡的建議連結會在你的瀏覽器開啟)。**Conflicts** 說明每個檔案的贏家與輸家。
**Overwrite** 一步就把遊戲寫出來的東西變成真正的模組。**Saves** 解析每份存檔的
標頭 - 角色、等級、位置、遊玩時間 - 並把烘進存檔的外掛清單和你目前的比對,還附一個
按鈕啟用它需要的模組,因為只點名它們然後丟給你自己處理,是無聊的那一半。

「Information...」會開啟每個模組專屬的對話框:general、conflicts、filetree、
INI tweaks、notes。在 filetree 裡(以及在 Data 樹裡),任何檔案都可以被**隱藏** -
改名為 `<name>.mohidden`,這會讓它從虛擬檢視中消失而不刪掉它,所以某個模組多出來
的三個 mesh 可以被壓下去,而不必動到優先順序。filetree 也做一般的檔案操作:新
資料夾、改名、刪除、開啟。它們全都經過同一個解析器,凡不是那個模組內部的單純路徑
都會被拒絕 - 不能有 `..`、不能是絕對路徑,也不能有任何一段是 symlink,因為跟著它
走會把一次刪除帶到模組資料夾之外。改名只替換最後一段,所以它永遠不會變成移動,而
遇到已被占用的名稱它會拒絕,而不是靜悄悄地覆蓋那個檔案。刪除要兩次點擊;它是這裡
唯一一個再點一次也還原不了的動作。

在 filetree 或 Data 樹的任一列上按 **View** 會預覽該檔案:圖片與文字。DDS 或 NIF
不行 - 那需要一個區塊解碼器,以及一個這棵樹沒有的算繪器 - 但它們會直說,而不是給你
一個空盒子,並指向 Reveal。文字最多讀到 64 KB,並會說明它在哪裡停下,因為預覽是
一瞥,而一份 Papyrus 日誌可以有上百 MB。**INI Tweaks** 列出模組放在自己
`INI Tweaks/` 資料夾裡的片段;啟用的那些會在啟動時按優先順序合併進設定檔的遊戲
INI,並在擷取這次執行的 INI 時再拿掉 - 否則一個 tweak 會靜悄悄變成一項設定,停用它
也不會有任何效果。

一個下載項目可以**從 Downloads 清單拖到模組清單的某個位置**,以那個優先順序安裝;
從檔案管理器拖進視窗的壓縮檔或資料夾同樣會安裝(後半這件事需要 X11 或 XWayland
工作階段 - winit 只為 X11 實作了檔案拖放)。下載本身可以暫停和續傳:暫停會停下
傳輸並保留已下載的部分,Resume 會重新解析一條新的連結,從停下的地方繼續。

Downloads 分頁是一座壓縮檔**圖書館**,不是傳輸佇列。可以依名稱篩選(也包括好記的
模組名稱,所以「skyui」找得到 `SkyUI_5_2_SE-12604-5-2SE.7z`)、依最新、名稱、大小
或狀態排序,還能把你用完的壓縮檔**隱藏**起來 - 那會保留檔案,只是把那一列拿掉,
所以把一本書收起來不等於把它燒掉。「Show hidden」會把它們帶回來,同一個按鈕也負責
取消隱藏。「Remove N installed」會用兩次點擊刪掉你已經安裝過的模組的壓縮檔,而且
只刪**畫面上**的那些:篩選就是你說明自己指的是哪些的方式。

### Nexus 合集

貼上一條合集連結 - 或在網站上點一條 - Eidos 就會列出該修訂版的成員,每一個都和這個
實例對照過:已安裝、已下載,或缺少。它**讀取**一份合集;它不安裝合集,面板上也這麼
寫著。有四件事讓安裝器在這裡不只是難做,而是不誠實:成員是普通的 Nexus 檔案,需要
每個檔案專屬的金鑰,而在網站自己的按鈕之外,只有 premium 帳號鑄得出來;一次完整
安裝是每個成員三次 API 呼叫,而這個用戶端拒絕超支這份預算;manifest 的階段、規則
與重播的 FOMOD 答案,無法對照一份真正發佈過的 Bethesda 合集來驗證,而用猜的會產出
一份看起來對、實際上不對的載入順序。讀取只花一次請求,而且是精確的。

一份合集只能對照**它自己的遊戲**來讀取。載入 Fallout 4 實例時開啟一份 Skyrim 合集,
它會指名拒絕,而不是把成員和錯誤的模組清單對照 - 在那份清單上,每一個「已安裝」和
每一個「缺少」都只是披著答案外形的雜訊。

### 離線模式

**Settings -> Nexus -> Offline** 會讓 Eidos 完全不去接觸 Nexus。更新檢查、登入、
下載與合集都會直說,而不是以連線錯誤失敗。除非你打開它,否則它是關的 - 舊版 Eidos
寫出的設定檔沒有這個鍵,而把一個缺少的鍵讀成「開」,會切斷每一個升級者的網路。

**Preferred servers** 為下載偏好的 CDN 節點排名,最好的在前。只有 premium 帳號才
會拿到超過一個鏡像可選,所以對其他人來說是 Nexus 決定,這個設定不改變任何事。它是
一個排序,不是篩選器:如果你指名的節點今天一個都沒提供,下載照樣進行,用 Nexus
最先給的那個節點。

**Categories** 是可以編輯的,不只是拿來顯示:把它們指派給單一模組或整批選取、在
同一個對話框裡編輯目錄本身,並從 Nexus 拉取該遊戲的官方類別清單。兩個目錄檔案都是
MO2 自己的(`categories.dat` 與 `nexuscatmap.dat`),所以共用的實例只會有一份目錄。

**View -> INI editor** 編輯設定檔的遊戲 INI - 會留存的那一份,而不是埋在 Proton
前綴裡、每次啟動都被覆寫的那一份。**View -> Log** 讀取工作階段日誌。
**View -> Extensions** 列出你自己的附加元件;見
[extensions.zh-TW.md](extensions.zh-TW.md)。

安裝什麼都接受:Simple 與 FOMOD 兩條路徑,加上 Wrye Bash 的 **BAIN** 套件(勾選子
套件,它們會依序合併),以及一個**手動**選擇器,會顯示壓縮檔的樹狀結構,讓你在沒有
任何啟發法認得出版面時自己指出資料根目錄。沒有任何壓縮檔會被拒絕。

**Diagnostics** 會跑即時的健康檢查:首要是啟動能力、缺少的 master(最可靠的單一
當機預測指標)、沒有任何啟用外掛會載入的封裝檔、模組清單是否仍與 mods 資料夾相符,
以及 - 在一次執行之後 - script extender 自己的日誌對它每一個外掛 DLL 說了什麼,這把
「我的 SKSE 外掛載入了嗎?」從推測變成證據。

要透過 GUI 啟動遊戲,把該遊戲的 Steam 啟動選項設成執行檔的絕對路徑(Steam 看不到
PATH 上的 `~/.cargo/bin`):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos 會在該遊戲的實例上開啟 - 你上次用的那一個,所以可攜實例和全域實例一樣會被再
找到;點 Run 就會穿過合併檢視啟動它。(如果你在 Steam 之外按下 Run 按鈕,它會顯示
這一行,帶著執行中執行檔的真實路徑。)

Bethesda 那幾款作品的 Steam `%command%` 通常指向 `<Game>Launcher.exe`。Eidos 從不
執行它:那個啟動器是一支獨立的設定程式,它會重新掃描 `Data` 並改寫 `plugins.txt`,
把剛剛部署好的載入順序推翻掉。如果裝了 script extender,它會換上 script extender
的載入器,否則換上遊戲主程式,而且在不得不退而求其次時會說明 - 一款啟動起來但每個
SKSE 模組都失效的遊戲,比一款根本啟動不了的更糟。

這裡舊版的說明會強制 `WINEDLLOVERRIDES="d3dcompiler_47=n"`。那已經不再需要,而且
從來就不完全正確:改成 *native* 的覆寫,只有在前綴裡已經有一份真正的
`d3dcompiler_47.dll` 時才有用。Eidos 現在會掃描已啟用模組的 DLL import,自己部署
真正的 Microsoft DLL,然後才設定那個覆寫。

## 試試概念驗證

不需要遊戲。它只用 user 命名空間裡的非特權 OverlayFS(Linux >= 5.11),就證明了
聯集 + copy-on-write + 零接觸 + 每個命名空間各自的範圍:

```sh
./scripts/poc-overlay.sh
```

## 工具

xEdit、BodySlide、DynDOLOD 以及同類工具,都是在遊戲的 Proton 前綴裡穿過合併檢視
執行:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # what the registered tools need, and its state
eidos prereqs skyrimse --install  # fetch whatever is missing
```

替工具命名之前要知道一件事:**標題決定 Eidos 為它準備哪些執行階段 DLL** -
`BodySlide` 會拿到它的 DirectX 函式庫,`BS` 什麼也拿不到。在 GUI 裡,Executables
對話框會在欄位底下顯示每一項前置需求的真實狀態,缺少的那些就是按鈕。

那張表、三個前置需求層級、DynDOLOD 為什麼需要一個 winetricks 裝不了的 .NET 執行
階段,以及以模組方式安裝的工具為什麼是從合併後的路徑而不是它自己的資料夾啟動,
都在 [tools.zh-TW.md](tools.zh-TW.md)。

從原始碼建置與儲存庫的目錄配置,在
[../internals/contributing.md](../internals/contributing.md)。

## 擴充

Eidos 不必重新建置就能擴充:一份放在 `~/.config/Colony/Eidos/addons/` 的 TOML
清單,可以在 Extensions 清單裡加一個工具,或在 Health 分頁加一項檢查。沒有任何
東西被載入 Eidos 內部 - 一個擴充就是它執行的一支程式。見
[extensions.zh-TW.md](extensions.zh-TW.md)。
