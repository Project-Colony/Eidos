<!-- eidos-i18n: source=README.md sha=9c6df24d2b5b403d78e0fd5afa489427e7bf9d71 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**여러분의 게임을 절대 건드리지 않는 네이티브 Linux 모드 매니저.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#설치)

Eidos는 Linux의 Bethesda 게임에 Mod Organizer 2가 Windows에서 주는 것을 줍니다 -
실행할 때마다 만들어지는 모드의 가상 병합 뷰를 - Windows API 후킹이 아니라
Linux 원시 기능으로 만들어서. 매니저에는 Wine이 없습니다. 게임 폴더로 복사되는
파일도 없습니다. 정리 절차도 없습니다, 정리할 것이 없으니까요.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **상태:** Skyrim SE는 매일 Eidos로 플레이되고 있습니다 - SKSE, 스크립트
> 익스텐더 프리로더, Creation Club, LOOT로 정렬한 로드 순서, 프로필별 세이브까지
> 전부. 지금까지 실제 플레이로 검증된 게임군은 하나이고, 열 개가 더 연결된 채
> 테스터를 기다리고 있습니다.

## Eidos를 쓰는 이유

- 🔒 **게임만 볼 수 있는 마운트.** 병합 뷰는 비공개 마운트 네임스페이스 안에
  있습니다: 파일 관리자도, 백업 작업도, 다른 게임도 - 어느 것도 그것을 보지
  못하고, 어느 것도 그것에 대한 권한이 필요 없습니다. 게임을 죽이든 전원을
  뽑든: 네임스페이스는 프로세스 트리와 함께 사라지고 설치 폴더는 정확히 이전
  그대로입니다. 잔여물은 *구조적으로* 없습니다.
- 🧾 **진실의 사본은 하나.** 프로필이 자기 모드 목록, 플러그인 순서, INI,
  세이브를 소유합니다. 플러그인 파일과 세이브 폴더는 실행 시 게임 자신의 경로
  위에 bind-mount되므로, 게임이 직접 쓰는 것조차 여러분의 프로필로 떨어집니다.
  프로필을 바꾸면 전부가 바뀝니다.
- 🐧 **완전한 rootless.** setuid 헬퍼도, 데몬도, `sudo setcap`도,
  `/etc/fuse.conf` 수정도 없습니다. 바이너리 하나, Steam 실행 옵션 하나.
- 🛡️ **근거를 남기는 안전장치.** 플러그인 목록을 망가뜨린 크래시는 세션 이전
  스냅샷과 대조해 표시되고, 한 번의 클릭으로 복원합니다. 로드 순서를 지워버릴
  캡처는 거부되고 그 이유를 말합니다.

### 비교

| | Eidos | Wine 위의 MO2 | Fluorine-Manager | Limo / 링크 배포기 |
|---|---|---|---|---|
| 매니저가 네이티브로 실행 | ✅ | ❌ Wine 안의 Windows 앱 | ✅ (Qt 포팅) | ✅ |
| 게임 폴더 무손상 | ✅ 언제나 | ✅ | ✅ | ❌ 그 안에 링크를 씀 |
| 마운트가 보이는 대상 | 게임만 | 게임만 | **시스템 전체** | 해당 없음 |
| 크래시 후 정리 필요 | 설계상 없음 | 없음 | 죽은 마운트 복구 | 수동 해제 |
| root 모드 (ENB, 프리로더) | ✅ 네이티브 | 플러그인 필요 | 플러그인 필요 | 부분적 |
| 필요한 권한 | 없음 | 없음 | `/etc/fuse.conf` 수정 | 없음 |

### 얼마나 빠른가

| | 이전 | 지금 |
|---|---|---|
| 세이브 로딩 | 약 20초 | **6-7초** |
| 한 세션의 디렉터리 읽기 | 560만 회 | 46만 5천 회 |

셀 이동은 즉각적입니다. 이 이득은 모드에 질문을 덜 해서 나왔습니다: 파일 하나를
찾는 데 쉰 개 전부를 차례로 캐물었고, 폴더 하나를 나열하는 데 그것을 쉰 번씩
되풀이했습니다. 이제 둘 다 그러지 않습니다. 벤치마크가 아니라 평범하게 플레이한
실제 인스턴스에서 측정했습니다.

### 선행 사례와 감사

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer)와
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos가 재현하는 의미론,
  그리고 그 동등성을 견주어 연구한 코드베이스
