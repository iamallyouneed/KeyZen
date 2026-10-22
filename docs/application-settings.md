# KeyZen 앱 설정 가이드

KeyZen tray 앱의 설정 파일 스키마, 최초 실행 동작, 트레이 메뉴, 시작 시 자동 실행, 실행 모드, 진단 로그를 설명합니다.

[문서 홈](README.md) · [설정 구성 가이드](configuration.md) · [메인 README](../README.md)

---

## 1. 설정 파일 위치

KeyZen tray 앱은 키 매핑 설정과 별개인 앱 설정 파일을 사용합니다.

```text
%APPDATA%\KeyZen\settings.yaml
```

키 매핑은 [설정 구성 가이드](configuration.md)의 `keyzen.yaml` 이 담당합니다. `settings.yaml` 은 "어떤 키 매핑을 쓸지", "로그인 때 켤지" 같은 앱 수준의 값을 담습니다.

---

## 2. 스키마

```yaml
start_at_login: false
key_config_path: C:\Users\user\AppData\Roaming\KeyZen\keyzen.yaml
```

| 필드 | 설명 |
| --- | --- |
| `start_at_login` | Windows 로그인 시 KeyZen tray를 실행할지 지정합니다. |
| `key_config_path` | tray 런타임이 사용할 키 매핑 YAML 파일 경로입니다. 상대 경로는 `settings.yaml` 이 있는 폴더를 기준으로 해석합니다. |

- 앱 설정은 임시 파일을 먼저 작성한 뒤 교체하는 방식으로 저장됩니다.
- Pause 상태는 현재 실행 세션에서만 유지되며 앱 설정에 저장되지 않습니다. 다시 실행하면 항상 실행 중 상태로 시작합니다.

---

## 3. 최초 실행

`keyzen.exe` 를 처음 실행하면 다음 파일을 자동 생성합니다.

```text
%APPDATA%\KeyZen\settings.yaml
%APPDATA%\KeyZen\keyzen.yaml
```

기본 `keyzen.yaml` 은 빈 `base` 레이어를 포함합니다.

```yaml
layers:
  base: {}
```

저장된 키 설정 파일이 없거나 유효하지 않으면 tray는 paused 상태로 실행됩니다. 이 상태에서도 **Choose key config...** 로 유효한 설정을 선택해 복구할 수 있습니다.

---

## 4. Tray 메뉴

메뉴 라벨은 영어로 표시되며 순서는 다음과 같습니다.

```text
Pause
---
Start at login
Choose key config...
---
Quit
```

| 메뉴 | 동작 |
| --- | --- |
| `Pause` | 체크된 동안 리매핑을 중단하고 키를 그대로 통과시킵니다. 전환 시 눌린 합성 키, 레이어, tap-hold, tap dance, one-shot 상태를 초기화합니다. |
| `Start at login` | 현재 사용자의 Windows Task Scheduler 작업을 등록하거나 삭제합니다. 작업 변경에 성공한 경우에만 앱 설정을 저장합니다. |
| `Choose key config...` | `.yaml` 또는 `.yml` 파일을 선택하고 즉시 검증합니다. 유효한 경우 바로 적용합니다. 설정이 없어 자동으로 일시정지된 상태였다면 적용 후 리매핑을 재개하고, 사용자가 직접 Pause한 경우에는 상태를 유지합니다. |
| `Quit` | 눌린 합성 키와 엔진 상태를 초기화한 뒤 종료합니다. |

- tray tooltip은 실행 중 `KeyZen`, 일시정지 중 `KeyZen (Paused)` 로 표시됩니다.
- 로그인 직후 Explorer 또는 알림 영역이 아직 준비되지 않아 tray 아이콘 등록이 실패해도 앱은 종료하지 않고, `TaskbarCreated` 와 hidden top-level window 메시지에서 아이콘 등록을 다시 시도합니다.

---

## 5. 시작 시 자동 실행

`Start at login` 메뉴는 현재 사용자 기준의 Windows Task Scheduler 작업을 만들거나 지웁니다.

| 항목 | 값 |
| --- | --- |
| 작업 이름 | `\KeyZen\Autorun for <username>` |
| 트리거 | 현재 사용자 로그온, 3초 지연(`PT03S`) |
| 실행 파일 | 현재 `keyzen.exe` 의 절대 경로 |
| 로그온 유형 | `TASK_LOGON_INTERACTIVE_TOKEN` |
| 실행 권한 | 일반 사용자 권한(`TASK_RUNLEVEL_LUA`) |

작업 변경(등록/삭제)에 성공한 경우에만 `start_at_login` 값이 저장됩니다. 설정 파일을 직접 고쳐서 켜고 싶다면 `start_at_login: true` 로 바꾸고 tray를 다시 실행하세요.

---

## 6. 실행 모드와 단일 인스턴스

KeyZen은 콘솔 없는 트레이 앱 하나로 동작합니다. `keyzen.exe` 를 실행하면 백그라운드로 뜨고 알림 영역에 아이콘이 생기며, 종료는 트레이 메뉴의 `Quit` 입니다.

- `Local\KeyZen.Runtime` named mutex로 KeyZen은 하나만 실행됩니다.
- 이미 실행 중이면 새 인스턴스는 실행 중인 트레이에 "already running" 알림을 보내고 종료합니다.

---

## 7. 진단 로그

tray 앱은 다음 파일에 시작, 트레이 아이콘 등록, 런타임 상태, 종료 요청과 오류를 기록합니다.

```text
%APPDATA%\KeyZen\keyzen.log
```

- 로그가 시작 시점에 5MB 이상이면 기존 로그를 `keyzen.log.1` 로 이동합니다.
- 실행 중에는 `tray.running` 마커가 존재하며 정상 종료 시 제거됩니다.
- 강제 종료나 프로세스 충돌처럼 마지막 원인을 기록할 수 없는 경우에는 다음 실행 로그에 이전 세션이 정상 종료되지 않았다고 기록됩니다.

> [!TIP]
> 키가 이상하게 동작할 때는 `keyzen.log` 에서 시작, 종료, 비정상 종료 여부를 먼저 확인하세요. 입력 내용 자체는 로그에 남지 않습니다.

---

## 8. 자주 묻는 질문 (FAQ)

### Q. Pause 상태로 종료했는데 다시 켜니 실행 중이 됩니다.
**A:** 정상입니다. Pause 상태는 현재 세션에만 유지되고 `settings.yaml` 에 저장되지 않습니다.

### Q. 설정 파일을 직접 고쳤는데 반영되지 않아요.
**A:** 설정 파일은 자동으로 다시 읽지 않습니다. 트레이 메뉴에서 **Choose key config...** 로 다시 고르거나 tray를 재시작하세요.

### Q. 설정이 잘못돼서 tray가 paused로 떴어요.
**A:** 파일을 고친 뒤 **Choose key config...** 로 다시 고르세요. 유효하지 않은 설정은 그대로는 적용되지 않으며, 유효한 설정을 고르면 리매핑이 다시 시작됩니다.

### Q. 자동 실행을 끄고 싶어요.
**A:** 트레이 메뉴의 `Start at login` 체크를 해제하면 Task Scheduler 작업이 삭제됩니다. 설정 파일의 `start_at_login` 값을 직접 `false` 로 바꿔도 됩니다.
