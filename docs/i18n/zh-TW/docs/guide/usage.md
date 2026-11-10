<!-- eidos-i18n: source=docs/guide/usage.md sha=039b6ffd055eabf9e9c4c8978bbc3e47ad68e45c -->

# 使用 Eidos

實務手冊:CLI、GUI、Steam 啟動選項、從原始碼建置,以及概念驗證腳本。
東西看起來不對勁時該怎麼辦,見 [troubleshooting.md](troubleshooting.md)。

## 使用它(CLI)

```sh
eidos games                       # supported games installed here (like MO2's list)
eidos init skyrimse               # create a modding instance
# ...drop each mod as a folder into <instance>/mods/ (the global instance lives
#    at ~/.local/share/Colony/Eidos/instances/skyrimse; `eidos init` prints yours)...
eidos install skyrimse mod.7z     # or install a downloaded archive (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adopt an existing MO2 profile's order + plugin state
eidos sort skyrimse               # LOOT-sort the plugin load order
eidos play skyrimse               # show what would be mounted
eidos play skyrimse -- <command>  # run <command> with the mods mounted over the game
eidos shell skyrimse              # 在合併檢視裡開一個 shell,寫入的內容進入 Overwrite
eidos check skyrimse              # 不開視窗跑健康檢查(腳本用 --json)
eidos sessions skyrimse           # 依由新到舊列出已記錄的遊戲與工具執行
eidos sessions skyrimse diff      # 顯示自上次以結束碼 0 結束的執行以來的變化
eidos collection skyrimse <link>  # install a Nexus collection the way its author built it
eidos nexus update skyrimse --apply  # install the Nexus files that replaced yours
eidos pack skyrimse backup.eidos  # the whole instance in one file, to move it elsewhere
eidos unpack backup.eidos <folder>   # put it back on the other machine
```

`eidos tool`、`eidos prereqs`、`eidos nexus`、`eidos nxm` 與 `eidos export` 把整套
補齊;不帶參數執行 `eidos` 就會列出完整清單。

### 從終端機執行健康檢查

`eidos check <instance>` 不開啟視窗就執行 **Diagnostics** 分頁裡的前綴、外掛、缺少的 master 與 script extender 檢查,因此可以透過 SSH 或在 Game Mode 下執行,結果也能放進錯誤回報。它先印出與 **Copy report** 相同的系統資訊行,再為每項發現印出一行,家目錄顯示為 `~`,登入名稱顯示為 `<user>`。沒有發現問題時結束碼為 0(只有建議仍為 0),至少發現一個問題時為 1,Eidos 無法檢查時為 2:參數錯誤、無法開啟的實例,或偵測不到的遊戲。

`eidos check <instance> --json` 改為印出一個 JSON 物件。最上層欄位是 `eidos`(版本)、`game`、`instance`、`system`(`proton`、`flatpak_steam`、`passthrough`、`session`、`distro`、`kernel`、`dev_fuse`、`fusermount3`、`max_map_count`、`open_files_hard`)與 `findings`。`max_map_count` 與 `open_files_hard` 是數字,Eidos 無法讀取時為 `null`。每項發現由 `code`、`level`(`problem` 或 `advice`)和它的 `args` 組成;Diagnostics 分頁會顯示每個代碼對應的句子。

### 合併檢視中的 shell

`eidos shell <instance>` 在遊戲所見的遊戲資料夾裡啟動你的 `$SHELL`(沒有就用 `/bin/sh`):每個已啟用的模組掛載在 `Data` 之上,Root 模組掛載在遊戲根目錄之上,寫到那裡的一切都進入 Overwrite。`eidos shell <instance> -- <command>` 則只執行一條指令,例如 `eidos shell skyrimse -- ls Data/meshes`,或 NifSkope 這類原生工具。這次執行像遊戲工作階段一樣持有實例鎖,所以在你結束之前視窗無法儲存變更,它在 `eidos sessions` 中以 `shell` 為名記錄,絕不算作一次遊戲啟動。對指令按 Ctrl+C 會像 Stop 一樣停止執行:Eidos 結束該指令、收集工作階段並卸載。在互動式 shell 中,Ctrl+C 照常屬於 shell;輸入 `exit` 結束執行。

### 實例:全域與可攜

上面每一個命令都指向某個實例。`skyrimse` 指的是**全域**的那一個 - 集中存放在
`~/.local/share/Colony/Eidos/instances/skyrimse`,由 Eidos 管理。另一種是**可攜**的:一個自我完備
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
有兩點但書值得知道:用手動的方式移動可攜資料夾,除了你以舊位置的絕對路徑註冊的
工具項目之外一切都保留(下面的 `eidos pack` / `eidos unpack` 會替你把那些路徑修好;
單純的 `mv` 不會),而共用的執行階段快取
(`~/.local/share/Colony/Eidos/runtimes/`)刻意維持在整台機器共用 - 一個 78 MB 的
.NET host 不該每個實例一份。

Eidos 把自己的檔案放在 `Colony/Eidos` 底下,那是 Colony 家族每個程式都用的配置:`~/.config/Colony/Eidos/` 放你選的東西(偏好設定、你的 Nexus 工作階段、你的實例清單、你自己寫的遊戲與附加元件定義),`~/.local/share/Colony/Eidos/` 放全域實例(`instances/`)、下載的執行環境與工作階段日誌(`logs/`),`~/.cache/Colony/Eidos/` 放 LOOT 主清單,缺少時隨時會重新下載。

舊版 Eidos 把這些放在 `~/.config/eidos/`、`~/.local/share/eidos/` 與 `~/.local/state/`。升級後的第一次啟動會把它們**複製**過來,並在日誌裡說明。全域實例用硬連結而不是逐位元組複製來搬移,所以幾十 GB 的模組一眨眼就搬完,也不佔用額外空間,它的工具項目也會改為指向新資料夾。舊目錄會原封不動留著 - 什麼都不刪,所以一次糟糕的升級不會讓你賠掉登入狀態或存檔 - 等你確認沒問題,可以自己移除。如果無法複製(磁碟已滿或唯讀、舊實例資料夾在另一顆磁碟上、遊戲仍從那裡執行中),Eidos 會繼續使用舊位置,並在下次啟動時再試一次。仍指向舊實例資料夾的 Steam 啟動選項會開啟搬移後的實例。不過不佔用額外空間只是一開始:搬移後你移除或重裝的模組和下載,其舊副本會透過舊實例資料夾繼續留在磁碟上,刪除 `~/.local/share/eidos/<game>/` 才能收回這部分空間。

可攜實例永遠不會被搬移:它留在你放的地方。

有一個位置會被直接拒絕:**遊戲安裝資料夾裡面**(MO2 老手的反射動作)。那棵樹歸
Steam 管 - 一次更新、一次「驗證檔案完整性」或一次解除安裝,都可能改寫或刪除它,
連你整套設定一起帶走 - 而且 Eidos 掛載在遊戲根目錄之上,所以放在那裡的實例會坐在
自己的掛載目標裡面。精靈、`eidos init` 與 `eidos play` 都會說不;請把資料夾放在
遊戲「旁邊」(同一顆硬碟上的同層目錄同樣方便)。

