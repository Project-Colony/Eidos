<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# 透過 Eidos 玩 Fallout 4

Fallout 4 不需要任何特殊啟動選項、不需要改名的執行檔、也不需要包裝腳本。它和其他所有
遊戲使用同一個啟動選項。

## 啟動選項

```
~/.local/bin/eidos-gui %command%
```

Steam 為 Fallout 4 設定的啟動目標是 `Fallout4Launcher.exe`,從來不是 `Fallout4.exe`,
所以讓 script extender 跑起來,本質上是「怎麼讓 Steam 啟動另一個程式」這個問題。常見
答案是用 bash 改寫 `%command%`:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

或者把 `f4se_loader.exe` 覆蓋到 `Fallout4Launcher.exe` 上,而 Steam 會在每次遊戲
更新時還原它,此後你就在沒有 F4SE 的情況下遊玩,卻沒有任何提示。

Eidos 自己完成這次替換,依據是遊戲描述符。視窗啟動遊戲時,如果載入器在遊戲資料夾、某個
已啟用模組的 `Root/` 或 `Overwrite/Root/` 裡,就用 `f4se_loader.exe` 替換
`Fallout4Launcher.exe`。沒有載入器時它執行 `Fallout4.exe`,狀態列會寫
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
如果那裡兩個檔案都找不到,它就原樣執行啟動器,同樣會說明。若啟動選項改用
`eidos play fallout4 -- %command%`,CLI 只在載入器存在時才替換,否則執行
`Fallout4Launcher.exe`。

還有第二個該避開啟動器的理由:它會重新掃描 `Data` 並重寫 `plugins.txt`,把剛剛部署好的
載入順序一筆勾銷。

你自己打開視窗並按 Run 時,Eidos 會請 Steam 啟動遊戲(`steam -applaunch 377160`),
Steam 再透過上面的啟動選項啟動 Eidos。如果實例中遊戲執行檔旁邊沒有 `f4se_loader.exe`,
它不會這麼做,狀態列會要你先安裝 script extender。Fallout 4 VR 的做法相同,使用
`Fallout4VRLauncher.exe` 和 `f4sevr_loader.exe`。

## Eidos 替你處理的事

