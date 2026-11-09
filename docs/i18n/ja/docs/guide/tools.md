<!-- eidos-i18n: source=docs/guide/tools.md sha=80ee63cfac4035db5e5010e5c389ef6bc6f5a370 -->

# ツール: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Eidos 越しに実行したツールは、ゲーム自身の Proton プレフィックスの中で
**統合ビュー**を見ます。ゲームが読むものをそのまま読み - 有効な MOD すべてを、
優先順位どおりに - 書き出したものは Overwrite に着地し、そこでひと押しすれば
本物の MOD になります。

## Eidos が自分で見つけるもの

名前が十分に一意なツールは、宣言しなくても見つけられます。xEdit がその典型で、
Fallout 4 なら `FO4Edit.exe`、Skyrim SE なら `SSEEdit.exe`、初代なら
`TES5Edit.exe`、といった具合です - それぞれの **QuickAutoClean** の片割れも
一緒に。LOOT が警告し続ける dirty edit のためのボタンがこれです。Eidos は
ファイル名で、次の場所を探します:

- ゲームのインストールフォルダと、有効な MOD の `Root/` ツリー;
- **このインスタンスの `mods/`**。MO2 の利用者がツールを入れる場所です;
- 環境設定で指定した**ツールフォルダ**(General -> Tools)。インスタンス間で
  共有するディレクトリ - `/mnt/Games/Tools` のようなもののためです。

