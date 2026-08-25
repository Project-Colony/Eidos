<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Eidos를 통한 Fallout 4

Fallout 4에는 특별한 실행 옵션도, 이름을 바꾼 실행 파일도, 감싸는 스크립트도 필요
없습니다. 이 말을 분명히 해둘 가치가 있습니다. 다른 모든 리눅스 F4SE 안내서는 반대로
말하고 있고, 그 조언은 다음 Steam 업데이트에서 부서지기 때문입니다.

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
업데이트될 때마다 Steam이 조용히 되돌려 놓고, 그 뒤로는 F4SE 없이 플레이하게 되지만
아무것도 그 사실을 알려주지 않습니다.

Eidos는 게임 서술자에 따라 그 교체를 직접 합니다. `f4se_loader.exe`가 설치되어
있으면 런처를 그것으로 바꾸고, 없으면 `Fallout4.exe`로 물러서며, **물러섰다는 사실을
알려 줍니다**. F4SE 모드가 전부 죽은 채로 시작되는 게임은 아예 시작되지 않는 게임보다
나쁩니다.

런처를 절대 실행하지 말아야 할 두 번째 이유가 있습니다. 런처는 `Data`를 다시 훑고
`plugins.txt`를 다시 써서, 방금 배치한 로드 순서를 되돌립니다. Eidos는 그것을 결코
실행하지 않습니다.

## Eidos가 대신 처리하는 것

| | |
|---|---|
| 아카이브 무효화 | `Fallout4Custom.ini`에 `[Archive]` `bInvalidateOlderFiles=1`과 빈 `sResourceDataDirsFinal=`이 들어갑니다. `Data` 바깥의 낱개 파일이 애초에 보이게 해주는 두 개의 키입니다. 게임 폴더가 아니라 프로필에 기록됩니다. |
| 로드 순서 | Fallout 4가 쓰는 별표 형식의 `plugins.txt`(`*`가 활성 표시). 암묵적인 Creation Club 플러그인에 대해서는 `Fallout4.ccc`를 따릅니다 |
| LOOT | 정렬은 Skyrim과 똑같이 동작합니다 - `eidos sort <instance>`가 `fallout4` 마스터리스트를 가져옵니다 |
| 세이브 | `.fos` 세이브와 그 `.f4se` 코세이브를 나열·복사하고 프로필별로 보관합니다. 상세 패널이 세이브 자체의 플러그인 표를 읽으므로, 당신이 꺼둔 플러그인을 필요로 하는 세이브는 불러오기 전에 그렇게 말해 줍니다 |
| Root 모드 | 모드가 실행 파일 옆에 두는 것들(F4SE 자체, ENB, `dxvk.conf`)은 Skyrim과 동일한 `Root/` 메커니즘으로 그곳에 놓입니다 |

## 버전 문제

Fallout 4는 2019년부터 2024년 사이의 그 얼어붙은 게임이 더는 아닙니다. 2026년 8월
현재 살아 있는 갈래가 셋이고, 한쪽을 위해 빌드된 모드 DLL은 다른 쪽에서 로드되지
않습니다:

| 갈래 | 버전 | F4SE |
|---|---|---|
| 클래식("old-gen") | 1.10.163 | 0.6.23 |
| 넥스트젠 | 1.10.984 | 0.7.2 |
| 애니버서리 / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

모드 목록을 짜기 전에 알아둘 만한 두 가지 결과:

- **실제로 무엇을 갖고 있는지 확인하세요.** 게임 루트에 `Creations/`와 `Mods/`
  폴더가 있으면 1.11.x 계열입니다. Eidos의 세이브 상세 패널도 그것을 기록한 빌드를
  보여줍니다 - Fallout이 세이브에 써 넣고, Eidos가 "Game build"로 드러냅니다.
- **막 나온 패치는 시작하기 좋은 날이 아닙니다.** F4SE는 보통 Bethesda 업데이트 후
  하루나 이틀 안에 나오지만, 대부분의 DLL 모드가 오프셋을 해석하는 통로인
  *Address Library for F4SE Plugins*는 자기 일정대로 움직입니다. 그 사이 생태계의 DLL
  절반은 쓰러져 있습니다. DLL이 없는 모드(텍스처, 메시, 플러그인)는 영향을 받지
  않습니다.

구성이 돌아가기 시작하면 Fallout 4의 Steam 자동 업데이트를 끄세요(속성 → 업데이트 →
"실행할 때만 이 게임 업데이트"). 그러지 않으면 다음 패치가 설치한 모든 DLL을
부숴 놓습니다.

## 하드웨어 참고: NVIDIA에서의 무기 파편 크래시

Fallout 4의 무기 파편 효과는 NVIDIA FleX 위에서 돌아갑니다. NVIDIA가 Pascal 세대
이후로 지원을 끊은 PhysX 파생물입니다. Turing 이후의 어떤 카드에서도 - GTX 16, RTX
20부터 RTX 50까지 - 게임이 죽습니다. 이것은 게임 자체의 결함이며 리눅스, Proton,
Eidos와는 무관합니다.

해결책은 둘, 어느 쪽이든 통합니다. 게임 설정에서 "Weapon Debris"를 끄거나,
*Weapon Debris Crash Fix*(Nexus 48078)를 설치하세요. 후자는 효과가 아니라 파편의
충돌을 끕니다.

## 뭔가 잘못돼 보인다면

일반적인 점검 목록은 [troubleshooting.ko.md](troubleshooting.ko.md)에 있습니다.
Fallout 고유의 첫 질문은 언제나 *실제로 어떤 실행 파일이 시작됐는가*입니다. Eidos는
전체 실행 명령을 인스턴스의 실행 로그에 기록하므로:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

거기에 `f4se_loader.exe`가 있으면 교체가 일어난 것입니다. `Fallout4Launcher.exe`가
있으면 F4SE가 Eidos가 찾을 수 있는 곳에 설치되어 있지 않다는 뜻입니다 - 그 자리는
게임 실행 파일 옆이고, 모드로 관리하는 구성에서는 어떤 모드의 `Root/` 디렉터리(또는
손으로 설치한 게임 폴더 자체)를 뜻합니다.
