<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders、DLSS 與影格生成

Community Shaders 1.4+ 自帶升頻(DLSS 4 / FSR 3.1 / XeSS,透過單獨的
「Upscaling - Community Shaders」套件)以及 FSR 3.1 影格生成。這些在 Linux 上都能
穿過 Eidos 運作 —— CS 及其附加套件按一般模組安裝,合併檢視像對待其他檔案一樣提供
它們的 DLL —— 但有三項設定在遊戲裡看不出來,而且每一項都會讓功能不報錯地什麼也不做:
NVAPI 變數、獨占全螢幕和 Reflex 的 FPS 上限。

對於有 Wine 前綴的遊戲(Steam 的 Proton 前綴,或 Heroic 建立的前綴),Diagnostics 分頁
會檢查前兩項以及下面的 ENB 那一行。Reflex 上限不檢查。實例從遊戲副本(例如 `Stock Game`)
執行遊戲時,檢查讀的是副本的檔案,而不是商店安裝。`eidos check <instance>` 在終端機裡
印出相同的卡片,只是沒有 Fix 按鈕。

- NVAPI 卡片出現的條件:Skyrim SE 或 Enderal SE 的 Steam 副本,已載入 NVIDIA 驅動程式
  (`/proc/driver/nvidia/version` 存在),合併後的 `Data` 在 `Shaders/Upscaling` 裡有
  東西,遊戲的 Proton 版本預設關閉 NVAPI,而且 Steam 的啟動選項、Eidos 自己的環境或
  設定檔的 `launch-env.ini` 都沒有設定 `PROTON_ENABLE_NVAPI=1`。
- 獨占全螢幕卡片只在已安裝 Upscaling 套件、設定檔的 `SkyrimPrefs.ini` 有
  `bFull Screen=1`、且 `SKSE/Plugins/SSEDisplayTweaks.ini` 沒有同時設定
  `Fullscreen=false` 和 `Borderless=true` 時出現。它的 Fix 按鈕會把 `bFull Screen=0`
  和 `bBorderless=1` 寫入設定檔的 `SkyrimPrefs.ini`。這項檢查對 Enderal SE 也讀
  `SkyrimPrefs.ini`,而 Enderal SE 設定檔裡是 `EnderalPrefs.ini`,所以 Enderal SE 請改用
  下面的 Display 卡片。
- ENB 卡片出現在任何 root 檢視中生效的 `enblocal.ini` 在 `[GLOBAL]` 裡沒有
  `LinuxVersion=true` 的遊戲。它也有 Fix 按鈕。

## DLSS 需要的啟動選項

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

請使用你自己的 `eidos-gui` 或 `eidos-linux` 的絕對路徑,如 [install.md](install.md) 所示。
卡片會給出該設定的那一行:遊戲目前的啟動選項,前面加上 `PROTON_ENABLE_NVAPI=1`;若
目前沒有啟動選項,則用執行中 Eidos 的路徑組出那一行。

沒有 NVIDIA NVAPI 層(dxvk-nvapi),CS 無法初始化 DLSS,會退回 FSR 升頻,畫面上不會有
任何提示說明原因。Proton 版本是否自己開啟這一層,寫在它的 `proton` 腳本裡,Eidos 讀
那裡的 `use_nvapi` 一行。選擇退出的開關(`disablenvapi`)表示預設開啟:GE-Proton11-9 和
Proton 11.0 採用這種形式(2026-11-09 檢查)。選擇加入的開關(`enablenvapi`)表示預設
關閉,只有那個版本需要這個變數。在非 NVIDIA 機器上設定它毫無代價。影格生成本身是
FSR 3.1,不需要 NVAPI;只有 DLSS 升頻器需要。

只想為一個設定檔設定時,把 `env=PROTON_ENABLE_NVAPI=1` 寫進該設定檔的
`launch-env.ini`(View -> INI editor,最後一個分頁)。

## 影格生成需要無邊框視窗

CS 的影格生成建立在 D3D12 呈現代理之上,並且乾脆拒絕獨佔全螢幕。`SkyrimPrefs.ini`
裡的 `bFull Screen=1` 意味著它永遠不會啟動:沒有報錯,沒有提示,遊戲停留在基礎影格率。
SSE Display Tweaks 在引擎層面設定模式,不管 INI 怎麼寫:

```ini
[Render]
Fullscreen=false
Borderless=true
```

視窗看起來一模一樣(無邊框、原生解析度)。變的只是引擎回報的模式,而 CS 檢查的正是
這個模式。

Diagnostics 裡的 Display 卡片一步設定尺寸和模式,適用於 Skyrim SE、Enderal SE 和
Fallout 4。它顯示最大的已連接螢幕的尺寸。選了 Borderless window 後,Apply 會把
`iSize W`、`iSize H`、`bFull Screen=0` 和 `bBorderless=1` 寫入設定檔的
`SkyrimPrefs.ini`(Enderal SE 是 `EnderalPrefs.ini`,Fallout 4 是 `Fallout4Prefs.ini`)。
由 SSE Display Tweaks 設定視窗時,Apply 還會把 `Resolution`、`Fullscreen=false` 和
`Borderless=true` 寫入 Overwrite 中它的 INI 的一份副本(`SKSE/Plugins/SSEDisplayTweaks.ini`),
模組自己的檔案保持原樣。開啟 Start the game at once when a store opens Eidos
(Preferences, General, Running a game)時,如果設定檔還沒有視窗尺寸而且 SSE Display
Tweaks 沒有設定視窗,視窗仍會開啟,以便在第一次啟動前設好尺寸。

