<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=af0c14c5192ccae592dd1c1661fd02d5242ecd5a -->

# 문제 해결과 진단

환경 스위치, 연산 카운터를 읽는 법, Diagnostics 탭의 Health 카드, 크래시 뒤에 있는
모드를 찾는 법, 버그 보고서에 첨부할 것, 그리고 알려진 문제와 그 내력입니다.

## VFS 진단하기

게임이 파일 시스템과 말이 맞지 않는 무언가를 보는 경우 대부분은 환경 변수 둘로
다룰 수 있습니다:

```sh
EIDOS_FUSE_STATS=1                  # 연산 카운터, 언마운트 때 덤프
EIDOS_FUSE_NO_CACHE=1               # 커널 쪽 캐시를 전부 끄기
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # 또는 하나씩 이름을 대기
```

아래 알려진 문제의 `FOPEN_KEEP_CACHE` 크래시를 찾아낸 것이 이 세분화된 형태입니다.
넷을 모두 끄면 "캐싱 때문인가?"에 답하고, 이름을 대야만 "어느 것인가"에 답합니다.
카운터는 나머지 절반에 답합니다. 패스스루를 켠 상태(기본값은 꺼짐)에서 `read 0`을
보여주는 로드는 `FUSE_PASSTHROUGH`가 모든 바이트를 커널 안에서 제공한 로드이므로,
읽기 경로에서 손보려던 것은 이미 공짜입니다.

스위치가 몇 개 더 있어 결함을 더 좁혀 줍니다:

```sh
EIDOS_FUSE_TRACE=open,opendir       # open / opendir마다 기록 (1 = 모든 채널)
EIDOS_FUSE_THREADS=1                # 데몬 스레드 하나 (기본값: CPU 수, 최대 4)
EIDOS_FUSE_OPENDIR=1                # 커널이 건너뛸 수 있을 때도 opendir 핸들 유지
EIDOS_NO_INDEX=1                    # 레이어 인덱스 대신 레이어를 훑기
EIDOS_NO_OW_INDEX=1                 # Overwrite의 소속 인덱스에 대해서도 같음
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE 다시 켜기, 알려진 문제 참고
```

Steam으로 실행할 때는 실행 옵션의 명령 앞에 붙이세요. 예를 들면
`EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`입니다. 출력은 그 실행의
로그로 갑니다. 창에서 실행했다면 인스턴스의 `logs/run-*.log`, Steam에서 실행했다면
Steam 콘솔입니다.

