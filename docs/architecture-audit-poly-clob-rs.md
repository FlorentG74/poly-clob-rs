# Architecture Audit: poly-clob-rs

**Date:** 2025-12-04
**Codebase:** Polymarket CLOB Rust SDK
**Total LOC:** ~2,656 lines across 32 source files

---

## Executive Summary

| Metric | Score | Notes |
|--------|-------|-------|
| **Modularity** | 6/10 | Clean layer separation, but tight coupling in key modules |
| **Error Handling** | 3/10 | 32 panic-inducing unwrap/expect calls |
| **Separation of Concerns** | 5/10 | Models clean, but API layer has god functions |
| **Production Readiness** | 4/10 | Needs hardening before production use |

---

## 1. Architecture Pattern

**Pattern:** Layered Architecture with Request Builder Pattern

```
┌─────────────────────────────────────────┐
│          External Dependencies          │
│  (reqwest, tokio, serde, alloy, etc.)   │
└────────────────────┬────────────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
    ┌──────────────┐      ┌──────────────┐
    │   Models     │      │   API Layer  │
    │              │      │              │
    │ · Account    │◄─────┤ · auth/      │
    │ · Order      │      │ · market_req │
    │ · Position   │      │ · order_req  │
    │ · Market     │      │ · position_  │
    │ · Side       │      │ · price_     │
    │ · OrderType  │      │ · event_     │
    │ · Price      │      │ · tag_       │
    │ · Event      │      │ · account_   │
    └──────────────┘      │ · webservice │
                          │ · response_  │
                          └──────┬───────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │  Polymarket CLOB APIs   │
                    │ - gamma-api.polymarket  │
                    │ - clob.polymarket       │
                    │ - data-api.polymarket   │
                    └─────────────────────────┘
```

---

## 2. Findings

### FINDING 1: Panic-Prone Error Handling (32 Instances)

**Severity:** 10/10
**Impact:** Application crashes on malformed data, auth failures, or API issues

#### Locations:

| File | Line | Code |
|------|------|------|
| `src/api/market_requests.rs` | 76 | `.expect("Error in String conversion")` |
| `src/api/market_requests.rs` | 104, 122 | `.expect("Error creating client")` |
| `src/api/order_requests.rs` | 204 | `.expect("Error creating client")` |
| `src/api/order_requests.rs` | 223 | `.unwrap()` on HTTP response |
| `src/api/order_requests.rs` | 230, 232 | `.expect()` on JSON parsing |
| `src/api/order_requests.rs` | 259 | `.expect("Cant attach Market to Position")` |
| `src/api/order_requests.rs` | 273, 277, 281 | `.expect()` on f64 parsing |
| `src/api/webservice_request.rs` | 124, 277 | `.expect("Error - can't extract API Response")` |
| `src/api/auth/clob_auth.rs` | 102 | `.unwrap()` on PrivateKeySigner |
| `src/api/auth/clob_auth.rs` | 154, 158 | `.unwrap()` / `.expect()` on HMAC |
| `src/api/auth/l1_header.rs` | 67 | `.unwrap()` on Address parsing |
| `src/api/auth/helper_functions.rs` | 10, 20, 30 | `.unwrap()` in time utilities |

#### Remediation:

**File:** `src/api/order_requests.rs:270-281`

Replace:
```rust
original_size: market_order
    .original_size
    .parse::<f64>()
    .expect("Can't parse original_size"),
```

With:
```rust
original_size: market_order
    .original_size
    .parse::<f64>()
    .map_err(|e| format!("Invalid original_size '{}': {}", market_order.original_size, e))?,
```

**File:** `src/api/auth/clob_auth.rs:102`

Replace:
```rust
let wallet = PrivateKeySigner::from_str(signer_pk).unwrap();
```

With:
```rust
let wallet = PrivateKeySigner::from_str(signer_pk)
    .map_err(|e| format!("Invalid private key: {}", e))?;
```

---

### FINDING 2: Cross-Module Tight Coupling

**Severity:** 7/10
**Impact:** Difficult to test, maintain, and evolve independently

#### Locations:

| File | Line | Import |
|------|------|--------|
| `src/api/order_requests.rs` | 8 | `use crate::{market_requests, ...}` |
| `src/api/order_requests.rs` | 251 | `market_requests::map_multiple_market_by_condition_ids_ws(...)` |
| `src/api/account_requests.rs` | 8 | `use crate::{..., market_requests}` |

#### Problem:

Order management directly calls market data fetching, breaking separation of concerns.

#### Remediation:

**Option A:** Make market enrichment optional

```rust
// src/api/order_requests.rs
pub async fn get_open_orders_raw(account: &Account) -> Result<Vec<MarketOrder>, String> {
    // Just return raw orders without market enrichment
}

pub async fn get_open_orders_with_markets(account: &Account) -> Result<Vec<OpenOrder>, String> {
    let orders = get_open_orders_raw(account).await?;
    let markets = market_requests::fetch_markets_for_orders(&orders).await?;
    enrich_orders_with_markets(orders, markets)
}
```

