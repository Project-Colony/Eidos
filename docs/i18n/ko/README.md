<!-- eidos-i18n: source=README.md sha=31c66f68ad4e47e896eeb2256fd8f89c68cfdbef -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**게임 폴더를 절대 건드리지 않는 네이티브 Linux 모드 매니저.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#설치)

Eidos는 Linux의 Bethesda 게임에 Mod Organizer 2가 Windows에서 주는 것, 곧 실행할
때마다 만들어지는 모드의 병합 뷰를 줍니다. Windows API 후킹 대신 Linux의 마운트
네임스페이스와 FUSE를 쓰며, 매니저는 Wine 없이 실행됩니다. 게임 폴더로 복사되는
파일이 없으므로 나중에 정리할 것도 없습니다. Eidos에는 Stellar Blade, Stardew
Valley, 7 Days to Die, Crimson Desert의 정의도 들어 있습니다.

Steam 사본은 게임의 실행 옵션에서 Eidos를 실행합니다. Heroic이나 Legendary의 GOG와
Epic 사본은 Eidos를 래퍼로 실행합니다. 게임은 언제나 그 사본을 가진 스토어를 통해
시작되며, Eidos가 Proton으로 직접 시작하는 일은 없습니다.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **상태:** Skyrim SE는 매일 Eidos로 플레이되고 있습니다. SKSE, 스크립트 익스텐더
> 프리로더, Creation Club, LOOT로 정렬한 로드 순서, 프로필별 세이브를 모두 씁니다.
> 지금까지 실제 플레이로 검증된 게임군은 하나입니다. Bethesda 게임 열 개와 다른 게임
> 네 개가 연결된 채 테스터를 기다리고 있습니다.

## Eidos를 쓰는 이유

병합 뷰는 게임의 프로세스 트리만 볼 수 있는 비공개 마운트 네임스페이스에 마운트됩니다.
다른 프로그램은, 두 번째 게임도 포함해서, 그것을 보지 못하고 그것에 대한 권한도
필요 없습니다. 게임이 종료되거나 죽거나 전원이 끊기면 네임스페이스는 그 프로세스들과
함께 사라지고 설치 폴더는 이전 그대로입니다.

프로필마다 모드 목록, 플러그인 순서, INI, 세이브를 따로 가집니다. 실행할 때 플러그인
파일과 세이브 폴더가 게임 자신의 경로 위에 bind-mount되므로, 게임이 거기에 쓰는 것은
프로필로 들어갑니다. 프로필을 바꾸면 전부가 바뀝니다.

Eidos에는 setuid 도우미도, 시스템 데몬도, `sudo setcap`도, `/etc/fuse.conf` 수정도
필요 없습니다. FUSE 데몬은 한 번의 실행 동안 사용자 권한으로 돌다가 게임과 함께
끝납니다. 프로그램 하나를 설치하고 Steam 실행 옵션 하나를 설정하면 됩니다.

Eidos는 실행 전마다 플러그인 목록의 스냅샷을 보관합니다. 크래시가 목록을 망가뜨리면
Diagnostics가 그것을 스냅샷과 비교하고, Restore the pre-session order가 옛 순서를
되돌립니다. 로드 순서를 지워버릴 캡처는 거부되고 그 이유를 알려 줍니다.