병합된 뷰 자체를 보려면 `eidos shell <instance>`가 게임이 보는 그대로의 게임 폴더에서
셸을 열고, `eidos shell <instance> -- ls Data/meshes`는 거기서 명령 하나를 실행합니다.
쓴 내용은 Overwrite로 들어갑니다([usage.md](usage.md#병합된-뷰의-셸)). Data
탭의 Save tree 버튼은 병합된 `Data` 트리 전체를, 한 줄에 파일 하나와 그 파일이 오는
모드를 적어 텍스트 파일로 쓰며, 홈 폴더는 가려집니다.

## 유니온을 손으로 마운트하기

`eidos-fuse`는 Arch 패키지와 소스 빌드에 들어 있습니다. `just build`가 빌드하고,
있으면 `just install`이나 `install.sh`가 복사합니다. 릴리스 번들과 `eidos-linux`에는
들어 있지 않습니다. 사용법 줄은 이렇습니다:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

충돌하면 첫 번째 `--layer`가 이깁니다. 마지막은 손대지 않은 게임 데이터입니다.
마운트에 필요한 것은 `/dev/fuse`와 `fusermount3`뿐입니다(overlayfs도, Wine도
필요 없습니다):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... /mnt/point를 통해 읽고 쓰기 ...
fusermount3 -u /mnt/point
```

쓰기는 `--overwrite <dir>`에 떨어지므로(생략하면 임시 디렉터리), 여기서도 레이어
자체는 손대지 않은 채로 남습니다.

## 패스스루가 기본으로 꺼져 있는 이유

패스스루는 커널에 실제 백킹 파일을 넘겨주어 읽기가 이 데몬을 통째로 건너뛰게
합니다. 아래의 A/B는 예전 버전의 데몬으로, 뒤에 나오는 알려진 문제 "FUSE
패스스루가 게임이 모드 콘텐츠를 하나도 로드하지 못하게 함"에 설명된 변경 이전에
측정한 것입니다. 그 변경은 가장 유력한 원인을 피하지만 아직 게임에서 다시
측정되지 않았으므로, 패스스루는 다시 측정될 때까지 기본으로 꺼진 채로 남습니다.
Skyrim SE 1.6.1170, proton-cachyos 11.0, 커널 7.1.4, 동일한 82개 플러그인 로드
순서에서, 실행 파일이 그 능력을 지녔는지만을 변수로 두고 A/B로 측정한
결과입니다:

| 패스스루 | `STATUS_ACCESS_VIOLATION`으로 끝난 `NtCreateFile` 실패 |
|-------------|--------------------------------------------------------|
| 켬          | 152 - `.bsa` 75, `.esl` 65, `.esm` 10, `.esp` 2        |
| 끔          | 0                                                      |

켜 두었을 때 게임은 자기 아카이브도 플러그인도 하나 열지 못했고, 이는 게임
안에서 존재하지 않는 모드로 드러났습니다 - 오류도, 로그 한 줄도 없이. 꺼 두었을
때 같은 로드 순서가 플러그인과 아카이브와 Papyrus 스크립트를 살린 채 플레이에
도달했습니다.

이 실패는 데몬 안에서는 보이지 않았습니다. 데몬 자신의 `open`은 매번 성공했고
커널은 백킹 파일을 결코 거부하지 않았습니다(실패한 세션 전체를
`EIDOS_FUSE_TRACE=open`으로 확인: `open FAILED` 0건, `passthrough refused` 0건).
오류는 데몬이 `opened_passthrough`라고 답한 뒤에 만들어졌으므로, 데몬 쪽의 어떤
로깅도 그것을 볼 수 없었습니다. 확장자를 가리지도 않았습니다 - 아카이브와
플러그인을 똑같이 덮쳤으며, 곧 게임이 실행 내내 열어 두는 파일들입니다.

`EIDOS_FUSE_PASSTHROUGH=1`이 그것을 다시 켭니다. 무엇을 벌어들이는지 측정하거나
메커니즘을 다시 시험하기 위한 것입니다. 런처와 Diagnostics 탭의 능력 경고는 그것을
요청했을 때에만 나타납니다.

## 레이어 인덱스가 실제로 쓰이고 있는가?

`LayerStack::new`는 읽기 전용 레이어의 완전한 맵을 만들거나 아예 만들지
않습니다. 맵이 없으면 모든 조회는 예전처럼 레이어를 훑습니다. 답은 같고, 세이브
로드에 약 3배의 시간이 걸리며, 그 밖에 달라 보이는 것은 없습니다.

인덱스가 꺼져 있으면 마운트가 시작할 때 한 번 이유를 알려 줍니다:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

이유에는 읽을 수 없는 디렉터리, 64단계보다 깊은 레이어, 4,000,000개를 넘는 항목,
또는 `EIDOS_NO_INDEX`가 적힙니다. 이 줄은 실행 로그로 갑니다. 창에서 실행했다면
인스턴스의 `logs/run-*.log`, Steam에서 실행했다면 Steam 콘솔입니다.
`EIDOS_FUSE_STATS=1`을 주면 언마운트 때의 출력에도 `layer index:` 줄이 있고, 일반
마운트에서 `unindexed`가 0이 아니면 인덱스가 꺼져 있었다는 뜻입니다.

아래 예제는 같은 것을 오프라인으로, 인스턴스의 폴더를 대상으로, 마운트 없이
확인합니다:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health`는 인덱스가 있을 때와 없을 때 실제 경로를 해석하고 디렉터리 스캔을
비교합니다. `index_agrees`는 실제 인스턴스의 모든 경로와 모든 목록에서 둘이 똑같은
답을 내는지 확인합니다. `listing_cost`는 병합된 자식 맵이 `readdir`에서 무엇을
아껴 주는지 측정합니다.

`EIDOS_NO_INDEX=1`은 훑기를 강제합니다. 두 답의 차이 자체가 디버깅 대상일 때를 위한
것입니다.

## CLI로 게임 시작하기

창이 아니라 `eidos` CLI로 게임을 실행하려면 Steam 실행 옵션을 `eidos`의 절대 경로로
설정하세요. 예를 들면:

```
~/.local/bin/eidos play skyrimse -- %command%
```

`WINEDLLOVERRIDES` 접두어는 필요 없습니다. 활성화된 모드가 `d3dcompiler_47`을
가져다 쓰면, Eidos는 번들된 사본을 프리픽스에 배치하고 네이티브로 강제한 뒤,
모드가 함께 담아 오는 DLL 오버라이드(ENB/ReShade/`.asi` 로더)와 병합합니다.

Eidos는 게임을 Proton으로 직접 시작하지 않습니다. 게임을 시작하는 항목(스크립트
익스텐더의 로더, 런처, 게임 바이너리)에 대한 `eidos tool <instance> run <title>`은,
스토어가 열지 않은 창에서 Run이 하듯이, 그 사본을 가진 스토어에 시작을 요청하고,
스토어의 설정이 Eidos를 시작하지 않을 때는 무엇을 설정해야 하는지 출력합니다.
`--print`는 명령을 실행하는 대신 스토어 명령을 출력합니다.

## 닫히지 않는 게임이나 도구

게임이나 도구가 실행되는 동안 Run 버튼은 Stop으로 바뀌고, 잠금 카드에도 Stop 버튼이
있습니다. 두 번 누르세요. 두 번째로 누르면 그 실행이 시작한 모든 프로그램을 끝냅니다.
먼저 SIGTERM을 보내고, 10초 뒤에 남은 것에 SIGKILL을 보냅니다. 게임이 저장하지 않은
내용은 사라집니다. 그다음 Eidos는 여느 종료 뒤와 같은 일을 합니다. INI와 플러그인
변경은 프로필로 돌아가고, 도구 출력은 기록되며, 모드는 마운트 해제됩니다. 실행은
프로그램이 무엇을 반환했든 종료 코드 143으로 기록되므로, `eidos sessions <instance>
diff`는 그것을 마지막 정상 실행으로 보지 않습니다. Unlock은 다릅니다. 창을 다시 쓸 수
있게 돌려주고 프로그램은 계속 실행되게 둡니다. 터미널에서 `eidos` 프로세스에
`kill <pid>`를 보내도(pid는 잠금 카드에 있습니다) Stop과 같습니다.

## 무엇이 깨뜨렸는지 찾기

어느 변경이 게임을 깨뜨렸는지 찾으려면:

- `eidos sessions <instance> diff`는 종료 코드 0으로 끝난 마지막 게임 실행 이후 바뀐
  것을 나열합니다. 모드, 버전, 플러그인 순서, INI, 게임 실행 파일입니다
  ([usage.md](usage.md#마지막-정상-실행-이후-바뀐-내용)).
- Run 옆의 Test run은 프로필의 모드로, 모드 없이, 또는 선택한 모드와 그것이 필요로 하는
  마스터만으로 플레이하고, 게임이 쓰는 모든 것을 `<instance>/test-run/`으로 보냅니다.
  실행 뒤에 세이브, INI 변경, 플러그인 상태, Overwrite 출력, 로그의 각 묶음을 남기거나
  버립니다([usage.md](usage.md#테스트-실행)).
- Diagnostics 탭의 Find the mod behind a problem은 남은 용의자의 절반으로 매번 테스트
  실행을 돌리고 문제가 생겼는지 묻습니다. 용의자 200개에 약 8번의 실행이 들고, 검색은
  Eidos를 닫아도 이어집니다([usage.md](usage.md#문제를-일으키는-모드-찾기)).

중단된 실행은 종료 코드 143으로 기록되므로(위 참고), 이 검색에서 Yes가 미리
채워지지 않습니다.

## Health 카드

Diagnostics 탭은 찾아낸 문제마다 Health 카드를 보여주며, 원인과, 일부에는 해결하는
버튼이 있습니다. 실행 옵션 카드는 [install.md](install.md)에, NVAPI, 전체 화면, ENB
카드는 [graphics.md](graphics.md)에, Proton과 프리픽스 카드는 다음 절에 설명되어
있습니다. `eidos check <instance>`는 창이 필요 없는 결과를 출력하며
([usage.md](usage.md#터미널에서-상태-검사)에 목록이 있습니다), 프리픽스,
레지스트리, `S:` 드라이브 카드도 거기에 들어 있습니다. Flatpak, 알 수 없는 빌드, 더
오래된 빌드 변형이 있는 Proton 카드, 패치 순서, 익스텐더 스크립트, 어떤 플러그인도
로드하지 않는 아카이브, Nexus 요구 사항, 오래된 플러그인 사본, Display 카드는 창만
보여줍니다.

### 시스템

- "vm.max_map_count is <value>"는 1048576 미만일 때 나타납니다. 이는 Arch Linux,
  Fedora 39 이후, Ubuntu 24.04 이후의 기본값입니다. Proton의 일부 게임은 더 많은
  메모리 영역을 매핑하다가 오류 없이 크래시합니다. `/etc/sysctl.d` 아래의 파일, 예를
  들어 `80-gamecompatibility.conf`에 `vm.max_map_count = 1048576`을 넣고
  `sudo sysctl --system`을 실행하세요.
- "The open files limit is <value>"는 하드 제한이 524288 미만일 때 나타납니다. Proton의
  esync에 필요한 값이며, 그보다 낮으면 게임이 시작되지 않거나 크래시할 수 있습니다.
  systemd 240 이후는 이 값을 설정합니다. 더 오래된 환경에서는
  `/etc/systemd/system.conf`와 `/etc/systemd/user.conf`에 `DefaultLimitNOFILE=524288`을
  설정하고 다시 로그인하세요.
- "Game files on a Windows file system"은 게임 폴더, 프리픽스, 인스턴스 루트가
  `ntfs`, `ntfs3`, `fuseblk`(ntfs-3g, exfat-fuse), `exfat`, `vfat` 위에 있을 때 그것을
  알려 줍니다. Proton에는 심링크, Unix 권한, 대소문자를 구분하는 이름이 필요합니다.
  게임, 그 프리픽스, 인스턴스를 ext4, btrfs, xfs 같은 Linux 파일 시스템으로 옮기세요.

### 게임 파일

탭 맨 위의 Game files 카드는 인스턴스가 스토어 설치본을 실행하는지, 인스턴스 안의 게임
사본을 실행하는지 보여줍니다([usage.md](usage.md#게임-파일-스토어-설치본-또는-사본)).
세 경우에는 이유를 알려 주는 카드와 함께 실행이 거부됩니다:

- "The game copy '<folder>' is missing": 인스턴스가 실행하는 사본이 사라졌습니다. 다시
  가져다 놓거나 스토어 버전으로 돌아가세요.
- "'<folder>' has no <binary>": 인스턴스가 실행하는 폴더에 게임 실행 파일이 없으므로
  게임의 사본이 아닙니다.
- "<game> cannot run from a copy yet": Morrowind와 `bUseMyGamesDirectory=0`인 Oblivion은
  플러그인 목록을 게임 폴더에 둡니다.

"ModOrganizer.ini points tools at another game folder"는 Eidos가 쓰지 않은 목록 자체의
`ModOrganizer.ini`에 인스턴스가 실행하는 게임 파일과 다른 `gamePath`가 있을 때
나타납니다. PGPatcher 같은 Mod Organizer 2 모드의 도구는 게임을 거기서 읽습니다. 그
파일의 `gamePath`를 바꾸거나, 파일을 지워 Eidos가 자기 것을 쓰게 하세요.

### 로드 순서

- "sTestFile lines make the game ignore plugins.txt"는 Fallout 4, Fallout 4 VR,
  Starfield에서 프로필의 `Fallout4Custom.ini`나 `StarfieldCustom.ini`의 `[General]`에
  `sTestFile1`부터 `sTestFile10`까지의 줄이 있을 때 나타납니다. 하나라도 설정되어
  있으면 게임은 그 플러그인을 로드하고 Eidos가 쓴 로드 순서는 효과가 없습니다. Fix는
  프로필의 사본에서 그 줄을 지우고 이전 파일을 `<ini>.pre-fix`로 남깁니다.
- Starfield: 마스터 플래그가 없는 활성 `.esp` 플러그인에는 카드가 붙습니다. Starfield는
  이들을 `.esm`과 마스터 플래그가 있는 플러그인 사이에 정렬하지 못하고, 그 레코드
  일부를 세션 내내 로드한 채로 두기 때문입니다. 제작자의 `.esm` 릴리스를 쓰면 둘 다
  피할 수 있습니다.
- Starfield: "Invalid blueprint plugins"는 블루프린트 플래그, `BlueprintShips-`로
  시작하는 이름, `.esm` 확장자 가운데 일부만 갖춘 플러그인을 나열합니다. "Blueprint
  plugins without their main plugin"은 메인 플러그인이 활성화되지 않은 것들을
  나열합니다. `BlueprintShips-Example.esm`에는 `Example.esm`, `.esp`, `.esl`이
  필요합니다.
- "<patch> loses <n> file(s) to <master>"는 한 모드의 플러그인이 다른 모드의 마스터를
  필요로 하는데 그 다른 모드의 우선순위가 더 높아, 둘 다 담고 있는 파일의 사본이 그쪽에서
  이길 때 나타납니다. 패치는 보통 패치 대상 모드 아래로 가며, 카드의 Move 버튼이 그렇게
  옮깁니다. 패치가 그 파일을 내주도록 만든 것이라면 순서를 그대로 두세요.
- "Out-of-date copy of <plugin>"은 모드가 정리했거나 라이트 플래그를 단 플러그인의
  사본을 갖고 있는데 원본이 그 뒤에 바뀌었을 때(모드나 게임 업데이트) 나타납니다.
  게임은 옛 사본을 로드합니다. 그것을 숨기고, 새 플러그인을 다시 정리하거나 플래그를
  다세요.

### 모드 목록

- "Order rules of the collection <name> that no longer hold"는 설치한 컬렉션의 모드가 더
  이상 그 규칙이 요구하는 순서에 놓여 있지 않을 때 나타납니다. Restore collection order는
  그 컬렉션의 모드만 되돌려 놓으며, Ctrl+Z로 되돌릴 수 있습니다.
- "<mod> requires <mod>, which is not installed"(또는 "which is disabled")는 Check for
  updates가 읽는 Nexus 요구 사항에서 옵니다. Open on Nexus는 필요한 모드의 페이지로
  가고, Download main file은 프리미엄 계정에서 그것을 내려받으며, Ignore는 다른 모드가
  이미 충족한 요구 사항을 숨깁니다. DLC 요구 사항은 조언일 뿐입니다. Eidos는 DLC 이름의
  플러그인을 찾습니다.
- "fontconfig.txt names a font library that is missing"(Skyrim 엔진)은 어떤 레이어에도
  아카이브에도 없는 폰트와, `Interface/fontconfig.txt`가 이기는 모드를 알려 줍니다.
  폰트를 설치하거나 그 줄을 지우기 전까지 메뉴 글자가 네모로 보일 수 있습니다.
- "BA2 archive stores audio compressed"(Fallout 4와 Fallout 4 VR)는 모드의 BA2가
  압축해서 담은 `.wav`와 `.xwm` 파일의 수를 셉니다. 게임이 그것을 재생할 때 크래시할 수
  있습니다. 오디오를 압축하지 않고 아카이브를 다시 패킹하세요.

### 스크립트 익스텐더와 게임 업데이트

- "<loader> is not installed"는 스크립트 익스텐더 플러그인이 설치되어 있는데(`SKSE/Plugins`,
  `F4SE/Plugins` 등) 로더가 켜 둔 어느 모드에도, 게임 폴더에도 없을 때 나타납니다.
  게임은 그 플러그인 없이 시작됩니다.
- "The script extender does not match game version <version>"은 로더는 설치되어 있는데
  실행 중인 게임 버전용 런타임 DLL이 없을 때 나타납니다. 예를 들면
  `skse64_1_6_1170.dll`입니다. 로더가 오류와 함께 멈춥니다. 보통 게임 업데이트 뒤에
  일어납니다. 이 카드와 위의 카드에는 익스텐더 사이트로 가는 Open the download page
  버튼과, 로더를 모드의 `Root/` 폴더에 넣는 Install the downloaded archive 버튼이
  있습니다.
- "<game> changed since the last clean launch"는 게임 실행 파일, 또는 Steam 사본이면 그
  빌드 id가 오류 없이 끝난 마지막 Eidos 실행 이후 바뀌었을 때 나타납니다. 실행 파일을
  그대로 둔 Steam 업데이트는 이 카드를 띄우지 않습니다. 이전 버전용으로 빌드된
  스크립트 익스텐더 플러그인이 로드되지 않을 수 있으며, 그 아래의 카드가 그것을 선언한
  플러그인을 알려 줍니다. 목록을 만든 게임 버전을 유지하려면 Game files 카드의 Freeze
  this version으로, 또는 `freeze`만 실행해 크기를 확인한 뒤
  `eidos stock-game <instance> freeze --yes`로 그 버전을 인스턴스에 고정하세요. 이후
  스토어 업데이트는 사본이 아니라 스토어 설치본을 바꿉니다.
- "<mod> replaces <n> <extender> script(s)"는 스크립트 익스텐더보다 우선순위가 높은 모드가
  익스텐더 자체 스크립트의 낱개 `.pex` 사본을 갖고 있을 때 나타납니다. 오래된 사본은
  그것이 선언한 함수를 망가뜨리고, 게임은 오류를 보여주지 않습니다. 익스텐더를 그
  모드 아래로 옮기거나(둘 다 모드라면 카드의 Move 버튼이 해 줍니다), 그 모드에서 해당
  파일을 숨기세요. 익스텐더의 스크립트가 게임의 `Data`에 낱개로 놓여 있다면, 순서를
  정할 수 있도록 익스텐더를 모드로 설치하세요. 아카이브는 확인하지 않고 낱개 파일만
  확인합니다.

### 도구 출력

- "Overwrite/Root holds meshes"(또는 textures)는 데이터 경로가 게임 폴더를 가리키던
  도구가 그 폴더들을 거기에 썼다는 뜻이며, 게임은 그곳을 결코 읽지 않습니다. 도구의
  경로를 고치고, 그 폴더를 지운 다음 도구를 다시 실행하세요. Open folder 버튼은
  `Overwrite/Root`를 엽니다.
- "BodySlide's GameDataPath is not the game's Data folder"(또는 `OutputDataPath`):
  [도구](tools.md#도구-자신의-설정은-여전히-그-도구의-몫)를 보세요.

### 마지막 세션의 크래시

"The game crashed in the last session"은 크래시 로거가 마지막 실행 중에 로그를 썼을 때
나타납니다. 카드는 예외와, 로그에 나온 DLL과 플러그인 중 당신의 모드나 Overwrite가
설치한 것을 알려 줍니다. 이것들은 단서로 다루세요. 크래시 로그에 이름이 나온 모드는
원인이 아니라 크래시가 드러난 자리인 경우가 많습니다. 로그가 당신의 모드의 파일을
하나도 가리키지 않으면 카드가 그렇다고 말합니다. Open the log는 로그를 열고, Copy the
log (home path hidden)는 홈 폴더를 가려서 복사합니다.

### 창 크기

Skyrim SE, Enderal SE, Fallout 4에서는 Display 카드가 프로필이 게임을 시작하는 창
크기와 모드를 보여주며, 프로필의 Prefs INI도 prefix의 것도 크기를 정하지 않았으면 "No
window size set for this profile"이라고 말합니다. Apply는 크기와 모드를 프로필의
사본에 쓰고, SSE Display Tweaks가 창을 설정하는 경우 Overwrite 안의 그 INI 사본에도
씁니다([usage.md](usage.md#diagnostics)).

## 어떤 Proton이 게임을 실행하는지, 그리고 망가진 프리픽스

Diagnostics 탭에는 Proton 카드가 있습니다. Eidos와 Steam이 그 게임에 쓰는 빌드, Steam
자체 설정, 그리고 프리픽스를 마지막으로 실행한 빌드를 보여 줍니다. 선택한 것보다
새로운 Proton이 프리픽스를 마지막으로 준비했다면 카드는 조언이 됩니다. 더 새로운
프리픽스에서 더 오래된 Proton을 쓰면 Wine 파일이 초기화되거나 시작되지 않을 수 있기
때문입니다. Steam에서 게임의 속성, 호환성 항목에서 적어도 그만큼 새로운 빌드를
고르거나, 프리픽스를 다시 만드세요.

그 밖의 Proton과 프리픽스 카드:

- "Proton comes from the Flatpak Steam install": 그 Proton은 런타임과 steamclient
  라이브러리를 Flatpak 샌드박스 안에 두므로 호스트에서 실행하면 실패할 수 있습니다.
  Eidos는 이를 `flatpak run`으로 시작하지 않습니다. 그러면 게임이 병합된 뷰 바깥의
  샌드박스에 들어가기 때문입니다. `~/.steam/root/compatibilitytools.d/`에 Proton을
  설치하고 그 게임에 선택하세요.
- "Eidos cannot tell which Proton runs this game": Steam의 `config.vdf`가 그 게임에 호환성
  도구를 지정하지 않았고, 프리픽스에도 설치된 빌드가 기록되어 있지 않습니다. Steam에서
  하나를 고르기 전에는(게임의 속성, 호환성) 도구가 시작되지 않습니다.
- "<game> has no Proton prefix yet": 게임을 Steam으로 한 번 실행한 뒤 F5를 누르세요.
  Steam이 프리픽스를 만들지 않으면, `steamapps/compatdata` 아래 그 게임의 폴더에 쓸 수
  있는지 확인하세요. root 소유의 빈 폴더가 있으면 아무 메시지 없이 막힙니다.
- "Windows tools cannot find <game>": 프리픽스에 기록된 설치 경로가 틀렸거나 없으며,
  xEdit, DynDOLOD, Wrye Bash, Creation Kit이 그것을 읽습니다. Eidos는 다음에 도구를
  실행할 때 올바른 경로를 씁니다.
- "The prefix's S: drive points outside the Steam library": `S:`가 Eidos가 도구에 맞춰 두는
  라이브러리 루트와도, Steam이 게임에 맞춰 두는 `<library>/steamapps`와도 맞지
  않습니다. 라이브러리가 옮겨졌거나 프리픽스가 다른 기계에서 왔을 가능성이 큽니다.
  프리픽스에 저장된 `S:` 상대 경로는 모두, 다음에 Eidos나 Steam으로 실행해 다시 맞춰질
  때까지 어디로도 해석되지 않습니다.
- "<store> installation: started through its store"는 GOG나 Epic 사본의 카드입니다. Eidos가
  그곳에서 게임의 래퍼이면 Run이 Heroic이나 Legendary에 게임 시작을 요청하고, 도구는
  Heroic이 그 사본에 기록한 Wine이나 Proton 빌드로 실행됩니다.

File 메뉴는 winecfg, regedit, Wine 파일 탐색기, 제거 프로그램을 게임의 프리픽스에서,
게임의 도구를 실행하는 Proton으로 실행합니다
([tools.md](tools.md#게임-프리픽스-안의-wine-자체-프로그램)). 프리픽스에
그 프로그램들이 생기기 전까지는 해당 행이 회색으로 남습니다.

Proton 카드의 Rebuild prefix 버튼은 더 이상 작동하지 않는 프리픽스를 위한 것입니다.
두 번 누르세요. Eidos는 아무것도 지우지 않습니다. 같은 폴더 안에서
`steamapps/compatdata/<appid>`의 이름을 `<appid>.eidos-backup-<date>-<time>`으로 바꾸고,
다음에 Steam에서 게임을 시작할 때 Steam이 새 프리픽스를 만듭니다. 게임, 도구 또는 다른
Wine 프로그램이 프리픽스를 쓰는 동안에는 거부합니다. 새 프리픽스가 생기기 전까지는
창의 Run이 게임을 Steam에 넘길 수 없고 실행 옵션을 보여주므로, Steam에서 Play를
누르세요. 옛 프리픽스에만 있던 세이브와 설정은 이름이 바뀐 폴더에 남습니다. 필요한
것을 다시 복사하고, 더 이상 필요 없으면 그 폴더는 직접 지우세요.

## 버그 보고서에 첨부할 것

- Diagnostics 탭의 Copy report는 Eidos 버전, 게임, Proton 빌드, Steam이 Flatpak인지,
  패스스루 상태, 세션 종류, 배포판, 커널, `vm.max_map_count`, 열린 파일 수 제한,
  `/dev/fuse`와 `fusermount3`가 있는지, 프로필의 실행 변수와 래퍼 프로그램의 이름, 그리고
  모든 Health 카드를 복사합니다. 홈 폴더는 `~`로, 로그인 이름은 `<user>`로 표시됩니다.
  `eidos check <instance>`는 같은 머리말을 터미널에 출력하며, `--json`은 JSON으로
  줍니다.
- 실행 로그 `<instance>/logs/run-*.log`는 Eidos가 실행한 정확한 명령으로 시작합니다.
  File, Eidos logs는 Eidos 자체 로그의 폴더 `~/.local/share/Colony/Eidos/logs`를
  엽니다.
- 크래시 카드의 Copy the log (home path hidden)는 크래시 로그를 복사합니다.
- Data 탭의 Save tree는 병합된 `Data` 트리를 각 파일의 출처와 함께 저장합니다.

## 알려진 문제

### DLSS나 프레임 생성이 조용히 아무 일도 하지 않음

원인이 셋으로 갈리고 어느 것도 오류 메시지를 내지 않습니다. 실행 옵션에서 NVAPI가
켜지지 않았거나, 독점 전체 화면이거나, 예전 값으로 남은 Reflex FPS 상한입니다. 점검
목록 전체는 [graphics.md](graphics.md)에 있습니다.

### Flatpak Steam에서 Run이 아무 일도 하지 않음

스토어가 열지 않은 창에서 Run은 `steam -applaunch <appid>`를 실행해 Steam에 게임 시작을
요청합니다. Flatpak Steam은 호스트의 `PATH`에 `steam`을 두지 않으므로, 상태 표시줄에
오류와 함께 "Could not start Steam"이 표시됩니다. 대신 Steam에서 Play를 누르세요.

### 다른 기기의 세이브가 보이지 않음

Eidos를 거치지 않고 프리픽스에 들어온 세이브, 예를 들어 Steam Deck에서 받은 Steam
Cloud 다운로드는 프로필의 세이브가 마운트된 동안 숨겨진 채로 있습니다. 아무것도 그것을
지우지 않습니다. Saves 탭은 제목 아래에 이런 세이브의 수를 보여주며, Import는 프로필
세이브를 덮어쓰지 않고 그것들을 프로필로 복사합니다
([usage.md](usage.md#세이브는-프로필에-속합니다)).

### 한 디렉터리를 두 가지로 적은 모드가 두 번째 철자 아래의 모든 것을 잃었음

고쳐졌습니다. ext4는 `meshes/`와 `Meshes/`를 별개로 두지만 병합된 뷰는 그래서는 안
되고, 실제 모드는 둘 다 담아 배포합니다 - XP32 Maximum Skeleton은 애니메이션과 FNIS
동작 파일을 대문자 쪽에, `character assets`를 다른 쪽에 둡니다.

리졸버는 경로 요소마다 대소문자가 정확히 맞는 것을 택하고 거기에 매달렸습니다.
`meshes/`로 들어갔다가 나머지 경로를 거기서 찾지 못하자 레이어 전체를 포기한
것입니다. 다른 철자 아래의 모든 파일이 게임에게는 보이지 않았습니다 - 오류도, 로그도,
어떤 진단에도 아무것도 없었습니다. 실제 50레이어 인스턴스에서 그것은 74개
파일이었습니다.

이제 맞아떨어지는 요소는 결정이 아니라 후보입니다. 정확한 대소문자를 여전히 먼저
시도하고, 그 아래에서 나머지가 실패했을 때에만 스캔이 대소문자를 접었을 때 같아지는
형제를 찾습니다. 목록 조회에도 한 디렉터리 위에서 같은 결함이 있었고, 이제 레이어마다
대소문자를 접었을 때 같아지는 디렉터리를 모두 읽습니다.

경로 인덱스에는 이 버그가 없었습니다. 찾아낸 디렉터리를 모두 훑기 때문입니다. 그래서
한동안 인덱스는 폴백이 내놓지 못하는 파일을 돌려주었는데, 결코 틀리지 않아야 하는
답은 폴백 쪽입니다.

### DynDOLOD의 LODGen이 빈 로그를 남기고 죽음

`dotnet10`으로 해결됩니다. [tools.md](tools.md)를 보세요. 모든 월드마다
`LODGen_SSE_<world>_log.txt`에 버전 배너와 `.NET Version:` 줄만 있고 그 밖에는
아무것도 없으며, "failed to generate object LOD for one or more worlds"만 적힌 대화
상자가 뜹니다. 원인은 Wine의 Mono가 .NET Framework 행세를 하는 것이고, .NET
Framework를 아무리 설치해도 고쳐지지 않습니다 - Proton은 프리픽스를 갱신할 때마다
`mscoree.dll`을 자기 트리 안으로 향하는 심링크로 바꿔 놓습니다. 릴리스 번들은 .NET 10
런타임을 싣고 있고, 다른 설치는 그것을 한 번 내려받습니다.

### Wine은 마운트가 대소문자를 접는다는 것을 알 수 없었음

고쳐졌습니다.

Wine의 `get_dir_case_sensitivity`는 먼저 `FS_IOC_GETFLAGS`로 디렉터리에 묻고,
그다음 CIOPFS가 자기가 제공하는 디렉터리에 남기는 표식을 찾아봅니다. 둘 다
없으면 Wine은 대소문자를 구분한다고 가정하고, 철자가 바이트 단위로 맞지 않는
모든 조회는 대소문자 무시 일치를 찾으려 디렉터리 전체를 읽는 쪽으로 물러섭니다.
Bethesda 게임은 파일이 `ccBGSSSE001-Fish.bsa`인데도
`data/ccbgssse001-fish.bsa`를 요구하므로, 이는 거의 모든 에셋에서 발동했습니다.
8초 동안 표식 탐침 4471회와 디렉터리 전체 재읽기 2236회, 90초 동안 `Data` 열거
195796회. Skyrim SE는 메인 메뉴에 끝내 이르지 못했습니다 - 상주 메모리 240 MB에
앉아 있는 동안 데몬은 코어 하나의 92%를 태웠습니다.

Eidos는 처음부터 `resolve_read`에서 대소문자를 접고 있었습니다. 그 모든 비용은
그렇다고 말하지 않은 데서 왔습니다. 이제 `lookup`이 `.ciopfs`에 답합니다.
`readdir`은 여전히 그것을 나열하지 않습니다. `ioctl`도 디렉터리에 대한
`FS_IOC_GETFLAGS`에 casefold 플래그로 답하며, 현재 Wine은 표식을 찾기 전에
이것을 먼저 묻습니다.

이것을 그저 느린 정도가 아니라 치명적으로 만든 것은 둘입니다. 비용이 디렉터리 크기에
비례해서 커지므로 Anniversary 콘텐츠 설치(`Data`가 37개 파일에서 177개로)가 저울을
넘겼습니다. 그리고 `opendir`이 병합된 목록을 미리 만들었는데, Wine이 디렉터리를 여는
이유가 그 안의 표식을 `stat`하기 위해서일 뿐일 때 이는 순전한 낭비입니다 - 이제
스냅숏은 첫 `readdir`에서 찍힙니다.

그 뒤: 메인 메뉴 도달, 상주 메모리 2.1 GB, 데몬 CPU 0%.

이것을 찾아낸 것은 `EIDOS_FUSE_TRACE=opendir`이고, 그대로 배포됩니다. 연산 카운터는
몇 번인지 말해 줍니다. 한 디렉터리를 195796번 열거한 것은 총합 안에서는 보이지
않습니다.

### 게임이 `plugins.txt`를 비워서 다시 썼음

거의 확실히 위의 대소문자 접기와 같은 일이었습니다 - 어떤 합리적인 시간 안에도 열거할
수 없는 `Data`를 두고, 거기에 아무것도 없다고 결론짓고 그것을 저장한 것입니다.
증명되지는 않았고 다시 확인해 볼 만합니다. 어느 쪽이든 캡처 가드(활성 집합을 통째로
비우는 캡처는 크기와 무관하게 거부됩니다) 덕분에 이제 그것이 프로필을 망가뜨릴 수는
없습니다.

### `FOPEN_KEEP_CACHE`는 꺼져 있음

고쳐졌습니다. 모드를 하나도 설치하지 않은 상태에서 메인 메뉴 몇 초 뒤에 널
역참조로 Skyrim SE를 결정론적으로 죽였습니다. 나머지 커널 쪽 캐시 셋은 하나씩
이분해서 배제했고 이것만이 문제였습니다. 그 이분 탐색은 `FUSE_PASSTHROUGH`를 켠
채로 진행되었는데, 커널은 `FOPEN_KEEP_CACHE`를 단 패스스루 열기를 EIO로 실패시키므로,
이 플래그를 켜면 모든 패스스루 열기가 실패했습니다. 게임을 죽인 것은 어쩌면 이 실패한
열기들이었고, 패스스루를 끈 상태에서 이 플래그만 따로 시험한 적은 없습니다. 패스스루
응답은 이제 이 플래그를 달지 않습니다. 그것을 잃는 비용은 그때 공짜로 측정되었지만,
패스스루가 켜져 있으면 데몬은 읽기를 *하나도* 제공하지 않으므로(`EIDOS_FUSE_STATS`는
전체 로드에 대해 `read 0`을 보고했습니다), 패스스루가 기본으로 꺼진 지금 실제 비용은
측정되지 않았습니다. 따로 시험하려면 `EIDOS_FUSE_KEEP_CACHE=1`로 다시 켜세요.

### FUSE 패스스루가 게임이 모드 콘텐츠를 하나도 로드하지 못하게 함

데몬은 이제 가장 유력한 원인(아래)을 피하지만, 이는 아직 게임에서 다시 측정되지
않았으므로 패스스루는 기본으로 꺼진 채로 남습니다. `EIDOS_FUSE_PASSTHROUGH=1`이
켭니다. 이 변경 전에는 패스스루를 켜면 Skyrim SE가 자기 파일 152개를
`STATUS_ACCESS_VIOLATION`으로 열지 못했고, 그래서 모드 콘텐츠가 조용히 하나도 로드되지
않았습니다. 측정과, 데몬 자신의 로그가 깨끗한 실행을 보여준 이유는 위의 "패스스루가
기본으로 꺼져 있는 이유"에 있습니다.

커널은 inode 하나에 백킹 파일 하나만 묶습니다(`fs/fuse/iomode.c`). 이미 백킹
파일이 있는 inode에 두 번째 백킹 ID를 가져오는 열기는 EIO로 실패하고, 백킹
파일이 있는 inode를 일반 열기(`FOPEN_PASSTHROUGH` 없이)로 여는 것도
마찬가지입니다. `FOPEN_KEEP_CACHE`를 단 패스스루 열기도 EIO로 실패합니다. 예전
데몬은 열기마다 새 ID를 등록했으므로, 게임이 이미 열어 둔 파일을 두 번째로 열면
실패했습니다. Wine은 EIO를 `STATUS_ACCESS_VIOLATION`으로 보고하며, 이것이
152건의 실패한 열기 뒤에 있던 오류입니다. 이 규칙은 커널 소스에서 읽은 것이고 그
152건의 실패와 맞아떨어지므로, 가장 유력한 원인입니다.

이제 데몬은 inode마다 백킹 파일 하나만 등록하고 그 inode의 모든 패스스루 열기가
이를 공유하며, 패스스루 열기에는 `FOPEN_KEEP_CACHE`를 절대 달지 않습니다.
패스스루는 게임과 모드 레이어의 파일 가운데 1 MiB 이상이거나 `.dll`, `.exe`,
`.asi`로 끝나는 파일의 읽기 전용 열기에만 쓰입니다. 파일에 백킹이 생기고 나면
그 파일의 모든 읽기 전용 열기가 그 백킹을 쓰고, 파일에 일반 열기가 남아 있는 동안에는
새 독자도 일반 열기를 받습니다. Overwrite에 있는 파일은 절대 패스스루를 거치지
않으며, 플러그인 타임스탬프를 투영하는 마운트는 패스스루를 아예 쓰지 않습니다.
패스스루 독자가 붙잡고 있는 파일을 쓰기용으로 열면 어떤 copy-up보다도 먼저
ETXTBSY(Wine에서는 `STATUS_SHARING_VIOLATION`)를 받으므로, `O_TRUNC` 열기가 모드
파일을 가리는 빈 사본을 Overwrite에 남길 수 없습니다.

### Mod Manager Download 버튼이 아무 반응이 없음

Preferences, Nexus, Downloads에는 `nxm://` 링크를 받는 프로그램이 표시됩니다. 그
프로그램이 Eidos가 아니거나, 없거나, 지정된 것이 없으면 Health 카드가 나타납니다.
Register 버튼은 `eidos-nxm.desktop`을 만들고 `x-scheme-handler/nxm`의 기본 처리기로
지정합니다. 터미널에서는 `eidos nxm --register`가 같은 일을 합니다.

상태에 이미 Eidos가 표시되는데도 버튼이 반응하지 않으면, 브라우저가 이전에 한 선택을
쓰고 있을 수 있습니다. 브라우저 설정에서 nxm 링크에 저장된 선택을 지워 다시 묻도록
하세요. Flatpak 브라우저는 호스트 프로그램을 직접 실행할 수 없고, 호스트에 등록된
처리기를 읽는 데스크톱 포털을 통해 링크를 넘깁니다.
