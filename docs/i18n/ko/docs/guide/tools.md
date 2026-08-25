<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# 도구: xEdit, BodySlide, DynDOLOD, FNIS

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
- 설정에서 지정한 **tools folder** (Tools -> Tools folder), 인스턴스끼리 공유하는
  디렉터리용 - `/mnt/Games/Tools` 같은 것.

목록은 게임별이라, Skyrim 인스턴스에 Fallout의 에디터가 제시되는 일은 없습니다.
검색은 네 단계 아래에서 멈추는데, 모드 풀은 수십만 개의 파일이고 이 작업은 도구
목록을 만들 때마다 실행되기 때문이며, 심볼릭 링크는 따라가지 않습니다. 이렇게
찾아낸 도구는 직접 입력한 것과 정확히 같은 방식으로 구성됩니다: 런타임은 그
이름에서 오고, 아래의 모든 것과 같은 규칙을 따릅니다.

도구가 다른 곳에 있거나 다른 인자를 주고 싶다면 직접 추가하십시오 - 같은 제목의
사용자 항목이 자동으로 찾아낸 것을 덮어씁니다.

## 하나 추가하기

GUI에서: **Tools -> Executables**, 그다음 Add. 명령줄에서:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
```

script extender, 게임 바이너리, 런처는 자동으로 감지됩니다; 등록이 필요한 것은
추가 도구뿐입니다.

### 실제 파일이 있는 자리를 가리키게 하십시오

실행 파일은 실제로 놓여 있는 자리에 등록하십시오. 도구가 모드로 설치되었다면
그것은 모드 폴더 안입니다:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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

## 도구의 출력을 자기 모드로 보내기

생성기 - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - 는 수백 개의 파일을
씁니다. 기본적으로 그것들은 나머지 전부와 함께 Overwrite로 떨어집니다.
Executables 편집기에서 **Capture output into**를 지정하면 이번 실행의 출력이
대신 그 모드로 들어갑니다:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

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

## 도구에 필요한 DLL은 그 이름으로 정해집니다

이건 놀라운 대목이라 분명히 말해둘 값어치가 있습니다: **도구에 붙이는 제목이
Eidos가 그 도구에 어떤 런타임 사전 요구 사항을 마련할지 결정합니다.** 일치는
제목의 대소문자 구분 없는 부분 문자열입니다.

| 제목에 이것이 들어 있으면 | Eidos가 요청하는 것 |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| 그 밖의 모든 것 | 없음 |

그래서 **`BodySlide`**로 등록한 도구는 DirectX DLL을 받습니다; 같은 실행 파일을
**`BS`**로 등록하면 아무것도 받지 못하고, DLL에 대해서는 한마디도 하지 않는 오류와
함께 시작하지 못할 수 있습니다. 도구 이름은 프로그램 이름을 따르십시오.

목록은 `default_prereqs`(`crates/eidos-instance/src/tools.rs`)에 있고,
Executables 대화상자의 `Prereqs` 항목은 편집할 수 있습니다 - 감지는 기본값이지
규칙이 아닙니다.

### 사전 요구 사항의 세 종류

**Tier 1 - 번들 DLL** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). Eidos가 함께
배포하며 실행할 때 프리픽스로 복사합니다. 할 일 없음, 네트워크 없음.

**Tier 2 - winetricks verb** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). 이것들은 레지스트리 키와 GAC와 CLR 호스트를 쓰기 때문에
파일 복사로는 되지 않습니다. **Microsoft에서 내려받습니다**.

**Tier 3 - 런타임** (`dotnet10`). 요즘 .NET 런타임은 자기 디렉터리에 사는 193개의
파일이고 `DOTNET_ROOT`를 통해 찾습니다: 등록되지도 않고 프리픽스에 설치되지도
아예 않으므로, 다른 두 계층 어느 쪽도 그것을 실어 나를 수 없습니다. Eidos가 직접
내려받아 바이너리에 박혀 있는 체크섬으로 확인하고
`~/.local/share/Colony/Eidos/runtimes/`에 캐시합니다 - **어느 인스턴스에도 속하지
않는 바깥**인데, 78 MB는 게임별도 프로필별도 아니기 때문입니다.

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

## 자기가 곧 Steam 앱인 도구

Creation Kit은 별개의 Steam 애플리케이션이고 자기 AppID를 원합니다; Steam을 통해
배포된 다른 몇몇 모딩 도구도 마찬가지입니다. 항목에 **Steam AppID**를 지정하면
Eidos는 게임의 것 대신 그 id로 실행합니다.

Windows에서 이것은 다른 런처를 뜻합니다. 여기서는 이미 만들어지고 있던 그 실행에
붙는 환경 변수 두 개입니다 - `SteamAppId`와 `SteamGameId`, 둘 다인데, Proton은
하나를 읽고 Steam 자신의 라이브러리는 다른 하나를 읽으며, 그 둘이 어긋난 것을 본
도구는 분명하게가 아니라 이상하게 실패하기 때문입니다. `eidos tool ... --print`는
진짜 실행이 무엇을 받게 될지 정확히 보여줍니다.

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

도구의 출력은 Overwrite에 속합니다. 실행이 간직할 만한 것을 만들어냈다면,
**Overwrite -> Create mod...**가 그것을 다른 모드와 마찬가지로 순서를 정하고
비활성화하고 제거할 수 있는 평범한 모드로 바꿔줍니다.
