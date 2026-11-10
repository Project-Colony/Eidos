<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS, 프레임 생성

Community Shaders 1.4+는 자체 업스케일링(DLSS 4 / FSR 3.1 / XeSS, 별도 패키지
"Upscaling - Community Shaders"를 통해)과 FSR 3.1 프레임 생성을 제공합니다. 이
모든 것이 리눅스에서 Eidos를 통해 동작합니다 - CS와 그 패키지들은 평범한 모드로
설치되고, 병합된 뷰는 다른 파일과 똑같이 그 DLL을 제공합니다 - 그러나 게임 안에서는
확인할 수 없는 설정이 세 가지 있고, 각각이 오류 없이 기능을 아무 일도 하지 않게
만듭니다. NVAPI 변수, 독점 전체 화면, Reflex FPS 상한입니다.

Diagnostics 탭은 앞의 둘과 아래의 ENB 줄을, Wine prefix가 있는 게임(Steam의 Proton
prefix, 또는 Heroic이 만든 것)에 대해 확인합니다. Reflex 상한은 확인하지 않습니다.
인스턴스가 `Stock Game` 같은 게임 사본에서 게임을 실행하면 확인은 스토어 설치본이
아니라 그 사본의 파일을 읽습니다. `eidos check <instance>`는 같은 카드를 터미널에
Fix 버튼 없이 출력합니다.

- NVAPI 카드는 Skyrim SE나 Enderal SE의 Steam 사본에서, NVIDIA 드라이버가 로드되어
  있고(`/proc/driver/nvidia/version`이 있음), 병합된 `Data`의 `Shaders/Upscaling`에
  뭔가 있고, 게임의 Proton 빌드가 NVAPI를 기본으로 꺼 두며, Steam의 실행 옵션도 Eidos
  자체 환경도 프로필의 `launch-env.ini`도 `PROTON_ENABLE_NVAPI=1`을 설정하지 않았을 때
  나타납니다.
- 독점 전체 화면 카드는 Upscaling 패키지가 설치되어 있고, 프로필의 `SkyrimPrefs.ini`에
  `bFull Screen=1`이 있으며, `SKSE/Plugins/SSEDisplayTweaks.ini`가 `Fullscreen=false`와
  `Borderless=true`를 둘 다 설정하지 않았을 때만 나타납니다. Fix 버튼은 프로필의
  `SkyrimPrefs.ini`에 `bFull Screen=0`과 `bBorderless=1`을 씁니다. 이 확인은 Enderal
  SE에도 `SkyrimPrefs.ini`를 읽는데 Enderal SE 프로필에는 `EnderalPrefs.ini`가
  있으므로, Enderal SE에서는 아래의 Display 카드를 쓰세요.
- ENB 카드는 루트 뷰에서 우선 적용되는 `enblocal.ini`의 `[GLOBAL]`에
  `LinuxVersion=true`가 없는 모든 게임에서 나타납니다. Fix 버튼도 있습니다.

## DLSS에 필요한 실행 옵션

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

[install.md](install.md)처럼 자신의 `eidos-gui`나 `eidos-linux`의 절대 경로를
쓰세요. 카드가 설정할 줄을 알려 줍니다. 게임의 현재 실행 옵션 앞에
`PROTON_ENABLE_NVAPI=1`을 붙인 것이고, 실행 옵션이 없으면 실행 중인 Eidos의 경로로
만든 줄입니다.

NVIDIA NVAPI 계층(dxvk-nvapi)이 없으면 CS는 DLSS를 초기화하지 못하고, 화면에 이유가
전혀 뜨지 않은 채 FSR 업스케일링으로 물러납니다. Proton 빌드가 이 계층을 스스로
켜는지는 그 빌드의 `proton` 스크립트에 적혀 있고, Eidos는 거기의 `use_nvapi` 줄을
읽습니다. 끄기 스위치(`disablenvapi`)는 기본으로 켜져 있다는 뜻입니다. GE-Proton11-9와
Proton 11.0이 그 형태입니다(2026-11-09 확인). 켜기 스위치(`enablenvapi`)는 꺼져
있다는 뜻이며, 그런 빌드만 변수가 필요합니다. NVIDIA가 아닌 기기에서 변수를 설정해도
손해가 없습니다. 프레임 생성 자체는 FSR 3.1이며 NVAPI가 필요 없습니다. 필요한 것은
DLSS 업스케일러뿐입니다.

