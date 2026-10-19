# KeyZen 설정 구성 가이드

KeyZen 키 매핑 설정(`keyzen.yaml`)의 파일 구조, 레이어 해석 규칙, 전체 액션 레퍼런스, 키 이름, 검증 규칙을 설명합니다.

[문서 홈](README.md) · [앱 설정 가이드](application-settings.md) · [메인 README](../README.md)

---

## 1. 설정 파일

KeyZen 설정 파일은 YAML 형식입니다. 기본 실행 파일명은 `keyzen.yaml`이며, 다른 파일을 쓰려면 `--config`로 경로를 넘깁니다.

```powershell
cargo run --bin keyzen -- --config examples/keyzen.yaml
cargo run --bin keyzen -- validate --config examples/keyzen.yaml
cargo run --bin keyzen -- dump --config examples/keyzen.yaml
```

- `validate` 는 설정 오류를 검사하고, `dump` 는 KeyZen이 해석한 정규화된 설정을 출력합니다.
- 실행 중인 tray는 설정 파일 변경을 자동으로 감지하지 않습니다. 고친 뒤에는 트레이 메뉴에서 **Choose key config...** 로 다시 고르세요.

> [!TIP]
> 설정을 고칠 때는 먼저 `validate` 로 검사하고, 적용 결과가 헷갈리면 `dump` 로 정규화된 형태를 확인하는 습관이 좋습니다.

---

## 2. 전체 구조

설정 파일은 `settings` 와 `layers` 로 구성됩니다.

```yaml
settings:
  tapping_term_ms: 200
  tap_dance_term_ms: 180
  one_shot_timeout_ms: 1000

layers:
  base:
    CapsLock:
      tap_hold:
        tap: Escape
        hold:
          layer_while_held: nav

  nav:
    H: Left
    J: Down
    K: Up
    L: Right
```

- `settings` 는 생략할 수 있습니다. 생략하면 기본값이 사용됩니다.
- `layers` 는 필수이며, 반드시 `base` 레이어를 포함해야 합니다.

---

## 3. Settings

타이밍 관련 값 세 가지를 정합니다. 단위는 밀리초(ms)입니다.

| 필드 | 기본값 | 설명 |
| --- | ---: | --- |
| `tapping_term_ms` | `200` | tap-hold 키를 탭으로 볼지 홀드로 볼지 결정하는 시간입니다. |
| `tap_dance_term_ms` | `180` | 같은 키를 여러 번 탭하는 tap dance 입력을 기다리는 시간입니다. |
| `one_shot_timeout_ms` | `1000` | one-shot 레이어·모디파이어가 다음 키를 기다리는 최대 시간입니다. |

세 값은 모두 0보다 커야 합니다.

---

## 4. Layers

`layers` 아래에는 레이어 이름을 정의합니다. 각 레이어는 `키: 액션` 형태의 매핑입니다.

```yaml
layers:
  base:
    A: B
    CapsLock:
      layer_while_held: nav

  nav:
    H: Left
```

### 4.1 레이어 해석 규칙

- KeyZen은 현재 레이어 스택의 맨 위부터 아래로 키 매핑을 찾습니다.
- 맨 아래에는 항상 `base` 가 있습니다.
- 위쪽 레이어에 해당 키가 없을 때만 아래 레이어로 fallback합니다.
- 위쪽 레이어에서 fallback을 막고 아무 입력도 보내지 않으려면 `noop: true` 를 매핑합니다.
- `base` 레이어는 액션의 대상으로 지정할 수 없습니다.
- `layer_while_held` 는 키를 누를 때 레이어를 올리고, 키를 뗄 때 해당 레이어를 내리는 hold 레이어로 동작합니다.
- `layer_toggle` 은 레이어가 없으면 추가하고, 있으면 제거합니다. 같은 레이어가 중복으로 쌓이지 않습니다.

---

## 5. 액션 레퍼런스

액션은 짧은 문자열 또는 액션 객체로 작성합니다.

### 5.1 Send (`send`)

문자열 값은 해당 키를 한 번 탭하는 `send` 의 축약형입니다.

```yaml
layers:
  base:
    A: B
    Escape:
      send: CapsLock
```

위 설정에서 `A` 는 `B` 를 보내고, `Escape` 는 `CapsLock` 을 보냅니다.

### 5.2 Noop (`noop`)

`noop: true` 는 키를 소비하지만 아무 입력도 보내지 않습니다. 상위 레이어에서 하위 레이어 fallback을 명시적으로 막을 때 사용합니다.

