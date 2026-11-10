<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders、DLSS、フレーム生成

Community Shaders 1.4+ は独自のアップスケーリング(DLSS 4 / FSR 3.1 / XeSS。
別パッケージ「Upscaling - Community Shaders」経由)と FSR 3.1 のフレーム生成を
備えています。どれも Linux の Eidos 越しに動きます - CS もその追加パッケージも
普通の MOD として入り、統合ビューはそれらの DLL を他と同じように供給します -
けれども三つの設定はゲームの中から見えず、どれも、エラーなしに機能を何もしない
状態にします。NVAPI の変数、排他的フルスクリーン、Reflex の FPS 上限です。

Diagnostics タブは、Wine プレフィックス(Steam の Proton プレフィックス、または
Heroic が用意したもの)を持つゲームについて、最初の二つと、下の ENB の行を確認します。
Reflex の上限は確認しません。インスタンスが `Stock Game` のようなコピーからゲームを
動かしているときは、確認はストアのインストールではなくコピーのファイルを読みます。
`eidos check <instance>` は同じカードをターミナルに出力します(Fix ボタンはありません)。

- NVAPI カードは、Skyrim SE または Enderal SE の Steam 版で、NVIDIA ドライバーが
  読み込まれていて(`/proc/driver/nvidia/version` がある)、統合した `Data` の
  `Shaders/Upscaling` に何かがあり、ゲームの Proton ビルドが NVAPI を既定でオフに
  していて、Steam の起動オプションにも Eidos 自身の環境にもプロファイルの
  `launch-env.ini` にも `PROTON_ENABLE_NVAPI=1` がないときに出ます。
- 排他的フルスクリーンのカードは、Upscaling パッケージが入っていて、プロファイルの
  `SkyrimPrefs.ini` が `bFull Screen=1` で、`SKSE/Plugins/SSEDisplayTweaks.ini` が
  `Fullscreen=false` と `Borderless=true` の両方を設定していないときだけ出ます。Fix
  ボタンは、プロファイルの `SkyrimPrefs.ini` に `bFull Screen=0` と `bBorderless=1` を
  書きます。この確認は Enderal SE でも `SkyrimPrefs.ini` を読みますが、Enderal SE の
  プロファイルにあるのは `EnderalPrefs.ini` なので、Enderal SE では代わりに下の
  Display カードを使ってください。
- ENB カードは、ルートビューで優先される `enblocal.ini` の `[GLOBAL]` に
  `LinuxVersion=true` がないゲームで出ます。こちらにも Fix ボタンがあります。

## DLSS が必要とする起動オプション

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

[install.md](install.md) と同様に、自分の `eidos-gui` か `eidos-linux` の絶対パスを
使ってください。カードは設定する行を示します。ゲームの現在の起動オプションの前に
`PROTON_ENABLE_NVAPI=1` を付けたもの、起動オプションがなければ、実行中の Eidos の
パスから組み立てた行です。

NVIDIA NVAPI 層(dxvk-nvapi)がないと、CS は DLSS を初期化できず、画面に理由を
出さないまま FSR アップスケーリングへ落ちます。Proton ビルドがこの層を自動でオンに
するかどうかは、その `proton` スクリプトに書かれていて、Eidos はそこの `use_nvapi`
行を読みます。オプトアウト用のスイッチ(`disablenvapi`)は既定でオンを意味します。
GE-Proton11-9 と Proton 11.0 がこの形です(2026-11-09 に確認)。オプトイン用の
スイッチ(`enablenvapi`)はオフを意味し、そのビルドだけがこの変数を必要とします。
設定しても、NVIDIA でないマシンでは何も失いません。フレーム生成そのものは FSR 3.1 で
NVAPI を必要とせず、必要なのは DLSS アップスケーラだけです。

1つのプロファイルだけに設定するには、そのプロファイルの `launch-env.ini` に
`env=PROTON_ENABLE_NVAPI=1` を書きます(View -> INI editor の最後のタブ)。

## フレーム生成にはボーダーレスウィンドウが要る

CS のフレーム生成は D3D12 のプレゼンテーションプロキシ上で動き、排他的フルスクリーンを
きっぱり拒みます。`SkyrimPrefs.ini` に `bFull Screen=1` があると、フレーム生成は
決して働きません。エラーもメッセージもなく、ゲームは基本フレームレートのままです。
SSE Display Tweaks は、INI が何と言おうとエンジンの層でモードを設定します:

```ini
[Render]
Fullscreen=false
Borderless=true
```

見た目のウィンドウは同じ(ネイティブ解像度のボーダーレス)です。変わるのはエンジンが
報告するモードだけで、CS が確かめているのはそのモードです。

Diagnostics の Display カードは、Skyrim SE、Enderal SE、Fallout 4 で、サイズとモードを
一度に設定します。接続されている最大の画面のサイズを表示します。Borderless window を
選んで Apply すると、プロファイルの `SkyrimPrefs.ini`(Enderal SE なら
`EnderalPrefs.ini`、Fallout 4 なら `Fallout4Prefs.ini`)に `iSize W`、`iSize H`、
`bFull Screen=0`、`bBorderless=1` が書かれます。SSE Display Tweaks がウィンドウを
設定している場合は、Apply はその INI のコピー(Overwrite 内の
`SKSE/Plugins/SSEDisplayTweaks.ini`)にも `Resolution`、`Fullscreen=false`、
`Borderless=true` を書き、MOD 自身のファイルはそのまま残ります。Start the game at
once when a store opens Eidos をオン(Preferences, General, Running a game)にして
いても、プロファイルにまだウィンドウサイズがなく、SSE Display Tweaks もウィンドウを
設定していない場合はウィンドウが開くので、最初の起動の前にサイズを設定できます。