한 프로필에만 설정하려면 그 프로필의 `launch-env.ini`에 `env=PROTON_ENABLE_NVAPI=1`을
넣으세요(View -> INI editor의 마지막 탭).

## 프레임 생성은 테두리 없는 창을 요구합니다

CS의 프레임 생성은 D3D12 표시 프록시 위에서 돌며, 독점 전체 화면을 아예 거부합니다.
`SkyrimPrefs.ini`에 `bFull Screen=1`이 있으면 프레임 생성은 결코 켜지지 않습니다 -
오류도, 메시지도 없이 게임은 기본 프레임 속도에 머뭅니다. SSE Display Tweaks는 INI가
무엇이라 하든 엔진 수준에서 모드를 설정합니다:

```ini
[Render]
Fullscreen=false
Borderless=true
```

창은 똑같아 보입니다(네이티브 해상도의 테두리 없는 창). 달라지는 것은 엔진이 보고하는
모드뿐이며, CS가 확인하는 것이 바로 그 모드입니다.

Diagnostics의 Display 카드는 Skyrim SE, Enderal SE, Fallout 4에서 크기와 모드를 한
번에 설정합니다. 연결된 가장 큰 화면의 크기를 보여줍니다. Borderless window를 고르고
Apply를 누르면 프로필의 `SkyrimPrefs.ini`(Enderal SE는 `EnderalPrefs.ini`, Fallout 4는
`Fallout4Prefs.ini`)에 `iSize W`, `iSize H`, `bFull Screen=0`, `bBorderless=1`을 씁니다.
SSE Display Tweaks가 창을 설정하는 경우에는 Apply가 `Resolution`, `Fullscreen=false`,
`Borderless=true`도 Overwrite 안의 그 INI 사본(`SKSE/Plugins/SSEDisplayTweaks.ini`)에
쓰며, 모드 자신의 파일은 그대로 둡니다. Start the game at once when a store opens Eidos가
켜져 있어도(Preferences, General, Running a game), 프로필에 아직 창 크기가 없고 SSE
Display Tweaks가 창을 설정하지 않으면 창이 열리므로, 첫 시작 전에 크기를 정할 수
있습니다.

활성화 조건이 둘 더 있고, 실패 방식은 똑같이 조용합니다:

- 화면 주사율이 120 Hz 이상이거나, CS의 업스케일링 설정에서
  `frameGenerationForceEnable`을 켜야 합니다. 프레임 생성은 표시 속도를 두 배로
  만들기 때문에, 결과를 보여줄 수 없는 화면에서는 CS가 작동을 거부합니다.
- Upscaling 패키지가 설치되어 있어야 합니다(그 `Data/Shaders/Upscaling/` 트리에
  Streamline과 FidelityFX DLL이 들어 있습니다). 이것이 없으면 CS는 메뉴 항목만
  보여주고 아무것도 켜지 못합니다.

## Reflex의 프레임 상한이 출력을 목 조를 수 있습니다

CS의 Reflex 설정에는 자체 FPS 상한(`reflexFPSLimit`, `reflexUseFPSLimit`과 함께)이
있습니다. 예전 값으로 남아 있는 상한 - 우리 쪽은 오래전 조정에서 온 79였습니다 - 은
프레임 생성의 하류에 앉아, 그것이 만들어낸 프레임을 정확히 잘라냅니다. 기본 60이
120으로 배가되었다가 다시 79로 깎이면 "프레임 생성이 아무것도 안 한다"로 읽힙니다.
144 Hz 화면에서 통상적인 Reflex 상한은 약 138입니다. 생성된 프레임이 빠진 것 같을 때는
언제나 확인하세요.

## 알려진 상호작용: SSE Display Tweaks와 검은 화면

FG + Display Tweaks + DXVK 조합에는 알려진 검은 화면 결함이 있습니다. 순서대로:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. 그것으로 부족하면 게임 실행 파일 옆에 `dxvk.conf`(모드의 `Root/` 디렉터리가 그곳에
   놓아 줍니다)를 두고
   `dxvk.enableGraphicsPipelineLibrary = False`

## 텍스처 모드가 섞인 경우: PGPatcher

