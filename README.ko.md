# toss

[![CI](https://github.com/azyu/tossinvest-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/azyu/tossinvest-cli/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/azyu/tossinvest-cli)](https://github.com/azyu/tossinvest-cli/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[English](README.md) | [한국어](README.ko.md)

> Rust로 만든 작고 에이전트 친화적인 토스증권 Open API CLI입니다.

## 특징

- 현재가, 호가, 차트, 종목, 시장, 계좌 목록, 보유잔고 조회 명령 제공
- 주문 명령은 dry-run 출력과 명시적 `--confirm` 안전장치 제공
- 사람용 text 출력과 자동화용 안정적인 JSON 출력 지원
- `toss setup`, config file, environment override 지원
- Linux, macOS, Windows용 릴리즈 바이너리 제공
- `toss --version`, `toss -V`로 build metadata 출력

## 설치

### Homebrew

```bash
brew install azyu/tap/toss
```

### 미리 빌드된 바이너리

[GitHub Releases](https://github.com/azyu/tossinvest-cli/releases/latest)에서 최신 아카이브를 다운로드하세요.

| 플랫폼 | 에셋 |
|--------|------|
| Linux amd64 | `toss_0.x.y_linux_amd64.tar.gz` |
| Linux arm64 | `toss_0.x.y_linux_arm64.tar.gz` |
| macOS arm64 | `toss_0.x.y_macos_arm64.tar.gz` |
| Windows x64 | `toss_0.x.y_windows_x64.zip` |
| Windows arm64 | `toss_0.x.y_windows_arm64.zip` |

### 소스에서 빌드

Rust 1.93+가 필요합니다.

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --release --bin toss
mkdir -p ~/.local/bin
install -m 755 rust/target/release/toss ~/.local/bin/toss
```

## 빠른 시작

### 1. 인증 정보 설정

권장 경로는 interactive setup 명령입니다.

```bash
toss setup
```

script에서 설정할 때는 secret을 command-line argument로 넘기지 말고 stdin으로 전달하세요.

```bash
printf '%s\n' "$TOSSINVEST_CLIENT_SECRET" | \
  toss setup --client-id "$TOSSINVEST_CLIENT_ID" --with-secret-stdin --no-check
```

`toss setup`은 Unix에서 제한적인 permission으로 `~/.config/tossinvest/config.yaml`을 씁니다. 이 파일에는 plaintext 인증 정보가 들어가므로 repository와 신뢰하지 않는 backup 밖에 두세요.

config file을 쓰지 않고 환경변수만 사용할 수도 있습니다.

```bash
export TOSSINVEST_CLIENT_ID="issued-client-id"
export TOSSINVEST_CLIENT_SECRET="issued-client-secret"
export TOSSINVEST_ACCOUNT_SEQ="1"
```

### 2. 인증 확인

```bash
toss --json config
toss --json auth token
```

`config`는 `client_id`를 masking하고 `client_secret`을 출력하지 않습니다. `auth token`은 token 발급 가능 여부를 확인하지만 token 값을 출력하지 않습니다.

#### 2.1. 필요할 때 계좌 선택 (선택 사항)

```bash
toss account list
toss account use 1
```

`account_seq`는 계좌가 필요한 명령을 사용하기 전까지는 선택 사항입니다. `toss account use`는 선택한 계좌 sequence를 로컬 config file에 저장합니다. 일회성 override에는 `--account <seq>`를 사용하세요.

### 3. 자주 쓰는 조회 명령 실행

```bash
toss price AAPL
toss quote orderbook AAPL
toss quote trades AAPL
toss chart candles AAPL --interval 1d --count 200

toss stock get AAPL
toss stock warnings 005930
toss stock search --symbols 005930,AAPL

toss market exchange-rate
toss market calendar kr
toss market calendar us

toss holdings
```

캔들 pagination은 이전 응답의 `nextBefore`를 `--before`로 넘기세요.

```bash
toss chart candles AAPL --interval 1m --count 200 --before "2026-06-19T18:20:00+09:00"
```

### 4. 주문 안전장치 확인

읽기 전용 주문/계좌 정보 명령은 Toss API를 호출하지만 주문을 생성, 정정, 취소하지 않습니다.

```bash
toss --json order buying-power --currency USD
toss --json order sellable-quantity --symbol AAPL
toss --json order commissions
toss --json order list --status open
toss --json order show <orderId>
```

Dry-run 변경 주문 명령은 요청 형태를 출력하고 실주문을 보내지 않습니다.

```bash
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
```

실제 변경 주문 명령은 `--confirm`이 필요합니다. 아래 명령은 그대로 실행하는 예시가 아니라 template로만 취급하세요.

```bash
toss order buy --symbol <SYMBOL> --qty <QTY> --type limit --price <PRICE> --client-order-id <CLIENT_ORDER_ID> --confirm
toss order sell --symbol <SYMBOL> --qty <QTY> --type market --confirm
toss order modify <ORDER_ID> --qty <QTY> --type limit --price <PRICE> --confirm --confirm-high-value-order
toss order cancel <ORDER_ID> --confirm
```

> [!CAUTION]
> 확인한 토스 Open API 문서에는 sandbox가 명시되어 있지 않습니다. `--confirm`이 붙은 주문 명령은 실제 증권 계좌에 주문을 전송하는 실거래 요청으로 취급하세요.

## 명령 개요

| 그룹 | 서브커맨드 |
|------|-----------|
| `toss setup` | `client_id`와 `client_secret`을 로컬 config file에 저장 |
| `toss config` | 적용된 config 요약 출력 |
| `toss auth` | `token` |
| `toss price` | symbol별 현재가 |
| `toss quote` | `orderbook`, `trades`, `limits` |
| `toss chart` | `candles` |
| `toss stock` | `get`, `warnings`, `search` |
| `toss market` | `exchange-rate`, `calendar` |
| `toss account` | `list`, `use` |
| `toss holdings` | 계좌 보유잔고 |
| `toss order` | `buy`, `sell`, `modify`, `cancel`, `list`, `show`, `buying-power`, `sellable-quantity`, `commissions` |
| `toss --version` / `toss -V` | build metadata 출력 |

> [!NOTE]
> `toss setup`은 로컬 config file에 plaintext 인증 정보를 쓰지만 `client_secret`을 출력하지 않습니다.
> `toss account use <seq>`는 같은 로컬 config file의 account sequence만 업데이트합니다. 일회성 계좌 override에는 `--account <seq>`를 사용하세요.
> `toss order list`에는 `--status open|closed`가 필요하고, `toss order show`에는 order ID가 필요하며, `toss order sellable-quantity`에는 `--symbol`이 필요합니다.

## 설정과 인증

Config path 우선순위:

1. `--config <path>`
2. `~/.config/tossinvest/config.yaml`

인증 정보 설정 명령:

```bash
# Interactive: client_id와 client_secret을 prompt로 입력합니다.
toss setup

# Scripted: client_secret은 argv가 아니라 stdin으로 읽습니다.
printf '%s\n' "$TOSSINVEST_CLIENT_SECRET" | \
  toss setup --client-id "$TOSSINVEST_CLIENT_ID" --with-secret-stdin --no-check

# 인증 정보 저장과 동시에 계좌를 선택합니다.
printf '%s\n' "$TOSSINVEST_CLIENT_SECRET" | \
  toss setup --client-id "$TOSSINVEST_CLIENT_ID" --with-secret-stdin --account 1
```

`toss setup`은 저장 후 token check를 실행합니다. offline setup, CI smoke test, 아직 활성화되지 않은 인증 정보에는 `--no-check`를 사용하세요.

환경변수 override:

| 변수 | 목적 |
|------|------|
| `TOSSINVEST_CLIENT_ID` | Toss Open API client ID |
| `TOSSINVEST_CLIENT_SECRET` | Toss Open API client secret |
| `TOSSINVEST_ACCOUNT_SEQ` | 계좌가 필요한 명령에서 사용할 선택적 기본 account sequence |

Token cache path:

```text
~/.tossinvest/token.json
```

> [!CAUTION]
> `toss setup`은 로컬 config file에 `client_secret`을 plaintext로 저장합니다. 인증 정보와 token cache file은 repository 밖에 두세요. 로컬 plaintext 저장이 부담스럽다면 환경변수 방식을 사용하세요.

## 출력 계약

자동화에는 `--json` 또는 `--output json`을 사용하세요.

```json
{"ok":true,"command":"price","data":{}}
```

```json
{"ok":false,"command":"price","error":{"kind":"api","code":"stock-not-found","message":"...","requestId":"..."}}
```

Text output은 사람이 읽기 위한 출력입니다. Text mode에서는 command error가 stderr로 출력됩니다. JSON mode에서는 성공과 오류 envelope가 모두 stdout으로 출력됩니다.

## 주문 안전 규칙

- `--dry-run`은 `--confirm`보다 우선합니다.
- 실주문 `buy`, `sell`, `modify`, `cancel`에는 `--confirm`이 필요합니다.
- 주문 생성은 `--client-order-id`를 받을 수 있습니다. 필요하면 직접 idempotency key를 생성하세요.
- 주문 생성에는 size field를 정확히 하나만 제공해야 합니다: `--qty` 또는 `--amount`.
- `--confirm-high-value-order`는 Toss `confirmHighValueOrder`에 대응합니다. `--confirm`을 대체하지 않습니다.
- 가격, 수량, 금액, rate는 floating-point가 아니라 string 또는 JSON value로 유지합니다.

## 개발자 문서

- [Technical spec](docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md)
- [Phase 1 plan](docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md)
- [Phase 2 plan](docs/superpowers/plans/2026-06-19-tossinvest-cli-phase2.md)
- [Phase 3 plan](docs/superpowers/plans/2026-06-19-tossinvest-cli-phase3.md)
