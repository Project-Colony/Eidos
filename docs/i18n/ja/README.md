<!-- eidos-i18n: source=README.md sha=37e97bafbd2cf43f2101a28b23c1464420a5f9ec -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**ゲームフォルダに一切触れない、ネイティブ Linux の MOD マネージャ。**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#インストール)

Eidos は、Windows で Mod Organizer 2 が Bethesda のゲームに与えるものを Linux で
与えます。起動ごとに作られる、MOD の統合ビューです。Windows API のフックの代わりに
Linux のマウント名前空間と FUSE を使い、マネージャは Wine なしで動きます。ゲーム
フォルダには何もコピーされないので、あとで片付けるものもありません。Eidos には
Stellar Blade、Stardew Valley、7 Days to Die、Crimson Desert の定義もあります。

Steam 版では、ゲームの起動オプションから Eidos を実行します。Heroic や Legendary
の GOG 版と Epic 版では、ラッパーとして実行します。ゲームは常に、そのコピーを
持つストア経由で起動します。Eidos が自分で Proton を使って起動することはありません。

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **状況:** Skyrim SE は Eidos 越しに日常的に遊ばれています。SKSE、script
> extender のプリローダ、Creation Club、LOOT で並べ替えたロード順、プロファイル
> ごとのセーブがそろっています。実際のプレイで実証済みのゲーム系列は今のところ
> 一つです。ほかに Bethesda のゲーム 10 本とそれ以外の 4 本が組み込み済みで、
> テスターを待っています。

## なぜ Eidos か

統合ビューは、ゲームのプロセスツリーだけが見えるプライベートなマウント名前空間の
中にマウントされます。ほかのプログラム、二本目のゲームも含めて、それを見ることは
なく、そのための許可も要りません。ゲームが終了したり強制終了されたり、マシンの電源が
落ちたりすると、名前空間はプロセスとともに消え、インストール先は元のままです。

プロファイルは、それぞれ自分の MOD 一覧、プラグインの順序、INI、セーブを持ちます。
起動時にプラグインのファイルとセーブのフォルダがゲーム自身のパスへ bind-mount
されるので、ゲームがそこへ書いたものはプロファイルに着地します。プロファイルを
切り替えれば、そのすべてが切り替わります。

Eidos に必要なものは、setuid のヘルパも、システムのデーモンも、`sudo setcap` も、
`/etc/fuse.conf` の編集もありません。FUSE デーモンはあなたのユーザーで 1 回の起動の
間だけ動き、ゲームとともに終了します。インストールするプログラムは一つ、設定する
Steam の起動オプションも一つです。

実行のたびに、Eidos はプラグイン一覧のスナップショットを保存します。クラッシュで
一覧が壊れたら、Diagnostics がそのスナップショットと比べ、Restore the pre-session
order で元の順序に戻せます。ロード順を消してしまう取り込みは、理由を添えて
拒否されます。

