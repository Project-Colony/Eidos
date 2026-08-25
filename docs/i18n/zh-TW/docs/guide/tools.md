<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# 工具:xEdit、BodySlide、DynDOLOD、FNIS

透過 Eidos 執行的工具看到的是**合併檢視**,就在遊戲自己的 Proton 前綴裡。
它讀到的正是遊戲會讀到的東西 - 每一個啟用的模組,按優先順序 - 而它寫出的
任何東西都落進 Overwrite,在那裡一鍵就能變成真正的模組。

## Eidos 自己找得到的那些

有些工具的名字夠獨特,不必宣告就能被找到,xEdit 就是最明顯的例子:Fallout 4
是 `FO4Edit.exe`,Skyrim SE 是 `SSEEdit.exe`,初代是 `TES5Edit.exe`,依此類推 -
連同各自的 **QuickAutoClean** 雙生版,那就是用來處理 LOOT 一直警告的 dirty
edits 的按鈕。Eidos 會依檔名,在這些地方找它們:

- 遊戲的安裝資料夾,以及已啟用模組的 `Root/` 樹;
- **這個實例的 `mods/`**,MO2 使用者就是把工具裝在那裡;
- 你在 Settings 裡設定的**工具資料夾**(Tools -> Tools folder),用於實例之間
  共用的目錄 - 像 `/mnt/Games/Tools` 之類。

清單是按遊戲分的,所以 Skyrim 實例永遠不會被提供 Fallout 的編輯器。搜尋在第
四層就停下,因為一個模組池有數十萬個檔案,而每次建立工具清單時都會跑一次;
它也不跟隨 symlink。這樣找到的工具,設定方式和你自己輸入的完全一樣:它的
執行階段依名字而定,規則與下面所有內容相同。

如果工具在別的地方,或你想用不同的參數,就手動加 - 標題相同的使用者項目會
覆蓋任何自動找到的結果。

## 加一個進去

