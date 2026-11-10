<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=c9bb5a3ae78695b89ce4595223a2d2ff1902345e -->

# 疑難排解與診斷

為了遊戲看見的東西與檔案系統對不上的那一天所準備的一切:環境開關、如何讀操作
計數器、已知問題及其歷史,以及 passthrough 的來龍去脈。

### 診斷 VFS

當遊戲看見的東西與檔案系統對不上時,有兩個環境變數可用:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

細分的那種形式,正是找出 troubleshooting.md 所述那次當機的東西:四個全關回答的是
「是不是快取的問題?」,只逐一指名才回答「是哪一個」。計數器回答另外一半 - 一次
顯示 `read 0` 的載入,代表每一個位元組都由 `FUSE_PASSTHROUGH` 在核心裡服務掉了,
所以你原本打算在讀取路徑上調校的東西,早就不花成本了。

## 手動掛載一個聯集

衝突時第一個 `--layer` 勝出;最後一個是你原封不動的遊戲資料。掛載只需要
`/dev/fuse` 與 `fusermount3`(不需要 overlayfs,不需要 Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

寫入會落在 `--overwrite <dir>`(省略時是一個暫存目錄),所以即使在這裡,各層本身
仍然原封不動。


#### 為什麼 passthrough 預設關閉

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

要透過 Eidos 啟動遊戲本身,把它的 Steam 啟動選項設成:

```
eidos play skyrimse -- %command%
```

不需要加上 `WINEDLLOVERRIDES` 前綴。當某個已啟用的模組匯入 `d3dcompiler_47` 時,
Eidos 會把自己隨附的那份部署進前綴、強制它以原生方式載入,並把這項設定與模組自帶的
任何 DLL override 合併(ENB/ReShade/`.asi` 載入器)。


### 層索引真的有在用嗎?

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

## 無法關閉的遊戲或工具

遊戲或工具執行時，執行按鈕 Run 會變成 **Stop**，鎖定卡片上也有一個 Stop 按鈕。按兩次。第二次按下會結束這次執行啟動的所有程式：先送 SIGTERM，10 秒後對剩下的送 SIGKILL。遊戲沒有儲存的內容會遺失。之後 Eidos 做的事與任何結束之後相同：INI 和外掛的變更回到設定檔，工具的輸出被記錄，模組被卸載。**Unlock** 不一樣：它把視窗交還給你，程式繼續執行。在終端機裡對 `eidos` 行程執行 `kill <pid>`（pid 顯示在鎖定卡片上）與 Stop 效果相同。

## 遊戲由哪個 Proton 執行，以及損壞的前綴

Diagnostics 分頁有一張 **Proton** 卡片：Eidos 和 Steam 為該遊戲使用的版本、Steam 自己的設定，以及最後執行該前綴的版本。如果最後準備前綴的 Proton 比所選的更新，卡片會變成建議，因為在較新的前綴上用較舊的 Proton 可能會重設其中的 Wine 檔案，或者無法啟動。請在 Steam 中遊戲的內容、相容性裡選擇一個至少同樣新的版本。

該卡片上的 **Rebuild prefix** 用於已經無法使用的前綴。按兩次。Eidos 不刪除任何東西：它在同一資料夾裡把 `steamapps/compatdata/<appid>` 重新命名為 `<appid>.eidos-backup-<date>-<time>`，下次你從 Steam 啟動遊戲時，Steam 會建立一個新前綴。只要有遊戲、工具或任何 Wine 程式在使用該前綴，它就會拒絕。只存在舊前綴裡的存檔和設定會留在重新命名後的資料夾中：把需要的複製回來，不再需要時請自己刪除那個資料夾。

## 已知問題

### DLSS 或畫格生成靜悄悄地毫無作用

三種各自獨立的成因,每一種都沒有任何錯誤訊息:啟動選項裡沒有啟用 NVAPI、獨佔
全螢幕,或是一個過時的 Reflex FPS 上限。完整的檢查清單在
[graphics.md](graphics.md)。

**一個把同一個目錄寫成兩種拼法的模組,弄丟了第二種拼法底下的一切。** 已修正。ext4
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

**DynDOLOD 的 LODGen 死掉,只留下一份空的日誌。** 已由 `dotnet10` 修正;見
[tools.md](tools.md)。症狀不會認錯:每一個世界的
`LODGen_SSE_<world>_log.txt` 裡只有一段版本橫幅、一行 `.NET Version:`,再無其他,
而對話框只說「failed to generate object LOD for one or more worlds」。成因是 Wine
的 Mono 代替 .NET Framework 回應,而且裝再多次 .NET Framework 都修不好 - 每次前綴
更新,Proton 都會把 `mscoree.dll` 換成一個指向它自己樹裡的 symlink。

**Wine 分辨不出這個掛載會摺疊大小寫。** 已修正,而且這是最要緊的那一個。

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

**遊戲把 `plugins.txt` 重寫成空的**很可能就是同一回事 - 一個它在任何合理時間內都
列舉不完的 `Data`,於是它認定那裡什麼都沒有,並把這個結論存了下來。尚未證實,值得
重新查一次。無論如何,擷取防護(任何規模下,會把啟用集合整個清空的擷取一律拒絕)
意味著它再也弄不壞設定檔了。

**`FOPEN_KEEP_CACHE` 是關的。** 已修正,而且值得知道原因。它會在主選單之後幾秒讓
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
