# poly-clob-rs: Migration Plan for Polymarket CLOB API v2

**Reference:** https://github.com/Polymarket/py-clob-client-v2  
**Status:** Ready for implementation — deploy when v2 protocol goes live  
**Date drafted:** 2026-04-09

---

## Summary of Breaking Changes

The v2 protocol changes the on-chain order struct and EIP712 signature format. Orders signed with the v1 schema will be **rejected** by the new contracts. The REST API endpoints are mostly unchanged; the core breakage is in order signing and the JSON body sent to `POST /order`.

---

## 1. EIP712 Order Struct — Breaking Field Changes

### V1 type string (current)
```
Order(uint256 salt,address maker,address signer,address taker,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint256 expiration,uint256 nonce,uint256 feeRateBps,uint8 side,uint8 signatureType)
```
Domain: `"Polymarket CTF Exchange"` version `"1"`

### V2 type string (new)
```
Order(uint256 salt,address maker,address signer,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint8 side,uint8 signatureType,uint256 timestamp,bytes32 metadata,bytes32 builder)
```
Domain: `"Polymarket CTF Exchange"` version `"2"`

### Field-level diff

| Field | V1 | V2 |
|---|---|---|
| `salt` | ✅ uint256 | ✅ uint256 (same) |
| `maker` | ✅ address | ✅ address (same) |
| `signer` | ✅ address | ✅ address (same) |
| `taker` | ✅ address (zero addr default) | ❌ **removed from type hash** |
| `tokenId` | ✅ uint256 | ✅ uint256 (same) |
| `makerAmount` | ✅ uint256 | ✅ uint256 (same) |
| `takerAmount` | ✅ uint256 | ✅ uint256 (same) |
| `expiration` | ✅ uint256 | ❌ **removed from type hash** (still sent in JSON body) |
| `nonce` | ✅ uint256 | ❌ **removed from type hash and JSON** |
| `feeRateBps` | ✅ uint256 | ❌ **removed from type hash and JSON** |
| `side` | ✅ uint8 | ✅ uint8 (same) |
| `signatureType` | ✅ uint8 | ✅ uint8 (same) |
| `timestamp` | ❌ | ✅ **new** uint256 (milliseconds since epoch) |
| `metadata` | ❌ | ✅ **new** bytes32 (default: `0x000...0`) |
| `builder` | ❌ | ✅ **new** bytes32 (default: `0x000...0`) |

### Key semantic changes
- `expiration` is **still sent in the JSON POST body** for GTD orders, but is no longer part of the EIP712 hash.
- `timestamp` replaces the role of `salt` as a time-bound unique value; it is in **milliseconds** (not seconds).
- `salt` remains but is now a random nonce (no longer a Unix timestamp).
- `taker`, `nonce`, and `feeRateBps` are completely gone from the protocol.

---

## 2. Verifying Contract Addresses — Breaking Change

The new exchange contracts have different addresses.

| Market type | V1 (current) | V2 (new) |
|---|---|---|
| Non-neg-risk | `0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E` | `0xE111180000d2663C0091e4f400237545B87B996B` |
| Neg-risk | `0xC5d563A36AE78145C45a50134d48A1215220f80a` | `0xe2222d279d744050d28e00520010520000310F59` |

---

## 3. JSON POST Body for `POST /order` — Breaking Change

### V1 JSON (current `SignedOrderRequest`)
```json
{
  "order": {
    "salt": 1234567890,
    "maker": "0x...",
    "signer": "0x...",
    "taker": "0x0000000000000000000000000000000000000000",
    "tokenId": "979134...",
    "makerAmount": "100000",
    "takerAmount": "200000",
    "expiration": "0",
    "nonce": "0",
    "feeRateBps": "0",
    "side": "BUY",
    "signatureType": 1,
    "signature": "0x..."
  },
  "owner": "<api_key>",
  "orderType": "FOK"
}
```

### V2 JSON (new `SignedOrderRequestV2`)
```json
{
  "order": {
    "salt": 1234567890,
    "maker": "0x...",
    "signer": "0x...",
    "tokenId": "979134...",
    "makerAmount": "100000",
    "takerAmount": "200000",
    "side": "BUY",
    "signatureType": 1,
    "timestamp": "1712700000000",
    "metadata": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "builder": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "expiration": "0",
    "signature": "0x..."
  },
  "owner": "<api_key>",
  "orderType": "FOK"
}
```