`play` 會在一個私有命名空間裡,把實例的模組掛載到遊戲自己的 `Data` 目錄之上
(透過 bind-stash,所以常駐程式仍然讀得到原封不動的檔案),然後讓命令穿過那個
檢視執行。寫入(存檔、重新產生的設定)落在實例的 `overwrite/` 層;遊戲安裝目錄與
每一份模組來源都逐位元組保持原封不動。

### Wabbajack 清單

Eidos 沒有 Wabbajack 安裝程式。請在 Eidos 之外用 CLF3 或 Jackify 安裝清單，然後在精靈中把安裝資料夾選為可攜式執行個體。Eidos 依 MO2 的配置讀取該資料夾：`mods/`、`profiles/<名稱>/modlist.txt` 和外掛清單。安裝程式寫入的 `ModOrganizer.ini` 會被保留，絕不會被覆寫，因此清單自帶的執行檔仍維持註冊。

許多清單會加入一個 `Stock Game` 或 `Game Root` 資料夾，裡面是修改過的遊戲副本，MO2 從那裡啟動遊戲。Eidos 啟動的是 Steam 安裝，也沒有指向其他遊戲資料夾的設定。精靈發現這樣的資料夾時，只會在確認畫面提示，不會做其他事：請自行把清單放在那裡的檔案移到某個模組的 `Root/` 資料夾中。CLF3 用自己的工具重建 BSA 和 DDS 檔案，所以結果可能與 Wabbajack 安裝不同，清單作者也不提供支援。

### 模組清單

和 MO2 一樣,每個設定檔都有自己的 `modlist.txt`(哪些模組啟用、優先順序如何),
而模組資料夾則放在實例共用的 `mods/` 裡。你手動丟進 `mods/` 的資料夾還不在清單上:
它會以**停用、最高優先順序**的狀態出現 - 也就是視窗清單的最底部,MO2 也是把它放在
那裡 - 所以一旦你啟用它,它就會贏下它的衝突。舊版會把它放在最上面,在那裡其他每個
模組都會蓋過它。

因為資料夾是共用的,移除或改名一個模組會改動每一個設定檔:它那一行會從每個設定檔的
`modlist.txt` 裡拿掉,或在其中改名,而其餘的部分每個設定檔都保有自己的狀態與順序。
一個不剩任何模組的設定檔會完全沒有 `modlist.txt`,而不是留一份空的;空的那份過去會
被讀成損毀的檔案,並擋住下一次安裝。

視窗是從它自己那份模組清單與載入順序的複本在工作。如果在它開著的時候,另一個 Eidos
行程寫入了其中任何一份 - 終端機裡的 `eidos install` 或 `eidos sort`、一份合集、一個
由 Steam 啟動的工作階段、第二個視窗 - 視窗的下一次編輯會被拒絕,而不是把那些成果
覆蓋掉。它會從磁碟重新載入,並顯示 *Not saved: another Eidos process changed the mod
list. It has been reloaded - redo the change.*(載入順序的情況也一樣);損失的只有
那一次點擊,所以再做一次就好。Rename、New mod、Add separator、Remove,以及把
Overwrite 變成模組,都會在動到 `mods/` 之前先檢查,所以一次被拒絕的編輯不會改動磁碟
上的任何東西。INI editor 也一樣:檔案在它開啟之後若被改動過,它會拒絕把內容存回去
覆蓋它。

### 存檔屬於設定檔

每個設定檔都把自己的存檔放在 `profiles/<name>/saves/`,而在一次執行的期間,那個
資料夾會掛載到 Proton 前綴裡遊戲的存檔資料夾之上。一個設定檔第一次啟動時,會把前綴
裡既有的存檔複製進去,只做一次,所以你之後從設定檔刪掉的存檔就會一直是刪掉的。每次
執行之後,設定檔最新的存檔會被複製回前綴,給 Steam Cloud 用;一份不是 Eidos 放進
前綴的存檔,會先以 `orphan-<time>-<name>` 保存在設定檔裡,絕不會被覆蓋。

在那第一次複製之後、沒有經過 Eidos 就落進前綴的存檔 - 一次從 Steam Deck 來的
Steam Cloud 下載、一次沒有透過 Eidos 的啟動 - 在設定檔的存檔掛載著的時候會一直被
遮住。沒有任何東西會刪掉它:啟動時會在這次執行的日誌裡指名它(從視窗啟動是實例的
`logs/run-*.log`,從 Steam 啟動則是 Steam 的主控台),那組檔案每變動一次就提一次。
要玩它,就在 Saves 分頁按下 Import。Import 會把它連同它的 co-save 一起
複製到設定檔裡,不會取代設定檔裡的任何存檔。

### 自上次正常執行以來的變化

無論從視窗、終端機還是 Steam 啟動，每次透過 `eidos play` 或 `eidos tool run` 的執行都會在 `<instance>/logs/sessions/` 寫入一筆記錄：設定檔、每個啟用的 mod 及其版本和 Nexus 檔案 ID、外掛順序、每個設定檔 INI 的檢查碼、遊戲執行檔的版本、Eidos 版本、結束碼，以及這次執行建立的存檔。保留最新的 50 筆，`logs/` 不會進入打包的執行個體。`eidos sessions <instance> diff` 會把最近一次以結束碼 0 結束的遊戲執行與目前狀態比較；`diff <id>` 從清單中選另一筆記錄。Eidos 目前不偵測當機，在 Proton 下當機的遊戲仍可能以結束碼 0 結束：若選中的執行屬於這種情況，請用 `diff <id>` 指定更早的一筆。

### 測試執行

Run 旁邊的 Test run 按鈕會像 Run 一樣啟動遊戲，但遊戲寫下的一切都會進入 `<instance>/test-run/`，而不是你的設定檔、Overwrite 和前綴：新的和改過的存檔、INI 變更、`plugins.txt`、寫進 Data 或遊戲資料夾的檔案，以及 `Documents/My Games/<game>` 裡的日誌。你可以用設定檔的模組測試，也可以不帶任何模組，或者只用清單裡選取的模組，加上提供它們外掛所需 master 的模組。

遊戲結束後，Eidos 會把這些檔案分成五組列出（存檔、INI 變更、外掛狀態、Overwrite 輸出、日誌），並逐組問你要保留還是丟棄。保留存檔絕不會從設定檔刪掉任何存檔，即使是遊戲在執行中刪掉的那個。丟棄任何東西都要再按一次。如果 Eidos 或電腦在測試執行期間停下，檔案會留在 `test-run/` 裡，下次你開啟這個實例或按 F5 時清單就會開啟。

