<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# 확장

확장은 Eidos의 일부가 되지 않으면서 Eidos에 항목을 하나 더합니다. 실체는 어떤
프로그램을 가리키는 TOML 매니페스트와, 많아야 그 프로그램뿐입니다.

매니페스트는 `~/.config/Colony/Eidos/addons/`에 두며 확장마다 `.toml` 하나입니다.
View -> Extensions -> Open folder로 폴더를 열고 Reload를 누르세요 - 재시작은
필요 없습니다.

`id`, `kind`, `exec`는 필수이며 `name`을 생략하면 id가 이름이 됩니다. `exec`는
프로그램을 가리킵니다. `/`가 없는 이름은 `PATH`에서 찾습니다. 프로그램이 없으면
Eidos는 그 확장을 건너뛰고, View -> Extensions의 해당 행에 이유가 표시됩니다
(`<exec> is not on PATH` 또는 `<exec> is not there`). `games`는 확장을 해당 게임
id로 한정합니다. 생략하면 모든 게임에 적용되며, 다른 게임에서는 행에 "not for this
game"이라고 표시됩니다.

Eidos가 거부한 매니페스트는 이유와 함께 맨 위에 나열됩니다. 읽을 수 없는 파일, 올바른
TOML이 아니거나 필수 필드가 없는 경우, 비어 있거나 공백이 들어 있는 `id`, 비어 있는
`exec`, 앞선 매니페스트가 이미 쓴 `id`(파일은 이름순으로 읽습니다), Eidos가 모르는
`kind`, 그리고 `protocol = 1`과 파일 이름 `extensions`가 없는 설치 프로그램, 미리 보기,
저장 정보 종류입니다.

## 왜 Eidos 안으로 아무것도 적재하지 않는가

Mod Organizer 2는 플러그인을 공유 라이브러리로 적재하고 파이썬 플러그인은 Qt로
호스팅합니다. 둘 다 여기로는 옮겨오지 못합니다. Rust에는 안정된 ABI가 없어서, 다른
컴파일러로 - 혹은 다른 최적화 플래그로, 공유 의존성의 다른 기능 집합으로 - 빌드된
공유 라이브러리는 미정의 동작이며, 버전 불일치로 보고되지도 않습니다. 게다가 Eidos의 위젯은
컴파일 시점 제네릭이라, ABI가 안정적이더라도 라이브러리가 돌려줄 위젯을 만들어낼 수
없습니다.

확장은 Eidos가 사용자 계정의 파일 시스템 접근 권한으로 실행하는 프로그램입니다. 신뢰하는 도우미만 설치하세요. 프로세스 격리와 응답 검증이 운영 체제의 샌드박스를 제공하는 것은 아닙니다.

## 도구

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # 모든 게임에 쓰려면 생략
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

View -> Extensions에 Run 버튼과 함께 나타나며 분리된 채로 시작합니다 - Eidos는
기다리지 않고, 상태 표시줄에 pid가 나옵니다. 출력은 버려집니다. `workdir`가 없으면
Eidos 자체를 시작한 폴더에서 시작합니다.

## 점검

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

새로 고칠 때마다, `workdir`로 달리 지정하지 않으면 인스턴스 루트에서 실행되며 한 줄에
하나씩 결과를 출력합니다:

```
level<TAB>title<TAB>detail
```

여기서 `level`은 `problem`, `advice`, `ok` 중 하나입니다. detail은 선택이며 title이 없는 줄은
버려집니다. 알려진
등급으로 시작하지 않는 것은 모두 무시되므로, 진행 상황 출력이나 흘러나온 경고가
Eidos 자체 점검처럼 보이는 줄을 만들어낼 수 없습니다. 결과는 Diagnostics 탭에
`<name> - <title>` 형태로, prefix 카드와 Nexus 카드 뒤, 다른 Health 카드 앞에 나타납니다.

점검에는 3초가 주어집니다. 넘긴 점검은 중단되고 자기 자신에 대한 문제로 보고됩니다 -
클릭마다 뒤따르는 바로 그 새로 고침에서 실행되므로, 멈춘 점검은 창을 얼려버립니다. 시계는 점검이 종료하고 출력 파이프가 닫힐 때까지
돌아가므로, 파이프를 계속 쥐고 있는 백그라운드 자식 프로세스도 3초에 포함됩니다.
오류로 종료하면서 아무것도 출력하지 않는 점검도 오류 출력의 첫 줄과 함께 같은 식으로
보고되며, 어느 한 스트림에 1 MiB를 넘게 출력하는 점검도 마찬가지입니다. 어느 쪽이든
Reload를 누를 때까지 Eidos는 다시 실행하지 않고, 해당 행에 "It will not be retried
until you press Reload."라고 표시됩니다. 점검의 표준 입력은 비어 있습니다.

## 자리표시자

`args`와 `workdir` 모두 다음을 확장합니다:

| 자리표시자      | 무엇인가                                     |
| --------------- | -------------------------------------------- |
| `{instance}`    | 인스턴스 루트                                |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | 활성 프로필의 이름                           |
| `{profile_dir}` | 활성 프로필의 디렉터리                       |
| `{game}`        | 게임 id, 예: `skyrimse`                      |
| `{game_name}`   | 게임의 표시 이름                             |
| `{install}`     | 인스턴스가 실행하는 게임 폴더                |
| `{data}`        | 그 폴더의 `Data` 디렉터리                    |

`{install}`은 스토어 설치본이고, 인스턴스가 게임 사본을 실행 중이면 그 사본입니다
(고정한 `Stock Game` 또는 목록 자체의 게임 폴더. [usage.md](usage.md#게임-파일-스토어-설치본-또는-사본)
참고). `{data}`는 그것을 따라갑니다.

점검은 매니페스트가 들어 있는 폴더인 `{addon_dir}`도 받으며, 아래의 프로토콜 1
확장도 마찬가지입니다. 도구는 받지 않습니다.

알 수 없는 자리표시자는 절대 비우지 않습니다. 비우면 `--out {typo}`가
`--out --next-flag`로 둔갑하기 때문입니다. 자리표시자를 모두 해석할 수 없는 도구나
점검의 실행은 거부되며, Eidos가 어느 것이 빠졌는지 말해 줍니다. 도구는 상태 표시줄에,
점검은 Diagnostics 탭의 해당 행에 표시됩니다.

## 확장이 할 수 없는 일

확장은 Eidos를 역호출하거나, 아카이브에서 자신을 등록하거나, 자체 위젯을 그릴 수 없습니다. 구조화된 프로토콜 1 설치 프로그램은 검증된 파일 계획과 선택 질문을 반환할 수 있으며, 검토와 임시 준비 및 게시 과정은 Eidos가 관리합니다. FOMOD와 BAIN은 계속 네이티브 설치 프로그램으로 제공됩니다. 미리 보기와 저장 정보 확장도 응답에 제한을 둔 같은 호스트를 사용합니다. 일치 우선순위, 기록된 CLI 답변, 제한 및 재실행 시 동일성 검증은 [프로토콜 명세와 실행 가능한 예제](../../../../guide/examples/extensions/README.md)를 참고하세요.