---

## 4. Signature Type — New Variant

`SignatureTypeV2` adds one new value:

| Value | Name | V1 | V2 |
|---|---|---|---|
| 0 | EOA | ✅ | ✅ |
| 1 | POLY_PROXY | ✅ | ✅ |
| 2 | POLY_GNOSIS_SAFE | ✅ | ✅ |
| 3 | POLY_1271 | ❌ | ✅ new |

The default used in practice (POLY_PROXY = 1) is unchanged.

---

## 5. New Endpoints (Non-Breaking Additions)

These do not break existing functionality but can be added for completeness:

| Endpoint | Path | Purpose |
|---|---|---|
| `GET /version` | `/version` | Detect server protocol version |
| `GET /markets-by-token/` | `/markets-by-token/{token_id}` | Market lookup by token |
| `GET /clob-markets/` | `/clob-markets/{condition_id}` | CLOB market details |
| `GET /prices-history` | `/prices-history` | Historical price data |
| `GET /data/pre-migration-orders` | `/data/pre-migration-orders` | V1-era orders query |
| `POST /v1/heartbeats` | `/v1/heartbeats` | Builder heartbeat |
| `GET /builder/trades` | `/builder/trades` | Builder trade history |
| `GET /fees/builder-fees/{addr}` | `/fees/builder-fees/{addr}` | Builder fee lookup |
| `GET /auth/ban-status/closed-only` | `/auth/ban-status/closed-only` | Account ban status |
| RFQ suite | `/rfq/...` | Request-for-Quote (new system) |

---

## 6. Implementation Plan

### Phase 1: Core Order Signing (CRITICAL — must ship on protocol upgrade day)

#### 6.1 `poly-clob-rs/src/models/order.rs`

1. **Add new fields to `Order` struct:**
   ```rust
   pub timestamp: u64,         // milliseconds since epoch
   pub metadata: [u8; 32],     // bytes32, default zeros
   pub builder: [u8; 32],      // bytes32, default zeros
   ```

2. **Remove obsolete fields from `Order` struct and `OrderQueryBody`:**
   - Remove `taker: String`
   - Remove `fee_rate_bps: i32` (from both struct and EIP712 message)
   - Keep `expiration` in the struct and JSON body, remove it from `get_message_values()`/`get_message_type_hash()`

3. **Update `get_message_type_hash()`** — new keccak256 input:
   ```
   Order(uint256 salt,address maker,address signer,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint8 side,uint8 signatureType,uint256 timestamp,bytes32 metadata,bytes32 builder)
   ```

4. **Update `get_domain_values()`** — domain version `"1"` → `"2"`.

5. **Update verifying contract addresses:**
   ```rust
   // V2 Non-neg-risk
   const NON_NEG_RISK_VERIFYING_CONTRACT: Address =
       address!("E111180000d2663C0091e4f400237545B87B996B");
   // V2 Neg-risk
   const NEG_RISK_VERIFYING_CONTRACT: Address =
       address!("e2222d279d744050d28e00520010520000310F59");
   ```

6. **Update `get_message_values()`** — new field list in order (field order matters for EIP712):
   - Remove `taker_addr`, `expiration`, `nonce`, `fee_rate_bps` values
   - Add after `side` and `signature_type`: `timestamp` (uint256), `metadata` (bytes32), `builder` (bytes32)

7. **Update `SignedOrderRequest` serialization struct** in `build_order_query_body()`:
   - Remove `taker`, `nonce`, `fee_rate_bps` fields
   - Add `timestamp: u64` (serialize as decimal string), `metadata: String` (hex bytes32), `builder: String` (hex bytes32)
   - Keep `expiration` (still sent in JSON, just not hashed)

8. **Update `Order::builder()` defaults:**
   - Remove `taker` setter
   - Remove `nonce` parameter from `build_order_query_body()` signature
   - Remove `fee_rate_bps` field
   - Add `timestamp` with default `chrono::Utc::now().timestamp_millis() as u64`
   - Add `metadata` with default `[0u8; 32]`
   - Add `builder` with default `[0u8; 32]`