```yaml
layers:
  base:
    A: B
    CapsLock:
      layer_while_held: nav

  nav:
    A:
      noop: true
```

위 설정에서 `CapsLock` 을 누른 상태로 `A` 를 누르면 `base` 의 `A: B` 로 fallback하지 않고 아무 입력도 보내지 않습니다. `noop` 값은 반드시 boolean `true` 여야 합니다.

### 5.3 Chord (`chord`)

`chord` 는 여러 키를 순서대로 누른 뒤 역순으로 뗍니다.

```yaml
layers:
  base:
    C:
      chord: [Ctrl, C]
    V:
      chord: [Ctrl, V]
```

빈 chord는 허용되지 않습니다.

### 5.4 Layer While Held (`layer_while_held`)

`layer_while_held` 는 키를 누르는 동안 대상 레이어를 활성화합니다.

```yaml
layers:
  base:
    CapsLock:
      layer_while_held: nav

  nav:
    H: Left
```

`CapsLock` 을 누른 상태에서 `H` 를 누르면 `Left` 가 입력됩니다. `CapsLock` 을 떼면 `nav` 레이어가 내려갑니다.

### 5.5 Layer Toggle (`layer_toggle`)

`layer_toggle` 은 레이어를 켜고 끕니다.

```yaml
layers:
  base:
    F12:
      layer_toggle: nav

  nav:
    H: Left
```

레이어가 꺼져 있으면 켜고, 켜져 있으면 끕니다.

### 5.6 Tap Hold (`tap_hold`)

`tap_hold` 는 짧게 누르면 `tap`, 길게 누르거나 다른 키로 인터럽트되면 `hold` 를 실행합니다.

```yaml
layers:
  base:
    CapsLock:
      tap_hold:
        tap: Escape
        hold:
          layer_while_held: nav

  nav:
    H: Left
```

동작 규칙:

- `tapping_term_ms` 안에 키를 떼면 `tap` 이 실행됩니다.
- `tapping_term_ms` 가 지나면 `hold` 가 시작됩니다.
- 누른 상태에서 다른 키를 먼저 누르면 즉시 `hold` 로 판정합니다.
- `hold` 가 `layer_while_held` 이면 원래 키를 뗄 때 레이어가 내려갑니다.

현재 구현에서는 `tap_hold` 나 `tap_dance` 안에 또 다른 `tap_hold`/`tap_dance` 를 중첩할 수 없습니다.

### 5.7 Tap Dance (`tap_dance`)

`tap_dance` 는 같은 키를 몇 번 탭했는지에 따라 다른 액션을 실행합니다.

```yaml
layers:
  base:
    Quote:
      tap_dance:
        1: Quote
        2: DoubleQuote
```

위 설정에서 `Quote` 를 한 번 탭하면 `'`, 두 번 탭하면 `"` 가 입력됩니다. 정의되지 않은 탭 횟수는 아무 입력도 보내지 않고 경고 로그만 남깁니다. 탭 횟수 `0` 은 허용되지 않습니다.

### 5.8 One-Shot Modifier (`one_shot_modifier`)

`one_shot_modifier` 는 다음 키 하나에만 모디파이어를 적용합니다.

```yaml
layers:
  base:
    LeftShift:
      one_shot_modifier: Shift
    A: A
```

`LeftShift` 를 눌렀다 떼고 `A` 를 누르면 `Shift+A` 가 전송됩니다. 다음 키가 입력되거나 `one_shot_timeout_ms` 가 지나면 one-shot 상태는 사라집니다.

### 5.9 One-Shot Layer (`one_shot_layer`)

`one_shot_layer` 는 다음 키 하나를 지정한 레이어에서 해석합니다.

```yaml
layers:
  base:
    O:
      one_shot_layer: nav
    H: H

  nav:
    H: Left
```

`O` 를 누른 뒤 `H` 를 누르면 `nav` 레이어의 `H: Left` 가 적용됩니다. 다음 키가 입력되거나 `one_shot_timeout_ms` 가 지나면 one-shot 레이어는 사라집니다.

---

## 6. 키 이름

키 이름은 대소문자를 엄격히 구분하지 않습니다. 공백, `-`, `_` 는 정규화 과정에서 무시됩니다. 예를 들어 `left-shift`, `left_shift`, `LeftShift` 는 모두 `LeftShift` 로 해석됩니다.

### 6.1 정규 이름

