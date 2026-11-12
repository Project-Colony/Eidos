<!-- eidos-i18n: source=docs/guide/usage.md sha=6459b22b0401855f4cacc644cb35eade2e2a023d -->

# 使用 Eidos

如何使用 Eidos:實例與遊戲副本、命令列、測試執行、視窗、Nexus 合集與更新、把實例搬到
另一台機器、工具與擴充。遇到問題請見 [troubleshooting.md](troubleshooting.md);從原始碼
建置與儲存庫的目錄配置,請見
[../internals/contributing.md](../../../../internals/contributing.md)。

## 實例:全域與可攜

接受實例的命令,可以給遊戲 id,也可以給可攜實例的資料夾。`skyrimse` 指的是全域實例,
Eidos 把它放在 `~/.local/share/Colony/Eidos/instances/skyrimse`。可攜實例是一個自我完備
的資料夾,放在你想要的任何地方(第二顆硬碟、遊戲分割區),可以移動,和 MO2 的可攜實例
一樣:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # create a portable instance there
eidos install /mnt/games/EidosSkyrim mod.7z  # every command accepts the folder
eidos play /mnt/games/EidosSkyrim -- %command%
```

資料夾裡的 `eidos-instance.ini` 指明了遊戲,所以資料夾不需要別的東西。環境中的
`EIDOS_INSTANCE=<folder>` 會把一個遊戲 id 重新導向到該資料夾,例如用在 Steam 啟動選項
裡。你建立過或開啟過的可攜實例會被記在 `~/.config/Colony/Eidos/instances.ini`,最近
使用的在前:視窗的歡迎畫面會列出它們,Steam 啟動會落在你上次玩的那一個,`nxm://`
處理器也會下載進去。