如果視窗不是由遊戲的商店啟動的，Test run 會像 Run 一樣把遊戲交給那個商店（Steam、Heroic 或 Legendary），商店啟動的那次執行就是測試執行。測試執行只涵蓋上面列出的資料夾：遊戲的其他寫入，例如 Wine 登錄檔，照常寫到原處。測試執行的存檔要等你保留它們、再玩一次一般執行之後，才會進入 Steam Cloud。

### 找出造成問題的模組

Diagnostics 裡的 Find the mod behind a problem 按鈕會尋找造成某個可重現問題的模組，例如在某個地點當機。每一步都是一次測試執行，帶上剩下嫌疑模組的一半、它們需要的 master，以及你一直開著的模組（開始時在清單中選取的模組，和提供 script extender 的模組）。每次執行後，你回答問題是否出現；Not sure 會換一種方式分割嫌疑模組，Run it again 會重做這一步。回答或按 Run it again 會在再按一次後丟棄這一步寫下的檔案。想保留其中一部分，先按 See the files：它會開啟與測試執行之後相同的清單。如果遊戲當機且某個 crash logger 寫了日誌，或者遊戲以錯誤碼結束，Eidos 會把 Yes 標為可能的回答。200 個嫌疑模組大約需要 8 次執行。

搜尋保存在 `<instance>/bisect.json` 裡，所以你可以在兩步之間關閉 Eidos。Stop the search 在再按一次後放棄這次搜尋。結果會指出那個模組，或一組彼此需要對方 master 的模組，並保存在 `<instance>/logs/bisect-<time>.txt`。產生的模組（Nemesis 或 DynDOLOD 的輸出、Bashed Patch）是為完整清單建置的，只用一半時可能出錯：讓它們一直開著，或在整個搜尋期間都關著。

### 不需要任何特權步驟

Eidos 完全不需要 root 就能執行。它掛載在私有的 user + mount 命名空間裡,所以沒有
setuid 輔助程式、沒有常駐程式,也沒有什麼權限要授予。

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 是**選用的**,而且只管一件事:
核心 FUSE passthrough,它預設關閉,因為較早版本的常駐程式開著它會弄壞遊戲,而避開最
可能原因的變更還沒在遊戲裡重新測量過(見下)。帶著這個 capability 時,Eidos 取用的
是單純的 mount 命名空間而不是 user 命名空間;兩種方式部署模組的結果一模一樣。


