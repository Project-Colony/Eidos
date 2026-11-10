<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Eidos のインストール

入口は四つ。Colony はプログラム全体である `eidos-linux` というファイルを一つだけ
インストールします。リリースバンドル、Arch パッケージ、ソースからのビルドでは、
`eidos`(CLI)、`eidos-gui`、静的な NIF プレビュー用ヘルパー `eidos-nif-preview` が
並んで入ります。Arch パッケージとソースビルドは、さらに下位の 2 つの CLI である
`eidos-fuse` と `eidos-launch` も入れます。どの経路でも、Nexus の「Mod Manager
Download」ボタンからインスタンスへダウンロードを届ける `nxm://` ハンドラを登録
できます。

## 先に必要なもの

| 必要なもの | 理由 |
|---|---|
| FUSE のある Linux | PATH に `fusermount3`。現行のディストリビューションはどれも同梱しています。 |
| 一度起動した Proton のゲーム | Steam はゲームの Wine プレフィックスを初回起動時にしか作らず、Eidos はその中で動きます。Eidos は、Skyrim Special Edition の GOG 版と Epic 版、Fallout 3 と Fallout: New Vegas の Epic 版、Stardew Valley の GOG 版も検出します。これらは Heroic か Legendary 経由で、Eidos をそこでのゲームのラッパーにして動かします。[usage.md](usage.md#ゲームの起動) を参照。 |
| 7-Zip のバイナリ: `7z`、`7zz`、`7za` | MOD 書庫のインストールに使います。Arch では `7zip`、多くのディストリビューションでは `p7zip`。 |
| `par2`(任意) | `eidos pack` のバックアップ用の PAR2 修復データ。Arch と Fedora では `par2cmdline`、Debian と Ubuntu では `par2`。 |

root も、デーモンも、`/etc/fuse.conf` の編集も、グループへの追加も要りません。
Eidos はゲームのプロセスに属するプライベートな名前空間の中でマウントします。

システム設定の中には、クラッシュとしてしか現れないものがあります。Diagnostics
タブは `vm.max_map_count`(1048576 以上)、open files のハードリミット(524288 以上)、
そしてゲーム、そのプレフィックス、インスタンスが NTFS、exFAT、FAT の上にないかを
確認します。それぞれの対処は [troubleshooting.md](troubleshooting.md#システム) に
あります。

## Colony 経由

[Colony](https://github.com/Project-Colony/Colony) で **Eidos** を検索して
インストールします。更新はランチャー経由で届きます。Colony が置くのは
`~/.local/share/Colony/apps/Eidos/eidos-linux` というファイル一つです。これを
実行するとウィンドウが開きます。`eidos` という名前のリンク経由で呼び出すと、
同じファイルがコマンドラインになります。`PATH` 上にリンクを一つ作り、Nexus の
ダウンロードハンドラを登録してください:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

ウィンドウのほうにリンクは要りません。ゲームやツールを起動するときは、いつも
自分自身のファイルを CLI として実行します。NIF モデルのプレビューには
`eidos-nif-preview` が必要で、これを含むのはリリースバンドル、Arch パッケージ、
ソースからのビルドだけです。ない場合はプレビューがその旨を表示し、ほかには何の
影響もありません。`eidos-linux` は、バンドルが同梱する Microsoft のランタイム
([下記](#バンドルに入っている-microsoft-のランタイム))も持っていません。ツールが必要と
するときは、前提条件のインストールを始めた時点で Eidos が Microsoft から
ダウンロードします。Steam は、下のとおりファイルそのものに向けてください。

## リリースバンドル

[最新リリース](https://github.com/Project-Colony/Eidos/releases/latest) から
`eidos-linux-bundle.tar.gz` と、その `.sig`、`.meta`、`.meta.sig` を
ダウンロードし、README の [コード署名ポリシー](../../README.md#コード署名ポリシー)
に示す方法で検証します。バンドルの最上位には `eidos`、`eidos-gui`、
`eidos-nif-preview` があり、ほかに `install.sh`、`README.md`、`LICENSE`、
`SECURITY.md`、`assets/brand/png` のアイコン、`licenses/` のサードパーティ表示
(nifly、omod、microsoft-directx、microsoft-runtimes と Rust クレート)、そして下で
説明する `runtimes/` フォルダが入っています。`eidos-fuse` と `eidos-launch` は
含まれません。そのあと:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` はバイナリを `~/.local/bin` に、`$XDG_BIN_HOME` が設定されていれば
そこにコピーします。`--system` なら sudo 経由で `/usr/local/bin`、`--bindir DIR`
なら任意の場所へ、`--from DIR` は別のディレクトリからバイナリを取ります。
スクリプトは次のことも行います。

- `eidos nxm --register` を実行して `nxm://` ハンドラを登録するので、Colony の節の
  登録の手順は要りません。
- `~/.local/share/applications/eidos.desktop` と、`~/.local/share/icons/hicolor`
  以下の Eidos のアイコンを書きます。
- ライセンス表示を `~/.local/share/licenses/eidos` にコピーします
  (`--system` なら `/usr/local/share/licenses/eidos`)。
- `runtimes/` を `~/.local/share/Colony/Eidos/runtimes` にコピーします。`--system`
  でも同じなので、ランタイムはスクリプトを実行したユーザーのものになります。
- 最後に、インストールした `eidos-gui` の絶対パス入りの Steam 起動オプションを
  表示します。

`--cap` は、インストールした `eidos` に `CAP_SYS_ADMIN` を与えるよう sudo に頼みます
(下の FUSE パススルーを参照)。`--no-cap` が既定で、root 権限なしのままにします。
再実行が想定された更新手順です。バンドルと `eidos-linux` には、glibc 2.39 以降の
x86_64 Linux が必要です。

### バンドルに入っている Microsoft のランタイム

バンドルの `runtimes/` フォルダには、ツールが求める Microsoft のファイルが入って
います。`VC_redist.x86.exe` と `VC_redist.x64.exe`(Visual C++ 2015-2022、
`vcrun2022` 前提条件)、`windowsdesktop-runtime-8.0.12-win-x64.exe`(.NET 8 Desktop
Runtime、`dotnet8` と `dotnetdesktop8` 用)、`dotnet-runtime-10.0.10-win-x64.zip`
(.NET 10 ランタイム、`dotnet10` 用)です。Eidos は Visual C++ と .NET 8 の
インストーラーを、winetricks を使わずにゲームのプレフィックスの中で自分で実行し、
.NET 10 の zip は `~/.local/share/Colony/Eidos/runtimes` に一度だけ展開します。
そこはすべてのインスタンスが共有します(展開後 78 MB)。各ファイルは、Eidos に
固定された SHA-256 と一致するときだけ使われます。Synthesis が必要とする .NET 10
SDK(`dotnetsdk10`、約 300 MB)はバンドルに入っておらず、常にダウンロードされます。

展開したバンドルはその場でも動きます。Eidos は、上のデータフォルダの次に、自分の
バイナリの隣の `runtimes/` を探します。

## Arch

```bash
cd packaging && makepkg -si
```

この PKGBUILD は現在の `main` をソースからビルドします。`eidos`、`eidos-gui`、
`eidos-nif-preview`、`eidos-fuse`、`eidos-launch` を `/usr/bin` に入れ、システム
全体の `nxm://` ハンドラ、デスクトップエントリ、アイコンも用意します。依存は
`fuse3`、`7zip`、`xdg-utils` です。`runtimes/` フォルダは含まないので、Microsoft の
ランタイムはツールが必要としたときにダウンロードされます。`winetricks` は任意
です。Eidos が自分ではインストールしない前提条件(`dotnet48` や `xact` など)には、
今も winetricks を使います。

## ソースから

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

ソースからのビルドには Rust 1.92 以降、`just`、CMake 3.20 以降、C++17 コンパイラ、
およびネイティブヘルパーのテスト用に Python 3 が必要です。`just build` は
ワークスペース全体をビルドするので `eidos-fuse` と `eidos-launch` も作られ、
`just install` はビルド出力に対して `packaging/install.sh` を実行し、残りと一緒に
それらもコピーします。`just` を使わないコマンドは
[ネイティブヘルパーの手順](../../../../../crates/eidos-nif-preview/README.md)を
参照してください。Rust を `cargo build --release --locked` でビルドし、ヘルパーを
`target/release` にコピーしてから `packaging/install.sh --from target/release` を
実行します。ソースビルドにも `runtimes/` フォルダはないので、Microsoft の
ランタイムはツールが必要としたときにダウンロードされます。

リリースプロファイルは、codegen unit 1、fat LTO、シンボルの strip を使います。
rustc 1.99.1、ビルドジョブ 3 で、`eidos` と `eidos-gui` のクリーンビルドは 334 秒
かかり(thin LTO では 243 秒)、`eidos` は 20.9 MB、`eidos-gui` は 49.7 MB になり
ました。各バイナリは 8 つの DirectX DLL を gzip 圧縮で埋め込んでおり、17.9 MB の
ところ 7.9 MB です。`just profile=dev build` は最適化なしでビルドします。

## そのあと: Steam をここへ向ける

Eidos はあなたのゲームの起動コマンド*として*動きます。だからこそゲームが
始まる前にマウントできます。Steam でゲームを右クリック -> プロパティ ->
起動オプションを開き、Eidos のウィンドウの絶対パスに続けて `%command%` を
入力します:

```
~/.local/bin/eidos-gui %command%
```

絶対パスを使ってください - Steam はシェルの `PATH` を読みません。パスは Eidos を
どうインストールしたかで変わり、`install.sh` は使ったディレクトリ向けの行を
表示します:

| インストール方法 | 起動オプション |
|---|---|
| リリースバンドルまたは `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Arch パッケージ | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

「プレイ」を押します。Eidos はそのゲームのインスタンスで開きます。MOD を入れ、
LOOT で並べ替え、Run を押してください。終了するとマウントも一緒に消え、
インストール先は元のままです。

ゲームは常にそのストア経由で起動します。自分で開いたウィンドウで Run を押すと
`steam -applaunch <appid>` が実行され、Steam が起動オプションを通して Eidos を
起動し、2 つ目の Eidos ウィンドウが開くので、そこで Run を押します。起動オプションが
Eidos を起動するまで、Run は何も起動せず、ステータス行に設定する行が表示されます。
Start the game at once when a store opens Eidos(Preferences, General, Running a
game。既定ではオフ)をオンにすると、Health に問題が一つもないとき、Steam の
Play はウィンドウなしでゲームを起動します。Heroic と Legendary もラッパー設定を
通して同じように動きます。詳細は [usage.md](usage.md#ゲームの起動) に。

Diagnostics タブはこの行を確認します。起動オプションが空、Eidos のバイナリを
指していない、存在しないファイルを指している、Eidos の後に `%command%` がない、
または実行中のものとは別の Eidos のコピーを起動する場合に、Health カードが
設定する行を示します。

### 端末のほうが好みなら

```sh
eidos init skyrimse               # インスタンスを作る(フォルダを渡せばポータブル)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root の MOD
eidos sort skyrimse               # LOOT でロード順を並べ替える
eidos check skyrimse              # ウィンドウを要しない Health チェック
eidos play skyrimse -- %command%  # 何でも統合ビュー越しに実行する
eidos shell skyrimse              # 統合ビューのシェル。書き込みは Overwrite へ
```

ゲーム ID を取るコマンドは、ポータブルインスタンスのフォルダも同じように
受け取ります - [usage.md](usage.md#インスタンス-グローバルとポータブル) を参照。
詳しい案内は [usage.md](usage.md) に。

## 更新

Colony は `eidos-linux` をランチャー経由で更新します。バンドルなら、新しい
バンドルの `install.sh` を実行します。Arch パッケージなら `makepkg -si` をもう一度
実行します。ソースビルドなら pull して `just install` を実行します。ウィンドウは
起動時に GitHub へ最新リリースを問い合わせます(1 日に 1 回まで)。より新しい版が
あれば、メニューバーの Help の後ろにリリースノートへのリンクが出ます。自動で
ダウンロードしたり、自分を置き換えたりはしません。Preferences, About, Version で
この確認を切れます。オフラインモードでは確認を行わず、`/usr` 以下にインストール
されたコピー(Arch パッケージ)は、パッケージマネージャーが更新するので問い合わせ
ません。

## 任意: FUSE パススルー

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` は任意のケーパビリティを
付与しますが、パススルーは有効にしません。`./install.sh --cap` と `just setcap` は、
それぞれがインストールまたはビルドした `eidos` に付与します。`nosuid` のマウントでは
カーネルがファイルケーパビリティを無視するので、`install.sh` は警告を出して付与を
飛ばし、`just setcap` はエラーで止まります。ケーパビリティを付けるのは、バンドル、
Arch パッケージ、ソースビルドの独立した `eidos` バイナリであり、ウィンドウでもある
`eidos-linux` には決して付けません([SECURITY.md](../../../../../SECURITY.md))。
再ビルドや再インストールしたバイナリはケーパビリティのない新しいファイルなので、
使うなら更新のたびに付け直してください。

`EIDOS_FUSE_PASSTHROUGH=1` は別の実行時スイッチで、既定ではオフです。パススルーを
有効にすると、以前のデーモンでは Skyrim SE が MOD の内容を一切読み込まなくなり
ました。最も可能性の高い原因は、そのデーモンがオープンのたびに新しいバッキング
ファイルを登録していたのに、カーネルは inode ごとに一つしか受け付けないことです。
デーモンはいま、一つのファイルのパススルーのオープン全部で一つのバッキングを
共有しますが、それはまだゲームで再測定されていません。それまでは、テストのため
だけにパススルーを有効にしてください。実測値と原因は
[troubleshooting.md](troubleshooting.md) にあります。

## すでに何かおかしい?

[troubleshooting.md](troubleshooting.md) に環境スイッチ、操作カウンタの読み方、
Health カード、これまで誰かを困らせた問題のすべてがあります。
