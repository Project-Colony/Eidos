<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Eidos를 통한 Fallout 4

Eidos를 쓰면 Fallout 4에는 특별한 실행 옵션도, 이름을 바꾼 실행 파일도, 감싸는
스크립트도 필요 없습니다. 다른 모든 게임과 같은 실행 옵션을 씁니다.

## 실행 옵션

```
~/.local/bin/eidos-gui %command%
```

Steam이 Fallout 4에 대해 실행하는 대상은 `Fallout4Launcher.exe`이지 결코
`Fallout4.exe`가 아닙니다. 그래서 스크립트 익스텐더를 돌린다는 것은 사실 "어떻게
Steam이 다른 프로그램을 시작하게 만들까"라는 질문입니다. 흔한 답은 `%command%`를
bash로 다시 쓰는 것이거나:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

`f4se_loader.exe`를 `Fallout4Launcher.exe` 위에 덮어쓰는 것입니다. 후자는 게임이
업데이트될 때마다 Steam이 되돌려 놓고, 그 뒤로는 F4SE 없이 플레이하게 되지만
아무것도 그 사실을 알려주지 않습니다.

Eidos는 게임 서술자에 따라 그 교체를 직접 합니다. 창에서 게임을 시작할 때,
`f4se_loader.exe`가 게임 폴더, 켜 둔 모드의 `Root/`, 또는 `Overwrite/Root/`에 있으면
`Fallout4Launcher.exe`를 그것으로 바꿉니다. 로더가 없으면 `Fallout4.exe`를 실행하고,
상태 표시줄에 `f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
라고 표시됩니다. 어느 파일도 없으면 런처를 그대로 실행하고, 그 사실도 알려 줍니다.
실행 옵션을 `eidos play fallout4 -- %command%`로 쓰면 CLI는 로더가 있을 때만 교체하고,
없으면 `Fallout4Launcher.exe`를 실행합니다.

런처를 피해야 할 두 번째 이유가 있습니다. 런처는 `Data`를 다시 훑고 `plugins.txt`를
다시 써서, 방금 배치한 로드 순서를 되돌립니다.

창을 직접 열고 Run을 누르면 Eidos가 Steam에 게임 시작을 요청하고
(`steam -applaunch 377160`), Steam이 위의 실행 옵션을 통해 Eidos를 시작합니다.
인스턴스의 게임 실행 파일 옆에 `f4se_loader.exe`가 없으면 시작하지 않으며, 상태
표시줄이 먼저 스크립트 익스텐더를 설치하라고 안내합니다. Fallout 4 VR도 같은
방식으로 동작하고, `Fallout4VRLauncher.exe`와 `f4sevr_loader.exe`를 씁니다.

## Eidos가 대신 처리하는 것

| 영역 | Eidos가 하는 일 |
|---|---|
| 아카이브 무효화 | `Fallout4Custom.ini`에 `[Archive]` `bInvalidateOlderFiles=1`과 빈 `sResourceDataDirsFinal=`이 들어갑니다. `Data` 바깥의 낱개 파일이 애초에 보이게 해주는 두 개의 키입니다. 게임 폴더가 아니라 프로필에 기록됩니다. |
| INI | `Fallout4.ini`, `Fallout4Prefs.ini`, `Fallout4Custom.ini`는 프로필에 속합니다. Eidos로 실행한 BethINI Pie는 프로필의 사본을 편집하며, Backups -> INIs가 Eidos가 실행 전마다 보관해 둔 사본을 되돌립니다([tools.md](tools.md#bethini-pie)). |
| 로드 순서 | Fallout 4가 쓰는 별표 형식의 `plugins.txt`(`*`가 활성 표시). 암묵적인 Creation Club 플러그인에 대해서는 `Fallout4.ccc`를 따릅니다 |
| LOOT | 정렬은 Skyrim과 똑같이 동작합니다 - `eidos sort <instance>`가 `fallout4` 마스터리스트를 가져옵니다 |
| 세이브 | `.fos` 세이브와 그 `.f4se` 코세이브를 나열·복사하고 프로필별로 보관합니다. 상세 패널이 세이브 자체의 플러그인 표를 읽으므로, 당신이 꺼둔 플러그인을 필요로 하는 세이브는 불러오기 전에 그렇게 말해 줍니다. 프로필에서 가장 최근 세이브 세 개 중 하나가 쓰는 플러그인의 모드를 끄거나 제거할 때는 먼저 묻습니다. |
| Root 모드 | 모드가 실행 파일 옆에 두는 것들(F4SE 자체, ENB, `dxvk.conf`)은 Skyrim과 동일한 `Root/` 메커니즘으로 그곳에 놓입니다 |

## 버전 문제

Fallout 4는 2019년부터 2024년 사이의 그 얼어붙은 게임이 더는 아닙니다. 2026년 8월
현재 살아 있는 갈래가 셋이고, 한쪽을 위해 빌드된 모드 DLL은 다른 쪽에서 로드되지
않습니다:

| 갈래 | 버전 | F4SE |
|---|---|---|
| 클래식("old-gen") | 1.10.163 | 0.6.23 |
| 넥스트젠 | 1.10.984 | 0.7.2 |
| 애니버서리 / Creations | 1.11.137 ~ 1.11.240 | 0.7.4 ~ 0.7.9 |

모드 목록을 짜기 전에 알아둘 두 가지:

- 게임 루트에 `Creations/`와 `Mods/` 폴더가 있으면 1.11.x 계열입니다. Eidos의 세이브
  상세 패널도 세이브를 기록한 빌드를 보여줍니다. Fallout이 세이브에 써 넣고, Eidos가
  "Game build"로 드러냅니다.
- F4SE는 보통 Bethesda 업데이트 후 하루나 이틀 안에 나오지만, 대부분의 DLL 모드가
  오프셋을 해석하는 통로인 *Address Library for F4SE Plugins*는 자기 일정대로
  움직입니다. 그 사이에는 DLL 모드가 로드되지 않습니다. DLL이 없는 모드(텍스처,
  메시, 플러그인)는 영향을 받지 않으므로, 막 나온 패치는 모드 목록을 시작하기 좋은
  날이 아닙니다.

구성이 돌아가기 시작하면 게임 버전을 인스턴스에 고정하세요. Diagnostics 탭의 Game
files 카드에서 Freeze this version을 누르거나, `eidos stock-game fallout4 freeze`로
크기를 확인하고 `eidos stock-game fallout4 freeze --yes`로 복사합니다. Eidos는 스토어
설치본을 `<instance>/Stock Game`으로 복사하고 그 사본에서 게임을 실행하며, 시작은
여전히 Steam이 합니다. 이후 패치는 스토어 설치본만 바꾸고 사본은 바꾸지 않으므로,
설치한 DLL이 맞춰 둔 게임 버전이 유지됩니다. btrfs나 XFS에서는 사본이 스토어의 블록을
공유해, Steam이 파일을 바꾸기 전까지 공간을 거의 쓰지 않습니다. ext4에서는 게임
폴더만큼 큰 전체 복사입니다. Steam이 게임을 사용 중이라고 하거나 완전히 설치되지
않았다고 보고하는 동안에는 Eidos가 거부합니다. 자세한 내용은
[usage.md](usage.md#게임-파일-스토어-설치본-또는-사본)에 있습니다.

사본이 없다면 Fallout 4의 Steam 자동 업데이트를 끄세요(속성, 업데이트, "실행할
때만 이 게임 업데이트"). 그러지 않으면 다음 패치가 설치한 모든 DLL을 부숴 놓습니다.

## 하드웨어 참고: NVIDIA에서의 무기 파편 크래시

Fallout 4의 무기 파편 효과는 NVIDIA FleX 위에서 돌아갑니다. NVIDIA가 Pascal 세대
이후로 지원을 끊은 PhysX 파생물입니다. Turing 이후의 어떤 카드에서도 - GTX 16, RTX
20부터 RTX 50까지 - 게임이 죽습니다. 이것은 게임 자체의 결함이며 리눅스, Proton,
Eidos와는 무관합니다.

해결책은 둘, 어느 쪽이든 통합니다. 게임 설정에서 "Weapon Debris"를 끄거나,
*Weapon Debris Crash Fix*(Nexus 48078)를 설치하세요. 후자는 효과가 아니라 파편의
충돌을 끕니다.

## Fallout 4의 Health 카드

Diagnostics 탭은 다음 각 경우에 Health 카드를 보여줍니다.

- F4SE 플러그인이 `F4SE/Plugins`에 설치되어 있지만 `f4se_loader.exe`가 켜 둔 어느
  모드에도, 게임 폴더에도 없습니다. 게임은 그 플러그인 없이 시작됩니다.
- `f4se_loader.exe`는 설치되어 있지만 실행 중인 게임 버전용 `f4se_<version>.dll`
  (1.10.984라면 `f4se_1_10_984.dll`)이 없습니다. 로더가 오류와 함께 멈추며, 보통
  게임 업데이트 뒤에 일어납니다.
- 게임 실행 파일이나 그 Steam 빌드가, 오류 없이 끝난 마지막 Eidos 실행 이후 바뀌었습니다.
  이전 버전용으로 빌드된 F4SE 플러그인이 로드되지 않을 수 있습니다.
- F4SE보다 우선순위가 높은 모드가 F4SE 자체 스크립트의 낱개 `.pex` 사본을 갖고
  있습니다. 오래된 사본은 그것이 선언한 F4SE 함수를 망가뜨리고, 게임은 오류를
  보여주지 않습니다.
- 프로필의 `Fallout4Custom.ini`의 `[General]`에 `sTestFile1`부터 `sTestFile10`까지의
  줄이 있습니다. 하나라도 설정되어 있으면 게임은 그 플러그인을 로드하고
  `plugins.txt`를 무시합니다. Fix는 그 줄을 지우고 이전 파일을
  `Fallout4Custom.ini.pre-fix`로 남깁니다.
- 어떤 모드의 BA2 아카이브가 `.wav`나 `.xwm` 오디오를 압축해 담고 있습니다. 게임이
  그 소리를 재생할 때 크래시할 수 있습니다. 오디오를 압축하지 않고 아카이브를 다시
  패킹하세요. Fallout 4 VR도 같은 카드를 받습니다.
- 크래시 로거가 마지막 실행 중에 로그를 썼습니다. `Documents/My Games/Fallout4/F4SE`의
  Buffout 4 `crash-*.log`, 또는 `F4SE/Crashlogs`의 `.log`입니다. 카드는 예외와, 로그에
  나온 DLL과 플러그인 중 당신의 모드가 설치한 것을 알려 줍니다.
- Display 카드는 프로필의 `Fallout4Prefs.ini`에 설정된 창 크기와 모드를 보여주며,
  Apply는 `iSize W`, `iSize H`, `bFull Screen`, `bBorderless`를 거기에 씁니다.
- Nexus 요구 사항이 DLC를 지목합니다. Eidos는 DLC 이름의 플러그인을 찾지만 Fallout 4의
  DLC 플러그인은 이름이 다르므로(Far Harbor는 `DLCCoast.esm`) 이 카드는 조언입니다.
  직접 확인한 뒤 Ignore를 누르세요.

로더 카드 둘에는 <https://f4se.silverlock.org/>로 가는 Open the download page 버튼과
Install the downloaded archive 버튼이 있으며, 후자는 로더를 모드의 `Root/`에 넣습니다.
`eidos check fallout4`는 `.pex` 카드, Display 카드, Nexus 요구 사항 카드를 뺀 같은
결과를 출력합니다. 이 카드들은 창만 계산합니다.

## 뭔가 잘못돼 보인다면

일반적인 점검 목록은 [troubleshooting.md](troubleshooting.md)에 있습니다.
Fallout 고유의 첫 질문은 언제나 *실제로 어떤 실행 파일이 시작됐는가*입니다. 창에서
게임을 시작하면 전체 실행 명령이 인스턴스의 실행 로그에 기록되므로:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

거기에 `f4se_loader.exe`가 있으면 교체가 일어난 것입니다. `Fallout4.exe`가 있으면
F4SE가 Eidos가 찾을 수 있는 곳에 설치되어 있지 않다는 뜻입니다 - 그 자리는 게임 실행
파일 옆이고, 모드로 관리하는 구성에서는 어떤 모드의 `Root/` 디렉터리(또는 손으로
설치한 게임 폴더 자체)를 뜻합니다. `Fallout4Launcher.exe`가 있으면 Eidos가 루트 뷰에서
로더도 `Fallout4.exe`도 찾지 못한 것입니다.

올바른 실행 파일이 시작되는데도 게임이 크래시한다면, Test run과 Find the mod behind a
problem이 프로필의 세이브를 건드리지 않고 원인을 모드 하나로 좁혀 줍니다
([usage.md](usage.md#테스트-실행)).
