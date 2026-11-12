<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=c365b34dbc69a832b1c9305498c94eb223b567a1 -->

# トラブルシューティングと診断

環境スイッチ、操作カウンタの読み方、Diagnostics タブの Health カード、クラッシュの
原因になっている MOD の見つけ方、バグ報告に添付するもの、そして既知の問題とその
経緯です。

## VFS を診断する

ゲームがファイルシステムと食い違うものを見ているときの大半は、環境変数二つで
足ります。

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

既知の問題にある `FOPEN_KEEP_CACHE` のクラッシュを見つけたのは、細かく指定できる
ほうの形です。四つ全部を切れば「キャッシュのせいか?」に答えが出て、個別に名指しして
初めて「どれか」に答えが出ます。カウンタは残りの半分に答えます。パススルーを有効に
した場合(既定では無効)、`read 0` が出るロードは `FUSE_PASSTHROUGH` が全バイトを
カーネル内で処理したもので、read パスで調整しようとしていたことはすでに無料で
済んでいます。

ほかにも、不具合をさらに絞り込むスイッチがあります。

```sh
EIDOS_FUSE_TRACE=open,opendir       # log each open / opendir (1 = every channel)
EIDOS_FUSE_THREADS=1                # one daemon thread (default: CPU count, at most 4)
EIDOS_FUSE_OPENDIR=1                # keep opendir handles even when the kernel can skip them
EIDOS_NO_INDEX=1                    # walk the layers instead of using the layer index
EIDOS_NO_OW_INDEX=1                 # the same for the Overwrite's membership index
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE back on, see Known issues
```

Steam からの起動では、起動オプションのコマンドの前に置きます。たとえば
`EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%` です。出力はその実行のログに
出ます。ウィンドウからの起動ならインスタンスの `logs/run-*.log`、Steam からの起動
なら Steam のコンソールです。