還有兩個啟用條件,失敗方式同樣安靜:

- 螢幕更新率 120 Hz 或更高,或者在 CS 的升頻設定裡開啟 `frameGenerationForceEnable`。
  影格生成會把呈現影格率翻倍,所以 CS 拒絕在顯示不出結果的螢幕上啟用它。
- 已安裝 Upscaling 套件(它的 `Data/Shaders/Upscaling/` 目錄裡放著 Streamline 與
  FidelityFX 的 DLL)。沒有它,CS 會顯示選單項目卻什麼也開不了。

## Reflex 的影格率上限可能把輸出掐死

CS 的 Reflex 設定自帶 FPS 上限(`reflexFPSLimit`,搭配 `reflexUseFPSLimit`)。停留在
舊值的上限 —— 我們的是 79,來自一次舊的調校 —— 位於影格生成的下游,恰好把它
產出的影格砍掉:基礎 60 翻倍到 120,再被壓回 79,看上去就是「影格生成沒起作用」。
144 Hz 螢幕上常規的 Reflex 上限約為 138。只要覺得生成的影格不見了,就查一下它。

## 已知互動:與 SSE Display Tweaks 一起出現黑畫面

FG + Display Tweaks + DXVK 這個組合有已知的黑畫面故障。按順序修:

1. `SSEDisplayTweaks.ini`:`DisableBufferResizing=true`
2. 若仍不行,在遊戲執行檔旁放一個 `dxvk.conf`(模組的 `Root/` 目錄就能放到那裡),
   內容為 `dxvk.enableGraphicsPipelineLibrary = False`

## 混用貼圖模組:PGPatcher

Community Shaders 能算出視差、complex material 與 PBR,但只在網格已為此設定好的
地方。過去這意味著先裝一堆預先打好補丁的網格,再裝一個模組,在貼圖缺失的地方把
效果重新關掉:覆蓋範圍受限於你手頭恰好有的網格,還要在執行時花 CPU 去撤銷安裝時
做出的選擇。

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) 把它反了
過來。你隨意安裝任何著色器類型的貼圖,它會重寫網格與外掛,讓每個表面都用上對的
那一種 —— 包括外掛裡的備用貼圖記錄,那是舊方法覆蓋不到的。沒有它時,需要某種著色器
而其網格並未要求該著色器的貼圖會被載入,但不會按預期呈現,這在混用多種貼圖類型的
配置上最常見。

在 Linux 上執行前有兩點:

- 它需要參數 `--ignore-mo2vfscheck`,否則會立刻結束並抱怨 MO2。Eidos 會提供 ——
  這個旗標是什麼、檢查為何在這裡失敗,見[工具](tools.md#為什麼-pgpatcher-需要---ignore-mo2vfscheck);
- 它會修改網格,所以要在所有貼圖模組裝完之後、在需要看到最終網格的 DynDOLOD
  之前執行。之後再改貼圖,就得按這個順序把兩者都重跑一遍。

它的輸出不會進入 Overwrite。PGPatcher 只寫入其啟動器裡 Output location 指定的
資料夾,而且不接受 `Data` 裡面的資料夾。把這個位置指向它專用的資料夾,例如在實例的
`mods/` 底下新建一個 `PGPatcher Output` 資料夾。PGPatcher 執行期間讓那個模組保持
停用,因為 PGPatcher 會先清空資料夾再重新寫入,而 Eidos 假定檢視掛載期間啟用的模組
不會變動。執行結束後再以高優先權啟用它:它本來就是要贏的。

不要給 PGPatcher 的項目設定 Capture output into 模組,再把該模組當作 Output location。
每次執行之後,Eidos 會把 PGPatcher 自己的 `cfg/` 和 `log/` 搬進那個模組,PGPatcher
接著就會回報「Output directory has non-PGPatcher related files」並停止。

## Proton 下的 ENB:LinuxVersion

Jackify 為它在 Proton 下安裝的每個 ENB,都會在 `enblocal.ini` 的 `[GLOBAL]` 區段寫入
`LinuxVersion=true`。ENB 自己的文件沒有說明這個鍵。Diagnostics 分頁裡的一張 Health
卡片會提示缺少這一行的 `enblocal.ini`,它的 Fix 按鈕則把生效的 `enblocal.ini` 加上這
一行後複製一份寫入 `overwrite/Root/`,ENB 模組的檔案保持不變。

另一張卡片會在 ENB 位於遊戲資料夾(有 `enblocal.ini`、`enbseries.ini` 或
`enbseries/` 資料夾),而某個已啟用的模組帶有 Community Shaders
(`SKSE/Plugins/CommunityShaders.dll`)時提示。兩者可以一起執行。如果畫面看起來不對,
請在其中一個的 INI 裡把它關掉。

## 事後怎麼讀這些數字

生成的影格只存在於呈現端:引擎仍以基礎影格率模擬,Havok 仍以基礎影格率步進,一切統計
*引擎*影格的東西(包括 CS 自己的計數器)都會繼續報 ~60,而螢幕顯示 ~120。計數器是對的,
也正因如此,影格生成對物理是安全的,而抬高引擎自身影格率則不然。如果想在畫面上看到
計數器,可以在啟動選項裡加 `DXVK_HUD=fps`。

一條規則:驅動層插影格(NVIDIA Smooth Motion,`NVPRESENT_ENABLE_SMOOTH_MOTION=1`)
與 CS 的影格生成是互相競爭的技術。二選一,絕不要同時開。
