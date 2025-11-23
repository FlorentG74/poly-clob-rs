# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-11-23

### Added
- Initial release of poly-clob-rs
- Support for Polymarket CLOB API v1
- Market data fetching (markets, events, event series)
- Position querying
- Price data retrieval
- Tag/category support
- Order placement with EIP-712 signatures (L1 authentication)
- HMAC-based API authentication (L2 authentication)
- Comprehensive type definitions for all API responses
- Builder pattern for constructing API requests
- Examples for common use cases
- Full API documentation

### Features
- `WebserviceRequest` builder for all API endpoints
- `Account` type for credential management
- `Order` type with EIP-712 signing support
- Response types: Markets, Events, Positions, Prices, Tags
- Authentication helpers for L1 and L2 auth
- Pagination support via `ApiResponse` trait
- Type-safe enums for order sides, types, and asset types

[Unreleased]: https://github.com/yourusername/poly-clob-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/yourusername/poly-clob-rs/releases/tag/v0.1.0
