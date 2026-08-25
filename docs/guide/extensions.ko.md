<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# 확장

확장은 Eidos의 일부가 되지 않으면서 Eidos에 항목을 하나 더합니다. 실체는 어떤
프로그램을 가리키는 TOML 매니페스트와, 많아야 그 프로그램뿐입니다.

매니페스트는 `~/.config/Colony/Eidos/addons/`에 두며 확장마다 `.toml` 하나입니다.
**View -> Extensions -> Open folder**로 폴더를 열고 **Reload**를 누르세요 - 재시작은
필요 없습니다.

## 왜 Eidos 안으로 아무것도 적재하지 않는가

Mod Organizer 2는 플러그인을 공유 라이브러리로 적재하고 파이썬 플러그인은 Qt로
호스팅합니다. 둘 다 여기로는 옮겨오지 못합니다. Rust에는 안정된 ABI가 없어서, 다른
컴파일러로 - 혹은 다른 최적화 플래그로, 공유 의존성의 다른 기능 집합으로 - 빌드된
공유 라이브러리는 버전 불일치가 아니라 미정의 동작입니다. 게다가 Eidos의 위젯은
컴파일 시점 제네릭이라, ABI가 안정적이더라도 라이브러리가 돌려줄 위젯을 만들어낼 수
없습니다.

그래서 확장은 Eidos가 *실행하는* 프로그램입니다. 창을 죽일 수 없고, 모드 목록을
망가뜨릴 수 없으며, Eidos가 갱신되어도 계속 동작합니다.

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

**View -> Extensions**에 Run 버튼과 함께 나타나며 분리된 채로 시작합니다 - Eidos는
기다리지 않습니다.

## 점검

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

새로 고칠 때마다 실행되어 한 줄에 하나씩 결과를 출력합니다:

```
level<TAB>title<TAB>detail
```

여기서 `level`은 `problem`, `advice`, `ok` 중 하나입니다. detail은 선택입니다. 알려진
등급으로 시작하지 않는 것은 모두 무시되므로, 진행 상황 출력이나 흘러나온 경고가
Eidos 자체 점검처럼 보이는 줄을 만들어낼 수 없습니다. 결과는 **Health** 탭에 확장
이름을 앞에 붙여 나타납니다.

점검에는 3초가 주어집니다. 넘긴 점검은 중단되고 자기 자신에 대한 문제로 보고됩니다 -
클릭마다 뒤따르는 바로 그 새로 고침에서 실행되므로, 멈춘 점검은 창을 얼려버립니다.

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
| `{install}`     | 게임 설치 디렉터리                           |
| `{data}`        | 게임의 `Data` 디렉터리                       |

알 수 없는 자리표시자는 비우지 않고 적힌 그대로 남습니다. 오타가 눈에 띄게 실패하게
하려는 것이고, `--out {typo}`가 `--out --next-flag`로 둔갑하지 않게 하려는 것입니다.
자리표시자를 모두 해석할 수 없는 도구의 실행은 거부되며, Eidos가 어느 것이 빠졌는지
말해 줍니다.

## 확장이 할 수 없는 일

값을 받아 실행할 뿐, Eidos를 되부를 수도, 모드 목록을 바꿀 수도, 창에 무언가를 그릴
수도 없습니다. 의도된 것입니다. MO2가 플러그인으로 해결하면서 실제로 내부에 손을
뻗어야 하는 것들 - 게임 지원, 설치 마법사, 충돌 엔진 - 은 여기서는 덧붙인 것이 아니라
내장되어 있습니다. 게임 정의는 `~/.config/Colony/Eidos/games/`에 있는 그 자신의
TOML이고, FOMOD와 BAIN 설치기는 네이티브입니다.