`PGPatcher.exe` も Creation Engine 系のゲームで同じように見つかり、これがないと動作できない引数とともに登録されます - [下記](#pgpatcher-に---ignore-mo2vfscheck-が要る理由)を参照。

一覧はゲームごとなので、Skyrim のインスタンスに Fallout のエディタが出ることは
ありません。探索は四階層で止まります。MOD プールは数十万ファイルあり、これは
ツール一覧を組み立てるたびに走るからです。シンボリックリンクも辿りません。
こうして見つかったツールは、手で入力したものとまったく同じように設定されます。
ランタイムは名前から決まり、規則は以下すべてと同じです。

ツールが別の場所にある場合や、違う引数を渡したい場合は、手で追加してください -
同じタイトルのユーザー項目は、自動で見つかったものを上書きします。

## 追加する

GUI なら **Tools -> Executables** を開いて Add。コマンドラインなら:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
```

スクリプトエクステンダ、ゲーム本体、ランチャーは自動で検出されます。登録が
要るのは追加のツールだけです。

### どこにあろうと、実体のファイルを指す

実行ファイルは、実際に置かれている場所で登録します。ツールを MOD として
入れたなら、それは MOD フォルダの中です:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(これはグローバルインスタンスのパスです - ポータブルインスタンスでも同じ規則が
自身のフォルダの下 `<instance>/mods/...` に当てはまります。なお、こうした絶対
パスは、あとでポータブルフォルダを移動したときに唯一生き残らないものです)。

Eidos は起動前にそのパスを統合ビューのものへ書き換えます。ツールは
`<game>/Data/CalienteTools/BodySlide/` から動き、そこにある他の MOD の
ファイルもすべて見えます。これは聞こえる以上に効きます。BodySlide が同梱する
`SliderSets` ディレクトリは**空**で、作れる体型はすべて CBBE と衣装 MOD から
来ます。自分の MOD フォルダから起動すると何も見つからず、壊れているように
見えます。

MO2 も同じ理由で同じ書き換えをしています - そのコメントは FNIS を名指しして
います。

**無効な** MOD の中のツールは書き換えられません。そのファイルもビューに
無いからです。Eidos はそう伝えたうえで、取り繕わずに元のフォルダから実行します。

## ツールの出力を専用の MOD へ送る

ジェネレータ - FNIS、Nemesis、BodySlide、DynDOLOD、Synthesis - は何百もの
ファイルを書きます。既定では他のすべてと一緒に Overwrite に着地します。
Executables のエディタで **Capture output into** を設定すると、その実行の
出力は代わりにその MOD へ入ります:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

MOD が無ければ作られます。動くのはこの実行が生んだファイルだけで、すでに
Overwrite にあったものはそのまま残ります。だから取り込み先を持つ二つのツールが
互いの出力を奪い合うことはありません。何も書かなかった実行は、空の MOD を
残しません。

これは書き込みレイヤーを MOD に向けるのではなく、実行の後で行います。MO2 は
前者のやり方です。書き込みレイヤーを MOD に向けると、その MOD は実行のあいだ
最優先へ繰り上がり - 関わる競合をすべてひっくり返し、終わったらまた戻すことに
なり - MOD 自身のファイルにコピーアップ無しで直接書き込みます。取り込みは
そのどちらも無しに同じ最終状態へ辿り着きます。

対象の MOD が無効なら、出力は書かれますがゲームからは見えず、ツールは次の実行で
同じファイルを作り直すことになります。Eidos はその場合に警告します。

## ツールに要る DLL は、その名前で決まる

ここは意外なので、はっきり書きます。**ツールに付けたタイトルが、Eidos が
用意するランタイムの前提条件を決めます。** 照合はタイトルに対する大文字小文字を
無視した部分一致です。

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

つまり **`BodySlide`** として登録したツールは DirectX の DLL を受け取り、同じ
実行ファイルを **`BS`** として登録すると何も受け取らず、DLL について何も言わない
エラーで起動に失敗しうるということです。ツールにはプログラムの名前を
付けてください。

一覧は `default_prereqs`(`crates/eidos-instance/src/tools.rs`)にあり、
Executables ダイアログの `Prereqs` 欄は編集できます - この検出は既定であって、
規則ではありません。

タイトルは **引数** も同じように決めます。誰にも推測できない引数を必要とする
唯一のツールのためです。`pgpatcher`（または旧名の `parallaxgen`）を含む
タイトルには `--ignore-mo2vfscheck` が与えられます。理由と、それが任意では
ない理由は[下記](#pgpatcher-に---ignore-mo2vfscheck-が要る理由)。

### 前提条件は三種類

**Tier 1 - 同梱の DLL**(`d3dx9_43`、`d3dcompiler_47`、`d3dcompiler_43`、
`d3dx11_43`)。Eidos が
同梱しており、起動時にプレフィックスへコピーします。やることは無く、ネット
ワークも要りません。

**Tier 2 - winetricks の verb**(`vcrun2022`、`dotnet8`、`dotnetdesktop8`、
`dotnet48`、`xact` など)。これらはレジストリキー、GAC、CLR ホストを書くので、
ファイルのコピーでは済みません。**Microsoft からダウンロード**します。

**Tier 3 - ランタイム**(`dotnet10`, `dotnetsdk10`)。現代の .NET ランタイムは 193 個の
ファイルで、専用のディレクトリに置かれ、`DOTNET_ROOT` を通して見つけられます。
登録もされず、プレフィックスにインストールもされないので、他のどちらの Tier でも
運べません。Eidos は自分でダウンロードし、バイナリに埋め込んだチェックサムで
検証して、`~/.local/share/Colony/Eidos/runtimes/` にキャッシュします -
**どのインスタンスの外側**でもあります。78 MB はゲームごとでもプロファイル
ごとでもないからです。

`dotnetsdk10` は .NET 10 SDK で、ダウンロードは 301 MB、展開すると 808 MB です。
Synthesis はパッチャーのコンパイルにこれを必要とし、ビルドと実行にはパスなしの `dotnet` を呼び出します。
そのため `dotnetsdk10` を宣言したツールには、`WINEPATH` 上の SDK フォルダに加えて、`NUGET_CERT_REVOCATION_MODE=offline`、`DOTNET_GENERATE_ASPNET_CERTIFICATE=false`、`DOTNET_CLI_TELEMETRY_OPTOUT=1` も設定されます。
`tools.ini` の Synthesis エントリが古い既定値 `dotnet8`, `vcrun2022` のままなら、Eidos が読み込むときに `dotnetsdk10`, `vcrun2022` に置き換わります。

NuGet は復元するすべてのパッケージの署名を Windows のルート証明書ストアで確認しますが、Wine のプレフィックスには、SDK が `sdk/10.0.403/trustedroots/codesignctl.pem` と `timestampctl.pem` として同梱しているコード署名用とタイムスタンプ用のルート証明書がありません。
これがないと、`Mutagen.Bethesda.Synthesis` 0.36.6 の復元は `NU3018` と `NU3028` で失敗します。
そのため、`dotnetsdk10` を宣言したツールを実行する前に、SDK がダウンロード済みであれば、Eidos は 2 つのファイルの 385 個の証明書をゲームのプレフィックスの現在のユーザーのルート証明書ストアに追加します。これはゲームのパスを書き込むのと同じ `regedit` のインポートで行います (後述)。
どれかが消えた場合は、もう一度インポートします。
以後、そのプレフィックス内のすべての Windows プログラムがこれらのルート証明書を信頼します。
これがあれば、新しいプレフィックスと空の NuGet キャッシュで、同じ復元とビルドがシステムの Wine 11.21 と GE-Proton11-9 の両方で成功します。

Tier 2 と Tier 3 は、どちらも黙って動くことがありません:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

GUI では同じ状態が Prereqs 欄の下に並び、欠けているものはボタンになっています。
同梱でもランタイムでも既知の winetricks の verb でもないものは、ダウンロードと
して提示されるのではなく、綴り間違いの可能性として報告されます。

### DynDOLOD に `dotnet10` が要る理由

DynDOLOD 自身はオブジェクト LOD を作りません。LODGen を呼び出すだけで、三つ
同梱しています。`LODGenx64.exe` は .NET Framework 4.8 向けで、Proton の下では
Wine の Mono へ回されます - その `System.Uri` の初期化子は、Mono が実装して
いないメソッドを呼びます。仕事の一行目に入る前に死に、ログにはバージョンの
表示だけが残り、DynDOLOD のダイアログは「failed for one or more worlds」としか
言いません。

本物の .NET Framework を入れても直りません。Proton は `mscoree.dll` - それを
見つけるはずのローダー - を自身のツリーへのシンボリックリンクで置き換え、
プレフィックスの更新のたびにやり直すからです。

動くビルドは `LODGenx64Win10.exe` で、現代の .NET 向けであり `mscoree` に
触れません。`DOTNET_ROOT` を .NET 10 のランタイムへ向ければ動きます。
`dotnet10` が用意するのはそれで、Eidos はそれを宣言したツールを起動するときに
この変数を設定します。

Eidos はシステムの `winetricks` を、Proton 自身の `wine` とゲームのプレフィックス
に対して実行します。これで Steam の pressure-vessel コンテナと、
protontricks + Proton-GE の食い違いを回避します。未インストールの Tier 2 の
verb を宣言したツールも起動はします。その verb と、直すためのコマンドを挙げた
警告付きで - 別の経路ですでに入っていることもあるからです。

### PGPatcher に `--ignore-mo2vfscheck` が要る理由

PGPatcher は、導入しているテクスチャ MOD が vanilla・parallax・complex
material・PBR のいずれであっても、各面で正しいシェーダーが使われるように
メッシュとプラグインを書き換えます。MOD マネージャーの外では動作を拒否しますが、
これは妥当です。素のゲームフォルダーに手を入れるのは破壊的だからです。

問題はその確認の「やり方」です。Windows で MO2 がゲームに注入する DLL フック層、
**usvfs** を探すのです。Eidos にそれはなく、今後もありません。統合ビューは Wine
が存在を知らないまま入っていく FUSE マウントだからです。したがって確認は何も
見つけられず、ツールは作業を始める前に次のメッセージで即座に終了します。

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

しかも、利用者が起動してもいないプログラムの名前を挙げてです。作者自身が二件の
Linux 報告（[#716][pg716] と [#725][pg725]、いずれも同じ理由で FUSE を用いる
MO2 の Linux フォーク Fluorine から）に示した答えが、この確認を省くフラグです。

そこで Eidos があらかじめ入れます。MOD プールで見つかった PGPatcher は引数が
設定済みの状態で現れ、Executables ダイアログのタイトルに `PGPatcher` と入力
しても Arguments 欄が同じように埋まります。入力した内容が常に優先されます。
補完されるのは空のままにした欄だけで、これは上記の前提条件と同じ規則です。

一覧は `default_args`（`crates/eidos-instance/src/tools.rs`）にあり、
`default_prereqs` の隣です。項目は一つですが、一つで関数を設ける価値があります。
フラグは誰にも推測できず、防いでいる失敗が別のプログラムの名を挙げるからです。

実行時に必要なものはありません。.NET ツールとしては珍しく、自前のランタイムを
同梱し（`dotnetlib/`、`includedFrameworks` を宣言する `runtimeconfig.json`
つき）、Microsoft の CRT DLL を実行ファイルの隣に置いています。Prereqs は空の
ままで構いません。

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### Mod Organizer 2 モードの PGPatcher と PGLite

Mod Manager が `None` のとき、PGPatcher には統合された `Data` フォルダーが一つ
見えるだけで、メッシュやテクスチャがどの MOD から来たのか分かりません。そのため
示せる競合もなく、従うべき MOD の順序もありません。`Mod Organizer 2` モードは
インスタンスそのものを読みます。プロファイルの `modlist.txt`、有効な MOD とその
優先順位、各 MOD のフォルダー内のファイルです。ゲーム内版の PGLite も同じ方法で
インスタンスを読みます。Eidos のインスタンスはすでに MO2 の構成（`mods/`、
`profiles/<name>/modlist.txt`、`overwrite/`）なので、このモードで使えます。

PGPatcher のランチャーで次のようにします。

1. **Mod Manager** を `Mod Organizer 2` にします。
2. **MO2 Options -> Instance Location** を Eidos のインスタンスフォルダーに
   します。Wine の `Z:` ドライブ（`/` にあたります）上の Windows パスとして書き
   ます。`/mnt/Games/Eidos-Skyrim` にあるポータブルインスタンスは
   `Z:\mnt\Games\Eidos-Skyrim`、グローバルインスタンスは
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>` です。
3. ツールの引数に `--ignore-mo2vfscheck` を残しておきます。このモードの
   PGPatcher は usvfs を探し、見つからなければ停止します。

PGPatcher はフォルダーに `modorganizer.ini` がある場合にだけそれを受け付けます。
そこで Eidos は、ゲームやツールを起動するたびに、起動するプロファイルについて
インスタンスのルートに `ModOrganizer.ini` を書きます。内容が変わったときにだけ
書き直します。中身は次のとおりです。

- `[General]` に `gameName`（MO2 でのゲーム名。Fallout: New Vegas なら `New Vegas`）、`gamePath`（ゲームのインストールフォルダーを `Z:`
  パスで）、`selected_profile`
- `[Settings]` に、絶対 `Z:` パスでの `mod_directory`、`profiles_directory`、
  `overwrite_directory`。インスタンスに `downloads/` フォルダーがあれば
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

Eidos が自分のファイルを書くのは何かを起動するときだけなので、起動と起動の間は
最後に起動したプロファイルの名前が入っています。Eidos が起動したツールは、
そこに自分のプロファイルを見つけます。

PGPatcher は、プロファイルに有効な MOD が一つもないときと、出力フォルダーが
有効な MOD のフォルダーであるときに、MO2 の下と同じくエラーで停止します。

## プレフィックスの中のゲームパス

Windows のツールは `HKLM\Software\Bethesda Softworks\<game>` の
`installed path` を読んでゲームを見つけます。ゲーム自身のインストーラが書く
キーで - Proton 下の Steam はそのインストーラを一度も走らせません。これが
無いと xEdit も Wrye Bash も DynDOLOD も空のパスで開きます。Eidos はツールを
実行する前にこれを書きます。冪等で、追加のみ、プレフィックスが未初期化か
使用中ならスキップします。

## ツールに手を伸ばす: 隠す、固定する、デスクトップショートカット

ゲームの既定には一度も使わないツールも含まれます。二番目に辿り着くのに八項目を
並べるピッカーは、誰も読まないピッカーです。Executables ダイアログでは:

- **Pin to top** は項目を Run 一覧の先頭に置きます。
- **Hide from picker** は削除せずに一覧から外します。
- **Desktop shortcut** は `~/.local/share/applications` に `.desktop` を
  書きます - freedesktop なシステムでランチャーが属する場所なので、デスク
  トップではなくアプリケーションメニューと検索に出てきます。実行するのは
  `eidos tool <instance> run <title>` そのもので、つまり Eidos のウィンドウを
  まったく開かずに、ツールは**このインスタンスのプロファイルで統合ビュー越しに**
  立ち上がります。

隠すことと固定することは、何を実行するかではなくツールへの*辿り着き方*の話
なので、自分で作った項目だけでなくゲームごとの既定にも効きます。

## それ自体が Steam のアプリであるツール

Creation Kit は別個の Steam アプリケーションで、自分の AppID を欲しがります。
Steam で配られている他のいくつかの modding ツールも同じです。項目に
**Steam AppID** を設定すると、Eidos はゲームのものではなくその id で起動します。

Windows ではこれは別のランチャーを意味します。ここでは、すでに組み立て中だった
実行に環境変数を二つ足すだけです - `SteamAppId` と `SteamGameId` の両方。
Proton は一方を、Steam 自身のライブラリはもう一方を読み、食い違いを見たツールは
明快にではなく妙な形で失敗するからです。`eidos tool ... --print` は、本当の
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

Eidos はこれらの行を、自分で設定するものをすべて設定した後に適用します。
そのため、ここに書いた値は Eidos 自身の値を置き換えます。MOD が同梱する DLL から
Eidos が組み立てる `WINEDLLOVERRIDES` も同じで、これを設定するツールは、自分が
必要とする DLL をすべてそこに並べる必要があります。名前は英字、数字、`_` から
なり、数字で始まりません。それ以外の名前の行は無視されます。

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
フォルダー、ここではランナーのフォルダーなので、`workdir=` を設定してください。
統合されたパスに書き換えられるのは `exe` だけです。MOD の中にある Windows
プログラムには、統合ビューがそれを見せるゲームの `Data` フォルダー以下のパスを
指定してください。ネイティブのツールでは Proton 用の手順は行われません。上で
述べたレジストリキー、Steam のシェーダーキャッシュ、空の .NET ルート、
`LD_PRELOAD` からの Steam オーバーレイの除去がそれです。

Heroic でインストールした GOG または Epic のコピーでは、ゲームの Heroic 設定
(`GamesConfig/<appName>.json`) に Wine か Proton のビルドが記録されていれば、
ツールにこのエントリは要りません。Eidos はツールをそのビルドで、ゲームの
Heroic プレフィックスの中で実行し、`eidos prereqs <instance> --install` も
winetricks にそのビルドを使わせます。Legendary だけでインストールしたコピーには
Heroic の設定がありません。

`slr=1` を書くと、ツールとその前のレジストリのインポートを、ツールの Proton ビルドが指定する Steam Linux Runtime のコンテナ内で起動します (`toolmanifest.vdf` の `require_tool_appid`)。Steam がゲームを起動するのと同じ方法です。
そのランタイムはいずれかの Steam ライブラリにインストールされている必要があり、ない場合は実行を止めて、インストールすべき Steam アプリを示します。
`eidos prereqs` は引き続きホスト上で winetricks を実行し、`runner=native` のツールはこのキーを無視します。
コンテナから見えるのはホームフォルダーとゲームの Steam ライブラリです。
それ以外の場所、たとえば別のドライブにあるツール、MOD、ランタイムには `env=STEAM_COMPAT_MOUNTS=/その/パス` が必要です (複数のパスは `:` で区切ります)。
`/usr/share/steam/compatibilitytools.d` にインストールした Proton は、コンテナ内の `/usr` がランタイムのものなので、`slr=1` ではおそらく使えません。
コンテナ内でのツールの実行はまだ試していません。

`eidos tool <instance> run <title> --print` は、最終的なコマンドと、実行が継承した
環境の上に設定する変数すべてを、これらの行も含めて表示します。

## ツール自身の設定は、やはりツール自身のもの

Eidos はツールを正しい場所に、正しい DLL とともに置きます。そのうえでツールが
自分の設定をどう扱うかは、あなたとツールのあいだの話で、失敗はたいてい黙って
起きます。

実例をひとつ。知らないと一時間を失うからです。BodySlide の
**Game Data Path**(Settings)は、その上のゲームフォルダではなく、ゲームの
`Data` ディレクトリを指していなければなりません。一階層高く設定すると、
バッチビルドは「All sets processed successfully」と報告し、ゲームが決して
探さない場所に 1439 個のメッシュを書きます。Eidos はそれを受け止め -
インストール先ではなく `Overwrite/Root/` に着地します - しかしゲームから
見れば、体型が作られていないこと以外に何もおかしくありません。

ツールの出力は Overwrite に属します。実行が取っておく価値のあるものを生んだら、
**Overwrite -> Create mod...** でそれを普通の MOD に変えられます。他と同じ
ように並べ替え、無効化し、削除できます。