- [LOOT](https://loot.github.io/) - libloot을 통한 정렬 엔진
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) 그리고 다른 Linux 매니저들 -
  이 문제가 풀리기를 바라는 커뮤니티가 있다는 증거

## 하는 일

**모드.** 단순 압축 파일, FOMOD 마법사, Wrye Bash BAIN 패키지, 나머지를 위한
수동 선택기 - 그리고 **root 모드를 네이티브로** (스크립트 익스텐더 프리로더,
ENB, Engine Fixes), Root Builder 플러그인 없이, 설치 폴더로 복사되는 것 하나
없이. 개별 파일 숨기기, 구분선으로 묶기, 지정 이동, 모드별 메모와 카테고리,
그리고 MO2 프로필 가져오기.

목록은 MO2의 것이고, 그 습관까지 그대로입니다: 선택 열 여덟 개와 그중 어느
것으로든 정렬, 카테고리별 또는 출처별 그룹화, 더블클릭 제스처, 타이핑으로
건너뛰기, 복원하기 전까지는 아무 일도 하지 않는 모드별 백업, 그리고 이 게임이
로드하지 못할 배치이거나 다른 게임용으로 받은 모드에 붙는 권고 플래그. 파일
트리는 평범한 작업을 합니다 - 새 폴더, 이름 변경, 삭제, 열기 - 그리고 아무것도
띄우지 않고 이미지와 텍스트를 미리 봅니다.

**플러그인.** LOOT 정렬이 내장된 로드 순서, 게임이 계산하는 방식 그대로의 모드
인덱스, 마스터 누락 경고, 그리고 DLC와 Creation Club 콘텐츠를 있는 그대로
관리되지 않는 행으로 표시.