Test run은 게임을 프로필의 모드로, 모드 없이, 또는 선택한 모드 중 켜진 것과 그 마스터만으로
시작합니다. 그 실행의 세이브, INI 변경, 플러그인 상태, Overwrite 출력, 그리고 로그를
포함한 My Games의 다른 파일은 `<instance>/test-run/`으로 갑니다. 게임이 끝나면 Eidos가
그 다섯 그룹을 나열하고, 그룹마다 남길지 버릴지 고릅니다. Wine 레지스트리 같은 다른
쓰기는 늘 가던 곳으로 갑니다. Diagnostics의 Find the mod behind a problem은 용의 모드의
절반과 그 모드들이 필요로 하는 마스터로 매번 테스트 실행을 돌립니다. 실행이 끝날
때마다 문제가 일어났는지 답하면, Eidos가 용의 모드를 좁혀 마침내 그 모드, 또는 서로의
마스터를 필요로 하는 모드들을 알려 줍니다. 용의 모드가 200개면 약 8번의 실행이
듭니다. 시작하기 전에 목록에서 선택한 모드 중 켜진 것은 모든 단계에서 켜진 채로 있습니다. 자세한
내용은 [테스트 실행](docs/guide/usage.md#테스트-실행)와
[문제를 일으키는 모드 찾기](docs/guide/usage.md#문제를-일으키는-모드-찾기)에 있습니다.

### 비교

| | Eidos | Wine 위의 MO2 | Fluorine-Manager | Limo / 링크 배포기 |
|---|---|---|---|---|
| 매니저가 네이티브로 실행 | 예 | 아니요, Wine 안의 Windows 앱 | 예 (Qt 포팅) | 예 |
| 게임 폴더 무손상 | 예, 언제나 | 예 | 예 | 아니요, 그 안에 링크를 씀 |
| 마운트가 보이는 대상 | 게임만 | 게임만 | 시스템 전체 | 해당 없음 |
| 크래시 후 정리 필요 | 없음 | 없음 | 죽은 마운트 복구 | 수동 해제 |
| root 모드 (ENB, 프리로더) | 예, 네이티브 | 플러그인 필요 | 플러그인 필요 | 부분적 |
| 필요한 권한 | 없음 | 없음 | `/etc/fuse.conf` 수정 | 없음 |

### 얼마나 빠른가

| | 이전 | 지금 |
|---|---|---|
| 세이브 로딩 | 약 20초 | 6-7초 |
| 한 세션의 디렉터리 읽기 | 560만 회 | 33만 5천 회 |

셀 이동은 즉각적입니다. 파일 하나를 찾는 데 모드 쉰 개에 차례로 물었고, 폴더 하나를
나열하는 데 그것을 쉰 번씩 되풀이했습니다. 이제 읽기 전용 레이어는 마운트할 때 한 번
인덱싱되므로 파일 찾기는 해시 조회 한 번이고, 폴더의 병합된 목록도 마운트할 때 한 번
계산됩니다. 수치는 벤치마크가 아니라 평범하게 플레이한 실제 인스턴스에서 나온
것입니다: [internals/performance.md](../../../docs/internals/performance.md)(영어).

### 선행 사례와 감사

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer)와
  [usvfs](https://github.com/ModOrganizer2/usvfs): Eidos가 재현하는 의미론, 그리고 그
  동등성을 견주어 연구한 코드베이스
- [LOOT](https://loot.github.io/): libloot을 통한 정렬 엔진
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo), 그리고 다른 Linux 매니저들: 이 문제가
  풀리기를 바라는 커뮤니티가 있다는 증거

## 하는 일

### 모드

Eidos는 단순 압축 파일, FOMOD 마법사, Wrye Bash BAIN 패키지, 지원하는 OBMM 스크립트가
든 OMOD 컨테이너를 설치하고, 나머지는 수동 선택기로 설치합니다. Root 모드(스크립트
익스텐더 프리로더, ENB, Engine Fixes)는 Root Builder 플러그인 없이, 설치 폴더로 복사되는
것 하나 없이 네이티브로 동작합니다. 개별 파일 숨기기, 구분자로 모드 묶기, 선택한 모드를
지정한 위치로 이동, 모드별 메모와 카테고리, 그리고 MO2 프로필 가져오기가 가능합니다.

목록은 MO2처럼 동작합니다. 선택 열 여덟 개와 그중 어느 것으로든 정렬, 카테고리별 또는
출처별 그룹화, 더블클릭 제스처, 타이핑으로 건너뛰기, 복원하기 전까지는 아무 일도 하지
않는 모드별 백업, 그리고 이 게임이 로드하지 못할 배치이거나 다른 게임용으로 받은 모드에
붙는 플래그가 있습니다. 필터는 Nexus 상태(추천함, 추적 중, 업데이트 있음), 메모, 숨긴
파일, 내용 종류(플러그인, 아카이브, 텍스처, 메시, 스크립트, 사운드 등)로 모드를
고릅니다. 파일 트리는 폴더를 만들고, 이름을 바꾸고, 지우고, 파일을 열며, 아무것도
실행하지 않고 이미지와 텍스트를 미리 봅니다.

Ctrl+Z와 Ctrl+Shift+Z는 모드 목록과 로드 순서의 변경을 되돌리고 다시 실행합니다.
프로필의 가장 최근 세이브 세 개 중 하나가 쓰는 모드를 끄거나 제거할 때는 먼저 묻고
플러그인 이름을 알려 줍니다(Skyrim과 Fallout 4 세이브). FOMOD 모드를 다시 설치하거나
업데이트하면 마법사가 지난 설치의 답으로 열립니다. 모드의 정보는 그 모드가 필요로
하는 모드와 그 모드를 필요로 하는 모드를 나열하며, 활성 플러그인의 마스터에서
읽습니다.

### 플러그인

로드 순서에는 LOOT 정렬이 내장되어 있고, 게임이 계산하는 방식 그대로의 모드 인덱스,
마스터 누락 경고, 관리되지 않는 행으로 표시되는 DLC와 Creation Club 콘텐츠가 있습니다.
헤더는 인덱스 공간(full, medium, light)별로 쓴 슬롯 수를 셉니다. 플러그인에 마우스를
올리면 작성자, 설명, 헤더 버전이 보입니다. Skyrim 엔진에서는 게임이 같은 이름의 INI를
로드하는 플러그인에 표시가 붙습니다.

LOOT가 dirty로 표시한 플러그인은 제자리가 아니라 xEdit의 QuickAutoClean으로 정리해
`Cleaned plugins` 모드로 들어갑니다. Flag as light는 새 모드에 플러그인의 light 사본을
쓰며, 가장 최근 세이브 중 하나가 그 플러그인을 full 플러그인으로 불러오면
거부합니다(Skyrim과 Fallout 4 세이브).

### 인스턴스

인스턴스는 `~/.local/share/Colony/Eidos/instances/` 아래에 두는 전역이거나 휴대용입니다.
휴대용은 원하는 곳 어디에나 두는 자족적 폴더(두 번째 드라이브, 게임 파티션)이며, MO2의
것처럼 옮길 수 있고 격리되어 있습니다. 휴대용 인스턴스는 세션을 넘어 기억됩니다. 창도,
Steam 실행도, 모든 CLI 명령도 마지막에 쓴 것을 따라가고, 게임 id를 받는 곳이면 어디서나
그 폴더를 받습니다. 자세한 내용은
[usage.md](docs/guide/usage.md#인스턴스-전역과-휴대용)에 있습니다.

인스턴스는 인스턴스 안에 둔 게임 폴더의 사본에서 게임을 실행할 수도 있습니다. Eidos가
Wabbajack 목록을 처음 열면 목록 자체의 사본을 실행합니다. `ModOrganizer.ini`가 지목하는
폴더, 없으면 `Stock Game`이나 `Game Root` 폴더입니다. Diagnostics의 Game files 카드에
있는 Freeze this version은 스토어 설치본을 `<instance>/Stock Game`으로 복사하므로,
스토어 업데이트가 더 이상 게임이 실행하는 파일을 바꾸지 않습니다. 두 폴더가 블록을 공유할
수 있는 하나의 파일 시스템(btrfs나 XFS 등)에 있으면 사본은 스토어 설치본과 블록을
공유합니다(reflink). ext4에서나 두 파일 시스템에 걸치면 전체 사본이며, 크기와 남은 공간이
먼저 표시됩니다. 게임은 여전히 스토어가 시작합니다. Return to the store version과 Delete
the copy가 되돌리며, 터미널에서는
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`가
같은 일을 합니다. Morrowind와 `bUseMyGamesDirectory=0`인 Oblivion은 사본에서 실행할 수
없습니다. 자세한 내용은
[usage.md](docs/guide/usage.md#게임-파일-스토어-설치본-또는-사본)에 있습니다.

### 프로필

프로필마다 모드 순서, 플러그인 상태, INI, 세이브를 따로 가집니다. 세이브는 파싱되어
현재 플러그인과 비교되고, 세이브가 필요로 하는 것을 켜주는 버튼이 있으며, 매 세션 뒤
Steam Cloud를 위해 프리픽스로 다시 복사됩니다. Steam Cloud가 다른 기기에서 가져온
세이브나 Eidos 없이 한 실행이 프리픽스에 남긴 세이브는 Saves 탭에 나열됩니다. Import는
그것들을 프로필로 복사하며, 프로필이 이미 쓰는 이름의 세이브는 교체하지 않고 새
`orphan-` 이름으로 들어옵니다.

프로필은 게임과 도구 실행을 위한 실행 변수와 래퍼(INI 편집기에서 한 줄에 하나씩
`env=NAME=value`와 `wrapper=command`)를 가질 수 있습니다. Skyrim SE, Enderal SE,
Fallout 4에서는 Diagnostics의 Display 카드가 프로필의 INI 사본에 창 크기와 모드를
설정합니다. Backups는 프로필 INI의 복원 지점을 보관하고, Eidos는 BethINI Pie를 실행할
때마다 하나를 만듭니다. 프로필의 바탕화면 바로가기는 스토어를 통해 게임을 시작합니다.
`eidos sessions`는 기록된 실행을 나열하고 하나를 현재 상태와 비교합니다.

### Nexus

계정을 연결하면 Mod Manager Download 링크를 받고, 업데이트를 확인하고, 제작자와 프로필을
볼 수 있습니다. 각 모드는 그 파일이 온 Nexus 도메인에서 확인, 추천, 링크됩니다. 그래서
Enderal SE 인스턴스의 Skyrim SE 모드는 Skyrim SE 사이트에 머뭅니다. 업데이트는 제작자가
Nexus에 지정한 후속 파일 사슬을 따라가고, 모드 폴더를 새 파일로 교체하며(옛 폴더의
백업을 제안합니다), 우선순위와 활성화 여부를 유지하고, 설치할 때 저장한 FOMOD 답을
재생합니다. `eidos nexus update <instance> --apply`는 터미널에서 프리미엄 계정에 같은
일을 하며, 무료 계정이면 파일 페이지를 출력합니다. Track과 Endorse는 모드 메뉴에 있고,
업데이트를 확인할 때마다 Nexus 계정과 동기화됩니다. 모드의 정보에는 설명과 변경 내역을
일반 텍스트로 보여주는 Nexus page 탭이 있고, General 탭 끝에는 마지막 업데이트가 바꾼
것, 곧 추가, 삭제, 크기가 바뀐 파일과 추가되거나 삭제된 마스터가 나옵니다. Install
downloads when they finish(Preferences, Nexus, 기본값은 꺼짐)는 창이 열려 있는 동안
끝난 다운로드를 바로 설치합니다.

컬렉션은 레시피에 따라 설치되며 중단된 진행을 재개할 수 있습니다. 지원하지 않는 단계는
명확히 보고됩니다. 일치 여부는 정확한 원본 파일로 확인하며, 불완전한 예전 메타데이터는
미검증 상태로 표시됩니다. 컬렉션 패널은 게임용으로 게시된 컬렉션을 둘러보고 검색하며,
설치한 컬렉션에 더 새로운 리비전이 있으면 알려 주고, 무료 계정에는 빠진 파일을 한
페이지씩 안내합니다. 끌기가 컬렉션의 순서 규칙을 깨면 Diagnostics가 알려 주고, Restore
collection order가 그 모드를 되돌려 놓습니다. Downloads 탭은 압축 파일을 필터링, 정렬,
숨기기, 삭제합니다. Offline 모드(Preferences, Nexus)는 Nexus 접근과 하루 한 번의 릴리스
확인을 멈춥니다.

### 도구

xEdit, BodySlide, DynDOLOD 같은 모딩 도구는 게임의 Wine 프리픽스(Steam의 Proton
프리픽스, 또는 Heroic이 만든 프리픽스) 안에서 병합된 뷰를 통해 실행됩니다. 도구는
여러분의 모드를 보고, 출력은 Overwrite에 떨어지며, 한 번의 클릭으로 그 출력이 진짜
모드가 됩니다. Skyrim SE에서 Eidos는 SSEEdit, SSEEdit QuickAutoClean, PGPatcher,
Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen, Synthesis, BethINI Pie를
게임 폴더 안, 모드 안, 또는 게임들 옆에 두는 도구 폴더에서 찾아 주며, 각각 필요한
런타임과 함께입니다. 다른 게임에는 자기 목록이 있습니다. Pandora, Nemesis, FNIS,
BodySlide, Synthesis는 출력을 이름 붙은 모드(`Nemesis Output` 등)에 씁니다.

릴리스 번들은 Microsoft의 Visual C++ 2015-2022와 .NET 8 Desktop Runtime 설치 프로그램,
그리고 .NET 10 런타임을 싣고 있습니다. Eidos는 `vcrun2022`와 .NET 8을 그 설치 프로그램으로
winetricks 없이 설치하며, 번들이 없으면 같은 파일을 Microsoft에서 내려받고 각각 Eidos에
내장된 SHA-256과 대조합니다. Synthesis에는 .NET 10 SDK가 필요한데, 번들에 싣지 않은 300
MB 내려받기입니다. 나머지 사전 요구 사항(.NET Framework 4.8, XACT, 더 오래된 .NET 버전)은
여전히 winetricks를 거칩니다. 도구의 사전 요구 사항을 설치하겠다고 하기 전에는 아무것도
내려받지 않습니다.

쓰는 도구는 고정하고, 쓰지 않는 것은 숨기고, 자체 Steam 앱인 도구에는 자기 Steam AppID를
주고, Eidos를 열지 않고도 병합된 뷰를 통해 도구를 실행하는 `.desktop` 바로가기를 쓸 수
있습니다. `eidos tool <game> import-mo2`는 목록 폴더의 `ModOrganizer.ini`에 있는 실행
파일을 나열하고, `--write`가 그것을 추가합니다. Data 탭의 exe는 한 번 실행하거나 도구로
추가할 수 있으며, 터미널에서는 `eidos tool <instance> run --exe <path>`가 하나를
실행합니다. Ask before playing을 켜면 생성기의 출력이 오래되었을 때 Run이 먼저 묻습니다.

Stop은 실행 중인 게임이나 도구를 끝냅니다. File 메뉴는 게임의 프리픽스에서 winecfg,
regedit, Wine 파일 탐색기, Windows 제거 프로그램을 실행합니다. Diagnostics의 Rebuild
prefix는 프리픽스의 이름을 바꿔 Steam이 새로 만들게 하며 아무것도 지우지 않습니다.
`eidos shell <instance>`는 병합된 뷰에서 셸을 열거나 `--` 뒤에 준 명령을 실행하며,
쓴 내용은 Overwrite로 들어갑니다.

창이 백그라운드에 있는 동안 Eidos는 도구 실행, 컬렉션 설치, pack이나 unpack, 마지막으로
실행 중이던 다운로드가 끝날 때, 그리고 LOOT 정렬이 실패할 때 데스크톱 알림을 보낼 수
있습니다. Preferences의 Notify the desktop when a long job ends가 이를 켜며, 기본값은
꺼짐입니다.

### 진단

Diagnostics 탭은 마스터 누락, 고아 아카이브, 모드 목록 어긋남, 손상된 플러그인 집합을
나열하고, 실행 뒤에는 스크립트 익스텐더 자신의 로그가 무엇이 로드되었다고 말하는지
보여줍니다. 다음도 보고합니다:

- 없거나 게임 버전과 맞지 않는 스크립트 익스텐더
- 마지막 정상 실행 이후 바뀐 게임
- Eidos를 시작하지 않거나 그 뒤에 `%command%`가 없는 Steam 실행 옵션
- 프리픽스를 마지막으로 준비한 것보다 오래된 Proton, 또는 Flatpak Steam의 Proton
- DLSS에 필요한데 빠진 `PROTON_ENABLE_NVAPI`, 그리고 Community Shaders의 프레임 생성을
  막는 독점 전체 화면
- `LinuxVersion=true`가 없는 `enblocal.ini`
- 지난 세션의 크래시 로그와 그 로그가 이름을 대는 모드
- 없거나 비활성화된 Nexus 요구 사항
- 다른 프로그램이거나 없는 파일인 `nxm://` 처리기
- 게임이 `plugins.txt`를 무시하게 만드는 `sTestFile` 줄
- Starfield의 활성 `.esp` 플러그인, 그리고 잘못되었거나 짝이 없는 블루프린트 플러그인
- 낱개 파일이 패치 대상 모드에 지는 패치
- 스크립트 익스텐더 자신의 것보다 앞서는 Papyrus 스크립트 사본을 가진 모드
- 낮은 `vm.max_map_count`나 열린 파일 수 제한
- Windows 파일 시스템 위의 게임 파일
- 게임의 Data 폴더가 아닌 BodySlide 데이터 경로, 그리고 Data에 있어야 할 `Overwrite/Root`의
  폴더
- 도구를 다른 게임 폴더로 향하게 하는 목록의 `ModOrganizer.ini`
- 없거나 게임 실행 파일이 없는 게임 사본

`eidos check <instance> [--json]`은 창이 필요 없는 점검을 돌리며, 문제를 찾으면 1로
끝납니다. Copy report는 홈 폴더를 `~`로, 사용자 이름을 `<user>`로 적은 보고서를 복사하고,
Data 탭의 Save tree는 병합된 Data 트리를 각 파일의 출처와 함께 같은 방식으로 가려서
씁니다.

교체 시 보호, BSA/BA2 내부 파일 충돌, SKSE 정적 진단, 생성 기록 및 그 한계는
[무결성 검사와 생성된 파일](../../../docs/internals/integrity.md)(영어)을 참고하세요.

### 자기 파일을 두는 곳

Eidos는 Colony 제품군의 모든 프로그램이 쓰는 배치를 씁니다. 고른 것(환경 설정, Nexus
세션, 인스턴스 목록, 직접 쓴 게임과 애드온 정의)은 `~/.config/Colony/Eidos/`에, 전역
인스턴스, 도구 런타임(내려받거나 `install.sh`가 번들에서 복사한 것), 세션 로그는
`~/.local/share/Colony/Eidos/`에, LOOT 마스터리스트는 `~/.cache/Colony/Eidos/`에 둡니다.
인스턴스 안에서는 `Stock Game`이 고정한 게임 사본을, `test-run/`이 검토를 기다리는 테스트
실행을 담습니다.

예전 Eidos는 파일을 `~/.config/eidos/`, `~/.local/share/eidos/`, `~/.local/state/`에
두었습니다. 업그레이드 후 첫 실행이 그것들을 옮기고, 로그에 그렇게 적고, 예전 디렉터리는
그대로 둡니다. 옮기기가 끝나기 전까지 Eidos는 예전 파일을 계속 씁니다. 자세한 내용은
[internals/paths.md](../../../docs/internals/paths.md)(영어)를 참고하세요.

### 지원 게임

Skyrim SE/AE는 실제 플레이로 검증되었습니다. Fallout 4도 끝에서 끝까지 연결되어 있습니다
(F4SE 자동 교체, 아카이브 무효화, 별표 로드 순서, LOOT, `.fos` 세이브).
[guide/fallout4.md](docs/guide/fallout4.md)를 참고하세요. 연결되어 있고 테스터를 찾는
게임은 Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV, Starfield,
Oblivion, Morrowind입니다. Fallout 3, Fallout NV, Oblivion, Morrowind는 파일의 날짜로
플러그인 순서를 정합니다. Eidos는 프로필마다 순서를 하나씩 보관하고, 디스크의 파일은
바꾸지 않은 채 마운트 안에서 그 날짜를 보여 줍니다.

다른 게임 넷도 연결되어 있고 역시 테스터를 찾습니다. Stellar Blade(`~mods`의 Unreal
`.pak` 모드), Stardew Valley(SMAPI 모드, SMAPI는 따로 설치), 7 Days to Die(게임 옆의
`Mods/` 폴더), Crimson Desert입니다. Crimson Desert는 코드 모드만 받습니다. `bin64`의
ASI 플러그인, ReShade, 로더 DLL입니다. PAZ 그룹에 들어 있는 데이터 모드는 지원하지
않습니다.

게임 하나는 TOML 파일 하나이고, 직접 만든 것은 다시 빌드하지 않고
`~/.config/Colony/Eidos/games/`에 넣을 수 있습니다:
[internals/adding-games.md](../../../docs/internals/adding-games.md).

## 설치

### Colony를 통해 (권장)

[Colony](https://github.com/Project-Colony/Colony)에서 **Eidos**를 검색해
설치하세요. 업데이트는 런처를 통해 들어옵니다. Colony는 파일 하나,
`eidos-linux`를 설치합니다. 창이면서, `eidos`라는 이름의 링크를 통해 호출하면
`eidos` 명령줄이기도 합니다. 이 파일에는 NIF 모델 미리 보기 도우미도, 아래 번들의
Microsoft 런타임도 들어 있지 않습니다. 도우미가 없으면 미리 보기가 그렇다고 알려 주고
나머지는 모두 동작하며, 도구의 사전 요구 사항을 설치하면 Eidos가 Microsoft에서
런타임을 내려받습니다. 설치 후에 할 일과 Steam이 이것을 가리키게 하는 법:
[docs/guide/install.md](docs/guide/install.md#colony를-통해).

### 바이너리 직접 내려받기

[최신 릴리스](https://github.com/Project-Colony/Eidos/releases/latest)에서
파일 하나를 받으세요:

| 파일 | 무엇인가 |
|---|---|
| `eidos-linux` | Colony가 설치하는 것과 같은, 파일 하나로 된 프로그램 전체 |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, NIF 미리 보기 도우미, Visual C++ 2015-2022와 .NET 8 Desktop Runtime 설치 프로그램, .NET 10 런타임, 아이콘, 라이선스 고지, 그리고 데스크톱 항목과 Nexus 다운로드 처리기를 등록하는 설치 프로그램 |

먼저 [코드 서명 정책](#코드-서명-정책)에 나온 대로 내려받은 파일을 확인한 다음:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

설치 프로그램은 sudo 없이 바이너리를 `~/.local/bin`에 넣고, 번들의 `runtimes/`를
`~/.local/share/Colony/Eidos/runtimes`로 복사합니다. `--system`과 `--bindir DIR`은
바이너리를 다른 곳에 넣고, `--from DIR`은 다른 폴더에서 가져옵니다. `--cap`은 sudo에
`CAP_SYS_ADMIN`을 요청하는데, 실험적인 FUSE 패스스루에만 필요하며 패스스루는 꺼진 채로
있습니다. `--no-cap`이 기본값입니다. 두 파일 모두 glibc 2.39 이상의 x86_64 Linux, FUSE
(`fusermount3`), 그리고 모드 압축 파일용 7-Zip이 필요합니다.

Eidos는 스스로 업데이트하지 않습니다. 하루에 한 번 시작할 때 창이 GitHub에 최신 릴리스를
묻고, 더 새로운 릴리스가 있으면 메뉴 모음의 Help 뒤에 릴리스 노트로 가는 링크를
보여줍니다. Preferences에서 이를 끌 수 있으며, `/usr/local`이 아닌 `/usr` 아래에 있는
사본은 패키지 관리자가 업데이트하므로 아예 묻지 않습니다.

### 소스에서 빌드

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Rust 1.92 이상이 필요하고(CI가 확인하는 `rust-version`입니다. CI와 릴리스는 1.99.1로
빌드합니다), NIF 미리 보기 도우미를 위해 `just`, CMake 3.20 이상, C++17 컴파일러도
필요합니다. `just build`는 코드 생성 단위 하나와 fat LTO를 쓰는 릴리스 프로필을
사용합니다. 빌드 작업 3개로 `eidos`와 `eidos-gui`를 처음부터 빌드하니 그렇게 334초,
thin LTO로는 243초가 걸렸고, fat LTO는 `eidos`를 1.2 MB, `eidos-gui`를 3.8 MB 더 작게
만듭니다(측정값은 `Cargo.toml`에 있습니다). `just profile=dev build`는 더 빨리
컴파일됩니다. Arch 패키지는 `packaging/PKGBUILD`에서 빌드됩니다. 사전 요구 사항과 `just`
없이 실행하는 명령: [docs/guide/install.md](docs/guide/install.md).

### Steam 실행 옵션

Eidos는 게임의 실행 명령으로 동작하며, 그래서 게임이 시작되기 전에 마운트할 수
있습니다. 대부분의 구성에는 설치 위치의 절대 경로를 쓴 기본 한 줄이면 충분합니다(Steam은
`PATH`를 읽지 않습니다):

```
~/.local/bin/eidos-gui %command%
```

Colony로 설치했다면 그 경로는 `~/.local/share/Colony/apps/Eidos/eidos-linux`입니다.
그다음 플레이를 누르세요. Eidos가 그 게임의 인스턴스로 열리고, Run이 병합된 뷰를 통해
게임을 시작합니다.

직접 연 창에서 Run은 Steam에 게임 시작을 요청하므로(`steam -applaunch <appid>`) 실행
옵션이 그대로 적용되고, Steam이 여는 창에서 Run을 누르게 됩니다. Heroic은
`heroic://launch` 링크를 받고, Legendary는 Eidos가 그곳에서 게임의 래퍼로 지정되어 있으면
`legendary launch <app>`를 실행합니다. 스토어 설정이 없으면 Run은 아무것도 시작하지 않고
무엇을 설정할지 알려 줍니다:
[guide/usage.md](docs/guide/usage.md#게임-실행하기). Start the game at once when a store
opens Eidos(Preferences, 기본값은 꺼짐)를 켜면, Eidos는 Diagnostics 점검을 돌리고 확인할
것이 없을 때 창을 보이지 않은 채 게임을 시작합니다.

나머지는 전부 그 앞에 쌓는 환경 변수이고, 자유롭게 조합됩니다:

| 원하는 것... | 앞에 붙일 것 |
|---|---|
| Community Shaders와 함께 쓰는 DLSS | `PROTON_ENABLE_NVAPI=1`. 이것이 없으면 DLSS는 아무 말 없이 초기화되지 않습니다. 전체 점검 목록은 [guide/graphics.md](docs/guide/graphics.md) |
| 화면에 FPS 카운터 | `DXVK_HUD=fps` |
| 모드 없는 드라이버 수준 프레임 보간 (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`. Community Shaders 자체의 프레임 생성과는 절대 함께 쓰지 마세요 |
| 버그 신고용 상세 로그 | `EIDOS_LOG=debug` (세션 로그는 `~/.local/share/Colony/Eidos/logs/`에 떨어집니다) |
| 마운트의 세션별 I/O 보고 | `EIDOS_FUSE_STATS=1` |
| 다른 FUSE 워커 수 | `EIDOS_FUSE_THREADS=8` (기본값 4. 동시성 버그를 쫓을 때 가장 먼저 시도할 것은 `1`입니다) |
| 이번 실행을 휴대용 인스턴스 하나에 고정 | `EIDOS_INSTANCE=/path/to/folder`. 이것이 없으면 Eidos는 마지막에 쓴 인스턴스를 엽니다 |
| Start the game at once가 켜져 있을 때 한 번만 창 보기 | `EIDOS_WINDOW=1` |

요즘의 모드 구성(Community Shaders, DLSS, 프레임 생성)에서 그대로 쓸 줄은 예시가 아니라
최종 명령입니다:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

구성이 되는지 확인하는 동안에는 앞에 `DXVK_HUD=fps`를 붙이고, 되고 나면 빼세요.

같은 변수를 Steam 줄 대신 프로필의 실행 변수에 넣을 수도 있으며, 그러면 프로필별로
유지되고 도구 실행에도 적용됩니다:
[guide/usage.md](docs/guide/usage.md#프로필별-변수와-래퍼).

더 깊은 진단 스위치(`EIDOS_FUSE_TRACE`, 캐시와 인덱스 이분 탐색 토글,
`EIDOS_FUSE_PASSTHROUGH`가 기본으로 꺼져 있는 이유)는
[guide/troubleshooting.md](docs/guide/troubleshooting.md)에 있습니다.

## 문서

모든 것은 [docs/](docs/)에 있고, [docs/README.md](docs/README.md)에서 읽는
사람을 기준으로 정리되어 있습니다. 사용자 안내서(설치, CLI와 GUI, xEdit과
DynDOLOD 같은 도구, 그래픽, 문제 해결), Eidos가 내부에서 어떻게 동작하는지,
그리고 왜 존재하는지입니다. 보안 정책과 취약점 신고 방법:
[SECURITY.md](../../../SECURITY.md).

## 언어

플레이어에게 필요한 페이지는 번역되어 있습니다. 영어가 정본입니다. 번역이
영어와 어긋나면, 영어 파일이 맞습니다.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

나머지는 영어입니다. `docs/internals/`와 `docs/project/`는 Rust 코드도 함께 읽는
사람들이 읽고, `CHANGELOG.md`는 생성되며, `SECURITY.md`도 번역하지 않습니다. 앞의 둘을
번역하면 필요로 하지 않는 독자를 위해 28,000 단어가 넘는 분량을 동기화해야 하게 됩니다.

번역마다 그것이 만들어진 영어 파일의 해시를 담고 있고, 그 해시가 영어와 더 이상 맞지
않으면 CI가 실패합니다. [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh)를
참고하세요. 그러면 번역을 고쳐 쓰고 `scripts/i18n-check.sh --fix`로 해시를 다시 찍습니다.
오래된 번역은 삭제하지 않고 고쳐 씁니다.

한 언어는 디렉터리 하나입니다. `docs/i18n/<lang>/`가 저장소 루트를 그대로
비추므로, `docs/i18n/de/docs/guide/install.md`는 독일어판 `docs/guide/install.md`입니다.
그래서 번역된 두 페이지 사이의 링크가 그 영어 원문 사이의 링크와 똑같은 문자열이
되고, 언어 하나를 거둬들이는 일은 `rm -r` 한 번이 됩니다.

## 코드 서명 정책

모든 릴리스 파일은 개발자 기기가 아니라 CI에서 Project Colony 조직이 서명합니다.
릴리스 페이지에서 각 파일 옆에 있는 것:

| 파일 | 무엇인가 |
|---|---|
| `<asset>.sig` | 조직의 릴리스 키로 만든, 파일에 대한 ed25519 서명 |
| `<asset>.meta` | 파일을 그 파일 이름, sha256, 릴리스 버전에 묶는 세 줄 |
| `<asset>.meta.sig` | `.meta`에 대한 ed25519 서명 |

개인 키는 조직의 비밀 값이며, 아무것도 빌드하지 않는 작업 안에서 공용
[sign-and-publish 워크플로](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)만
사용합니다. Eidos를 컴파일하는 작업은 그것을 절대 보지 못합니다.
Colony는 Eidos를 설치하거나 업데이트하기 전에 세 파일을 모두 확인하고, 설치된
것보다 오래된 릴리스는 거부합니다. OpenSSL 3으로 내려받은 파일을 직접 확인하려면
(모든 파일에 같은 명령이 통합니다):

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
sha256sum "$a"    # 이 다이제스트가 sha256 줄과 같아야 합니다
```

릴리스를 빌드하고 서명하고 게시하는 방법:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## 개인정보

Eidos는 원격 측정도, 분석 데이터도, 크래시 보고도 보내지 않습니다. 이 표에 있는
서버 말고는 어디에도 연결하지 않으며, 그것도 거기 적힌 동작을 할 때만입니다.
Offline 모드(Preferences, Nexus)는 Nexus로 가는 모든 연결과 하루 한 번의 릴리스 확인을
멈춥니다.

| 데이터 | 저장 또는 전송 | 어디에, 그리고 왜 |
|---|---|---|
| 환경 설정, 인스턴스 목록, 게임과 애드온 정의 | 저장 | `~/.config/Colony/Eidos/` |
| 인스턴스, 도구 런타임, 세션 로그 | 저장 | `~/.local/share/Colony/Eidos/`, 또는 휴대용 인스턴스에 고른 폴더 |
| LOOT 마스터리스트 | 저장 | `~/.cache/Colony/Eidos/loot/` |
| 릴리스 확인 | 전송 | 시작할 때 하루 한 번까지, `api.github.com/repos/Project-Colony/Eidos/releases/latest`로 요청 한 번. Eidos는 릴리스 태그만 읽고 아무것도 내려받지 않습니다. 기본으로 켜져 있고 Preferences에서 끌 수 있으며, Offline 모드에서는 건너뛰고, `/usr/local`이 아닌 `/usr` 아래에 설치된 사본은 패키지 관리자가 업데이트하므로 건너뜁니다. [GitHub 개인정보 처리방침](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Nexus 로그인 | 전송 | 로그인하면 브라우저에서 `users.nexusmods.com`이 열리고, 그 결과는 Eidos가 그 로그인 동안만 `127.0.0.1`에 여는 수신기로 돌아옵니다. 그다음 Eidos는 `users.nexusmods.com/oauth/token`에서 그것을 토큰으로 바꾸고, Nexus가 토큰 서명에 쓰는 공개 키를 가져옵니다. [Nexus Mods 개인정보 처리방침](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus 자격 증명 | 저장 | 액세스 토큰과 갱신 토큰, `~/.config/Colony/Eidos/nexus.ini`에 모드 0600으로. 예전 Eidos가 거기 쓴 `api_key=` 줄은 남겨 두지만 절대 읽지 않습니다. 원하면 지우세요. |
| Nexus 요청 | 전송 | Nexus 기능을 쓸 때 `api.nexusmods.com`으로: 계정 이름과 프리미엄 여부, 모드와 파일 정보, 업데이트 확인, 컬렉션, 다운로드 링크. 추천과 추적 중인 모드는 업데이트를 확인할 때마다 읽고, Endorse나 Track을 누르면 씁니다. 모드 요구 사항, 설명, 변경 내역, 컬렉션 검색은 `api.nexusmods.com/v2/graphql`로 갑니다. 모든 요청에는 액세스 토큰이 실리고 Eidos와 그 버전이 명시됩니다. |
| Nexus 다운로드 | 전송 | 요청한 파일을, 다운로드 링크가 가리키는 Nexus CDN 호스트에서 받아 인스턴스의 다운로드 폴더에 저장하고, 각 파일 옆에 MO2 형식의 `.meta`를 둡니다. |
| `nxm://` 링크 | 저장 | `install.sh`가 대신 실행해 주는 `eidos nxm --register`가 `~/.local/share/applications/eidos-nxm.desktop`을 쓰고 `xdg-mime`으로 그것을 `nxm://` 처리기로 지정하므로, 사이트의 Mod Manager Download 버튼이 링크를 Eidos에 넘깁니다. Arch 패키지는 같은 항목을 시스템 전체에 설치합니다. |
| LOOT 마스터리스트 | 전송 | 게임의 첫 LOOT 정렬 때와 업데이트를 요청할 때마다 `raw.githubusercontent.com/loot/...`에서. 여러분의 것은 아무것도 보내지 않습니다. [GitHub 개인정보 처리방침](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| 도구 런타임 | 전송 | 도구의 사전 요구 사항을 설치할 때만, 그리고 릴리스 번들에 없던 것만(단일 `eidos-linux` 파일과 소스 빌드에는 아무것도 없습니다): `builds.dotnet.microsoft.com`의 .NET 10 런타임과 `download.visualstudio.microsoft.com`의 Visual C++ 및 .NET 8 설치 프로그램(각각 Eidos에 내장된 SHA-256과 일치하지 않으면 거부), 항상 `builds.dotnet.microsoft.com`에서 받고 같은 방식으로 확인하는 .NET 10 SDK, 그리고 나머지 사전 요구 사항(.NET Framework 4.8, XACT, .NET 6과 7)을 위한 `winetricks`로, 설치할 것을 Microsoft와 winetricks가 쓰는 다른 호스트에서 내려받습니다. [Microsoft 개인정보 처리방침](https://www.microsoft.com/en-us/privacy/privacystatement) |
| 세션 로그 | 저장 | `~/.local/share/Colony/Eidos/logs/`, 모드 0600: 인스턴스당 세션 열 개, 30일 뒤 정리. 홈 폴더는 `~`로, 사용자 이름은 `<user>`로 적히므로 로그를 그대로 이슈에 붙여 넣을 수 있습니다. 여러분이 보내지 않는 한 기기를 떠나지 않습니다. |
| 실행 기록 | 저장 | `<instance>/logs/sessions/`: 게임이나 도구 실행마다 프로필, 활성화된 모드와 그 버전 및 Nexus 파일 id, 플러그인 순서, 프로필 INI별 체크섬, 게임과 Eidos 버전, 종료 코드, 그 실행에서 만들어진 세이브. 최신 50개를 보관하며 `eidos sessions`가 읽고 `eidos pack`은 포함하지 않습니다. 기기를 떠나지 않습니다. |
| 업데이트 보고서 | 저장 | `<instance>/logs/updates/<mod>.json`: 마지막 업데이트나 Replace가 그 모드 폴더에서 바꾼 것. `eidos pack`은 포함하지 않습니다. |
| 바로가기 | 저장 | `install.sh`가 `eidos.desktop`과 아이콘을 `~/.local/share` 아래에 씁니다. 여러분이 만든 도구나 프로필 바로가기는 `~/.local/share/applications`로 갑니다. |
| 데스크톱 알림 | 해당 없음 | 기본값은 꺼짐. 켜면 D-Bus(`org.freedesktop.Notifications`)로 데스크톱의 알림 서비스에 가고, 끝난 작업의 이름을 담습니다. |
| 여러분이 여는 링크 | 해당 없음 | 모드 페이지와 그 밖의 링크는 `xdg-open`을 통해 여러분의 브라우저에서 열립니다. |
| 애드온 | 해당 없음 | 설치한 확장은 여러분의 기기에서 별도 프로그램으로 실행되며, 그것이 하는 일은 그 확장의 몫입니다. |

## 라이선스

Eidos의 원본 코드는 GPL-3.0-or-later입니다. [LICENSE](../../../LICENSE)를
참고하세요. 다음 부분은 "or later" 선택지가 없는 GPL-3.0입니다:

- `eidos-install`의 OMOD 스크립트 이식 코드.
  [원저자 표기](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md)를 참고하세요;
- 내장된 nifly 라이브러리 위에 만든 `eidos-nif-preview` 도우미.
  [구성 요소의 라이선스와 저작자 표기](../../../crates/eidos-nif-preview/UPSTREAM.md)를 참고하세요;
- 로드 순서 코드가 링크하는 `esplugin`과 `libloadorder` crate. 이들은
  `GPL-3.0`을 선언합니다.

바이너리에 들어 있는 다른 Rust crate는 각자의 라이선스(MIT, Apache-2.0, BSD
등)를 유지합니다. 릴리스 번들은 `licenses/crates/`에 그 고지문과 함께 모든
crate와 그 라이선스의 목록을 담고 있습니다.

바이너리에는 진짜 Microsoft DirectX 재배포 DLL 여덟 개(`d3dcompiler_47`,
`d3dcompiler_43`, `d3dx9_43`, `d3dx11_43`, 32비트와 64비트)도 들어 있으며, gzip으로
압축해 저장하고(17.9 MB 대신 7.9 MB) 모드나 도구가 필요로 할 때 Eidos가 풀어 게임의
프리픽스에 배치합니다. 이것들은 Microsoft의 재배포 조건을 따릅니다.
[출처](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md)를 참고하세요.

릴리스 번들은 `runtimes/` 아래에 Microsoft의 Visual C++ 2015-2022 설치 프로그램, .NET 8
Desktop Runtime 설치 프로그램, .NET 10 런타임 zip도 Microsoft가 배포한 그대로 싣고
있습니다. 그 조건은 `licenses/microsoft-runtimes/`(`vc_redist.txt`, `dotnet.txt`)에
있습니다.
