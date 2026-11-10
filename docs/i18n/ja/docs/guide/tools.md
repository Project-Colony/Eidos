<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# ツール: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Eidos 越しに実行したツールは、ゲームの Wine プレフィックス(Steam の Proton
プレフィックス、または Heroic が用意したプレフィックス)の中で統合ビューを見ます。
ゲームが読むものをそのまま読み(有効な MOD すべてを、優先順位どおりに)、書き出した
ものは Overwrite に着地し、そこでひと押しすれば本物の MOD になります。
`runner=native` のツールは Proton なしで動きます。
[下記](None#環境変数とproton-を使わないツール)を参照してください。

## Eidos が自分で見つけるもの

名前が十分に一意なツールは、宣言しなくても見つけられます。xEdit がその典型で、
Fallout 4 なら `FO4Edit.exe`、Skyrim SE なら `SSEEdit.exe`、初代なら
`TES5Edit.exe`、といった具合です。それぞれに QuickAutoClean のエントリが付き、
LOOT が警告する dirty edit をクリーニングします。`PGPatcher.exe` は、Morrowind
を除くすべての Bethesda ゲームで同じように見つかり、これがないと動作できない引数
とともに登録されます([下記](None#pgpatcher-に---ignore-mo2vfscheck-が要る理由)を参照)。
Eidos は、ファイル名で次の場所を探します:

- `Overwrite/Root/`、インスタンスが動かすゲームフォルダ(ストアのインストール、
  またはそのゲームのコピー)、そして有効な MOD の `Root/` ツリー;
- このインスタンスの `mods/`。MO2 の利用者がツールを入れる場所です;
- 環境設定で指定したツールフォルダ(General -> Tools)。インスタンス間で
  共有するディレクトリ、`/mnt/Games/Tools` のようなもののためです。

よく使われるジェネレーターも同じ方法で見つかります。それぞれ対応するゲームでのみ
対象になります。対象は Pandora、Nemesis、FNIS、BodySlide、DynDOLOD、TexGen、
xLODGen、Synthesis です。BethINI Pie(`Bethini.exe`)も、PGPatcher と同じゲームで
同じように見つかります。[BethINI Pie](None#bethini-pie) を参照してください。Stardew
Valley では、SMAPI(`StardewModdingAPI.exe`)が同じ方法で見つかります。これはその
ゲームのスクリプトエクステンダのローダーでもあるので、
[ツールに手を伸ばす](None#ツールに手を伸ばす-隠す固定するデスクトップショートカット)で説明する
とおり、ストア経由でゲームを起動します。

一覧はゲームごとなので、Skyrim のインスタンスに Fallout のエディタが出ることは
ありません。探索は四階層で止まります。MOD プールは数十万ファイルあり、これは
ツール一覧を組み立てるたびに走るからです。シンボリックリンクも辿りません。
こうして見つかったツールは、手で入力したものとまったく同じように設定されます。
前提条件、引数(PGPatcher の `--ignore-mo2vfscheck`)、取り込み先の MOD は、
タイトルから、以下の規則で決まります。

ツールが別の場所にある場合や、違う引数を渡したい場合は、手で追加してください。
同じタイトルのユーザー項目は、自動で見つかったものを上書きします。

## LOOT が dirty と判定したプラグインをクリーニングする

LOOT でソートすると、Diagnostics タブに LOOT が dirty と判定したプラグインが一覧表示されます。対象は xEdit と LOOT のクリーニングデータの両方があるゲームです(Skyrim LE、SE、VR、Enderal SE、Fallout 3、New Vegas、Fallout 4 と 4 VR、Oblivion、Starfield)。Eidos が xEdit を見つけた場合(Executables に登録されているか、インスタンス内でツールを探す場所のどこかにある場合)、カードには Clean all が、dirty な各プラグインの行メニューには Clean with QuickAutoClean が表示されます。見つからない場合、カードは Executables に xEdit を追加するよう案内します。Eidos は xEdit をダウンロードしません。xEdit の QuickAutoClean モードを、統合ビュー越しに `-qac -autoexit -autoload` を付けて、一度に一つのプラグインに対して実行します。xEdit はクリーニングしたプラグインを読み込んだものの上に保存します。統合ビューではこれは Overwrite 内の新しいファイルになり、Eidos はそれを `Cleaned plugins` という名前の MOD に移します。この MOD は有効な状態でリストの末尾に作られ、すべての競合に勝ちます。元の MOD 内のプラグインにも、ゲームの Data フォルダ内のプラグインにも、書き込むことはありません。クリーニングを元に戻すには、`Cleaned plugins` を無効にするか、その中のファイルを一つ非表示にします。Overwrite にあるプラグインはクリーニングされません。xEdit がそのファイル自体を書き換えてしまうためです。先に Overwrite を MOD にしてください。

xEdit のバックアップ、キャッシュ、ログは統合ビューの外の `<instance>/logs/qac-<time>/` に置かれ、ステータス行はそのログから読んだ件数を表示します。クリーニングが成功すると Eidos はこのフォルダを削除し、失敗した場合は残します。Eidos は各コピーの元になったプラグインのサイズと日付を `Cleaned plugins/meta.ini` に記録します。そのプラグインが変わると、Diagnostics タブに **Out-of-date copy of <plugin>** が表示され、古いコピーを非表示にするボタンが付きます。

## 追加する

GUI なら、メニューバーの Tools か、ツールバーの Executables を開いて Add。コマンドラインなら:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse rm BodySlide          # remove an entry you added
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # exe を登録せずに一度だけ実行する
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` は、ツール自身の設定に関係なく、その実行の出力を `mods/<mod>` に取り込みます。`rm` が削除するのは自分で追加した項目だけで、ゲームごとの既定は隠せますが削除はできません。

`--exe` で実行した exe のファイル名が、そのゲームで Eidos が知っているものと(大文字小文字を無視して)一致すると、そのツールのタイトルが付き、下で述べる前提条件、引数、取り込み先の MOD も付きます。`GenerateFNISforUsers.exe` は FNIS なので、上の `--exe` の行は `dotnet48` を求め、`FNIS Output` に取り込みます。それ以外の exe は、ファイル名がタイトルになります。Data タブの `.exe` の行には Add tool があり、項目が入力された状態で Executables ダイアログを開きます。また Run があり、`--exe` と同じようにそのファイルを一度だけ実行します。どちらも同じ方法でファイルにタイトルを付けます。

Windows のツールではないコマンドには、`eidos shell skyrimse` が `$SHELL`(未設定なら
`/bin/sh`)を、統合ビューをマウントしたゲームフォルダで開きます。シェルが書き込んだ
ものは Overwrite に入り、シェルを抜けるとビューはアンマウントされます。
`eidos shell skyrimse -- <command>` は代わりにコマンドを 1 つ実行します。対話シェルでの
Ctrl+C は入力したコマンドを止め、シェルは開いたままです。シェルは Proton なしで動く
ので、Windows のプログラムは `eidos tool` を通します。

ウィンドウでは、固まったツールの実行を Stop が終わらせます。2 回押すと、Eidos は
その実行のプログラムを終了し、あとは通常の終了後と同じようにファイルを処理します。
Notify the desktop when a long job ends(Preferences, General, Running a game)をオンに
していれば、ウィンドウがバックグラウンドにある間に終わったツールの実行は、デスクトップ
通知を送ります。

スクリプトエクステンダ、ゲーム本体、ランチャーは自動で検出されます。登録が
要るのは追加のツールだけです。ゲームのコピーから動くインスタンスでは、それらは
コピーの中で検出されます。[ゲームのコピー上のツール](None#ゲームのコピー上のツール)を参照して
ください。

### どこにあろうと、実体のファイルを指す

実行ファイルは、実際に置かれている場所で登録します。ツールを MOD として
入れたなら、それは MOD フォルダの中です:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

これはグローバルインスタンスのパスです。ポータブルインスタンスでも同じ規則が
自身のフォルダの下 `<instance>/mods/...` に当てはまります。こうした絶対パスは、
あとでポータブルフォルダを移動したときに唯一生き残らないものです。

Eidos は起動前にそのパスを統合ビューのものへ書き換えます。ツールは
`<game>/Data/CalienteTools/BodySlide/` から動き、そこにある他の MOD の
ファイルもすべて見えます。BodySlide が同梱する `SliderSets` ディレクトリは空で、
作れる体型はすべて CBBE と衣装 MOD から来ます。自分の MOD フォルダから起動すると
何も見つからず、壊れているように見えます。

MO2 も同じ理由で同じ書き換えをしています。そのコメントは FNIS を名指ししています。

無効な MOD の中のツールは書き換えられません。そのファイルもビューにないからです。
Eidos はそう伝えたうえで、元のフォルダから実行します。

## Mod Organizer 2 のリストのツール

Wabbajack のリストや、Eidos のインスタンスとして開いた別の MO2 インスタンスには `ModOrganizer.ini` が残っています。そこに設定された実行ファイルを `tools.ini` にコピーできます。

```sh
eidos tool /path/to/list import-mo2           # 追加される内容を表示
eidos tool /path/to/list import-mo2 --write   # 追加する
```

ウィンドウでは、Executables ダイアログに Import from ModOrganizer.ini があり、項目ごとにチェックを外せます。Add selected はチェックした項目を一覧に入れ、Save を押すまで何も保存されません。数えられるのは、Eidos が書いたものではない `ModOrganizer.ini` だけです。Eidos が書くものについては [PGPatcher と PGLite](None#mod-organizer-2-モードの-pgpatcher-と-pglite) を参照してください。

`Z:` のパスは Linux のパスで、そのまま使われます。別のドライブ文字のパスは、リストのフォルダ名を含んでいればそのフォルダの下に移されます(`D:\Lists\MyList\tools\SSEEdit.exe` は `<list>/tools/SSEEdit.exe` になります)。実行ファイル、作業ディレクトリ、引数のすべてが対象です。スクリプトエクステンダのローダー、ランチャー、ゲーム本体は単なる名前になり、組み込みの項目と同じく、Eidos がビューを重ねるゲームフォルダから実行されます。MO2 の `hide` は Hide from picker に、`steamAppID` は Steam AppID になります。それ以外を指す項目(Windows のシステムプログラム、リスト外のツール)は理由付きでスキップとして表示され、`tools.ini` に既にあるタイトルは置き換えられません。

取り込まれた項目は MO2 のタイトルと引数を保ち、Eidos はタイトルから何も補いません。前提条件も、追加の引数も、取り込み先の MOD もありません。それらは Executables ダイアログで設定してください。Windows で作られたリストの PGPatcher の項目には、たいてい `--ignore-mo2vfscheck` がありません。足してください([下記](None#pgpatcher-に---ignore-mo2vfscheck-が要る理由)を参照)。

リストの `Stock Game` フォルダを指す取り込み済みのパスは、インスタンスがそのコピーから動くときにだけ、実行時にストアのインストールへ移されます。次の節を参照してください。

## ゲームのコピー上のツール

インスタンスがゲームのコピー(リストの `Stock Game` や `Game Root` フォルダ、または
凍結したコピー。[Game files](usage.md#ゲームファイル-ストアのインストールかコピーか)を参照)
から動くとき、ツールにはストアのインストールではなくコピーのファイルが見えます。
統合ビューは今もストアのインストールの上にマウントされています。ルートビューでは
コピーが一番下にあり、Data ビューはコピーの `Data` を読みます。Eidos は、スクリプト
エクステンダ、ランチャー、ゲーム本体、既知のツールを、ストアのインストールではなく
コピーの中で探します。

コピーを指すツールのパス、引数、作業ディレクトリは、実行前にストアのインストールへ
移されます。Linux のパスでも Wine の `Z:` パスでも書けます。
`-D:"Z:\...\Stock Game\Data"` は、ストアのインストールの下の同じパスになります。
リストの `ModOrganizer.ini` から取り込んだ項目には、これが必要です。インスタンスが
ストアのインストールから動くときは何も移されないので、そうした項目は MOD のない
素のコピーを読みます。

プレフィックスのレジストリにあるゲームの `installed path` と、Eidos が書く
`ModOrganizer.ini` の `gamePath` は、統合ビューがあるストアのインストールを指した
ままです。コピーのフォルダがない、またはゲームのバイナリがない場合、ゲームの起動と
同じようにツールの実行も止まり、メッセージはストアのインストールに戻すための
`eidos stock-game <instance> store` を挙げます。

`eidos stock-game <instance>` は、インスタンスがどのゲームファイルを動かしているか、
どのコピーに切り替えられるかを表示します。`use <folder>` と `store` が切り替え、
`freeze --yes` はストアのインストールをインスタンスの `Stock Game` フォルダにコピー
して、そこから動かします。Diagnostics タブの Game files カードも同じことをします。
[Game files](usage.md#ゲームファイル-ストアのインストールかコピーか)を参照してください。

## ツールの出力を専用の MOD へ送る

ジェネレータ(FNIS、Nemesis、BodySlide、DynDOLOD、Synthesis)は何百ものファイルを
書きます。既定では他のすべてと一緒に Overwrite に着地します。Executables のエディタ
で Capture output into を設定すると、その実行の出力は代わりにその MOD へ入ります:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora、Nemesis、FNIS、BodySlide、Synthesis には、取り込み先の MOD があらかじめ設定されます。`Pandora Output`、`Nemesis Output`、`FNIS Output`、`BodySlide Output`、`Synthesis Output` です。DynDOLOD、TexGen、xLODGen は、指定した出力フォルダに書き込むため、設定されません。

MOD が無ければ作られます。動くのはこの実行が生んだファイルだけで、すでに
Overwrite にあったものはそのまま残ります。だから取り込み先を持つ二つのツールが
互いの出力を奪い合うことはありません。何も書かなかった実行は、空の MOD を
残しません。

Eidos はファイルを実行の後で移します。MO2 は逆に、書き込みレイヤーを MOD に向けます。
するとその MOD は実行のあいだ最優先へ繰り上がり、関わる競合をすべてひっくり返し、
終わったらまた戻します。書き込みは、コピーアップなしで MOD 自身のファイルに直接
入ります。取り込みはそのどちらも無しに同じ最終状態へ辿り着きます。

対象の MOD が無効なら、出力は書かれますがゲームからは見えず、ツールは次の実行で
同じファイルを作り直すことになります。Eidos はその場合に警告します。

### 出力が古くなったとき

Eidos は、ツールの出力をどの MOD、プラグインの順序、プロファイルから作ったか、ツールの実行ファイルのサイズと日付、そして `tools.ini` にある項目のパスと引数を記録します。そのどれかが変わると、Diagnostics タブに **Generated output inputs changed** が表示され、カードには現在の設定でツールを実行する **Run <tool> again** ボタンが出ます。

古い出力でゲームが壊れるジェネレーター(Pandora や Nemesis など)には、`tools.ini` のその項目に `check_before_play=1` を足すか、Executables ダイアログでそのツールの **Ask before playing when its output is out of date** を有効にします。すると Run は先に確認します。ツールを実行するか、そのまま遊ぶかです。Play anyway の選択は、MOD、プラグイン、プロファイルがまた変わるか、ウィンドウを閉じるまで覚えられます。Eidos がツールを自分で起動することはありません。**Start the game at once when a store opens Eidos** がオンのときは、そうした確認はゲームを起動せずにウィンドウを開いたままにします。

## 前提条件と引数は、タイトルで決まる

ツールに付けたタイトルが、Eidos が用意するランタイムの前提条件を決めます。
照合はタイトルに対する大文字小文字を無視した部分一致です。

| タイトルに含まれる文字列 | Eidos が要求するもの |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| それ以外 | なし |

つまり `BodySlide` として登録したツールは DirectX の DLL を受け取り、同じ
実行ファイルを `BS` として登録すると何も受け取らず、DLL について何も言わない
エラーで起動に失敗しうるということです。ツールにはプログラムの名前を
付けてください。

一覧は `default_prereqs`(`crates/eidos-instance/src/tools.rs`)にあり、
Executables ダイアログの `Prereqs` 欄は編集できます。タイトルが入れるのは既定値
だけです。

タイトルは引数も同じように決めます。誰にも推測できない引数を必要とする
唯一のツールのためです。`pgpatcher`(または旧名の `parallaxgen`)を含む
タイトルには `--ignore-mo2vfscheck` が与えられます。理由と、それが任意では
ない理由は[下記](None#pgpatcher-に---ignore-mo2vfscheck-が要る理由)にあります。取り込み先の
MOD もタイトルから決まります([上記](None#ツールの出力を専用の-mod-へ送る)を参照)。

### 前提条件は三種類

Tier 1 は同梱の DLL です。`d3dx9_43`、`d3dcompiler_47`、`d3dcompiler_43`、`d3dx11_43`。
Eidos がバイナリの中に持っており、起動時にプレフィックスへコピーします。64 ビットの
プレフィックス(Proton の既定)では、64 ビット版が `system32` に、32 ビット版が
`syswow64` に入ります。やることは無く、ネットワークも要りません。各ビルドは gzip
圧縮で保存されているので、8 つのファイルが各バイナリで占めるのは 17.9 MB ではなく
7.9 MB で、コピーするときに展開されるのは必要な 1 つだけです(最大 4.7 MB)。

Tier 2 はインストーラです。`vcrun2022`、`dotnet6`、`dotnet7`、`dotnet8`、
`dotnetdesktop6`、`dotnetdesktop8`、`dotnet48`、`xact`、`xact_x64`。これらはレジスト
リキー、GAC、CLR ホストを書くので、ファイルのコピーでは済みません。Eidos はそのうち
3 つを winetricks なしでインストールします。`vcrun2022` は Microsoft の
`VC_redist.x86.exe` と `VC_redist.x64.exe` を実行し、`dotnet8` と `dotnetdesktop8` は
.NET 8.0.12 Desktop Runtime のインストーラを実行します。このインストーラには .NET
ランタイムも入っています。リリースバンドル(`eidos-linux-bundle.tar.gz`)はこれらの
インストーラを `runtimes/` フォルダに同梱しており、`install.sh` がそれを実行した
ユーザーのために `~/.local/share/Colony/Eidos/runtimes/` にコピーします(`--system`
でも同じです)。Eidos は各ファイルをまずそこで、次に `eidos` バイナリの隣の
`runtimes/` フォルダ(展開したバンドルをその場で動かす場合)で探します。どちらの
コピーもない場合(単体の `eidos-linux` ファイル、ソースからのビルド、システム
インストールの別のユーザー)、Eidos は Microsoft(`download.visualstudio.microsoft.com`)
からダウンロードして、`~/.local/share/Colony/Eidos/runtimes/` に保持します。どちらの
場合も、SHA-256 が Eidos に組み込まれた値と一致しないファイルは拒否され、ゲームの
ツールを実行する Proton または Wine のビルドで、ゲームのプレフィックスの中で、
ウィンドウなし(`/install /quiet /norestart`)で実行されます。winetricks と同じく、
`vcrun2022` はまずプレフィックスの DLL オーバーライドで Visual C++ の DLL を
`native,builtin` にします。その後、Proton のビルドでは、Eidos はプレフィックスの C++
ランタイムが半分 Microsoft、半分 Proton のものになっていないかを確認します。そう
なっていればインストールは失敗し、メッセージが異なるビルドに由来する DLL の組を挙げ、
代わりに winetricks で `vcrun2022` をインストールするよう告げます。winetricks は
`msvcp140.dll` 自体を置き換えます。インストールされるのは 64 ビットの .NET 8 だけで
す。32 ビットの .NET 8 ツールには、引き続き `winetricks dotnetdesktop8` が必要です。

残りの 6 つの verb は、`PATH` 上になければならないシステムの `winetricks` を通ります。
Eidos は、ゲームのツールを動かす Proton または Wine のビルドの `wine` で、ゲームの
プレフィックスに対して実行します。これで Steam の pressure-vessel コンテナと、
protontricks + Proton-GE の食い違いを回避します。winetricks の verb には
`cabextract` を必要とするものがあり、ない場合は Eidos が警告します。未インストールの
Tier 2 の verb を宣言したツールも起動はします。その verb と、直すためのコマンドを
挙げた警告付きで - 別の経路ですでに入っていることもあるからです。

Tier 2 の verb は、プレフィックス自身の `winetricks.log` にそれが載っている(誰が
winetricks や protontricks を実行したかは問いません)か、Eidos が同じプレフィックスに
インストールしたときに、完了と見なされます。Eidos は自分のインストールを、インス
タンスの `prereqs.done` に、プレフィックスのパスと inode に結び付けて記録するので、
作り直したプレフィックスでは verb が再び未完了として表示されます。**Rebuild prefix**
は Steam のコピー向けの Diagnostics の操作で、`compatdata/<appid>` を
`<appid>.eidos-backup-<time>` に名前変更し、何も削除しません。Steam は次にゲームを
起動するとき新しいプレフィックスを作ります。

Tier 3 はランタイム、`dotnet10` と `dotnetsdk10` です。現代の .NET ランタイムは 193
個のファイルで、専用のディレクトリに置かれ、`DOTNET_ROOT` を通して見つけられます。
登録もされず、プレフィックスにインストールもされないので、他のどちらの Tier でも
運べません。Eidos はこれをどのインスタンスの外にも置きます。78 MB はゲームごとでも
プロファイルごとでもないからです。各ランタイムは
`~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/`(`dotnet10-10.0.10`、
`dotnetsdk10-10.0.403`)に展開されます。Eidos は、ランタイムの zip をダウンロード名
のまま、Tier 2 のインストーラと同じ 2 つのフォルダで探し、リリースバンドルが同梱する
コピー(`dotnet-runtime-10.0.10-win-x64.zip`、`dotnet10` 専用)を展開するか、
ダウンロードします。バンドルは SDK の `dotnet-sdk-10.0.403-win-x64.zip` を同梱しませんが、
そこに手で置いたコピーは使われ、ダウンロードを省けます。Eidos は、どちらのファイル
も SHA-256 が組み込まれた値と一致しなければ拒否します。zip は MOD アーカイブに使うのと
同じ 7-Zip の `7z` で展開します。

`dotnetsdk10` は .NET 10 SDK で、ダウンロードは 301 MB、展開すると 808 MB です。
Synthesis はパッチャーのコンパイルにこれを必要とし、ビルドと実行にはパスなしの `dotnet` を呼び出します。
そのため `dotnetsdk10` を宣言したツールには、`WINEPATH` 上の SDK フォルダに加えて、`NUGET_CERT_REVOCATION_MODE=offline`、`DOTNET_GENERATE_ASPNET_CERTIFICATE=false`、`DOTNET_CLI_TELEMETRY_OPTOUT=1` も設定されます。
パッチャーのビルドは MSBuild のノードと VBCSCompiler サーバーを動かしたまま残し、Wine ではそれらがプレフィックスの wineserver を終了させないため、ツールには `MSBUILDDISABLENODEREUSE=1`、`DOTNET_CLI_USE_MSBUILD_SERVER=0`、`UseSharedCompilation=false` も設定されます。
`DOTNET_ROLL_FORWARD=Major` により、.NET 8 向けにビルドされたパッチャー(Mutagen 0.54 より前)は、.NET 8 ランタイムが見つからないとき SDK の .NET 10 ランタイムで動きます。
`tools.ini` の Synthesis エントリが古い既定値 `dotnet8`, `vcrun2022` のままなら、Eidos が読み込むときに `dotnetsdk10`, `vcrun2022` に置き換わります。

NuGet は復元するすべてのパッケージの署名を Windows のルート証明書ストアで確認しますが、Wine のプレフィックスには、SDK が `sdk/10.0.403/trustedroots/codesignctl.pem` と `timestampctl.pem` として同梱しているコード署名用とタイムスタンプ用のルート証明書がありません。
これがないと、`Mutagen.Bethesda.Synthesis` 0.36.6 の復元は `NU3018` と `NU3028` で失敗します。
そのため、`dotnetsdk10` を宣言したツールを実行する前に、SDK がダウンロード済みであれば、Eidos は 2 つのファイルの 385 個の証明書をゲームのプレフィックスの現在のユーザーのルート証明書ストアに追加します。これはゲームのパスを書き込むのと同じ `regedit` のインポートで行います(後述)。
どれかが消えた場合は、もう一度インポートします。
以後、そのプレフィックス内のすべての Windows プログラムがこれらのルート証明書を信頼します。
これがあれば、新しいプレフィックスと空の NuGet キャッシュで、同じ復元とビルドがシステムの Wine 11.21 と GE-Proton11-9 の両方で成功します。

Eidos は、頼むまで Tier 2 と Tier 3 の何もインストールしません:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # install what is missing (the bundle's copies first, then downloads)
```

GUI では同じ状態が Prereqs 欄の下に並び、欠けているものはボタンになっています。
同梱でもランタイムでも Tier 2 の verb でもないものは、ダウンロードとして提示される
のではなく、綴り間違いの可能性として報告されます。

`eidos prereqs <instance> --install` は、ゲーム、Steam、ツールがまだプレフィックスを
保持している間は拒否します。Tier 3 のランタイムは、プレフィックスがまだなくても
インストールされます。Eidos が自分で実行するインストーラにはプレフィックスが必要で、
ストアがゲームを一度も起動していない間は "start the game once through its store, then
install again" と言って止まります。Tier 2 の verb は一つずつインストールされ、成功する
たびに記録されるので、失敗のあとは同じコマンドで再開します。Heroic の設定がない GOG
や Epic のコピーでは、`--install` は同梱の DLL をコピーしたあと、Tier 2 の verb の前で
止まります。そのランナー自身の winetricks を使うよう告げ、次の確認でプレフィックスの
`winetricks.log` を読みます。

### DynDOLOD に `dotnet10` が要る理由

DynDOLOD 自身はオブジェクト LOD を作りません。LODGen を呼び出すだけで、三つ
同梱しています。`LODGenx64.exe` は .NET Framework 4.8 向けで、Proton の下では
Wine の Mono へ回されます。その `System.Uri` の初期化子は、Mono が実装して
いないメソッドを呼びます。仕事の一行目に入る前に死に、ログにはバージョンの
表示だけが残り、DynDOLOD のダイアログは「failed for one or more worlds」としか
言いません。

本物の .NET Framework を入れても直りません。Proton は `mscoree.dll`(それを
見つけるはずのローダー)を自身のツリーへのシンボリックリンクで置き換え、
プレフィックスの更新のたびにやり直すからです。

動くビルドは `LODGenx64Win10.exe` で、現代の .NET 向けであり `mscoree` に
触れません。`DOTNET_ROOT` を .NET 10 のランタイムへ向ければ動きます。
`dotnet10` が用意するのはそれで、Eidos はそれを宣言したツールを起動するときに
この変数を設定します。

### Pandora に `dotnet10` が要る理由

Pandora は WPF ではなく Avalonia のアプリで、Pandora 4.1.0 以降は `net10.0` を対象
にしているので、.NET 10 ランタイムだけで動きます。ランタイムが見つからないと、.NET
Desktop Runtime をインストールするよう言います。これは、ランタイムが見つからない
GUI プログラムに対して .NET のランチャーが出す汎用の文言で、`dotnetdesktop8` では
直りません。

Eidos 1.21.0 以前は Pandora に `dotnetdesktop8` を与えており、項目は `prereqs=` の行
を保ちます。Eidos が `dotnetdesktop8` だけを挙げている Pandora の項目を読み込み、
その実行ファイルの隣の `.runtimeconfig.json` が `net10.0` を挙げているときは、項目は
代わりに `dotnet10` になります。.NET 8 を対象にする Pandora 2.x は、今あるものを
保ちます。

### PGPatcher に `--ignore-mo2vfscheck` が要る理由

PGPatcher は、導入しているテクスチャ MOD が vanilla・parallax・complex
material・PBR のいずれであっても、各面で正しいシェーダーが使われるように
メッシュとプラグインを書き換えます。MOD マネージャーの外では動作を拒否しますが、
素のゲームフォルダに手を入れるのは破壊的だからです。

PGPatcher は usvfs を探します。Windows で MO2 がゲームに注入する DLL フック層です。
Eidos にそれはありません。統合ビューは Wine が存在を知らないまま入っていく FUSE
マウントだからです。したがって確認は何も見つけられず、ツールは作業を始める前に
次のメッセージで即座に終了します。

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

利用者が起動してもいないプログラムの名前を挙げてです。作者自身が二件の
Linux 報告([#716][pg716] と [#725][pg725]、いずれも同じ理由で FUSE を用いる
MO2 の Linux フォーク Fluorine から)に示した答えが、この確認を省くフラグです。

そこで Eidos があらかじめ入れます。MOD プールで見つかった PGPatcher は引数が
設定済みの状態で現れ、Executables ダイアログのタイトルに `PGPatcher` と入力
しても Arguments 欄が同じように埋まります。入力した内容が常に優先されます。
補完されるのは空のままにした欄だけで、これは上記の前提条件と同じ規則です。

一覧は `default_args`(`crates/eidos-instance/src/tools.rs`)にあり、
`default_prereqs` の隣です。

実行時に必要なものはありません。.NET ツールとしては珍しく、自前のランタイムを
同梱し(`dotnetlib/`、`includedFrameworks` を宣言する `runtimeconfig.json`
つき)、Microsoft の CRT DLL を実行ファイルの隣に置いています。Prereqs は空の
ままにしてください。

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### Mod Organizer 2 モードの PGPatcher と PGLite

Mod Manager が `None` のとき、PGPatcher には統合された `Data` フォルダが一つ
見えるだけで、メッシュやテクスチャがどの MOD から来たのか分かりません。そのため
示せる競合もなく、従うべき MOD の順序もありません。`Mod Organizer 2` モードは
インスタンスそのものを読みます。プロファイルの `modlist.txt`、有効な MOD とその
優先順位、各 MOD のフォルダ内のファイルです。ゲーム内版の PGLite も同じ方法で
インスタンスを読みます。Eidos のインスタンスはすでに MO2 の構成(`mods/`、
`profiles/<name>/modlist.txt`、`overwrite/`)なので、このモードで使えます。

PGPatcher のランチャーで次のようにします。

1. **Mod Manager** を `Mod Organizer 2` にします。
2. **MO2 Options -> Instance Location** を Eidos のインスタンスフォルダに
   します。Wine の `Z:` ドライブ(`/` にあたります)上の Windows パスとして書き
   ます。`/mnt/Games/Eidos-Skyrim` にあるポータブルインスタンスは
   `Z:\mnt\Games\Eidos-Skyrim`、グローバルインスタンスは
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>` です。
3. ツールの引数に `--ignore-mo2vfscheck` を残しておきます。このモードの
   PGPatcher は usvfs を探し、見つからなければ停止します。

PGPatcher はフォルダに `modorganizer.ini` がある場合にだけそれを受け付けます。
そこで Eidos は、ゲームやツールを起動するたびに、起動するプロファイルについて
インスタンスのルートに `ModOrganizer.ini` を書きます。内容が変わったときにだけ
書き直します。中身は次のとおりです。

- `[General]` に `gameName`(MO2 でのゲーム名。Fallout: New Vegas なら `New Vegas`)、`gamePath`(ゲームのストアのインストールフォルダを `Z:` パスで。ゲームのコピーから動くインスタンスでも同じです)、`selected_profile`
- `[Settings]` に、絶対 `Z:` パスでの `mod_directory`、`profiles_directory`、
  `overwrite_directory`。インスタンスに `downloads/` フォルダがあれば
  `download_directory` も

最初の行は、Eidos が書いたことを示すコメントです。この行のない
`ModOrganizer.ini` は、名前の大文字小文字がどうであれ、変更も削除もしません。
Eidos で開いた MO2 インスタンスは自分の ini をそのまま持ち、PGPatcher はその
ファイルをそのまま読みます。MO2 は設定を保存するときにコメントを捨てるので、
MO2 が上書き保存したファイルはそれ以降 MO2 のものです。空のファイルは Eidos のものとして扱います。

こうしたファイルには MO2 が最後に選んだプロファイルが入っており、PGPatcher は
Eidos が起動するプロファイルではなく、そのプロファイルを使います。両者が異なると、
Eidos は起動時に警告をログに出します。MO2 で同じプロファイルを選ぶか、ファイルの
名前を変えるか削除して、Eidos が自分のファイルを書けるようにしてください。

Mod Organizer 2 モードのツールは、ゲームもそのファイルの `gamePath` から読みます。
それが、インスタンスが動かすゲームフォルダにもストアのインストールにも当たらない
場合、Diagnostics タブに **ModOrganizer.ini points tools at another game folder** が
表示され、そのパスと、このインスタンスが動かすゲームフォルダが示されます。ファイル
の `gamePath` を直すか、ファイルを削除して Eidos に自分のものを書かせてください。

Eidos が自分のファイルを書くのは何かを起動するときだけなので、起動と起動の間は
最後に起動したプロファイルの名前が入っています。Eidos が起動したツールは、
そこに自分のプロファイルを見つけます。

PGPatcher は、プロファイルに有効な MOD が一つもないときと、出力フォルダが
有効な MOD のフォルダであるときに、MO2 の下と同じくエラーで停止します。

## ゲームのプレフィックスで動く Wine 自身のプログラム

File メニューは、ゲームのプレフィックスで Wine のプログラムを 4 つ実行します。winecfg、regedit、Wine のファイルエクスプローラー、アンインストーラーです。他のツールと同じように、ゲームのツールを動かす Proton または Wine のビルドで、MOD をマウントした状態で起動し、ウィンドウは終了を待ちます。**Prefix drive_c** は、プレフィックスの `C:` ドライブをファイルマネージャーで開きます。プレフィックスにそのプログラムがない間は行が灰色になります。ストアが初めてゲームを起動すると揃います。ターミナルから `eidos tool <instance> run winecfg` でも同じことができ、`regedit`、`explorer`、`uninstaller` も、その名前のタイトルのツールが一覧にないときは同様です。

regedit でゲームの `installed path` を変えても、次のツール実行で元に戻ります。次の節を参照してください。

## プレフィックスの中のゲームパス

Windows のツールは `HKLM\Software\Bethesda Softworks\<game>` の
`installed path` を読んでゲームを見つけます。ゲーム自身のインストーラが書く
キーで、Proton 下の Steam はそのインストーラを一度も走らせません。このキーが
ないと xEdit も Wrye Bash も DynDOLOD も空のパスで開きます。

ツールを実行する前に、Eidos はゲームのプレフィックスでの `regedit` のインポートで
このキーを書きます。以前のインポートがすでに同じパスを書いていて、プレフィックスが
それを保っている場合は除きます。キーが指すのはストアのインストールで、ゲームの
コピーから動くインスタンスでも同じです。統合ビューがそこにマウントされているから
です。`dotnetsdk10` を宣言したツールでは、同じインポートが NuGet の署名用ルート証明書
も追加します([上記](None#前提条件は三種類)を参照)。`runner=native` のツールは
この手順を飛ばします。プレフィックスがまだない、または別のプログラムが保持している
ときも、ツールは実行され、Eidos は、ゲームを一度起動すべきストア、または閉じるべき
プログラムを挙げた警告を出します。キーがストアのインストール以外のフォルダを指して
いる場合は、次のツール実行がそれをもう一度書くまで、Diagnostics タブに
**Windows tools cannot find <game>** が表示されます。

## ツールに手を伸ばす: 隠す、固定する、デスクトップショートカット

ゲームの既定には一度も使わないツールも含まれます。Executables ダイアログでは:

- **Pin to top** は項目を Run 一覧の先頭に置きます。
- **Hide from picker** は削除せずに一覧から外します。
- **Desktop shortcut** は `~/.local/share/applications` に `.desktop` を
  書くので、デスクトップではなくアプリケーションメニューと検索に出てきます。実行
  するのは `eidos tool <instance> run <title>` そのもので、Eidos のウィンドウを
  開かずに、ツールがこのインスタンスのプロファイルで統合ビュー越しに立ち上がります。

隠すことと固定することは、ツールへの辿り着き方だけを変えるので、自分で作った項目
だけでなくゲームごとの既定にも効きます。

スクリプトエクステンダのローダー、ランチャー、ゲーム本体には Desktop shortcut
ボタンがありません。これらはゲームを起動し、ゲームは常にそのストア経由で起動
するので、それはストア自身のショートカットがすでにやっています。Eidos 1.22
以前でこのいずれかに作ったショートカットは今も動きます。ターミナルで打った
`eidos tool <instance> run <title>` と同じく、ストアにゲームの起動を頼みます。
ストアが開いたのではないウィンドウで Run がするのと同じです。ストアの設定が
ない場合、このコマンドは何も起動せず、何を設定するかを表示します。
ショートカットはメッセージを出さないので、同じコマンドをターミナルで実行して
読んでください。

Eidos は、実行ファイルがスクリプトエクステンダのローダー、ランチャー、ゲーム本体の
ファイル名(大文字小文字は無視)であるときは、どの項目もこのようにストアに渡します。
既定の項目、自分で追加した項目、MO2 から取り込んだ項目、`--exe` の実行のどれでも
同じです。まずこのインスタンスを最後に使ったものとして記録し、ストアが開くウィンドウ
がこのインスタンスを表示するようにしてから、`steam -applaunch <id>`、Heroic なら
`xdg-open heroic://launch?appName=<app>&runner=<runner>`、または
`legendary launch <app>` を実行します。`--print` を付けるとそのストアのコマンドを表示
するだけで、何も起動しません。`steam` コマンドは `PATH` 上になければなりません。
Flatpak 版の Steam はそこに置かないので、起動は失敗し、メッセージは Steam から
ゲームを起動するよう告げます。

ストアがプロセスを起動したとき、つまり `SteamGameId`、`SteamAppId`、
`HEROIC_APP_NAME`、`HEROIC_APP_RUNNER`、`EIDOS_STORE_STARTED` のどれかが設定されて
いるときは、項目はどのツールとも同じように統合ビュー越しに直接実行されます。どれも
ない場合、Eidos が自分で Proton を使ってゲームを起動することはありません。インスタンス
にスクリプトエクステンダのローダーがない(動かしているゲームフォルダ、有効な MOD の
`Root/`、`Overwrite/Root/` のいずれにもない)場合は、何も起動せず、先にスクリプト
エクステンダをインストールするよう表示します。Heroic の外にある GOG のコピーや、
Eidos が渡せないストア ID では、そのストアからゲームを起動するよう告げます。

プロファイルごとに、ゲーム用のショートカットを持てます。プロファイルを右クリック
して **Desktop shortcut** を選ぶと、そのエントリは
`eidos tool <instance> run "<game>" --profile <name>` を実行します。このコマンド
はウィンドウでプロファイルを選んだときと同じくそのプロファイルをアクティブにし、
そのあとストアにゲームの起動を頼みます。ストアが開く Eidos のウィンドウにはその
プロファイルが表示されます。`--profile` はどの `eidos tool ... run` でも使え、
ツールが起動する前にプロファイルを選択します。`--print` と一緒なら何も選択しません。

## それ自体が Steam のアプリであるツール

Creation Kit は別個の Steam アプリケーションで、自分の AppID を欲しがります。
Steam で配られている他のいくつかの modding ツールも同じです。項目に
**Steam AppID** を設定すると、Eidos はゲームのものではなくその id で起動します。

Windows ではこれは別のランチャーを意味します。ここでは、同じ実行に環境変数を二つ
足すだけです。`SteamAppId` と `SteamGameId` です。Proton は一方を、Steam 自身の
ライブラリはもう一方を読むので、Eidos は両方を設定します。食い違いを見たツールは、
はっきりしないエラーで失敗するからです。`eidos tool ... --print` は、本当の
実行が受け取るものをそのまま見せます。

## 環境変数と、Proton を使わないツール

`<instance>/tools.ini` のキーのうち三つは、Executables ダイアログにまだ入力欄が
ありません。ツールのセクションに手で書き足してください。`env=NAME=value` の行
一つが、そのツールの実行にだけ効く変数を一つ設定します。

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos はこれらの行を、自分で設定するものをすべて設定した後、そしてプロファイルの
`launch-env.ini` の変数の後に適用します。そのため、ここに書いた値はその両方を置き
換えます。MOD が同梱する DLL から Eidos が組み立てる `WINEDLLOVERRIDES` も同じで、
これを設定するツールは、自分が必要とする DLL をすべてそこに並べる必要があります。
プロファイルのラッパーは、ゲームのコマンドと同じようにツールのコマンドも包みます。
[プロファイルごとの変数とラッパー](usage.md#プロファイルごとの変数とラッパー)を
参照してください。名前は英字、数字、`_` からなり、数字で始まりません。それ以外の
名前の行は無視されます。

`runner=native` は Proton を通さずに `exe` そのものを実行します。統合ビューは
どのツールとも同じようにマウントされ、`arg0=`、`arg1=` の行がその引数に
なります。Linux のプログラムや、Steam が管理しておらず Heroic にもランナーの
記録がないコピー向けのツールにはこれを使います。`exe` はそのコピーの Wine、
Windows のプログラムが最初の引数で、プレフィックスは `env=` で指定します。

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` は絶対パスでなければなりません。作業ディレクトリの既定値は `exe` のある
フォルダ、ここではランナーのフォルダなので、`workdir=` を設定してください。
統合されたパスに書き換えられるのは `exe` だけです。MOD の中にある Windows
プログラムには、統合ビューがそれを見せるゲームの `Data` フォルダ以下のパスを
指定してください。ネイティブのツールでは Proton 用の手順は行われません。上で
述べたレジストリキー、Steam のシェーダーキャッシュ、空の .NET ルート、
`LD_PRELOAD` からの Steam オーバーレイの除去がそれです。

Heroic でインストールした GOG または Epic のコピーでは、ゲームの Heroic 設定
(`GamesConfig/<appName>.json`)に Wine か Proton のビルドが記録されていれば、
ツールにこのエントリは要りません。Eidos はツールをそのビルドで、ゲームの
Heroic プレフィックスの中で実行し、`eidos prereqs <instance> --install` も
そのビルドをインストーラと winetricks に使います。Legendary だけでインストールしたコピーには
Heroic の設定がありません。

`slr=1` を書くと、ツールとその前のレジストリのインポートを、ツールの Proton ビルドが指定する Steam Linux Runtime のコンテナ内で起動します(`toolmanifest.vdf` の `require_tool_appid`)。Steam がゲームを起動するのと同じ方法です。
そのランタイムはいずれかの Steam ライブラリにインストールされている必要があり、ない場合は実行を止めて、インストールすべき Steam アプリを示します。
`eidos prereqs` は引き続きホスト上でインストーラと winetricks を実行し、`runner=native` のツールはこのキーを無視します。
コンテナから見えるのはホームフォルダとゲームの Steam ライブラリです。
それ以外の場所、たとえば別のドライブにあるツール、MOD、ランタイムには `env=STEAM_COMPAT_MOUNTS=/その/パス` が必要です(複数のパスは `:` で区切ります)。
`/usr/share/steam/compatibilitytools.d` にインストールした Proton は、コンテナ内の `/usr` がランタイムのものなので、`slr=1` ではおそらく使えません。
コンテナ内でのツールの実行はまだ試していません。

`eidos tool <instance> run <title> --print` は、最終的なコマンドと、実行が継承した
環境の上に設定する変数すべてを、これらの行も含めて表示します。ゲームを起動する項目
では、代わりにストアのコマンドを表示します
([ツールに手を伸ばす](None#ツールに手を伸ばす-隠す固定するデスクトップショートカット)を参照)。

## BethINI Pie

Eidos は BethINI Pie を同梱しません。Nexus のページからダウンロードしたものを、このインスタンスの `mods/` かツール用フォルダに置くと、ツール一覧に `Bethini.exe` が現れ、PGPatcher と同じゲームで使えます。Eidos 経由で実行すると、プロファイルが持つゲーム INI のコピーを編集します。タイトルに `bethini` を含むツールの実行のたびに、Eidos はその INI の復元ポイントを残し、ツールバーの **Backups**、続いて **INIs** で元に戻せます。コピーを取った時点で存在しなかった INI には手を付けません。ダイアログは種類ごとに新しい方から 10 個の復元ポイントを保ち、復元するときは先に現在の INI をバックアップするので、選び間違いも取り消せます。

## ツール自身の設定は、やはりツール自身のもの

Eidos はほとんどのツールの設定を確認しません。間違っていても、たいていは
エラーなしに失敗します。

BodySlide の **Game Data Path**(Settings)は、その上のゲームフォルダではなく、ゲームの
`Data` ディレクトリを指していなければなりません。一階層高く設定すると、
バッチビルドは「All sets processed successfully」と報告し、ゲームが決して
探さない場所に 1439 個のメッシュを書きます。Eidos はそれを `Overwrite/Root/` で
受け止めるので、ゲームのインストール先には触れず、ゲームから見たときの唯一の兆候は、
体型が作られていないことです。

BodySlide または Outfit Studio を実行する前に、優先される `CalienteTools/BodySlide/Config.xml` の `GameDataPath` か `OutputDataPath` が実行中にゲームの `Data` フォルダに通じていないと、ステータス行が警告し、Diagnostics タブには設定すべき値を添えた同じカードが表示されます。`eidos check <instance>` も報告します。`S:` のパスは、Eidos がツール用に `S:` を割り当てるとおり、Steam ライブラリのフォルダとして読みます。インスタンスの `mods/` フォルダ内の `OutputDataPath` は、出力 MOD 用として受け入れます。`Overwrite/Root/` に `meshes` または `textures` フォルダがある場合も、Diagnostics にカードが表示されます。

ツールの出力は Overwrite に属します。実行が取っておく価値のあるものを生んだら、
**Overwrite -> Create mod...** でそれを普通の MOD に変えられます。他と同じ
ように並べ替え、無効化し、削除できます。