統合ビューそのものを見るには、`eidos shell <instance>` がゲームから見えるとおりの
ゲームフォルダでシェルを開き、`eidos shell <instance> -- ls Data/meshes` はそこで
コマンドを 1 つ実行します。書き込んだものは Overwrite に入ります
([usage.md](usage.md#統合ビューでのシェル))。Data タブの Save tree ボタンは、
統合した `Data` ツリー全体を、1 行に 1 ファイルとその出所の MOD を添えたテキスト
ファイルに書き出し、ホームフォルダは伏せられます。

## ユニオンを手でマウントする

`eidos-fuse` は Arch パッケージとソースビルドに付いてきます。`just build` がそれを
ビルドし、あれば `just install` や `install.sh` がコピーします。リリースバンドルと
`eidos-linux` には入っていません。使い方の行は次のとおりです。

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

衝突したときは最初の `--layer` が勝ち、最後があなたの手つかずのゲームデータ
です。マウントに要るのは `/dev/fuse` と `fusermount3` だけです(overlayfs も
Wine も不要):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

書き込みは `--overwrite <dir>`(省略時は一時ディレクトリ)へ落ちるので、ここ
でもレイヤ自体は手つかずのままです。

## なぜパススルーは既定で無効なのか

パススルーは実体のバッキングファイルをカーネルへ渡すので、読み取りはこのデーモン
を完全に飛ばします。下の A/B は以前のデーモンで、後ろの既知の問題「FUSE パススルー
があるとゲームは MOD の中身を一切読み込まない」で説明する変更より前に測定したも
のです。その変更は最も可能性の高い原因を避けていますが、まだゲームで再測定されて
いないので、パススルーは再測定されるまで既定で無効のままです。Skyrim SE 1.6.1170、
proton-cachyos 11.0、カーネル 7.1.4、同じ 82 プラグインのロード順で、バイナリが
そのケーパビリティを持つかどうかだけを変数にして A/B を実測しました:

| パススルー | `STATUS_ACCESS_VIOLATION` を伴う `NtCreateFile` の失敗 |
|-------------|--------------------------------------------------------|
| 有効        | 152 - `.bsa` 75、`.esl` 65、`.esm` 10、`.esp` 2        |
| 無効        | 0                                                      |

有効にするとゲームは自分の書庫もプラグインも一つも開けず、ゲーム内では MOD が存
在しないという形で表れました - エラーもログ行もありませんでした。無効なら同じロー
ド順が、プラグインも書庫も Papyrus スクリプトも生きたままゲームプレイに届きまし
た。

この失敗はデーモンの内側からは見えませんでした。デーモン自身の `open` は毎回成功
し、カーネルがバッキングファイルを拒むこともありませんでした(失敗するセッション
一回分を `EIDOS_FUSE_TRACE=open` で通して確認: `open FAILED` ゼロ、
`passthrough refused` ゼロ)。エラーはデーモンが `opened_passthrough` と返した
あとに生まれていたので、デーモン側のどんなログにも映りませんでした。拡張子に固有
でもなく、書庫もプラグインも等しく襲いました。つまりゲームが実行中ずっと開いた
ままにするファイルです。

`EIDOS_FUSE_PASSTHROUGH=1` で戻せます。何が得られるかを測るため、あるいは
仕組みを再検証するためです。ランチャーと Diagnostics タブに出る capability の
警告は、こちらから求めたときにしか現れません。

## レイヤのインデックスは本当に使われているのか?

`LayerStack::new` は読み取り専用レイヤの完全なマップを作るか、まったく作らない
かのどちらかです。マップがなければ、すべての問い合わせは以前と同じようにレイヤを
歩きます。答えは同じで、セーブのロードに約 3 倍の時間がかかり、それ以外に違いは
見えません。

インデックスが無効なときは、マウントが起動時に一度だけ理由を出力します。

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

理由には、読めないディレクトリ、64 階層より深いレイヤ、4,000,000 を超える
エントリ、または `EIDOS_NO_INDEX` のいずれかが入ります。この行は実行のログに出ます。
ウィンドウからの起動ならインスタンスの `logs/run-*.log`、Steam からの起動なら
Steam のコンソールです。`EIDOS_FUSE_STATS=1` を付けると、アンマウント時の出力にも
`layer index:` の行が入り、通常のマウントで `unindexed` が 0 でなければインデックス
は無効でした。

以下の例は同じことをオフラインで、インスタンスのフォルダに対し、マウントなしで
確かめます。

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` はインデックスあり/なしで実パスを解決し、ディレクトリ走査を
比べます。`index_agrees` は実インスタンスのすべてのパスとすべての一覧に
ついて、両者が同じものを返すか確かめます。`listing_cost` は統合済みの children
マップが `readdir` で何を節約するかを測ります。

`EIDOS_NO_INDEX=1` は歩くほうを強制します。二つの答えの差そのものをデバッグ
しているとき用です。

## CLI でゲームを起動する

ウィンドウではなく `eidos` CLI 経由でゲームを起動するには、Steam の起動オプションを
`eidos` の絶対パスにします。たとえば:

```
~/.local/bin/eidos play skyrimse -- %command%
```

`WINEDLLOVERRIDES` を前に付ける必要はありません。有効な MOD が `d3dcompiler_47`
をインポートしていれば、Eidos は同梱のコピーをプレフィックスへ配置してネイティブ
を強制し、それを MOD が同梱する DLL オーバーライド(ENB/ReShade/`.asi` ローダ)と
統合します。

Eidos が自分で Proton を使ってゲームを起動することはありません。ゲームを起動する
エントリ(スクリプトエクステンダのローダー、ランチャー、ゲームのバイナリ)に対する
`eidos tool <instance> run <title>` は、どのストアも開いていないウィンドウでの Run
と同じように、そのコピーを持つストアに起動を頼み、ストア側の設定が Eidos を起動
しない場合は、何を設定すべきかを表示します。`--print` は、実行する代わりにストアの
コマンドを表示します。

## 閉じないゲームやツール

ゲームやツールの実行中は、Run ボタンが Stop に変わり、ロックカードにも Stop ボタンが
出ます。2 回押してください。2 回目で、その実行が起動したすべてのプログラムを終了
します。まず SIGTERM を送り、10 秒後に残っているものへ SIGKILL を送ります。ゲームが
保存していなかった内容は失われます。そのあと Eidos は通常の終了後と同じ処理をします。
INI とプラグインの変更はプロファイルに戻り、ツールの出力は記録され、MOD は
アンマウントされます。実行はプログラムが何を返したかにかかわらず終了コード 143 で
記録されるので、`eidos sessions <instance> diff` がそれを最後の正常な実行と見なす
ことはありません。Unlock は別物で、ウィンドウを操作できるように戻し、プログラムは
動かしたままにします。ターミナルから `eidos` プロセスに `kill <pid>` を送っても
(pid はロックカードに表示されます)、Stop と同じ結果になります。

## 何が壊れたかを探す

どの変更がゲームを壊したかを探すには:

- `eidos sessions <instance> diff` は、終了コード 0 で終わった直近のゲーム実行以降に
  何が変わったかを一覧します。MOD、バージョン、プラグインの順序、INI、ゲームの
  実行ファイルです([usage.md](usage.md#前回の正常な実行から何が変わったか))。
- Run の隣の Test run は、プロファイルの MOD ありで、MOD なしで、または選択した MOD のうち有効なものと
  それに必要なマスターだけでプレイし、ゲームが書くものすべてを
  `<instance>/test-run/` に送ります。実行後、ファイルの各グループ(セーブ、INI の
  変更、プラグインの状態、Overwrite の出力、ログ)を残すか捨てるかを選びます
  ([usage.md](usage.md#テスト実行))。
- Diagnostics タブの Find the mod behind a problem は、残った容疑者の半分ずつで Test run
  を繰り返し、問題が起きたかを尋ねます。容疑者が 200 個なら実行は約 8 回で、Eidos を
  閉じても探索は続きから再開できます
  ([usage.md](usage.md#問題の原因になっている-mod-を見つける))。

停止された実行は終了コード 143 で記録されるので(上を参照)、この探索で Yes が
自動入力されることはありません。

## Health カード

Diagnostics タブには、見つかった問題ごとに Health カードが出て、原因と、一部には
それを直すボタンが付きます。起動オプションのカードは [install.md](install.md) に、
NVAPI、フルスクリーン、ENB のカードは [graphics.md](graphics.md) に、Proton と
プレフィックスのカードは次の節にあります。`eidos check <instance>` はウィンドウを
要しない所見を出力し([usage.md](usage.md#ターミナルからの-health-チェック) に
一覧があります)、その中にはプレフィックス、レジストリ、`S:` ドライブのカードも
含まれます。ウィンドウだけが出すのは、Flatpak、不明なビルド、古いビルドの派生を
含む Proton カード、パッチの順序、エクステンダのスクリプト、どのプラグインも読み
込まないアーカイブ、Nexus の要件、古くなったプラグインのコピー、そして Display
カードです。

### システム

- "vm.max_map_count is <value>" は、1048576 を下回るときに出ます。1048576 は Arch
  Linux、Fedora 39 以降、Ubuntu 24.04 以降の既定値です。Proton 上の一部のゲームは
  それより多くのメモリ領域をマップし、エラーなしでクラッシュします。
  `/etc/sysctl.d` 下のファイル(たとえば `80-gamecompatibility.conf`)に
  `vm.max_map_count = 1048576` を書き、`sudo sysctl --system` を実行してください。
- "The open files limit is <value>" は、ハードリミットが Proton の esync が必要とする
  524288 を下回るときに出ます。それを下回ると、ゲームが起動しなかったりクラッシュ
  したりすることがあります。systemd 240 以降はこれを設定します。古い環境では、
  `/etc/systemd/system.conf` と `/etc/systemd/user.conf` に
  `DefaultLimitNOFILE=524288` を設定し、ログインし直してください。
- "Game files on a Windows file system" は、ゲームフォルダ、プレフィックス、
  インスタンスのルートが `ntfs`、`ntfs3`、`fuseblk`(ntfs-3g、exfat-fuse)、`exfat`、
  `vfat` の上にあるときに、その場所を名指しします。Proton にはシンボリックリンク、
  Unix のパーミッション、大文字小文字を区別する名前が必要です。ゲーム、そのプレ
  フィックス、インスタンスを、ext4、btrfs、xfs などの Linux のファイルシステムへ
  移してください。

### ゲームファイル

タブの先頭にある Game files カードは、インスタンスがストアのインストールを動かして
いるのか、インスタンス内のゲームのコピーを動かしているのかを示します
([usage.md](usage.md#ゲームファイル-ストアのインストールかコピーか))。次の 3 つの場合、
起動は拒否され、理由を示すカードが出ます。

- "The game copy '<folder>' is missing": インスタンスが動かすコピーが消えています。
  元に戻すか、ストア版に戻してください。
- "'<folder>' has no <binary>": インスタンスが動かすフォルダにゲームの実行ファイルが
  なく、ゲームのコピーではありません。
- "<game> cannot run from a copy yet": Morrowind と、`bUseMyGamesDirectory=0` の
  Oblivion は、プラグインのリストをゲームフォルダに持ちます。

"ModOrganizer.ini points tools at another game folder" は、Eidos が書いたものでは
ないリスト自身の `ModOrganizer.ini` が、インスタンスが動かすゲームファイルとは別の
`gamePath` を持つときに出ます。PGPatcher のような Mod Organizer 2 モードのツールは、
ゲームをそこから読みます。そのファイルの `gamePath` を直すか、ファイルを削除して
Eidos に自分のものを書かせてください。

### ロード順

- "sTestFile lines make the game ignore plugins.txt" は、Fallout 4、Fallout 4 VR、
  Starfield で、プロファイルの `Fallout4Custom.ini` または `StarfieldCustom.ini` の
  `[General]` に `sTestFile1` から `sTestFile10` の行があるときに出ます。どれかが設定
  されている間、ゲームはそれらのプラグインを読み込み、Eidos が書くロード順は効き
  ません。Fix はプロファイル側のコピーから行を削除し、元のファイルを
  `<ini>.pre-fix` として残します。
- Starfield: マスターフラグのない有効な `.esp` プラグインにはカードが出ます。
  Starfield は、それらを `.esm` やマスターフラグ付きのプラグインの間に並べられず、
  一部のレコードをセッションの間ずっと読み込んだままにするためです。作者が
  `.esm` で出していれば、どちらも避けられます。
- Starfield: "Invalid blueprint plugins" は、blueprint フラグの一部だけを満たす
  プラグイン、つまり名前が `BlueprintShips-` で始まり拡張子が `.esm` のものを一覧
  します。"Blueprint plugins without their main plugin" は、メインのプラグインが有効
  でないものを一覧します。`BlueprintShips-Example.esm` には `Example.esm`、`.esp`、
  `.esl` が必要です。
- "<patch> loses <n> file(s) to <master>" は、ある MOD のプラグインがほかの MOD の
  マスターを必要とし、その MOD の優先度のほうが高いため、両方が同梱するファイルの
  コピーがそちらに勝つときに出ます。パッチは普通、パッチ対象の MOD の下に置くもの
  で、カードの Move ボタンがそこへ移します。パッチがそれらのファイルに負けてよい
  なら、順序はそのままにしてください。
- "Out-of-date copy of <plugin>" は、MOD が持つクリーニング済みまたは light フラグ
  付きのプラグインのコピーについて、元のプラグインがその後(MOD やゲームの更新で)
  変わったときに出ます。ゲームは古いコピーを読み込みます。それを隠し、新しい
  プラグインをもう一度クリーニングするか、フラグを付けてください。

### MOD 一覧

- "Order rules of the collection <name> that no longer hold" は、インストール済みの
  コレクションの MOD が、その規則が求める順序に並んでいないときに出ます。Restore
  collection order はそのコレクションの MOD だけを元に戻し、Ctrl+Z で取り消せます。
- "<mod> requires <mod>, which is not installed"(または "which is disabled")は、
  Check for updates が読む Nexus の要件から来ます。Open on Nexus は必要な MOD の
  ページへ移動し、Download main file はプレミアムアカウントでそれを取得し、Ignore は
  すでに別の MOD が満たしている要件を隠します。DLC の要件は助言にとどまります。
  Eidos は DLC と同じ名前のプラグインを探します。
- "fontconfig.txt names a font library that is missing"(Skyrim 系のエンジン)は、
  どのレイヤにもアーカイブにもないフォントと、`Interface/fontconfig.txt` が勝って
  いる MOD を名指しします。フォントを入れるか、その行を消すまで、メニューの文字が
  四角で出ることがあります。
- "BA2 archive stores audio compressed"(Fallout 4 と Fallout 4 VR)は、MOD の BA2 が
  圧縮して保持している `.wav` と `.xwm` ファイルを数えます。ゲームはそれを再生する
  ときに落ちることがあります。音声を非圧縮にしてアーカイブを作り直してください。

### スクリプトエクステンダとゲームの更新

- "<loader> is not installed" は、スクリプトエクステンダのプラグインが入っている
  (`SKSE/Plugins`、`F4SE/Plugins` など)のに、ローダーが有効などの MOD にもゲーム
  フォルダにもないときに出ます。ゲームはそれらなしで起動します。
- "The script extender does not match game version <version>" は、ローダーはあるが、
  動いているゲームのバージョン用のランタイム DLL(たとえば `skse64_1_6_1170.dll`)が
  ないときに出ます。ローダーはエラーで止まります。ゲームの更新のあとによく起きます。
  このカードと上のカードには、エクステンダのサイトを開く Open the download page
  ボタンと、ローダーを MOD の `Root/` フォルダに入れる Install the downloaded archive
  ボタンがあります。
- "<game> changed since the last clean launch" は、ゲームの実行ファイル(Steam の
  コピーならそのビルド ID)が、Eidos 経由で最後にエラーなく終了した起動以降に変わった
  ときに出ます。実行ファイルが変わらない Steam の更新では出ません。旧バージョン向けに
  ビルドされたスクリプトエクステンダのプラグインは読み込まれなくなることがあり、
  その下のカードが、それを宣言しているものを名指しします。リストの作成時のゲーム
  バージョンを保つには、Game files カードの Freeze this version で、またはサイズを
  `freeze` だけで確認した後に `eidos stock-game <instance> freeze --yes` で、インスタ
  ンスに凍結してください。以後のストアの更新が変えるのはストアのインストールで、
  コピーではありません。
- "<mod> replaces <n> <extender> script(s)" は、スクリプトエクステンダより優先度の
  高い MOD が、エクステンダ自身のスクリプトのルーズな `.pex` コピーを持つときに出ます。
  古いコピーは、それが宣言する関数を壊し、ゲームはエラーを出しません。エクステンダを
  その MOD の下へ移すか(両方が MOD なら、カードの Move ボタンで移せます)、MOD の
  それらのファイルを隠してください。エクステンダのスクリプトがゲームの `Data` に
  ルーズに置かれている場合は、順序を付けられるよう、エクステンダを MOD として
  インストールしてください。アーカイブは確認されず、ルーズファイルだけが対象です。

### ツールの出力

- "Overwrite/Root holds meshes"(またはテクスチャ)は、データパスがゲームフォルダを
  指していたツールが、ゲームが決して読まない場所にそれらのフォルダを書いたことを
  意味します。ツールのパスを直し、フォルダを削除して、ツールをもう一度実行して
  ください。Open folder ボタンは `Overwrite/Root` を開きます。
- "BodySlide's GameDataPath is not the game's Data folder"(または `OutputDataPath`):
  [ツール](tools.md#ツール自身の設定はやはりツール自身のもの)を参照してください。

### 直近のセッションでのクラッシュ

"The game crashed in the last session" は、クラッシュロガーが直前の起動でログを書い
たときに出ます。カードは、ログにある例外と、MOD や Overwrite が入れた DLL および
プラグインを挙げます。それらは手がかりとして扱ってください。クラッシュログに名前が
出る MOD は、クラッシュの原因ではなく、クラッシュが表面化した場所であることが多い
のです。ログが MOD のファイルを何も挙げていなければ、カードはそう告げます。Open the
log はログを開き、Copy the log (home path hidden) はホームフォルダを伏せてコピーします。

### ウィンドウサイズ

Skyrim SE、Enderal SE、Fallout 4 では、Display カードが、プロファイルがゲームを
起動するときのウィンドウサイズとモードを示し、プロファイルの Prefs INI にもプレ
フィックスにもどちらも設定されていなければ "No window size set for this profile" と
出ます。Apply は、プロファイルのコピーに、そして SSE Display Tweaks がウィンドウを
設定している場合は Overwrite 内のその INI のコピーにも、サイズとモードを書きます
([usage.md](usage.md#diagnostics))。

## どの Proton がゲームを動かすか、そして壊れたプレフィックス

Diagnostics タブには Proton カードがあります。Eidos と Steam がそのゲームに使う
ビルド、Steam 側の設定、そしてプレフィックスを最後に動かしたビルドを表示します。選択
中のものより新しい Proton がプレフィックスを最後に準備していた場合、カードは助言に
なります。新しいプレフィックスに古い Proton を使うと、Wine のファイルがリセットされ
たり、起動できなかったりするためです。Steam でゲームのプロパティ、互換性から、少なく
とも同じくらい新しいビルドを選ぶか、プレフィックスを作り直してください。

そのほかの Proton とプレフィックスのカードです。

- "Proton comes from the Flatpak Steam install": その Proton はランタイムと
  steamclient のライブラリを Flatpak のサンドボックスの中に持つので、ホストから
  実行すると失敗することがあります。Eidos は `flatpak run` 経由では起動しません。
  それではゲームが統合ビューの外のサンドボックスに入ってしまうからです。
  `~/.steam/root/compatibilitytools.d/` に Proton を入れ、ゲームに選択してください。
- "Eidos cannot tell which Proton runs this game": Steam の `config.vdf` にそのゲーム
  の互換ツールの指定がなく、プレフィックスにもインストール済みのビルドの記録が
  ありません。Steam(ゲームのプロパティ、互換性)で一つ選ぶまで、ツールは起動し
  ません。
- "<game> has no Proton prefix yet": ゲームを一度 Steam から起動して、F5 を押して
  ください。Steam がプレフィックスを作らない場合は、`steamapps/compatdata` 下の
  そのゲームのフォルダが書き込み可能かを確認してください。root が所有する空の
  フォルダがあると、メッセージなしで作成が妨げられます。
- "Windows tools cannot find <game>": プレフィックスにあるインストールパスの記録が
  誤っているか存在せず、xEdit、DynDOLOD、Wrye Bash、Creation Kit がそれを読みます。
  次にツールを実行するときに、Eidos が正しいパスを書きます。
- "The prefix's S: drive points outside the Steam library": `S:` が、Eidos がツール
  用に向けるライブラリのルートにも、Steam がゲーム用に向ける
  `<library>/steamapps` にも一致しません。ライブラリが移動したか、プレフィックスが
  別のマシンから来たのでしょう。プレフィックスに保存された `S:` 相対のパスは、次に
  Eidos か Steam 経由で起動して向き直されるまで、どこにも解決しません。
- "<store> installation: started through its store" は、GOG または Epic のコピーの
  カードです。Eidos がそこでゲームのラッパーになっていれば、Run は Heroic や
  Legendary にゲームの起動を頼み、ツールは Heroic がそのコピーに記録している Wine
  か Proton のビルドで動きます。

File メニューは、ゲームのプレフィックスで winecfg、regedit、Wine のファイル
エクスプローラー、アンインストーラーを、ゲームのツールを動かす Proton で実行し
ます([tools.md](tools.md#ゲームのプレフィックスで動く-wine-自身のプログラム))。プレフィックス
にそれらのプログラムが入るまで、その行はグレーのままです。

Proton カードの Rebuild prefix ボタンは、動かなくなったプレフィックス用です。2 回
押してください。Eidos は何も削除しません。`steamapps/compatdata/<appid>` を同じ
フォルダ内で `<appid>.eidos-backup-<date>-<time>` に名前変更し、次に Steam から
ゲームを起動したときに Steam が新しいプレフィックスを作ります。ゲーム、ツール、その他
の Wine プログラムがプレフィックスを使っている間は拒否します。新しいプレフィックスが
できるまで、ウィンドウの Run はゲームを Steam に渡せず、代わりに起動オプションを
表示するので、Steam の Play を押してください。古いプレフィックスにしかないセーブや
設定は、名前を変えたフォルダに残ります。必要なものをコピーして戻し、不要になったら
そのフォルダはご自身で削除してください。

## バグ報告に添付するもの

- Diagnostics タブの Copy report は、Eidos のバージョン、ゲーム、Proton のビルド、
  Steam が Flatpak かどうか、パススルーの状態、セッションの種類、ディストリビュー
  ション、カーネル、`vm.max_map_count`、open files の上限、`/dev/fuse` と
  `fusermount3` があるか、プロファイルの起動変数とラッパープログラムの名前、そして
  すべての Health カードをコピーします。ホームフォルダは `~`、ログイン名は
  `<user>` と表示されます。`eidos check <instance>` は同じヘッダーをターミナルに出力
  し、`--json` なら JSON で出します。
- 実行のログ `<instance>/logs/run-*.log` は、Eidos が実行した正確なコマンドで
  始まります。File, Eidos logs は、Eidos 自身のログのフォルダ
  `~/.local/share/Colony/Eidos/logs` を開きます。
- クラッシュカードの Copy the log (home path hidden) は、クラッシュログをコピー
  します。
- Data タブの Save tree は、統合した `Data` ツリーを、各ファイルの出所を添えて保存
  します。

## 既知の問題

### DLSS やフレーム生成が黙って何もしない

原因は三つあり、どれもエラーメッセージを出しません。起動オプションで NVAPI が
有効になっていない、排他的フルスクリーン、古い Reflex の FPS 上限。チェック
リスト全体は [graphics.md](graphics.md) にあります。

### Flatpak 版の Steam では Run が何も起こさない

どのストアも開いていないウィンドウでは、Run は `steam -applaunch <appid>` を実行して
Steam にゲームの起動を頼みます。Flatpak 版の Steam はホストの `PATH` に `steam` を
置かないので、ステータス行にエラー付きで "Could not start Steam" と出ます。代わりに
Steam の Play を押してください。

### 別のデバイスからのセーブが表示されない

Eidos を通らずにプレフィックスへ届いたセーブ、たとえば Steam Deck からの Steam Cloud
のダウンロードは、プロファイルのセーブがマウントされている間は隠れたままです。何も
それを削除しません。Saves タブは見出しの下でこれらのセーブを数え、Import はプロ
ファイルのセーブを置き換えずに、それらをプロファイルへコピーします
([usage.md](usage.md#セーブはプロファイルのもの))。

### 一つのディレクトリを二通りに綴った MOD が、二つ目の下にあるものをすべて失っていた

修正済み。ext4 は `meshes/` と `Meshes/` を別物として扱います。
統合ビューはそうであってはならず、実際の MOD は両方を同梱します - XP32
Maximum Skeleton はアニメーションと FNIS の behaviour ファイルを大文字のほう
に、`character assets` をもう一方に置いています。

リゾルバはパスの各要素について大小が完全に一致するものを取り、それに賭けて
いました。`meshes/` に入り、そこで残りのパスを見つけられず、レイヤ丸ごとを
捨てていたのです。もう一方の綴りの下にあるファイルはすべてゲームから見えません
でした - エラーもログもなく、どの診断にも出ません。実際の 50 レイヤの
インスタンスでは 74 ファイルでした。

一致した要素はいまや決定ではなく候補です。大小の完全一致はいまも先に試され、
その下で残りが失敗したときにだけ、走査は大小を畳み込んで等しい兄弟を探します。
一覧のほうも一つ上のディレクトリで同じ欠陥を抱えており、いまはレイヤごとに大小
を畳み込んで等しいディレクトリをすべて読みます。

パスインデックスにこのバグはありませんでした。見つけたディレクトリをすべて
歩くからです。そのため一時期、フォールバックが返せないファイルをインデックスが
返していました。フォールバックこそ決して間違えないはずの答えなのに、です。

### DynDOLOD の LODGen が空のログを残して死ぬ

`dotnet10` で修正、[tools.md](tools.md) を参照。すべてのワールドについて
`LODGen_SSE_<world>_log.txt` にバージョンのバナーと `.NET Version:` の行だけが
あって他は何もなく、ダイアログは「failed to generate object LOD for one or more
worlds」とだけ言います。原因は Wine の Mono が .NET Framework の代わりに応答する
ことで、.NET Framework をいくら入れても直りません - Proton はプレフィックスを更新
するたびに `mscoree.dll` を自分のツリーへのシンボリックリンクで置き換えます。
リリースバンドルは .NET 10 ランタイムを同梱しており、ほかのインストールでは一度だけ
ダウンロードされます。

### Wine はマウントが大小を畳み込むと分からなかった

修正済み。

Wine の `get_dir_case_sensitivity` はまず `FS_IOC_GETFLAGS` でディレクトリに尋ね、
次に CIOPFS が自分の配下のディレクトリに残すマーカーを嗅ぎに行きます。どちらも無
ければ Wine は大小を区別すると仮定し、綴りがバイト単位で一致しないルックアップは
すべて、大小無視の一致を探すためにディレクトリ丸ごとの読み取りへ落ちます。
Bethesda のゲームはファイルが `ccBGSSSE001-Fish.bsa` であるのに
`data/ccbgssse001-fish.bsa` を求めるので、ほぼすべてのアセットで発火しました。八
秒で 4471 回のマーカー探査と 2236 回のディレクトリ全読み直し、九十秒で `Data` の
列挙 195796 回。Skyrim SE はメインメニューに一度も届かず、常駐 240 MB のままでデー
モンはコアの 92% を焼いていました。

Eidos は最初から `resolve_read` で大小を畳み込んでいました。コストのすべては、そ
う言わなかったことにありました。`lookup` はいま `.ciopfs` に答えます。`readdir`
はいまもそれを一覧しません。`ioctl` もディレクトリに対する `FS_IOC_GETFLAGS` に
casefold フラグで答えます。現行の Wine はマーカーを探す前にこれを尋ねます。

これを単に遅いではなく致命的にしたものが二つあります。コストはディレクトリの
大きさに比例するので、Anniversary のコンテンツを入れたこと(`Data` が 37
ファイルから 177 へ)が均衡を崩しました。そして `opendir` が統合済みの一覧を
先回りして作っていたこと。Wine がディレクトリを開くのは中のマーカーを `stat`
するためだけ、という場合には純粋な無駄です - スナップショットはいまは最初の
`readdir` で取られます。

その後: メインメニュー、常駐 2.1 GB、デーモンは CPU 0%。

見つけたのは `EIDOS_FUSE_TRACE=opendir` で、これは同梱されています。操作
カウンタは数を教えます。一つのディレクトリの 195796 回の列挙は、合計の中では
見えません。

### ゲームが `plugins.txt` を空に書き直した

上の大小の畳み込みとほぼ同じことでしょう - まともな時間で列挙しきれない `Data` を
前に、そこには何もないと結論して、それを保存した。証明はされておらず、確かめ直す
価値があります。いずれにせよキャプチャのガード(有効なセットを丸ごと消すキャプチャ
は、大きさによらず拒まれる)があるので、これがプロファイルを壊すことはもうありません。

### `FOPEN_KEEP_CACHE` は無効

修正済み。MOD を一つも入れていない状態で、メインメニューの数秒後に Skyrim SE を
null 参照で決定論的に落としました。カーネル側の残り三つのキャッシュは個別に二分
探索で除外され、効いていたのはこれだけでした。その二分探索は `FUSE_PASSTHROUGH` を
有効にしたまま行われており、カーネルは `FOPEN_KEEP_CACHE` を付けたパススルーの
オープンを EIO で失敗させるので、このフラグを有効にするとパススルーのオープンは
すべて失敗していました。ゲームを落としていたのはこの失敗したオープンだったのかも
しれず、パススルーを無効にした状態でこのフラグ単独を試したことはありません。
パススルーの応答はもうこのフラグを付けません。失っても無料だと当時は実測されま
したが、パススルーが有効だとデーモンは読み取りを*一つも*処理しない
(`EIDOS_FUSE_STATS` はロード一回分で `read 0` を報告)ので、パススルーが既定で無効の
いま、本当のコストは未測定です。単独で試すなら `EIDOS_FUSE_KEEP_CACHE=1` で戻せます。

### FUSE パススルーがあるとゲームは MOD の中身を一切読み込まない

デーモンはいま最も可能性の高い原因(下記)を避けていますが、それはまだゲームで再測
定されていないので、パススルーは既定で無効のままです。`EIDOS_FUSE_PASSTHROUGH=1`
で有効になります。この変更の前は、パススルーが有効だと Skyrim SE は自分のファイル
152 個を `STATUS_ACCESS_VIOLATION` で開けず、MOD の中身が黙って一つも読み込まれま
せんでした。計測の内容と、デーモン自身のログがきれいな実行を映していた理由は、上の
「なぜパススルーは既定で無効なのか」にあります。

カーネルは inode ごとにバッキングファイルを一つだけ結び付けます
(`fs/fuse/iomode.c`)。すでにバッキングファイルを持つ inode に二つ目のバッキング
ID を持ち込むオープンは EIO で失敗し、バッキングファイルを持つ inode の通常のオー
プン(`FOPEN_PASSTHROUGH` なし)も同じく失敗します。`FOPEN_KEEP_CACHE` を付けたパ
ススルーのオープンも EIO で失敗します。以前のデーモンはオープンのたびに新しい ID
を登録していたので、ゲームがすでに開いているファイルの二度目のオープンが失敗して
いました。Wine は EIO を `STATUS_ACCESS_VIOLATION` として報告し、これが 152 個の
失敗したオープンの裏にあったエラーです。このルールはカーネルのソースから読んだも
ので、その 152 個の失敗と一致するため、最も可能性の高い原因です。

いまのデーモンは inode ごとにバッキングファイルを一つだけ登録してその inode のパ
ススルーのオープン全部で共有し、パススルーのオープンには `FOPEN_KEEP_CACHE` を決
して付けません。パススルーを使うのは、ゲームと MOD のレイヤにあるファイルのうち
1 MiB 以上か、`.dll`、`.exe`、`.asi` で終わるものの読み取り専用オープンだけです。
そのバッキングファイルが登録されたあとは、そのファイルの読み取り専用オープンはす
べてそれを使い、ファイルに通常のオープンが残っている間は、新しい読み手も通常のオー
プンになります。Overwrite にあるファイルは決してパススルーを通らず、プラグインの
タイムスタンプを投影するマウントはパススルーをまったく使いません。パススルーの読
み手が保持しているファイルを書き込み用に開くと、どの copy-up よりも先に
ETXTBSY(Wine では `STATUS_SHARING_VIOLATION`)が返るので、`O_TRUNC` のオープンが
MOD のファイルを隠す空のコピーを Overwrite に残すことはありません。

### Mod Manager Download ボタンが何も起こさない

Preferences, Nexus, Downloads に、`nxm://` リンクを受け取るプログラムが表示されます。
そのプログラムが Eidos でない場合、存在しない場合、未設定の場合は、Health にカード
が追加されます。Register ボタンは `eidos-nxm.desktop` を書き出し、
`x-scheme-handler/nxm` の既定のハンドラにします。ターミナルからは
`eidos nxm --register` で同じことができます。

状態がすでに Eidos を示しているのにボタンが動かない場合、ブラウザが以前の選択を使っ
ている可能性があります。ブラウザの設定で nxm リンクの保存済みの選択を消去し、再度
確認されるようにしてください。Flatpak 版のブラウザはホストのプログラムを自分では
起動できず、デスクトップポータル経由でリンクを渡します。ポータルはホストに登録された
ハンドラを読みます。