**인스턴스.** 전역 - `~/.local/share/Colony/Eidos/instances/` 아래에서 중앙 관리 - 또는 휴대용:
원하는 곳 어디에나 두는 자족적 폴더(두 번째 드라이브, 게임 파티션), 옮길 수
있고 격리되어 있으며, MO2의 것과 같습니다. 휴대용 인스턴스는 세션을 넘어
기억됩니다; GUI도, Steam 실행도, 모든 CLI 명령도 마지막에 쓴 것을 따라가고,
게임 id를 받는 모든 명령은 그 폴더도 받습니다. 자세한 내용은
[usage.md](docs/guide/usage.md#인스턴스-전역과-휴대용).

**프로필.** 프로필별 모드 순서, 플러그인 상태, INI, 세이브. 세이브는 파싱되어
현재 플러그인과 비교되고 - 세이브가 필요로 하는 것을 켜주는 버튼과 함께 - 매
세션 뒤 Steam Cloud를 위해 다시 동기화됩니다.

**Nexus.** 계정을 연결하면 Mod Manager Download 링크를 받고, 업데이트를 확인하고, 제작자와 프로필을 볼 수 있습니다. 컬렉션은 레시피에 따라 설치되며 중단된 진행을 재개할 수 있습니다. 지원하지 않는 단계는 명확히 보고됩니다. 일치 여부는 정확한 원본 파일로 확인하며, 불완전한 예전 메타데이터는 미검증 상태로 표시됩니다. 다운로드 탭에서 압축 파일을 필터링, 정렬, 숨기기, 삭제할 수 있습니다. **오프라인** 스위치는 Nexus 접근을 중단합니다.

**도구.** xEdit, BodySlide, DynDOLOD 같은 것들은 게임의 Proton 프리픽스 안에서
*병합된 뷰를 통해* 실행됩니다 - 여러분의 모드를 보고, 결과물은 Overwrite에
떨어지며, 한 번의 클릭으로 진짜 모드가 됩니다. 각각이 필요로 하는 런타임은
요청할 때 받아오므로, DLL 누락은 오후 반나절이 아니라 버튼 하나입니다. xEdit과
그 QuickAutoClean 짝은 알아서 찾아 줍니다 - 게임 폴더 안, 모드 안, 또는 게임들
옆에 두는 도구 폴더에서 - 맞는 런타임까지 이미 골라서. 쓰는 것은 고정하고,
쓰지 않는 것은 숨기고, 자체 Steam 앱인 도구에는 자기
Steam AppID를 주고, Eidos를 전혀 열지 않고도 병합된 뷰를 통해 실행하는
`.desktop` 바로가기를 쓸 수 있습니다.

**진단.** 마스터 누락, 고아 압축 파일, 모드 목록 어긋남, 손상된 플러그인 집합 -
그리고 실행 뒤에는, 스크립트 익스텐더 자신의 로그가 실제로 무엇이 로드됐다고
말하는지.

**자기 파일을 두는 곳.** Colony 제품군의 모든 프로그램이 쓰는 배치입니다. 여러분이 고른 것 - 환경 설정, Nexus 세션,
인스턴스 목록, 직접 쓴 게임과 애드온 정의 - 은 `~/.config/Colony/Eidos/`에, 전역 인스턴스와 내려받은 런타임, 세션
로그는 `~/.local/share/Colony/Eidos/`에, LOOT 마스터리스트는 `~/.cache/Colony/Eidos/`에 둡니다.
예전 Eidos는 이것들을 `~/.config/eidos/`, `~/.local/share/eidos/`, `~/.local/state/`에
두었습니다; 업그레이드 후 첫 실행이 그것들을 옮기고, 로그에 그렇게 적고, 예전 폴더는 정확히 이전 그대로 둡니다. 옮기기가 끝나기 전까지
Eidos는 예전 파일을 계속 씁니다. 자세한 내용은
[internals/paths.md](../../../docs/internals/paths.md)(영어)를 참고하세요.

교체 시 보호, BSA/BA2 내부 파일 충돌, SKSE 정적 진단, 생성 기록 및 그 한계는 [무결성 검사와 생성된 파일](../../../docs/internals/integrity.md)(영어)을 참고하세요.

### 지원 게임

**Skyrim SE/AE** - 실제 플레이로 검증됨. **Fallout 4**도 끝에서 끝까지 연결되어
있습니다 (F4SE 자동 교체, 아카이브 무효화, 별표 로드 순서, LOOT, `.fos`
세이브) - [guide/fallout4.md](docs/guide/fallout4.md) 참고. 공유 게임
디스크립터에 따라 연결되어 있고 테스터를 찾는 것: Skyrim LE, Skyrim VR, Enderal SE, Fallout 3, Fallout NV,
Fallout 4 (+ VR), Starfield, Oblivion, Morrowind (뒤의 둘은 마운트하고 모드를
관리합니다; 타임스탬프로 정렬되는 플러그인 목록은 아직 관리하지 않습니다).

게임군 하나를 추가하는 것은 디스크립터 한 행입니다:
[internals/adding-games.md](../../../docs/internals/adding-games.md).

## 설치

### Colony를 통해 (권장)

[Colony](https://github.com/Project-Colony/Colony)에서 **Eidos**를 검색해
설치하세요. 업데이트는 런처를 통해 들어옵니다. Colony는 파일 하나,
`eidos-linux`를 설치합니다: 창이면서, `eidos`라는 이름의 링크를 통해 호출하면
`eidos` 명령줄이기도 합니다. NIF 모델 미리 보기에는 아래 번들에만 들어 있는
도우미가 필요합니다. 없으면 미리 보기가 그렇다고 알려 주고, 나머지는 모두
동작합니다. 설치 후에 할 일과 Steam이 이것을 가리키게 하는 법:
[docs/guide/install.md](docs/guide/install.md#colony를-통해).

### 바이너리 직접 내려받기

[최신 릴리스](https://github.com/Project-Colony/Eidos/releases/latest)에서
파일 하나를 받으세요:

| 파일 | 무엇인가 |
|---|---|
| `eidos-linux` | Colony가 설치하는 것과 같은, 파일 하나로 된 프로그램 전체 |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, NIF 미리 보기 도우미, 아이콘, 그리고 데스크톱 항목과 Nexus 다운로드 처리기를 등록하는 설치 프로그램 |

먼저 [코드 서명 정책](#코드-서명-정책)에 나온 대로 내려받은 파일을 확인한 다음:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

설치 프로그램은 sudo 없이 모든 것을 `~/.local/bin`에 넣습니다. `--system`과
`--bindir DIR`로 다른 곳을 고를 수 있습니다. 두 파일 모두 glibc 2.39 이상의
x86_64 Linux, FUSE(`fusermount3`), 그리고 모드 압축 파일용 7-Zip이 필요합니다.

### 소스에서 빌드

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Rust 1.89 이상이 필요하고(CI가 확인하는 `rust-version`입니다. CI와 릴리스는
1.99.0으로 빌드합니다), NIF 미리 보기 도우미를 위해 `just`, CMake 3.20 이상,
C++17 컴파일러도 필요합니다. Arch 패키지는 `packaging/PKGBUILD`에서
빌드됩니다. 사전 요구 사항과 `just` 없이 실행하는 명령:
[docs/guide/install.md](docs/guide/install.md).

### Steam 실행 옵션

Eidos는 게임의 실행 명령 *그 자체로* 동작하며, 그래서 게임이 시작되기 전에
마운트할 수 있습니다. 대부분의 구성에는 설치 위치의 절대 경로를 쓴 기본 한
줄이면 충분합니다(Steam은 여러분의 `PATH`를 읽지 않습니다):

```
~/.local/bin/eidos-gui %command%
```

Colony로 설치했다면 그 경로는 `~/.local/share/Colony/apps/Eidos/eidos-linux`입니다.
그다음 플레이를 누르세요: Eidos가 그 게임의 인스턴스로 열리고, Run이 병합된
뷰를 통해 게임을 시작합니다.

나머지는 전부 그 앞에 쌓는 환경 변수이고, 자유롭게 조합됩니다:

| 원하는 것... | 앞에 붙일 것 |
|---|---|
| Community Shaders와 함께 쓰는 DLSS | `PROTON_ENABLE_NVAPI=1` - 이것이 없으면 DLSS는 조용히 초기화되지 않습니다; 전체 점검 목록은 [guide/graphics.md](docs/guide/graphics.md) |
| 화면에 FPS 카운터 | `DXVK_HUD=fps` |
| 모드 없는 드라이버 수준 프레임 보간 (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - Community Shaders 자체의 프레임 생성과는 절대 함께 쓰지 마세요 |
| 버그 신고용 상세 로그 | `EIDOS_LOG=debug` (세션 로그는 `~/.local/share/Colony/Eidos/logs/`에 떨어집니다) |
| 마운트의 세션별 I/O 보고 | `EIDOS_FUSE_STATS=1` |
| 다른 FUSE 워커 수 | `EIDOS_FUSE_THREADS=8` (기본값 4; 동시성 버그를 쫓을 때 가장 먼저 시도할 것은 `1`입니다) |
| 이번 실행을 휴대용 인스턴스 하나에 고정 | `EIDOS_INSTANCE=/path/to/folder` - 이것이 없으면 Eidos는 마지막에 쓴 인스턴스를 여는데, 보통은 그게 원하는 바입니다 |

요즘의 모드 구성(Community Shaders, DLSS, 프레임 생성)에서 그대로 쓸 줄 -
이것은 예시가 아니라 최종 명령입니다:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

구성이 되는지 확인하는 동안에는 앞에 `DXVK_HUD=fps`를 붙이고, 되고 나면 빼세요.

더 깊은 진단 스위치(`EIDOS_FUSE_TRACE`, 캐시와 인덱스 이분 탐색 토글,
`EIDOS_FUSE_PASSTHROUGH`가 기본으로 꺼져 있는 이유)는
[guide/troubleshooting.md](docs/guide/troubleshooting.md)에 있습니다.

## 문서

모든 것은 [docs/](docs/)에 있고, [docs/README.md](docs/README.md)에서 읽는
사람을 기준으로 정리되어 있습니다: 사용자 안내서(설치, CLI와 GUI, xEdit과
DynDOLOD 같은 도구, 그래픽, 문제 해결), Eidos가 내부에서 어떻게 동작하는지,
그리고 왜 존재하는지. 보안 정책과 취약점 신고 방법:
[SECURITY.md](../../../SECURITY.md).

## 언어

플레이어에게 필요한 페이지는 번역되어 있습니다. **영어가 정본입니다**: 번역이
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

**나머지는 빠뜨린 것이 아니라 일부러 영어입니다.** `docs/internals/`와
`docs/project/`는 Rust 코드도 함께 읽는 사람들이 읽고, `CHANGELOG.md`는
생성됩니다. 그것들을 번역한다는 것은 필요로 하지 않는 독자를 위해 17,678 단어를
더 정직하게 유지한다는 뜻입니다.

번역마다 그것이 만들어진 영어 파일의 해시를 담고 있고, 영어가 앞서 나가면 CI가
실패합니다 - [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh) 참고. 다시 최신
상태로 되돌릴 수 없는 번역은 그대로 두지 않고 **삭제합니다**: 낡은 페이지도
여전히 권위 있어 보이고 지난달의 명령을 건네주는데, 독자에게는 그것이 영어로
보내지는 것보다 나쁩니다.

한 언어는 디렉터리 하나입니다. `docs/i18n/<lang>/`가 저장소 루트를 그대로
비추므로, `docs/i18n/de/docs/guide/install.md`는 독일어판 `docs/guide/install.md`입니다 -
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
**오프라인** 모드(Preferences -> Nexus)는 Nexus로 가는 모든 연결을 멈춥니다.

| 데이터 | 저장 또는 전송 | 어디에, 그리고 왜 |
|---|---|---|
| 환경 설정, 인스턴스 목록, 게임과 애드온 정의 | 저장 | `~/.config/Colony/Eidos/` |
| 인스턴스, 내려받은 런타임, 세션 로그 | 저장 | `~/.local/share/Colony/Eidos/`, 또는 휴대용 인스턴스에 고른 폴더 |
| LOOT 마스터리스트 | 저장 | `~/.cache/Colony/Eidos/loot/` |
| Nexus 로그인 | 전송 | 로그인하면 브라우저에서 `users.nexusmods.com`이 열리고, 그 결과는 Eidos가 그 로그인 동안만 `127.0.0.1`에 여는 수신기로 돌아옵니다. 그다음 Eidos는 `users.nexusmods.com/oauth/token`에서 그것을 토큰으로 바꾸고, Nexus가 토큰 서명에 쓰는 공개 키를 가져옵니다. [Nexus Mods 개인정보 처리방침](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus 자격 증명 | 저장 | 액세스 토큰과 갱신 토큰, `~/.config/Colony/Eidos/nexus.ini`에 모드 0600으로. 예전 Eidos가 거기 쓴 `api_key=` 줄은 남겨 두지만 절대 읽지 않습니다. 원하면 지우세요. |
| Nexus 요청 | 전송 | Nexus 기능을 쓸 때 `api.nexusmods.com`으로: 계정 이름과 프리미엄 여부, 모드와 파일 정보, 업데이트 확인, 컬렉션, 다운로드 링크. 모든 요청에는 액세스 토큰이 실리고 Eidos와 그 버전이 명시됩니다. |
| Nexus 다운로드 | 전송 | 요청한 파일을, 다운로드 링크가 가리키는 Nexus CDN 호스트에서 받아 인스턴스의 다운로드 폴더에 저장하고, 각 파일 옆에 MO2 형식의 `.meta`를 둡니다. |
| `nxm://` 링크 | 저장 | `install.sh`가 대신 실행해 주는 `eidos nxm --register`가 `~/.local/share/applications/eidos-nxm.desktop`을 쓰고 `xdg-mime`으로 그것을 `nxm://` 처리기로 지정하므로, 사이트의 Mod Manager Download 버튼이 링크를 Eidos에 넘깁니다. Arch 패키지는 같은 항목을 시스템 전체에 설치합니다. |
| LOOT 마스터리스트 | 전송 | 게임의 첫 LOOT 정렬 때와 업데이트를 요청할 때마다 `raw.githubusercontent.com/loot/...`에서. 여러분의 것은 아무것도 보내지 않습니다. [GitHub 개인정보 처리방침](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| 도구 런타임 | 전송 | 도구 사전 요구 사항을 설치하거나 모드가 필요로 할 때만: `builds.dotnet.microsoft.com`의 .NET 런타임(Eidos에 내장된 SHA-256과 일치하지 않으면 거부), 그리고 게임의 Proton 프리픽스에 설치할 것(Visual C++와 .NET 재배포 패키지)을 Microsoft와 winetricks가 쓰는 다른 호스트에서 내려받는 `winetricks`. [Microsoft 개인정보 처리방침](https://www.microsoft.com/en-us/privacy/privacystatement) |
| 세션 로그 | 저장 | `~/.local/share/Colony/Eidos/logs/`, 모드 0600: 인스턴스당 세션 열 개, 30일 뒤 정리. 홈 폴더는 `~`로, 사용자 이름은 `<user>`로 적히므로 로그를 그대로 이슈에 붙여 넣을 수 있습니다. 여러분이 보내지 않는 한 기기를 떠나지 않습니다. |
| 바로가기 | 저장 | `install.sh`가 `eidos.desktop`과 아이콘을 `~/.local/share` 아래에 씁니다. 여러분이 만든 도구 바로가기는 `~/.local/share/applications`로 갑니다. |
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
`d3dcompiler_43`, `d3dx9_43`, `d3dx11_43`, 32비트와 64비트)도 들어 있으며,
모드나 도구가 필요로 할 때 Eidos가 게임의 프리픽스에 배치합니다. 이것들은
Microsoft의 재배포 조건을 따릅니다. [출처](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md)를
참고하세요.

모드 관리는 모두의 것입니다.