Community Shaders는 parallax, complex material, PBR을 그릴 수 있지만 메시가 그에
맞게 설정된 곳에서만 가능합니다. 예전에는 미리 패치된 메시를 설치하고, 텍스처가
없는 곳에서는 효과를 다시 끄는 모드를 얹어야 했습니다. 적용 범위는 가진 메시에
묶였고, 설치 시점에 내린 결정을 되돌리는 데 실행 중 CPU 시간이 쓰였습니다.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946)는 이를
뒤집습니다. 어떤 셰이더 유형의 텍스처든 원하는 대로 설치하면, 각 표면이 올바른
것을 쓰도록 메시와 플러그인을 다시 씁니다. 예전 방식이 다루지 못하던 플러그인의
대체 텍스처 레코드까지 포함해서요. 이것이 없으면 메시가 요청하지 않는 셰이더가 필요한
텍스처는 불러와지기만 하고 의도대로 그려지지 않으며, 텍스처 유형이 섞인 구성에서
가장 흔합니다.

Linux에서 실행하기 전에 알아둘 두 가지:

- `--ignore-mo2vfscheck` 인자가 필요합니다. 없으면 MO2를 탓하며 즉시 종료합니다.
  Eidos가 제공합니다. 이 플래그가 무엇이고 왜 여기서 확인이 실패하는지는
  [도구](tools.md#pgpatcher에---ignore-mo2vfscheck가-필요한-이유)를 보세요;
- 메시를 수정하므로 모든 텍스처 모드를 설치한 뒤, 최종 메시를 보아야 하는 DynDOLOD
  앞에 실행합니다. 나중에 텍스처를 바꾸면 이 순서로 둘 다 다시 돌려야 합니다.

출력은 Overwrite로 가지 않습니다. PGPatcher는 런처에서 지정한 Output location에만
쓰며, `Data` 안의 폴더는 거부합니다. 그 위치는 전용 폴더로 지정하세요. 예를 들면
인스턴스의 `mods/` 아래에 새로 만든 `PGPatcher Output` 폴더입니다. PGPatcher가
실행되는 동안에는 그 모드를 비활성화해 두세요. PGPatcher는 다시 쓰기 전에 폴더를
비우고, Eidos는 뷰가 마운트된 동안 활성화된 모드가 바뀌지 않는다고 가정하기
때문입니다. 끝나면 높은 우선순위로 활성화하세요. 이기라고 만든 것입니다.

PGPatcher의 항목에 Capture output into 모드를 지정한 다음 그 모드를 Output location으로
쓰지 마세요. Eidos가 실행이 끝날 때마다 PGPatcher 자신의 `cfg/`와 `log/`를 그 모드로
옮기므로, PGPatcher는 "Output directory has non-PGPatcher related files"라며
멈춥니다.

## Proton에서의 ENB: LinuxVersion

Jackify는 Proton에서 설치하는 모든 ENB의 `enblocal.ini` `[GLOBAL]` 섹션에 `LinuxVersion=true`를 씁니다. ENB 자체 문서에는 이 키에 대한 설명이 없습니다. Diagnostics 탭의 Health 카드는 이 줄이 없는 `enblocal.ini`를 알려 주고, Fix 버튼은 우선 적용되는 `enblocal.ini`의 복사본에 이 줄을 설정해 `overwrite/Root/`에 씁니다. 그래서 ENB 모드는 자기 파일을 그대로 유지합니다.

또 다른 카드는 ENB가 게임 폴더에 있는데(`enblocal.ini`, `enbseries.ini`, 또는
`enbseries/` 폴더) 켜 둔 모드가 Community Shaders(`SKSE/Plugins/CommunityShaders.dll`)를
제공할 때 알려 줍니다. 둘은 함께 돌 수 있습니다. 화면이 이상해 보이면 둘 중 하나를
해당 INI에서 끄세요.

## 그 뒤에 숫자를 읽는 법

생성된 프레임은 표시 쪽에만 존재합니다. 엔진은 여전히 기본 속도로 시뮬레이션하고,
Havok도 기본 속도로 째깍이며, *엔진* 프레임을 세는 모든 것(CS 자신의 계수기 포함)은
화면이 ~120을 보여주는 동안에도 ~60을 계속 보고합니다. 계수기는 맞고, 엔진 자체의
프레임 속도를 올리는 것과 달리 프레임 생성이 물리적으로 안전한 이유가 바로 이것입니다.
화면에 숫자를 띄우고 싶다면 실행 옵션의 `DXVK_HUD=fps`가 하나 보여줍니다.

규칙 하나: 드라이버 수준 보간(NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`)과 CS의 프레임 생성은 서로 경쟁하는 기술입니다.
둘 중 하나만 쓰고, 결코 함께 쓰지 마세요.