舊的 `setcap` 建議為什麼消失了 - 以及 FUSE passthrough 為什麼出貨時就關著 - 在
[troubleshooting.md](troubleshooting.md#為什麼-passthrough-預設關閉)
有說明。

## 安裝一份 Nexus 合集

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run     # read it and say what would happen
```

在視窗裡:把連結貼進 **File -> Open a Nexus collection...**,然後按
*Install this collection*。

### 這為什麼不是「下載這些模組」

一份合集是一張「食譜」,不是一張購物清單,而且這張食譜幾乎沒有哪個部分能從 Nexus
API 看得到。安裝順序、作者給每個模組的腳本安裝器的那些答案、檔案衝突時哪個模組
勝出、哪些外掛載入到哪裡、那些二進位修補 - 全都住在合集自己的壓縮檔裡的一份
`collection.json` 中。下載同樣那些模組、再用它們的預設選項安裝,做出來的是另一個
遊戲。

所以 Eidos 會下載那份壓縮檔,讀出食譜,然後把它套用上去:

| 合集說的 | Eidos 做的 |
| --- | --- |
| `phase` | 依那個順序安裝,選用的成員排在最後 |
| `choices` | 重播作者對每一個 FOMOD 給的答案,做不到時就直說 |
| `modRules` | 重新排列模組清單,讓該贏的模組贏下每一個檔案 |
| `plugins` / `pluginRules` | 把 LOOT 規則合併進你的 userlist 並排序 |
| `INI Tweaks/` | 把它們裝成一個自成一格的模組 |
| `tools` | 告訴你它預期哪些工具;什麼都不建立 |

成員會在安裝的當下就被**啟用**,以安裝順序排在模組清單的最後面,在 `modRules` 把
它們搬動之前 - 沒有任何清單列到的模組,就是不會被載入的模組,而一份就這麼呆在磁碟
上不動的合集,正是那份報告看不見的唯一一種失敗。

狀態會在**每一個**成員之後寫進
`<instance>/collections/<slug>-<revision>.state.json` - 就在那個修訂版的資料夾
旁邊,絕不在它裡面,這樣一份合集就沒辦法夾帶自己的帳本 - 所以一次在 200 個成員中的
第 147 個被中斷的安裝,會從 147 接著跑。它會先以一個暫時的名稱寫出,再改名到定位,
所以一次中斷留下的不是舊的那份紀錄、就是新的那份,絕不會是任何一份的一半;萬一它
讀不出來,Eidos 會停下來直說,而不是靜悄悄地把整份合集從頭再跑一遍。再執行一次
同樣的命令就好。

安裝一份你已經有的合集的較新修訂版時,會接手上一個修訂版的資料夾:沒有變動的成員會
就地驗證,作者更新過檔案的成員會在它原本的資料夾裡被替換,而新修訂版拿掉的成員會
維持已安裝,並在報告裡被指名,讓你去停用或移除。另一個設定檔啟用中的資料夾絕不會被
接手,因為 mods 資料夾是共用的,那個設定檔會在你背後被改掉:新修訂版會改為在它旁邊
裝一份自己的複本,而報告會請你在這個設定檔裡停用舊的那一份,絕不會要你移除它。退回
較舊的修訂版時,會以同樣的方式把它的資料夾接手回來,接手不了的那些就裝上全新的複本。
每當這樣替換一個成員,而你曾在它的資料夾裡改過或加過檔案時,舊資料夾會先以 `<name>_backup` 的名稱留在旁邊,它絕不會被載入遊戲,報告也會指出它的名稱。

一個來自較早修訂版、仍然啟用著的殘留項目,會讓這次安裝無法算作作者建置的那一套;
你停用它之後,那則說明就會消失。你用 `--no-optional` 略過的成員不會被接手,並以同樣
的方式被指名。如果接手本身失敗,這次執行會停下,顯示
*previous revision: its folders could not be taken over* 以及原因;把那個問題修好,
再執行一次那道命令。

### 它不會假裝什麼

**免費的 Nexus 帳號抓不到那些成員模組。** Nexus 不會透過 API 為免費帳號鑄出下載
連結;每一個檔案都需要網站自己的「Mod Manager Download」按鈕。合集「壓縮檔」本身
用免費帳號下載毫無問題 - 所以你讀得到整張食譜 - 但每一個成員都會連同它確切的頁面
URL 一起回報,讓你自己去抓。那是 Nexus 的商業規則,重試再多次也繞不過去。透過網站
的按鈕把它們抓下來,再執行一次那道命令:每一次執行都會到 `downloads/` 裡找那些
成員,所以你補到哪裡,合集就完成到哪裡。

在視窗裡,免費帳號在合集面板中看到的是
*Fetch the N missing required files, one by one*,取代分批抓取。它會在 Nexus 上打開
第一個缺少的必要檔案的頁面,等那個檔案落進 `downloads/`,再打開下一個。每個檔案頁面上
的下載按鈕仍然由你自己按。面板會顯示進度 (*3 of 12*),並提供 *Skip* 和 *Stop*。如果落
進來的是同一個模組的另一個檔案,面板會寫出兩個檔案 id,並繼續等待合集指定的那一個。
選用成員不在其中,保留各自的 *Open* 按鈕。

依合集的描述套用:**二進位修補**(每個都依記錄的 CRC 校驗)、**檔案覆寫**(指定的檔案像 MO2 那樣用 `.mohidden` 隱藏),以及那些檔案隨著合集壓縮檔一起走的成員(`bundle`)。合集預期你親手去抓的成員
(`browse`、`manual`)會把作者自己的說明一路帶進報告裡。名稱已經和你某個模組撞上
的成員,會以一個沒被占用的名稱裝在它旁邊,絕不覆蓋它,而報告會說明是哪幾個。

### 那份報告

報告的重點,是讓「已安裝」值得信任。一個安裝器答案沒能全部重播完的成員,
是**已安裝,但不是照合集要求的方式** - 它自成一行,和它表面上很像的那些成功項目分開,
因為磁碟上的檔案和作者的不一樣。在合集裡沒有指到任何東西的排序規則、卡在一圈互相
矛盾的規則裡的模組、根本不存在的 LOOT 群組,以及這個版本的 Eidos 看不懂的 manifest
的任何部分,也全都會被列出來。

另一種做法 - 把那類事情記進日誌,然後宣告安裝完成 - 正是一份合集後來玩起來不對、
卻沒有人找得出原因的由來。

## 把一個實例搬到另一台機器

`eidos pack` 會把一整個實例 - 每一個模組、載入順序、所有設定檔、裝著你存檔的
Overwrite,以及當初安裝這一切所用的壓縮檔 - 寫進單獨一個檔案。`eidos unpack`
會把它放回去:

```sh
eidos pack skyrimse ~/backup.eidos                  # or give a folder: named for the game and the date
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # on the other machine
```

`eidos pack --dry-run` 會準確列出哪些東西會進去、哪些不會以及為什麼,還有它需要多少
空間,而不寫出任何東西。`eidos unpack --info` 對你手上已經有的檔案做同樣的事。

如果 7-Zip 有什麼讀不到,或是走訪目錄時不得不略過某個你會預期出現在備份裡的東西
(一條符號連結、一個讀不到的資料夾、一個無法安全交給 7-Zip 的名稱),壓縮檔還是會
寫出來、也會改成正式的名稱 - 它仍然值得留著 - 但 `eidos pack` 會以 **1** 結束,並
說明缺了什麼。一份不完整的備份不算成功,而這是一個
大家會擺在 `&&` 前面的命令。

一個 `.eidos` 檔案就是一個 7-Zip 壓縮檔,只是掛著自己的副檔名,這是一個選擇而不是
偽裝:安裝模組本來就需要 7-Zip,所以這不多添任何相依,而且就算你哪天沒了 Eidos,你的
備份用手邊任何一個解壓縮程式都還打得開。它用 LZMA2 以 `-mx1` 壓縮 - 在一份材質很
多的清單上大約是原始大小的 43%,花的時間只有 `-mx9` 為了多省幾個百分點所花的一小
部分。材質與 mesh 本來就是已經壓縮過的格式;更大的字典幾乎再也找不到什麼東西。如
果你對自己的內容有不同看法,`--level`(0、1、3、5、7、9)可以覆寫這個設定,而
`--no-downloads` 會把 `downloads/` 排除在外。

在預設的等級 1 下,檔案會放進**最大 16 MB 的固實(solid)區塊**裡。從一份 70 GB 的
備份裡取出一個檔案,最多只需解碼它所在的那個區塊,絕不會把排在它前面的東西全部解
開;打包遊戲內容也比每個檔案各自壓縮快 2 到 5 倍,解包快 2 到 4 倍。代價要在損壞時
才付:一個壞掉的位元組可能連帶它所在區塊裡的所有檔案,最多 16 MB 的相鄰檔案。下文
介紹的修復資料就是用來應付這種情況的。其他等級都維持非固實:每個檔案各自是一個資
料流,一個壞位元組只會損失這一個檔案。等級 0 只儲存檔案、不壓縮。從等級 3
起,LZMA2 本身就以 16 MB 或更大的分段運作,固實區塊在測量中反而更慢:在等級 5,打包
mesh 要花 1.4 倍的時間,解包材質要花 6.8 倍。

打包會取得實例鎖,所以遊戲或另一個 Eidos 正在寫入那個實例時它跑不起來 - 備份是一
張快照,不是一團糊掉的影像。它會先以 `.part` 名稱寫出,確實寫入磁碟,再用 `7z t` 從
頭到尾讀回一遍,通過後才改成你選的名稱。這次檢查會解碼每個區塊並比對每個校驗和,和在另一台機器上解包時做的工作相同。它說明的是寫完之後磁碟立刻讀回的內容,而不是這顆磁碟日後還能保住它;那是下文修復資料的用途。如果檢查失敗,`.part` 會被刪除,你選的名稱底下不
會寫入任何東西,`eidos pack` 會引用 7-Zip 找到損壞的那一行。一次被中斷或失敗的打
包留下的是一個明顯沒做完的東西,或者什麼都不留,而不是一個看起來完整、實際上不是的
`.eidos` 檔案。

裝了 par2cmdline 時(Arch 上 `sudo pacman -S par2cmdline`,Debian 與 Ubuntu 上
`sudo apt install par2`,Fedora 上 `sudo dnf install par2cmdline`),`eidos pack`
還會在壓縮檔旁邊寫出 PAR2 修復資料:`backup.eidos.par2` 和幾個
`backup.eidos.volNN+MM.par2` 檔案,總共是壓縮檔的 5% 再加上一個小索引。請把它們以寫出時的名稱和 `.eidos` 檔案放在同一個資料夾,並且一起複製:沒帶著它們搬走的備份就沒有東西可以拿來修復,而且修復資料記下了壓縮檔的名稱,改過名的備份就和它們對不上了。`eidos unpack` 會依名稱找到它們,在解出任何東西之前先用它們核對壓縮檔,結尾被截斷、7-Zip 已經打不開的備份也一樣。如
果有幾個位元組壞了,par2 會先修好壓縮檔,並把損壞的副本保留為 `backup.eidos.1`,為此壓縮檔所在的資料夾裡需要有和壓縮檔一樣大的可用空間;超
出這 5% 能彌補的損壞,會讓解包在寫出任何一個檔案之前就停下。和壓縮檔對不上的修復檔案,例如同名的舊備份留下的那些,在 7-Zip 讀完壓縮檔並確認它完好時不會被使用,解包會說明這一點。在沒有 par2 的機器上
解包照樣進行,但會警告修復資料沒有被用上。`--no-recovery` 打包時不寫修復資料,
`eidos pack --dry-run` 會說明是否會寫。

### 裡面沒有哪些東西,以及為什麼

| 未納入 | 為什麼 |
| --- | --- |
| Proton 前綴 | 它的形狀取決於這台機器(磁碟機對應、對*這個*檔案系統的 `Z:` 檢視),而 Eidos 用 `eidos prereqs <instance> --install` 一分鐘就能重建它。帶著它會讓壓縮檔大約大上一倍,只為了運送一個另一台機器反正得重新產生的東西。 |
| 遊戲本身 | 一個實例改的是一款它並不包含的遊戲。在那邊從 Steam 安裝它。 |
| 實例之外的工具 | xEdit、DynDOLOD 以及同類工具是第三方執行檔,有自己的安裝程式。它們的名字會**記在**壓縮檔裡,所以解包時會準確告訴你新機器上缺了哪幾個。 |
| `logs/`、`.eidos.lock`、`prereqs.done`、`prereqs.log` | 這些是做出這份備份的那台機器的紀錄。尤其是 `prereqs.done`,它會宣稱執行階段函式庫已經裝在一個根本不存在的前綴裡。 |
| `loot/`,但 `userlist.yaml` 除外 | 一份 masterlist 快取,Eidos 需要時會重新抓。你自己的 LOOT 規則不是快取 - 沒有人會去重新抓那些 - 所以那一個檔案會跟著一起走。 |
| `.base/`、`.base-root/` | 空的掛載點,工作階段期間遊戲自己的檔案被暫存在這裡。 |
| 寫到一半的檔案 | 暫停中的下載(`*.unfinished`)、進行到一半的原子寫入(`*.eidos-tmp*`)。 |
| 符號連結 | 7-Zip 會跟著它走,把它指向的東西複製進來,而對一條絕對路徑的連結來說,那等於把一棵不相干的目錄樹拉進你的備份。改成回報它們,而且這份備份會被算作不完整:連結到另一顆硬碟的 `mods/` 或 `downloads/` **不會**被打包。如果你本來就打算不帶那個連結過去的 `downloads/` 來備份,就加上 `--no-downloads`(或在視窗裡取消勾選 *Include downloads/*);刻意的排除不算缺少。 |

以上每一項都會連同它的原因寫進壓縮檔根目錄的 `eidos-backup.ini` - 所以「這裡面沒有
哪些東西」的答案是一個 `cat`,而不是用猜的。同一個檔案也記下了實例原本住在哪裡、
當時啟用的是哪個設定檔、解開後有多大,以及新機器會需要哪些工具。

下載紀錄(`downloads/*.meta`)會被納入,但它們的 `url=` 那一行會先被**清空**。那條
URL 是一條簽章過的 Nexus 連結:它幾個小時內就會失效,而且它帶著下載那個檔案的帳號
的 `user_id`。備份就是拿來交給別人的,所以它不帶那個東西。所有讓這份下載能被重新
找回來的資訊 - 模組 id、檔案 id、版本 - 都留著。

### 在視窗裡

兩者都在 **File** 選單裡:*Pack this instance...* 與 *Unpack a backup...*。解包也放在
歡迎畫面上,就在實例清單下面,因為最需要它的那台機器,正是還沒有任何實例的那一台。

打包對話框會在動手之前先預覽:有多少個檔案、多大、其中有多少是 `downloads/`、壓縮
檔大概會變成多大,以及有哪些工具只被記下名字而沒有被帶走。它的 *Write recovery
data* 核取方塊在裝了 par2 時是勾選的,沒裝時則呈灰色,並寫明要安裝的套件。解包對話
框會在你選好檔案的那一刻就讀取備份的 manifest,所以在你把資料夾交給它之前,它就能
告訴你這份備份裝的是哪款遊戲、是什麼時候做的、原本住在哪裡,以及它的工具有哪幾個
在這裡是缺少的。

兩者都在工作執行緒上執行,帶著進度列,所以它們忙著做事的時候視窗仍然有回應,而做完時
那條進度列就變成報告。打包會在整段執行期間握著實例鎖:實例正在被讀取的時候,別的東西
都不准碰它,這正是讓備份成為一張快照、而不是一團糊掉的影像的原因。

### 解包會修好什麼

單純複製那個資料夾,會讓每一個工具按鈕都還指著舊機器上的路徑。`eidos unpack` 只
改寫那些指名舊實例根目錄的值,而且只改那些:`tools.ini` 裡的 `exe`、`workdir` 與
帶編號的 `arg` 鍵,以及每個模組 `meta.ini` 裡的 `installationFile`。從來就不在實例
裡面的工具(`/opt/xedit/SSEEdit.exe`)會原封不動留著,並被列為需要重新安裝的東西 -
猜別人的 xEdit 裝到哪去了是發明,不是修復。它也會更正實例自稱是集中式還是可攜式,
好讓後續的命令到它現在所在的地方去找它。

解包會拒絕這些情況:資料夾不是空的(如果你確實想要覆蓋,加上 `--force`)、磁碟
空間小於 manifest 自己給的數字、壓縮檔是由比你新的 Eidos 做出來的,以及任何含有
會被寫到你所選資料夾*外面*的項目的壓縮檔。

之後:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # rebuild the Proton prefix
eidos play /mnt/games/EidosSkyrim
```

## GUI

```sh
cargo run -p eidos-gui
```

一個 MO2 風格的首次啟動精靈,以 Colony 共用的主題繪製:歡迎 -> 實例類型
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

### 偏好設定

點左上角的 **Eidos**,偏好設定就會取代兩個窗格開啟;再點一次(或點 **Close**)即可
返回。最前面是 General、Appearance 與 Accessibility,和每個 Colony 程式一樣,接著是
Mod list、Nexus 與 About。每項變更都會立即套用並儲存;沒有儲存按鈕。

- **General**:Eidos 開啟的遊戲、視窗大小、介面語言、工具資料夾、遊戲的執行方式,
  以及視窗在背景時是否以桌面通知告知長時間工作結束。
  視窗介面支援英文與法文;**System** 會依循 `LC_ALL`、`LC_MESSAGES` 或
  `LANG`,變更立即生效。`eidos` 命令列一律以英文回應。
- **Appearance**:主題(25 個系列、57 套配色,預設為 Eidos 的羊皮紙)、強調色、
  文字大小、效果,以及即時預覽。
- **Accessibility**:高對比、適合閱讀障礙者的字型、減少動態效果,以及一個與
  Appearance 文字大小相乘的閱讀文字大小。

這些設定存放在 `~/.config/Colony/Eidos/preferences.json`。舊版 Eidos 存在
`settings.ini` 裡的主題,會在第一次啟動時沿用過來。

**About -> Version** 裡有版本檢查,預設開啟。啟動時,每天最多一次,Eidos 會向 GitHub 詢問 `Project-Colony/Eidos` 的最新版本。如果它比正在執行的這一份新,選單列會在 Help 後面顯示 `Eidos 1.24.0 is available: release notes`,點這個連結會開啟該版本的頁面與它的變更記錄。Eidos 從不自己下載或取代自己。離線模式下不做這項檢查。由套件管理員裝在 `/usr` 下的副本也不做檢查,因為由套件管理員負責更新;`install.sh --system` 放置 Eidos 的 `/usr/local` 算作你自己的安裝。

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

Ctrl+Z 復原對模組清單或載入順序的上一次變更:一次勾選、一次移動、一次批次停用、一次 LOOT 排序。Ctrl+Shift+Z 重做。
視窗保留 50 步,在按 F5、遊戲或工具執行結束、切換設定檔以及另一個 Eidos 程序變更清單時
清除。安裝、重新命名或移除模組會改變磁碟上的資料夾,所以越過這類操作的復原會被拒絕。

### 篩選清單

Filters 按鈕為每個條件列出一列:已啟用、有衝突、有更新、無類別、已在 Nexus
上推薦 (endorsed)、已追蹤 (tracked)、有 Nexus id、有隱藏檔案、沒有有效的遊戲
資料、給另一個遊戲用、有備註,以及每種內容一列(外掛、BSA 或 BA2 壓縮檔、材質、
網格、指令碼、script extender 外掛、介面檔案、音效、FaceGen)。每點一下,該列
就依關閉、「only」、「except」的順序切換,然後回到關閉。最上面一列在「符合所有
已開啟的條件」與「符合其中任何一個」之間切換。名稱框與類別清單一律會再縮小
結果。隱藏檔案這一列只看已啟用的模組。

### 清單會告訴你關於一個模組的什麼事

兩個提示性旗標,都是一個字符,滑鼠停留就有說明。**No valid game data** 表示模組
最上層沒有任何東西看起來像這個遊戲會載入的內容;它可能需要把資料夾往上移一層,也
可能根本不是這個遊戲的模組。**Another game** 表示模組自己的 `meta.ini` 指名了另一
款遊戲。兩者都不會擋住任何事 - 模組照樣部署 - 而列選單上的「Mark as valid」可以
讓其中任一個閉嘴,靠的是 MO2 自己的 `validated=` 鍵,所以你在一個管理器裡背書過的
模組,到另一個裡也是安靜的。

這個版面檢查刻意放寬:一棵 `Root/` 樹算數,一個讀不到的資料夾算數,一個空的也
算數。在一份五百列的清單上,一個錯誤的警告比一個漏掉的警告更糟。

### 停用存檔或其他模組用到的模組

在停用或移除一個模組之前,Eidos 會讀取該設定檔最新三個存檔的外掛列表。只要其中一個
存檔用到了這個模組裡的外掛,列表上方就會出現一條提示列,列出這些外掛,並等待你按
「Disable anyway」或「Cancel」(按 Esc 也會取消)。Remove 會把同樣的外掛名稱加進它
要求再點一次的訊息裡。只讀取 Skyrim 和 Fallout 4 的存檔;其他遊戲不詢問,直接停用。

模組的 Information 裡的 General 分頁會列出它需要的模組和需要它的模組,依據已啟用外掛的主檔案讀取。停用其他模組需要的模組時會出現同樣的提示列:「Disable them too」會同時停用所有需要它的模組,以及需要那些模組的模組;「Keep them enabled」則只停用所選的。 Health 分頁上的缺少主檔案卡片有 "Enable what provides them":它會啟用包含每個缺失主檔案的模組,並勾選存在但未勾選的主外掛。

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
欄位理清什麼在哪裡,還能把任一列在檔案管理器裡 Reveal 出來。
模組最上層的 `meta.ini`、`readme.txt`、`fomod/` 與 `Root/` 不會進入 Data,這個分頁和掛載裡都一樣:前三個是給模組管理器用的,`Root/` 則改放到遊戲資料夾。遊戲或工具以這些名稱寫入的檔案會落在 Overwrite,並保持可見。**Plugins** 是
ESP/ESM/ESL 的載入順序(切換、手動重新排序,或用 LOOT 排序並閱讀排序後的報告,
報告裡的建議連結會在你的瀏覽器開啟)。它的標題列依索引空間統計已啟用的外掛數量,並與遊戲的上限對照,例如 `full 231/254, light 512/4096`,某個空間滿了就變成橙色。LOOT 排序之後,新紀錄全部落在 light 範圍內的完整外掛會在滑鼠停留時說明這一點,它那一列的選單裡也會出現 Flag as light。Eidos 會把帶 light 旗標的副本寫進一個名為 `<plugin> (light)` 的新模組,放在模組清單最底部(優先順序最高),原外掛不動,所以停用這個模組就能復原。如果該設定檔最新三個存檔中有一個把這個外掛當作完整外掛載入,Eidos 會拒絕。只會讀取 Skyrim 和 Fallout 4 的存檔,所以在 Starfield 上不會檢查任何存檔。LOOT 只檢查紀錄範圍:會新增儲存格或世界空間的外掛,標記之前先在 xEdit 裡看一下。**Conflicts** 說明每個檔案的贏家與輸家。
**Overwrite** 一步就把遊戲寫出來的東西變成真正的模組。**Saves** 解析每份存檔的
標頭 - 角色、等級、位置、遊玩時間 - 並把烘進存檔的外掛清單和你目前的比對,還附一個
按鈕啟用它需要的模組,因為只點名它們然後丟給你自己處理,是無聊的那一半。

將滑鼠停在外掛上,會顯示它的來源、作者、標頭版本與表單版本、說明的前 300 個字元、它的主檔,以及在 LOOT 排序之後 LOOT 的訊息。 在 Skyrim、Skyrim SE、Skyrim VR 與 Enderal SE 上,遊戲還會讀取與已載入外掛同名的 INI;INI 欄會標出這些外掛,點擊即以唯讀方式開啟該檔案。

Data 分頁中的 `.exe` 列有 Add tool 和 Run 兩個按鈕。Add tool 會開啟 Executables 編輯器並填入該檔案(按下 Save 之前不會儲存任何內容);Run 會像已註冊的工具一樣,透過合併檢視把它執行一次。Save tree 會把合併後的整個 Data 樹寫入文字檔，每個檔案一行，並註明提供它的 Mod，其中已移除您的家目錄與使用者名稱，可用於錯誤回報。

Steam Cloud 從其他裝置下載的存檔,或沒有透過 Eidos 的啟動寫下的存檔,會落進遊戲自己的存檔資料夾,而遊戲執行期間設定檔的存檔會蓋住這個資料夾。Saves 分頁在標題下方那一行顯示它們的數量,Import 會把每個存檔連同它的 co-save 一起複製到設定檔裡。設定檔裡已有同名存檔的,會以 `orphan-<time>-<name>` 匯入;設定檔裡的任何內容都不會被取代。

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

**Preferences -> Nexus -> Install downloads when they finish** 預設為關閉。開啟後，視窗開著時完成的 Nexus 下載會直接安裝，不必再到 Downloads 分頁。這是一般的安裝：FOMOD 會開啟它的安裝程式，模組清單裡已有的名稱會詢問如何處理。視窗開啟之前、或選項開啟之前就在 `downloads/` 裡的壓縮檔不會被動，你取消過安裝的下載也一樣。在合集面板中開啟的合集的成員，以及等待中的 Update 所等的檔案，只由各自的流程安裝。遊戲或另一個 Eidos 佔用實例時，安裝會等到它空閒。

### 模組更新

工具列右端的更新按鈕會把每個已安裝的檔案,與 Nexus 上取代它的
檔案相比較。作者會把一次新上傳標記為某個舊檔案的後繼,Eidos 從模組 `meta.ini` 記錄的
檔案 ID 出發,沿著這條鏈往下找。鏈上沒有後繼時,改為比較模組頁面的版本。

模組右鍵選單裡的 Track 和 Untrack 會把該模組加入或移出你在 Nexus 上的追蹤模組,
這樣網站的通知就會與你在這裡追蹤的內容一致。每次檢查更新時,Eidos 也會從 Nexus 讀取你的
推薦(endorsement)與追蹤模組(兩次請求),並依模組自己的 Nexus 網域,把每個模組的
Endorsed 與 Tracked 標記設成一致。兩邊不一致時,以 Nexus 為準。任一清單讀取失敗時,
檢查結果會說明這一點,並保留那些標記不變。

「Information...」的 **Nexus files** 分頁列出該模組在 Nexus 上的檔案,最新的在前,
並標出這裡安裝的那個與取代它的那個。後者那一列有一個 **Update** 按鈕。premium 帳號由
Eidos 自己下載檔案。免費帳號則開啟檔案頁面,在那裡 Mod Manager Download 會把檔案送回來,
壓縮檔抵達 `downloads/` 時安裝就開始。之後就是平常的安裝:FOMOD 會以模組安裝時的答案
開啟,接著的詢問提供 Replace,並勾選「Keep a backup of the existing mod」,不論你上次
詢問時選了什麼。Replace 會保留模組的優先順序與啟用狀態。

含有多個 Nexus 檔案的資料夾(例如混入了選用檔案的主檔案)會被拒絕:用新的主檔案取代它
會丟掉其他檔案。這些請手動更新。

`eidos nexus update skyrimse --apply` 會做同樣的檢查,然後在 premium 帳號上下載每個新
檔案,並用 `--replace --backup` 安裝到它的資料夾上。在免費帳號上則改為替每個檔案頁面
印出 `OPEN <url>`。合併了多個檔案的資料夾會被略過並附上原因。沒有儲存答案的 FOMOD
模組(例如在 Eidos 開始儲存答案之前安裝的)不會用安裝程式的預設值重新安裝:這次更新會
失敗,並提示你在視窗中更新該模組一次,視窗會儲存答案。安裝程式沒有任何可選項的 FOMOD
照常安裝。只要有一個更新失敗,命令就以 1 結束。

### Nexus 合集

貼上一條合集連結 - 或在網站上點一條 - Eidos 就會列出該修訂版的成員,每一個都和這個
實例對照過:已安裝、已下載,或缺少。接著按 *Install this collection*,它就會在工作
執行緒上跑完整張食譜,用的是打包與解包工作用的同一條進度列,而做完時那條進度列就
變成報告。上面關於什麼會被套用、什麼只會被指名的一切,在這裡一字不差地同樣成立:
這個視窗和 `eidos collection` 是同一具引擎的兩個前端。

一份合集只能對照**它自己的遊戲**來讀取。載入 Fallout 4 實例時開啟一份 Skyrim 合集,
它會指名拒絕,而不是把成員和錯誤的模組清單對照 - 在那份清單上,每一個「已安裝」和
每一個「缺少」都只是披著答案外形的雜訊。

檢查更新時，Eidos 還會向 Nexus 詢問此執行個體中每個已安裝合集的最新修訂版。模組清單上方的一行會列出有較新修訂版的合集，並統計新增 (+)、更新 (~) 和移除 (-) 的模組數量，把滑鼠移上去可看到名稱。如果已安裝修訂版的配方不再快取，則沒有數量。點選**更新**會在合集面板中開啟較新的修訂版，由 *Install this collection* 執行安裝。該清單在每次檢查時重建，不會儲存。

### 離線模式

**Preferences -> Nexus -> Offline** 會讓 Eidos 完全不去接觸 Nexus,也會跳過每天一次的 GitHub 版本檢查。更新檢查、登入、
下載與合集都會直說,而不是以連線錯誤失敗。除非你打開它,否則它是關的 - 舊版 Eidos
寫出的設定檔沒有這個鍵,而把一個缺少的鍵讀成「開」,會切斷每一個升級者的網路。

**Preferred servers** 為下載偏好的 CDN 節點排名,最好的在前。只有 premium 帳號才
會拿到超過一個鏡像可選,所以對其他人來說是 Nexus 決定,這個設定不改變任何事。它是
一個排序,不是篩選器:如果你指名的節點今天一個都沒提供,下載照樣進行,用 Nexus
最先給的那個節點。

**Categories** 是可以編輯的,不只是拿來顯示:把它們指派給單一模組或整批選取、在
同一個對話框裡編輯目錄本身,並從 Nexus 拉取該遊戲的官方類別清單。兩個目錄檔案都是
MO2 自己的(`categories.dat` 與 `nexuscatmap.dat`),所以共用的實例只會有一份目錄。 從 Nexus 下載的模組,在「從 Nexus 取得」對應好目錄之後,安裝時會自動歸類。

**View -> INI editor** 編輯設定檔的遊戲 INI - 會留存的那一份,而不是埋在 Proton
前綴裡、每次啟動都被覆寫的那一份。**View -> Log** 讀取工作階段日誌。
**View -> Extensions** 列出你自己的附加元件;見
[extensions.md](extensions.md)。

啟動時如果在設定檔和前綴裡都找不到遊戲的主 INI(`Skyrim.ini`、`Fallout4.ini`、`Oblivion.ini` 等),Eidos 會像 MO2 一樣,先把遊戲資料夾裡的預設檔(`Skyrim_Default.ini`、`Fallout4_Default.ini` 等)複製到設定檔。前綴和遊戲資料夾不會被改動。

安裝什麼都接受:Simple 與 FOMOD 兩條路徑,加上 Wrye Bash 的 **BAIN** 套件(勾選子
套件,它們會依序合併),以及一個**手動**選擇器,會顯示壓縮檔的樹狀結構,讓你在沒有
任何啟發法認得出版面時自己指出資料根目錄。沒有任何壓縮檔會被拒絕。

重新安裝或更新 FOMOD 模組時，精靈開啟時會勾好上次安裝的答案，停在之後選項有變動
的第一步，並列出變動。**Reinstall with new choices** 則從安裝程式的預設值開始。
`eidos install --replace` 或 `--merge` 沿用儲存的答案，答案不再適用時會列出變動並
拒絕安裝。

**Diagnostics** 會跑即時的健康檢查:首要是啟動能力、缺少的 master(最可靠的單一
當機預測指標)、沒有任何啟用外掛會載入的封裝檔、模組清單是否仍與 mods 資料夾相符,
以及 - 在一次執行之後 - script extender 自己的日誌對它每一個外掛 DLL 說了什麼,這把
「我的 SKSE 外掛載入了嗎?」從推測變成證據。
它也會回報:自上次乾淨啟動以來的遊戲更新,缺少或與遊戲版本不符的 script extender(附 extender 的下載頁面,以及安裝你下載的封裝檔的按鈕),沒有用 `%command%` 啟動 Eidos 的 Steam 啟動選項,DLSS 需要卻仍關閉的 NVAPI,阻擋畫格生成的獨佔全螢幕,沒有 `LinuxVersion=true` 的 ENB `enblocal.ini`,F4SE 與 SFSE 外掛的宣告,以及上次工作階段的當機日誌和其中點名的模組。
檢查更新時也會讀取每個 mod 在 Nexus 上的前置需求。Diagnostics 會列出缺少或已停用的前置 mod,已用其他方式滿足的需求可用 Ignore 忽略。

對於 Skyrim SE、Enderal SE 和 Fallout 4,Diagnostics 還有一張「顯示」卡片。它顯示設定檔的 `SkyrimPrefs.ini`(`EnderalPrefs.ini`、`Fallout4Prefs.ini`)中設定的視窗大小和模式;設定檔還沒有這個檔案時,則顯示前綴裡那份副本的設定。兩者都沒有設定大小時,輸入欄會取 `/sys/class/drm` 中列出的已連接的最大螢幕,所以 Steam Deck 上顯示 1280x800。Apply 會把 `iSize W`、`iSize H`、`bFull Screen` 和 `bBorderless` 寫進設定檔的副本;設定檔沒有副本時,先接管前綴裡的那份。SSE Display Tweaks 設定視窗時,Apply 還會把同樣的大小和模式寫進它在 Overwrite 中的副本;模組本身的檔案不會被改動。

要透過 GUI 啟動遊戲,把該遊戲的 Steam 啟動選項設成執行檔的絕對路徑(Steam 看不到
PATH 上的 `~/.cargo/bin`):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos 會在該遊戲的實例上開啟 - 你上次用的那一個,所以可攜實例和全域實例一樣會被再
找到;點 Run 就會穿過合併檢視啟動它。

開啟 **Start the game at once when a store opens Eidos**(Preferences、General、Running a game)後,Eidos 會省掉這次點擊:它先檢查 Health,再不顯示視窗直接啟動遊戲。Health 列出問題、模組清單需要處理、設為在遊戲前執行的工具輸出已過期,或 Skyrim SE、Enderal SE、Fallout 4 的設定檔還沒有視窗大小時,視窗照常開啟(最後這種情況下視窗會停在 Diagnostics 分頁)。想在某一次啟動時看到視窗,就在命令前加上 `EIDOS_WINDOW=1`:`EIDOS_WINDOW=1 ~/.cargo/bin/eidos-gui %command%`。

在 Steam 之外,Run 不會自己用 Proton 或 Wine 啟動遊戲。它請擁有這份副本的商店來啟動,再由商店啟動 Eidos:

- Steam:設好上面的啟動選項後,執行 `steam -applaunch <appid>`。Steam 沒開時會先啟動 Steam。
- Heroic(GOG 或 Epic 副本,原生版或 Flatpak 版):當 Eidos 是 Heroic 裡該遊戲設定中的一個包裝器(wrapper),執行檔為 `eidos-gui` 的絕對路徑且不帶參數時,它會收到一個 `heroic://launch` 連結。Heroic 沒開時會先啟動 Heroic。
- 單獨的 Legendary:當 `~/.config/legendary/config.ini` 在遊戲的 `[<app>]` 區段或 `[default]` 下有 `wrapper =` 加上 `eidos-gui` 的絕對路徑時,執行 `legendary launch <app>`。

接著 Eidos 會在你上次用的實例上開啟第二個視窗,你在那裡點 Run。第一個視窗不會追蹤這次執行,所以玩完後在它裡面按 F5 重新載入清單。商店的設定缺少,或 script extender 的 loader 不在遊戲執行檔旁邊時,Run 什麼也不啟動,並顯示要在哪裡設定什麼,帶著執行中執行檔的真實路徑。

### 每個設定檔的變數與包裝器

設定檔可以在 `profiles/<名稱>/launch-env.ini` 裡帶上自己的環境變數與包裝器指令,每行一項:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

它們對該設定檔的每次 `eidos play`(由 Steam 的 `%command%` 啟動的遊戲,或由商店開啟的視窗裡的 Run 啟動的遊戲)與每次工具執行生效。`eidos shell` 只會得到變數,沒有包裝器。第一條 `wrapper=` 行在最外層。含有 `%command%` 的 `wrapper=` 行會把指令放在那個位置,和 Steam 的啟動選項一樣;不含的行把指令加在最後。包裝器在 Eidos 的私有命名空間裡執行,所以看得到模組。

這裡的變數會取代 Eidos 為同名變數設定的值,而工具在 `tools.ini` 裡自己的 `env=` 行會同時取代這兩者。`WINEDLLOVERRIDES` 例外:它會附加在 Eidos 為 ENB、ReShade 與內建 DLL 設定的覆寫之後,所以兩邊都提到的 DLL 採用設定檔的設定,其餘維持原樣。

View -> INI editor 會把這個檔案當作最後一個分頁開啟,每款遊戲都一樣。儲存時會列出 Eidos 忽略的行號:既不是 `env=名稱=值` 也不是 `wrapper=指令` 的行。不帶指令的 `eidos play <實例>` 會列出一次執行得到的變數與包裝器。Diagnostics 裡的 Copy report 會寫出變數名稱與包裝器程式名稱,不含它們的值。

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
都在 [tools.md](tools.md)。

從原始碼建置與儲存庫的目錄配置,在
[../internals/contributing.md](../../../../internals/contributing.md)。

## 擴充

Eidos 不必重新建置就能擴充:一份放在 `~/.config/Colony/Eidos/addons/` 的 TOML
清單,可以在 Extensions 清單裡加一個工具,或在 Health 分頁加一項檢查。沒有任何
東西被載入 Eidos 內部 - 一個擴充就是它執行的一支程式。見
[extensions.md](extensions.md)。
