# Changelog

## 0.6.2 - 2024-10-25

### Added

- Add `--bt-peer-connect-timeout`, `--bt-peer-read-write-timeout` and `--bt-peer-keep-alive-interval` options.

## 0.6.1 - 2024-10-16

### Fixed

- Fix overflow when progress bar length is small

## 0.6.0 - 2024-10-14

### Added

Support torrent and magnet link

## 0.5.1 - 2024-01-11

### Added

- Add option `--insecure` to skip to verify the server's TLS certificate

## 0.5.0 - 2023-12-15

### Changed

- Use `reqwest` to instead of `awc`

### Updated

- Support proxy

  Use `--proxy` option or set global proxy environment variables

## 0.4.1 - 2022-04-20

### Added

- Use `tracing` to log.

## 0.4.0 - 2022-04-19

### Updated

- Update dependencies.

### Changed

- No dependency on `OpenSSL`, using `rustls`.
