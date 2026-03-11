# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-11

### ⚙️ Miscellaneous

- Update gitignore, remove tool-versions ([d2df16a](https://github.com/rsvalerio/cargo-ops/commit/d2df16a3ce86b5fa81101803bc85b46674433204))

- Add configuration for automated releases and update documentation ([ad3f945](https://github.com/rsvalerio/cargo-ops/commit/ad3f945ec1490df2b677555e34a162efc6b1eda3))

- Remove unused Cargo configuration file ([c0934aa](https://github.com/rsvalerio/cargo-ops/commit/c0934aa85b1fe67c5d0f5e0ef39327622644210e))

- Update changelog commit preprocessors to clean up commit messages ([142b8b1](https://github.com/rsvalerio/cargo-ops/commit/142b8b19b42ccbae4bf4ce98531fe8b396aad64c))

- Update quinn-proto to version 0.11.14 and add audit configuration ([832d66d](https://github.com/rsvalerio/cargo-ops/commit/832d66da9bf218795523ba352eb1cabecec2eca8))

- Add protection for breaking changes in changelog configuration ([0ac1aa9](https://github.com/rsvalerio/cargo-ops/commit/0ac1aa90faa2354e470005a4c8690e3ba2512c7e))

### 🎨 Styling

- Format code ([fa7a174](https://github.com/rsvalerio/cargo-ops/commit/fa7a17499fddd4d3d64b390c26b57e44af625b39))

### 📚 Documentation

- Update documentation for workspace structure ([e9e1760](https://github.com/rsvalerio/cargo-ops/commit/e9e1760dfcc0d03a2b02f9c8c5ee22c1a0fc12d1))

- Update license in README to Apache-2.0 ([e2ddc17](https://github.com/rsvalerio/cargo-ops/commit/e2ddc17f8298f5694a02c45b3ec281431442f799))

### 🔄 CI/CD

- Switch to rust-lang/setup-rust action ([afaf494](https://github.com/rsvalerio/cargo-ops/commit/afaf494c5e1f6200ad8dfee3193bc8edc863bd1f))

- Use actions-rust-lang/setup-rust-toolchain ([f6967a7](https://github.com/rsvalerio/cargo-ops/commit/f6967a71e6b40c0c81bfda782dafae43be8d1523))

### 🔧 Build System

- Setup cargo workspace structure ([1113e3d](https://github.com/rsvalerio/cargo-ops/commit/1113e3d9dd4198660439e4087b25bf5e0c7ec5f5))

### 🚀 Features

- Add duckdb and tokei extensions ([1abae1e](https://github.com/rsvalerio/cargo-ops/commit/1abae1e5077442c8ba9fb0b3e3a8cc1c0f109a99))

### 🚜 Refactoring

- Use strum derives for Stack enum ([5fb8555](https://github.com/rsvalerio/cargo-ops/commit/5fb855564774e6cd853659ff5cea07c2fbf6ef50))

- Use toml_edit for theme config ([c898e98](https://github.com/rsvalerio/cargo-ops/commit/c898e98094697083d8d0c4888b909df6c3d9e1ce))

- Introduce ansi_style! macro ([884de2f](https://github.com/rsvalerio/cargo-ops/commit/884de2f6c060a6f8a7e47c4cb3fb13d0c1d27078))

- Extract core library crate ([a19bc64](https://github.com/rsvalerio/cargo-ops/commit/a19bc64315654f0e2b130e567b63057896676074))

- Extract runner crate for command execution ([8e42d72](https://github.com/rsvalerio/cargo-ops/commit/8e42d7263d7d6690def371b087bfc9afa5863433))

- Extract theme crate ([f8aae47](https://github.com/rsvalerio/cargo-ops/commit/f8aae47dd5d0d144de16fe2cb08f31caa46ec529))

- Extract extension trait crate ([4bede57](https://github.com/rsvalerio/cargo-ops/commit/4bede5785789d9e7ef7e8ebf724d63d41110c2a6))

- Extract cli binary crate ([152351f](https://github.com/rsvalerio/cargo-ops/commit/152351f534cddd649c59d6456f3765bee4267724))

- Move rust extensions to extensions-rust/ ([62af0fb](https://github.com/rsvalerio/cargo-ops/commit/62af0fb679ef0c2c70b919a730aa7446d78d0794))

### 🧪 Testing

- Update integration tests for workspace ([2bcf937](https://github.com/rsvalerio/cargo-ops/commit/2bcf93781e6c67c410bba5a0e3863ceebfa9bca6))
