<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Eidos 설치

들어오는 길은 셋입니다. 어느 쪽이든 같은 실행 파일 두 개 - `eidos`(명령줄)와
`eidos-gui` - 그리고 Nexus의 "Mod Manager Download" 버튼이 여러분의 인스턴스로
떨어지게 해주는 `nxm://` 처리기를 줍니다.

## 먼저 필요한 것

| | |
|---|---|
| **FUSE가 있는 Linux** | PATH에 `fusermount3`. 요즘 배포판이면 모두 들어 있습니다. |
| **한 번 실행해 본 Proton 게임** | Steam은 첫 실행 때에만 게임의 Wine 프리픽스를 만들고, Eidos는 그 안에서 동작합니다. |
| **`7z`** | 모드 압축 파일 설치용. 대부분의 배포판에서는 `p7zip`. |

root도, 데몬도, `/etc/fuse.conf` 수정도, 그룹 추가도 필요 없습니다. Eidos는
게임 프로세스에 속한 비공개 네임스페이스 안에서 마운트합니다.

## Arch

```bash
cd packaging && makepkg -si
```

## 릴리스 압축 파일

```bash
./install.sh
```

기본값은 `~/.local/bin`입니다. `--system`은 `/usr/local/bin`에, `--bindir DIR`은
원하는 곳에 넣습니다. 다시 실행하는 것이 정식 업그레이드 방법입니다.

## 소스에서 빌드

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## 그다음: Steam이 이것을 가리키게 하기

Eidos는 게임의 실행 명령 *그 자체로* 동작합니다. 그래서 게임이 시작되기 전에
마운트할 수 있습니다. Steam에서 게임을 오른쪽 클릭 -> 속성 -> 실행 옵션:

```
~/.local/bin/eidos-gui %command%
```

플레이를 누르세요. Eidos가 그 게임의 인스턴스로 열립니다. 모드를 설치하고,
LOOT로 정렬하고, Run을 누르십시오. 종료하면 마운트도 함께 사라지고, 설치 폴더는
정확히 이전 그대로입니다.

절대 경로를 쓰세요 - Steam은 셸의 `PATH`를 읽지 않습니다.

### 터미널이 더 편하다면

```sh
eidos init skyrimse               # 인스턴스 만들기 (폴더를 주면 휴대용이 됩니다)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root 모드
eidos sort skyrimse               # LOOT로 로드 순서 정렬
eidos play skyrimse -- %command%  # 무엇이든 병합된 뷰를 통해 실행
```

게임 id를 받는 모든 명령은 휴대용 인스턴스의 폴더도 받습니다 -
[usage.ko.md](usage.md) 참고. 전체 안내도 거기에 있습니다.

## 선택: FUSE 패스스루

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"`는 커널 FUSE 패스스루를
켭니다. **기본값은 꺼짐이고, 거의 확실히 그대로 두는 편이 좋습니다**: Skyrim
SE에서 측정한 결과, 게임이 자기 아카이브와 플러그인을 열지 못하게 되어 모드가
조용히 로드되지 않습니다. 이 스위치는 메커니즘을 다시 시험하기 위한 것이지
권장되기 때문이 아닙니다.

자세한 내용과 그 결정을 뒷받침한 측정값은
[troubleshooting.ko.md](troubleshooting.md)에 있습니다.

## 벌써 뭔가 잘못됐나요?

[troubleshooting.ko.md](troubleshooting.md)가 환경 스위치, 연산 카운터 읽는
법, 그리고 지금까지 누군가를 물었던 모든 문제를 다룹니다.
