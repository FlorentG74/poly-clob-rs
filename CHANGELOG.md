# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Caller-supplied configuration (`config::Config` + `config::init`) — the library no longer reads `.env` or the environment on its own; requests made before `init` panic instead of silently defaulting
- Polymarket network policy on all HTTP clients: split-tunnel interface binding (`SPLIT_TUNNEL_IFACE`) and DNS override (`DNS_RESOLVER`) via a single client factory (`api::http_client::get_http_client`)
- Typed request builders with async `execute()`: `MarketsRequest` (keyset pagination), `MarketBySlugRequest`, `EventBySlugRequest`, `SeriesEventsRequest`, `OrderBooksRequest`, `ActivityRequest`, `CryptoPriceRequest`, `LimitOrderRequest`, `CancelOrderRequest`
- Relayer client for gasless transactions (redeem, approvals) via the Builder API, and bridge withdrawal support
- WebSocket transport helpers (`ws` module) with HTTP/1.1 upgrade handling
- Structured error types (`ClobError` and friends) with `is_retryable()` / `retry_after()`; fetch helpers retry transient failures

### Changed
- `Decimal`-based order prices/sizes with automatic rounding to Polymarket precision limits

## [0.1.0] - 2025-11-23

### Added
- Initial release: Polymarket CLOB/Gamma/Data API coverage — market, event, position, price, and tag queries; order placement with EIP-712 (L1) signing and HMAC (L2) authentication; `WebserviceRequest` builder with offset pagination; typed response models; examples
