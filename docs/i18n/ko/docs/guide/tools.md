<!-- eidos-i18n: source=docs/guide/tools.md sha=7f872c8fb702f4417931831cc14a48ffb9d9e30b -->

# 도구: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Eidos를 통해 실행된 도구는 게임 자신의 Proton 프리픽스 안에서 **병합된 뷰**를
봅니다. 게임이 읽게 될 것 - 활성화된 모든 모드를 우선순위 순서대로 - 을 그대로
읽고, 그것이 쓰는 것은 무엇이든 Overwrite로 떨어져, 클릭 한 번이면 진짜 모드가
됩니다.

## Eidos가 스스로 찾아내는 것들

어떤 도구들은 이름이 충분히 고유해서 선언하는 대신 찾아낼 수 있고, xEdit이
대표적인 경우입니다: Fallout 4는 `FO4Edit.exe`, Skyrim SE는 `SSEEdit.exe`,
원작은 `TES5Edit.exe`, 이런 식입니다 - 각각의 **QuickAutoClean** 쌍둥이도
함께이고, 그것이 LOOT가 계속 경고하는 dirty edit을 위한 버튼입니다. Eidos는
이것들을 파일 이름으로 다음에서 찾습니다:

- 게임 설치 폴더, 그리고 활성화된 모드의 `Root/` 트리;
- **이 인스턴스의 `mods/`** - MO2 사용자가 도구를 설치하는 곳입니다;
- Preferences에서 지정한 **tools folder** (General -> Tools), 인스턴스끼리 공유하는
  디렉터리용 - `/mnt/Games/Tools` 같은 것.

자주 쓰는 생성기도 같은 방식으로 찾습니다. 각각 지원하는 게임에서만 대상이 됩니다. 대상은 Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen, Synthesis입니다.