| 종류 | 키 이름 |
| --- | --- |
| 문자/숫자 | `A`-`Z`, `0`-`9` |
| 기능키 | `F1`-`F24` |
| 이동 | `Left`, `Right`, `Up`, `Down`, `Home`, `End`, `PageUp`, `PageDown` |
| 편집 | `Backspace`, `Delete`, `Insert`, `Tab`, `Enter`, `Escape`, `Space` |
| 모디파이어 | `Shift`, `LeftShift`, `RightShift`, `Ctrl`, `LeftCtrl`, `RightCtrl`, `Alt`, `LeftAlt`, `RightAlt`, `Win`, `LeftWin`, `RightWin` |
| 시스템 | `CapsLock`, `Menu`, `PrintScreen`, `ScrollLock`, `Pause`, `NumLock` |
| 기호 | `Quote`, `DoubleQuote`, `Semicolon`, `Colon`, `Comma`, `LessThan`, `Period`, `GreaterThan`, `Slash`, `Question`, `Backslash`, `Pipe`, `Minus`, `Underscore`, `Equal`, `Plus`, `Grave`, `Tilde`, `LeftBracket`, `LeftBrace`, `RightBracket`, `RightBrace` |
| Shift 기호 | `Exclamation`, `At`, `Hash`, `Dollar`, `Percent`, `Caret`, `Ampersand`, `Asterisk`, `LeftParen`, `RightParen` |

### 6.2 Alias

| Alias | 정규 이름 |
| --- | --- |
| `esc` | `Escape` |
| `caps`, `capslock` | `CapsLock` |
| `spacebar` | `Space` |
| `bksp` | `Backspace` |
| `del` | `Delete` |
| `ins` | `Insert` |
| `pgup`, `pgdn` | `PageUp`, `PageDown` |
| `lshift`, `rshift` | `LeftShift`, `RightShift` |
| `lctrl`, `rctrl` | `LeftCtrl`, `RightCtrl` |
| `lalt`, `ralt`, `altgr` | `LeftAlt`, `RightAlt`, `RightAlt` |
| `meta`, `super` | `Win` |
| `apps`, `application` | `Menu` |
| `prtsc` | `PrintScreen` |
| `break` | `Pause` |

---

## 7. 검증 규칙

KeyZen은 실행 전에 다음 조건을 검사합니다.

- `layers.base` 가 반드시 있어야 합니다.
- 레이어 이름은 비어 있을 수 없습니다.
- 레이어 액션이 참조하는 대상 레이어는 반드시 존재해야 합니다.
- `noop` 값은 반드시 `true` 여야 합니다.
- `layer_while_held`, `layer_toggle`, `one_shot_layer` 는 `base` 를 대상으로 삼을 수 없습니다.
- `chord` 는 최소 하나 이상의 키를 가져야 합니다.
- `tap_dance` 는 최소 하나 이상의 탭 횟수를 가져야 하며, `0` 은 사용할 수 없습니다.
- `tap_hold` 와 `tap_dance` 는 서로 중첩할 수 없습니다.

설정을 수정한 뒤에는 먼저 다음 명령으로 확인하는 것을 권장합니다.

```powershell
cargo run -- validate --config path\to\keyzen.yaml
```

---

## 8. 전체 예제

모든 액션을 한 파일에 담은 예제가 [examples/fullkeyzen.yaml](../examples/fullkeyzen.yaml) 에 있습니다. 최소 구성은 [examples/keyzen.yaml](../examples/keyzen.yaml) 을 보세요.

---

## 9. 자주 묻는 질문 (FAQ)

### Q. 레이어가 안 내려가요.
**A:** `layer_while_held` 는 그 키를 떼야 내려갑니다. 떼기 전에 다른 키로 인터럽트되어 `tap_hold` 의 `hold` 로 판정된 경우에도 원래 키를 떼면 레이어가 내려갑니다.

### Q. tap_hold 키를 눌렀는데 탭으로 인식돼요.
**A:** `tapping_term_ms`(기본 200ms) 안에 키를 떼면 탭으로 봅니다. 값을 늘리면 더 길게 눌러야 홀드로 판정됩니다.

### Q. `layer_toggle` 로 켠 레이어가 중복으로 쌓이나요?
**A:** 아닙니다. 이미 켜져 있으면 다시 켤 때 제거되므로 같은 레이어가 두 번 쌓이지 않습니다.

### Q. 설정에 없는 키를 누르면 어떻게 되나요?
**A:** 레이어 스택을 따라 아래로 내려가며 매핑을 찾고, `base` 에도 없으면 키를 그대로 통과시킵니다.