**Option B:** Use dependency injection

```rust
pub trait MarketProvider {
    async fn get_markets(&self, condition_ids: &[String]) -> Result<HashMap<String, Market>, String>;
}

pub async fn get_open_orders<M: MarketProvider>(
    account: &Account,
    market_provider: &M
) -> Result<Vec<OpenOrder>, String> { ... }
```

---

### FINDING 3: Repeated HTTP Client Instantiation

**Severity:** 6/10
**Impact:** Resource inefficiency, lost connection pooling benefits

#### Locations:

| File | Lines | Count |
|------|-------|-------|
| `src/api/market_requests.rs` | 102, 120 | 2 |
| `src/api/account_requests.rs` | 31, 73 | 2 |
| `src/api/order_requests.rs` | 112, 202 | 2 |

#### Current Code (`src/api/order_requests.rs:112-114`):
```rust
let client = reqwest::Client::builder()
    .build()
    .map_err(|e| format!("Error creating HTTP client: {}", e))?;
```

#### Remediation:

Create a shared client module:

```rust
// src/api/http_client.rs
use once_cell::sync::Lazy;
use reqwest::Client;

pub static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
});
```

Then use everywhere:
```rust
use crate::api::http_client::HTTP_CLIENT;

let response = HTTP_CLIENT.get(url).send().await?;
```

---

### FINDING 4: God Function in Order Requests

**Severity:** 6/10
**Impact:** Hard to test, maintain, and understand

#### Location: `src/api/order_requests.rs:201-307`

The `get_open_orders_by_market()` function handles:
1. HTTP client creation
2. GET request execution
3. HTTP response handling
4. JSON parsing
5. Market data fetching
6. String-to-float conversions
7. Order-market joining

#### Remediation:

Break into smaller functions:

```rust
async fn fetch_raw_orders(client: &Client, account: &Account) -> Result<MarketOrders, String> { ... }

fn parse_orders(raw: MarketOrders) -> Result<Vec<ParsedOrder>, String> { ... }

async fn enrich_with_markets(orders: Vec<ParsedOrder>) -> Result<Vec<OpenOrder>, String> { ... }

pub async fn get_open_orders_by_market(account: &Account) -> Result<Vec<OpenOrder>, String> {
    let raw = fetch_raw_orders(&HTTP_CLIENT, account).await?;
    let parsed = parse_orders(raw)?;
    enrich_with_markets(parsed).await
}
```

---

### FINDING 5: Code Duplication in Account Loading

**Severity:** 5/10
**Impact:** Inconsistent behavior, maintenance burden

#### Location: `src/models/account.rs:23-87`

`load_poly_account()` and `load_paper_account()` share ~80% identical code, with subtle behavioral differences:

| Behavior | load_poly_account | load_paper_account |
|----------|-------------------|-------------------|
| Token empty check | `!token.is_empty()` | None |
| Required fields | Panics on missing | Uses Default |

#### Remediation:

```rust
fn load_telegram_config() -> (Option<i64>, Option<String>) {
    let chat_id = std::env::var("TELEGRAM_CHAT_ID")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|&id| id != 0);

    let token = std::env::var("TELEGRAM_BOT_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());

    (chat_id, token)
}

pub fn load_poly_account() -> Result<Self, AccountError> {
    dotenv::dotenv().ok();
    let (telegram_chat_id, telegram_bot_token) = load_telegram_config();

    Ok(Account {
        poly_address: std::env::var("POLY_ADDRESS").map_err(|_| AccountError::MissingField("POLY_ADDRESS"))?,
        // ... rest
        telegram_chat_id,
        telegram_bot_token,
    })
}
```

---

### FINDING 6: Large Optional Struct (Market Model)

**Severity:** 5/10
**Impact:** No compile-time guarantees, defensive coding required everywhere

#### Location: `src/models/market.rs`

`PolyResponseMarket` has 80+ `Option<String>` fields with no semantic structure.

#### Current Usage (`examples/fetch_markets.rs:36-47`):
```rust
// Every field access requires defensive handling
let question = market.question.as_deref().unwrap_or("Unknown");
```

#### Remediation:

Split into required vs optional structs:

```rust
pub struct MarketCore {
    pub id: String,
    pub condition_id: String,
    pub question: String,
}

pub struct MarketDetails {
    pub slug: Option<String>,
    pub image: Option<String>,
    // ... truly optional fields
}

pub struct Market {
    pub core: MarketCore,
    pub details: MarketDetails,
}
```

---

### FINDING 7: Incomplete HTTP Status Handling

**Severity:** 4/10
**Impact:** Poor error messages for debugging

#### Location: `src/api/response_handler.rs:37-73`

Only handles: 200, 400, 401, 429. Missing: 403, 404, 5xx.

#### Remediation:

```rust
pub async fn handle_api_response(response: Response, url: &str) -> Result<String, String> {
    match response.status() {
        StatusCode::OK => Ok(response.text().await.map_err(|e| e.to_string())?),
        StatusCode::BAD_REQUEST => {
            let body = response.text().await.unwrap_or_default();
            Err(format!("Bad Request (400): {}", body))
        }
        StatusCode::UNAUTHORIZED => Err("Unauthorized (401): Check API credentials".into()),
        StatusCode::FORBIDDEN => Err("Forbidden (403): Insufficient permissions".into()),
        StatusCode::NOT_FOUND => Err(format!("Not Found (404): Resource at {} does not exist", url)),
        StatusCode::TOO_MANY_REQUESTS => {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Err("Rate limited (429): Retry after delay".into())
        }
        status if status.is_server_error() => {
            Err(format!("Server Error ({}): API infrastructure issue", status.as_u16()))
        }
        other => Err(format!("Unexpected status {}: {}", other.as_u16(), url)),
    }
}
```

---

### FINDING 8: Magic Numbers Scattered

**Severity:** 3/10
**Impact:** Configuration inflexibility

#### Locations:

| File | Line | Value | Purpose |
|------|------|-------|---------|
| `src/api/webservice_request.rs` | 11 | `MAX_RETRIES: u32 = 3` | Retry count |
| `src/api/webservice_request.rs` | 12 | `RETRY_DELAY_MS: u64 = 2000` | Retry delay |
| `src/api/response_handler.rs` | 61 | `from_secs(5)` | Rate limit delay |
| `src/api/auth/helper_functions.rs` | 3-7 | Time constants | Year/day millis |

#### Remediation:

Create a config module:

```rust
// src/config.rs
pub struct ApiConfig {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub rate_limit_delay_secs: u64,
    pub request_timeout_secs: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 2000,
            rate_limit_delay_secs: 5,
            request_timeout_secs: 30,
        }
    }
}
```

---

### FINDING 9: No Authentication Abstraction

**Severity:** 3/10
**Impact:** Inflexibility when adding new auth methods

#### Location: `src/api/auth/` directory

L1 (EIP-712) and L2 (HMAC) auth are separate implementations with no common interface.

#### Remediation:

```rust
pub trait Authenticator {
    fn sign_request(&self, method: &str, path: &str, body: Option<&str>) -> Result<AuthHeaders, AuthError>;
}

pub struct L1Authenticator { wallet: PrivateKeySigner }
pub struct L2Authenticator { api_key: String, api_secret: String, passphrase: String }

impl Authenticator for L1Authenticator { ... }
impl Authenticator for L2Authenticator { ... }
```

---

### FINDING 10: Blocking Sleep on Rate Limit

**Severity:** 3/10
**Impact:** All concurrent requests block during rate limiting

#### Location: `src/api/response_handler.rs:61`

```rust
tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
```

#### Remediation:

Return a retryable error instead of blocking:

```rust
#[derive(Debug)]
pub enum ApiError {
    RateLimited { retry_after: Duration },
    Unauthorized,
    BadRequest(String),
    // ...
}

// Let caller decide retry strategy
StatusCode::TOO_MANY_REQUESTS => {
    Err(ApiError::RateLimited { retry_after: Duration::from_secs(5) })
}
```

---

## 3. Bottlenecks

| Bottleneck | Impact | Location |
|------------|--------|----------|
| No connection pooling | Resource exhaustion under load | HTTP client creation |
| Sequential market fetching | Blocking on slow market API | `order_requests.rs:251` |
| No caching | Redundant API calls | All request modules |
| Manual pagination | Poor DX, error-prone | `WebserviceRequest` |
| Blocking rate limit sleep | Concurrent request starvation | `response_handler.rs:61` |

---

## 4. Summary: Priority Matrix

| Priority | Finding | Effort | Impact |
|----------|---------|--------|--------|
| **P0** | Panic-prone error handling | Medium | Critical - crashes |
| **P1** | Shared HTTP client | Low | Performance |
| **P1** | Cross-module coupling | Medium | Maintainability |
| **P2** | God function refactor | Medium | Testability |
| **P2** | Account loading dedup | Low | Consistency |
| **P3** | Market model restructure | High | Type safety |
| **P3** | HTTP status handling | Low | Debugging |
| **P3** | Auth abstraction | Medium | Extensibility |
| **P4** | Config centralization | Low | Maintainability |
| **P4** | Rate limit handling | Low | Concurrency |

---

## 5. Recommendations

1. **Immediate:** Replace all `.unwrap()` and `.expect()` with proper `Result` propagation
2. **High Priority:** Create shared HTTP client singleton using `once_cell::Lazy`
3. **High Priority:** Extract market enrichment to optional/injectable service
4. **Medium:** Refactor `Account::load_*` with proper error types
5. **Medium:** Break god functions into smaller, testable units
6. **Low:** Add comprehensive HTTP status handling
7. **Low:** Implement caching layer for market/event data