`PGPatcher.exe`도 Creation Engine 게임에서 같은 방식으로 발견되며, 없으면 동작할 수 없는 인자와 함께 등록됩니다 - [아래](#pgpatcher에---ignore-mo2vfscheck가-필요한-이유) 참조.

목록은 게임별이라, Skyrim 인스턴스에 Fallout의 에디터가 제시되는 일은 없습니다.
검색은 네 단계 아래에서 멈추는데, 모드 풀은 수십만 개의 파일이고 이 작업은 도구
목록을 만들 때마다 실행되기 때문이며, 심볼릭 링크는 따라가지 않습니다. 이렇게
찾아낸 도구는 직접 입력한 것과 정확히 같은 방식으로 구성됩니다: 런타임은 그
이름에서 오고, 아래의 모든 것과 같은 규칙을 따릅니다.

도구가 다른 곳에 있거나 다른 인자를 주고 싶다면 직접 추가하십시오 - 같은 제목의
사용자 항목이 자동으로 찾아낸 것을 덮어씁니다.

## LOOT가 dirty로 표시한 플러그인 정리하기

LOOT로 정렬하고 나면 Diagnostics 탭이 LOOT가 dirty로 표시한 플러그인을 나열합니다. xEdit와 LOOT 정리 데이터가 모두 있는 게임이 대상입니다(Skyrim LE, SE, VR, Enderal SE, Fallout 3, New Vegas, Fallout 4와 4 VR, Oblivion, Starfield). Eidos가 xEdit를 찾으면(Executables에 등록되어 있거나 인스턴스 안에서 도구를 찾는 곳 어디에든 있으면) 카드에 Clean all이 나타나고, dirty 플러그인마다 행 메뉴에 Clean with QuickAutoClean이 나타납니다. 찾지 못하면 카드가 Executables에 xEdit를 추가하라고 안내합니다. Eidos는 xEdit를 내려받지 않습니다. Eidos는 xEdit의 QuickAutoClean 모드를 병합된 뷰를 통해 `-qac -autoexit -autoload`와 함께 한 번에 플러그인 하나씩 실행합니다. xEdit는 정리한 플러그인을 불러온 플러그인 위에 저장하는데, 병합된 뷰에서는 이것이 Overwrite에 생기는 새 파일입니다. Eidos는 이 파일을 `Cleaned plugins`라는 모드로 옮기며, 이 모드는 활성화된 상태로 목록 맨 아래에 만들어져 모든 충돌에서 이깁니다. 원래 모드 안의 플러그인이나 게임의 Data 폴더 안의 플러그인에는 절대 쓰지 않습니다. 정리를 되돌리려면 `Cleaned plugins`를 비활성화하거나 그 안의 파일 하나를 숨기십시오.

xEdit의 백업, 캐시, 로그는 병합된 뷰 바깥인 `<instance>/logs/qac-<time>/`에 들어가며, 상태 줄은 그 로그에서 읽은 개수를 보여 줍니다. 정리가 성공하면 Eidos는 이 폴더를 지우고, 실패하면 남겨 둡니다. Eidos는 각 사본을 만든 원본 플러그인의 크기와 날짜를 `Cleaned plugins/meta.ini`에 기록합니다. 그 플러그인이 바뀌면 Health 탭이 사본이 오래되었다고 알리고 숨기기를 제안합니다.

## 하나 추가하기

GUI에서: **Tools -> Executables**, 그다음 Add. 명령줄에서:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # exe를 등록하지 않고 한 번만 실행
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>`는 도구 자체 설정과 관계없이 해당 실행의 출력을 `mods/<mod>`에 담습니다.

script extender, 게임 바이너리, 런처는 자동으로 감지됩니다; 등록이 필요한 것은
추가 도구뿐입니다.

### 실제 파일이 있는 자리를 가리키게 하십시오

실행 파일은 실제로 놓여 있는 자리에 등록하십시오. 도구가 모드로 설치되었다면
그것은 모드 폴더 안입니다:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(이것은 전역 인스턴스의 경로입니다 - 휴대용 인스턴스에서는 같은 규칙이 자기 폴더
아래 `<instance>/mods/...`에 적용됩니다; 이런 절대 경로는 나중에 휴대용 폴더를
옮길 때 살아남지 못하는 유일한 것임에 유의하십시오).

Eidos는 실행하기 전에 그 경로를 병합된 쪽으로 다시 씁니다. 그래서 도구는
`<game>/Data/CalienteTools/BodySlide/`에서 실행되고 거기서 다른 모든 모드의
파일도 봅니다. 이것은 들리는 것보다 중요합니다: BodySlide는 **비어 있는**
`SliderSets` 디렉터리를 배포하고, 그것이 만들 수 있는 모든 바디는 CBBE와 복장
모드에서 옵니다. 자기 모드 폴더에서 실행되면 아무것도 찾지 못하고 고장난 것처럼
보입니다.

MO2도 같은 이유로 같은 재작성을 합니다 - 그쪽 주석은 FNIS를 이름으로 듭니다.

**비활성화된** 모드 안의 도구는 다시 쓸 수 없는데, 그 파일들이 뷰에도 없기
때문입니다. Eidos는 그렇다고 말하고, 아닌 척하는 대신 자기 폴더에서 실행합니다.

## Mod Organizer 2 목록의 도구

Wabbajack 목록이나 Eidos 인스턴스로 연 다른 MO2 인스턴스에는 `ModOrganizer.ini`가 남아 있고, 거기에 설정된 실행 파일을 `tools.ini`로 복사할 수 있습니다.

```sh
eidos tool /path/to/list import-mo2           # 추가될 내용을 표시
eidos tool /path/to/list import-mo2 --write   # 추가
```

창에서는 실행 파일 대화상자에 **ModOrganizer.ini에서 가져오기**가 있으며, 항목마다 체크를 해제할 수 있습니다. 저장을 누르기 전에는 아무것도 저장되지 않습니다.

`Z:`의 경로는 Linux 경로이므로 그대로 둡니다. 다른 드라이브 문자의 경로는 목록 폴더 이름이 들어 있으면 그 폴더 아래로 옮겨집니다(`D:\Lists\MyList\tools\SSEEdit.exe`는 `<list>/tools/SSEEdit.exe`가 됩니다). 실행 파일, 작업 디렉터리, 인수 모두 해당합니다. 스크립트 확장기의 로더와 게임 실행 파일은 단순한 이름이 되어, 기본 제공 항목처럼 Eidos가 뷰를 얹는 게임 폴더에서 실행됩니다. 다른 곳을 가리키는 항목(Windows 시스템 프로그램, 목록 밖의 도구)은 이유와 함께 건너뜀으로 표시되며, `tools.ini`에 이미 있는 제목은 절대 바꾸지 않습니다.

## 도구의 출력을 자기 모드로 보내기

생성기 - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - 는 수백 개의 파일을
씁니다. 기본적으로 그것들은 나머지 전부와 함께 Overwrite로 떨어집니다.
Executables 편집기에서 **Capture output into**를 지정하면 이번 실행의 출력이
대신 그 모드로 들어갑니다:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide, Synthesis는 캡처 모드가 미리 지정된 채로 추가됩니다. `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output`, `Synthesis Output`입니다. DynDOLOD, TexGen, xLODGen은 사용자가 지정한 출력 폴더에 기록하므로 지정되지 않습니다.

모드가 없으면 만들어집니다. 이번 실행이 만들어낸 파일만 옮겨집니다; 이미
Overwrite에 있던 것은 그대로 남으므로, 캡처 대상이 있는 도구 둘이 서로의 출력을
훔치지 않습니다. 아무것도 쓰지 않은 실행은 빈 모드를 남기지 않습니다.

이 일은 쓰기 계층을 그 모드로 향하게 하는 대신 실행이 끝난 뒤에 이루어지는데,
MO2는 앞쪽 방식입니다. 쓰기 계층을 모드로 향하게 하면 실행 내내 그 모드가 최상위
우선순위로 올라가 - 그것이 끼어 있는 모든 충돌을 뒤집었다가 나중에 되돌리게 되고
- copy-up 없이 그 모드 자신의 파일을 그대로 관통해 씁니다. 캡처는 둘 중 어느
것도 없이 같은 최종 상태에 도달합니다.

대상 모드가 비활성화되어 있으면 출력은 그래도 쓰이지만 게임은 그것을 보지
못하므로, 도구는 다음 실행 때 같은 파일을 다시 만들어내게 됩니다. Eidos는 그런
경우 경고합니다.

### 출력이 오래되었을 때

Eidos는 도구의 출력이 어떤 모드, 플러그인 순서, 프로필, 실행 파일로 만들어졌는지 기록합니다. 그중 하나가 바뀌면 Diagnostics 탭이 알려 주고, 카드에는 도구를 현재 설정대로 실행하는 **Run <tool> again** 버튼이 나옵니다.

오래된 출력이 게임을 망가뜨리는 생성기(Pandora, Nemesis 등)는 `tools.ini`의 해당 항목에 `check_before_play=1`을 추가하거나, Executables 대화상자에서 **Ask before playing when its output is out of date**를 켜세요. 그러면 Run이 먼저 묻습니다. 도구를 실행할지, 그대로 플레이할지입니다. 그대로 플레이를 고르면 모드, 플러그인, 프로필이 다시 바뀌거나 창이 닫힐 때까지 기억됩니다. Eidos가 도구를 스스로 시작하는 일은 없습니다.

## 도구에 필요한 DLL은 그 이름으로 정해집니다

이건 놀라운 대목이라 분명히 말해둘 값어치가 있습니다: **도구에 붙이는 제목이
Eidos가 그 도구에 어떤 런타임 사전 요구 사항을 마련할지 결정합니다.** 일치는
제목의 대소문자 구분 없는 부분 문자열입니다.

| 제목에 이것이 들어 있으면 | Eidos가 요청하는 것 |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| 그 밖의 모든 것 | 없음 |

그래서 **`BodySlide`**로 등록한 도구는 DirectX DLL을 받습니다; 같은 실행 파일을
**`BS`**로 등록하면 아무것도 받지 못하고, DLL에 대해서는 한마디도 하지 않는 오류와
함께 시작하지 못할 수 있습니다. 도구 이름은 프로그램 이름을 따르십시오.

목록은 `default_prereqs`(`crates/eidos-instance/src/tools.rs`)에 있고,
Executables 대화상자의 `Prereqs` 항목은 편집할 수 있습니다 - 감지는 기본값이지
규칙이 아닙니다.

제목은 **인자**도 같은 방식으로 결정합니다. 누구도 짐작할 수 없는 인자가
필요한 유일한 도구를 위해서입니다. `pgpatcher`(또는 옛 이름 `parallaxgen`)를
포함한 제목에는 `--ignore-mo2vfscheck`가 주어집니다. 그 이유와, 그것이 선택
사항이 아닌 이유는 [아래](#pgpatcher에---ignore-mo2vfscheck가-필요한-이유)에 있습니다.

### 사전 요구 사항의 세 종류

**Tier 1 - 번들 DLL** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). Eidos가 함께
배포하며 실행할 때 프리픽스로 복사합니다. 할 일 없음, 네트워크 없음.

**Tier 2 - winetricks verb** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). 이것들은 레지스트리 키와 GAC와 CLR 호스트를 쓰기 때문에
파일 복사로는 되지 않습니다. **Microsoft에서 내려받습니다**.

**Tier 3 - 런타임** (`dotnet10`, `dotnetsdk10`). 요즘 .NET 런타임은 자기 디렉터리에 사는 193개의
파일이고 `DOTNET_ROOT`를 통해 찾습니다: 등록되지도 않고 프리픽스에 설치되지도
아예 않으므로, 다른 두 계층 어느 쪽도 그것을 실어 나를 수 없습니다. Eidos가 직접
내려받아 바이너리에 박혀 있는 체크섬으로 확인하고
`~/.local/share/Colony/Eidos/runtimes/`에 캐시합니다 - **어느 인스턴스에도 속하지
않는 바깥**인데, 78 MB는 게임별도 프로필별도 아니기 때문입니다.

`dotnetsdk10`은 .NET 10 SDK로, 내려받는 크기는 301 MB이고 풀면 808 MB입니다. Synthesis는 패처를
컴파일하는 데 이것이 필요하고, 패처를 빌드하고 실행할 때 경로 없는 `dotnet`을 호출합니다. 그래서 `dotnetsdk10`을 선언한
도구는 `WINEPATH`에 SDK 폴더를 받고, 그와 함께 `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false`, `DOTNET_CLI_TELEMETRY_OPTOUT=1`도
받습니다.
패처를 빌드하면 MSBuild 노드와 VBCSCompiler 서버가 계속 실행된 채 남고, Wine에서는 이들이 프리픽스의 wineserver를
살아 있게 하므로, 도구는 `MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0`,
`UseSharedCompilation=false`도 받습니다. `DOTNET_ROLL_FORWARD=Major` 덕분에 .NET 8용으로
빌드된 패처(Mutagen 0.54 이전)는 .NET 8 런타임을 찾지 못하면 SDK의 .NET 10 런타임에서 실행됩니다.
`tools.ini`의 Synthesis 항목이 아직 예전 기본값 `dotnet8`, `vcrun2022`를 담고 있으면,
Eidos가 읽을 때 `dotnetsdk10`, `vcrun2022`로 바뀝니다.

NuGet은 복원하는 모든 패키지의 서명을 Windows 루트 인증서 저장소를 기준으로 확인하는데, Wine 프리픽스에는 SDK가
`sdk/10.0.403/trustedroots/codesignctl.pem`과 `timestampctl.pem`으로 함께 제공하는 코드 서명용
및 타임스탬프용 루트 인증서가 없습니다. 이것이 없으면 `Mutagen.Bethesda.Synthesis` 0.36.6 복원이 `NU3018`과
`NU3028`로 실패합니다. 그래서 `dotnetsdk10`을 선언한 도구를 실행하기 전에, SDK가 내려받아져 있으면 Eidos가 두 파일의
인증서 385개를 게임 프리픽스의 현재 사용자 루트 인증서 저장소에 추가합니다. 이 작업은 게임 경로를 기록하는 것과 같은 `regedit`
가져오기에서 이루어집니다(아래 참조). 그중 하나라도 없어지면 다시 가져옵니다. 그 뒤로는 그 프리픽스의 모든 Windows 프로그램이 이 루트
인증서를 신뢰합니다. 이 인증서가 있으면 새 프리픽스와 빈 NuGet 캐시에서 같은 복원과 빌드가 시스템 Wine 11.21과
GE-Proton11-9 모두에서 성공합니다.

계층 2나 3의 어떤 것도 조용히 실행되지 않습니다:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

GUI에서는 같은 상태들이 Prereqs 항목 아래에 있고, 빠진 것들은 버튼입니다. 번들도
아니고 런타임도 아니고 알려진 winetricks verb도 아닌 verb는 내려받기로 제시되는
대신 오타일 가능성이 높은 것으로 보고됩니다.

### DynDOLOD에 `dotnet10`이 필요한 이유

DynDOLOD는 object LOD를 스스로 만들지 않습니다: LODGen을 불러 쓰며, 그것을 세 개
배포합니다. `LODGenx64.exe`는 .NET Framework 4.8을 대상으로 하는데, Proton
아래에서는 Wine의 Mono로 넘겨지고 - 그 `System.Uri` 초기화기는 Mono가 구현하지
않은 메서드를 호출합니다. 첫 줄의 일도 하기 전에 죽으면서, 버전 배너만 있고 그
밖에는 아무것도 없는 로그와 "failed for one or more worlds"라고만 말하는 DynDOLOD
대화상자를 남깁니다.

진짜 .NET Framework를 설치해도 고쳐지지 않습니다: Proton은 `mscoree.dll` -
그것을 찾아낼 로더 - 을 자기 트리 안으로 향하는 심볼릭 링크로 바꿔놓고,
프리픽스가 업데이트될 때마다 그것을 다시 합니다.

동작하는 빌드는 `LODGenx64Win10.exe`로, 요즘 .NET을 대상으로 하고 `mscoree`는
건드리지 않습니다. `DOTNET_ROOT`를 .NET 10 런타임으로 향하게 하면 실행됩니다.
그것이 `dotnet10`이 마련하는 것이고, Eidos는 그것을 선언한 도구를 실행할 때 그
변수를 설정합니다.

Eidos는 시스템 `winetricks`를 Proton 자신의 `wine`과 게임 프리픽스에 대고
실행하는데, 이것은 Steam의 pressure-vessel 컨테이너와 protontricks + Proton-GE
불일치를 비껴갑니다. 설치되지 않은 Tier-2 verb를 선언한 도구도 그대로 실행되며,
그 verb의 이름과 고치는 명령을 담은 경고가 함께 나옵니다 - 사용자가 다른 데서
이미 가지고 있을 수도 있으니까요.

### PGPatcher에 `--ignore-mo2vfscheck`가 필요한 이유

PGPatcher는 보유한 텍스처 모드가 vanilla, parallax, complex material, PBR 중
무엇이든 각 표면에서 올바른 셰이더를 쓰도록 메시와 플러그인을 다시 씁니다. 모드
관리자 밖에서는 실행을 거부하는데, 이는 타당합니다. 맨 게임 폴더에 패치를 가하는
것은 파괴적이기 때문입니다.

문제는 그 확인 '방식'입니다. Windows에서 MO2가 게임에 주입하는 DLL 후킹 계층인
**usvfs**를 찾습니다. Eidos에는 그런 것이 없고 앞으로도 없습니다. 병합된 뷰는
Wine이 존재를 모른 채 들어가는 FUSE 마운트이기 때문입니다. 따라서 확인은 아무것도
찾지 못하고, 도구는 어떤 작업도 하기 전에 다음 메시지와 함께 즉시 종료됩니다.

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

게다가 사용자가 실행하지도 않은 프로그램의 이름을 대면서요. 두 건의 Linux 보고
([#716][pg716], [#725][pg725], 둘 다 같은 이유로 FUSE를 쓰는 MO2의 Linux 포크
Fluorine에서 온 것)에 대한 제작자 본인의 답이 이 확인을 건너뛰는 플래그입니다.

그래서 Eidos가 미리 넣어 둡니다. 모드 풀에서 발견된 PGPatcher는 인자가 이미
설정된 채로 나타나고, Executables 대화상자의 제목에 `PGPatcher`를 입력해도
Arguments 칸이 같은 방식으로 채워집니다. 입력한 내용이 언제나 우선합니다.
채워지는 것은 비워 둔 칸뿐이며, 위의 선행 요구 사항과 같은 규칙입니다.

목록은 `default_args`(`crates/eidos-instance/src/tools.rs`)에 있으며
`default_prereqs` 옆입니다. 항목은 하나지만 하나로도 함수를 둘 이유가 됩니다.
플래그는 누구도 짐작할 수 없고, 그것이 막아 주는 실패는 엉뚱한 프로그램의 이름을
대기 때문입니다.

실행에 필요한 것은 없습니다. .NET 도구로는 드물게 자체 런타임을 포함하고
(`dotnetlib/`, `includedFrameworks`를 선언하는 `runtimeconfig.json` 포함),
Microsoft CRT DLL을 실행 파일 옆에 함께 제공합니다. Prereqs는 비워 두세요.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### Mod Organizer 2 모드의 PGPatcher와 PGLite

Mod Manager가 `None`이면 PGPatcher는 병합된 `Data` 폴더 하나만 보게 되고,
메시나 텍스처가 어느 모드에서 왔는지 알 수 없습니다. 그래서 보여 줄 충돌도,
따를 모드 순서도 없습니다. `Mod Organizer 2` 모드는 인스턴스 자체를 읽습니다.
프로필의 `modlist.txt`, 우선순위 순서대로 켜진 모드, 각 모드 폴더 안의 파일을
읽습니다. 게임 안에서 도는 버전인 PGLite도 같은 방식으로 인스턴스를 읽습니다.
Eidos 인스턴스는 이미 MO2의 구조(`mods/`, `profiles/<name>/modlist.txt`,
`overwrite/`)를 갖고 있으므로 이 모드로 쓸 수 있습니다.

PGPatcher 런처에서:

1. **Mod Manager**를 `Mod Organizer 2`로 설정합니다.
2. **MO2 Options -> Instance Location**을 Eidos 인스턴스 폴더로 설정합니다.
   Wine의 `Z:` 드라이브(`/`에 해당) 위의 Windows 경로로 적습니다.
   `/mnt/Games/Eidos-Skyrim`에 있는 포터블 인스턴스는
   `Z:\mnt\Games\Eidos-Skyrim`이고, 전역 인스턴스는
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`입니다.
3. 도구의 인수에 `--ignore-mo2vfscheck`를 그대로 두십시오. 이 모드에서
   PGPatcher는 usvfs를 찾고, 없으면 멈춥니다.

PGPatcher는 폴더에 `modorganizer.ini`가 있을 때만 받아들입니다. 그래서 Eidos는
게임이나 도구를 시작할 때마다, 시작하는 프로필에 맞춰 인스턴스 루트에
`ModOrganizer.ini`를 씁니다. 내용이 바뀔 때만 다시 씁니다. 들어 있는 내용은
다음과 같습니다.

- `[General]`에 `gameName`(MO2에서 쓰는 게임 이름, 그래서 Fallout: New Vegas는 `New Vegas`), `gamePath`(게임 설치 폴더를 `Z:` 경로로),
  `selected_profile`
- `[Settings]`에 절대 `Z:` 경로로 된 `mod_directory`, `profiles_directory`,
  `overwrite_directory`, 그리고 인스턴스에 `downloads/` 폴더가 있으면
  `download_directory`

첫 줄은 Eidos가 이 파일을 썼다고 알리는 주석입니다. 이 줄이 없는
`ModOrganizer.ini`는 이름의 대소문자와 상관없이 절대 바꾸거나 지우지 않습니다.
Eidos에서 연 MO2 인스턴스는 자기 파일을 그대로 유지하고, PGPatcher는 그 파일을
있는 그대로 읽습니다. MO2는 설정을 저장할 때 주석을 버리므로, MO2가 덮어써서
저장한 파일은 그때부터 MO2의 것입니다. 빈 파일은 Eidos의 파일로 칩니다.

이런 파일에는 MO2가 마지막으로 선택한 프로필이 들어 있고, PGPatcher는 Eidos가 시작하는 프로필이 아니라 그 프로필을 씁니다.
둘이 다르면 Eidos는 실행할 때 경고를 로그에 남깁니다. MO2에서 같은 프로필을 선택하거나, 파일 이름을 바꾸거나 지워서 Eidos가 자기
파일을 쓰게 하세요.

Eidos는 무언가를 시작할 때만 자기 파일을 쓰므로, 두 번의 실행 사이에는 마지막
실행의 프로필 이름이 들어 있습니다. Eidos가 시작한 도구는 그곳에서 자기
프로필을 찾습니다.

PGPatcher는 프로필에 켜진 모드가 하나도 없을 때와, 출력 폴더가 켜진 모드의
폴더일 때 MO2에서와 마찬가지로 오류를 내고 멈춥니다.

## 게임 프리픽스 안의 Wine 자체 프로그램

파일 메뉴는 게임 프리픽스에서 Wine 프로그램 네 가지를 실행합니다. winecfg, regedit, Wine 파일 탐색기, 제거 프로그램입니다. 다른 도구와 똑같이 게임의 도구를 실행하는 Proton 또는 Wine 빌드로, 모드를 마운트한 상태에서 시작되며, 창은 프로그램이 끝나기를 기다립니다. **Prefix drive_c**는 프리픽스의 `C:` 드라이브를 파일 관리자에서 엽니다. 프리픽스에 해당 프로그램이 없는 동안에는 행이 회색으로 표시되며, 스토어가 게임을 처음 실행하면 생깁니다.

Eidos가 직접 쓰는 레지스트리 값은 `HKLM\Software\Bethesda Softworks\<game>` 아래 게임의 `installed path` 하나뿐이며, 값이 틀렸을 때 도구를 실행할 때마다 먼저 고칩니다. regedit에서 이 값을 바꿔도 다음 도구 실행 때 되돌려집니다.

## 프리픽스 안의 게임 경로

Windows 도구는 `HKLM\Software\Bethesda Softworks\<game>`의 `installed path`를
읽어 게임을 찾습니다. 게임 자신의 설치 프로그램이 쓰는 키인데 - Proton 아래의
Steam은 그것을 결코 실행하지 않습니다. 그것 없이는 xEdit, Wrye Bash, DynDOLOD가
빈 경로로 열립니다. Eidos는 도구를 실행하기 전에 그 키를 씁니다: 멱등하고,
덧붙이기만 하며, 프리픽스가 초기화되지 않았거나 사용 중이면 건너뜁니다.

## 도구에 닿기: 숨기기, 고정, 그리고 바탕화면 바로 가기

게임의 기본값에는 여러분이 결코 쓰지 않을 도구도 들어 있고, 두 번째 것에 닿으려고
여덟 개 항목을 늘어놓는 선택기는 아무도 읽지 않는 선택기입니다. Executables
대화상자에서:

- **Pin to top**은 항목을 Run 목록의 맨 앞에 놓습니다.
- **Hide from picker**는 삭제하지 않고 하나를 빼냅니다.
- **Desktop shortcut**은 `~/.local/share/applications`에 `.desktop`을 씁니다 -
  freedesktop 시스템에서 런처가 있어야 할 자리이고, 그래서 바탕화면이 아니라
  애플리케이션 메뉴와 검색에 나타납니다. 이것은
  `eidos tool <instance> run <title>`을 직접 실행하며, 그 말은 Eidos 창이 전혀
  열려 있지 않아도 도구가 **이 인스턴스의 프로필과 함께 병합된 뷰를 통해**
  뜬다는 뜻입니다.

숨기기와 고정은 도구가 무엇을 실행하는지가 아니라 어떻게 *닿는지*에 관한 것이라,
여러분 자신의 항목뿐 아니라 게임별 기본값에도 적용됩니다.

스크립트 익스텐더의 로더, 런처, 게임 바이너리에는 Desktop shortcut 버튼이
없습니다. 이들은 게임을 시작하고, 게임은 언제나 자기 스토어를 거쳐 시작되는데,
그 일은 스토어 자체의 바로가기가 이미 합니다. Eidos 1.22 이하에서 이 중 하나에
만든 바로가기는 계속 동작합니다. 터미널에 입력한
`eidos tool <instance> run <title>`처럼 스토어에 게임 시작을 요청하며, 스토어가
열지 않은 창에서 Run이 하는 것과 같습니다. 스토어 설정이 없으면 명령은 아무것도
시작하지 않고 무엇을 설정할지 출력합니다. 바로가기는 메시지를 보여주지
않으므로, 같은 명령을 터미널에서 실행해 읽으세요.

프로필마다 게임용 바로가기를 따로 둘 수 있습니다. 프로필을 오른쪽 클릭하고
**Desktop shortcut**을 고르면, 그 항목은
`eidos tool <instance> run "<game>" --profile <name>`을 실행합니다. 이 명령은
창에서 프로필을 고를 때처럼 그 프로필을 활성 프로필로 만든 다음, 스토어에 게임
시작을 요청합니다. 스토어가 여는 Eidos 창은 그 프로필을 보여줍니다.
`--profile`은 모든 `eidos tool ... run`에서 쓸 수 있고 도구가 시작되기 전에
프로필을 선택합니다. `--print`와 함께 쓰면 아무것도 선택하지 않습니다.

## 자기가 곧 Steam 앱인 도구

Creation Kit은 별개의 Steam 애플리케이션이고 자기 AppID를 원합니다; Steam을 통해
배포된 다른 몇몇 모딩 도구도 마찬가지입니다. 항목에 **Steam AppID**를 지정하면
Eidos는 게임의 것 대신 그 id로 실행합니다.

Windows에서 이것은 다른 런처를 뜻합니다. 여기서는 이미 만들어지고 있던 그 실행에
붙는 환경 변수 두 개입니다 - `SteamAppId`와 `SteamGameId`, 둘 다인데, Proton은
하나를 읽고 Steam 자신의 라이브러리는 다른 하나를 읽으며, 그 둘이 어긋난 것을 본
도구는 분명하게가 아니라 이상하게 실패하기 때문입니다. `eidos tool ... --print`는
진짜 실행이 무엇을 받게 될지 정확히 보여줍니다.

## 환경 변수, 그리고 Proton 없이 실행하는 도구

`<instance>/tools.ini`의 키 세 개는 아직 Executables 대화 상자에 입력란이
없으므로, 도구의 섹션에 직접 적어 넣으십시오. `env=NAME=value` 줄 하나가 그
도구의 실행에만 적용되는 변수 하나를 설정합니다:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos는 자신이 설정하는 모든 것을 설정한 뒤에 이 줄들을 적용하므로, 여기에
적은 값이 Eidos 자신의 값을 대체합니다. 모드가 함께 제공하는 DLL로 Eidos가
만드는 `WINEDLLOVERRIDES`도 마찬가지입니다. 이것을 설정하는 도구는 필요한
DLL을 모두 그 안에 나열해야 합니다. 이름은 영문자, 숫자, `_`로 이루어지며
숫자로 시작하지 않습니다. 다른 이름을 가진 줄은 무시됩니다.

`runner=native`는 Proton 없이 `exe` 자체를 실행합니다. 병합된 뷰는 다른 모든
도구와 똑같이 마운트되고, `arg0=`, `arg1=` 줄이 그 인수가 됩니다. Linux
프로그램이나, Steam이 관리하지 않고 Heroic에도 러너 기록이 없는 사본용 도구에
이것을 쓰십시오. `exe`는 그 사본의 Wine이고, Windows 프로그램이 첫 번째
인수이며, 프리픽스는 `env=`에서 옵니다.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe`는 절대 경로여야 합니다. 작업 디렉터리의 기본값은 `exe`가 있는 폴더,
여기서는 러너의 폴더이므로 `workdir=`를 설정하십시오. 병합된 경로로 다시
쓰이는 것은 `exe`뿐입니다. 모드 안에 있는 Windows 프로그램이라면, 병합된 뷰가
그것을 보여 주는 게임의 `Data` 폴더 아래 경로를 적으십시오. 네이티브 도구는
Proton 단계를 건너뜁니다. 위에서 설명한 레지스트리 키, Steam의 셰이더 캐시,
빈 .NET 루트, 그리고 `LD_PRELOAD`에서 Steam 오버레이를 빼는 일입니다.

Heroic으로 설치한 GOG 또는 Epic 사본에서는, 게임의 Heroic 설정
(`GamesConfig/<appName>.json`)에 Wine이나 Proton 빌드가 지정되어 있으면 도구에
이런 항목이 필요 없습니다. Eidos는 그 빌드로 게임의 Heroic 프리픽스 안에서
도구를 실행하고, `eidos prereqs <instance> --install`도 그 빌드를 winetricks에
넘깁니다. Legendary만으로 설치한 사본에는 Heroic 설정이 없습니다.

`slr=1`은 도구와 그 앞의 레지스트리 가져오기를 도구의 Proton 빌드가 지정하는 Steam Linux Runtime 컨테이너 안에서
시작합니다(`toolmanifest.vdf`의 `require_tool_appid`). Steam이 게임을 시작하는 방식과 같습니다. 그
런타임은 Steam 라이브러리 중 하나에 설치되어 있어야 하며, 없으면 실행이 멈추고 설치할 Steam 앱을 알려 줍니다. `eidos
prereqs`는 계속 호스트에서 winetricks를 실행하고, `runner=native` 도구는 이 키를 무시합니다. 컨테이너는 홈 폴더와
게임의 Steam 라이브러리를 봅니다. 다른 곳, 예를 들어 다른 드라이브에 있는 도구, 모드, 런타임에는
`env=STEAM_COMPAT_MOUNTS=/그/경로`가 필요합니다(여러 경로는 `:`로 구분).
`/usr/share/steam/compatibilitytools.d`에 설치한 Proton은 컨테이너 안의 `/usr`가 런타임의 것이므로
`slr=1`로는 아마 쓸 수 없습니다. 컨테이너 안에서 도구를 실행하는 것은 아직 시험하지 않았습니다.

`eidos tool <instance> run <title> --print`는 최종 명령과, 실행이 상속된 환경
위에 설정하는 모든 변수를 이 줄들까지 포함해 보여 줍니다.

## BethINI Pie

Eidos는 BethINI Pie를 포함하지 않습니다. Nexus 페이지에서 받은 파일을 이 인스턴스의 `mods/` 또는 도구 폴더에 넣으면 도구 목록에 `Bethini.exe`가 나타납니다. Eidos를 통해 실행하면 프로필이 가진 게임 INI 사본을 수정합니다. 실행할 때마다 Eidos가 그 INI의 사본을 보관하며, **Backups -> INIs**에서 하나를 되돌릴 수 있습니다. 사본을 만들 때 없었던 INI는 그대로 둡니다.

## 도구 자신의 설정은 여전히 그 도구의 몫

Eidos는 도구를 올바른 DLL과 함께 올바른 자리에 놓습니다. 그다음 도구가 자기
설정을 가지고 무엇을 하는지는 여러분과 그 도구 사이의 일이고, 실패는 보통
조용합니다.

이렇게 해두지 않으면 한 시간이 드는 실제 예: BodySlide의 **Game Data
Path**(Settings)는 그 위의 게임 폴더가 아니라 게임의 `Data` 디렉터리를 가리켜야
합니다. 한 단계 위로 지정하면 batch build는 "All sets processed successfully"라고
보고하고 게임이 결코 찾지 않을 자리에 1439개의 메시를 씁니다. Eidos는 그것들을
잡아냅니다 - 여러분의 설치 폴더가 아니라 `Overwrite/Root/`로 떨어집니다 - 그러나
여러분의 바디가 만들어지지 않았다는 것 말고는 게임 입장에서 잘못된 것이 하나도
없습니다.

Eidos가 이를 확인합니다. BodySlide나 Outfit Studio를 실행하기 전에, 우선하는 `CalienteTools/BodySlide/Config.xml`의 `GameDataPath`나 `OutputDataPath`가 실행 중에 게임의 `Data` 폴더로 이어지지 않으면 상태 줄이 경고하고, Health 탭에는 설정할 값과 함께 같은 카드가 나타납니다. `S:` 경로는 Eidos가 도구용으로 `S:`를 연결하는 방식대로 Steam 라이브러리 폴더로 읽습니다. 인스턴스의 `mods/` 폴더 안에 있는 `OutputDataPath`는 출력 모드용이므로 받아들입니다. `Overwrite/Root/`에 `meshes`나 `textures` 폴더가 있을 때도 Health에 카드가 나타납니다.

도구의 출력은 Overwrite에 속합니다. 실행이 간직할 만한 것을 만들어냈다면,
**Overwrite -> Create mod...**가 그것을 다른 모드와 마찬가지로 순서를 정하고
비활성화하고 제거할 수 있는 평범한 모드로 바꿔줍니다.
