# toss

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> Rust로 만든 작고 자동화 친화적인 토스증권 Open API CLI입니다.

`toss`는 [토스증권 Open API](https://developers.tossinvest.com/docs)를 감싸는 CLI입니다. 자동화를 위한 안정적인 JSON envelope, 사람이 읽기 쉬운 text 출력, 주문 명령을 위한 명시적 안전장치를 제공합니다.

## 기능

- 현재가, 호가, 체결, 차트, 종목, 시장, 계좌, 보유잔고 조회
- 재사용 가능한 Rust core crate(`toss-core`)와 CLI crate(`toss-cli`) 분리
- 사람이 읽는 text 출력과 자동화용 JSON 출력 지원
- config file + environment override 지원
- 주문 dry-run smoke check
- 실주문 명령은 명시적인 `--confirm` 필요
- `toss --version`, `toss -V` build metadata 출력

## 설치

### Homebrew

```bash
brew install azyu/tap/toss
```

### source에서 빌드

Rust 1.93+가 필요합니다.

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --release --bin toss
mkdir -p ~/.local/bin
install -m 755 rust/target/release/toss ~/.local/bin/toss
```

설치된 binary 확인:

```bash
toss --version
```

출력 예시:

```text
toss version 0.0.1+<commit>
commit: <commit>
built: <UTC timestamp>
```

## 빠른 시작

### 1. 인증 정보 설정

기본 config file을 만듭니다.

```bash
mkdir -p ~/.config/tossinvest
chmod 700 ~/.config/tossinvest
$EDITOR ~/.config/tossinvest/config.yaml
chmod 600 ~/.config/tossinvest/config.yaml
```

Config 형식:

```yaml
client_id: "issued-client-id"
client_secret: "issued-client-secret"
```

계좌가 필요한 명령을 사용할 때만 계좌를 선택하고 저장합니다.

```bash
toss account list
toss account use 1
```

세션 한정 환경변수도 사용할 수 있습니다.

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

### 3. 조회 명령 실행

```bash
toss price AAPL
toss quote orderbook AAPL
toss quote trades AAPL
toss chart candles AAPL --interval 1d

toss stock get AAPL
toss stock warnings 005930
toss stock search --symbols 005930,AAPL

toss market exchange-rate
toss market calendar kr
toss market calendar us

toss account list
toss account use 1
toss holdings
```

### 4. 주문 안전장치 확인

Dry-run 주문 명령은 요청 형태를 출력하고 실주문을 보내지 않습니다.

```bash
toss --json order buying-power --currency USD
toss --json order buy --symbol AAPL --qty 1 --type limit --price 1 --dry-run
```

실제 변경 주문 명령은 `--confirm`이 필요합니다.

```bash
toss order buy --symbol AAPL --qty 1 --type limit --price 180 --client-order-id client-123 --confirm
toss order sell --symbol AAPL --qty 1 --type market --confirm
toss order modify ORD-123 --qty 2 --type limit --price 181 --confirm --confirm-high-value-order
toss order cancel ORD-123 --confirm
```

> [!CAUTION]
> 확인한 토스 Open API 문서에는 sandbox가 명시되어 있지 않습니다. `--confirm`이 붙은 주문 명령은 production brokerage traffic으로 취급하세요.

## 명령 개요

| 그룹 | 하위 명령 |
|------|-----------|
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

## 전역 옵션

```text
--config <path>       config file 경로 (기본: ~/.config/tossinvest/config.yaml)
--account <seq>       계좌가 필요한 명령의 accountSeq override
--output text|json    출력 형식
--json                --output json 단축 옵션
--quiet               text output의 부가 출력 억제
```

## 설정과 인증

Config path 우선순위:

1. `--config <path>`
2. `~/.config/tossinvest/config.yaml`

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
> 인증 정보와 token cache file은 repository 밖에 두고 제한적인 file permission을 사용하세요.

## 출력 계약

자동화에는 `--json` 또는 `--output json`을 사용하세요.

성공 JSON 출력:

```json
{"ok":true,"command":"price","data":{}}
```

오류 JSON 출력:

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

## 개발 문서

- Design spec: [`docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md`](docs/superpowers/specs/2026-06-18-tossinvest-cli-design.md)
- Phase 1 plan: [`docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md`](docs/superpowers/plans/2026-06-18-tossinvest-cli-phase1.md)
- Phase 2 plan: [`docs/superpowers/plans/2026-06-19-tossinvest-cli-phase2.md`](docs/superpowers/plans/2026-06-19-tossinvest-cli-phase2.md)
- Phase 3 plan: [`docs/superpowers/plans/2026-06-19-tossinvest-cli-phase3.md`](docs/superpowers/plans/2026-06-19-tossinvest-cli-phase3.md)