#### 6.2 `poly-clob-rs/src/api/auth/clob_auth.rs`

- Update `generate_values_hash()` to handle `bytes32` type (`DynSolType::FixedBytes(32)` / `DynSolValue::FixedBytes(...)`)

#### 6.3 `poly-clob-rs/src/api/auth/eip712_trait.rs`

- Update `get_message_values()` signature if needed.

#### 6.4 `poly-clob-rs/src/api/order_requests.rs`

1. **`LimitOrderRequest::build()`:**
   - Remove `taker(get_zero_address())` from `Order::builder()` call
   - Remove `fee_rate_bps(fee_rate_bps)` from `Order::builder()` call
   - Remove the `with_fee` option (or keep it as dead code initially)
   - Populate `timestamp` with `chrono::Utc::now().timestamp_millis() as u64`

2. **`LimitOrderRequest::execute()`:**
   - Remove `nonce` argument from `build_order_query_body()` call

3. **Remove `fee_requests` import** (or keep if `fee-rate` endpoint still exists for other purposes).

#### 6.5 `poly-clob-rs/src/models/clob_orders.rs`

- `MarketOrder.associate_trades` may change shape — audit response structure if needed. Not a compile-time break.

### Phase 2: New Enum Variant (Low Priority)

#### 6.6 `poly-clob-rs/src/models/order_type.rs` (or `signature_type`)

- If a `SignatureType` enum exists or is added, include `Poly1271 = 3`.

### Phase 3: New Endpoints (Can be done anytime, non-breaking)

#### 6.7 `poly-clob-rs/src/api/clob_endpoints.rs`

Add:
```rust
pub static VERSION: &str = "/version";
pub static GET_MARKET_BY_TOKEN: &str = "/markets-by-token/";
pub static GET_CLOB_MARKET: &str = "/clob-markets/";
pub static GET_PRICES_HISTORY: &str = "/prices-history";
pub static PRE_MIGRATION_ORDERS: &str = "/data/pre-migration-orders";
pub static POST_HEARTBEAT: &str = "/v1/heartbeats";
pub static GET_BUILDER_TRADES: &str = "/builder/trades";
pub static GET_BUILDER_FEE_RATE: &str = "/fees/builder-fees/";
pub static CLOSED_ONLY: &str = "/auth/ban-status/closed-only";
pub static CREATE_BUILDER_API_KEY: &str = "/auth/builder-api-key";
```

---

## 7. Files to Change (Summary)

| File | Change type | Priority |
|---|---|---|
| `src/models/order.rs` | **BREAKING** — new struct, new EIP712 hash, new contracts | Critical |
| `src/api/auth/clob_auth.rs` | **BREAKING** — add bytes32 encoding support | Critical |
| `src/api/order_requests.rs` | **BREAKING** — remove taker/nonce/fee, add timestamp | Critical |
| `src/api/clob_endpoints.rs` | Additive — new endpoint constants | Low |
| `src/models/order_type.rs` | Additive — add POLY_1271 signature type if needed | Low |
| `src/api/fee_requests.rs` | May be obsolete — fee_rate_bps removed from orders | Low |

---

## 8. Test Plan

After implementing Phase 1, validate with:

1. **Unit test:** Regenerate `test_build_order_query_body_structure` in `order.rs` tests against a known V2 signed order from the Python client (same inputs → same signature).
2. **Integration test:** Place a small FOK order on the testnet (Amoy, chain 80002) against the V2 contract `0xE111180000d2663C0091e4f400237545B87B996B`.
3. **Verify `signature_type=1` (POLY_PROXY)** still works as the default.
4. **Regression:** Existing replay-based tests should be unaffected since they use `PaperAccount`.

---

## 9. Open Questions (Verify Before Implementing)