Test run は、プロファイルの MOD ありで、MOD なしで、または選択した MOD とそのマスター
だけでゲームを起動します。その実行のセーブ、INI の変更、プラグインの状態、
Overwrite の出力、そのほかの My Games のファイル(ログを含む)は
`<instance>/test-run/` に入ります。ゲームが終了すると、Eidos はこの 5 つのグループを
並べ、それぞれを残すか捨てるかを選べます。Wine のレジストリなど、それ以外の書き込みは
いつもの場所に行きます。Diagnostics の Find the mod behind a problem は、疑わしい MOD
の半分とそれに必要なマスターを足して Test run を繰り返します。毎回、問題が起きたかを
答えると、Eidos は容疑者を絞り込み、MOD か、互いのマスターを必要とする MOD の組を
特定します。容疑者が 200 個なら、実行は約 8 回です。始める前に一覧で選んだ MOD は、
どの段階でもオンのままです。詳細は [usage.md](docs/guide/usage.md#テスト実行) と
[usage.md](docs/guide/usage.md#問題の原因になっている-mod-を見つける) に。

### 他との比較

| | Eidos | Wine 上の MO2 | Fluorine-Manager | Limo / リンク配置型 |
|---|---|---|---|---|
| マネージャがネイティブに動く | はい | いいえ、Wine の中の Windows アプリ | はい(Qt 移植) | はい |
| ゲームディレクトリに触れない | はい、常に | はい | はい | いいえ、リンクが書き込まれる |
| マウントが見える範囲 | ゲームだけ | ゲームだけ | システム全体 | 該当なし |
| クラッシュ後の後片付け | 不要 | 不要 | 残留マウントの復旧 | 手動で解除 |
| root MOD(ENB、プリローダ) | はい、ネイティブ | プラグインが必要 | プラグインが必要 | 一部 |
| 必要な権限 | なし | なし | `/etc/fuse.conf` の編集 | なし |

### 速さ

| | 以前 | 現在 |
|---|---|---|
| セーブの読み込み | 約 20 秒 | 6-7 秒 |
| 1 セッションのディレクトリ読み取り | 560 万 | 33.5 万 |

セル移動は即座です。ファイルを一つ探すのに以前は 50 個の MOD を順に問い合わせ、
フォルダを一つ列挙するのにそれを 50 回繰り返していました。いまは読み取り専用の層を
マウント時に一度だけ索引化するので、ファイルの検索はハッシュの引き一回で済み、
フォルダの統合した一覧もマウント時に一度だけ計算されます。数値はベンチマークでは
なく、普通に遊んでいる実際のインスタンスで計測したものです:
[internals/performance.md](../../../docs/internals/performance.md)。

### 先行事例と謝辞

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) と
  [usvfs](https://github.com/ModOrganizer2/usvfs): Eidos が再現する意味論と、
  その同等性を突き合わせて調べたコードベース
- [LOOT](https://loot.github.io/): 並べ替えのエンジン、libloot 経由
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager)、
  [Limo](https://github.com/limo-app/limo) をはじめとする Linux のマネージャ群。
  これを解決してほしいコミュニティが存在することを示しています

## できること

### MOD

Eidos は、単純なアーカイブ、FOMOD ウィザード、Wrye Bash の BAIN パッケージ、
対応する OBMM スクリプトを含む OMOD コンテナをインストールし、それ以外は手動の
ピッカーで扱います。root MOD(script extender のプリローダ、ENB、Engine Fixes)は、
Root Builder プラグインもインストール先へのコピーも要らず、ネイティブに動きます。
個々のファイルを隠す、セパレーターで MOD をまとめる、選択を目的の位置へ移す、MOD
ごとにメモとカテゴリを持つ、MO2 のプロファイルを取り込む、といったことができます。

一覧は MO2 と同じように動きます。任意の 8 列とその並べ替え、カテゴリやソースに
よるグループ化、ダブルクリックの操作、入力でのジャンプ、復元するまで何もしない
MOD ごとのバックアップ、そしてこのゲームが読み込まない構造の MOD や別のゲーム用に
ダウンロードされた MOD へのフラグ。フィルターは、Nexus の状態(Endorse 済み、
Track 中、更新あり)、メモ、隠しファイル、内容の種類(プラグイン、アーカイブ、
テクスチャ、メッシュ、スクリプト、サウンドなど)で MOD を選びます。ファイル
ツリーでは、フォルダの作成、名前の変更、削除、ファイルを開く操作ができ、何も
起動せずに画像とテキストをプレビューできます。

Ctrl+Z と Ctrl+Shift+Z で、MOD 一覧とロード順の変更を取り消し、やり直せます。
プロファイルの新しい方から 3 つのセーブのどれかが使う MOD を無効化または削除する
ときは、先に確認が出て、プラグイン名が示されます(Skyrim と Fallout 4 のセーブ)。
FOMOD の MOD を再インストールまたは更新すると、前回のインストール時の回答が入った
状態でウィザードが開きます。MOD の information は、その MOD が必要とする MOD と、
その MOD を必要とする MOD を、有効なプラグインのマスターから読んで並べます。

### プラグイン

ロード順には、LOOT による並べ替えが組み込まれています。ほかにも、ゲームが計算する
とおりの MOD インデックス、マスター欠落の警告、そして DLC と Creation Club の内容が
管理外の行として表示されます。ヘッダーには、各インデックス空間(full、medium、
light)で使ったスロット数が出ます。プラグインにカーソルを合わせると、作者、説明、
ヘッダーのバージョンが出ます。Skyrim 系のエンジンでは、同名の INI をゲームが読み込む
プラグインに印が付きます。

LOOT が dirty と示したプラグインは、xEdit の QuickAutoClean で、その場ではなく
`Cleaned plugins` MOD の中にクリーニングされます。Flag as light, into a new mod は
プラグインの light コピーを書き、新しい方のセーブのどれかがそのプラグインを full
プラグインとして読み込んでいる場合は拒否します(Skyrim と Fallout 4 のセーブ)。

### インスタンス

インスタンスはグローバルで `~/.local/share/Colony/Eidos/instances/` の下に置かれるか、
ポータブルで、好きな場所(2 台目のドライブやゲーム用パーティション)にある自己完結
したフォルダです。MO2 のように移動でき、独立しています。ポータブルインスタンスは
セッションをまたいで記憶されます。GUI、Steam からの起動、すべての CLI コマンドが、
最後に使ったものに従い、ゲーム ID を取る場所では、どのコマンドもフォルダを取れます。
詳細は [usage.md](docs/guide/usage.md#インスタンス-グローバルとポータブル) に。

インスタンスは、インスタンス内にあるゲームフォルダのコピーからゲームを動かすことも
できます。Eidos が Wabbajack のリストを初めて開くと、そのリスト自身のコピーを使います。
`ModOrganizer.ini` が指すフォルダがあればそれを、なければ `Stock Game` か `Game Root`
フォルダを使います。Diagnostics の Game files カードにある Freeze this version は、
ストアのインストールを `<instance>/Stock Game` にコピーするので、ストアの更新で
ゲームが動かすファイルが変わることはなくなります。両方のフォルダがブロックを共有
できる 1 つのファイルシステム(btrfs や XFS など)にあるときは、コピーはストアの
インストールとブロックを共有します(reflink)。ext4 や、2 つのファイルシステムを
またぐ場合は完全なコピーになり、先にサイズと空き容量が表示されます。ゲームは今も
ストアが起動します。Return to the store version と Delete the copy で元に戻せ、
端末では
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
で同じことができます。Morrowind と、`bUseMyGamesDirectory=0` の Oblivion は、コピー
からは動かせません。詳細は
[usage.md](docs/guide/usage.md#ゲームファイル-ストアのインストールかコピーか) に。

### プロファイル

各プロファイルは、自分の MOD の順序、プラグインの状態、INI、セーブを持ちます。
セーブは解析されて現在のプラグインと比べられ、セーブが必要とするものを有効にする
ボタンがあり、セッションのたびに Steam Cloud 用にプレフィックスへコピーし戻されます。
Steam Cloud が別のデバイスから持ってきたセーブや、Eidos なしの起動がプレフィックスに
残したセーブは、Saves タブに一覧されます。Import でプロファイルにコピーされ、
プロファイルがすでに使っている名前のセーブは、置き換えられずに新しい `orphan-` 名で
入ります。

プロファイルは、そのゲームとツールの実行に対する起動変数とラッパー(INI エディタで
1 行に 1 つ、`env=NAME=value` と `wrapper=command`)を持てます。Skyrim SE、
Enderal SE、Fallout 4 では、Diagnostics の Display カードが、プロファイルの INI
コピーにウィンドウのサイズとモードを設定します。Backups はプロファイルの INI の
復元ポイントを保持し、Eidos は BethINI Pie の実行のたびにその前に 1 つ作ります。
プロファイルのデスクトップショートカットは、ゲームをそのストア経由で起動します。
`eidos sessions` は記録された実行を一覧し、一つを現在の状態と比べます。

### Nexus

アカウントをつなぐと、Mod Manager Download のリンクを受け取り、更新を確認し、作者と
そのプロフィールを見られます。各 MOD は、そのファイルの取得元の Nexus ドメインで確認、
Endorse、リンクされるので、Enderal SE のインスタンスにある Skyrim SE の MOD は
Skyrim SE のサイトのままです。更新は、作者が Nexus に設定した後継ファイルの連鎖を
たどり、MOD のフォルダを新しいファイルで置き換え(古いもののバックアップを提案
します)、優先度と有効状態を保ち、インストール時に保存した FOMOD の回答を再生します。
`eidos nexus update <instance> --apply` は、プレミアムアカウントなら端末で同じことを
行い、無料アカウントならファイルのページを表示します。Track と Endorse は MOD の
メニューにあり、更新確認のたびに Nexus アカウントと同期されます。MOD の information
には、説明と変更履歴をプレーンテキストで見せる Nexus page タブがあり、General タブ
の末尾には、直近の更新で何が変わったかが出ます。追加、削除、サイズ変更されたファイル、
追加または削除されたマスターです。Install downloads when they finish(Preferences,
Nexus。既定ではオフ)は、ウィンドウが開いている間、ダウンロードが終わるとすぐに
インストールします。

コレクションは、そのレシピから、再開できる進捗と、未対応の手順についての明示的な
レポートとともにインストールされます。照合は元のファイルを厳密に確認し、不完全な
古いメタデータは未検証のままです。コレクションのペインは、ゲーム向けに公開された
コレクションを閲覧と検索でき、インストール済みのコレクションに新しいリビジョンが
あるときを示し、無料アカウントには足りないファイルを 1 ページずつ案内します。
ドラッグでコレクションの順序の規則が崩れると、Diagnostics がそれを告げ、Restore
collection order が MOD を元に戻します。Downloads タブは、アーカイブの絞り込み、
並べ替え、非表示、削除を行います。Offline モード(Preferences, Nexus)は、Nexus への
アクセスと 1 日 1 回のリリース確認を止めます。

### ツール

xEdit、BodySlide、DynDOLOD などの MOD ツールは、ゲームの Wine プレフィックス
(Steam の Proton プレフィックス、または Heroic が用意したもの)の中で、統合ビュー
越しに動きます。ツールにはあなたの MOD が見え、出力は Overwrite に入り、ワン
クリックでその出力が本物の MOD になります。Skyrim SE では、Eidos は SSEEdit、SSEEdit
QuickAutoClean、PGPatcher、Pandora、Nemesis、FNIS、BodySlide、DynDOLOD、TexGen、
xLODGen、Synthesis、BethINI Pie を、ゲームフォルダ、MOD の中、またはゲームの隣に
置いたツールフォルダから見つけ、それぞれに必要なランタイムを用意します。ほかの
ゲームには、それぞれの一覧があります。Pandora、Nemesis、FNIS、BodySlide、Synthesis は、
出力を名前付きの MOD(`Nemesis Output` など)に書きます。

リリースバンドルには、Microsoft の Visual C++ 2015-2022 と .NET 8 Desktop Runtime の
インストーラー、そして .NET 10 ランタイムが入っています。Eidos はそれらのインストーラー
で `vcrun2022` と .NET 8 を、winetricks なしでインストールし、バンドルがなければ
同じファイルを Microsoft からダウンロードします。どれも Eidos に組み込まれた SHA-256
と照合されます。Synthesis には .NET 10 SDK が使われます。300 MB のダウンロードで、
バンドルには入っていません。そのほかの前提条件(.NET Framework 4.8、XACT、古い .NET
のバージョン)は、今も winetricks を通ります。ツールの前提条件のインストールを頼む
まで、何もダウンロードされません。

使うツールはピン留めし、使わないものは隠し、ツールが独立した Steam アプリのときは
専用の Steam AppID を与え、Eidos を開かずに統合ビュー越しに起動する `.desktop`
ショートカットを書けます。`eidos tool <game> import-mo2` は、リストのフォルダの
`ModOrganizer.ini` にある実行ファイルを一覧し、`--write` でそれを追加します。Data
タブの exe は、1 回だけ実行するかツールとして追加でき、`eidos tool <instance> run
--exe <path>` は端末から 1 つ実行します。Ask before playing をオンにすると、生成ツール
の出力が古いとき、Run が先に確認します。

Stop は、実行中のゲームやツールを終わらせます。File メニューは、ゲームのプレフィックスで
winecfg、regedit、Wine のファイルエクスプローラー、Windows のアンインストーラーを
実行します。Diagnostics の Rebuild prefix は、Steam が新しいものを作るようにプレ
フィックスの名前を変え、何も削除しません。`eidos shell <instance>` は統合ビューの
シェルを開くか、`--` の後に与えたコマンドを実行し、それが書いたものは Overwrite に
入ります。

ウィンドウがバックグラウンドにある間、Eidos は、ツールの実行、コレクションのインストール、
pack や unpack、最後に実行中だったダウンロードが終わったとき、そして LOOT の並べ替えが
失敗したときに、デスクトップ通知を送れます。Preferences の Notify the desktop when a
long job ends でオンになります。既定ではオフです。

### Diagnostics

Diagnostics タブは、欠落したマスター、孤立したアーカイブ、MOD 一覧のずれ、壊れた
プラグインの組、そして実行後に script extender 自身のログが何を読み込んだと言って
いるかを一覧します。次のことも報告します。

- script extender が見つからない、またはゲームのバージョンと合っていない
- 前回の正常な起動以降にゲームが変わった
- Eidos を起動しない、または後ろに `%command%` がない Steam の起動オプション
- プレフィックスを最後にセットアップした Proton より古い Proton、または Flatpak
  Steam の Proton
- DLSS 用の `PROTON_ENABLE_NVAPI` の欠落と、Community Shaders のフレーム生成を
  止める排他的フルスクリーン
- `LinuxVersion=true` のない `enblocal.ini`
- 直近のセッションのクラッシュログと、そこに名前が出る MOD
- 欠落または無効な Nexus の要件
- 別のプログラム、または存在しないファイルを指す `nxm://` ハンドラ
- ゲームに `plugins.txt` を無視させる `sTestFile` の行
- Starfield で有効な `.esp` プラグイン、そして不正または対になっていない blueprint
  プラグイン
- パッチ対象の MOD に、ルーズファイルが負けているパッチ
- script extender の Papyrus スクリプトのコピーが、extender 自身のものに勝っている MOD
- 低い `vm.max_map_count` や open files の上限
- Windows のファイルシステム上にあるゲームファイル
- ゲームの Data フォルダではない BodySlide のデータパスと、Data に置くべき
  `Overwrite/Root` 内のフォルダ
- ツールを別のゲームフォルダへ向けているリストの `ModOrganizer.ini`
- 存在しない、またはゲームの実行ファイルがないゲームのコピー

`eidos check <instance> [--json]` はウィンドウを要しないチェックを実行し、問題が
あれば 1 で終了します。Copy report は、ホームフォルダを `~`、ユーザー名を `<user>`
と書き換えたレポートをコピーします。Data タブの Save tree は、各ファイルの出所を
添えた統合 Data ツリーを、同じ伏せ字処理をして書き出します。

置き換え時の保護、BSA/BA2 内のファイル競合、SKSE の静的診断、生成履歴とその制限に
ついては、[整合性チェックと生成ファイル](../../../docs/internals/integrity.md)
(英語)を参照してください。

### 自分のファイルを置く場所

Eidos は、Colony ファミリのどのプログラムも使う配置を使います。あなたが選んだもの
(設定、Nexus のセッション、インスタンスの一覧、自分で書いたゲームとアドオンの定義)は
`~/.config/Colony/Eidos/` に、グローバルインスタンス、ツールのランタイム(ダウンロード
したもの、または `install.sh` がバンドルからコピーしたもの)、セッションログは
`~/.local/share/Colony/Eidos/` に、LOOT のマスターリストは `~/.cache/Colony/Eidos/`
に置かれます。インスタンスの中では、`Stock Game` にゲームの凍結コピーが、`test-run/`
にレビュー待ちの Test run が入ります。

古い Eidos は、ファイルを `~/.config/eidos/`、`~/.local/share/eidos/`、
`~/.local/state/` に置いていました。更新後の最初の起動がそれらを移し、ログにその旨を
書き、古いディレクトリはそのまま残します。移行が完了するまでは、Eidos は古い
ファイルを使い続けます。詳細は
[internals/paths.md](../../../docs/internals/paths.md) に。

### 対応ゲーム

Skyrim SE/AE は実際のプレイで実証済みです。Fallout 4 も端から端まで組み込み済み
です(F4SE の自動入れ替え、アーカイブ無効化、アスタリスク形式のロード順、LOOT、
`.fos` のセーブ)。[guide/fallout4.md](docs/guide/fallout4.md) を参照してください。
組み込み済みでテスターを探しているのは、Skyrim LE、Skyrim VR、Enderal SE、
Fallout 4 VR、Fallout 3、Fallout NV、Starfield、Oblivion、Morrowind です。Fallout 3、
Fallout NV、Oblivion、Morrowind はファイルの日付でプラグインを並べます。Eidos は
プロファイルごとに順序を一つ保ち、ディスク上のファイルを変えずに、マウント内で
それらの日付を見せます。

ほかの 4 つのゲームも組み込み済みで、やはりテスターを探しています。Stellar Blade
(`~mods` の Unreal `.pak` MOD)、Stardew Valley(SMAPI の MOD。SMAPI は別途インス
トール)、7 Days to Die(ゲームの隣の `Mods/` フォルダ)、Crimson Desert です。
Crimson Desert はコード MOD だけを扱います。`bin64` にある ASI プラグイン、ReShade、
ローダー DLL です。PAZ グループにあるデータ MOD は未対応です。

ゲームは TOML ファイル一つで、自分のものを再ビルドなしで
`~/.config/Colony/Eidos/games/` に置けます:
[internals/adding-games.md](../../../docs/internals/adding-games.md)。

## インストール

### Colony 経由(推奨)

[Colony](https://github.com/Project-Colony/Colony) で **Eidos** を検索して
インストールします。更新はランチャー経由で届きます。Colony が入れるのは
`eidos-linux` というファイル一つです。ウィンドウであり、`eidos` という名前の
リンク経由で呼べば `eidos` コマンドラインにもなります。このファイルには、NIF モデル
プレビューのヘルパーも、下のバンドルにある Microsoft のランタイムも入っていません。
ヘルパーがなくてもプレビューがその旨を表示するだけでほかはすべて動き、Eidos は、
ツールの前提条件をインストールするときに Microsoft からランタイムをダウンロード
します。インストール後にすることと、Steam の向け方は
[docs/guide/install.md](docs/guide/install.md#colony-経由) に。

### バイナリを直接ダウンロード

[最新リリース](https://github.com/Project-Colony/Eidos/releases/latest)からアセットを取得します:

| アセット | 中身 |
|---|---|
| `eidos-linux` | Colony がインストールするのと同じ、プログラム全体が入った 1 ファイル |
| `eidos-linux-bundle.tar.gz` | `eidos`、`eidos-gui`、NIF プレビューのヘルパー、Visual C++ 2015-2022 と .NET 8 Desktop Runtime のインストーラー、.NET 10 ランタイム、アイコン、ライセンス表示、そしてデスクトップエントリと Nexus ダウンロードハンドラを登録するインストーラー |

ダウンロードは、[コード署名ポリシー](#コード署名ポリシー)に示すとおり先に検証して
から:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

インストーラーは sudo なしでバイナリを `~/.local/bin` に置き、バンドルの `runtimes/`
を `~/.local/share/Colony/Eidos/runtimes` にコピーします。`--system` と `--bindir DIR`
はバイナリを別の場所に置き、`--from DIR` は別のフォルダから取ります。`--cap` は
sudo に `CAP_SYS_ADMIN` を頼みます。これは実験的な FUSE パススルーだけが必要と
するもので、パススルーはオフのままです。`--no-cap` が既定です。どちらのアセット
も、x86_64 Linux、glibc 2.39 以降、FUSE(`fusermount3`)、そして MOD アーカイブ用の
7-Zip が必要です。

Eidos は自分自身を更新しません。起動時に 1 日 1 回、ウィンドウが GitHub に最新リリース
を問い合わせ、より新しいものがあれば、メニューバーの Help の後ろにリリースノート
へのリンクを出します。Preferences でこれを切れます。パッケージマネージャーが更新
する `/usr/local` 以外の `/usr` 以下のコピーは、問い合わせません。

### ソースからビルドする

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Rust 1.92 以降(CI が確認する `rust-version`。CI とリリースは 1.99.1 でビルドします)、
それに `just`、CMake 3.20 以降、NIF プレビューのヘルパー用の C++17 コンパイラが
必要です。`just build` はリリースプロファイル(codegen unit 1 と fat LTO)を使います。
ビルドジョブ 3 での `eidos` と `eidos-gui` のクリーンビルドは、この設定で 334 秒、
thin LTO で 243 秒かかり、fat LTO は `eidos` を 1.2 MB、`eidos-gui` を 3.8 MB 小さく
します(計測値は `Cargo.toml` に)。`just profile=dev build` はもっと速くコンパイル
します。Arch のパッケージは `packaging/PKGBUILD` からビルドされます。前提条件と、
`just` を使わないコマンドは [docs/guide/install.md](docs/guide/install.md) に。

### Steam の起動オプション

Eidos はゲームの起動コマンドとして動きます。だからこそゲームが始まる前にマウント
できます。たいていの構成に必要なのは基本の行だけで、パスはインストール先の絶対
パスです(Steam は `PATH` を読みません):

```
~/.local/bin/eidos-gui %command%
```

Colony からの場合、そのパスは `~/.local/share/Colony/apps/Eidos/eidos-linux` です。
あとは Play を押します。Eidos はそのゲームのインスタンスで開き、Run は統合ビュー
越しにゲームを起動します。

自分で開いたウィンドウでは、Run は Steam にゲームの起動を頼み
(`steam -applaunch <appid>`)、起動オプションは今までどおり適用され、Steam が開いた
ウィンドウで Run を押します。Heroic には `heroic://launch` のリンクが渡され、
Legendary は、Eidos がそこでゲームのラッパーに設定されていれば
`legendary launch <app>` を実行します。ストア側の設定がないと、Run は何も起動せず、
何を設定するかを告げます: [guide/usage.md](docs/guide/usage.md#ゲームの起動)。
Start the game at once when a store opens Eidos(Preferences。既定ではオフ)をオンに
すると、Eidos は Diagnostics のチェックを実行し、対処が必要なものがなければ、
ウィンドウを出さずにゲームを起動します。

それ以外はすべて、その前に積む環境変数で、自由に組み合わせられます:

| やりたいこと | 前に付けるもの |
|---|---|
| Community Shaders で DLSS | `PROTON_ENABLE_NVAPI=1`。これがないと DLSS は黙って初期化されません。全体のチェックリストは [guide/graphics.md](docs/guide/graphics.md) |
| 画面に FPS カウンタ | `DXVK_HUD=fps` |
| ドライバ側のフレーム補間、MOD なし(RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`。Community Shaders 自身のフレーム生成とは併用しないこと |
| バグ報告用の詳細ログ | `EIDOS_LOG=debug`(セッションログは `~/.local/share/Colony/Eidos/logs/` に入ります) |
| マウントのセッションごとの I/O レポート | `EIDOS_FUSE_STATS=1` |
| FUSE のワーカー数を変える | `EIDOS_FUSE_THREADS=8`(既定は 4。並行処理のバグを追うときは `1` が最初に試すもの) |
| この起動を 1 つのポータブルインスタンスに固定 | `EIDOS_INSTANCE=/path/to/folder`。これがなければ、Eidos は最後に使ったインスタンスで開きます |
| Start the game at once がオンのとき、1 回の起動だけウィンドウを出す | `EIDOS_WINDOW=1` |

最新の MOD 構成(Community Shaders、DLSS、フレーム生成)で残しておく行は、例では
なく最終的なコマンドです:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

構成が動くかを確認する間は `DXVK_HUD=fps` を前に足し、動いたら外してください。

同じ変数は、Steam の行の代わりにプロファイルの起動変数に置くこともできます。そうすると
プロファイルごとに保たれ、ツールの実行にも適用されます:
[guide/usage.md](docs/guide/usage.md#プロファイルごとの変数とラッパー)。

より深い診断用スイッチ(`EIDOS_FUSE_TRACE`、キャッシュと索引の切り分けトグル、
`EIDOS_FUSE_PASSTHROUGH` が既定でオフの理由)は
[guide/troubleshooting.md](docs/guide/troubleshooting.md) に。

## ドキュメント

すべては [docs/](../../../docs/) にあり、読む人ごとに
[docs/README.md](docs/README.md) で索引されています。ユーザーガイド(インストール、
CLI と GUI、xEdit や DynDOLOD などのツール、グラフィックス、トラブルシューティング)、
Eidos の内部の仕組み、そしてなぜ存在するのか、です。セキュリティ方針と脆弱性の
報告方法: [SECURITY.md](../../../SECURITY.md)。

## 言語

プレイヤーが必要とするページは翻訳されています。英語が基準で、翻訳が食い違うときは
英語のファイルが正しいものです。

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

それ以外は英語です。`docs/internals/` と `docs/project/` を読む人は Rust も読んで
いますし、`CHANGELOG.md` は生成物で、`SECURITY.md` も翻訳しません。前の 2 つを
翻訳すれば、それを必要としない読者のために同期を保ち続ける語が 19,000 語以上
増えます。

各翻訳は、元にした英語ファイルのハッシュを持っています。英語が先に進むと CI が
落ちます - [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh) を参照。そのときは
翻訳を更新し、`scripts/i18n-check.sh --fix` で刻印し直します。古くなった翻訳は
削除せず、更新します。

言語はディレクトリ一つです。`docs/i18n/<lang>/` はリポジトリのルートを写して
いるので、`docs/i18n/de/docs/guide/install.md` はドイツ語版の
`docs/guide/install.md` です。だからこそ翻訳ページ同士のリンクは、その英語原文
同士のリンクとまったく同じ文字列になり、言語の廃止は `rm -r` 一回で済みます。

## コード署名ポリシー

リリースのアセットはすべて、開発者のマシンではなく CI の中で、Project Colony
という組織によって署名されます。リリースページでは各アセットの隣に次のものが
あります:

| ファイル | 中身 |
|---|---|
| `<asset>.sig` | 組織のリリース鍵で作った、アセットに対する ed25519 署名 |
| `<asset>.meta` | アセットを、そのファイル名、sha256、リリースのバージョンに結びつける三行 |
| `<asset>.meta.sig` | `.meta` に対する ed25519 署名 |

秘密鍵は組織のシークレットで、共有の
[sign-and-publish ワークフロー](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
が、何もビルドしないジョブの中でだけ使います。Eidos をコンパイルするジョブが
それを目にすることはありません。Colony は Eidos をインストールまたは更新する前に
三つのファイルすべてを検証し、インストール済みのものより古いリリースは拒否します。
ダウンロードを自分で検証するには、OpenSSL 3 で次のようにします(どのアセットにも
同じコマンドが使えます):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

リリースのビルド、署名、公開の方法:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)。

## プライバシー

Eidos はテレメトリも、利用状況の分析も、クラッシュレポートも送りません。この表に
あるサーバ以外には接続せず、それも表に書かれた操作のときだけです。Offline モード
(Preferences, Nexus)は、Nexus へのあらゆる接続と 1 日 1 回のリリース確認を止めます。

| データ | 保存か送信か | 場所と理由 |
|---|---|---|
| 環境設定、インスタンスの一覧、ゲームとアドオンの定義 | 保存 | `~/.config/Colony/Eidos/` |
| インスタンス、ツールのランタイム、セッションログ | 保存 | `~/.local/share/Colony/Eidos/`、またはポータブルインスタンス用に選んだフォルダ |
| LOOT のマスターリスト | 保存 | `~/.cache/Colony/Eidos/loot/` |
| リリース確認 | 送信 | 起動時に、1 日 1 回まで、`api.github.com/repos/Project-Colony/Eidos/releases/latest` への 1 回のリクエスト。Eidos が読むのはリリースタグだけで、何もダウンロードしません。既定でオンで、Preferences で切れます。Offline モードでは行われず、`/usr/local` 以外の `/usr` 以下にインストールされたコピー(パッケージマネージャーが更新するもの)でも行われません。[GitHub のプライバシーステートメント](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Nexus へのサインイン | 送信 | サインインするとブラウザで `users.nexusmods.com` が開き、その結果は、Eidos がそのサインインのためだけに `127.0.0.1` で開くリスナーへ返されます。Eidos はそれを `users.nexusmods.com/oauth/token` でトークンと交換し、Nexus がトークンの署名に使う公開鍵を取得します。[Nexus Mods のプライバシーポリシー](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus の認証情報 | 保存 | アクセストークンとリフレッシュトークンを、モード 0600 の `~/.config/Colony/Eidos/nexus.ini` に。以前の Eidos がそこに書いた `api_key=` の行は残されますが、読まれることはありません。消してもかまいません。 |
| Nexus へのリクエスト | 送信 | Nexus の機能を使うときに `api.nexusmods.com` へ: アカウント名とプレミアムの状態、MOD とファイルの詳細、更新確認、コレクション、ダウンロードリンク。Endorse と Track した MOD は更新確認のたびに読まれ、Endorse や Track を押したときに書き込まれます。MOD の要件、説明、変更履歴、コレクションの検索は `api.nexusmods.com/v2/graphql` へ行きます。どのリクエストもアクセストークンを伴い、Eidos とそのバージョンを名乗ります。 |
| Nexus からのダウンロード | 送信 | 要求したファイルを、ダウンロードリンクが示す Nexus の CDN ホストから取得し、インスタンスのダウンロードフォルダに、それぞれの隣に MO2 形式の `.meta` を添えて保存します。 |
| `nxm://` リンク | 保存 | `eidos nxm --register`(`install.sh` が代わりに実行します)が `~/.local/share/applications/eidos-nxm.desktop` を書き、`xdg-mime` でそれを `nxm://` のハンドラにします。これでサイトの Mod Manager Download ボタンがリンクを Eidos に渡します。Arch のパッケージは同じエントリをシステム全体にインストールします。 |
| LOOT のマスターリスト | 送信 | `raw.githubusercontent.com/loot/...` から、そのゲームで初めて LOOT の並べ替えをするときと、更新を求めたときに。あなたのものは何も送られません。[GitHub のプライバシーステートメント](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| ツールのランタイム | 送信 | ツールの前提条件をインストールするときだけ、それもリリースバンドルが同梱していなかったものだけ(単体の `eidos-linux` ファイルとソースからのビルドは何も同梱しません): `builds.dotnet.microsoft.com` からの .NET 10 ランタイムと、`download.visualstudio.microsoft.com` からの Visual C++ と .NET 8 のインストーラー(どれも Eidos に組み込まれた SHA-256 と一致しなければ拒否されます)、常に `builds.dotnet.microsoft.com` からの .NET 10 SDK(同じ方法で検証されます)、そしてほかの前提条件(.NET Framework 4.8、XACT、.NET 6 と 7)のための `winetricks`。これはインストールするものを、Microsoft や winetricks が使うほかのホストからダウンロードします。[Microsoft のプライバシーに関する声明](https://www.microsoft.com/en-us/privacy/privacystatement) |
| セッションログ | 保存 | `~/.local/share/Colony/Eidos/logs/`、モード 0600。インスタンスごとに 10 セッション、30 日で掃除されます。ホームフォルダは `~`、ユーザー名は `<user>` と書かれるので、ログはそのまま issue に貼れます。あなたが送らない限り、マシンの外へ出ることはありません。 |
| 実行記録 | 保存 | `<instance>/logs/sessions/`: ゲームやツールの実行ごとに、プロファイル、有効な MOD とそのバージョンおよび Nexus のファイル ID、プラグインの順序、各プロファイル INI のチェックサム、ゲームと Eidos のバージョン、終了コード、その実行が作ったセーブ。新しい 50 件が保たれ、`eidos sessions` が読み、`eidos pack` は含めません。あなたのマシンの外には出ません。 |
| 更新レポート | 保存 | `<instance>/logs/updates/<mod>.json`: 直近の更新や Replace がその MOD のフォルダで何を変えたか。`eidos pack` は含めません。 |
| ショートカット | 保存 | `install.sh` が `eidos.desktop` とそのアイコンを `~/.local/share` の下に書きます。自分で作ったツールやプロファイルのショートカットは `~/.local/share/applications` に入ります。 |
| デスクトップ通知 | どちらでもない | 既定ではオフ。オンにすると、D-Bus(`org.freedesktop.Notifications`)経由でデスクトップの通知サービスへ渡され、終わったジョブの名前が書かれます。 |
| 開いたリンク | どちらでもない | MOD のページなどのリンクは、`xdg-open` を通じてあなた自身のブラウザで開きます。 |
| アドオン | どちらでもない | インストールした拡張は、あなたのマシン上で独立したプログラムとして動きます。それが何をするかは、その拡張次第です。 |

## ライセンス

Eidos のオリジナルコードは GPL-3.0-or-later です。[LICENSE](../../../LICENSE) を
参照してください。次の部分は「or later」の選択肢のない GPL-3.0 です:

- `eidos-install` 内の OMOD スクリプト移植コード。
  [元の作者へのクレジット](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md)を参照;
- 同梱の nifly ライブラリの上に作られた `eidos-nif-preview` ヘルパー。
  [各コンポーネントのライセンスとクレジット](../../../crates/eidos-nif-preview/UPSTREAM.md)を参照;
- ロード順のコードがリンクする `esplugin` と `libloadorder` の各クレート。
  これらは `GPL-3.0` を宣言しています。

バイナリに含まれるその他の Rust クレートは、それぞれのライセンス(MIT、
Apache-2.0、BSD など)を保ちます。リリースバンドルは、すべてのクレートとその
ライセンスの索引とともに、それらの告知を `licenses/crates/` に収めています。

バイナリにはさらに、Microsoft DirectX の正規の再頒布可能 DLL が八つ
(`d3dcompiler_47`、`d3dcompiler_43`、`d3dx9_43`、`d3dx11_43` の 32 ビット版と
64 ビット版)埋め込まれています。gzip 圧縮で保存されており(17.9 MB のところ
7.9 MB)、MOD やツールが必要とするときに Eidos が展開してゲームのプレフィックスへ
配置します。これらは Microsoft の再頒布条件のもとにあります。
[入手元の記録](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md)を参照してください。

リリースバンドルには、`runtimes/` の下に、Microsoft が公開しているままの Visual C++
2015-2022 のインストーラー、.NET 8 Desktop Runtime のインストーラー、.NET 10 ランタイム
の zip も入っています。その条件は `licenses/microsoft-runtimes/`(`vc_redist.txt`、
`dotnet.txt`)にあります。
