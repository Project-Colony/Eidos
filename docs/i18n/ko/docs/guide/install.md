<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Eidos 설치

설치 방법은 네 가지입니다. Colony는 프로그램 전체인 파일 하나, `eidos-linux`를
설치합니다. 릴리스 번들, Arch 패키지, 소스 빌드는 `eidos`(명령줄), `eidos-gui`,
정적 NIF 미리 보기 도우미 `eidos-nif-preview`를 나란히 설치합니다. Arch 패키지와
소스 빌드는 더 낮은 수준의 CLI 둘인 `eidos-fuse`와 `eidos-launch`도 설치합니다.
어느 방법이든 Nexus의 "Mod Manager Download" 버튼으로 받은 다운로드를 인스턴스에
전달하는 `nxm://` 처리기를 등록할 수 있습니다.

## 먼저 필요한 것

| 요구 사항 | 이유 |
|---|---|
| FUSE가 있는 Linux | PATH에 `fusermount3`. 요즘 배포판이면 모두 들어 있습니다. |
| 한 번 실행해 본 Proton 게임 | Steam은 첫 실행 때에만 게임의 Wine 프리픽스를 만들고, Eidos는 그 안에서 동작합니다. Eidos는 Skyrim Special Edition의 GOG와 Epic 사본, Fallout 3와 Fallout: New Vegas의 Epic 사본, Stardew Valley의 GOG 사본도 감지합니다. 이들은 Heroic이나 Legendary를 통해 실행되며 Eidos가 그곳에서 게임의 래퍼가 됩니다. [usage.md](usage.md#게임-실행하기)를 참고하세요. |
| 7-Zip 바이너리: `7z`, `7zz` 또는 `7za` | 모드 압축 파일 설치용. Arch에서는 `7zip`, 대부분의 다른 배포판에서는 `p7zip`. |
| `par2` (선택) | `eidos pack` 백업용 PAR2 복구 데이터. Arch와 Fedora에서는 `par2cmdline`, Debian과 Ubuntu에서는 `par2`. |

root도, 데몬도, `/etc/fuse.conf` 수정도, 그룹 추가도 필요 없습니다. Eidos는
게임 프로세스에 속한 비공개 네임스페이스 안에서 마운트합니다.

일부 시스템 설정은 크래시로만 드러납니다. Diagnostics 탭은 `vm.max_map_count`(1048576
이상), 열린 파일 수의 하드 제한(524288 이상), 그리고 게임, 그 prefix, 인스턴스가
NTFS, exFAT, FAT 위에 있는지를 확인합니다. 각각의 해결 방법은
[troubleshooting.md](troubleshooting.md#시스템)에 있습니다.

## Colony를 통해

[Colony](https://github.com/Project-Colony/Colony)에서 Eidos를 검색해
설치하세요. 업데이트는 런처를 통해 들어옵니다. Colony는 파일 하나,
`~/.local/share/Colony/apps/Eidos/eidos-linux`를 둡니다. 실행하면 창이 열립니다.
`eidos`라는 이름의 링크를 통해 호출하면 같은 파일이 명령줄이 됩니다. `PATH`에
링크 하나를 두고 Nexus 다운로드 처리기를 등록하세요:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # 이제 사이트의 Mod Manager Download 버튼이 Eidos로 연결됩니다
```

창에는 링크가 필요 없습니다. 게임이나 도구를 실행할 때마다 창은 자기 파일을
명령줄로 실행합니다. NIF 모델 미리 보기에는 `eidos-nif-preview`가 필요한데,
이것은 릴리스 번들, Arch 패키지, 소스 빌드에만 들어 있습니다. 없으면 미리 보기가
그렇다고 알려 주고, 다른 것에는 아무 영향이 없습니다. `eidos-linux`에는 번들이
싣고 있는 Microsoft 런타임도 하나도 들어 있지 않습니다
([아래](#번들에-든-microsoft-런타임)). 도구에 런타임이 필요하면 전제 조건 설치를 시작할
때 Eidos가 Microsoft에서 내려받습니다. Steam은 아래처럼 그 파일 자체를 가리키게
하세요.

## 릴리스 번들

[최신 릴리스](https://github.com/Project-Colony/Eidos/releases/latest)에서
`eidos-linux-bundle.tar.gz`와 그 `.sig`, `.meta`, `.meta.sig`를 내려받고,
README의 [코드 서명 정책](../../README.md#코드-서명-정책)에 나온 대로 확인하세요.
번들의 최상위에는 `eidos`, `eidos-gui`, `eidos-nif-preview`가 있고, 그 옆에
`install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, `assets/brand/png`의 아이콘,
`licenses/`의 서드파티 고지(nifly, omod, microsoft-directx, microsoft-runtimes,
Rust 크레이트), 그리고 아래의 `runtimes/` 폴더가 들어 있습니다. `eidos-fuse`와
`eidos-launch`는 들어 있지 않습니다. 그다음:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh`는 바이너리를 `~/.local/bin`에 복사하며, `$XDG_BIN_HOME`이 설정되어
있으면 그곳에 복사합니다. `--system`은 sudo로 `/usr/local/bin`에, `--bindir DIR`은
원하는 곳에 넣고, `--from DIR`은 다른 디렉터리에서 바이너리를 가져옵니다. 이
스크립트는 다음도 합니다:

- `eidos nxm --register`를 실행해 `nxm://` 처리기를 등록하므로, Colony 절의 등록
  단계는 필요 없습니다.
- `~/.local/share/applications/eidos.desktop`과 `~/.local/share/icons/hicolor` 아래의
  Eidos 아이콘을 씁니다.
- 라이선스 고지를 `~/.local/share/licenses/eidos`에 복사합니다(`--system`이면
  `/usr/local/share/licenses/eidos`).
- `runtimes/`를 `~/.local/share/Colony/Eidos/runtimes`에 복사하며, `--system`일 때도
  마찬가지입니다. 그래서 런타임은 스크립트를 실행한 사용자의 것이 됩니다.
- 마지막에 설치한 `eidos-gui`의 절대 경로가 들어간 Steam 실행 옵션을 출력합니다.

`--cap`은 sudo에 설치된 `eidos`에 `CAP_SYS_ADMIN`을 부여해 달라고 요청합니다(아래의
FUSE 패스스루 참고). `--no-cap`이 기본값이며 root 없이 둡니다. 다시 실행하는 것이
정식 업그레이드 방법입니다. 번들과 `eidos-linux`에는 glibc 2.39 이상의 x86_64
Linux가 필요합니다.

### 번들에 든 Microsoft 런타임

번들의 `runtimes/` 폴더에는 도구가 요구하는 Microsoft 파일이 들어 있습니다.
`VC_redist.x86.exe`와 `VC_redist.x64.exe`(Visual C++ 2015-2022, `vcrun2022` 전제 조건),
`windowsdesktop-runtime-8.0.12-win-x64.exe`(.NET 8 Desktop Runtime, `dotnet8`과
`dotnetdesktop8`용), `dotnet-runtime-10.0.10-win-x64.zip`(.NET 10 런타임, `dotnet10`용)
입니다. Eidos는 Visual C++와 .NET 8 설치 프로그램을 winetricks 없이 게임의 prefix
안에서 직접 실행하고, .NET 10 zip은 한 번 `~/.local/share/Colony/Eidos/runtimes`에
풀어 모든 인스턴스가 공유합니다(풀면 78 MB). 각 파일은 SHA-256이 Eidos에 고정된
값과 일치할 때만 쓰입니다. Synthesis에 필요한 .NET 10 SDK(`dotnetsdk10`, 약 300 MB)는
번들에 없고 항상 내려받습니다.

풀어 둔 번들은 그 자리에서도 실행됩니다. Eidos는 위의 데이터 폴더 다음으로 자기
바이너리 옆의 `runtimes/`를 찾습니다.

## Arch

```bash
cd packaging && makepkg -si
```

PKGBUILD는 현재 `main`을 소스에서 빌드합니다. `eidos`, `eidos-gui`,
`eidos-nif-preview`, `eidos-fuse`, `eidos-launch`를 `/usr/bin`에 설치하고, 시스템 전역
`nxm://` 처리기, 데스크톱 항목, 아이콘도 함께 둡니다. `fuse3`, `7zip`, `xdg-utils`에
의존합니다. `runtimes/` 폴더는 싣지 않으므로 도구가 필요로 할 때 Microsoft 런타임을
내려받습니다. `winetricks`는 선택 사항입니다. Eidos가 직접 설치하지 않는 전제
조건, 예를 들어 `dotnet48`과 `xact`에는 여전히 winetricks를 씁니다.

## 소스에서 빌드

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

소스 빌드에는 Rust 1.92 이상, `just`, CMake 3.20 이상, C++17 컴파일러, 네이티브 도우미 테스트용 Python 3이 필요합니다. `just build`는 워크스페이스 전체를 빌드하므로 `eidos-fuse`와 `eidos-launch`도 함께 빌드되고, `just install`은 빌드 결과에 `packaging/install.sh`를 실행해 나머지와 함께 복사합니다. `just` 없이 실행할 명령은 [네이티브 도우미 안내](../../../../../crates/eidos-nif-preview/README.md)를 참고하세요. `cargo build --release --locked`로 Rust를 빌드하고 도우미를 `target/release`에 복사한 다음 `packaging/install.sh --from target/release`를 실행하세요. 소스 빌드에도 `runtimes/` 폴더가 없으므로 도구가 필요로 할 때 Microsoft 런타임을 내려받습니다.

릴리스 프로필은 코드 생성 단위 하나, fat LTO, 심볼 제거를 씁니다. rustc 1.99.1과
빌드 작업 3개로 `eidos`와 `eidos-gui`를 처음부터 빌드하니 334초가 걸렸고(thin LTO는
243초), `eidos`는 20.9 MB, `eidos-gui`는 49.7 MB였습니다. 각 바이너리는 DirectX DLL
여덟 개를 gzip으로 압축해 내장하며, 17.9 MB 대신 7.9 MB입니다.
`just profile=dev build`는 최적화 없이 빌드합니다.

## 그다음: Steam이 이것을 가리키게 하기

Eidos는 게임의 실행 명령 *그 자체로* 동작합니다. 그래서 게임이 시작되기 전에
마운트할 수 있습니다. Steam에서 게임을 오른쪽 클릭 -> 속성 -> 실행 옵션을 열고,
Eidos 창의 절대 경로 뒤에 `%command%`를 입력하세요:

```
~/.local/bin/eidos-gui %command%
```

절대 경로를 쓰세요 - Steam은 셸의 `PATH`를 읽지 않습니다. 경로는 Eidos를 어떻게
설치했는지에 따라 달라지며, `install.sh`는 자신이 쓴 디렉터리에 맞는 줄을
출력합니다:

| 설치 방법 | 실행 옵션 |
|---|---|
| 릴리스 번들 또는 `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Arch 패키지 | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

플레이를 누르세요. Eidos가 그 게임의 인스턴스로 열립니다. 모드를 설치하고,
LOOT로 정렬하고, Run을 누르세요. 종료하면 마운트도 함께 사라지고, 설치 폴더는
정확히 이전 그대로입니다.

게임은 항상 스토어를 통해 시작됩니다. 직접 연 창에서 Run은 `steam -applaunch <appid>`를
실행하고, Steam이 실행 옵션을 통해 Eidos를 시작하며, 두 번째 Eidos 창이 열려 거기서
Run을 누르게 됩니다. 실행 옵션이 Eidos를 시작하기 전까지 Run은 아무것도 시작하지
않고, 상태 표시줄이 설정할 줄을 보여줍니다. Start the game at once when a store opens
Eidos(Preferences, General, Running a game, 기본값은 꺼짐)를 켜면 Health에 문제가
없을 때 Steam에서 Play를 눌러도 창 없이 게임이 시작됩니다. Heroic과 Legendary도
래퍼 설정을 통해 같은 방식으로 동작합니다. 자세한 내용은
[usage.md](usage.md#게임-실행하기)에 있습니다.

Diagnostics 탭이 그 줄을 확인합니다. 실행 옵션이 비어 있거나, Eidos 바이너리를
가리키지 않거나, 없는 파일을 가리키거나, Eidos 뒤에 `%command%`가 없거나, 지금
실행 중인 것과 다른 Eidos 사본을 시작하면 Health 카드가 설정할 줄을 보여줍니다.

### 터미널이 더 편하다면

```sh
eidos init skyrimse               # 인스턴스 만들기 (폴더를 주면 휴대용이 됩니다)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root 모드
eidos sort skyrimse               # LOOT로 로드 순서 정렬
eidos check skyrimse              # 창이 필요 없는 Health 점검
eidos play skyrimse -- %command%  # 무엇이든 병합된 뷰를 통해 실행
eidos shell skyrimse              # 병합된 뷰 안의 셸, 쓴 내용은 Overwrite로 들어갑니다
```

게임 id를 받는 모든 명령은 휴대용 인스턴스의 폴더도 받습니다 -
[usage.md](usage.md#인스턴스-전역과-휴대용) 참고. 전체 안내는
[usage.md](usage.md)에 있습니다.

## 업데이트

Colony는 런처를 통해 `eidos-linux`를 업데이트합니다. 번들은 새 번들의 `install.sh`를
실행하고, Arch 패키지는 `makepkg -si`를 다시 실행하고, 소스 빌드는 pull한 뒤
`just install`을 실행합니다. 창은 시작할 때 GitHub에 최신 릴리스를 물어보며 하루에
한 번을 넘지 않고, 더 새로운 릴리스가 있으면 메뉴 모음의 Help 뒤에 릴리스 노트로
가는 링크를 보여줍니다. 스스로 내려받거나 교체하지는 않습니다. Preferences, About,
Version에서 이 확인을 끌 수 있고, 오프라인 모드에서는 건너뛰며, `/usr` 아래에 설치된
사본(Arch 패키지)은 패키지 관리자가 업데이트하므로 아예 묻지 않습니다.

## 선택: FUSE 패스스루

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"`는 선택적 권한을 부여하지만
패스스루를 켜지는 않습니다. `./install.sh --cap`과 `just setcap`은 각자 설치하거나
빌드한 `eidos`에 이 권한을 줍니다. `nosuid` 마운트에서는 커널이 파일 권한을
무시하므로 `install.sh`는 경고와 함께 부여를 건너뛰고 `just setcap`은 오류로
멈춥니다. 이 권한은 번들, Arch 패키지, 소스 빌드의 별도 `eidos` 바이너리에 주어야
하며, 창이기도 한 `eidos-linux`에는 절대 주지 마세요
([SECURITY.md](../../../../../SECURITY.md)). 다시 빌드하거나 다시 설치한 바이너리는
권한이 없는 새 파일이므로, 쓴다면 업그레이드할 때마다 다시 부여하세요.

`EIDOS_FUSE_PASSTHROUGH=1`은 별도의 실행 시 스위치이며 기본값은 꺼짐입니다.
패스스루를 켜면 예전 버전의 데몬은 Skyrim SE가 모드 내용을 전혀 불러오지 못하게
했습니다. 가장 유력한 원인은 그 버전이 열기마다 새 백킹 파일을 등록했는데 커널은
inode마다 하나만 받아들인다는 점입니다. 데몬은 이제 한 파일의 모든 패스스루 열기가
하나의 백킹을 공유하게 하지만, 이는 아직 게임에서 다시 측정되지 않았습니다. 그때까지는
시험할 때만 패스스루를 켜세요. 측정값과 원인은
[troubleshooting.md](troubleshooting.md)에 있습니다.

## 벌써 뭔가 잘못됐나요?

[troubleshooting.md](troubleshooting.md)가 환경 스위치, 연산 카운터 읽는
법, Health 카드, 그리고 지금까지 누군가를 물었던 모든 문제를 다룹니다.