有効化の条件はあと二つ。失敗の仕方は同じく無言です:

- ディスプレイのリフレッシュレートが 120 Hz 以上であること。あるいは CS の
  アップスケーリング設定で `frameGenerationForceEnable` を立てること。フレーム生成は
  提示レートを倍にするので、結果を表示できないディスプレイでは CS が起動を拒みます。
- Upscaling パッケージが入っていること(その `Data/Shaders/Upscaling/` 以下に
  Streamline と FidelityFX の DLL があります)。これがないと CS はメニュー項目を
  出すだけで、何も有効化できません。

## Reflex のフレームレート上限が出力を絞め殺すことがある

CS の Reflex 設定は独自の FPS 上限(`reflexFPSLimit` と `reflexUseFPSLimit`)を
持ちます。昔の値のまま残った上限 - こちらでは古い調整の 79 でした - はフレーム生成の
下流に座り、生成されたフレームをちょうど刈り取ります。基本 60 が 120 に倍増され、
79 に戻されると「フレーム生成が効いていない」と読めるわけです。144 Hz のディスプレイ
なら通常の Reflex 上限は約 138 です。生成分が見当たらないと感じたら確認してください。

## 既知の相互作用: SSE Display Tweaks と黒画面

FG + Display Tweaks + DXVK の組み合わせには既知の黒画面不具合があります。順に:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. それで足りなければ、ゲーム実行ファイルの隣に `dxvk.conf`(MOD の `Root/`
   ディレクトリがそこへ置いてくれます)を用意し
   `dxvk.enableGraphicsPipelineLibrary = False`

## テクスチャ MOD が混在する場合: PGPatcher

Community Shaders は parallax・complex material・PBR を描画できますが、メッシュ
がそのように設定されている場所に限られます。従来はあらかじめパッチ済みの
メッシュを導入し、テクスチャが欠けている箇所では効果を切り戻す MOD を重ねる
必要がありました。対応範囲は手持ちのメッシュ次第で、導入時に下した判断を実行時に
取り消すために CPU も使っていたのです。

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) はこれ
を逆転させます。どの種類のシェーダーのテクスチャでも好きに導入すれば、各面が
正しいものを使うようにメッシュとプラグインを書き換えます。旧来の方法では
扱えなかった、プラグイン内の代替テクスチャレコードも含めてです。これがないと、
メッシュが要求しないシェーダーを必要とするテクスチャは読み込まれても意図どおりには
描画されず、テクスチャの種類が混在した構成でいちばん起きやすくなります。

Linux で実行する前に二点:

- 引数 `--ignore-mo2vfscheck` が必要です。ないと MO2 について文句を言って
  即座に終了します。Eidos が付与します。フラグの内容とここで確認が失敗する
  理由は[ツール](tools.md#pgpatcher-に---ignore-mo2vfscheck-が要る理由)を参照;
- メッシュを編集するため、テクスチャ MOD をすべて導入した後、最終的な
  メッシュを見る必要がある DynDOLOD の前に実行します。後からテクスチャを
  変えたら、この順で両方を実行し直すことになります。

出力は Overwrite には入りません。PGPatcher はランチャーで設定した Output location
にだけ書き込み、`Data` の中のフォルダは受け付けません。その場所には専用の
フォルダを指定してください。たとえばインスタンスの `mods/` の下に新しく作る
`PGPatcher Output` フォルダです。PGPatcher の実行中はその MOD を無効にして
おいてください。PGPatcher は書き直す前にフォルダを空にし、Eidos はビューが
マウントされている間、有効な MOD が変わらないことを前提にしているためです。
終わったら高い優先度で有効にしてください。勝つために作られたものです。

PGPatcher のエントリに Capture output into の MOD を設定し、その MOD を Output
location としても使うことはしないでください。Eidos は実行のたびに PGPatcher 自身の
`cfg/` と `log/` をその MOD へ移すため、PGPatcher は「Output directory has
non-PGPatcher related files」で止まります。

## Proton 上の ENB: LinuxVersion

Jackify は、Proton 上でインストールするすべての ENB について、`enblocal.ini` の
`[GLOBAL]` セクションに `LinuxVersion=true` を書き込みます。ENB 自身のドキュメントに
このキーの説明はありません。Diagnostics タブの Health カードはこの行がない
`enblocal.ini` を知らせ、Fix ボタンは、優先される `enblocal.ini` のコピーにこの行を
設定して `overwrite/Root/` に書き込みます。ENB の MOD のファイルはそのまま残ります。

別のカードは、ゲームフォルダに ENB(`enblocal.ini`、`enbseries.ini`、`enbseries/`
フォルダのどれか)があり、同時に有効な MOD が Community Shaders
(`SKSE/Plugins/CommunityShaders.dll`)を含んでいる場合に出ます。二つは一緒に動かせます。
画面がおかしければ、どちらかをその INI でオフにしてください。

## あとで数字をどう読むか

生成フレームは提示側だけの話です。エンジンは依然として基本レートでシミュレートし、
Havok も基本レートで刻み、*エンジンの*フレームを数えるもの(CS 自身のカウンタを
含む)は表示が ~120 でも ~60 を報告し続けます。カウンタは正しく、エンジン自体の
フレームレートを上げるのと違ってフレーム生成が物理的に安全なのは、このためです。
画面に数値が欲しければ、起動オプションの `DXVK_HUD=fps` がカウンタを出します。

規則は一つ。ドライバ側の補間(NVIDIA Smooth Motion、
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`)と CS のフレーム生成は競合する技術です。
どちらか一方だけを使い、両方は使わないでください。
