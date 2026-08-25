<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Eidos のインストール

入口は三つ。どれも同じ二つの実行ファイル - `eidos`(コマンドライン)と
`eidos-gui` - に加えて、Nexus の「Mod Manager Download」ボタンをあなたの
インスタンスへ着地させる `nxm://` ハンドラを用意します。

## 先に必要なもの

| | |
|---|---|
| **FUSE のある Linux** | PATH に `fusermount3`。現行のディストリビューションはどれも同梱しています。 |
| **一度起動した Proton のゲーム** | Steam はゲームの Wine プレフィックスを初回起動時にしか作らず、Eidos はその中で動きます。 |
| **`7z`** | MOD 書庫の展開に使います。多くのディストリビューションでは `p7zip`。 |

root も、デーモンも、`/etc/fuse.conf` の編集も、グループへの追加も要りません。
Eidos はゲームのプロセスに属するプライベートな名前空間の中でマウントします。

## Arch

```bash
cd packaging && makepkg -si
```

## リリースの書庫

```bash
./install.sh
```

既定では `~/.local/bin` に入ります。`--system` なら `/usr/local/bin`、
`--bindir DIR` なら任意の場所へ。再実行が想定された更新手順です。

## ソースから

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## そのあと: Steam をここへ向ける

Eidos はあなたのゲームの起動コマンド*として*動きます。だからこそゲームが
始まる前にマウントできます。Steam でゲームを右クリック -> プロパティ ->
起動オプション:

```
~/.local/bin/eidos-gui %command%
```

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
受け取ります - [usage.ja.md](usage.ja.md) を参照。詳しい案内もそちらに。

## 任意: FUSE パススルー

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` でカーネルの FUSE
パススルーが有効になります。**既定では無効で、ほぼ確実にそのままがよい**です。
Skyrim SE で実測したところ、ゲームが自分の書庫やプラグインを開けなくなり、
MOD が黙って読み込まれません。このスイッチは仕組みを再検証するためにあり、
推奨されているからではありません。

詳細と、その判断の裏にある実測値は
[troubleshooting.ja.md](troubleshooting.ja.md) に。

## すでに何かおかしい?

[troubleshooting.ja.md](troubleshooting.ja.md) に環境スイッチ、操作カウンタの
読み方、これまで誰かを噛んだ問題のすべてがあります。
