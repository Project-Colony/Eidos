<!-- eidos-i18n: source=docs/guide/install.md sha=bc05db1de00bb7c80224f065c3f730f3adc542dc -->

# Eidos 설치

설치 방법은 네 가지입니다. Colony는 프로그램 전체인 파일 하나, `eidos-linux`를
설치합니다. 나머지 방법은 `eidos`(명령줄), `eidos-gui`, 정적 NIF 미리 보기
도우미를 나란히 설치하고, Nexus의 "Mod Manager Download" 버튼으로 받은
다운로드를 인스턴스에 전달하는 `nxm://` 처리기도 함께 제공합니다.

## 먼저 필요한 것

| | |
|---|---|
| **FUSE가 있는 Linux** | PATH에 `fusermount3`. 요즘 배포판이면 모두 들어 있습니다. |
| **한 번 실행해 본 Proton 게임** | Steam은 첫 실행 때에만 게임의 Wine 프리픽스를 만들고, Eidos는 그 안에서 동작합니다. |
| **`7z`** | 모드 압축 파일 설치용. 대부분의 배포판에서는 `p7zip`. |
| **`par2`** (선택) | `eidos pack` 백업용 PAR2 복구 데이터. Arch와 Fedora에서는 `par2cmdline`, Debian과 Ubuntu에서는 `par2`. |

root도, 데몬도, `/etc/fuse.conf` 수정도, 그룹 추가도 필요 없습니다. Eidos는
게임 프로세스에 속한 비공개 네임스페이스 안에서 마운트합니다.

## Colony를 통해

[Colony](https://github.com/Project-Colony/Colony)에서 **Eidos**를 검색해
설치하세요. 업데이트는 런처를 통해 들어옵니다. Colony는 파일 하나,
`~/.local/share/Colony/apps/Eidos/eidos-linux`를 둡니다. 실행하면 창이 열립니다.
`eidos`라는 이름의 링크를 통해 호출하면 같은 파일이 명령줄이 됩니다. `PATH`에
링크 하나를 두고 Nexus 다운로드 처리기를 등록하세요:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # 이제 사이트의 Mod Manager Download 버튼이 Eidos로 연결됩니다
```

창에는 링크가 필요 없습니다: 게임이나 도구를 실행할 때마다 창은 자기 파일을
명령줄로 실행합니다. NIF 모델 미리 보기에는 `eidos-nif-preview`가 필요한데,
이것은 릴리스 번들과 소스 빌드에만 들어 있습니다. 없으면 미리 보기가 그렇다고
알려 주고, 다른 것에는 아무 영향이 없습니다. Steam은 아래처럼 그 파일 자체를
가리키게 하세요.

## 릴리스 번들

[최신 릴리스](https://github.com/Project-Colony/Eidos/releases/latest)에서
`eidos-linux-bundle.tar.gz`와 그 `.sig`, `.meta`, `.meta.sig`를 내려받고,
README의 [코드 서명 정책](../../README.md#코드-서명-정책)에 나온 대로 확인하세요.
그다음:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

기본값은 `~/.local/bin`입니다. `--system`은 `/usr/local/bin`에, `--bindir DIR`은
원하는 곳에 넣습니다. 다시 실행하는 것이 정식 업그레이드 방법입니다.
번들과 `eidos-linux`에는 glibc 2.39 이상의 x86_64 Linux가 필요합니다.

## Arch

```bash
cd packaging && makepkg -si
```

PKGBUILD는 현재 `main`을 소스에서 빌드합니다.

## 소스에서 빌드

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

소스 빌드에는 Rust 1.89 이상, `just`, CMake 3.20 이상, C++17 컴파일러, 네이티브 도우미 테스트용 Python 3이 필요합니다. `just` 없이 실행할 명령은 [네이티브 도우미 안내](../../../../../crates/eidos-nif-preview/README.md)를 참고하세요. `cargo build --release --locked`로 Rust를 빌드하고 도우미를 `target/release`에 복사한 다음 `packaging/install.sh --from target/release`를 실행하세요.

## 그다음: Steam이 이것을 가리키게 하기

Eidos는 게임의 실행 명령 *그 자체로* 동작합니다. 그래서 게임이 시작되기 전에
마운트할 수 있습니다. Steam에서 게임을 오른쪽 클릭 -> 속성 -> 실행 옵션:

```
~/.local/bin/eidos-gui %command%
```

Colony로 설치했다면 경로는 `~/.local/share/Colony/apps/Eidos/eidos-linux`입니다.
플레이를 누르세요. Eidos가 그 게임의 인스턴스로 열립니다. 모드를 설치하고,
LOOT로 정렬하고, Run을 누르십시오. 종료하면 마운트도 함께 사라지고, 설치 폴더는
정확히 이전 그대로입니다.

절대 경로를 쓰세요 - Steam은 셸의 `PATH`를 읽지 않습니다.

### 터미널이 더 편하다면

```sh
eidos init skyrimse               # 인스턴스 만들기 (폴더를 주면 휴대용이 됩니다)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root 모드
eidos sort skyrimse               # LOOT로 로드 순서 정렬
eidos play skyrimse -- %command%  # 무엇이든 병합된 뷰를 통해 실행
```

게임 id를 받는 모든 명령은 휴대용 인스턴스의 폴더도 받습니다 -
[usage.md](usage.md#인스턴스-전역과-휴대용) 참고. 전체 안내는
[usage.md](usage.md)에 있습니다.

## 선택: FUSE 패스스루

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"`는 선택적 권한을 부여하지만 패스스루를 켜지는 않습니다.
이 권한은 번들이나 소스 빌드의 별도 `eidos` 바이너리에 주어야 하며, 창이기도 한
`eidos-linux`에는 절대 주지 마세요([SECURITY.md](../../../../../SECURITY.md)).
실행 시 별도의 `EIDOS_FUSE_PASSTHROUGH=1` 설정으로 활성화합니다. **기본값은 꺼짐이고, 거의 확실히 그대로 두는 편이 좋습니다**: Skyrim
SE에서 측정한 결과, 게임이 자기 아카이브와 플러그인을 열지 못하게 되어 모드가
조용히 로드되지 않습니다. 이 스위치는 메커니즘을 다시 시험하기 위한 것이지
권장되기 때문이 아닙니다.

자세한 내용과 그 결정을 뒷받침한 측정값은
[troubleshooting.md](troubleshooting.md)에 있습니다.

## 벌써 뭔가 잘못됐나요?

[troubleshooting.md](troubleshooting.md)가 환경 스위치, 연산 카운터 읽는
법, 그리고 지금까지 누군가를 물었던 모든 문제를 다룹니다.
