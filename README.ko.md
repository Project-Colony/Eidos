<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**여러분의 게임을 절대 건드리지 않는 네이티브 Linux 모드 매니저.**

</div>

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

**인스턴스.** 전역 - `~/.local/share/eidos` 아래에서 중앙 관리 - 또는 휴대용:
원하는 곳 어디에나 두는 자족적 폴더(두 번째 드라이브, 게임 파티션), 옮길 수
있고 격리되어 있으며, MO2의 것과 같습니다. 휴대용 인스턴스는 세션을 넘어
기억됩니다; GUI도, Steam 실행도, 모든 CLI 명령도 마지막에 쓴 것을 따라가고,
게임 id를 받는 모든 명령은 그 폴더도 받습니다. 자세한 내용은
[usage.ko.md](docs/guide/usage.ko.md#인스턴스-전역과-휴대용).

**프로필.** 프로필별 모드 순서, 플러그인 상태, INI, 세이브. 세이브는 파싱되어
현재 플러그인과 비교되고 - 세이브가 필요로 하는 것을 켜주는 버튼과 함께 - 매
세션 뒤 Steam Cloud를 위해 다시 동기화됩니다.

**Nexus.** 계정을 연결하면 사이트의 "Mod Manager Download" 버튼이 곧바로
인스턴스로 떨어지고, 설치되어 있는 것과 대조한 업데이트 확인, 각 모드를 만든
사람과 그 프로필 링크가 따라옵니다. **컬렉션** 링크는 그 구성원들을 여러분의
인스턴스와 대조해 나열합니다 - 설치됨, 다운로드됨, 없음 - 이는 컬렉션을
설치하는 것이 아니라 읽는 것이고, 창이 그 이유를 말해 줍니다. Downloads 탭은
압축 파일 라이브러리입니다: 필터, 정렬, 삭제하지 않고 숨기기, 이미 설치된 것
비우기. **오프라인** 스위치가 이 전부를 멈춥니다.

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

**자기 파일을 두는 곳.** 여러분이 고른 것 - 환경 설정, Nexus 세션, 인스턴스
목록, 직접 쓴 게임과 애드온 정의 - 은 `~/.config/Colony/Eidos/`에, 로그는
`~/.local/state/Colony/Eidos/` 아래에. Colony 제품군의 모든 프로그램이 쓰는
배치입니다. 예전 Eidos는 이것들을 `~/.config/eidos/`에 두었습니다; 업그레이드
후 첫 실행이 그것들을 옮겨 복사하고, 로그에 그렇게 적고, 예전 폴더는 정확히
이전 그대로 둡니다.

## 비교

| | Eidos | Wine 위의 MO2 | Fluorine-Manager | Limo / 링크 배포기 |
|---|---|---|---|---|
| 매니저가 네이티브로 실행 | ✅ | ❌ Wine 안의 Windows 앱 | ✅ (Qt 포팅) | ✅ |
| 게임 폴더 무손상 | ✅ 언제나 | ✅ | ✅ | ❌ 그 안에 링크를 씀 |
| 마운트가 보이는 대상 | 게임만 | 게임만 | **시스템 전체** | 해당 없음 |
| 크래시 후 정리 필요 | 설계상 없음 | 없음 | 죽은 마운트 복구 | 수동 해제 |
| root 모드 (ENB, 프리로더) | ✅ 네이티브 | 플러그인 필요 | 플러그인 필요 | 부분적 |
| 필요한 권한 | 없음 | 없음 | `/etc/fuse.conf` 수정 | 없음 |

## 얼마나 빠른가

| | 이전 | 지금 |
|---|---|---|
| 세이브 로딩 | 약 20초 | **6-7초** |
| 한 세션의 디렉터리 읽기 | 560만 회 | 46만 5천 회 |

셀 이동은 즉각적입니다. 이 이득은 모드에 질문을 덜 해서 나왔습니다: 파일 하나를
찾는 데 쉰 개 전부를 차례로 캐물었고, 폴더 하나를 나열하는 데 그것을 쉰 번씩
되풀이했습니다. 이제 둘 다 그러지 않습니다. 벤치마크가 아니라 평범하게 플레이한
실제 인스턴스에서 측정했습니다.

## 시작하기

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

그다음 게임의 Steam 실행 옵션을 `~/.local/bin/eidos-gui %command%`로 설정하고
플레이를 누르세요.

Arch 패키지와 릴리스 압축 파일, 먼저 설치해야 하는 것, 그리고 CLI 경로:
**[docs/guide/install.ko.md](docs/guide/install.ko.md)**.

## Steam 실행 옵션

기본 한 줄이면 대부분의 구성에는 충분합니다:

```
~/.local/bin/eidos-gui %command%
```

나머지는 전부 그 앞에 쌓는 환경 변수이고, 자유롭게 조합됩니다:

| 원하는 것... | 앞에 붙일 것 |
|---|---|
| Community Shaders와 함께 쓰는 DLSS | `PROTON_ENABLE_NVAPI=1` - 이것이 없으면 DLSS는 조용히 초기화되지 않습니다; 전체 점검 목록은 [guide/graphics.ko.md](docs/guide/graphics.ko.md) |
| 화면에 FPS 카운터 | `DXVK_HUD=fps` |
| 모드 없는 드라이버 수준 프레임 보간 (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - Community Shaders 자체의 프레임 생성과는 절대 함께 쓰지 마세요 |
| 버그 신고용 상세 로그 | `EIDOS_LOG=debug` (세션 로그는 `~/.local/state/Colony/Eidos/logs/`에 떨어집니다) |
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
[guide/troubleshooting.ko.md](docs/guide/troubleshooting.ko.md)에 있습니다.

## 다음에 갈 곳

| 하고 싶은 일... | |
|---|---|
| 설치하기 | [guide/install.ko.md](docs/guide/install.ko.md) |
| CLI와 GUI 익히기 | [guide/usage.ko.md](docs/guide/usage.ko.md) |
| xEdit, BodySlide, DynDOLOD 설정하기 | [guide/tools.ko.md](docs/guide/tools.ko.md) |
| Fallout 4 플레이하기 (F4SE, 버전, NVIDIA 파편 크래시) | [guide/fallout4.ko.md](docs/guide/fallout4.ko.md) |
| DLSS / 프레임 생성 작동시키기 (Community Shaders) | [guide/graphics.ko.md](docs/guide/graphics.ko.md) |
| 잘못돼 보이는 것 고치기 | [guide/troubleshooting.ko.md](docs/guide/troubleshooting.ko.md) |
| 왜 빠른지 알고 직접 확인하기 | [internals/performance.md](docs/internals/performance.md) |
| 내부에서 어떻게 동작하는지 이해하기 | [internals/architecture.md](docs/internals/architecture.md) |
| 빌드하고, 테스트하고, 기여하기 | [internals/contributing.md](docs/internals/contributing.md) |
| 애초에 왜 존재하는지 알기 | [project/landscape.md](docs/project/landscape.md) |

전체 색인은 [docs/README.ko.md](docs/README.ko.md)에, 보안 정책과 취약점 신고
방법은 [SECURITY.md](SECURITY.md)에 있습니다.

## 언어

플레이어에게 필요한 페이지는 번역되어 있습니다. **영어가 정본입니다**: 번역이
영어와 어긋나면, 영어 파일이 맞습니다.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**나머지는 빠뜨린 것이 아니라 일부러 영어입니다.** `docs/internals/`와
`docs/project/`는 Rust 코드도 함께 읽는 사람들이 읽고, `CHANGELOG.md`는
생성됩니다. 그것들을 번역한다는 것은 필요로 하지 않는 독자를 위해 17,678 단어를
더 정직하게 유지한다는 뜻입니다.

번역마다 그것이 만들어진 영어 파일의 해시를 담고 있고, 영어가 앞서 나가면 CI가
실패합니다 - [`scripts/i18n-check.sh`](scripts/i18n-check.sh) 참고. 다시 최신
상태로 되돌릴 수 없는 번역은 그대로 두지 않고 **삭제합니다**: 낡은 페이지도
여전히 권위 있어 보이고 지난달의 명령을 건네주는데, 독자에게는 그것이 영어로
보내지는 것보다 나쁩니다.

언어 하나를 추가하는 것은 파일 네 개와 이 표의 행 하나입니다;
[`docs/internals/contributing.md`](docs/internals/contributing.md)에 그 절차가
있습니다.

## 지원 게임

**Skyrim SE/AE** - 실제 플레이로 검증됨. **Fallout 4**도 끝에서 끝까지 연결되어
있습니다 (F4SE 자동 교체, 아카이브 무효화, 별표 로드 순서, LOOT, `.fos`
세이브) - [guide/fallout4.ko.md](docs/guide/fallout4.ko.md) 참고. 공유 게임
디스크립터에 따라 연결되어 있고 테스터를 찾는 것: Skyrim LE, Skyrim VR, Enderal SE, Fallout 3, Fallout NV,
Fallout 4 (+ VR), Starfield, Oblivion, Morrowind (뒤의 둘은 마운트하고 모드를
관리합니다; 타임스탬프로 정렬되는 플러그인 목록은 아직 관리하지 않습니다).

게임군 하나를 추가하는 것은 디스크립터 한 행입니다:
[internals/adding-games.md](docs/internals/adding-games.md).

## 선행 사례와 감사

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer)와
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos가 재현하는 의미론,
  그리고 그 동등성을 견주어 연구한 코드베이스
- [LOOT](https://loot.github.io/) - libloot을 통한 정렬 엔진
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) 그리고 다른 Linux 매니저들 -
  이 문제가 풀리기를 바라는 커뮤니티가 있다는 증거

## 라이선스

GPL-3.0-or-later. 모드 관리는 모두의 것입니다.