| 項目 | Eidos 做什麼 |
|---|---|
| 封裝檔失效 | `Fallout4Custom.ini` 會被寫入 `[Archive]` `bInvalidateOlderFiles=1` 和空的 `sResourceDataDirsFinal=`,這兩個鍵讓 `Data` 之外的散裝檔案根本能被看見。寫進設定檔,而不是遊戲目錄。 |
| INI | `Fallout4.ini`、`Fallout4Prefs.ini` 和 `Fallout4Custom.ini` 屬於設定檔。透過 Eidos 執行的 BethINI Pie 編輯的是設定檔中的副本,Backups -> INIs 會放回 Eidos 在每次執行前保留的那份副本([tools.md](tools.md#bethini-pie))。 |
| 載入順序 | `plugins.txt` 採用 Fallout 4 使用的星號格式(`*` 表示啟用),並遵循 `Fallout4.ccc` 處理隱含的 Creation Club 外掛 |
| LOOT | 排序方式與 Skyrim 相同:`eidos sort <instance>` 會取 `fallout4` 的 masterlist |
| 存檔 | `.fos` 存檔及其 `.f4se` 協同存檔會被列出、複製並按設定檔保存;詳情面板會讀取存檔自身的外掛表,所以一個需要你已停用外掛的存檔會在讀取前就說出來。停用或移除某個外掛被設定檔三個最新存檔用到的模組時,會先詢問。 |
| Root 模組 | 模組隨執行檔一同提供的東西(F4SE 本身、ENB、`dxvk.conf`)都透過 Skyrim 同樣的 `Root/` 機制落到那裡 |

## 版本問題

Fallout 4 不再是 2019 到 2024 年間那個凍結的遊戲。截至 2026 年 8 月,有三條活躍分支,
為其中一條建置的模組 DLL 無法在另一條上載入:

| 分支 | 版本 | F4SE |
|---|---|---|
| 經典版(old-gen) | 1.10.163 | 0.6.23 |
| 次世代版 | 1.10.984 | 0.7.2 |
| 週年版 / Creations | 1.11.137 到 1.11.240 | 0.7.4 到 0.7.9 |

在搭建模組清單之前值得知道的兩個後果:

- 遊戲根目錄裡有 `Creations/` 和 `Mods/` 資料夾,說明你在 1.11.x 這條線上。Eidos 裡
  存檔的詳情面板也會顯示寫下它的建置版本:Fallout 會把它寫進存檔,Eidos 以
  「Game build」呈現。
- F4SE 通常在 Bethesda 更新後一兩天內跟上,但 *Address Library for F4SE Plugins*
  (大多數 DLL 模組靠它解析偏移)按自己的節奏走。在這兩者之間,DLL 模組無法載入。
  不含 DLL 的模組(貼圖、模型、外掛)不受影響,所以剛打完修補程式不是開始搭建模組
  清單的好時機。

一旦你的整套設定跑通,就把遊戲版本凍結進實例:在 Diagnostics 分頁的 Game files 卡片上
按 Freeze this version,或用 `eidos stock-game fallout4 freeze` 查看大小、用
`eidos stock-game fallout4 freeze --yes` 進行複製。Eidos 會把商店安裝複製到
`<instance>/Stock Game`,並從該副本執行遊戲,仍由 Steam 啟動。之後的修補程式改動的是
商店安裝而不是副本,所以你裝的 DLL 保持原本的遊戲版本。在 btrfs 或 XFS 上,副本與商店
共用區塊,在 Steam 改動檔案之前幾乎不佔空間;在 ext4 上則是與遊戲資料夾一樣大的完整
複製。Steam 回報遊戲忙碌或尚未完整安裝時,Eidos 會拒絕。細節見
[usage.md](usage.md#遊戲檔案商店安裝還是副本)。

沒有副本時,請把 Fallout 4 的 Steam 自動更新關掉(內容 -> 更新 ->「僅在啟動遊戲時更新」),
否則下一個修補程式會把你裝的每一個 DLL 都打碎。

## 硬體提示:NVIDIA 上的武器碎片當機

Fallout 4 的武器碎片效果基於 NVIDIA FleX,那是 NVIDIA 在 Pascal 之後停止支援的 PhysX
衍生物。在任何 Turing 及更新的顯示卡上(GTX 16、RTX 20 直到 RTX 50),它都會讓遊戲
當掉。這是遊戲本身的缺陷,與 Linux、Proton 或 Eidos 無關。

兩個辦法,任選其一:在遊戲設定裡關閉「Weapon Debris」,或安裝
*Weapon Debris Crash Fix*(Nexus 48078),它停用的是碎片的碰撞而不是效果本身。

## Fallout 4 的 Health 卡片

Diagnostics 分頁會為下列每一種情況顯示一張 Health 卡片:

- F4SE 外掛裝在 `F4SE/Plugins`,但 `f4se_loader.exe` 不在任何已啟用的模組裡,也不在
  遊戲資料夾裡。遊戲會在沒有它們的情況下啟動。
- 已安裝 `f4se_loader.exe`,但缺少與執行中遊戲版本對應的 `f4se_<version>.dll`
  (1.10.984 對應 `f4se_1_10_984.dll`)。載入器會報錯停止;這通常發生在遊戲更新之後。
- 自上一次經由 Eidos 啟動且未報錯結束以來,遊戲執行檔或它的 Steam 建置改變了。為舊
  版本建置的 F4SE 外掛可能不再載入。
- 優先順序高於 F4SE 的某個模組帶有 F4SE 自身腳本的散裝 `.pex` 副本。舊副本會破壞它
  所宣告的 F4SE 函式,而遊戲不會顯示任何錯誤。
- 設定檔的 `Fallout4Custom.ini` 在 `[General]` 中有 `sTestFile1` 到 `sTestFile10` 行。
  只要設了其中一個,遊戲就會載入那些外掛並忽略 `plugins.txt`。Fix 會移除它們,並把先前
  的檔案保留為 `Fallout4Custom.ini.pre-fix`。
- 某個模組的 BA2 封裝檔以壓縮方式存放 `.wav` 或 `.xwm` 音訊。遊戲播放該聲音時可能
  當掉。請用不壓縮音訊的方式重新封裝。Fallout 4 VR 也有同一張卡片。
- 崩潰記錄器在上次啟動時寫了一份記錄:Buffout 4 在 `Documents/My Games/Fallout4/F4SE`
  裡的 `crash-*.log`,或 `F4SE/Crashlogs` 裡的 `.log`。卡片會指出例外,以及記錄中由你
  的模組安裝的 DLL 和外掛。
- Display 卡片顯示設定檔 `Fallout4Prefs.ini` 中設定的視窗尺寸與模式,Apply 會把
  `iSize W`、`iSize H`、`bFull Screen` 和 `bBorderless` 寫到那裡。
- 某個 Nexus 需求指名了一個 DLC。Eidos 會找與該 DLC 同名的外掛,而 Fallout 4 的 DLC
  外掛名稱不同(Far Harbor 是 `DLCCoast.esm`),所以這張卡片只是建議:請手動確認,
  然後按 Ignore。

兩張載入器卡片有 Open the download page 按鈕,連到 <https://f4se.silverlock.org/>,
還有 Install the downloaded archive 按鈕,它會把載入器放進該模組的 `Root/`。
`eidos check fallout4` 會印出相同的結果,但不含 `.pex` 卡片、Display 卡片和 Nexus
需求卡片,這些只有視窗會計算。

## 如果哪裡不對勁

通用排查清單在 [troubleshooting.md](troubleshooting.md);而 Fallout 特有的
第一個問題永遠是*究竟啟動了哪個執行檔*。視窗啟動遊戲時,會把完整的啟動命令寫進實例的
執行紀錄,所以:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

如果它寫的是 `f4se_loader.exe`,替換成功了。如果寫的是 `Fallout4.exe`,說明
F4SE 沒裝在 Eidos 找得到的地方:它該待在遊戲執行檔旁邊,對於用模組管理器的設定
來說,那就是某個模組的 `Root/` 目錄(或者手動裝進遊戲資料夾本身)。如果寫的是
`Fallout4Launcher.exe`,說明 Eidos 在 root 檢視裡既沒找到載入器也沒找到
`Fallout4.exe`。

當啟動的執行檔正確而遊戲仍然當機時,Test run 和 Find the mod behind a problem 可以在
不動設定檔存檔的前提下把問題縮小到一個模組([usage.md](usage.md#測試執行))。
