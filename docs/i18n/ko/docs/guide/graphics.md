<!-- eidos-i18n: source=docs/guide/graphics.md sha=ee3cee732aafbf91d8085d03a74f7d4e0cfa224d -->

# Community Shaders, DLSS, 프레임 생성

Community Shaders 1.4+는 자체 업스케일링(DLSS 4 / FSR 3.1 / XeSS, 별도 패키지
"Upscaling - Community Shaders"를 통해)과 FSR 3.1 프레임 생성을 제공합니다. 이
모든 것이 리눅스에서 Eidos를 통해 동작합니다 - CS와 그 패키지들은 평범한 모드로
설치되고, 병합된 뷰는 다른 파일과 똑같이 그 DLL을 제공합니다 - 그러나 게임 안에서는
**알아챌 수 없는** 세 가지가 있고, 각각이 기능을 조용히 아무 일도 하지 않게 만듭니다.
이 페이지가 그 목록이며, 실제 환경에서 호되게 배운 것입니다.

## DLSS에 필요한 실행 옵션

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton은 게임이 Valve의 허용 목록에 없으면 NVIDIA NVAPI 계층(dxvk-nvapi)을 끕니다.
그리고 Skyrim은 목록에 없습니다. 이것이 없으면 CS는 DLSS를 초기화하지 못하고 조용히
FSR 업스케일링으로 물러납니다. 화면에는 이유가 전혀 뜨지 않습니다. NVIDIA가 아닌
기기에서 이 변수를 설정해도 손해가 없으므로, 안심하고 쓸 실행 옵션은 위의 한 줄이면
충분합니다. 프레임 생성 자체는 FSR 3.1이며 NVAPI가 필요 없습니다. 필요한 것은 DLSS
업스케일러뿐입니다.

## 프레임 생성은 테두리 없는 창을 요구합니다

CS의 프레임 생성은 D3D12 표시 프록시 위에서 돌며, 독점 전체 화면을 아예 거부합니다.
`SkyrimPrefs.ini`의 `bFull Screen=1`은 그것이 결코 물리지 않는다는 뜻입니다 - 오류도,
메시지도 없이 그저 기본 프레임 속도뿐입니다. 튼튼한 해법은 SSE Display Tweaks로,
INI가 무엇이라 하든 엔진 수준에서 모드를 강제합니다:

```ini
[Render]
Fullscreen=false
Borderless=true
```

창은 똑같아 보입니다(네이티브 해상도의 테두리 없는 창). 달라지는 것은 엔진의 인식뿐이며,
CS가 확인하는 것이 바로 그 엔진의 인식입니다.

활성화 조건이 둘 더 있고, 실패 방식은 똑같이 조용합니다:

- **화면 주사율 120 Hz 이상**, 또는 CS의 업스케일링 설정에서
  `frameGenerationForceEnable`을 켜기. 프레임 생성은 표시 속도를 두 배로 만들기
  때문에, 결과를 보여줄 수 없는 화면에서는 CS가 작동을 거부합니다.
- **Upscaling 패키지 설치**(그 `Data/Shaders/Upscaling/` 트리에 Streamline과
  FidelityFX DLL이 들어 있습니다). 이것이 없으면 CS는 메뉴 항목만 보여주고 아무것도
  켜지 못합니다.

## Reflex의 프레임 상한이 출력을 목 조를 수 있습니다

CS의 Reflex 설정에는 자체 FPS 상한(`reflexFPSLimit`, `reflexUseFPSLimit`과 함께)이
있습니다. 예전 값으로 남아 있는 상한 - 우리 쪽은 오래전 조정에서 온 79였습니다 - 은
프레임 생성의 하류에 앉아, 그것이 만들어낸 프레임을 정확히 잘라냅니다. 기본 60이
120으로 배가되었다가 다시 79로 깎이면 "프레임 생성이 아무것도 안 한다"로 읽힙니다.
144 Hz 화면에서 통상적인 Reflex 상한은 약 138입니다. 생성된 출력이 사라진 것 같을 때
반드시 확인하세요. 독점 전체 화면 다음가는 두 번째 조용한 살인자입니다.

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
묶였고, 애초에 내리지 말았어야 할 결정을 되돌리는 데 실행 중 CPU 시간이
쓰였습니다.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946)는 이를
뒤집습니다. 어떤 셰이더 유형의 텍스처든 원하는 대로 설치하면, 각 표면이 올바른
것을 쓰도록 메시와 플러그인을 다시 씁니다. 예전 방식이 다루지 못하던 플러그인의
대체 텍스처 레코드까지 포함해서요. 보이는 텍스처 팩과 그저 불러와지기만 하는
팩의 차이이며, 섞인 구성 즉 현실의 모든 로드 오더에서 가장 크게 드러납니다.

Linux에서 실행하기 전에 두 가지:

- 인자 **`--ignore-mo2vfscheck`**가 필요합니다. 없으면 MO2를 탓하며 즉시
  종료합니다. Eidos가 제공합니다. 이 플래그가 무엇이고 왜 여기서 확인이
  실패하는지는 [도구](tools.md#pgpatcher에---ignore-mo2vfscheck가-필요한-이유)를 보세요;
- 메시를 수정하므로 모든 텍스처 모드를 설치한 **뒤**, 최종 메시를 보아야 하는
  DynDOLOD **앞**에 실행합니다. 나중에 텍스처를 바꾸면 이 순서로 둘 다 다시
  돌려야 합니다.

출력은 다른 도구와 마찬가지로 Overwrite로 가며, 클릭 한 번으로 모드가 됩니다.
그 모드에는 높은 우선순위를 주세요. 이기라고 만든 것입니다.

## 그 뒤에 숫자를 읽는 법

생성된 프레임은 표시 쪽에만 존재합니다. 엔진은 여전히 기본 속도로 시뮬레이션하고,
Havok도 기본 속도로 째깍이며, *엔진* 프레임을 세는 모든 것(CS 자신의 계수기 포함)은
화면이 ~120을 보여주는 동안에도 ~60을 계속 보고합니다. 이는 올바른 동작이지 고장 난
계수기가 아닙니다 - 엔진 자체의 프레임 속도를 올리는 것과 달리 프레임 생성이 물리적으로
안전한 이유가 바로 이것입니다. 화면에 숫자를 띄우고 싶다면 실행 옵션의
`DXVK_HUD=fps`가 하나 보여줍니다.

규칙 하나: 드라이버 수준 보간(NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`)과 CS의 프레임 생성은 서로 경쟁하는 기술입니다.
둘 중 하나만 쓰고, 결코 함께 쓰지 마세요.
