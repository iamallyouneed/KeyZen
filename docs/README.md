# KeyZen 문서 · Docs

KeyZen의 키 매핑 설정, 트레이 앱 동작, 자동 실행, 진단 방법을 다루는 공식 문서입니다.

[메인 README](../README.md) · [변경 기록](../CHANGELOG.md)

---

## 📚 문서 목차

| 문서 | 설명 |
| --- | --- |
| ⚙️ [**설정 구성 가이드**](configuration.md) | `keyzen.yaml` 의 `settings`·`layers` 구조, 전체 액션 레퍼런스, 키 이름과 alias, 검증 규칙 |
| 🖥️ [**앱 설정 가이드**](application-settings.md) | `settings.yaml` 스키마, 트레이 메뉴, 시작 시 자동 실행, 실행 모드, 진단 로그 |

---

## ⚡ 빠른 시작

```powershell
cargo build --workspace                 # keyzen.exe 빌드
cargo run                               # 트레이 실행 (로그: %APPDATA%\KeyZen\keyzen.log)
```

- 릴리스 zip을 쓰는 경우 `keyzen.exe` 를 실행하면 콘솔 없이 백그라운드로 뜹니다.
- 트레이 아이콘을 마우스 오른쪽 버튼으로 클릭 → **Choose key config...** 로 키 매핑 파일을 고릅니다.
- 예시 설정은 [examples/keyzen.yaml](../examples/keyzen.yaml) (최소 구성)과 [examples/fullkeyzen.yaml](../examples/fullkeyzen.yaml) (모든 액션)에 있습니다.

## 🗂️ 설정 파일 위치

| 파일 | 위치 | 내용 |
| --- | --- | --- |
| `settings.yaml` | `%APPDATA%\KeyZen\settings.yaml` | 앱 설정 (시작 시 자동 실행, 키 설정 경로) |
| `keyzen.yaml` | `%APPDATA%\KeyZen\keyzen.yaml` | 키 매핑 (레이어·액션) |
| `keyzen.log` | `%APPDATA%\KeyZen\keyzen.log` | 트레이 진단 로그 |

## 💡 문제가 생기면

- 설정이 적용되지 않으면 트레이 메뉴에서 파일을 다시 고르세요. 설정 파일은 자동으로 다시 읽지 않습니다.
- 잘못된 설정이면 tray가 paused 상태로 시작합니다. `Pause` 를 끄기 전에 유효한 설정을 고르세요.
- 자세한 증상별 해결은 [메인 README의 문제 해결](../README.md#문제가-생겼을-때)과 [앱 설정 가이드](application-settings.md)를 보세요.
- 그래도 해결되지 않으면 [이슈](https://github.com/iamallyouneed/KeyZen/issues)로 알려 주세요.