用手動的方式移動可攜資料夾,除了你以舊位置的絕對路徑註冊的工具項目之外,一切都保留。
`eidos pack` 和 `eidos unpack`([下文](#把一個實例搬到另一台機器))會把那些路徑修好;
單純的 `mv` 不會。`~/.local/share/Colony/Eidos/runtimes/` 裡的執行階段快取由機器上的
每個實例共用,所以一個 78 MB 的 .NET host 不會複製進每一個實例。

有一個位置會被拒絕:遊戲安裝資料夾裡面,MO2 使用者常把實例放在那裡。那棵樹歸 Steam 管,
一次更新、一次「驗證檔案完整性」或一次解除安裝,都可能改寫或刪除它,連你的設定一起帶走。
Eidos 也掛載在遊戲根目錄之上,所以放在那裡的實例會坐在自己的掛載目標裡面。精靈、
`eidos init`、`eidos play` 和 `eidos shell` 都會拒絕。請把資料夾放在遊戲旁邊;同一顆硬碟上
的同層資料夾同樣方便。

### Eidos 的檔案放在哪裡

Eidos 把自己的檔案放在 `Colony/Eidos` 底下,那是 Colony 家族每個程式都用的配置:
`~/.config/Colony/Eidos/` 放你選的東西(偏好設定、你的 Nexus 工作階段、你的實例清單、
你自己寫的遊戲與附加元件定義),`~/.local/share/Colony/Eidos/` 放全域實例(`instances/`)、
下載的執行階段與工作階段日誌(`logs/`),`~/.cache/Colony/Eidos/` 放 LOOT 主清單,缺少時
隨時會重新下載。

當它找到舊版 Eidos 配置的檔案(`~/.config/eidos/`、`~/.local/share/eidos/` 和
`~/.local/state/`)時,第一次啟動會把它們複製過來,並在日誌裡說明。全域實例用硬連結而不是
逐位元組複製來搬移,所以它的模組不佔用額外空間,它的工具項目也會改為指向新資料夾。舊目錄
裡的任何東西都不會被改動或刪除;等新配置運作正常後,請自己移除它們。如果無法複製(磁碟已滿
或唯讀、舊實例資料夾在另一顆磁碟上、遊戲仍從那裡執行中),Eidos 會繼續使用舊位置,並在
下次啟動時再試一次。仍指向舊實例資料夾的 Steam 啟動選項會開啟搬移後的實例。

硬連結只在一開始省空間。搬移後你移除或重裝的模組和下載,其舊副本會透過舊實例資料夾繼續
留在磁碟上,刪除 `~/.local/share/eidos/<game>/` 才能收回這部分空間。

可攜實例永遠不會被搬移:它留在你放的地方。

## 命令列

```sh
eidos games                                # supported games installed here, like MO2's list
eidos init skyrimse [folder]               # create an instance; with a folder it is portable
eidos init skyrimse --game-path <folder>   # use the copy installed in <folder> when several are installed
eidos install skyrimse mod.7z [name]       # install an archive (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # reinstall over the mod (--merge installs over its files)
eidos import skyrimse <mo2-profile>        # take over an MO2 profile's mod order and plugin state
eidos sort skyrimse [--dry-run]            # LOOT-sort the load order (--update-masterlist refreshes it first)
eidos play skyrimse                        # show what would be mounted
eidos play skyrimse -- <command>           # run <command> with the mods mounted over the game
eidos shell skyrimse                       # a shell in the merged view; writes land in Overwrite
eidos check skyrimse [--json]              # Health checks without the window
eidos sessions skyrimse                    # the recorded game and tool runs, newest first
eidos sessions skyrimse diff [<id>]        # what changed since the last run that exited with code 0
eidos stock-game skyrimse                  # which game files the instance runs; freeze a copy or switch
eidos tool skyrimse                        # list the tools (add, rm and run: see Tools below)
eidos tool skyrimse run --exe <path>       # run an exe once through the merged view
eidos tool skyrimse import-mo2 [--write]   # the executables of a ModOrganizer.ini; --write adds them
eidos prereqs skyrimse [--install]         # what the tools need; --install fetches what is missing
eidos collection skyrimse <link>           # install a Nexus collection (--dry-run, --no-optional)
eidos nexus status                         # check the stored Nexus sign-in
eidos nexus update skyrimse [--apply]      # check for mod updates; --apply installs them
eidos nxm <link>                           # download a Mod Manager Download link
eidos nxm --register                       # make the site's Mod Manager Download button open Eidos
eidos export skyrimse -o mods.csv --active # MO2's CSV format; --active lists enabled mods only
eidos pack skyrimse backup.eidos           # the whole instance in one file
eidos unpack backup.eidos [folder]         # without a folder a global instance goes back to its central place
eidos --version                            # print the version
```

不帶參數執行 `eidos` 會印出命令清單。多數命令在缺少參數時會印出自己的選項,
`eidos tool --help` 則列出 tool 的子命令。`eidos pack` 接受 `--dry-run`、`--level`、
`--no-downloads`、`--no-recovery` 和 `--force`,`eidos unpack` 接受 `--info` 和
`--force`;兩者都在[下文](#把一個實例搬到另一台機器)說明。`eidos stock-game` 在
[遊戲檔案](#遊戲檔案商店安裝還是副本)裡說明。

### 透過合併檢視執行遊戲

`eidos play` 會在一個私有命名空間裡,把實例的模組掛載到遊戲自己的 `Data` 目錄之上,然後
讓命令穿過那個檢視執行。遊戲自己的檔案會先被 bind-stash,所以常駐程式仍然讀得到原封不動的
檔案。寫入(存檔、重新產生的設定)落在實例的 `overwrite/` 層;遊戲安裝目錄與每一個模組
資料夾都逐位元組保持原封不動。

Eidos 不需要 root。它掛載在私有的 user + mount 命名空間裡,沒有 setuid 輔助程式、沒有系統
服務,也沒有什麼權限要授予。`eidos` 執行檔上的一個選用 capability 允許 FUSE passthrough,
它預設關閉,除非設定 `EIDOS_FUSE_PASSTHROUGH=1`;見
[install.md](install.md#選用fuse-passthrough)。帶著這個 capability 時,Eidos 取用的是單純
的 mount 命名空間而不是 user 命名空間,兩種方式部署模組的結果一模一樣。

### 合併檢視中的 shell

`eidos shell <instance>` 在遊戲所見的遊戲資料夾裡啟動你的 `$SHELL`(沒有就用 `/bin/sh`):每個已啟用的模組掛載在 `Data` 之上,Root 模組掛載在遊戲根目錄之上,寫到那裡的一切都進入 Overwrite。`eidos shell <instance> -- <command>` 則只執行一條指令,例如 `eidos shell skyrimse -- ls Data/meshes`,或 NifSkope 這類原生工具。這次執行像遊戲工作階段一樣持有實例鎖,所以在你結束之前視窗無法儲存變更,它在 `eidos sessions` 中以 `shell` 為名記錄,絕不算作一次遊戲啟動。對指令按 Ctrl+C 會像 Stop 一樣停止執行:Eidos 結束該指令、收集工作階段並卸載。在互動式 shell 中,Ctrl+C 照常屬於 shell;輸入 `exit` 結束執行。

### 從終端機執行健康檢查

`eidos check <instance>` 執行不需要視窗的 Health 檢查,因此可以透過 SSH 或在 Game Mode
下執行,結果也能放進錯誤回報:

- Proton 前綴及其登錄檔;
- 外掛診斷與缺少的 master;
- script extender 及其外掛;
- 自上次正常啟動以來的遊戲更新;
- Steam 啟動選項;
- 圖形設定(NVAPI、全螢幕、ENB);
- 上次執行的當機日誌;
- 設定檔 INI 裡的 `sTestFile` 行;
- Starfield 的 `.esp` 與 blueprint 外掛;
- `vm.max_map_count` 與開啟檔案數上限;
- 實例所執行的遊戲副本,以及清單 `ModOrganizer.ini` 裡指名另一個遊戲資料夾的
  `gamePath`;
- 不再成立的合集順序規則;
- 位於 NTFS、exFAT 或 FAT 上的遊戲檔案;
- BodySlide 的資料路徑;
- 留在 `Overwrite/Root` 裡的 `Data` 資料夾;
- `Interface/fontconfig.txt` 指名而沒有任何模組提供的字型庫;
- Fallout 4 與 Fallout 4 VR 模組的 BA2 封裝檔中的壓縮音訊。

有些卡片只有視窗會在 Diagnostics 分頁裡計算:修補程式順序、模組取代的擴充器腳本、沒有任何
外掛載入的封裝檔、Nexus 需求、LOOT 標記為 dirty 的外掛、執行遊戲的 Proton 版本、Display
卡片,以及比它的輸入還舊的工具輸出。

這個命令先印出與 Copy report 相同的系統資訊行,再為每項發現印出一行,家目錄顯示為 `~`,
登入名稱顯示為 `<user>`。沒有發現問題時結束碼為 0(只有建議仍為 0),至少發現一個問題時為
1,Eidos 無法檢查時為 2:參數錯誤、無法開啟的實例,或偵測不到的遊戲。

`eidos check <instance> --json` 改為印出一個 JSON 物件。最上層欄位是 `eidos`(版本)、
`game`、`instance`、`system`(`proton`、`flatpak_steam`、`passthrough`、`session`、
`distro`、`kernel`、`dev_fuse`、`fusermount3`、`max_map_count`、`open_files_hard`)與
`findings`。`max_map_count` 與 `open_files_hard` 是數字,Eidos 無法讀取時為 `null`。每項
發現由 `code`、`level`(`problem` 或 `advice`)和它的 `args` 組成;Diagnostics 分頁會顯示
每個代碼對應的句子。

### 自上次正常執行以來的變化

無論從視窗、終端機還是 Steam 啟動,每次透過 `eidos play`、`eidos shell` 或 `eidos tool run` 的執行都會在 `<instance>/logs/sessions/` 寫入一筆記錄:設定檔、每個啟用的模組及其版本和 Nexus 檔案 ID、外掛順序、每個設定檔 INI 的檢查碼、遊戲執行檔的版本、Eidos 版本、結束碼,以及這次執行建立的存檔。保留最新的 50 筆,`logs/` 不會進入打包的實例。`eidos sessions <instance> diff` 會把最近一次以結束碼 0 結束的遊戲執行與目前狀態比較;`diff <id>` 從清單中選另一筆記錄。清單把[測試執行](#測試執行)標為 `test`,測試執行絕不會被選為最近一次正常執行。被你停止的執行記錄的結束碼是 143,所以也絕不會被選中。Eidos 目前不偵測當機,在 Proton 下當機的遊戲仍可能以結束碼 0 結束:若選中的執行屬於這種情況,請用 `diff <id>` 指定更早的一筆。

## 模組清單

和 MO2 一樣,每個設定檔都有自己的 `modlist.txt`(哪些模組啟用、優先順序如何),而模組
資料夾則放在實例共用的 `mods/` 裡。你手動丟進 `mods/` 的資料夾還不在清單上:它會以停用、
最高優先順序的狀態出現,也就是視窗清單的最底部,和 MO2 一樣,所以一旦你啟用它,它就會
贏下它的衝突。

因為資料夾是共用的,移除或改名一個模組會改動每一個設定檔:它那一行會從每個設定檔的
`modlist.txt` 裡拿掉,或在其中改名,而其餘的部分每個設定檔都保有自己的狀態與順序。
一個不剩任何模組的設定檔完全沒有 `modlist.txt`。

視窗是從它自己那份模組清單與載入順序的複本在工作。如果在它開著的時候,另一個 Eidos 行程
寫入了其中任何一份(終端機裡的 `eidos install` 或 `eidos sort`、一份合集、一個由 Steam
啟動的工作階段、第二個視窗),視窗的下一次編輯會被拒絕,而不是把那些成果覆蓋掉。它會從
磁碟重新載入,並顯示 *Not saved: another Eidos process changed the mod list. It has been
reloaded - redo the change.*(載入順序的情況也一樣);損失的只有那一次點擊,所以再做一次
就好。Rename、New mod、Add separator、Remove,以及把 Overwrite 變成模組,都會在動到
`mods/` 之前先檢查,所以一次被拒絕的編輯不會改動磁碟上的任何東西。INI editor 也一樣:
檔案在它開啟之後若被改動過,它會拒絕把內容存回去覆蓋它。

## 存檔屬於設定檔

每個設定檔都把自己的存檔放在 `profiles/<name>/saves/`,而在一次執行的期間,那個資料夾會
掛載到 Proton 前綴裡遊戲的存檔資料夾之上。一個設定檔第一次啟動時,會把前綴裡既有的存檔
複製進去,只做一次,所以你之後從設定檔刪掉的存檔就會一直是刪掉的。每次執行之後,設定檔
最新的存檔會被複製回前綴,給 Steam Cloud 用;一份不是 Eidos 放進前綴的存檔,會先以
`orphan-<time>-<name>` 保存在設定檔裡,絕不會被覆蓋。

在那第一次複製之後、沒有經過 Eidos 就落進前綴的存檔(從 Steam Deck 來的 Steam Cloud 下載,
或一次沒有透過 Eidos 的啟動),在設定檔的存檔掛載著的時候會一直被遮住。沒有任何東西會刪掉
它:啟動時會在這次執行的日誌裡指名它(從視窗啟動是實例的 `logs/run-*.log`,從 Steam 啟動
則是 Steam 的主控台),那組檔案每變動一次就提一次,Saves 分頁也在標題下的那一行統計這些
存檔。要玩它,就在 Saves 分頁按下 Import。Import 會把每一個連同它的 co-save 複製到設定
檔裡,絕不取代設定檔裡的存檔:設定檔已經用了那個名稱的存檔,會以 `orphan-<time>-<name>`
匯入。

## 測試執行

Run 旁邊的 Test run 按鈕會像 Run 一樣啟動遊戲,但遊戲寫下的一切都會進入 `<instance>/test-run/`,而不是你的設定檔、Overwrite 和前綴:新的和改過的存檔、INI 變更、`plugins.txt`、寫進 Data 或遊戲資料夾的檔案,以及 `Documents/My Games/<game>` 裡的日誌。你可以用設定檔的模組測試,也可以不帶任何模組,或者只帶清單裡選取且已啟用的模組,加上提供它們外掛所需 master 的模組。

遊戲結束後,Eidos 會把這些檔案分成五組列出(存檔、INI 變更、外掛狀態、Overwrite 輸出、日誌),並逐組問你要保留還是丟棄。Apply 要等每一個有檔案的組都有了答案才會執行。當這次執行用的模組集合與設定檔的不同時,不會提供外掛狀態。保留存檔絕不會從設定檔刪掉任何存檔,即使是遊戲在執行中刪掉的那個。保留 Overwrite 輸出時,也會把遊戲刪掉的檔案從 Overwrite 裡移除,清單會先把它們計數。丟棄任何東西都要再按一次。Decide later 會關閉清單並把檔案留在 `test-run/` 裡,清單在你下次開啟這個實例或按 F5 時會再次開啟。如果 Eidos 或電腦在測試執行期間停下,檔案同樣會留在那裡,清單以同樣的方式開啟。

如果視窗不是由遊戲的商店啟動的,Test run 會像 Run 一樣把遊戲交給那個商店(Steam、Heroic 或 Legendary),商店啟動的那次執行就是測試執行。如果商店沒有啟動遊戲,這個請求就會等待:該設定檔的下一次遊戲執行,不論來自 Run 還是來自商店,就是測試執行,Cancel the waiting test run 會放棄它。測試執行只涵蓋上面列出的資料夾:遊戲的其他寫入,例如 Wine 登錄檔,照常寫到原處。測試執行的存檔要等你保留它們、再玩一次一般執行之後,才會進入 Steam Cloud。

### 找出造成問題的模組

Diagnostics 分頁上的 Find the mod behind a problem 會尋找造成某個可重現問題的模組,例如在某個地點當機。每一步都是一次測試執行,帶上剩下嫌疑模組的一半、它們需要的 master,以及你一直開著的模組:開始時在清單中選取且已啟用的模組,和提供 script extender 的模組。每次執行後,你回答問題是否出現;Not sure 會換一種方式分割嫌疑模組,Run it again 會重做這一步。回答或按 Run it again 會在再按一次後丟棄這一步寫下的檔案。想保留其中一部分,在回答之前先按 See the files:它會開啟與測試執行之後相同的清單。如果遊戲當機且某個崩潰記錄器寫了日誌,或者遊戲以錯誤碼結束,Eidos 會把 Yes 標為可能的回答。200 個嫌疑模組大約需要 8 次執行。

搜尋保存在 `<instance>/bisect.json` 裡,所以你可以在兩步之間關閉 Eidos,而且它屬於開始它的那個設定檔。Stop the search 在再按一次後放棄這次搜尋。結果會指出那個模組,或一組彼此需要對方 master 的模組,並保存在 `<instance>/logs/bisect-<time>.txt`。產生的模組(Nemesis 或 DynDOLOD 的輸出、Bashed Patch)是為完整清單建置的,只用一半時可能出錯:讓它們一直開著,或在整個搜尋期間都關著。

## Wabbajack 清單

Eidos 沒有 Wabbajack 安裝程式。請在 Eidos 之外用 CLF3 或 Jackify 安裝清單,然後在精靈中把安裝資料夾選為可攜實例。Eidos 依 MO2 的配置讀取該資料夾:`mods/`、`profiles/<name>/modlist.txt` 和外掛清單。安裝程式寫入的 `ModOrganizer.ini` 會被保留,絕不會被覆寫,因此清單自帶的執行檔仍維持註冊。

許多清單會加入一個 `Stock Game` 或 `Game Root` 資料夾,裡面是修改過的遊戲副本,MO2 從那裡啟動遊戲。精靈或 `eidos init` 第一次接管清單資料夾,而該資料夾裡有遊戲執行檔時,實例也從這個資料夾執行遊戲。遊戲仍然透過 Steam 或擁有它的商店啟動,所以什麼都不用搬動。Eidos 依清單 `ModOrganizer.ini` 裡的 `gamePath`,或依這兩個名稱找到該資料夾。用較早版本的 Eidos 接管的實例會繼續使用商店安裝,直到你切換為止(見下一節)。當那個 `gamePath` 指名了實例不執行的遊戲資料夾時,Diagnostics 會說出來,因為以 Mod Organizer 2 模式執行的工具(例如 PGPatcher)會從那裡讀遊戲。CLF3 用自己的工具重建 BSA 和 DDS 檔案,所以結果可能與 Wabbajack 安裝不同,清單作者也不提供支援。

## 遊戲檔案:商店安裝還是副本

預設情況下,遊戲從它的商店安裝執行,也就是 Steam、Heroic 或 Legendary 存放它的資料夾。實例也可以改為從放在實例內的該資料夾副本執行。模組疊在副本之上,商店安裝只是 Eidos 掛載結果的位置。遊戲仍由商店啟動,所以 Steam 保留它的覆蓋介面、著色器快取和雲端存檔。

Diagnostics 分頁頂端的 Game files 卡片顯示實例執行的是哪份檔案,附上副本與商店安裝的遊戲版本,並在兩者之間切換。在終端機裡,`eidos stock-game <instance>` 印出同樣的資訊,`use <folder>`、`store`、`freeze` 和 `delete --yes` 做的事與卡片的按鈕相同。`freeze` 會印出大小和副本的種類,`freeze --yes` 才會複製。

Freeze this version 把商店安裝複製到 `<instance>/Stock Game`,並從那裡執行遊戲,這樣商店更新就不會再弄壞 script extender 外掛:更新改的是商店安裝,不是副本。第一次按下會顯示大小以及檔案系統會怎麼處理它,第二次按下才複製。在能共用資料區塊(reflink)的檔案系統上,例如 btrfs 或 XFS,副本在商店改動某個檔案之前幾乎不佔空間。在 ext4 上,或者遊戲和實例位於不同檔案系統時,它是一份和遊戲資料夾一樣大的完整副本。Eidos 會在這些情況下拒絕:Steam 回報遊戲忙碌或尚未完整安裝、實例的遊戲或工具正在執行、實例已經持有清單的遊戲副本、凍結的副本或未完成的副本,以及檔案系統的可用空間少於完整副本所需。Heroic 和 Legendary 的副本不會檢查是否有更新正在進行。Stop、失敗,或複製期間 Steam 更新了遊戲,都會留下 `Stock Game.eidos-tmp`,卡片會提議刪除它;這個資料夾永遠不會被使用。

副本會遮蔽商店在複製之後於遊戲資料夾新增、修改或刪除的每個檔案:新的 Creation Club 檔案、新的執行檔、新的 DLL。遊戲、外掛清單、LOOT 和 Health 檢查讀取的都是副本。它遮蔽不了遊戲資料夾之外發生的事:更新仍可能在下次啟動時向 Proton 前綴安裝執行階段,Steam 也可能把遊戲切換到另一個 Proton。

Return to the store version 在第二次按下後把副本從下次啟動中移除。副本留在磁碟上,Delete the copy 會刪除它,同樣需要按第二次。實例仍在執行該副本時,Delete 會被拒絕。Eidos 從不刪除清單自帶的 `Stock Game` 或 `Game Root` 資料夾。如果副本資料夾不見了,或裡面不再有遊戲的執行檔,啟動會被拒絕,Diagnostics 會說出該資料夾的名稱,所以商店版本永遠不會在意料之外執行。

Morrowind,以及 `bUseMyGamesDirectory=0` 的 Oblivion,會把外掛清單放在遊戲資料夾裡,Eidos 目前還不從副本執行它們。

## 安裝一份 Nexus 合集

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # read it and say what would happen
eidos collection skyrimse rqhcxy --no-optional  # skip the members it marks optional
```

在視窗裡,把連結貼進 File -> Open a Nexus collection...,或在網站上點一條。Eidos 會列出
該修訂版的成員,每一個都和這個實例對照過:已安裝、已下載,或缺少。*Install this
collection* 會在工作執行緒上跑完整張食譜,用的是打包與解包工作用的同一條進度列,做完時
那條進度列就變成報告。視窗和 `eidos collection` 執行的是同一個安裝。

沒有連結時,File -> Open a Nexus collection... 會開在 Browse 分頁上。它列出為目前開啟實例的遊戲所發布的合集,每次 20 個,依下載最多、最近更新或最新排序,搜尋框會把清單縮小到名稱或內文相符的合集。每一列列出作者、最新修訂版、模組數、下載次數和一個日期。View 會在 Link 分頁上開啟該合集的最新修訂版,和貼上它的連結一樣。標記為成人內容的合集會被略過,除非你的 Nexus 帳號顯示成人內容;沒有分級的合集一律略過,清單下方那一行會把兩者都計入。瀏覽需要已登入的 Nexus 帳號,和這個面板的其他部分一樣。

一份合集只能對照它自己的遊戲來開啟。載入 Fallout 4 實例時開啟一份 Skyrim 合集,它會指名拒絕,而不是把成員和錯誤的模組清單對照。

### Eidos 從合集讀取什麼

一份合集幾乎沒有哪個部分能從 Nexus API 看得到。安裝順序、作者給每個模組的腳本安裝器的那些答案、檔案衝突時哪個模組勝出、哪些外掛載入到哪裡、那些二進位修補,全都在合集自己的壓縮檔裡的一份 `collection.json` 中。下載同樣那些模組、再用它們的預設選項安裝,做出來的是另一個遊戲。

Eidos 會下載那份壓縮檔,讀出 `collection.json`,然後把它套用上去:

| 合集說的 | Eidos 做的 |
| --- | --- |
| `phase` | 依那個順序安裝,選用的成員排在最後 |
| `choices` | 重播作者對每一個 FOMOD 給的答案,做不到時就直說 |
| `modRules` | 重新排列模組清單,讓該贏的模組贏下每一個檔案 |
| `plugins` / `pluginRules` | 把 LOOT 規則合併進你的 userlist 並排序 |
| `INI Tweaks/` | 把它們裝成一個自成一格的模組 |
| `tools` | 告訴你它預期哪些工具;什麼都不建立 |

成員會在安裝的當下就被啟用,以安裝順序排在模組清單的最後面,在 `modRules` 把它們搬動之前。
沒有列進模組清單的成員不會被載入,而報告也不會顯示它。

狀態會在每一個成員之後寫進 `<instance>/collections/<slug>-<revision>.state.json`。它就在那個修訂版的資料夾旁邊,絕不在它裡面,這樣一份合集就沒辦法夾帶自己的狀態檔,所以一次在 200 個成員中的第 147 個被中斷的安裝,會從 147 接著跑。它會先以一個暫時的名稱寫出,再改名到定位,所以一次中斷留下的不是舊的那份紀錄、就是新的那份,絕不會是任何一份的一半。萬一它讀不出來,Eidos 會停下來直說,而不是把整份合集從頭再跑一遍。再執行一次同樣的命令就好。

安裝一份你已經有的合集的較新修訂版時,會接手上一個修訂版的資料夾:沒有變動的成員會就地驗證,作者更新過檔案的成員會在它原本的資料夾裡被替換,而新修訂版拿掉的成員會維持已安裝,並在報告裡被指名,讓你去停用或移除。另一個設定檔啟用中的資料夾絕不會被接手,因為 mods 資料夾是共用的,那個設定檔會在你背後被改掉:新修訂版會改為在它旁邊裝一份自己的複本,而報告會請你在這個設定檔裡停用舊的那一份,絕不會要你移除它。退回較舊的修訂版時,會以同樣的方式把它的資料夾接手回來,接手不了的那些就裝上全新的複本。每當這樣替換一個成員,而你曾在它的資料夾裡改過或加過檔案時,舊資料夾會先以 `<name>_backup` 的名稱留在旁邊,它絕不會被載入遊戲,報告也會指出它的名稱。

一個來自較早修訂版、仍然啟用著的殘留項目,會讓這次安裝無法算作作者建置的那一套;你停用它之後,那則說明就會消失。你用 `--no-optional` 略過的成員不會被接手,並以同樣的方式被指名。如果接手本身失敗,這次執行會停下,顯示 *previous revision: its folders could not be taken over* 以及原因;把那個問題修好,再執行一次那道命令。

### 免費帳號與你自己抓取的成員

免費的 Nexus 帳號抓不到那些成員模組。Nexus 不會透過 API 為免費帳號提供下載連結:每一個檔案都需要網站自己的「Mod Manager Download」按鈕。合集壓縮檔本身用免費帳號可以下載,所以你讀得到整張食譜,但每一個成員都會連同它確切的頁面 URL 一起回報,讓你自己去抓,重試也不會改變這一點。透過網站的按鈕把它們抓下來,再執行一次那道命令:每一次執行都會到 `downloads/` 裡找那些成員,所以你補到哪裡,合集就完成到哪裡。

在視窗裡,免費帳號在合集面板中看到的是 *Fetch the N missing required files, one by one*,取代分批抓取。它會在 Nexus 上打開第一個缺少的必要檔案的頁面,等那個檔案落進 `downloads/`,再打開下一個。每個檔案頁面上的下載按鈕仍然由你自己按。面板會顯示進度(*3 of 12*),並提供 *Skip* 和 *Stop*。如果落進來的是同一個模組的另一個檔案,面板會寫出兩個檔案 id,並繼續等待合集指定的那一個。選用成員不在其中,保留各自的 *Open* 按鈕。

Eidos 也會套用二進位修補(每個都依記錄的 CRC 校驗)、檔案覆寫(指定的檔案像 MO2 那樣用 `.mohidden` 隱藏),以及那些檔案隨著合集壓縮檔一起走的成員(`bundle`)。合集預期你親手去抓的成員(`browse`、`manual`)會把作者自己的說明一路帶進報告裡。名稱已經和你某個模組撞上的成員,會以一個沒被占用的名稱裝在它旁邊,絕不覆蓋它,而報告會說明是哪幾個。

### 那份報告

一個安裝器答案沒能全部重播完的成員,會自成一行 *installed, but not the way the collection asks*,和照作者建置方式安裝的成員分開,因為磁碟上的檔案和作者的不一樣。報告也會列出在合集裡沒有指到任何東西的排序規則、卡在一圈互相矛盾的規則裡的模組、不存在的 LOOT 群組,以及這個版本的 Eidos 看不懂的 manifest 的任何部分。

### 保持合集順序

Eidos 在安裝之後仍會保留每個已安裝合集的規則。如果你之後移動其中兩個模組,違反了一條前後順序規則,Diagnostics 分頁會為該合集顯示一張卡片,列出第一對這樣的模組,並統計不再成立的規則數量。只計算目前設定檔中啟用的模組;陷入循環的規則會被略過,因為沒有任何順序能滿足它們。Restore collection order 會把該合集的模組移回滿足每條規則的順序,在規則允許的地方保留你自己的順序,不屬於該合集的模組則留在原位。Ctrl+Z 可復原此操作。`eidos check` 會以 `collection_order_broken` 列出同一項發現。此檢查需要 `collections/` 中該修訂版的配方,因此快取配方已被刪除的合集不會被檢查。

## 把一個實例搬到另一台機器

`eidos pack` 會把一整個實例(每一個模組、載入順序、所有設定檔、裝著你存檔的 Overwrite,
以及當初安裝這一切所用的壓縮檔)寫進單獨一個檔案。`eidos unpack` 會把它放回去:

```sh
eidos pack skyrimse ~/backup.eidos                  # or give a folder: named for the game and the date
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # on the other machine
```

`eidos pack --dry-run` 會準確列出哪些東西會進去、哪些不會以及為什麼,還有它需要多少空間,而不寫出任何東西。`eidos unpack --info` 對你手上已經有的檔案做同樣的事。

如果 7-Zip 有什麼讀不到,或是走訪目錄時不得不略過某個你會預期出現在備份裡的東西(一條符號連結、一個讀不到的資料夾、一個無法安全交給 7-Zip 的名稱),壓縮檔還是會以你選的名稱寫出來,但 `eidos pack` 會以 1 結束並說明缺了什麼,所以執行 `eidos pack ... && ...` 的腳本會停在那裡。

一個 `.eidos` 檔案就是一個 7-Zip 壓縮檔,只是掛著自己的副檔名。安裝模組本來就需要 7-Zip,所以這不多添任何相依,而且備份用任何讀得了 7-Zip 的解壓縮程式都打得開,不需要 Eidos。它用 LZMA2 以 `-mx1` 壓縮,在一份材質很多的清單上大約是原始大小的 43%;`-mx9` 花的時間長得多,只多省幾個百分點。材質與網格本來就是已經壓縮過的格式,所以更大的字典幾乎再也找不到什麼可壓縮的東西。`--level`(0、1、3、5、7 或 9)可以選別的等級,`--no-downloads` 會把 `downloads/` 排除在外。

在預設的等級 1 下,檔案會放進最大 16 MB 的固實(solid)區塊裡。從一份 70 GB 的備份裡取出一個檔案,最多只需解碼它所在的那個區塊,絕不會把排在它前面的東西全部解開;打包遊戲內容也比每個檔案各自壓縮快 2 到 5 倍,解包快 2 到 4 倍。代價要在損壞時才付:一個壞掉的位元組可能連帶它所在區塊裡的所有檔案,最多 16 MB 的相鄰檔案。下文介紹的修復資料就是用來應付這種情況的。其他等級都維持非固實:每個檔案各自是一個資料流,一個壞位元組只會損失這一個檔案。等級 0 只儲存檔案。從等級 3 起,LZMA2 本身就以 16 MB 或更大的分段運作,固實區塊在測量中反而更慢:在等級 5,打包網格要花 1.4 倍的時間,解包材質要花 6.8 倍。

打包會取得實例鎖,所以遊戲或另一個 Eidos 正在寫入那個實例時它跑不起來,而且讀取期間沒有任何東西會改動實例。備份會先以 `.part` 名稱寫出,確實寫入磁碟,再用 `7z t` 從頭到尾讀回一遍,通過後才改成你選的名稱。這次檢查會解碼每個區塊並比對每個校驗和,和在另一台機器上解包時做的工作相同。它說明的是寫完之後磁碟立刻讀回的內容,而不是這顆磁碟日後還能保住它;那是下文修復資料的用途。如果檢查失敗,`.part` 會被刪除,你選的名稱底下不會寫入任何東西,`eidos pack` 會引用 7-Zip 找到損壞的那一行。一次被中斷或失敗的打包留下的是一個 `.part` 檔案或什麼都不留,而不是一個看起來完整、實際上不是的 `.eidos` 檔案。

裝了 par2cmdline 時(Arch 上 `sudo pacman -S par2cmdline`,Debian 與 Ubuntu 上 `sudo apt install par2`,Fedora 上 `sudo dnf install par2cmdline`),`eidos pack` 還會在壓縮檔旁邊寫出 PAR2 修復資料:`backup.eidos.par2` 和幾個 `backup.eidos.volNN+MM.par2` 檔案,總共是壓縮檔的 5% 再加上一個小索引。請把它們以寫出時的名稱和 `.eidos` 檔案放在同一個資料夾,並且一起複製:沒帶著它們搬走的備份就沒有東西可以拿來修復,而且修復資料記下了壓縮檔的名稱,改過名的備份就和它們對不上了。`eidos unpack` 會依名稱找到它們,在解出任何東西之前先用它們核對壓縮檔,結尾被截斷、7-Zip 已經打不開的備份也一樣。如果有幾個位元組壞了,par2 會先修好壓縮檔,並把損壞的副本保留為 `backup.eidos.1`,為此壓縮檔所在的資料夾裡需要有和壓縮檔一樣大的可用空間;超出這 5% 能彌補的損壞,會讓解包在寫出任何一個檔案之前就停下。和壓縮檔對不上的修復檔案,例如同名的舊備份留下的那些,在 7-Zip 讀完壓縮檔並確認它完好時不會被使用,解包會說明這一點。在沒有 par2 的機器上解包照樣進行,但會警告修復資料沒有被用上。`--no-recovery` 打包時不寫修復資料,`eidos pack --dry-run` 會說明是否會寫。`--force` 會取代同名的既有檔案。

### 裡面沒有哪些東西,以及為什麼

| 未納入 | 為什麼 |
| --- | --- |
| Proton 前綴 | 它的形狀取決於這台機器(磁碟機對應、對*這個*檔案系統的 `Z:` 檢視),而 Eidos 用 `eidos prereqs <instance> --install` 一分鐘就能重建它。帶著它會讓壓縮檔大約大上一倍,只為了運送一個另一台機器反正得重新產生的東西。 |
| 遊戲本身 | 一個實例改的是一款它並不包含的遊戲。在那邊從 Steam 安裝它。 |
| 實例之外的工具 | xEdit、DynDOLOD 以及同類工具是第三方執行檔,有自己的安裝程式。它們的名字會記在壓縮檔裡,所以解包時會準確告訴你新機器上缺了哪幾個。 |
| `logs/`、`.eidos.lock`、`prereqs.done`、`prereqs.log` | 這些是做出這份備份的那台機器的紀錄。尤其是 `prereqs.done`,它會宣稱執行階段函式庫已經裝在一個根本不存在的前綴裡。 |
| `loot/`,但 `userlist.yaml` 除外 | 一份 masterlist 快取,Eidos 需要時會重新抓。你自己的 LOOT 規則放在 `userlist.yaml` 裡,沒辦法重新抓,所以那一個檔案會跟著一起走。 |
| `.base/`、`.base-root/` | 空的掛載點,工作階段期間遊戲自己的檔案被暫存在這裡。 |
| `test-run/`、`bisect.json` | 等待檢視的[測試執行](#測試執行),以及兩步之間的「找出造成問題的模組」搜尋。兩者都屬於這台機器上的執行。 |
| 寫到一半的檔案 | 暫停中的下載(`*.unfinished`)、進行到一半的原子寫入(`*.eidos-tmp*`)。 |
| 符號連結 | 7-Zip 會跟著它走,把它指向的東西複製進來,而對一條絕對路徑的連結來說,那等於把一棵不相干的目錄樹拉進你的備份。改成回報它們,而且這份備份會被算作不完整:連結到另一顆硬碟的 `mods/` 或 `downloads/` 不會被打包。如果你本來就打算不帶那個連結過去的 `downloads/` 來備份,就加上 `--no-downloads`(或在視窗裡取消勾選 *Include downloads/*);刻意的排除不算缺少。 |

實例中的遊戲副本(`Stock Game`)和其他資料夾一樣會被打包,所以凍結實例的備份大約會多出一個遊戲的大小。未完成的副本(`Stock Game.eidos-tmp`)不會被打包。

以上每一項都會連同它的原因寫進壓縮檔根目錄的 `eidos-backup.ini`,所以讀那個檔案就知道少了什麼。同一個檔案也記下了實例原本住在哪裡、當時啟用的是哪個設定檔、解開後有多大,以及新機器會需要哪些工具。

下載紀錄(`downloads/*.meta`)會被納入,但它們的 `url=` 那一行會先被清空。那條 URL 是一條簽章過的 Nexus 連結:它幾個小時內就會失效,而且它帶著下載那個檔案的帳號的 `user_id`,而備份可能會交給別人。模組 id、檔案 id 和版本都留著,所以那份下載仍能被重新找回來。

### 在視窗裡

兩者都在 File 選單裡:*Pack this instance...* 與 *Unpack a backup...*。解包也放在歡迎畫面上,就在實例清單下面,所以還沒有任何實例的機器也能解包出一個。

在寫出任何東西之前,打包對話框會顯示有多少個檔案、多大、其中有多少是 `downloads/`、壓縮檔大概會變成多大,以及有哪些工具只被記下名字而沒有被帶走。它的 *Write recovery data* 核取方塊在裝了 par2 時是勾選的,沒裝時則呈灰色,並寫明要安裝的套件。解包對話框會在你選好檔案的那一刻就讀取備份的 manifest,所以在你選資料夾之前,它就能告訴你這份備份裝的是哪款遊戲、是什麼時候做的、原本住在哪裡,以及它的工具有哪幾個在這裡是缺少的。

兩者都在工作執行緒上執行,帶著進度列,做完時那條進度列就變成報告。

### 解包會修好什麼

單純複製那個資料夾,會讓每一個工具按鈕都還指著舊機器上的路徑。`eidos unpack` 只改寫那些指名舊實例根目錄的值,而且只改那些:`tools.ini` 裡的 `exe`、`workdir` 與帶編號的 `arg` 鍵,以及每個模組 `meta.ini` 裡的 `installationFile`。從來就不在實例裡面的工具(`/opt/xedit/SSEEdit.exe`)會原封不動留著,並被列為需要重新安裝的東西;Eidos 不會去猜那個工具在新機器上的哪裡。它也會更正實例自稱是集中式還是可攜式,好讓後續的命令到它現在所在的地方去找它。

解包會拒絕這些情況:資料夾不是空的(如果你確實想要覆蓋,加上 `--force`)、磁碟空間小於 manifest 自己給的數字、壓縮檔是由比你新的 Eidos 做出來的,以及任何含有會被寫到你所選資料夾*外面*的項目的壓縮檔。沒有給資料夾時,全域實例的備份會回到該遊戲的集中位置;可攜實例的備份則需要給一個資料夾。

之後:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # rebuild the Proton prefix
eidos play /mnt/games/EidosSkyrim
```

## 視窗

啟動 `eidos-gui`,或 Colony 安裝的 `eidos-linux` 檔案。

首次啟動會開啟一個 MO2 風格的精靈:歡迎、實例類型(可攜或全域)、遊戲、名稱與位置、摘要,然後是主視窗。歡迎畫面也會列出每一個已知的實例(全域與可攜,最近使用的在前),一鍵開啟,所以它兼任實例切換器。把精靈指向一個已經放著實例的資料夾,會原樣開啟那個實例,而不是在上面建立新的;屬於另一款遊戲的資料夾會被拒絕。

主視窗有兩個窗格:模組清單,以及 Data、Plugins、Conflicts、Overwrite、Archives、Saves、Downloads 和 Diagnostics 分頁,還有一個 Run 按鈕、一個 Test run 按鈕([測試執行](#測試執行))和一個執行目標選擇器。你可以篩選模組清單、選取、重新排序、用分隔線分組、依類別縮小範圍並右鍵操作。除了移到最上/最下之外,模組選單還有 MO2 那些指定目標的移動:移到第一個衝突模組之上、最後一個之下、移到指定的優先順序,或移進某個分隔線的群組。

設定檔選擇器用來切換設定檔。在設定檔上按右鍵可以 Rename、Copy to new...、Desktop shortcut、Import from MO2... 和刪除,刪除要點兩次,並且拒絕刪除目前使用中的或最後一個設定檔。Import from MO2... 會接管一個 MO2 設定檔的模組順序與載入順序。Desktop shortcut 會寫入一個桌面項目,執行 `eidos tool <instance> run "<game>" --profile <name>`:它把那個設定檔設為目前的設定檔,並請商店啟動遊戲(細節見 [tools.md](tools.md#找到工具隱藏置頂以及桌面捷徑))。

File 選單也會在遊戲的前綴裡執行 winecfg、regedit、Wine 檔案總管和解除安裝程式,Prefix drive_c 會開啟前綴的 `C:` 磁碟;見 [tools.md](tools.md#wine-自己的程式在遊戲前綴裡)。*Instances...* 列出 Eidos 認識的實例;Forget 會把其中一個從清單裡拿掉,磁碟上不動任何東西。*Export the mod list...* 會以 MO2 的 CSV 格式寫出清單,和 `eidos export` 一樣。

### 偏好設定

點左上角的 Eidos,偏好設定就會取代兩個窗格開啟;再點一次(或點 Close)即可返回。最前面是 General、Appearance 與 Accessibility,和每個 Colony 程式一樣,接著是 Mod list、Nexus 與 About。每項變更都會立即套用並儲存;沒有儲存按鈕。

- General:Eidos 開啟的遊戲、視窗大小、介面語言、工具資料夾,以及遊戲的執行方式。Lock the window while a game runs 預設開啟。Start the game at once when a store opens Eidos,以及視窗在背景時告知長時間工作結束的桌面通知,預設都是關閉的。視窗介面支援英文與法文;System 會依循 `LC_ALL`、`LC_MESSAGES` 或 `LANG`,變更立即生效。`eidos` 命令列一律以英文回應。
- Appearance:主題(26 個系列、59 套配色,預設為 Eidos 的羊皮紙)、強調色、文字大小、效果,以及即時預覽。
- Accessibility:高對比、適合閱讀障礙者的字型、減少動態效果,以及一個與 Appearance 文字大小相乘的閱讀文字大小。
- Mod list:把模組拖到清單頂端或底端邊緣時的自動捲動速度。
- Nexus:登入、下載資料夾、`nxm://` 處理器及其 Register 按鈕、偏好的伺服器、Install downloads when they finish,以及 Offline mode。
- About:檔案放在哪裡、鍵盤快速鍵、版本,以及版本檢查。

外觀與介面語言存放在 `~/.config/Colony/Eidos/preferences.json`。其他設定(命令列也會讀:預設遊戲、工具資料夾、Nexus 伺服器、Offline mode 等等)存放在它旁邊的 `settings.ini`,Nexus 工作階段則在 `nexus.ini`。舊版 Eidos 存在 `settings.ini` 裡的主題,會在第一次啟動時沿用過來。

About -> Version 裡有版本檢查,預設開啟。啟動時,每天最多一次,Eidos 會向 GitHub 詢問 `Project-Colony/Eidos` 的最新版本。如果它比正在執行的這一份新,選單列會在 Help 後面顯示 `Eidos <version> is available: release notes`,點這個連結會開啟該版本的頁面與它的變更記錄。Eidos 從不自己下載或取代自己。離線模式下不做這項檢查。由套件管理員裝在 `/usr` 下的副本不做檢查,因為由套件管理員負責更新;`install.sh --system` 放置 Eidos 的 `/usr/local` 算作你自己的安裝。

### 欄位、排序與分組

清單預設顯示四個欄位(Category、Content、Version 和 Flags,即 MO2 顯示的那些),總共提供八個:Category、Content、Version、Author、Installed、Nexus id、Game、Flags。在 View 選單裡勾選它們。八個全開時,名稱欄就剩不了多少空間。

點任一欄標題就依它排序,再點一次反向,第三次點回到載入順序。清單只有在載入順序下才能拖曳:放下的位置定位的是真實的清單,而排序後一列在畫面上的位置並不是它在清單裡的位置。排序開著的時候,不會畫出插入條,拖曳會被拒絕,和 MO2 一樣。View 選單會說明這一點,並提供回去的辦法。

View 選單也可以把整份清單分組,依類別或依來源(來自 Nexus,或手動安裝)。群組標頭不是分隔線:它們不能改名、上色或移動。它們會摺疊,摺疊時計數留在標頭上。套上排序或分組時,分隔線會被隱藏,因為分隔線帶領的是載入順序中跟在它後面的那些列,而這兩者都改變了那些列在畫面上的順序。

### 滑鼠與鍵盤

雙擊模組開啟 Information,Ctrl+雙擊開它的資料夾,Shift+雙擊開它的 Nexus 頁面。Ctrl+F 把游標放進篩選框。按下一個字母會跳到下一個以它開頭的模組,再按一次會繼續走完其餘的,而不是卡在第一個。它們都不會落在被篩選、被摺疊的分隔線或被摺疊的群組藏起來的列上,所以下一次按 Space 絕不會切換到一個你看不見的模組。

分隔線選單上的「Collapse others」會把除了那一個以外的每個群組都摺起來。拖曳過程中,停在一個摺疊的群組上會把它展開,所以模組可以直接丟進去;只是掠過則不會。

Ctrl+Z 復原對模組清單或載入順序的上一次變更:一次勾選、一次移動、一次批次停用、一次 LOOT 排序。Ctrl+Shift+Z 重做。視窗保留 50 步,在按 F5、遊戲或工具執行結束、切換設定檔以及另一個 Eidos 行程變更清單時清除。安裝、重新命名或移除模組會改變磁碟上的資料夾,所以越過這類操作的復原會被拒絕。

### 篩選清單

Filters 按鈕為每個條件列出一列:已啟用、有衝突、有更新、無類別、已在 Nexus 上推薦(endorsed)、已追蹤(tracked)、有 Nexus id、有隱藏檔案、沒有有效的遊戲資料、給另一個遊戲用、有備註,以及每種內容一列(外掛、BSA 或 BA2 封裝檔、貼圖、網格、腳本、script extender 外掛、介面檔案、音效、FaceGen)。每點一下,該列就依關閉、「only」、「except」的順序切換。最上面一列在「符合所有已開啟的條件」與「符合其中任何一個」之間切換。名稱框與類別清單一律會再縮小結果。隱藏檔案這一列只看已啟用的模組。

### 清單會告訴你關於一個模組的什麼事

兩個提示性旗標,都是一個字符,滑鼠停留就有說明。No valid game data 表示模組最上層沒有任何東西看起來像這個遊戲會載入的內容;它可能需要把資料夾往上移一層,也可能根本不是這個遊戲的模組。Another game 表示模組自己的 `meta.ini` 指名了另一款遊戲。兩者都不會擋住任何事:模組照樣部署。列選單上的「Mark as valid」可以透過 MO2 自己的 `validated=` 鍵讓其中任一個安靜下來,所以在一個管理器裡標為有效的模組,在另一個裡不會被標記。

這個版面檢查只在確定時才標記模組:一棵 `Root/` 樹、一個讀不到的資料夾和一個空的資料夾,都算作有效的遊戲資料。

### 停用存檔或其他模組用到的模組

在停用或移除一個模組之前,Eidos 會讀取該設定檔最新三個存檔的外掛列表。只要其中一個存檔用到了這個模組裡的外掛,列表上方就會出現一條提示列,列出這些外掛,並等待你按「Disable anyway」或「Cancel」(按 Esc 也會取消)。Remove 會把同樣的外掛名稱加進它要求再點一次的訊息裡。只讀取 Skyrim 和 Fallout 4 的存檔;其他遊戲不詢問,直接停用。

模組的 Information 裡的 General 分頁會列出它需要的模組和需要它的模組,依據已啟用外掛的 master 讀取。停用其他模組需要的模組時會出現同樣的提示列:「Disable them too」會同時停用所有需要它的模組,以及需要那些模組的模組;「Keep them enabled」則只停用所選的。Diagnostics 分頁上的缺少 master 卡片有 Enable what provides them:它會啟用包含每個缺失 master 的模組,並勾選存在但未勾選的 master 外掛。

### 動它之前先備份一個模組

「Back up this mod」會把它的資料夾另存一份為 `<name>_backup`(接著是 `_backup2`,依此類推;備份永遠不會取代上一份)。這份複本不是模組:它的核取方塊什麼也不做,對合併檢視也毫無貢獻,因為勾選它等於把同一個模組的兩份複本疊著部署。「Restore this backup over the mod」會把它放回去,兩次點擊;目前的內容會先被移到一旁,等複製成功之後才丟棄。

工具列上的 Backups 為設定檔的模組清單、載入順序(`plugins.txt` 和 `loadorder.txt` 一起)和它的遊戲 INI 各保留 10 個還原點,放在 `<profile>/backups/`。Back up now 會新增一個,Restore 會先備份目前的狀態,所以選錯了也能復原。Eidos 也會在每次 BethINI 執行之前保留一個 INI 還原點(見 [tools.md](tools.md#bethini-pie))。

### 分頁

Data 是合併檢視的一棵樹,一次展開一層:打開一個資料夾,讀的是每個擁有它的層裡的那個資料夾,絕不會一次讀遍每個已啟用的模組。它建立在掛載本身服務所用的同一套層堆疊上,所以 whiteout 與隱藏檔案都被遵守,這個分頁顯示的就是遊戲將會看到的東西。可以依名稱篩選、縮小到只看有爭議的檔案、用 Size 與 Modified 欄位理清什麼在哪裡,還能把任一列在檔案管理器裡 Reveal 出來。滑鼠停在檔案上會列出其他也帶有它的模組。模組最上層的 `meta.ini`、`readme.txt`、`fomod/` 與 `Root/` 不會進入 Data,這個分頁和掛載裡都一樣:前三個是給模組管理器用的,`Root/` 則改放到遊戲資料夾。遊戲或工具以這些名稱寫入的檔案會落在 Overwrite,並保持可見。

Data 裡的 `.exe` 列有 Add tool 和 Run 兩個按鈕。Add tool 會開啟 Executables 編輯器並填入該檔案(按下 Save 之前不會儲存任何內容);Run 會像已註冊的工具一樣,透過合併檢視把它執行一次([工具](#工具))。Save tree 會把合併後的整個 Data 樹寫入文字檔,每個檔案一行,並註明提供它的模組,其中已移除你的家目錄與使用者名稱,可用於錯誤回報。

Plugins 是 ESP/ESM/ESL 的載入順序:切換、手動重新排序,或用 LOOT 排序並閱讀排序後的報告,報告裡的建議連結會在你的瀏覽器開啟。它的標題列依索引空間統計已啟用的外掛數量,並與遊戲的上限對照,例如 `full 231/254, light 512/4096`,某個空間滿了就變成橙色。篩選框依名稱縮小清單。Space、Ctrl+A、Shift 範圍選取,以及列選單的 Activate all shown 和 Deactivate all shown,只作用於顯示出來的列,而在清除篩選之前,拖曳會被拒絕。選取一個外掛時,它的 master 會在外掛清單中被著色,提供它的模組則在模組清單中被著色。被 LOOT 標記為 dirty 的外掛會有一張 Diagnostics 卡片,可以用 xEdit 清理它們([tools.md](tools.md#清理-loot-標記為-dirty-的外掛))。

LOOT 排序之後,新紀錄全部落在 light 範圍內的完整外掛會在滑鼠停留時說明這一點,它那一列的選單裡也會出現 Flag as light。Eidos 會把帶 light 旗標的副本寫進一個名為 `<plugin> (light)` 的新模組,放在模組清單最底部(優先順序最高),原外掛不動,所以停用這個模組就能復原。如果該設定檔最新三個存檔中有一個把這個外掛當作完整外掛載入,Eidos 會拒絕。只會讀取 Skyrim 和 Fallout 4 的存檔,所以在 Starfield 上不會檢查任何存檔。LOOT 只檢查紀錄範圍:會新增儲存格或世界空間的外掛,標記之前先在 xEdit 裡看一下。

將滑鼠停在外掛上,會顯示它的來源、作者、標頭版本與表單版本、說明的前 300 個字元、它的 master,以及在 LOOT 排序之後 LOOT 的訊息。在 Skyrim、Skyrim SE、Skyrim VR 與 Enderal SE 上,遊戲還會讀取與已載入外掛同名的 INI;INI 欄會標出這些外掛,點擊即以唯讀方式開啟該檔案。

Conflicts 說明每個檔案的贏家與輸家。Overwrite 一步就把遊戲寫出來的東西變成真正的模組。

Archives 列出已啟用的模組帶有的每一個 BSA 或 BA2、它來自哪個模組,以及它為什麼會被載入:同名的啟用外掛,或 INI 註冊了它。沒有任何東西載入的封裝檔會被標為沒用的累贅。當另一個模組帶有同名而且在衝突中勝出的封裝檔時,該列會指名那個模組。Extract 會把封裝檔解開成一個以它去掉副檔名的名稱命名的新模組(`Foo.ba2` 變成 `Foo (extracted)`),並把封裝檔留在它的模組裡;散裝檔案無論模組順序如何,都勝過任何封裝檔裡的檔案。

Saves 解析每份存檔的標頭(角色、等級、位置、遊玩時間),把烘進存檔的外掛清單和你目前的比對,還附一個按鈕啟用它需要的模組。沒有經過 Eidos 就到達前綴的存檔,在[存檔屬於設定檔](#存檔屬於設定檔)裡說明。

Downloads 和 Diagnostics 在下面有各自的小節。

### 模組資訊

「Information...」會開啟每個模組專屬的對話框,有 General、Conflicts、Filetree、INI Tweaks、Notes、Nexus files 和 Nexus page 分頁。在 Filetree 裡(以及在 Data 樹裡),任何檔案都可以被隱藏:改名為 `<name>.mohidden`,這會讓它從合併檢視中消失而不刪掉它,所以某個模組多出來的幾個檔案可以被壓下去,而不必動到優先順序。Filetree 也做一般的檔案操作:新資料夾、改名、刪除、開啟。它們每一個都會拒絕不是該模組內部單純路徑的路徑:不能有 `..`、不能是絕對路徑,也不能有任何一段是 symlink,因為跟著它走會把一次刪除帶到模組資料夾之外。改名只替換最後一段,所以它永遠不會變成移動,而遇到已被占用的名稱它會拒絕。刪除要兩次點擊,而且無法復原。

在 Filetree 或 Data 樹的任一列上按 View 會預覽該檔案:圖片、文字、DDS 貼圖(可選擇 mip、layer、face 和 channel),以及帶漫反射貼圖的靜態 NIF 模型。NIF 預覽需要 Eidos 旁邊或 `PATH` 上有 `eidos-nif-preview`;發行套件、Arch 套件和從原始碼建置的版本都有,`eidos-linux` 沒有。文字最多讀到 64 KB,並會說明它在哪裡停下。

INI Tweaks 列出模組放在自己 `INI Tweaks/` 資料夾裡的片段;啟用的那些會在啟動時按優先順序合併進設定檔的遊戲 INI,並在擷取這次執行的 INI 時再拿掉,所以停用一個 tweak 就會移除它的設定。

Nexus files 分頁列出該模組在 Nexus 上的檔案,最新的在前,並標出這裡安裝的那個與取代它的那個。後者那一列有一個 Update 按鈕;見[模組更新](#模組更新)。

Nexus page 分頁顯示模組的描述,以及頁面上每個檔案版本的更新日誌,最新的在前。第一次開啟該分頁時,Eidos 會從 Nexus 的 GraphQL API 讀取這兩項內容,這個 API 不計入每小時的請求額度;讀取的內容會保留到 Eidos 關閉或你按下 Refresh 為止。內容以純文字顯示:BBCode 與 HTML 格式會被移除,連結的位址寫在連結文字之後,圖片與影片不顯示。文字上方的 Endorse 與 Track 與模組右鍵選單的作用相同。成人模組的頁面會保持隱藏,除非你的 Nexus 帳號設定為顯示成人內容;沒有分級的頁面在任何情況下都會保持隱藏。

### 下載

一個下載項目可以從 Downloads 清單拖到模組清單的某個位置,以那個優先順序安裝;從檔案管理器拖進視窗的壓縮檔或資料夾同樣會安裝。檔案拖放需要 X11 或 XWayland 工作階段:winit 只為 X11 實作了檔案拖放。下載可以暫停和續傳:暫停會停下傳輸並保留已下載的部分,Resume 會向 Nexus 要一條新的連結,從停下的地方繼續。

Downloads 分頁保存你下載的壓縮檔。可以依名稱篩選(也包括好記的模組名稱,所以「skyui」找得到 `SkyUI_5_2_SE-12604-5-2SE.7z`)、依最新、名稱、大小或狀態排序,還能把你用完的壓縮檔隱藏起來:檔案會保留,只是把那一列拿掉。「Show hidden」會把它們帶回來,同一個按鈕也負責取消隱藏。「Remove N installed」會用兩次點擊刪掉你已經安裝過的模組的壓縮檔,而且只刪篩選結果顯示出來的那些。

Preferences -> Nexus -> Install downloads when they finish 預設為關閉。開啟後,視窗開著時完成的 Nexus 下載會直接安裝,不必再到 Downloads 分頁。這是一般的安裝:FOMOD 會開啟它的安裝程式,模組清單裡已有的名稱會詢問如何處理。視窗開啟之前、或選項開啟之前就在 `downloads/` 裡的壓縮檔不會被動,你取消過安裝的下載也一樣。在合集面板中開啟合集期間,沒有任何下載會自行安裝,期間完成的下載之後也不會自行安裝;合集會安裝它的成員。等待中的 Update 所等的檔案由 Update 安裝。只有 Nexus 下載會這樣安裝,被 Downloads 篩選隱藏的下載要等到清除篩選後才會安裝。拖曳進行中,或遊戲或另一個 Eidos 佔用實例時,安裝會等待。

### 模組更新

工具列右端附近的更新按鈕會檢查每個有 Nexus id 的模組。當模組頁面顯示的版本就是你安裝的版本時,不再多問。否則 Eidos 會讀取該模組的檔案清單(多一次請求,在整個檢查期間保留),並從模組 `meta.ini` 記錄的檔案 id 出發,沿著更新鏈往下找:作者會把一次新上傳標記為某個舊檔案的後繼,MO2 讀的是同一條鏈。鏈上較新的檔案就是更新。當鏈上沒有後繼,而已安裝的檔案仍列在頁面上時,即使頁面顯示的是另一個版本,該模組也算是最新的。只有在沒有記錄檔案 id、已安裝的檔案被移除或封存,或 Nexus 沒有送來檔案清單時,才由頁面的版本決定。

每個模組使用它自己的 `meta.ini` 所指名(`gameName`)的遊戲的 Nexus 網域,而不是實例的:更新檢查、Endorse、Track 和 Visit on Nexus 都去那裡。Enderal SE 也接受 `skyrimspecialedition` 的模組,所以裝在 Enderal SE 實例裡的 Skyrim SE 模組,會在它的 Skyrim SE 頁面上檢查。Mod Manager Download 連結,在你上次用的實例的遊戲接受該連結的網域時,會送到那個實例。否則它會先送到網域正是它自己的那款遊戲的實例,再考慮只是接受它的遊戲的實例。`EIDOS_INSTANCE=<folder>` 會覆蓋這兩者。

模組右鍵選單裡的 Track 和 Untrack 會把該模組加入或移出你在 Nexus 上的追蹤模組,這樣網站的通知就會與你在這裡追蹤的內容一致。同一個選單裡的 Endorse 和 Abstain 會在 Nexus 上推薦該模組或撤回推薦;更新按鈕旁邊的推薦按鈕會推薦選取的模組。每次檢查更新時,Eidos 也會從 Nexus 讀取你的推薦與追蹤模組(兩次請求),並依模組自己的 Nexus 網域,把每個模組的 Endorsed 與 Tracked 標記設成一致。兩邊不一致時,以 Nexus 為準。任一清單讀取失敗時,檢查結果會說明這一點,並保留那些標記不變。

「Information...」的 Nexus files 分頁上的 Update 按鈕會安裝較新的檔案。premium 帳號由 Eidos 自己下載檔案。免費帳號則開啟檔案頁面,在那裡 Mod Manager Download 會把檔案送回來,壓縮檔抵達 `downloads/` 時安裝就開始。之後就是平常的安裝:FOMOD 會以模組安裝時的答案開啟,接著的詢問提供 Replace,並勾選「Keep a backup of the existing mod」,不論你上次詢問時選了什麼。Replace 會保留模組的優先順序與啟用狀態。

Replace 取代模組資料夾之前,Eidos 會依檔名與大小比較舊資料夾和新資料夾,並讀取兩邊外掛的標頭。Update、合集更新,以及在安裝詢問中選擇的 Replace 都適用。接著狀態列會統計新增、刪除與大小改變的檔案數,並列出新版本需要而舊版本不需要的每個 master。「Information...」的 General 分頁末尾有一個 Last update 區塊,列出版本、新增或移除的外掛、新增或不再需要的 master、ESM、ESL 或 ESH 旗標改變的外掛、此模組現在共用或不再共用檔案的模組,以及檔案。以相同大小重寫的檔案不會列出。報告存放在實例的 `logs/updates/<mod>.json`,`eidos pack` 不會打包它;全新安裝或對該資料夾 Merge 會將其刪除。`eidos install --replace` 會在 `changes:` 行輸出相同的數字。

含有多個 Nexus 檔案的資料夾(例如混入了選用檔案的主檔案)會被拒絕:用新的主檔案取代它會丟掉其他檔案。這些請手動更新。

`eidos nexus update skyrimse --apply` 會做同樣的檢查,然後在 premium 帳號上下載每個新檔案,並用 `--replace --backup` 安裝到它的資料夾上。在免費帳號上則改為替每個檔案頁面印出 `OPEN <url>`。合併了多個檔案的資料夾會被略過並附上原因。沒有儲存答案的 FOMOD 模組(例如在 Eidos 開始儲存答案之前安裝的)不會用安裝程式的預設值重新安裝:這次更新會失敗,並提示你在視窗中更新該模組一次,視窗會儲存答案。安裝程式沒有任何可選項的 FOMOD 照常安裝。只要有一個更新失敗,命令就以 1 結束。

### 合集更新

當你執行 Check for updates 時,Eidos 也會向 Nexus 詢問此實例中每個已安裝合集的最新修訂版。模組清單上方的一行會列出有較新修訂版的合集,並統計新增(+)、更新(~)和移除(-)的模組數量,把滑鼠移上去可看到名稱。如果已安裝修訂版的配方不再快取,則沒有數量。Update 會在合集面板中開啟較新的修訂版,由 *Install this collection* 執行安裝。該清單在每次檢查時重建,不會儲存。

### Nexus 設定

Preferences -> Nexus -> Offline 會讓 Eidos 完全不去接觸 Nexus,也會跳過每天一次的 GitHub 版本檢查。更新檢查、登入、下載與合集都會直說,而不是以連線錯誤失敗。它預設是關的。

Preferred servers 為下載偏好的 CDN 節點排名,最好的在前。只有 premium 帳號才會拿到超過一個鏡像可選,所以對其他人來說是 Nexus 決定,這個設定不改變任何事。如果你指名的節點都沒有被提供,下載照樣進行,用 Nexus 最先給的那個節點。

### 類別

類別可以指派給單一模組或整批選取,可以在同一個對話框裡編輯目錄本身,也可以從 Nexus 拉取該遊戲的官方類別清單。兩個目錄檔案都是 MO2 自己的(`categories.dat` 與 `nexuscatmap.dat`),所以共用的實例只會有一份目錄。從 Nexus 下載的模組,在 Fetch from Nexus 對應好目錄之後,安裝時會自動歸類。

### INI 編輯器、日誌與擴充

View -> INI editor 編輯設定檔的遊戲 INI,也就是 Eidos 保留的那一份;Proton 前綴裡的那一份在每次啟動時都會被覆寫。它的最後一個分頁是設定檔的 `launch-env.ini`([每個設定檔的變數與包裝器](#每個設定檔的變數與包裝器))。View -> Log 讀取工作階段日誌。View -> Extensions 列出你自己的附加元件;見 [extensions.md](extensions.md)。

啟動時如果在設定檔和前綴裡都找不到遊戲的主 INI(`Skyrim.ini`、`Fallout4.ini`、`Oblivion.ini` 等),Eidos 會像 MO2 一樣,先把遊戲資料夾裡的預設檔(`Skyrim_Default.ini`、`Fallout4_Default.ini` 等)複製到設定檔。實例執行遊戲副本時,預設檔取自副本。前綴和遊戲資料夾不會被改動。

### 安裝模組

安裝程式處理 Simple 壓縮檔、FOMOD、Wrye Bash 的 BAIN 套件(勾選子套件,它們會依序合併),以及 Oblivion 的 OMOD 套件。OMOD 腳本在它使用 Eidos 支援的 OBMM 命令,或是 Eidos 認識的三個 C# 安裝程式之一時會執行;Eidos 不執行其他任何 C# 或 Visual Basic 程式碼,並會說明這一點。沒有任何啟發法認得出版面時,手動選擇器會顯示壓縮檔的樹狀結構,讓你自己指出資料根目錄。

重新安裝或更新 FOMOD 模組時,精靈開啟時會勾好上次安裝的答案,停在之後選項有變動的第一步,並列出變動。Reinstall with new choices 則從安裝程式的預設值開始。`eidos install --replace` 或 `--merge` 沿用儲存的答案,答案不再適用時會列出變動並拒絕安裝。

### 診斷

Diagnostics 分頁執行即時的健康檢查。它回報缺少的 master、被 LOOT 標記為 dirty 的外掛、沒有任何啟用外掛會載入的封裝檔、模組清單是否仍與 mods 資料夾相符、FUSE passthrough 是否開啟,以及在它被要求時,所需的啟動 capability 是否在。在一次執行之後,它會顯示 script extender 自己的日誌對它每一個外掛 DLL 說了什麼,讓你看到你的 SKSE 外掛有沒有載入。它也會回報自上次乾淨啟動以來的遊戲更新、缺少或與遊戲版本不符的 script extender(附 extender 的下載頁面,以及安裝你下載的封裝檔的按鈕)、沒有用 `%command%` 啟動 Eidos 的 Steam 啟動選項、DLSS 需要卻仍關閉的 NVAPI、阻擋影格生成的獨佔全螢幕、沒有 `LinuxVersion=true` 的 ENB `enblocal.ini`、F4SE 與 SFSE 外掛的宣告,以及上次工作階段的當機日誌和其中點名的模組。

設定檔 INI 與外掛的檢查會找 Fallout 4、Fallout 4 VR 和 Starfield 的自訂 INI 在 `[General]` 裡的 `sTestFile` 行,這些行會讓遊戲忽略 `plugins.txt`(Fix 會從設定檔的副本移除它們,並把先前的檔案保留為 `<ini>.pre-fix`),只對 Fallout 4、Fallout 4 VR 和 Starfield 做這項檢查;還有 Starfield 中已啟用的 `.esp` 外掛,以及缺少 blueprint 旗標、`BlueprintShips-` 前綴或 `.esm` 副檔名,或其主外掛未啟用的 blueprint 外掛。系統檢查會標出低於 Proton 所需的 `vm.max_map_count` 或硬性開啟檔案數上限,以及位於 NTFS、exFAT 或 FAT 上的遊戲、前綴或實例。模組檢查會指名沒有指向遊戲 `Data` 的 BodySlide 資料路徑、工具寫進 `Overwrite/Root` 的 `Data` 資料夾、散裝檔案輸給它所修補的模組的修補程式,以及取代 script extender 的 Papyrus 腳本的模組。

檢查更新時也會讀取每個模組在 Nexus 上的需求。Diagnostics 會列出缺少或已停用的需求模組,已用其他方式滿足的需求可用 Ignore 忽略。

這個分頁也顯示是哪個 Proton 版本在執行遊戲,並在較舊的 Proton 要執行較新 Proton 設好的前綴之前發出警告;Rebuild 會把損壞的前綴移到一旁,讓 Steam 建立新的前綴。其他卡片會標出從未設定過的前綴或登錄檔已過時的前綴、由 Eidos 之外的程式接收 Mod Manager Download 連結的情況(Register 可以修好)、比它的輸入還舊的工具輸出(附一個重新執行該工具的按鈕),以及原始外掛後來改變了的已清理或 light 副本。Proton 與前綴那幾張卡片在 [troubleshooting.md](troubleshooting.md#遊戲由哪個-proton-執行以及損壞的前綴)裡說明,每一張卡片都在 [troubleshooting.md](troubleshooting.md#health-卡片)裡說明。Copy report 會把系統與每項發現的報告放進剪貼簿,並隱藏你的家目錄與登入名稱,用於錯誤回報。

對於 Skyrim SE、Enderal SE 和 Fallout 4,Diagnostics 還有一張 Display 卡片。它顯示設定檔的 `SkyrimPrefs.ini`(`EnderalPrefs.ini`、`Fallout4Prefs.ini`)中設定的視窗大小和模式;設定檔還沒有這個檔案時,則顯示前綴裡那份副本的設定。兩者都沒有設定大小時,輸入欄會取 `/sys/class/drm` 中列出的已連接的最大螢幕,所以 Steam Deck 上顯示 1280x800。寬度與高度是 320 到 16384 的整數。Apply 會把 `iSize W`、`iSize H`、`bFull Screen` 和 `bBorderless` 寫進設定檔的副本;設定檔沒有副本時,先接管前綴裡的那份。SSE Display Tweaks 設定視窗時,Apply 還會把同樣的大小和模式寫進它在 Overwrite 中的副本;模組本身的檔案不會被改動。

分頁頂端的 Game files 卡片在[遊戲檔案](#遊戲檔案商店安裝還是副本)裡說明,合集順序卡片在[保持合集順序](#保持合集順序)裡說明,Find the mod behind a problem 則在[測試執行](#找出造成問題的模組)裡說明。

### 啟動遊戲

要透過視窗啟動遊戲,把該遊戲的 Steam 啟動選項設成程式的絕對路徑,因為 Steam 不會讀取你 shell 的 `PATH`([install.md](install.md#接著讓-steam-指向它)):

```
~/.local/bin/eidos-gui %command%
```

從 Colony 安裝時,路徑是 `~/.local/share/Colony/apps/Eidos/eidos-linux`。Eidos 會在該遊戲的實例上開啟,也就是你上次用的那一個,所以可攜實例和全域實例一樣會被再找到;點 Run 就會穿過合併檢視啟動它。

開啟 Start the game at once when a store opens Eidos(Preferences, General, Running a game;預設關閉)後,Eidos 會省掉這次點擊:它先檢查 Health,再不顯示視窗直接啟動遊戲。Health 列出問題、模組清單需要處理、設為在遊戲前執行的工具輸出已過期,或 Skyrim SE、Enderal SE、Fallout 4 的設定檔還沒有視窗大小時(視窗會停在 Diagnostics 分頁),視窗照常開啟。想在某一次啟動時看到視窗,就在命令前加上 `EIDOS_WINDOW=1`:`EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`。

在沒有商店開啟過的視窗裡,Run 不會自己啟動遊戲,Eidos 也從不直接透過 Proton 或 Wine 啟動遊戲。Run 請擁有這份副本的商店來啟動,再由商店啟動 Eidos:

- Steam:設好上面的啟動選項,而且遊戲有 Proton 前綴(Steam 第一次啟動遊戲時會建立)時,執行 `steam -applaunch <appid>`。Steam 沒開時會先啟動 Steam。
- Heroic(GOG 或 Epic 副本,原生版或 Flatpak 版):當 Eidos 是 Heroic 裡該遊戲設定中的一個包裝器,執行檔為 `eidos-gui`(或 `eidos-linux`)的絕對路徑且不帶參數時,它會收到一個 `heroic://launch` 連結。Heroic 沒開時會先啟動 Heroic。
- 單獨的 Legendary:當 `~/.config/legendary/config.ini` 在遊戲的 `[<app>]` 區段或 `[default]` 下有 `wrapper =` 加上 `eidos-gui`(或 `eidos-linux`)的絕對路徑時,執行 `legendary launch <app>`。

Run 清單裡 script extender 的載入器、啟動器和遊戲執行檔,都會這樣交給商店,在終端機裡對其中之一執行的 `eidos tool <instance> run <entry>` 也一樣。Eidos 無法把有些副本交給它們的商店,例如 Heroic 之外的 GOG 副本:這時 Run 會要你從那個商店啟動遊戲。

這次交接會把這個實例記錄為你上次使用的實例,然後商店會在它上面開啟第二個 Eidos 視窗。Start the game at once 關閉時,你在那裡點 Run;開啟時,那個視窗不顯示自己就直接啟動遊戲,除非上面的某種情況讓它開啟。第一個視窗不會追蹤這次執行,所以玩完後在它裡面按 F5 重新載入清單。商店的設定缺少,或 script extender 的載入器不在遊戲執行檔旁邊時,Run 什麼也不啟動,並顯示要在哪裡設定什麼,帶著執行中執行檔的真實路徑。

遊戲或工具執行時,Run 會變成 Stop。按兩次會結束這次執行啟動的所有程式,然後 Eidos 像任何結束之後一樣收集工作階段並卸載;被停止的執行絕不算作一次正常啟動([troubleshooting.md](troubleshooting.md#無法關閉的遊戲或工具))。

Bethesda 那幾款作品的 Steam `%command%` 通常指向 `<Game>Launcher.exe`。Eidos 從不執行它:那個啟動器是一支獨立的設定程式,它會重新掃描 `Data` 並改寫 `plugins.txt`,把剛剛部署好的載入順序推翻掉。如果裝了 script extender,Eidos 會改執行它的載入器,否則執行遊戲主程式,而且在不得不退回遊戲主程式時會說明。

Eidos 會掃描已啟用模組的 DLL 是否匯入 `d3dcompiler_47.dll`,把 Microsoft 的 DLL 複製進前綴,然後才設定 `WINEDLLOVERRIDES=d3dcompiler_47=n,b`;你不需要自己設定它。

### 每個設定檔的變數與包裝器

設定檔可以在 `profiles/<name>/launch-env.ini` 裡帶上自己的環境變數與包裝器指令,每行一項:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

它們對該設定檔的每次 `eidos play`(由 Steam 的 `%command%` 啟動的遊戲,或由商店開啟的視窗裡的 Run 啟動的遊戲)與每次工具執行生效。`eidos shell` 只會得到變數,沒有包裝器。第一條 `wrapper=` 行在最外層。含有 `%command%` 的 `wrapper=` 行會把指令放在那個位置,和 Steam 的啟動選項一樣;不含的行把指令加在最後。包裝器在 Eidos 的私有命名空間裡執行,所以看得到模組。

這裡的變數會取代 Eidos 為同名變數設定的值,而工具在 `tools.ini` 裡自己的 `env=` 行會同時取代這兩者。`WINEDLLOVERRIDES` 則是附加在 Eidos 為 ENB、ReShade 與內建 DLL 設定的覆寫之後,所以兩邊都提到的 DLL 採用設定檔的設定,其餘維持原樣。

View -> INI editor 會把這個檔案當作最後一個分頁開啟,每款遊戲都一樣。空行和以 `#` 或 `;` 開頭的行會被略過。儲存時會列出 Eidos 忽略的行號:既不是 `env=NAME=value` 也不是 `wrapper=command` 的行,以及第一個字是 `NAME=value` 的 `wrapper=` 行(Steam 的啟動選項允許這樣寫);請把那個變數改放在 `env=` 行。不帶指令的 `eidos play <instance>` 會列出一次執行得到的變數與包裝器。Diagnostics 裡的 Copy report 會寫出變數名稱與包裝器程式名稱,不含它們的值。

## 工具

xEdit、BodySlide、DynDOLOD 以及同類工具,都是在遊戲的 Wine 前綴裡(Steam 的 Proton 前綴,或 Heroic 為 GOG 或 Epic 副本設好的前綴)穿過合併檢視執行:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # run an exe once without registering it
eidos prereqs skyrimse            # what the registered tools need, and its state
eidos prereqs skyrimse --install  # fetch whatever is missing
```

Pandora、Nemesis、FNIS、BodySlide、DynDOLOD、TexGen、xLODGen 和 Synthesis 不必註冊就能被找到。Pandora、Nemesis、FNIS、BodySlide 和 Synthesis 加入時已設好擷取模組,所以它們的輸出會進入自己的模組(細節見 [tools.md](tools.md#把工具的輸出送進它自己的模組))。

你給工具取的標題,決定 Eidos 為它準備哪些執行階段與 DLL:`BodySlide` 會拿到它的 DirectX 函式庫,`BS` 什麼也拿不到。在視窗裡,Executables 對話框會在欄位底下顯示每一項前置需求的真實狀態,缺少的那些就是按鈕。

發行套件在 `runtimes/` 裡附上 vcrun2022 安裝程式(x86 與 x64)、.NET 8 Desktop Runtime 8.0.12 和 .NET 10 執行階段 10.0.10,`install.sh` 會把它們複製到 `~/.local/share/Colony/Eidos/runtimes/`,所以 `eidos prereqs <instance> --install` 不用下載就能安裝它們。Eidos 在遊戲的前綴裡以無視窗方式執行 Microsoft 的 vcrun2022 與 .NET 8 安裝程式,不經過 winetricks。`eidos-linux` 和從原始碼建置的版本沒有 `runtimes/` 資料夾,所以它們從 Microsoft 下載同樣固定的檔案。Synthesis 需要的 .NET 10 SDK(約 300 MB)一律要下載。

那張表、三類前置需求、DynDOLOD 為什麼需要一個 winetricks 裝不了的 .NET 執行階段,以及以模組方式安裝的工具為什麼是從合併後的路徑而不是它自己的資料夾啟動,都在 [tools.md](tools.md)。

## 擴充

Eidos 不必重新建置就能擴充:一份放在 `~/.config/Colony/Eidos/addons/` 的 TOML 清單,可以在 Extensions 清單裡加一個工具,或在 Diagnostics 分頁加一項檢查。沒有任何東西被載入 Eidos 內部:擴充就是 Eidos 執行的一支程式。見 [extensions.md](extensions.md)。

## 試試概念驗證

這個腳本不需要遊戲。它只用 user 命名空間裡的非特權 OverlayFS(Linux 5.11 或更新版本),展示合併檢視、copy-on-write、原封不動的遊戲資料夾,以及一個只存在於它自己命名空間裡的合併:

```sh
./scripts/poc-overlay.sh
```
