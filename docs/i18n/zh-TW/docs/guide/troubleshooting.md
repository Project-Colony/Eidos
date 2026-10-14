<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

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

Passthrough 把真正的後端檔案交給核心,於是讀取完全跳過這個常駐程式。它換來的是
吞吐量,代價是這裡的正確性。在 Skyrim SE 1.6.1170、proton-cachyos 11.0、核心
7.1.4、同一份 82 個外掛的載入順序上做 A/B 實測,唯一的變因是執行檔有沒有帶上那個
capability:

| passthrough | `NtCreateFile` 以 `STATUS_ACCESS_VIOLATION` 失敗的次數 |
|-------------|--------------------------------------------------------|
| 開啟        | 152 - 75 個 `.bsa`、65 個 `.esl`、10 個 `.esm`、2 個 `.esp` |
| 關閉        | 0                                                      |

開著時,遊戲一個自己的封裝檔或外掛都打不開,在遊戲裡呈現出來就是模組根本不存在 -
沒有錯誤,沒有一行日誌。關掉時,同一份載入順序能走到實際遊玩,外掛、封裝檔與
Papyrus 腳本都活著。

這個失敗從常駐程式內部是看不見的,這正是它難找的原因:我們自己的 `open` 每次都
成功,核心也從來不拒絕後端檔案(以 `EIDOS_FUSE_TRACE=open` 追完一整場失敗的執行
驗證過:零筆 `open FAILED`,零筆 `passthrough refused`)。錯誤是在常駐程式回覆
`opened_passthrough` 之後才產生的,所以常駐程式這一側再怎麼記錄都看不到。它也不挑
副檔名 - 封裝檔與外掛一視同仁,也就是遊戲整場執行期間一直開著的那些檔案。

`EIDOS_FUSE_PASSTHROUGH=1` 會把它重新開啟,用來衡量它帶來什麼,或重新測試這個
機制。啟動器與 Diagnostics 分頁裡的 capability 警告,只有在你主動要求它時才會出現。

要透過 Eidos 啟動遊戲本身,把它的 Steam 啟動選項設成:

```
eidos play skyrimse -- %command%
```

如果 Proton 需要原生 d3dcompiler 來編譯著色器,就在前面加上
`WINEDLLOVERRIDES="d3dcompiler_47=n"`;Eidos 會把它與模組自帶的任何 DLL override
合併(ENB/ReShade/`.asi` 載入器)。


### 層索引真的有在用嗎?

這個索引要嘛全有要嘛全無,而且是靜悄悄建起來的:`LayerStack::new` 拿到的不是唯讀
各層的完整對照表,就是 `None`,之後每一次查詢都跟以前一樣逐層走過。工作階段日誌裡
沒有任何東西能分辨這兩者,所以一個默默退回舊路的堆疊,看起來跟正常運作的一模一樣
- 只是照舊付出以前那份成本。

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` 會在有索引與沒索引的情況下解析真實路徑,並比對目錄掃描的結果。
`index_agrees` 檢查兩者在一個真實實例的每一條路徑與每一次列表上,給出的是完全相同
的答案。`listing_cost` 衡量合併子項對照表在 `readdir` 上省下了什麼。

`EIDOS_NO_INDEX=1` 會強制走訪,用在兩種答案之間的差異正是你要除錯的對象時。

## 已知問題

### DLSS 或畫格生成靜悄悄地毫無作用

三種各自獨立的成因,每一種都沒有任何錯誤訊息:啟動選項裡沒有啟用 NVAPI、獨佔
全螢幕,或是一個過時的 Reflex FPS 上限。完整的檢查清單在
[graphics.zh-TW.md](graphics.md)。

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
[tools.zh-TW.md](tools.md)。症狀不會認錯:每一個世界的
`LODGen_SSE_<world>_log.txt` 裡只有一段版本橫幅、一行 `.NET Version:`,再無其他,
而對話框只說「failed to generate object LOD for one or more worlds」。成因是 Wine
的 Mono 代替 .NET Framework 回應,而且裝再多次 .NET Framework 都修不好 - 每次前綴
更新,Proton 都會把 `mscoree.dll` 換成一個指向它自己樹裡的 symlink。

**Wine 分辨不出這個掛載會摺疊大小寫。** 已修正,而且這是最要緊的那一個。

沒有任何 API 能回答「這個檔案系統是不是不分大小寫」,所以 Wine 的
`get_dir_case_sensitivity` 會去嗅 CIOPFS 留在它所服務目錄裡的那個標記。標記不在,
Wine 就假設大小寫敏感,而每一次拼法沒有逐位元組相符的查找,都會退回去讀整個目錄來
找不分大小寫的相符項。Bethesda 的遊戲要的是 `data/ccbgssse001-fish.bsa`,而檔案叫
`ccBGSSSE001-Fish.bsa`,於是它幾乎在每一個素材上都被觸發:八秒內 4471 次標記探測與
2236 次整個目錄重讀,九十秒內 195796 次對 `Data` 的列舉。Skyrim SE 從來沒走到主
選單 - 它停在 240 MB 常駐記憶體,而常駐程式燒掉了一顆核心的 92%。

Eidos 從一開始就在 `resolve_read` 裡摺疊大小寫。這整份成本只是因為從來沒說出來。
現在 `lookup` 會回應 `.ciopfs`;`readdir` 仍然不把它列出來。

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
Skyrim SE 因空指標解參考而當機,可穩定重現,而且一個模組都沒裝;另外三個核心側的
快取被逐一二分排除,只有這一個有影響。當時測出來失去它是零成本,但那次測量是在
`FUSE_PASSTHROUGH` 開著的情況下做的,那時常駐程式服務的讀取是*零*次
(`EIDOS_FUSE_STATS` 在一次完整載入中回報 `read 0`),而核心早就針對後端檔案在快取
那些分頁了。Passthrough 現在預設關閉(見下),所以那個論據不再成立,真正的成本是
未經測量的 - 不管怎樣,光是那次當機就足以構成讓它保持關閉的理由。要調查的話用
`EIDOS_FUSE_KEEP_CACHE=1` 重新開啟;這兩個旗標不再糾纏在一起,所以現在可以單獨測試
它。

### FUSE passthrough 讓遊戲載入不了任何模組內容

已藉由關掉它修正;`EIDOS_FUSE_PASSTHROUGH=1` 會把它找回來。在核心 7.1.4 上,
passthrough 開著時,Skyrim SE 有 152 個自己的檔案(75 個 `.bsa`、65 個 `.esl`、
10 個 `.esm`、2 個 `.esp`)以 `STATUS_ACCESS_VIOLATION` 開啟失敗,關掉時是 0 個 -
於是沒有任何模組內容會載入,而且悄無聲息。核心是在常駐程式回覆
`opened_passthrough` 之後才拋出這個錯誤,所以常駐程式自己的日誌顯示的是一場乾淨的
執行(零次開啟失敗,零次後端檔案被拒)。核心路徑裡的根本原因尚未確立;保留這個開關
是為了它能被重新測試,也為了萬一 image-mapping 真的需要 passthrough,能把它縮小到
只給 DLL。