在 GUI 裡:**Tools -> Executables**,然後按 Add。在命令列裡:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
```

script extender、遊戲主程式與啟動器都會自動偵測;只有額外的工具需要註冊。

### 指向真正的檔案,不管它在哪

執行檔實際在哪裡,就註冊在哪裡。如果工具是以模組方式安裝的,那就在模組資料夾
裡面:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(那是全域實例的路徑 - 可攜實例套用同樣的規則,只是在它自己的資料夾底下,
`<instance>/mods/...`;要注意,像這樣的絕對路徑,是日後「移動」可攜資料夾時
唯一撐不過去的東西。)

Eidos 在啟動前會把那個路徑改寫成合併後的路徑,所以工具是從
`<game>/Data/CalienteTools/BodySlide/` 執行,也能在那裡看到其他每一個模組的
檔案。這件事比聽起來重要:BodySlide 附的 `SliderSets` 目錄是**空的**,它能建出
的每一具身體都來自 CBBE 與服裝模組。從它自己的模組資料夾啟動,它什麼都找不到,
看起來就像壞了。

MO2 也做同樣的改寫,理由相同 - 它自己的註解點名了 FNIS。

位於**已停用**模組裡的工具無法被改寫,因為它的檔案同樣不在檢視裡。Eidos 會直
說,並從它自己的資料夾執行,而不是假裝沒事。

## 把工具的輸出送進它自己的模組

產生器 - FNIS、Nemesis、BodySlide、DynDOLOD、Synthesis - 會寫出好幾百個檔案。
預設情況下它們和其他東西一起落進 Overwrite。在 Executables 編輯器裡設定
**Capture output into**,這一次執行的輸出就改為進入那個模組:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

模組不存在就會被建立。只有「這一次」執行產生的檔案會被搬走;原本就在 Overwrite
裡的東西留在原地,所以兩個各有擷取目標的工具不會互相搶走輸出。什麼都沒寫的執行
不會留下一個空模組。

這是在執行結束之後才做,而不是把寫入層指向那個模組 - MO2 是後者的做法。把寫入
層指向一個模組,會讓它在整段執行期間被提升到最高優先權 - 把它牽涉到的每一個
衝突都翻一次,結束後再翻回來 - 而且會直接寫穿模組自己的檔案,沒有 copy-up。
擷取達到同樣的最終狀態,而兩者都不需要。

如果目標模組是停用的,輸出仍然會寫出去,但遊戲看不到,於是工具下次執行時又會
重新產生同樣的檔案。遇到這種情況 Eidos 會提出警告。

## 工具需要哪些 DLL,是由它的「名字」決定的

這是最出人意料的部分,值得直說:**你給工具取的標題,決定了 Eidos 會替它準備
哪些執行階段前置需求。**比對方式是對標題做不分大小寫的子字串比對。

| 標題若含有 | Eidos 會請求 |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| 其他任何情況 | 無 |

所以註冊成 **`BodySlide`** 的工具會拿到它的 DirectX DLL;同一個執行檔註冊成
**`BS`** 就什麼都拿不到,可能啟動失敗,而錯誤訊息完全沒提到 DLL。工具就照程式
的名字命名。

這份清單在 `default_prereqs`(`crates/eidos-instance/src/tools.rs`)裡,而
Executables 對話框裡的 `Prereqs` 欄位可以編輯 - 偵測結果是預設值,不是規定。

### 三種前置需求

**第一層 - 隨附的 DLL**(`d3dx9_43`、`d3dcompiler_47`、`d3dx11_43`)。Eidos
自己帶著它們,並在啟動時複製進前綴。不必做任何事,不用網路。

**第二層 - winetricks verb**(`vcrun2022`、`dotnet8`、`dotnetdesktop8`、
`dotnet48`、`xact`...)。它們會寫入登錄機碼、GAC 與 CLR host,所以沒辦法用複製
檔案解決。它們會**從 Microsoft 下載**。

**第三層 - 執行階段**(`dotnet10`)。現代的 .NET 執行階段是 193 個檔案,住在
自己的目錄裡,透過 `DOTNET_ROOT` 被找到:從不註冊,也完全不安裝進前綴,所以
另外兩層都載不動它。Eidos 自己下載,對照編進二進位檔裡的 checksum 檢查,並快取
在 `~/.local/share/Colony/Eidos/runtimes/` - **在任何實例之外**,因為 78 MB 不是
每個遊戲一份,也不是每個 profile 一份。

第二層與第三層都不會靜悄悄地執行:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

在 GUI 裡,同樣的狀態就在 Prereqs 欄位底下,缺少的那些是按鈕。既不是隨附的、
也不是執行階段、又不是已知 winetricks verb 的項目,會被回報為可能的拼字錯誤,
而不是提供下載。

### 為什麼 DynDOLOD 需要 `dotnet10`

DynDOLOD 自己不建 object LOD:它把工作丟給 LODGen,而且附了三個版本。
`LODGenx64.exe` 的目標是 .NET Framework 4.8,在 Proton 底下會被導向 Wine 的
Mono - 而它的 `System.Uri` 初始化程序會呼叫一個 Mono 沒有實作的方法。它在做
第一行正事之前就掛掉,留下一份只有版本橫幅、其他什麼都沒有的記錄檔,以及一個
只寫著「failed for one or more worlds」的 DynDOLOD 對話框。

安裝真正的 .NET Framework 也修不好:Proton 把 `mscoree.dll` - 也就是本該找到它
的載入器 - 換成指向自己目錄樹的 symlink,而且每次前綴更新都會重做一遍。

能用的版本是 `LODGenx64Win10.exe`,它的目標是現代 .NET,完全不碰 `mscoree`。把
`DOTNET_ROOT` 指向一份 .NET 10 執行階段,它就跑得起來。`dotnet10` 準備的就是
這個,而 Eidos 在啟動任何宣告了它的工具時會設好這個變數。

Eidos 是用系統的 `winetricks`,搭配 Proton 自己的 `wine` 與遊戲前綴來執行的,
這樣就繞開了 Steam 的 pressure-vessel 容器,以及 protontricks 與 Proton-GE
不相容的問題。宣告了未安裝的第二層 verb 的工具照樣會啟動,只是帶一則警告,
點名該 verb 以及修好它的命令 - 使用者也可能已經從別處裝了。

## 前綴裡的遊戲路徑

Windows 工具是靠讀 `HKLM\Software\Bethesda Softworks\<game>` 的
`installed path` 來找遊戲的,那把機碼由遊戲自己的安裝程式寫入 - 而 Steam 在
Proton 底下從來不跑那個安裝程式。沒有它,xEdit、Wrye Bash 與 DynDOLOD 開起來
就是一個空路徑。Eidos 會在執行工具前寫好它:冪等、只增不減,若前綴尚未初始化
或正在使用中則跳過。

## 找到工具:隱藏、置頂,以及桌面捷徑

一款遊戲的預設項目裡有些工具你可能從來不用,而一個要列八個項目才輪到第二個的
選單,是沒人會讀的選單。在 Executables 對話框裡:

- **Pin to top** 把項目放到 Run 清單的最前面。
- **Hide from picker** 把項目從選單裡拿掉,但不刪除它。
- **Desktop shortcut** 會把一個 `.desktop` 寫進
  `~/.local/share/applications` - 在 freedesktop 系統上啟動器本來就該待的
  地方,所以它會出現在你的應用程式選單與搜尋裡,而不是出現在桌面上。它直接
  執行 `eidos tool <instance> run <title>`,也就是說,即使 Eidos 視窗根本沒開
  著,工具也是**帶著這個實例的 profile、穿過合併檢視**啟動的。

隱藏與置頂關乎的是一個工具*怎麼被找到*,而不是它執行什麼,所以它們對每款遊戲
的預設項目和你自己的項目一樣有效。

## 本身就是獨立 Steam 應用程式的工具

Creation Kit 是獨立的 Steam 應用程式,要用自己的 AppID;少數幾個透過 Steam
發行的模組工具也一樣。在項目上設定 **Steam AppID**,Eidos 就會用那個 id 而不是
遊戲的 id 來啟動它。

在 Windows 上這代表換一個啟動器。在這裡,它只是本來就要組出來的那次執行上的
兩個環境變數 - `SteamAppId` 與 `SteamGameId`,兩個都要,因為 Proton 讀其中
一個,而 Steam 自己的函式庫讀另一個,工具若看到兩者不一致,失敗的方式會很古怪
而不是乾脆。`eidos tool ... --print` 會原原本本顯示真正執行時會拿到什麼。

## 工具自己的設定,終究還是它自己的事

Eidos 把工具擺在對的位置,配上對的 DLL。工具接下來拿它的設定做什麼,是你和
工具之間的事,而且失敗通常是無聲的。

這裡給一個實例,因為不講的話會浪費你一小時:BodySlide 的 **Game Data Path**
(Settings)必須指向遊戲的 `Data` 目錄,而不是它上面那層遊戲資料夾。設高一層,
批次建置會回報「All sets processed successfully」,然後把 1439 個 mesh 寫到
遊戲永遠不會去找的地方。Eidos 會接住它們 - 它們落在 `Overwrite/Root/` 而不是
你的安裝目錄裡 - 但從遊戲的角度看一切正常,只不過你的身體模型沒建出來。

工具的輸出本來就該待在 Overwrite。當一次執行產生了值得留下的東西,
**Overwrite -> Create mod...** 會把它變成一個普通模組,可以像其他模組一樣
排序、停用和移除。
