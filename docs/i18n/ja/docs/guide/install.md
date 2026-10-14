<!-- eidos-i18n: source=docs/guide/install.md sha=40f02a87874c44292179b6dff322d0f623c307ff -->

# Eidos のインストール

入口は四つ。Colony はプログラム全体である `eidos-linux` というファイルを一つだけインストールします。ほかの方法では `eidos`（コマンドライン）、`eidos-gui`、静的な NIF プレビュー用ヘルパーが並んで入り、さらに Nexus の「Mod Manager Download」ボタンからインスタンスへダウンロードを届ける `nxm://` ハンドラが用意されます。

## 先に必要なもの

| | |
|---|---|
| **FUSE のある Linux** | PATH に `fusermount3`。現行のディストリビューションはどれも同梱しています。 |
| **一度起動した Proton のゲーム** | Steam はゲームの Wine プレフィックスを初回起動時にしか作らず、Eidos はその中で動きます。 |
| **`7z`** | MOD 書庫の展開に使います。多くのディストリビューションでは `p7zip`。 |

root も、デーモンも、`/etc/fuse.conf` の編集も、グループへの追加も要りません。
Eidos はゲームのプロセスに属するプライベートな名前空間の中でマウントします。

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
`eidos-nif-preview` が必要で、これを含むのはリリースバンドルとソースからの
ビルドだけです。ない場合はプレビューがその旨を表示し、ほかには何の影響も
ありません。Steam はファイルそのものに向けてください(下記のとおり)。

## リリースバンドル

[最新リリース](https://github.com/Project-Colony/Eidos/releases/latest) から
`eidos-linux-bundle.tar.gz` と、その `.sig`、`.meta`、`.meta.sig` を
ダウンロードし、README の [コード署名ポリシー](../../README.md#コード署名ポリシー)
に示す方法で検証します。そのあと:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

既定では `~/.local/bin` に入ります。`--system` なら `/usr/local/bin`、
`--bindir DIR` なら任意の場所へ。再実行が想定された更新手順です。
バンドルと `eidos-linux` には、glibc 2.39 以降の x86_64 Linux が必要です。

## Arch

```bash
cd packaging && makepkg -si
```

この PKGBUILD は現在の `main` をソースからビルドします。

## ソースから

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

ソースからのビルドには Rust 1.89 以降、`just`、CMake 3.20 以降、C++17 コンパイラ、およびネイティブヘルパーのテスト用に Python 3 が必要です。`just` を使わないコマンドは[ネイティブヘルパーの手順](../../../../../crates/eidos-nif-preview/README.md)を参照してください。Rust を `cargo build --release --locked` でビルドし、ヘルパーを `target/release` にコピーしてから `packaging/install.sh --from target/release` を実行します。

## そのあと: Steam をここへ向ける

Eidos はあなたのゲームの起動コマンド*として*動きます。だからこそゲームが
始まる前にマウントできます。Steam でゲームを右クリック -> プロパティ ->
起動オプション:

```
~/.local/bin/eidos-gui %command%
```

Colony からの場合、パスは `~/.local/share/Colony/apps/Eidos/eidos-linux` です。
「プレイ」を押します。Eidos はそのゲームのインスタンスで開きます。MOD を入れ、
LOOT で並べ替え、Run を押してください。終了するとマウントも一緒に消え、
インストール先は元のままです。

絶対パスを使ってください - Steam はシェルの `PATH` を読みません。

### 端末のほうが好みなら

```sh
eidos init skyrimse               # インスタンスを作る(フォルダを渡せばポータブル)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root の MOD
eidos sort skyrimse               # LOOT でロード順を並べ替える
eidos play skyrimse -- %command%  # 何でも統合ビュー越しに実行する
```

ゲーム ID を取るコマンドは、ポータブルインスタンスのフォルダも同じように
受け取ります - [usage.ja.md](usage.md#インスタンス-グローバルとポータブル) を参照。
詳しい案内は [usage.ja.md](usage.md) に。

## 任意: FUSE パススルー

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` は任意のケーパビリティを付与しますが、パススルーは有効にしません。
これを付けるのはバンドルまたはソースビルドの独立した `eidos` バイナリであり、
ウィンドウでもある `eidos-linux` には決して付けません([SECURITY.md](../../../../../SECURITY.md))。
実行時に有効にするには、別途 `EIDOS_FUSE_PASSTHROUGH=1` を指定します。**既定では無効で、ほぼ確実にそのままがよい**です。
Skyrim SE で実測したところ、ゲームが自分の書庫やプラグインを開けなくなり、
MOD が黙って読み込まれません。このスイッチは仕組みを再検証するためにあり、
推奨されているからではありません。

詳細と、その判断の裏にある実測値は
[troubleshooting.ja.md](troubleshooting.md) に。

## すでに何かおかしい?

[troubleshooting.ja.md](troubleshooting.md) に環境スイッチ、操作カウンタの
読み方、これまで誰かを噛んだ問題のすべてがあります。