1. **`expiration` in JSON body:** Confirm v2 server actually accepts `expiration` in the signed order JSON for GTD orders (it's in OrderDataV2 as optional but not in the EIP712 hash). This is the most likely source of subtle bugs.
2. **`salt` generation:** V1 used Unix timestamp seconds as salt. V2 salt is `int(random() * timestamp_ms)` — a pseudo-random product. Update salt generation to match (randomized, not pure timestamp).
3. **Builder/metadata:** `BYTES32_ZERO` (`0x000...0`) is the correct default for non-builder orders.
4. **RFQ system:** See below — RFQ is a new optional trading mode, not required for basic order flow.
5. **`GET /version` check:** The Python client retries order posting if the server version changes mid-session. Optionally add a startup version check to detect protocol mismatches early.

---

## 10. RFQ System (Request for Quote)

RFQ is an entirely new trading mechanism in v2 — separate from the regular CLOB order book. It is **optional** and does not affect standard limit/market order flow. Relevant for Polytrader if you want to act as a **quoter** (market maker in the RFQ system) or a **requester** (use it to get better fills on large orders).

### What It Is

RFQ is a bilateral negotiation layer sitting alongside the CLOB:
- A **requester** announces intent to trade (token, side, size, price)
- **Quoters** (market makers) respond with competing quotes
- Requester accepts the best quote; quoter approves
- Both sides sign V1 orders; the server settles the trade

The RFQ layer is useful for large trades that would move the CLOB too much, or for programmatic market makers who want to respond to specific trading intent rather than posting passive orders.

### Three Match Types

Polymarket prediction market tokens come in complementary pairs (YES/NO). RFQ has to handle three different ways two parties can settle:

| Match Type | What it means | Side logic | Token used |
|---|---|---|---|
| **COMPLEMENTARY** | Requester buys YES, quoter sells YES (direct opposite sides on same token) | Sides are inverted | Same `token` as request |
| **MINT** | No direct holder exists; mint a new YES+NO pair, give each party one side | Sides same direction | Uses `complement` token |
| **MERGE** | Both parties hold opposite tokens and want to settle; burn them for USDC | Sides same direction | Uses `complement` token |

For MINT/MERGE, the price is inverted (`1 - price`) because the quoter is taking the other side of the prediction.

### Full Lifecycle

```
Requester                         Server                          Quoter
    |                                |                               |
    |-- POST /rfq/request ---------->|                               |
    |<- { request_id } --------------|                               |
    |                                |<-- POST /rfq/quote -----------|
    |                                |-> { quote_id }  ------------->|
    |-- GET /rfq/data/requester/quotes (or /rfq/data/best-quote) --> |
    |                                |                               |
    |-- POST /rfq/request/accept --->|  (signed V1 order inside)     |
    |                                |-- notify quoter ------------->|
    |                                |<-- POST /rfq/quote/approve ---|  (signed V1 order inside)
    |<-- trade settled --------------|                               |
```

### Key Detail: RFQ Uses V1 Orders

Even in the v2 protocol, the signed orders inside RFQ accept/approve payloads are **V1 orders** (with `taker`, `nonce`, `feeRateBps`). The Python client comment explicitly states: *"RFQ accept/approve always use V1 orders since the RFQ protocol requires the V1 order fields."*

This means if Polytrader ever implements RFQ quoter/requester logic, `poly-clob-rs` will need to keep the V1 `Order` struct (or a parallel `OrderV1` type) available alongside the new V2 struct. The V1 signing path (old EIP712 type hash, old contracts) must not be deleted — just not used for normal order placement.

### Relevant Endpoints

```
POST /rfq/request           Create a request (requester)
DELETE /rfq/request         Cancel a request
GET /rfq/data/requests      Query requests (filters: state, token, price, size)
POST /rfq/quote             Create a quote (quoter)
DELETE /rfq/quote           Cancel a quote
GET /rfq/data/requester/quotes   Requester sees incoming quotes
GET /rfq/data/quoter/quotes      Quoter sees their quotes
GET /rfq/data/best-quote    Best quote for a given request
POST /rfq/request/accept    Requester accepts (sends signed V1 order)
POST /rfq/quote/approve     Quoter approves (sends signed V1 order)
GET /rfq/config             Server-side RFQ configuration
```

### Recommendation for Polytrader

- **Short term:** Ignore RFQ entirely. Normal CLOB orders are unaffected.
- **Medium term:** Consider implementing as a **quoter** if you want to earn the spread on large incoming requests (passive market making without posting to the book).
- **If implementing:** Keep `OrderV1` struct and its EIP712 logic in a parallel module (`src/models/order_v1.rs`). Do not delete it when migrating to V2 normal orders.
