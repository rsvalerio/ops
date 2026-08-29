# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -
## [v0.45.1](https://github.com/rsvalerio/ops/compare/7c85c9941eeef79a57b251a22c35be4d0df4b3e5..v0.45.1) - 2026-08-29
#### 🐛 Bug Fixes
- (**about**) return an empty string from truncate_to_width at zero width - ([cc8464d](https://github.com/rsvalerio/ops/commit/cc8464da22ab08ea5f9ecd8e3402f8f1a87ad876)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**about**) name the provider, target type and field path on payload errors - ([bb7fcdf](https://github.com/rsvalerio/ops/commit/bb7fcdf310b7c07a056eb321591bc75f623aea86)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) reject absolute workspace members and split resolve_member_globs - ([5f36374](https://github.com/rsvalerio/ops/commit/5f3637487bc4fe24a9cd5e1a1552b346233f1540)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) truncate by grapheme cluster instead of summing char widths - ([b2ead19](https://github.com/rsvalerio/ops/commit/b2ead19fb13a3bce3c475c088a33dd6202670ece)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-terraform**) count symlinked module directories - ([0601127](https://github.com/rsvalerio/ops/commit/06011277565d5080c10a759bf721fcde863d7d51)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**about-terraform**) track HCL braces and comments per token, not per line - ([7cb8373](https://github.com/rsvalerio/ops/commit/7cb83734b2eb18eb81c0aa3f911c74b227bd436f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config-checkers**) close JSON5 line comments at CR and U+2028/9 - ([4ca6619](https://github.com/rsvalerio/ops/commit/4ca6619485f9fafccb7cc78de75fa126f5f7fc26)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config-checkers**) bound untrusted input and cap the read itself - ([f1512ea](https://github.com/rsvalerio/ops/commit/f1512eaf6c45253b7c104e949c08c04b13ff5ac7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) reject an untrusted pre-existing ingest dir on every platform - ([5abb675](https://github.com/rsvalerio/ops/commit/5abb675d80b69513aa6bf1a45aaab9735f738ccd)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**duckdb**) refuse symlinked and in-memory ingest directories - ([627190f](https://github.com/rsvalerio/ops/commit/627190f2e81f1e977ce674d88b62c36112e6c949)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) make sql::validation crate-private and walk past nested DbError - ([c641ac5](https://github.com/rsvalerio/ops/commit/c641ac5a073bd7559c2fc4c8190146eff04b44a6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) gate load_with_sidecar DDL behind validated SQL newtypes - ([6c97245](https://github.com/rsvalerio/ops/commit/6c9724598910e797d7fb5588bc2dc483480e7158)) - [@rsvalerio](https://github.com/rsvalerio)
- (**metadata**) call CreateTableSql::as_str in the payload-cap ceiling test - ([23c1db3](https://github.com/rsvalerio/ops/commit/23c1db3026cb1a546157da9b9c532da0b36f998b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**metadata**) bound OPS_METADATA_MAX_BYTES and pin the cap-guard SQL cost - ([115ddbb](https://github.com/rsvalerio/ops/commit/115ddbba26737e3ddc4a26a5026a1f608481724d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**metadata**) reject a non-singleton metadata_raw and name every ingest path - ([72b8fb7](https://github.com/rsvalerio/ops/commit/72b8fb7e67e3495999e7cda19dcc0a3d278f3b04)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tfplan**) strip bidi and zero-width characters from table cells - ([7349661](https://github.com/rsvalerio/ops/commit/73496615ec15f8f6e9d2a91772947f5c59705a6d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**tfplan**) harden a partial plan artifact before reading the exit status - ([472d6f0](https://github.com/rsvalerio/ops/commit/472d6f044517ca86ac3dd986bb47964529147663)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**tfplan**) clean up plan artifacts on every path and bound the terraform ingress - ([bec482a](https://github.com/rsvalerio/ops/commit/bec482ae4727aaf3814f9e50813825dc248753b3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tfplan**) sanitise plan text and stop unreadable outputs rendering as no-op - ([5c394d5](https://github.com/rsvalerio/ops/commit/5c394d5dd8289538a308dc777c457c7fcdffec2c)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**about**) bound the manifest cache victim queue below the eviction cap - ([80a9822](https://github.com/rsvalerio/ops/commit/80a98221b0760179d0c6f2c65f8af606d4491140)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**config-checkers**) correct the stale accept/reject claim in the yaml header - ([4813b0e](https://github.com/rsvalerio/ops/commit/4813b0e33a82b2b9e4ddacd8051190b1cdb1a954)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config-checkers**) reattach the scanner doc and drop private doc links - ([303e57f](https://github.com/rsvalerio/ops/commit/303e57f750c9e90667f4dc76f6960d84f253d1bb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config-checkers**) state that the YAML expansion budget is stream-wide - ([eda4e7d](https://github.com/rsvalerio/ops/commit/eda4e7d120d7e9d75bedcbf95da1cd88d748e74f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
#### 🧪 Testing
- (**about**) drive every run_about_* entry point through a shared subpage context - ([9944831](https://github.com/rsvalerio/ops/commit/99448311a9ec9a519dc8550e77d5fe41a90fae4a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**metadata**) match on a reference to err in the wiring assertion - ([0b3da31](https://github.com/rsvalerio/ops/commit/0b3da31b3d00a8b846023827340ead7a93e22516)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**metadata**) replace provider assertions that cannot fail - ([7a02c42](https://github.com/rsvalerio/ops/commit/7a02c42cbea21d74e71d0620444020984a770644)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**about-terraform**) drop unused anyhow and ops-git, add ops-about test-support - ([7c85c99](https://github.com/rsvalerio/ops/commit/7c85c9941eeef79a57b251a22c35be4d0df4b3e5)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**metadata**) remove the unconsumed typed accessor layer - ([b4b5105](https://github.com/rsvalerio/ops/commit/b4b51053ed638a4f702bd7dca3162e2f0b2e9958)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) close code-review wave 172 - ([5ea65bc](https://github.com/rsvalerio/ops/commit/5ea65bcf4ecbd0e0c6479be9c5078f9010827451)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 170 - ([fb981db](https://github.com/rsvalerio/ops/commit/fb981dbc3cefb5f26503442c6aff1418588bc542)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 165 - ([19dd7da](https://github.com/rsvalerio/ops/commit/19dd7dafff066a5077417e85726ee7c71a608f0e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 169 - ([7d1c0f8](https://github.com/rsvalerio/ops/commit/7d1c0f8b6e4833b4c75588eb6502b65746f744c1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 168 - ([ec11002](https://github.com/rsvalerio/ops/commit/ec11002821ea3cf02c67a6748403c73892fa062b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 167 - ([136939b](https://github.com/rsvalerio/ops/commit/136939bad4598fa5820a1eb36145f81bf2c621f9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) drop blanket lint allows, fix stale docs, test public entry points - ([a4f024a](https://github.com/rsvalerio/ops/commit/a4f024a630c4375ddebbfab4e1eed3b3ca2cce2b)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.45.0](https://github.com/rsvalerio/ops/compare/115b413bc26f89fdb1866f309e66d20c8b6f315b..v0.45.0) - 2026-08-29
#### 🚀 Features
- (**cli**) cancel the run on SIGTERM/SIGINT instead of orphaning children - ([57ea02b](https://github.com/rsvalerio/ops/commit/57ea02b3e6a5512923c6e31f57f49ea39aa71d73)) - [@rsvalerio](https://github.com/rsvalerio)
- ![BREAKING](https://img.shields.io/badge/BREAKING-red) (**extension**) close the framework's bypassable API surfaces - ([9d606cc](https://github.com/rsvalerio/ops/commit/9d606cc950fef9b0d5736e8a92221b533bf3f329)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**about**) contain glob members in the workspace and quote untrusted log fields - ([903ad72](https://github.com/rsvalerio/ops/commit/903ad722f187edff5f251e65f9757124fa401a51)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**about-python**) degrade per field and drop non-http(s) manifest URLs - ([023b33b](https://github.com/rsvalerio/ops/commit/023b33bdcb264ad1f2821c02db611f593ed4af0a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cargo-toml**) make an omitted package field absent instead of empty - ([636a89a](https://github.com/rsvalerio/ops/commit/636a89aed8fbaf71fd9f0c42bc1a31df38902b40)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cargo-toml**) detect [workspace] headers with comments or quoted keys - ([940504e](https://github.com/rsvalerio/ops/commit/940504e3797aafeff09e3838ece883502e011faf)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cargo-update**) parse Downgrading lines and stop publishing unvalidated fields - ([b9f5195](https://github.com/rsvalerio/ops/commit/b9f5195c7b9739cbf00d052edd8168493acea61b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) compile the signal-number constants for unix only - ([87fdab6](https://github.com/rsvalerio/ops/commit/87fdab679c6e53201f5996e6fa3da6a290c1a78b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**cli**) derive help's value-taking global flags from Cli instead of transcribing them - ([e06fb0f](https://github.com/rsvalerio/ops/commit/e06fb0f569eeb36a543d16b401351d6f8323cab4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) gate prompts on the stream inquire uses and share one cancel path - ([cc37791](https://github.com/rsvalerio/ops/commit/cc37791948adf019a73b1283606941d3b1270b7c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) pin the winner of a compiled-in extension config_name collision - ([5868af1](https://github.com/rsvalerio/ops/commit/5868af1a369069d960cd956f0c71cfdbca033b8d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) count diagnostic envelopes before decoding their fields - ([22dcf3d](https://github.com/rsvalerio/ops/commit/22dcf3db4a440faf3c3bd3375275122b5068fa68)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**deps**) close the remaining cargo-upgrade fail-open and truncation paths - ([8c2aeb9](https://github.com/rsvalerio/ops/commit/8c2aeb9093e8760eb0d22c10147a53c58fb17b93)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) fail closed when cargo-deny diagnostics largely fail to decode - ([f0d90ef](https://github.com/rsvalerio/ops/commit/f0d90ef8f92c5db2482d54c738413f0105bcb1d6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python-about**) Debug-format normalized_key in the URL collision warn - ([25b3bc0](https://github.com/rsvalerio/ops/commit/25b3bc0c66e2da2c88a875f684f33878fee46f76)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**runner**) re-check group liveness before the SIGKILL escalation - ([d906ed2](https://github.com/rsvalerio/ops/commit/d906ed2a275c3d6c6544412947c5bbc73cb4786f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**runner**) fail closed when a Deny-policy cwd cannot be canonicalized - ([92c4823](https://github.com/rsvalerio/ops/commit/92c48239fe8f620424e713fe2cdd7e9dfc016e95)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) debug-format the .ops.toml env key in the SEC-002 warning - ([5ae589a](https://github.com/rsvalerio/ops/commit/5ae589aff4c8ae39663b5dd52bbc3f2a2de12531)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) observe a panicked task while a sibling floods output - ([694e14e](https://github.com/rsvalerio/ops/commit/694e14e5de73df194425f8d6d08a64b69c381d70)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) own the captured child's process tree and deny it stdin - ([92cfcd5](https://github.com/rsvalerio/ops/commit/92cfcd523e9f04f58bad6f96777d06f8bf8319fc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust-about**) treat a partial-segment members glob as unsupported - ([27cd2f4](https://github.com/rsvalerio/ops/commit/27cd2f47deec7421b4683c35866a011ff895c277)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**sec**) bound each trivy scan and refuse to report success when no scan ran - ([f5daaa8](https://github.com/rsvalerio/ops/commit/f5daaa82486389bd7ca2ae93dc333a67e417a0a2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) sanitise report title, row label and row result - ([311886c](https://github.com/rsvalerio/ops/commit/311886cc640214a8fab2019edaa26c4a250e862c)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**theme**) clamp the border title and pad flat report details - ([24721f3](https://github.com/rsvalerio/ops/commit/24721f3a15bf037b325d0d01ab0ecf037ebdc9de)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**theme**) clamp rendered lines to their column budget - ([89938e6](https://github.com/rsvalerio/ops/commit/89938e656554beaa1111f190ef88d8d6b9e586c8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) sanitise subprocess stderr before rendering it - ([8e9ca34](https://github.com/rsvalerio/ops/commit/8e9ca34aafb741681208364788dbd0c86b733459)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) strip 8-bit C1 introducers and bare control bytes - ([4a18a97](https://github.com/rsvalerio/ops/commit/4a18a979efbc9125a665df0a957f1db4dee9fe0d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) gate colour on stderr alone, not stdout or stderr - ([277e583](https://github.com/rsvalerio/ops/commit/277e583b91efebd536c1b9a0cb6f0d84ae1e9328)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**about**) bound the rust about caches' victim queues below the cap - ([b6cf657](https://github.com/rsvalerio/ops/commit/b6cf65777a5b41cb708822676e2f6c091adaf70b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**runner**) buffer tap-file writes and flush them at RunFinished - ([819e0a9](https://github.com/rsvalerio/ops/commit/819e0a90aa5518bcfc47e9cc44c7949fb21218f8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) stop the stderr tail ring pinning each step's capture buffer - ([73e9c45](https://github.com/rsvalerio/ops/commit/73e9c45df97e07e26c597558bfd0805c71891dd6)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**runner**) state the real capture-retention bound for a plan - ([c8c72b6](https://github.com/rsvalerio/ops/commit/c8c72b65f8276a556f1b0fd489d6b3613a0daefa)) - [@rsvalerio](https://github.com/rsvalerio)
- correct ownership and construction advice on two public APIs - ([13f2f2d](https://github.com/rsvalerio/ops/commit/13f2f2d2b130e9b64ff649fb55e516192055fdf1)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
#### 🧪 Testing
- (**about**) share the warn-capture harness and the dispatcher pin - ([ad55b09](https://github.com/rsvalerio/ops/commit/ad55b096c05998cbbddd1ee20a2990f1ef20c69d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-rust**) cover RustDepsProvider and the identity metrics module - ([08ec3f7](https://github.com/rsvalerio/ops/commit/08ec3f7c87c3ec761747cf7777ee69f41d273021)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cargo-toml**) assert with_root actually reaches the registered provider - ([f36806f](https://github.com/rsvalerio/ops/commit/f36806f7df440a6b84ba733bd1780ffb8c1312e5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) serialise the two run_until_signal tests - ([4044bba](https://github.com/rsvalerio/ops/commit/4044bba4acc455d06863acd49231a486163013f9)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**cli**) cover the pre-commit exit-code contract as spawned commands - ([b2d1a59](https://github.com/rsvalerio/ops/commit/b2d1a59836fa5b842f0e5f266ee0f4c6bccfa99e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) restore env vars through an RAII guard and run the lifecycle test by default - ([cbbe721](https://github.com/rsvalerio/ops/commit/cbbe721c06a05d87e2acc4c58f9fb2d3b70d2741)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) cover the deps command path and stop tests leaking global state - ([a3fad0c](https://github.com/rsvalerio/ops/commit/a3fad0ce912d78b4f162ce0ee5d8256e2bcaa119)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) cover the factory arms, the registries and error rendering - ([c35f544](https://github.com/rsvalerio/ops/commit/c35f544ae38eb1d5c2ae828f319de34094015ef0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) outlive the drain timeout in the grandchild pipe test - ([f152ba8](https://github.com/rsvalerio/ops/commit/f152ba875c71d5620818075fbc722fa5f510270b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) assert the raw payloads are gone, not that no ESC exists - ([fa1a0b3](https://github.com/rsvalerio/ops/commit/fa1a0b3818ef5600301a3578c79f3a2230ac7924)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) pin line width, truncation and error-block colour - ([23a4dc2](https://github.com/rsvalerio/ops/commit/23a4dc20ded04bf1b5122be700d334b1426a42f7)) - [@rsvalerio](https://github.com/rsvalerio)
- stop two assertions passing for the wrong reason - ([1bac837](https://github.com/rsvalerio/ops/commit/1bac8379c8c8a382d0b9614d91f055b8e0be801e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
#### 🔧 Build System
- (**about-python**) drop unused anyhow and ops-git deps - ([892264d](https://github.com/rsvalerio/ops/commit/892264dc8b4f7a161ef938dc0d69ca6bb1407229)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) inherit bitflags from the workspace dependency table - ([115b413](https://github.com/rsvalerio/ops/commit/115b413bc26f89fdb1866f309e66d20c8b6f315b)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) centralise the control-char and URL-scheme guards - ([be24389](https://github.com/rsvalerio/ops/commit/be2438946c4226048e3b73b63967d4e6c3ec868e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-rust**) resolve members against the workspace root, not the cwd - ([2dd9ad9](https://github.com/rsvalerio/ops/commit/2dd9ad9f8cc280aceed5b448ca71b81ac055e676)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) read Context through accessors and acknowledge register - ([cb25fd3](https://github.com/rsvalerio/ops/commit/cb25fd36fe4ffd18bc5520cf8acd9c6e02f4d304)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) file TASK-2040 for glob workspace.exclude entries - ([32545de](https://github.com/rsvalerio/ops/commit/32545de0c0ef1821d37863813efa95ebf20effad)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**backlog**) close code-review wave 163 - ([0642cb9](https://github.com/rsvalerio/ops/commit/0642cb9eaa0775bea274c99248d3715cdac15d33)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 161 - ([1a57baa](https://github.com/rsvalerio/ops/commit/1a57baa628a8c9f510c0f5792ca8dba704e75ef0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 159 - ([71368f1](https://github.com/rsvalerio/ops/commit/71368f12ce8567ebc8abd407a99722a98bd035d6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 160 - ([2849fce](https://github.com/rsvalerio/ops/commit/2849fce0b05a494919317c6aebd5af84338dff3c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 158 - ([99fba2a](https://github.com/rsvalerio/ops/commit/99fba2af6e23c064e3ba02370142d17faebdf7b7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 152 - ([fa99824](https://github.com/rsvalerio/ops/commit/fa99824de26823e7dd16757cd0853012c811107f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 148 - ([daf47c5](https://github.com/rsvalerio/ops/commit/daf47c5cb45e7f281e5fe8e26383b118b321a9a3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 153 - ([85fd2c5](https://github.com/rsvalerio/ops/commit/85fd2c5b2ab0219e216fd343d94ab285b58fc789)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 151 - ([dd744ac](https://github.com/rsvalerio/ops/commit/dd744acc883a71cbcce190ec459c94b46057c573)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.44.0](https://github.com/rsvalerio/ops/compare/0fc0f4ddc5720e0f334d4014bc55b3c4776c736c..v0.44.0) - 2026-08-28
#### 🚀 Features
- (**run-before-push**) gate the hook on git's ref-update stream - ([07e4d33](https://github.com/rsvalerio/ops/commit/07e4d33be4ea3898f6f2f4ce1d19599d9e33fed4)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**about-go**) reject go.work use directives with embedded parent-dir segments - ([f29b1a8](https://github.com/rsvalerio/ops/commit/f29b1a8164f724bdac9e0030140101265544c6f4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-go**) parse quoted, tab-separated and block-form directives - ([fec0277](https://github.com/rsvalerio/ops/commit/fec02774fe251c03ebcb606452b4685e4550a0b6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-go**) tokenize modfile lines and share the traversal guard - ([5ede2e6](https://github.com/rsvalerio/ops/commit/5ede2e6f4f8de6166b4f5bc526d1763ff85eb210)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-java**) scope gradle description to depth 0 and share the escape-aware quote scan - ([95913a9](https://github.com/rsvalerio/ops/commit/95913a940b3b1ccf6925b54283f9791b112d489d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-java**) stop single-line POM containers leaking into top-level fields - ([5063c44](https://github.com/rsvalerio/ops/commit/5063c4498c347a49b1a3c163b5668b9efe9fb29a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-node**) drop blank workspace globs and strip YAML comments by char - ([71d1da2](https://github.com/rsvalerio/ops/commit/71d1da2d5bc8be61127f3af312504231eb5ac73b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-node**) reject repository URLs outside the http(s) allowlist - ([850cf8c](https://github.com/rsvalerio/ops/commit/850cf8c03b0c6f1f77319b34b886bfe05b7cf21e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**create-review-tasks**) validate provider input and harden id allocation - ([fa67e96](https://github.com/rsvalerio/ops/commit/fa67e96c0f6503dab918caa7274617bf12c40f9c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**create-review-tasks-rust**) resolve a review target for every supported project shape - ([ac6f363](https://github.com/rsvalerio/ops/commit/ac6f363939be562e79e1f9888d2323324bbd32e2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java**) stop a single-line <developers> block hijacking the project name - ([0feb21f](https://github.com/rsvalerio/ops/commit/0feb21f7cdf3abc05b4e00fba0a71fd015513d47)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**rust-loc**) bound the streaming fallback by the reader, not the line - ([77c1af9](https://github.com/rsvalerio/ops/commit/77c1af9385ce4ff04b12e4627c3afb56d5bb0095)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**rust-loc**) bound scan recursion and file size, and walk in parallel - ([5228f63](https://github.com/rsvalerio/ops/commit/5228f631c11507874730c48b857c6944eb9903c7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**test-coverage**) COALESCE coverage_summary counts so an empty table yields 0 - ([5908d2c](https://github.com/rsvalerio/ops/commit/5908d2c9f497a0efd61998f488ee41f70a21d05d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tokei**) bound the source scan and anchor its exclusions to the root - ([bf3535d](https://github.com/rsvalerio/ops/commit/bf3535d3bfe4f87ded3e88e88026f249d0a46e6e)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**clippy**) refresh the site-local allow census after the drain - ([49aef12](https://github.com/rsvalerio/ops/commit/49aef12d34ad26f214bee4a12752eb0b5f4ab8b5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**readme**) document the hook skip environment variables - ([5077d82](https://github.com/rsvalerio/ops/commit/5077d821ef652f5374b5bd5773ec1332475261d2)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**about**) add a shared warn-counting tracing harness - ([3b3ed0e](https://github.com/rsvalerio/ops/commit/3b3ed0e1f0ad3c44151b1cda815eff88021abd8e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) add a shared write_file fixture helper to test_support - ([b75d82b](https://github.com/rsvalerio/ops/commit/b75d82bb6adb4272ebe1033b59822ee59e56e37a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-java**) assert the exact directory-name fallback and pin the new parser invariants - ([22107ea](https://github.com/rsvalerio/ops/commit/22107eac5d239f7224240e80374136231a90a964)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-node**) use the shared write_file fixture helper - ([a3af5c8](https://github.com/rsvalerio/ops/commit/a3af5c822fbfb4c4d2cf19bc9238465f8373e311)) - [@rsvalerio](https://github.com/rsvalerio)
- (**test-coverage**) split tests.rs into src/tests/ by concern - ([ae7dace](https://github.com/rsvalerio/ops/commit/ae7dace14b4c3479e4dd094ce36becd6b6d81432)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tokei**) pin that unsupported files do not consume the file cap - ([b0574c8](https://github.com/rsvalerio/ops/commit/b0574c8449de9fcd9511c4c177b999600e09d790)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**tokei**) replace live workspace scans with a canned fixture - ([10a911d](https://github.com/rsvalerio/ops/commit/10a911d152bc2e117e9a37c15a87468d0316b866)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**about-go**) drop the unused anyhow and ops-git dependencies - ([827757c](https://github.com/rsvalerio/ops/commit/827757c56b1bf8a42a339877a112ecf447a24b95)) - [@rsvalerio](https://github.com/rsvalerio)
- (**create-review-tasks**) depend on chrono for calendar arithmetic - ([289f4fd](https://github.com/rsvalerio/ops/commit/289f4fd869ba1ec13c42aa9288488f4ad143b45c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**create-review-tasks-rust**) add tracing-capture and serial test dependencies - ([9b61c34](https://github.com/rsvalerio/ops/commit/9b61c342309e470942f3e6846c73f4fd4e0ac1f6)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about-go**) consume go_mod::GoMod directly - ([8c74430](https://github.com/rsvalerio/ops/commit/8c74430b242908bc7148add0a905fd07a9d4df06)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-rust**) re-export the crate-manifest reader and member-path guard - ([1b48a8a](https://github.com/rsvalerio/ops/commit/1b48a8a9ac84f688645c280b98c01328efd1b3d3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**test-coverage**) name the soft-fail predicate and inject the cargo runner - ([222e1eb](https://github.com/rsvalerio/ops/commit/222e1eb090fa1e97a8663ae4afac0726cf1e0db2)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**about-java**) drop unused deps and the dead cast-lint allows - ([26edee4](https://github.com/rsvalerio/ops/commit/26edee4dfaede42b35ed5107df32b986d9d19a77)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 164 - ([3cc1921](https://github.com/rsvalerio/ops/commit/3cc1921c1b91dbb2be59c790e73589cf229e45d0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 178 - ([bf36283](https://github.com/rsvalerio/ops/commit/bf3628395ae15932e026772a7ef41ec3157ce7d1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 171 - ([f87d09c](https://github.com/rsvalerio/ops/commit/f87d09c1a3d69b52fe51d14a023f29ba83644701)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 166 - ([5e53d0d](https://github.com/rsvalerio/ops/commit/5e53d0d68b56d03c8f99e642fe18119c499f481b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 156 - ([826c291](https://github.com/rsvalerio/ops/commit/826c291a811c8a8b4394754da90094938a65cc16)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 155 - ([2e51c21](https://github.com/rsvalerio/ops/commit/2e51c21c8a6b12e0a4ee7f6365969320589ecea5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 157 - ([73628b7](https://github.com/rsvalerio/ops/commit/73628b74858a680488a501657ec137ad1fe3b2f9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 162 - ([bf9c1bb](https://github.com/rsvalerio/ops/commit/bf9c1bb186d6b5ae75614ff819fa5cdf952426e4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 176 - ([56dc563](https://github.com/rsvalerio/ops/commit/56dc56376a43c3782f2233da9796d050fd982230)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close code-review wave 154 - ([504b87d](https://github.com/rsvalerio/ops/commit/504b87d4b63899f03d0e428ba8971771745b6ab0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) triage 262 findings into waves 148-178 - ([2bc1a54](https://github.com/rsvalerio/ops/commit/2bc1a543cf63829b7c1250a7798366043d8231da)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**coderabbit**) skip reviewing the .backlog directory - ([480f6b9](https://github.com/rsvalerio/ops/commit/480f6b9d512ef7590f665d37bb95db7a9ce1d22e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**test-coverage**) drop three dead cast allows and give unwrap_used a reason - ([c22214b](https://github.com/rsvalerio/ops/commit/c22214bb3fb940bd3abd9dd52a7edda64c0f3fee)) - [@rsvalerio](https://github.com/rsvalerio)
- save backlog tasks - ([d13dca5](https://github.com/rsvalerio/ops/commit/d13dca5f337e4324ddb3076580dab786fb25aff4)) - [@rsvalerio](https://github.com/rsvalerio)
- save backlog tasks - ([0fc0f4d](https://github.com/rsvalerio/ops/commit/0fc0f4ddc5720e0f334d4014bc55b3c4776c736c)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.43.1](https://github.com/rsvalerio/ops/compare/78b62d1125f328dc92fe36ddd94939b60165cbca..v0.43.1) - 2026-08-27
#### 🐛 Bug Fixes
- (**about**) pin in-flight cache entries and reject partial-segment globs - ([fcdf5c2](https://github.com/rsvalerio/ops/commit/fcdf5c256948f4c708cba0c8449fb093bc40b7eb)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**lints**) keep the panic fixes const-callable and if-let-free - ([109fb04](https://github.com/rsvalerio/ops/commit/109fb04ee0076fe8b4660d63716e3f663c0c4954)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
#### 📚 Documentation
- (**clippy**) point the temporary-allow example at a live entry - ([9c07a01](https://github.com/rsvalerio/ops/commit/9c07a019fff9485d0679b1e8ecb57627b43cdeb6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**clippy**) explain the CLI integration test expect allow - ([530e4ce](https://github.com/rsvalerio/ops/commit/530e4ce6f560fb303258e417e1211759e0b5ca49)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**clippy**) give doc comments a one-line summary paragraph - ([3860e21](https://github.com/rsvalerio/ops/commit/3860e21046087f44748c435b1915adfc10138b4a)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**clippy**) deny expect_used, unreachable and panic_in_result_fn - ([7a7e1f3](https://github.com/rsvalerio/ops/commit/7a7e1f3be783b8dfd77a85aa3517e011ddb862a6)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**clippy**) deny arithmetic_side_effects and as_conversions workspace-wide - ([da28b99](https://github.com/rsvalerio/ops/commit/da28b9949cfb3145e6eb4a950a24794107d24675)) - [@rsvalerio](https://github.com/rsvalerio)
- (**clippy**) deny indexing_slicing and string_slice workspace-wide - ([b5aee7a](https://github.com/rsvalerio/ops/commit/b5aee7ae8a77c422f36e71397ce08ef798aa153e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**clippy**) deny significant_drop_tightening workspace-wide - ([9c0c729](https://github.com/rsvalerio/ops/commit/9c0c7290d9401412df7b73801b3bd8a661dbada9)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**lints**) enforce redundant_pub_crate, missing_const_for_fn and use_self - ([fc153c6](https://github.com/rsvalerio/ops/commit/fc153c6e22795132400482e101350837d5c7fa3b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**lints**) drop 15 lints from the temporary-allow block - ([11e29d2](https://github.com/rsvalerio/ops/commit/11e29d258c8e297e063cef4ba142e377e38be022)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
#### 🚜 Refactoring
- (**about**) drop the option_if_let_else allow via a named recovery helper - ([90db7fc](https://github.com/rsvalerio/ops/commit/90db7fc265b7f57b8e087d3238d10c88027b43ea)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**clippy**) make integer arithmetic and casts total in the extensions - ([b033e8c](https://github.com/rsvalerio/ops/commit/b033e8c68ed585fb048512df2c47b80a52512dfc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**clippy**) make integer arithmetic and casts total in the core crates - ([5eff373](https://github.com/rsvalerio/ops/commit/5eff373ce20406c44fd3ef735c59f516d45c5f0b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**clippy**) enforce option_if_let_else workspace-wide - ([b843254](https://github.com/rsvalerio/ops/commit/b843254d96e9b08ea87607317eff11e99f1c1deb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core,runner,cli**) remove panicking index and str slicing - ([b5d822b](https://github.com/rsvalerio/ops/commit/b5d822b3a389f0095c74b7cbd36397d3ec225d7d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**extensions**) remove panicking index and str slicing - ([c251f0c](https://github.com/rsvalerio/ops/commit/c251f0c9ef45b105c8af4fc2eb8186e8bc504f9d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**lints**) declare items with pub, const fn and Self - ([f2e9c78](https://github.com/rsvalerio/ops/commit/f2e9c78f08cbc6b698349f64fe5876ffad462684)) - [@rsvalerio](https://github.com/rsvalerio)
- (**lints**) clear the small-count clippy tail - ([f308fbf](https://github.com/rsvalerio/ops/commit/f308fbf5295154ec338dd59cc32fb81083c4bd14)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**lints**) remove or justify every production expect() call - ([3d45015](https://github.com/rsvalerio/ops/commit/3d45015dd64635d72c17523b4dc03f69c17df91d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**locks**) release mutex guards at their last use - ([ea459d5](https://github.com/rsvalerio/ops/commit/ea459d56605cff444d5f1997bdc7474e1ceb42bd)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
#### ⚙️ Miscellaneous
- (**backlog**) close TASK-1691 - ([752f446](https://github.com/rsvalerio/ops/commit/752f446cd23b67096f2501295fbcdd922c3a9bde)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**backlog**) close code-review wave 142 (TASK-1684) with TASK-1671 and TASK-1674 - ([6b19dc2](https://github.com/rsvalerio/ops/commit/6b19dc244636747c447c6ff95c43dd430512aebb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave141 (TASK-1683) with TASK-1672 and TASK-1673 - ([fdf184d](https://github.com/rsvalerio/ops/commit/fdf184d5a10cce160d646bc7dbed5225c468df4d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**backlog**) close wave144 (TASK-1686) with TASK-1676, TASK-1677 and TASK-1678 - ([0c01483](https://github.com/rsvalerio/ops/commit/0c014835dc8fba4ff4997489137ad1e3968ef816)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**backlog**) close wave143 (TASK-1685) with TASK-1675 and TASK-1682 - ([9a1a69a](https://github.com/rsvalerio/ops/commit/9a1a69a90a0a71905b97cdb9333db78acac6f250)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**backlog**) close wave145 (TASK-1687) and TASK-1680 - ([3d429d4](https://github.com/rsvalerio/ops/commit/3d429d411ee02ca9c263e31db04380e1ade33a5d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**backlog**) close wave147 (TASK-1689) and TASK-1681 - ([0ec4681](https://github.com/rsvalerio/ops/commit/0ec4681d127001d967686d3dafec5bd2b6a08a9f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**backlog**) close wave146 (TASK-1688) and TASK-1679 - ([734347d](https://github.com/rsvalerio/ops/commit/734347d4159afb353dcffc947b430666f0addd68)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) triage waves 141-147 into To Do - ([78b62d1](https://github.com/rsvalerio/ops/commit/78b62d1125f328dc92fe36ddd94939b60165cbca)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
#### 🎨 Styling
- (**clippy**) rustfmt the rebased conflict resolutions - ([b9a199e](https://github.com/rsvalerio/ops/commit/b9a199e75c0c1f237a44c5ca7474dc52f4070260)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) rustfmt the rebased line-scan condition - ([3b43761](https://github.com/rsvalerio/ops/commit/3b437610ce332e4d1e79d9b58d221a31ca2e76a9)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)

- - -

## [v0.43.0](https://github.com/rsvalerio/ops/compare/d64661547d9e54fe1af1f1e8fc56094510a50aaf..v0.43.0) - 2026-08-26
#### 🚀 Features
- (**cli**) wire the create-review-tasks subcommand - ([3625b59](https://github.com/rsvalerio/ops/commit/3625b59ab479eaf9fc9c49368a7dceab01905469)) - [@rsvalerio](https://github.com/rsvalerio)
- (**create-review-tasks**) add the backlog review-request task engine - ([24a3e6b](https://github.com/rsvalerio/ops/commit/24a3e6b1725e32dda8a46c1769d909453180f7d6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**create-review-tasks-rust**) provide cargo workspace members as review targets - ([c937668](https://github.com/rsvalerio/ops/commit/c937668aa25dd21b2806432738ce3757510f75dd)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**create-review-tasks**) re-check the id claim after reserving the main task - ([16bd3c1](https://github.com/rsvalerio/ops/commit/16bd3c18081f77b44ae99e78dee588b6814f6bee)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
- (**create-review-tasks**) make task-set creation collision- and failure-safe - ([0261793](https://github.com/rsvalerio/ops/commit/026179344bcb831071b04359455fd1cdb8c42e1e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context)
#### 📚 Documentation
- (**clippy**) document the deny levels and the temporary-allow block - ([3b8fe31](https://github.com/rsvalerio/ops/commit/3b8fe31c106d29a646a11ffefcaa46ab91c0f926)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**lints**) enable clippy pedantic and nursery at deny - ([d646615](https://github.com/rsvalerio/ops/commit/d64661547d9e54fe1af1f1e8fc56094510a50aaf)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about-rust**) export the resolved workspace member view - ([a1aa792](https://github.com/rsvalerio/ops/commit/a1aa792537362f8e8cfc87f4549d1635dca95c21)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) file TASK-1671..1682 to empty the clippy allow block - ([0eb6c71](https://github.com/rsvalerio/ops/commit/0eb6c712a58777c761015b69ba9899cd8c580996)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.42.0](https://github.com/rsvalerio/ops/compare/5495c49316e999739e910dd81188ab4428d6756d..v0.42.0) - 2026-08-23
#### 📚 Documentation
- update command-mappings references - ([2cfd78a](https://github.com/rsvalerio/ops/commit/2cfd78ad1b02b8a4e4c5e03d352b08a51df8c9c6)) - [@rsvalerio](https://github.com/rsvalerio)
- sync README and AGENTS.md with the current CLI surface - ([fd94b64](https://github.com/rsvalerio/ops/commit/fd94b641c45cc77f6e6aa57963fab09cffdbe407)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚀 Features
- (**qa**) fold nextest commands and qa legs into the rust stack defaults - ([5495c49](https://github.com/rsvalerio/ops/commit/5495c49316e999739e910dd81188ab4428d6756d)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- update run-before-push to use qax instead of qa - ([babc41b](https://github.com/rsvalerio/ops/commit/babc41b14f9df2d949e6154d30864af86e7ce280)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.41.0](https://github.com/rsvalerio/ops/compare/e4a3cd1b47e6efcba4003ce0e504f5b1c95ad8ee..v0.41.0) - 2026-08-23
#### 🚀 Features
- (**runner**) render builtin commands as ops subcommand - ([e4a3cd1](https://github.com/rsvalerio/ops/commit/e4a3cd1b47e6efcba4003ce0e504f5b1c95ad8ee)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- Update crates/core/src/config/commands.rs 🐰 - ([f92b6f3](https://github.com/rsvalerio/ops/commit/f92b6f3f904057353472cba09533c17141c2d9cf)) - [@rsvalerio](https://github.com/rsvalerio), coderabbitai[bot]

- - -

## [v0.40.0](https://github.com/rsvalerio/ops/compare/4d8397580e8db64922daba285883931ec6c82eac..v0.40.0) - 2026-08-23
#### 🚀 Features
- (**qa**) wire the sec scanner into the qa composite - ([4d83975](https://github.com/rsvalerio/ops/commit/4d8397580e8db64922daba285883931ec6c82eac)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**hook-common**) use is_ok_and for the skip env check - ([b3e09a2](https://github.com/rsvalerio/ops/commit/b3e09a2f4c1de9d2640883fe8711d41d99fb9be4)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.39.0](https://github.com/rsvalerio/ops/compare/43515288bf7adc89380ae8c7dc11076409703c83..v0.39.0) - 2026-08-19
#### 🚀 Features
- (**test-coverage**) hint how to install cargo-llvm-cov when coverage collect fails - ([30cdbd9](https://github.com/rsvalerio/ops/commit/30cdbd9c4da4ad41618ca9e651889b86ae75d0b5)) - Rodrigo Valerio, Claude
#### 🐛 Bug Fixes
- (**extension**) render the full error chain in DataProviderError display - ([4351528](https://github.com/rsvalerio/ops/commit/43515288bf7adc89380ae8c7dc11076409703c83)) - Rodrigo Valerio, Claude
- apply CodeRabbit auto-fixes - ([6d600c3](https://github.com/rsvalerio/ops/commit/6d600c33da03e82e450d3b98d55a8875f775082d)) - coderabbitai[bot], CodeRabbit

- - -

## [v0.38.0](https://github.com/rsvalerio/ops/compare/49f140ea25fe972dc878cecc057a55161ae733bf..v0.38.0) - 2026-08-18
#### 🚀 Features
- ![BREAKING](https://img.shields.io/badge/BREAKING-red) (**tools**) remove the ops tools extension and its [tools] config section - ([49f140e](https://github.com/rsvalerio/ops/commit/49f140ea25fe972dc878cecc057a55161ae733bf)) - Rodrigo Valerio
#### 📚 Documentation
- drop ops tools references from the README - ([0cbdd69](https://github.com/rsvalerio/ops/commit/0cbdd690ee1058d9321949406ffd9e1b400e6b79)) - Rodrigo Valerio

- - -

## [v0.37.0](https://github.com/rsvalerio/ops/compare/7218bb2ec09f24e128bbbd10b57896648661598c..v0.37.0) - 2026-08-18
#### 🚀 Features
- adopt cargo-nextest as the workspace test runner - ([5ecdf13](https://github.com/rsvalerio/ops/commit/5ecdf13cdca8ac0930dda9193e8c525a366d8e53)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**clippy**) record the settled policy, MSRV rationale and exception census - ([92fbe0f](https://github.com/rsvalerio/ops/commit/92fbe0f0d63d1bbf1b2b873bb805db175490bb47)) - Rodrigo Valerio
- (**clippy**) document lint policy, exceptions and how to add one - ([e6de2ac](https://github.com/rsvalerio/ops/commit/e6de2ac199ca5fd9a579d22766b8a520675a1828)) - Rodrigo Valerio
- (**duckdb**) record the Phase 3 dry-run verification - ([afbc740](https://github.com/rsvalerio/ops/commit/afbc74015a7ebf844c88d1cf9aa9cf0a756ae5f8)) - Rodrigo Valerio, Claude
- document error conditions and add must_use attributes workspace-wide - ([932c76f](https://github.com/rsvalerio/ops/commit/932c76f227cc958e9a884c8d73be17697ad4604e)) - Rodrigo Valerio
#### 🧪 Testing
- (**about**) assert operation counts instead of wall-clock time - ([7891deb](https://github.com/rsvalerio/ops/commit/7891deb42111946f8b5e3383ca1b862cc59ad708)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**about**) serialize every test that reaches the typed manifest cache - ([29f5dab](https://github.com/rsvalerio/ops/commit/29f5dabe401df1f7300c830f7b08535481351282)) - Rodrigo Valerio
- (**core**) test the poison breadcrumb on a local mutex, not the global cache - ([7aefde0](https://github.com/rsvalerio/ops/commit/7aefde0b7820ba126c893f2512ae26f58a18fcc5)) - Rodrigo Valerio
- (**core,runner**) drop two wall-clock assertions that cannot fail except under load - ([71fb164](https://github.com/rsvalerio/ops/commit/71fb164063784cbdb972d6e3e49404ab9225cb2b)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**git**) pin a global tracing dispatcher in the config capture test - ([e1704e5](https://github.com/rsvalerio/ops/commit/e1704e542a3562a2148562679af5028e3cf0d330)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**tools**) co-locate tests with the modules they cover - ([1935793](https://github.com/rsvalerio/ops/commit/19357935f59cd282b960890908e143fe09c75fd8)) - Rodrigo Valerio
#### 🔧 Build System
- (**duckdb**) link a prebuilt libduckdb instead of compiling the amalgamation - ([a4884d6](https://github.com/rsvalerio/ops/commit/a4884d6ebe85cd46f7118f404939b49258733785)) - Rodrigo Valerio, Claude
- (**lints**) raise rust-version to 1.88 and enforce the remaining pedantic lints - ([d6df7a3](https://github.com/rsvalerio/ops/commit/d6df7a3a5bbe81189eb9a3bba39e5a93ab769274)) - Rodrigo Valerio
- (**lints**) centralize clippy and rustc policy in [workspace.lints] - ([7218bb2](https://github.com/rsvalerio/ops/commit/7218bb2ec09f24e128bbbd10b57896648661598c)) - Rodrigo Valerio
- (**msrv**) declare rust-version 1.97 and write timeouts in minutes - ([bc5217d](https://github.com/rsvalerio/ops/commit/bc5217d5d1dc345e458a7b66ab1781e129d17542)) - Rodrigo Valerio
#### 🔄 CI/CD
- (**bump**) call forge's bump workflow at @v1 again - ([e300309](https://github.com/rsvalerio/ops/commit/e300309e17a14aac9c4eb05a9f43dd4039030da3)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**msrv**) build on the declared rust-version and pin clippy.toml to it - ([a5bff04](https://github.com/rsvalerio/ops/commit/a5bff04a12f53b36f252f87577291c5bdcfbb89c)) - Rodrigo Valerio
- (**release**) install the workspace-pinned Rust toolchain on build legs - ([4f733c2](https://github.com/rsvalerio/ops/commit/4f733c28572585f33b6af7ea8a7ccaee24ec6a86)) - Rodrigo Valerio, Claude
#### 🚜 Refactoring
- (**core**) split config/mod.rs into root, sections, command_id and init - ([d0c79f1](https://github.com/rsvalerio/ops/commit/d0c79f1b5da68a705f847aab636e17aa88c0c848)) - Rodrigo Valerio
- (**deps**) co-locate tests with the modules they cover - ([cf1ce96](https://github.com/rsvalerio/ops/commit/cf1ce9674d155a0b843f2453c8b76d6873c2b931)) - Rodrigo Valerio
- (**metadata**) build cargo-metadata test fixtures from a shared module - ([aa95980](https://github.com/rsvalerio/ops/commit/aa9598021f79af4f4fec6d2481642b0ccef1a8ac)) - Rodrigo Valerio
- take parameters by reference or Copy where they are not consumed - ([27356eb](https://github.com/rsvalerio/ops/commit/27356eb2b2450ad9ca4fedb6cbb37d4cc7eadf0b)) - Rodrigo Valerio
- satisfy clippy::pedantic across the workspace - ([cfb03a9](https://github.com/rsvalerio/ops/commit/cfb03a9343ab3336f228c648c839d89ee01d0892)) - Rodrigo Valerio
#### ⚙️ Miscellaneous
- (**backlog**) close TASK-1664 with the AC #6 closeout - ([2b20718](https://github.com/rsvalerio/ops/commit/2b207188795810277270a754cd8d517d3b5bce32)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**backlog**) record AC #6 progress on TASK-1664 - ([570970f](https://github.com/rsvalerio/ops/commit/570970fa007796989c47f4c58933af17b2d82f67)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**backlog**) close TASK-1540 and code-review waves 122 and 124 - ([50333e4](https://github.com/rsvalerio/ops/commit/50333e42d54e0036aaf0fe121493e1a061a53f06)) - Rodrigo Valerio
- (**backlog**) close TASK-1670 with the deps split write-up - ([b1281a7](https://github.com/rsvalerio/ops/commit/b1281a79d0868250f31c228e6328b4c7d33f44f1)) - Rodrigo Valerio
- (**backlog**) record the typed-manifest-cache race on TASK-1664 - ([5d59cf7](https://github.com/rsvalerio/ops/commit/5d59cf786bc5b8a726b83e23c1eb83ab6c87f730)) - Rodrigo Valerio
- (**backlog**) record the poison-breadcrumb fix on TASK-1664 - ([8d3e3c0](https://github.com/rsvalerio/ops/commit/8d3e3c01e77d638aa3e63b377b30bd4b14fd2852)) - Rodrigo Valerio
- (**backlog**) record the MSRV CI guard on TASK-1669 - ([58a7819](https://github.com/rsvalerio/ops/commit/58a7819d492d1d7ebe03d8c552d6d7b5e22530f1)) - Rodrigo Valerio
- (**backlog**) record the 1.97 floor decision on TASK-1669 - ([457c6a5](https://github.com/rsvalerio/ops/commit/457c6a5649efdd9aa7721e8b8d5312971fe3cd9c)) - Rodrigo Valerio
- (**backlog**) close TASK-1666, TASK-1668, TASK-1669; file TASK-1670 - ([7b47f36](https://github.com/rsvalerio/ops/commit/7b47f36ee5e9a6971d60f1ce14978f6b85e5241b)) - Rodrigo Valerio
- (**backlog**) close TASK-0137, TASK-0165, TASK-1567; file TASK-1666, TASK-1667 - ([a5f3039](https://github.com/rsvalerio/ops/commit/a5f3039e7f016c94b88c32c5c41b57294e8672e7)) - Rodrigo Valerio
- Merge pull request #21 from rsvalerio/refactor/enforce-pedantic-and-raise-msrv - ([dc29fe5](https://github.com/rsvalerio/ops/commit/dc29fe54f4ee13c985bdbc9d6e0f1f485c22fb9b)) - [@rsvalerio](https://github.com/rsvalerio)
- Merge pull request #22 from rsvalerio/fix/ops-root-cache-serial-group - ([47b75e3](https://github.com/rsvalerio/ops/commit/47b75e36ef32eb57df659bcce8fbcbde2b28f176)) - [@rsvalerio](https://github.com/rsvalerio)
- Merge pull request #20 from rsvalerio/refactor/lint-policy-and-module-splits - ([ab6f2e5](https://github.com/rsvalerio/ops/commit/ab6f2e53949bc5c85ad512e37ed652034620044c)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.36.2](https://github.com/rsvalerio/ops/compare/60a78e001f3a2e9de8d8edb2b1573d96301ecf16..v0.36.2) - 2026-08-15
#### 🐛 Bug Fixes
- (**tools**) cargo --list probe misreads every tool as missing under forced colour - ([f54757e](https://github.com/rsvalerio/ops/commit/f54757e348ff2e464dc620f69702e4e6fbb04f91)) - Rodrigo Valerio
#### 📚 Documentation
- (**changelog**) flag the v0.36.1 breaking change and how to fix it (#17) - ([ed8adf9](https://github.com/rsvalerio/ops/commit/ed8adf993d44e51f474b94ab9e1683de9b6d603a)) - [@rsvalerio](https://github.com/rsvalerio)
- ![BREAKING](https://img.shields.io/badge/BREAKING-red) (**releasing**) document that squash merges drop commit subjects - ([e50c832](https://github.com/rsvalerio/ops/commit/e50c832290f2ac03c978019eed6ddccec47d46fe)) - Rodrigo Valerio
#### 🧪 Testing
- (**core,runner**) replace wall-clock assertions with behavioural seams - ([98e9ef6](https://github.com/rsvalerio/ops/commit/98e9ef62fda7bce795da24c8b0c8076dcf84096d)) - Rodrigo Valerio
- (**runner**) assert step duration at nanosecond, not millisecond, granularity - ([6e795fc](https://github.com/rsvalerio/ops/commit/6e795fc5b86ea01326d3c53a9ce42a8f92a905c8)) - Rodrigo Valerio
- (**tools**) make the fake-cargo fixture argument-position independent - ([d9bd7ab](https://github.com/rsvalerio/ops/commit/d9bd7ab54cd1b5340ba7319985d5fa18723a1250)) - Rodrigo Valerio
#### 🔄 CI/CD
- run the full test suite, not only ignored tests - ([7257e53](https://github.com/rsvalerio/ops/commit/7257e5360a2aaf2bdba5e234781aab02a5248803)) - Rodrigo Valerio
#### ⚙️ Miscellaneous
- (**backlog**) triage nine stale In Progress tasks - ([f3cc88e](https://github.com/rsvalerio/ops/commit/f3cc88e4acc9abea8725aea19ceb8f178cddb923)) - Rodrigo Valerio
- (**backlog**) close TASK-1656..1663 as done - ([60a78e0](https://github.com/rsvalerio/ops/commit/60a78e001f3a2e9de8d8edb2b1573d96301ecf16)) - Rodrigo Valerio
- Merge pull request #19 from rsvalerio/fix/ci-runs-full-test-suite - ([0061411](https://github.com/rsvalerio/ops/commit/0061411cf249f4d35c23d3b7f7e94daca921d142)) - [@rsvalerio](https://github.com/rsvalerio)
- Merge pull request #18 from rsvalerio/docs/breaking-changes-squash-caveat - ([50a1058](https://github.com/rsvalerio/ops/commit/50a10585995a07eecdc43b2915a7e2ca1344c7d9)) - [@rsvalerio](https://github.com/rsvalerio)
- Merge pull request #16 from rsvalerio/chore/backlog-triage-stale-in-progress - ([591630c](https://github.com/rsvalerio/ops/commit/591630c38934214ebecc0b580b77cd86c330de83)) - [@rsvalerio](https://github.com/rsvalerio)
- Merge pull request #15 from rsvalerio/chore/backlog-reconcile-1656-1663 - ([c73bb89](https://github.com/rsvalerio/ops/commit/c73bb897bba922258170baa171cada0e063e79df)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.36.1](https://github.com/rsvalerio/ops/compare/8468d674d01c9aa92cbd3d1ee01312a1f425651c..v0.36.1) - 2026-08-15

> **⚠️ Breaking change, despite the patch version number.**
>
> This release contains a breaking change but is versioned as a patch
> (`0.36.0` → `0.36.1`). That is expected, not an oversight: while the project
> is on `0.y.z`, `cog bump --auto` [will not move it to `1.0.0`](https://docs.cocogitto.io/guide/bump.html)
> for a breaking change, so the version falls through to the `fix:` rule. See
> "Breaking Changes" in `docs/releasing.md`.
>
> The consequence is that the version number alone does not warn you, hence
> this note.
>
> **What changed.** A `.ops.toml` command group (`commands = [...]`) tree is
> expanded into one flat plan and scheduled as a single unit, so the `parallel`
> and `fail_fast` flags are plan-wide. Previously they were OR-folded across the
> tree, which let one `parallel = true` group silently promote an entire
> `parallel = false` plan to parallel — the config said sequential and the steps
> ran concurrently. Such a config is now **rejected** instead of running.
>
> **Who is affected.** Only configs that nest groups with *differing* `parallel`
> or `fail_fast` values. Flat configs, and nested groups whose flags agree, are
> unaffected. No stack default triggers it.
>
> **What you will see.**
>
> ```console
> $ ops verify
> ops: error: conflicting `parallel` in the plan for `verify`: `verify` sets parallel = false, but `lint` sets parallel = true
> ops: error:   composite commands are flattened into one plan and scheduled as a single unit, so mixed `parallel` values cannot both be honoured
> ops: error:   fix: make them agree — set `lint.parallel = false`, or set `verify.parallel = true`
> ```
>
> **How to fix.** Make the flags agree, as the error says. Choose `false`
> throughout if any step in the plan writes files another step reads;
> choose `true` throughout if the whole plan is safe to run concurrently.
>
> Expressing "run these groups in order, but let the steps inside one group run
> together" is not supported today — see the "Command groups and scheduling"
> section of the README.

#### 🐛 Bug Fixes
- ![BREAKING](https://img.shields.io/badge/BREAKING-red) composite scheduling correctness and CI supply-chain hardening (#14) - ([81e514d](https://github.com/rsvalerio/ops/commit/81e514d2c5b0d2baac1564c92259aae5887e705f)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔄 CI/CD
- (**bump**) pin forge to v0.2.0 rather than the v1 tag - ([617192d](https://github.com/rsvalerio/ops/commit/617192d7dd4b7ae9162d96f83c9a122f442c4a5a)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**release**) Merge pull request #13 from rsvalerio/ci/dispatch-releases - ([1f28b44](https://github.com/rsvalerio/ops/commit/1f28b449f18f94232ab5659febcd838696a3840d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**release**) trigger releases by dispatch instead of tag push - ([8468d67](https://github.com/rsvalerio/ops/commit/8468d674d01c9aa92cbd3d1ee01312a1f425651c)) - Rodrigo Valerio, Claude Opus 5 (1M context)

- - -

## [v0.36.0](https://github.com/rsvalerio/ops/compare/574926793f7249c0cfb18a9a83669ead9bf5be21..v0.36.0) - 2026-08-13
#### 🚀 Features
- (**about**) add `ops about loc` splitting production, test and example lines - ([14aa874](https://github.com/rsvalerio/ops/commit/14aa874576966246844fb57d3cfd9fdaa4b3c964)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**duckdb**) add Rust LOC region queries over rust_loc_summary - ([6814ad7](https://github.com/rsvalerio/ops/commit/6814ad78a7b5822f03b7c2889b3d3f0554881711)) - Rodrigo Valerio, Claude Opus 5 (1M context)
#### 🐛 Bug Fixes
- (**core**) bound the pipe drain after a subprocess timeout - ([a5fc809](https://github.com/rsvalerio/ops/commit/a5fc8091c566e4283afc9077324dfa365365cac7)) - Rodrigo Valerio, Claude Opus 5 (1M context)
#### 📚 Documentation
- (**readme**) document `ops about loc` and how it differs from `about code` - ([ff88dcf](https://github.com/rsvalerio/ops/commit/ff88dcf992830538ec53a7d6a533ca640a13ba81)) - Rodrigo Valerio, Claude Opus 5 (1M context)
#### 🧪 Testing
- (**run-before-commit**) retry fake-git exec on ETXTBSY - ([8cee2ac](https://github.com/rsvalerio/ops/commit/8cee2ac99467225c9e76b51f37a03d9fffaa48a0)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**tracing**) pin a global dispatcher so parallel log captures survive - ([55202b4](https://github.com/rsvalerio/ops/commit/55202b42c66f719a58cc179f028c093a31d312c3)) - Rodrigo Valerio, Claude Opus 5 (1M context)
#### 🔄 CI/CD
- (**bump**) call forge's shared bump workflow - ([0626a6e](https://github.com/rsvalerio/ops/commit/0626a6e8da164155401c06c64bc460675acad2b1)) - Rodrigo Valerio, Claude Opus 5 (1M context)
- (**bump**) publish the bump commit and tag as Verified (#9) - ([5749267](https://github.com/rsvalerio/ops/commit/574926793f7249c0cfb18a9a83669ead9bf5be21)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 5 (1M context), Claude Opus 5 (1M context), Claude Opus 5 (1M context), Claude Opus 5 (1M context)
#### ⚙️ Miscellaneous
- Merge pull request #11 from rsvalerio/fix/test-stability - ([08c443c](https://github.com/rsvalerio/ops/commit/08c443cc29bf89599de468ce4e0e2d3215ed0c2a)) - [@rsvalerio](https://github.com/rsvalerio)
- Merge pull request #10 - ([afd3fd7](https://github.com/rsvalerio/ops/commit/afd3fd7cbcfd0d54255da26dbd4e9f73d70abefe)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.35.0](https://github.com/rsvalerio/ops/compare/38404f9b5f82b3312b9e30e0cbaa7e10375d1545..v0.35.0) - 2026-08-09
#### 🚀 Features
- (**rust-loc**) split Rust LOC into production, test and example (#8) - ([8b9a55f](https://github.com/rsvalerio/ops/commit/8b9a55f07679135f1b819eff151fa98ac369c0f9)) - [@rsvalerio](https://github.com/rsvalerio), Claude
#### 🔄 CI/CD
- use client-id for GitHub App token minting (#7) - ([110498e](https://github.com/rsvalerio/ops/commit/110498ebdc5d7a406aac808052e519efefc1cdae)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) track bump.yml missing --skip-ci flag - ([9932333](https://github.com/rsvalerio/ops/commit/993233380b3fc707c86f8a4b06ae97aa2fd8112b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**changelog**) dedupe the duplicated v0.34.1 section - ([38404f9](https://github.com/rsvalerio/ops/commit/38404f9b5f82b3312b9e30e0cbaa7e10375d1545)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.34.1](https://github.com/rsvalerio/ops/compare/84742919f413cde73af7ae1762d99bac950bba05..v0.34.1) - 2026-08-08
#### 🐛 Bug Fixes
- (**rust**) check, lint and build every target by default - ([46ad4e0](https://github.com/rsvalerio/ops/commit/46ad4e03eb474d28d0ef4211e67ef34379068d94)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- spec Verified bump commits via Git Data API - ([8474291](https://github.com/rsvalerio/ops/commit/84742919f413cde73af7ae1762d99bac950bba05)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.8 (1M context)
#### 🔧 Build System
- (**deps**) add ignore crate for gitignore support - ([ff55835](https://github.com/rsvalerio/ops/commit/ff558358ad287ea249eeae26c94d2d6865070b0b)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**discovery**) use ignore crate to honor gitignore patterns - ([23aa465](https://github.com/rsvalerio/ops/commit/23aa465103c659402f66010e4d9da9a6ce74bac7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**gradle-lexer**) simplify extract_quoted with early return - ([aeb6561](https://github.com/rsvalerio/ops/commit/aeb656128b9270f3d3a16144f040ff557847e1b3)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) track findings surfaced by the Rust defaults fix - ([f06740d](https://github.com/rsvalerio/ops/commit/f06740ddb8079a2c76ab785d9b522d69c10846a7)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.34.0](https://github.com/rsvalerio/ops/compare/e3ce909df497f0fbf81d80ab5078933beb6ce649..v0.34.0) - 2026-06-28
#### 🚀 Features
- (**cli**) add security command - ([e3ce909](https://github.com/rsvalerio/ops/commit/e3ce909df497f0fbf81d80ab5078933beb6ce649)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add report module and rendering - ([a070eb6](https://github.com/rsvalerio/ops/commit/a070eb6b95bf10cbd5b0f00c119c256f338429f6)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- update command mappings for new security command - ([e003342](https://github.com/rsvalerio/ops/commit/e0033422b0cc047ed2d05d2c67721802595980ed)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- update lock file - ([ea32b9d](https://github.com/rsvalerio/ops/commit/ea32b9d306ab220c705ff155ea2e7e126e95aa27)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**deps**) reorganize and improve formatting logic - ([16060ef](https://github.com/rsvalerio/ops/commit/16060ef2a0ec887f5f4d60cc0f469f36adf03025)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) extract theme types and refactor configurable system - ([a699993](https://github.com/rsvalerio/ops/commit/a6999931a9c05eabf6ab16ae4576480ec737b686)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**config**) add security command to default stack configurations - ([c877623](https://github.com/rsvalerio/ops/commit/c877623da72d2a43777b3dd18201a71080e8b671)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.33.0](https://github.com/rsvalerio/ops/compare/5e43b356517b41c93367a613dce378498a7eff8c..v0.33.0) - 2026-06-14
#### 🚀 Features
- (**stack**) add Vite detection and configuration - ([5e43b35](https://github.com/rsvalerio/ops/commit/5e43b356517b41c93367a613dce378498a7eff8c)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**test-coverage**) write llvm-cov JSON to file to prevent stdout truncation - ([4779961](https://github.com/rsvalerio/ops/commit/477996167861e75c2dd3e04ab01c2755ab6e9d16)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**stacks**) add Vite to command mappings and stack reference - ([2c7e708](https://github.com/rsvalerio/ops/commit/2c7e708ae97a25b85dcedb60c2f224799ac4add0)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔄 CI/CD
- (**workflows**) remove redundant comments and obsolete install job - ([c6dbc07](https://github.com/rsvalerio/ops/commit/c6dbc07ab8c2e8067bda15d32415591a1580c840)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.32.0](https://github.com/rsvalerio/ops/compare/e8aa16cdbea7586f82acdefbcd19fac4a72ace95..v0.32.0) - 2026-06-07
#### 🚀 Features
- (**cli**) add import-makefile command to import Makefile targets as ops commands - ([4e267ab](https://github.com/rsvalerio/ops/commit/4e267ab69a03177cba6b626e9b6bd3a12ca4f33f)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**ci**) refine HOMEBREW_TAP_TOKEN check to match actual secret usage in release.yml - ([e5c16e0](https://github.com/rsvalerio/ops/commit/e5c16e0e195f952171c20ed33c64f4986b8955b4)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**runner,terraform**) fix compute_channel_capacity doc, improve error context in terraform plan - ([47f272b](https://github.com/rsvalerio/ops/commit/47f272b594189edf0a06605c679545feb76626fd)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**cargo-toml**) wave-136 rename overstated test name, split 110-line symlink test, replace Display substring assertions with variant match (TEST-2 TEST-11 FN-1 TASK-1502 TASK-1503 TASK-1504) - ([f2d7fcf](https://github.com/rsvalerio/ops/commit/f2d7fcfd1f08bbbc728d2e487db9bce4334d7588)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### 🔧 Build System
- inherit workspace lints across crates and extensions - ([7e1d27b](https://github.com/rsvalerio/ops/commit/7e1d27b109a16a9b4210945cc7fa34cc03107274)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔄 CI/CD
- upgrade GitHub Actions checkout from v4 to v6 - ([55aebd0](https://github.com/rsvalerio/ops/commit/55aebd0d1ed252bffb8ee4982fd52a5206ac26d4)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cargo-toml**) wave-135 extract workspace_root module, dedup walk scaffolding, replace full TOML parse with line scan, document silently dropped dep overrides (ARCH-1 DUP-1 PERF-3 READ-4 TASK-1497 TASK-1498 TASK-1505 TASK-1512) - ([3260d48](https://github.com/rsvalerio/ops/commit/3260d4806826f056e6dca79f997b250dc35436cf)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cargo-toml**) wave-134 wrap parse return type in crate-local ParseError, add section field to MissingWorkspaceDependency, attach context to resolve_root propagation (ERR-2 ERR-4 TASK-1508 TASK-1509 TASK-1510) - ([26f78d1](https://github.com/rsvalerio/ops/commit/26f78d168810906be76a1a43eaeb24687f0d179b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cargo-toml**) wave-133 add #[must_use] to constructors, derive Debug on Extension/Provider, remove stale #[allow(unused_imports)] and blanket #[allow(dead_code)], clarify PublishSpec::is_publishable None semantics, drop redundant linkme dev-dep, simplify resolve_root Ok(x?) to map_err (API-5 READ-5 API-3 API-1 VER-1 TRAIT-1 PATTERN-1 TASK-1499 TASK-1500 TASK-1501 TASK-1506 TASK-1507 TASK-1511 TASK-1513) - ([e8aa16c](https://github.com/rsvalerio/ops/commit/e8aa16cdbea7586f82acdefbcd19fac4a72ace95)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**config**) extract insert_command helper and expose new config API - ([91c4867](https://github.com/rsvalerio/ops/commit/91c4867a1b4228e57d0f0fabd8fd0fa832bbad50)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) wave-140 add #[must_use] to pub parsers/formatters, replace type aliases with newtypes, preserve error source chain via .context(), replace classify_code linear scan with match, normalise stderr trim before truncate (API-5 PATTERN-1 ERR-4 PATTERN-3 SEC-21 TASK-1514 TASK-1519 TASK-1523 TASK-1528 TASK-1531 TASK-1533) - ([63578ce](https://github.com/rsvalerio/ops/commit/63578ced18b6b01eb9de97c4491250b696e4f841)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) wave-139 add context to build_user_context current_dir, extract tracing-capture test helper, use Arc::unwrap_or_clone in run_deps, type DenyLine.fields directly as DiagnosticFields, take ownership in resolve_package (ERR-4 DUP-3 PERF-3 TASK-1493 TASK-1494 TASK-1496 TASK-1524 TASK-1529) - ([7d8cf3d](https://github.com/rsvalerio/ops/commit/7d8cf3d5d71ad29022c90d5d73f94b45811ac717)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) wave-138 return Cow from SeverityClass::style, inline severity_icon/colorize_severity wrappers, consolidate format_bans_summary with class-driven loop, unify format_severity_section row-emit path, table-drive has_issues DenyEntry sections (PERF-3 FN-1 DUP-3 READ-5 TASK-1495 TASK-1517 TASK-1520 TASK-1521 TASK-1525 TASK-1526 TASK-1530) - ([bdd08ae](https://github.com/rsvalerio/ops/commit/bdd08ae7b139664b87a3c1e6037146e23b5569b3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) wave-137 fix body_lines pre-separator inflation, extract classify/drift/slice helpers, replace bool with UpgradeKind enum, deduplicate row-write, rewrite separator_columns with iterator (PATTERN-1 FN-4 FN-1 DUP-1 TASK-1492 TASK-1515 TASK-1516 TASK-1518 TASK-1522 TASK-1527 TASK-1532) - ([4684ea3](https://github.com/rsvalerio/ops/commit/4684ea38596c37237526b7f9fe31e2823f992398)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### ⚙️ Miscellaneous
- (**backlog**) add task definitions from code review - ([88915dc](https://github.com/rsvalerio/ops/commit/88915dcb0f375ac2506d981a4c11ee25e541832b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-140 — deps error handling, security, API & type quality - ([c94c619](https://github.com/rsvalerio/ops/commit/c94c619b1f20337952c4752e8ce776114691b08c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-139 — deps performance, allocation & test scaffolding - ([dca55cd](https://github.com/rsvalerio/ops/commit/dca55cd226a84db558fa5370f526c929c99d34c5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-138 — deps formatting, severity & output helpers - ([5979a44](https://github.com/rsvalerio/ops/commit/5979a44efb4b004e604e4eaa866c96361110cb59)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-137 — deps upgrade-table parsing & format - ([28bd0ed](https://github.com/rsvalerio/ops/commit/28bd0ed3fdbdc94d180bc2bd1c911f7158979dfb)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**backlog**) close wave-136 — cargo-toml test quality - ([5ae5f69](https://github.com/rsvalerio/ops/commit/5ae5f690a2307d6d4ed21ce77a5e9897cf1e0c01)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**backlog**) close wave-135 — cargo-toml architecture, dedup & perf - ([11161b3](https://github.com/rsvalerio/ops/commit/11161b3f16abebe4709283b79d254fcdd59ff500)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**backlog**) close wave-134 — cargo-toml error handling hygiene - ([d25e0c5](https://github.com/rsvalerio/ops/commit/d25e0c5ec0e578ffec7980d12c7861eb1c624106)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**backlog**) close wave-133 — cargo-toml API, lint & readability hygiene - ([2bcebac](https://github.com/rsvalerio/ops/commit/2bcebac47d7281558bbee2b153cba1e39decc62b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)

- - -

## [v0.31.0](https://github.com/rsvalerio/ops/compare/304ce5edfea0fe275f3c82dc396af7e5bc4c81aa..v0.31.0) - 2026-05-22
#### 🚀 Features
- (**cli**) add check-json and check-yaml config checker subcommands - ([304ce5e](https://github.com/rsvalerio/ops/commit/304ce5edfea0fe275f3c82dc396af7e5bc4c81aa)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) register config checkers as builtin commands - ([e0fce55](https://github.com/rsvalerio/ops/commit/e0fce55633c1e166ec3c76323467fca2c244b8ba)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**config-checkers**) harden error handling, add size cap, JSON5 rename - ([fdd5b57](https://github.com/rsvalerio/ops/commit/fdd5b578b18f3702dab2feea9d16f424ba86f3ce)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### 📚 Documentation
- remove obsolete terraform plan summary - ([018cd69](https://github.com/rsvalerio/ops/commit/018cd6960a3632192feba516e0b518f91ece41a8)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**deps**) update arrow to 58.3.0 and hashbrown to 0.17.1 - ([11abfa1](https://github.com/rsvalerio/ops/commit/11abfa159c9255d7c6adeef37dd3b140360a1677)) - [@rsvalerio](https://github.com/rsvalerio)
- add ops-config-checkers extension crate - ([0d519d0](https://github.com/rsvalerio/ops/commit/0d519d0372ab2ee3384703f9df223884572d1d00)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) simplify extension loader test for generic extensions - ([7ca845f](https://github.com/rsvalerio/ops/commit/7ca845fa266691566404e8cefed9f79d71dcf4a5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust**) wave-130 promote install_*_with_timeout to pub, log kill/wait failures on timeout path, rename install-side timeout module, add #[must_use] to pure tools queries, gate install-timeout tests to cfg(unix), drop dead ProbeOutcome::map, add validate_rustup_toolchain accepting '.' for version-pinned identifiers, mark ProbeOutcome #[non_exhaustive], split get_active_toolchain into Resolved/None/ProbeFailed via ActiveToolchain (API-3 ERR-9 READ-1 API-5 TEST-19 READ-7 ERR-2 API-1 ERR-1 TASK-1581 TASK-1584 TASK-1585 TASK-1586 TASK-1591 TASK-1592 TASK-1608 TASK-1615 TASK-1619) - ([d8994aa](https://github.com/rsvalerio/ops/commit/d8994aa982c38c3225702b5e7cae8c93a7498b56)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust**) wave-129 hoist PATHEXT out of find_on_path_in loop, pair cargo built-in / rustup target arch lists with drift-detection tests, strip rule IDs from probe timeout warn, surface per-entry read_dir errors in capture_path_index_from, precompute cargo-list / rustup-components hash sets in collect_tools, extend PathIndex case-folding to macOS (PERF-3 PATTERN-1 READ-2 ERR-7 CONC-7 TASK-1580 TASK-1582 TASK-1583 TASK-1605 TASK-1614 TASK-1616 TASK-1617) - ([04cf7f4](https://github.com/rsvalerio/ops/commit/04cf7f4836d77a89857c252c229489800a4409df)) - [@rsvalerio](https://github.com/rsvalerio)
- (**ops-duckdb**) wave-132 wrap extra_opts in ExtraOpts newtype with validating constructor, rename query::helpers::TableName to QueryTableName, make SourceName/WorkspaceRoot fields private with ::new/::as_*, add #[must_use] to get_source_checksum / escape_sql_string / sanitize_path_for_sql, collapse create_table_from_json_sql to a single format!, add phase+table_name with_context to provide_via_ingestor, generalise query_project_row helper and route query_project_coverage / query_project_scalar through it, pin select_expr to &'static str, extract rename_json_to_done / unlink_and_remove_sidecar from cleanup_artifacts, switch downcast_duckdb to Option<&Arc<...>> (SEC-12 READ-1 API-2 API-5 READ-8 ERR-4 DUP-1 FN-1 OWN-6 TASK-1623 TASK-1624 TASK-1625 TASK-1626 TASK-1627 TASK-1628 TASK-1629 TASK-1630 TASK-1631 TASK-1632 TASK-1633) - ([fc1df28](https://github.com/rsvalerio/ops/commit/fc1df287fee302721102867ec77b48439e3423b8)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**ops-git**) wave-131 move SEC-33 cap check above lossy UTF-8 decode so in-cap configs with non-UTF-8 bytes are not false-rejected, annotate parse_remote_url / read_origin_url / read_origin_url_from / read_head_branch with #[must_use], extract shared decode_quoted_body helper for git-config quoted-string escapes (ERR-1 API-5 DUP-1 TASK-1620 TASK-1621 TASK-1622 TASK-1639) - ([6d02c23](https://github.com/rsvalerio/ops/commit/6d02c23818702666042d15fb0c98a74439352d00)) - [@rsvalerio](https://github.com/rsvalerio)
- (**test-coverage**) address code review findings — demote internal exports, harden error handling, add schema drift tracking (API-9 READ-1 READ-5 ERR-1 PATTERN-1 PATTERN-3 TEST-19 ARCH-11 TASK-1601 TASK-1602 TASK-1606 TASK-1609 TASK-1610 TASK-1611 TASK-1612 TASK-1613 TASK-1618) - ([2a12828](https://github.com/rsvalerio/ops/commit/2a12828758a454b7463eefefddcd385e3411fb46)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) add wave-133 task definitions from test-coverage code review - ([b1edfa4](https://github.com/rsvalerio/ops/commit/b1edfa43879c47bc75a8659cdec7f48a56e27b0c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 132 — ops-duckdb SQL & schema hardening - ([66e15f6](https://github.com/rsvalerio/ops/commit/66e15f6680768595422ce3bb3200e35d81a050fb)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**backlog**) close wave 128 — config-checkers hardening - ([3eb93e6](https://github.com/rsvalerio/ops/commit/3eb93e657e079183ecc033581f48274846aa8056)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)

- - -

## [v0.30.0](https://github.com/rsvalerio/ops/compare/21c3c7853f15ab302c2577844237e79c7def7ce9..v0.30.0) - 2026-05-21
#### 🚀 Features
- (**cli**) display command aliases in help output - ([e432c9f](https://github.com/rsvalerio/ops/commit/e432c9f4c9abe7d433081ffccf46c72ff11072a2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) add text-fixers subcommand and integration - ([5e8ee4d](https://github.com/rsvalerio/ops/commit/5e8ee4ddc00813d441e74ad5c4d238a340b377e5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) add text-fixers extension with discovery and fixers - ([a618b93](https://github.com/rsvalerio/ops/commit/a618b933c0922dc9f0a920b8af89b0805cf55c5d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) support builtin commands (end-of-file-fixer, trailing-whitespace) - ([6aa6842](https://github.com/rsvalerio/ops/commit/6aa6842441c4fb1651c03c7bf15761cc88ef4250)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**cli**) wave-116 OnceLock cache builtin_subcommand_names, HashSet dedupe in gather_available_commands, share Cli::command across run/dispatch, build clap tree once in hide_irrelevant_commands test (PERF-1 TEST-25 TASK-1318 TASK-1330 TASK-1368 TASK-1374) - ([214821e](https://github.com/rsvalerio/ops/commit/214821e8ef90118dfd87cffffa3ebe783016b129)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**cli**) wave-117 pin error provenance in run_cmd unknown/cycle tests, gate builtin_extensions loads_all by stack-feature cfg, source stack-specific subcommand visibility from stack_specific_commands(), replace register_extension_commands_empty_inputs tautology with preservation property (TEST-11 TASK-1336 TASK-1337 TASK-1338 TASK-1362 TASK-1365) - ([41e4ca6](https://github.com/rsvalerio/ops/commit/41e4ca6473b9c28347107ea049cbaf1ff0206e59)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) assert parse-error and resolved-command output in integration tests - ([afea855](https://github.com/rsvalerio/ops/commit/afea855b80e7ed21cde7cbf24c4c9f1d82ae49f7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) consolidate duplicate BufWriter+MakeWriter scaffolds via capture_tracing - ([0361a2c](https://github.com/rsvalerio/ops/commit/0361a2c15f04547a433c8901d4a2ef72bc2058d5)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- update dependencies for text-fixers extension - ([b14bbe4](https://github.com/rsvalerio/ops/commit/b14bbe4fda9f661a7f7a4410d1d0ce39ed3ed062)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) TASK-1341 unify run_commands tap on Option<&Path> - ([93b42a2](https://github.com/rsvalerio/ops/commit/93b42a2a9159234911f0b815460d29a69a655303)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) replace Option<Option<&str>> heading sentinel with named enum - ([d8a7408](https://github.com/rsvalerio/ops/commit/d8a7408704e7c281985fcc44e93565aa7e22bc2f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) strip rule-id and task-id prefixes from source comments - ([706734e](https://github.com/rsvalerio/ops/commit/706734efa1126d35b3e3e23577f2bea285773f59)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) wave-97 config/env/logging/init hardening (API-1 ERR-9 FN-1 READ-5 READ-7 TEST-25 TASK-1312 TASK-1320 TASK-1326 TASK-1333 TASK-1369 TASK-1379) - ([58fb9a3](https://github.com/rsvalerio/ops/commit/58fb9a3545dec81108eab2fc494b622ea5639f71)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) wave-95 about/theme/tools rendering, _to seams, raw+verbose warning (API-1 ERR-1 PATTERN-1 PERF-3 READ-2 READ-5 DUP-3 TEST-11 TEST-25 TASK-1321 TASK-1332 TASK-1335 TASK-1343 TASK-1344 TASK-1345 TASK-1346 TASK-1363 TASK-1364 TASK-1367 TASK-1373 TASK-1376 TASK-1381) - ([467ba57](https://github.com/rsvalerio/ops/commit/467ba57b0a594c4262c7ddf21294c2193df21a21)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) wave-94 extension registry decomposition + show-path single-walk (API-1 ERR-1 ERR-7 FN-1 PERF-1 PERF-3 PATTERN-1 READ-5 READ-7 DUP-3 TEST-11 TEST-25 TASK-1309 TASK-1314 TASK-1315 TASK-1327 TASK-1328 TASK-1329 TASK-1340 TASK-1348 TASK-1349 TASK-1350 TASK-1351 TASK-1353 TASK-1359 TASK-1360 TASK-1366 TASK-1371 TASK-1378 TASK-1380) - ([1332ec1](https://github.com/rsvalerio/ops/commit/1332ec13a047a74a3d3267edac1dffe908a59334)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) wave-93 hook install + dispatch (API-1 PATTERN-1 READ-5 READ-7 FN-1 FN-3 ARCH ERR-9 CL-3 TASK-1307 TASK-1308 TASK-1317 TASK-1322 TASK-1323 TASK-1324 TASK-1325 TASK-1331 TASK-1347 TASK-1352 TASK-1354 TASK-1361 TASK-1375) - ([21c3c78](https://github.com/rsvalerio/ops/commit/21c3c7853f15ab302c2577844237e79c7def7ce9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli,core**) wave-118 validate_command_name returns anyhow::Result, gate Tools subcommand on stack-rust, drop dead Subcommand alias, writer seam for new-command confirmation, pin ui::error end-to-end ESC sanitisation, is_toplevel_help consumes --tap value (ERR-10 API-1 READ-5 ERR-7 PATTERN-1 TASK-1316 TASK-1319 TASK-1342 TASK-1355 TASK-1370 TASK-1377) - ([1f818c7](https://github.com/rsvalerio/ops/commit/1f818c77d48350c4762190b558364f84a6b262f4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-114 sync_parent_dir warns on open failure, ATOMIC_WRITE_FALLBACK_MODE/MASK constants, build_tmp_basename uniqueness doc (ERR-1 READ-1 READ-5 TASK-1464 TASK-1467 TASK-1476) - ([fea0175](https://github.com/rsvalerio/ops/commit/fea0175a2174425f957fbaca620153540289ba04)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-113 from_env returns Result for non-UTF-8 OPS_ROOT, canonicalize on miss only, GLOBAL_CONFIG_PATH reset-token hook (ERR-1 PERF-3 READ-1 TASK-1462 TASK-1465 TASK-1475) - ([673e81c](https://github.com/rsvalerio/ops/commit/673e81ce77b71da9900c1eed6e2b3477f0b0f7c1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-112 shared sync::lock_recover Mutex poison-recover, migrate expand & stack/detect, test-seam breadcrumb (DUP-3 ERR-1 ERR-5 TASK-1470 TASK-1474 TASK-1477) - ([eb1e5ea](https://github.com/rsvalerio/ops/commit/eb1e5ea55ab5619d214580afbdf4c8dae9e87e1a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-111 split config/loader into env/global/conf_d submodules (ARCH-1 TASK-1471) - ([ea3244c](https://github.com/rsvalerio/ops/commit/ea3244c9e9994242207d1bea7ff5b72b6089691c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-111 split subprocess into cap/drain/mod, route output_byte_cap through cached_byte_cap_env, io::copy post-cap discard & timeout-drain breadcrumb (ARCH-1 ARCH-11 ERR-1 PERF-3 TASK-1463 TASK-1466 TASK-1471 TASK-1473) - ([524af94](https://github.com/rsvalerio/ops/commit/524af94ebf1e208cdae4de1e1e66f0608b035bbb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-110 read_capped IO refuses symlinks via O_NOFOLLOW & Debug-escapes paths (SEC-25 SEC-21 TASK-1461 TASK-1468 TASK-1472) - ([8c83481](https://github.com/rsvalerio/ops/commit/8c83481456106768dc8d6fb66bda1cbe4da028a6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-109 shared capture_tracing helper, test-support Debug derives & surface-index docs (DUP-3 TRAIT-1 READ-3 TASK-1429 TASK-1433 TASK-1438) - ([ad47458](https://github.com/rsvalerio/ops/commit/ad47458a8d722d3178db978958687a7cea09120c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-108 OpsTable Debug derive, shared stdout TTY cache, terminal width memoisation, io::Error::other (TRAIT-1 PERF-3 TASK-1436 TASK-1439 TASK-1440) - ([b0be712](https://github.com/rsvalerio/ops/commit/b0be71258b0be62a4fb174e3f3c591f991fb0172)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-107 style_gated Cow, error-tail decomp/stack-ring/inline CR, format_number Cow fast path, emit_to single-write, pad bulk extend & macro doc link (PERF-1 PERF-5 READ-6 FN-1 PERF-3 TASK-1396 TASK-1397 TASK-1398 TASK-1405 TASK-1422 TASK-1428 TASK-1432 TASK-1441) - ([6209e54](https://github.com/rsvalerio/ops/commit/6209e5488400988a5c06242565ad06a963829097)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-106 subprocess drain helper, empty-EIO surfacing, shell-quote join & extension-show passthrough (DUP-4 PERF-3 ARCH-2 DUP-1 TASK-1399 TASK-1412 TASK-1426 TASK-1449) - ([cf8d2e3](https://github.com/rsvalerio/ops/commit/cf8d2e3cc9febce9d5d39a252499798d33d0d0ea)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-105 CommandSpec disambig deserialize, CommandMeta, validate cwd/control-bytes, CommandId FromStr & RunError From symmetry (API-1 ERR-1 TRAIT-1 TASK-1402 TASK-1415 TASK-1430 TASK-1431 TASK-1445 TASK-1447) - ([e5bc0fc](https://github.com/rsvalerio/ops/commit/e5bc0fc65326929526f63b4e18c1cf0a64a90289)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-104 ops_root cache LRU/canonicalise & global_config_path memo (CONC-1 PERF-3 ERR-1 READ-1 TASK-1418 TASK-1419 TASK-1423 TASK-1424 TASK-1434) - ([d9c039f](https://github.com/rsvalerio/ops/commit/d9c039fd9b617ebeee893d31420f44ac5c8ad019)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-103 env-var scan, expand-warn cap & byte-cap clamp (ERR-1 PERF-3 CONC-1 TASK-1389 TASK-1411 TASK-1414 TASK-1443 TASK-1444) - ([47456cc](https://github.com/rsvalerio/ops/commit/47456ccc1c404ee17ccf6d1f1f00a7a7e722d246)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-102 config loader/merge cwd-decoupling, loud failures & caches (PERF-3 ERR-7 ERR-4 ERR-1 READ-5 TASK-1395 TASK-1400 TASK-1401 TASK-1403 TASK-1416 TASK-1421 TASK-1446 TASK-1448) - ([01a0838](https://github.com/rsvalerio/ops/commit/01a08380fd48428dc93f86a960b2ba7bf7de4664)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-101 about-card sanitise/filter/named-spec & traits (SEC-21 PERF-3 DUP-3 READ-1 READ-2 FN-4 API-1 TRAIT-1 TASK-1390 TASK-1391 TASK-1392 TASK-1406 TASK-1407 TASK-1408 TASK-1417 TASK-1420 TASK-1427 TASK-1435 TASK-1437) - ([4c4cf8f](https://github.com/rsvalerio/ops/commit/4c4cf8ff2f31487155f7859665e040fef26c4f76)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-100 Stack enum-derived metadata, memoize default_commands & detect canonicalize (READ-6 PERF-3 ERR-1 TASK-1404 TASK-1409 TASK-1410 TASK-1413) - ([d0b0e8e](https://github.com/rsvalerio/ops/commit/d0b0e8e85d9d3609c2ab335c068d7d7260174312)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) wave-99 atomic_write & read_capped symlink/path-context/perf (SEC-25 ERR-4 FN-2 PERF-3 TASK-1388 TASK-1393 TASK-1394 TASK-1425 TASK-1442) - ([1e40640](https://github.com/rsvalerio/ops/commit/1e40640b9f3fb5515e3a896154aff9d7a30a7299)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core,cli**) wave-115 default_commands_ref borrowing accessor, preprocess_args drops args[0] clone, try_push borrows name and clones only on insert (OWN-8 TASK-1357 TASK-1358 TASK-1469) - ([d5e6919](https://github.com/rsvalerio/ops/commit/d5e69195bd125970af827ada8719c4666a1519a2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) wave-125 DataRegistry::register uses entry() parity with CommandRegistry, drive provider_name escape test through real register path, split spliced SEC-21/DUP-3 doc comments (TEST-11 READ-1 PATTERN-3 TASK-1487 TASK-1488 TASK-1489) - ([c8324f7](https://github.com/rsvalerio/ops/commit/c8324f707318320319470d7e11d0fece90817c5b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust**) wave-124 move toml to dev-deps in about, rewrite classify_member with let-else, escape PATH paths via Debug in probe/path, extract run_probe_capturing helper, extract spawn_install_with_timeout helper, replace tautological ToolStatus derive tests with Display contract pin, cache per-workspace canonical Cargo.toml map on LoadedManifest, drop owned-String dep_counts key in favour of &str borrow, rework ABA cache test to drive cached_query_project_coverage, swap typed manifest cache hit-path PathBuf clone for Arc<PathBuf> share (VER-1 PATTERN-1 ERR-7 DUP-3 TEST-1 TEST-25 PERF-3 TASK-1490 TASK-1491 TASK-1563 TASK-1564 TASK-1565 TASK-1568 TASK-1569 TASK-1570 TASK-1571 TASK-1572) - ([99255ee](https://github.com/rsvalerio/ops/commit/99255ee627c4d5cb7ecc7013af9759ef306f5d47)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**extensions-rust**) wave-121 fold parse_update_output counts into single pass, preserve RunError source chain via anyhow context, scrub control bytes in cargo stderr_tail, lift tracing-capture scaffold into helper, reject 'no active toolchain' as a toolchain name (PERF-3 ERR-4 DUP-3 SEC-21 PATTERN-1 TASK-1534 TASK-1535 TASK-1536 TASK-1537 TASK-1566) - ([73aae98](https://github.com/rsvalerio/ops/commit/73aae98ddea85d9c6d21dfdc5f9f761ee0e7f0f1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**metadata**) wave-122 dedupe package_index helpers, fold cap-probe into single SQL round trip, return typed DataProviderError, derive Debug on public types, lift MetadataCaches substruct, extend JsonValueExt with array_iter/array_str_iter, split MetadataIngestor::load into named helpers, share TracingBuf in duplicate-warning tests, strengthen nonexistent-dir test, fix module preamble and ignore-doc drift, split 1288-line tests.rs into focused submodules under <400 lines each (TRAIT-1 ERR-2 FN-1 PATTERN-1 PERF-3 READ-1 READ-5 API-1 DUP-1 DUP-3 ARCH-1 TEST-1 TEST-26 TASK-1538 TASK-1539 TASK-1541 TASK-1542 TASK-1543 TASK-1544 TASK-1545 TASK-1546 TASK-1547 TASK-1548 TASK-1549 TASK-1550 TASK-1551 TASK-1552) - ([1460d83](https://github.com/rsvalerio/ops/commit/1460d83dbdc412bc81f85c6b8f0603332d2b787d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**test-coverage**) wave-123 split lib.rs into subprocess/parse/provider submodules, introduce CoverageRow as single schema source of truth, fold flatten_coverage_json into build_record+dedup_push helpers, switch dedup to HashMap::entry, factor cargo exit formatter, guard soft-fail on non-empty data array, replace include_str! grep with LLVM_COV_ARGS const slice, add must_use to public Result fns, drop duplicate linkme dev-dep, share setup_loaded_db across DuckDB integration tests (ARCH-1 ERR-1 FN-1 PATTERN-1 PATTERN-3 DUP-3 API-5 TEST-23 VER-1 TASK-1553 TASK-1554 TASK-1555 TASK-1556 TASK-1557 TASK-1558 TASK-1559 TASK-1560 TASK-1561 TASK-1562) - ([95e0b86](https://github.com/rsvalerio/ops/commit/95e0b86b402552b08f8e15507c2e56c9c5115cb4)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### ⚙️ Miscellaneous
- (**backlog**) close task-0059 - ([72cb54a](https://github.com/rsvalerio/ops/commit/72cb54ab98363793cd77e7d432eec52d71fadc2b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) start wave-120 code review - ([82a0e97](https://github.com/rsvalerio/ops/commit/82a0e97c5ffbe9ca4615b89780c93725f648eefa)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) start wave-119 code review - ([f0da859](https://github.com/rsvalerio/ops/commit/f0da8599b1207be8018561eca81410fc68e3c9fe)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-124 tasks Done; defer ARCH-1 tests.rs split - ([9227baa](https://github.com/rsvalerio/ops/commit/9227baa36db7091d4e4fedf594c26bc10dabe67a)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**backlog**) close wave-123 tasks Done - ([6848ccd](https://github.com/rsvalerio/ops/commit/6848ccd20ed555afa00aaec63fbeaf2f93b8b6aa)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**backlog**) close wave-122 tasks Done; defer DUP-4 fixture dedup - ([9ea5beb](https://github.com/rsvalerio/ops/commit/9ea5beb76c7cbdfc3c9ddc1900c7e68c0d55dfe8)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**backlog**) close wave-121 tasks Done - ([9e164b9](https://github.com/rsvalerio/ops/commit/9e164b9aa6b5c9180c1165d7da2e103040723e66)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-125 tasks Done - ([1d52bfd](https://github.com/rsvalerio/ops/commit/1d52bfdefe821a9baf644a4affc543d436e29150)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-118 tasks Done - ([a2abd44](https://github.com/rsvalerio/ops/commit/a2abd443f66130e046424a87997177548aac109d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-117 tasks Done - ([8fbfce6](https://github.com/rsvalerio/ops/commit/8fbfce6df609a806507ebd878aed9b5dae4a17bd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-116 tasks Done - ([0612857](https://github.com/rsvalerio/ops/commit/061285778ed5c26d47ec8f8ce79cbea8026647e5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-115 tasks Done; register wave-116/117/118 - ([d43d36b](https://github.com/rsvalerio/ops/commit/d43d36b19d6e3c43cbc667e4aef6af139f6dcf28)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-114 tasks Done - ([5d955a6](https://github.com/rsvalerio/ops/commit/5d955a613888b17b3bf13b9c8f60fd1030167426)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-113 tasks Done - ([16fb75a](https://github.com/rsvalerio/ops/commit/16fb75abae386caa5d6105e30b0b105a8a34cb67)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-112 tasks Done - ([6360e56](https://github.com/rsvalerio/ops/commit/6360e56e57e1f22e796773560290902c5147c764)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-111 tasks Done - ([60f8a83](https://github.com/rsvalerio/ops/commit/60f8a830004b6b1b961a099b0019d189463a6bd8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-110 + wave-96 tasks Done; register new triage tasks - ([a904593](https://github.com/rsvalerio/ops/commit/a904593700da0a74f73cf86f43682c56b1818e20)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-109 tasks Done - ([e4f3ebd](https://github.com/rsvalerio/ops/commit/e4f3ebdea4b8fa35dc5e79bdf8823f553ca4b7fb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-108 tasks Done - ([5856de4](https://github.com/rsvalerio/ops/commit/5856de45e3183e7c6d28ac4fafd8b3cfa3ff3fbe)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-107 tasks Done - ([8e547b9](https://github.com/rsvalerio/ops/commit/8e547b9c5e126807348a0dc862f6ae8561fbe728)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-106 tasks Done - ([f19b978](https://github.com/rsvalerio/ops/commit/f19b978b666b6d642177ff227086940738dfd9cc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-105 tasks Done - ([2ffb86b](https://github.com/rsvalerio/ops/commit/2ffb86bc20c22041868a61f57cc174b3ee1cb82a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-104 tasks Done - ([32ff41a](https://github.com/rsvalerio/ops/commit/32ff41a79d9ec8805ed699f9fe2e4bf9ae204884)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-103 tasks Done - ([547c0c9](https://github.com/rsvalerio/ops/commit/547c0c9adc42259d2e5c9b677a7e587130d6fced)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-102 tasks Done - ([5f6eb00](https://github.com/rsvalerio/ops/commit/5f6eb001af1743427ad34bd9b70bf62bf17d181d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-101 tasks Done - ([f08501f](https://github.com/rsvalerio/ops/commit/f08501f7e6efd7009acb5a10371af1eca3cde718)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-100 tasks Done - ([d51f4c2](https://github.com/rsvalerio/ops/commit/d51f4c217a5504238faa65b430b25fc3b6f04f71)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-99 and add new triage tasks - ([096da15](https://github.com/rsvalerio/ops/commit/096da15a33d076713e9a2045c55e1e876e2273fa)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-98 tasks Done - ([b740f23](https://github.com/rsvalerio/ops/commit/b740f239da4fbb9243d64c270fa91111967cecdb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-97 tasks Done - ([c474bdb](https://github.com/rsvalerio/ops/commit/c474bdb204cdfaa23d2028b54d47ff2d0a7c7369)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-95 tasks Done - ([1154d2e](https://github.com/rsvalerio/ops/commit/1154d2ecd3cec5d45c6454ff80bd12d64bcceff4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-94 tasks Done - ([6379e22](https://github.com/rsvalerio/ops/commit/6379e223dbd9e9b54ae9cdcc9062ce17c3907e1c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-93 tasks Done - ([a66ca89](https://github.com/rsvalerio/ops/commit/a66ca895ff4d40a3b5dcd6fe187af9d06cb0d20d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**ops**) add end-of-file-fixer and trailing-whitespace to pre-commit checks - ([09d5e3c](https://github.com/rsvalerio/ops/commit/09d5e3c7f4ea0d2cfab1411606d244a67d0eddf7)) - [@rsvalerio](https://github.com/rsvalerio)
- save code review md tasks - ([39b3930](https://github.com/rsvalerio/ops/commit/39b3930eb63af6ad2364a9c57386e3e0104855ec)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🎨 Styling
- (**docs**) add trailing newlines - ([29b0313](https://github.com/rsvalerio/ops/commit/29b03131b955d820a33ea267696ad9aedc2e8854)) - [@rsvalerio](https://github.com/rsvalerio)
- remove trailing whitespace - ([fa50b77](https://github.com/rsvalerio/ops/commit/fa50b77d4d51da256100620cb874b2856d059bf9)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.29.2](https://github.com/rsvalerio/ops/compare/19cf99389dac30a661598513cdeefffdf1a629c0..v0.29.2) - 2026-05-11
#### 🐛 Bug Fixes
- (**workspace**) wave-89 .ops.toml path/table-safety and decor (PATTERN-1 READ-5 DUP-1 ERR-5 API-1 TASK-1276 TASK-1277 TASK-1278 TASK-1292 TASK-1296 TASK-1300) - ([19cf993](https://github.com/rsvalerio/ops/commit/19cf99389dac30a661598513cdeefffdf1a629c0)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**registry**) fix builtin_extensions_unknown determinism race with tempdir - ([87eba75](https://github.com/rsvalerio/ops/commit/87eba751e3d88a03c00f3d9a6de89d3c09878a94)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) wave-92 env opt-in semantics, exit-code sentinel, tools probe injection, theme loud-fail (API-1 READ-5 TEST-25 ERR-1 TASK-1290 TASK-1293 TASK-1295 TASK-1298) - ([8593142](https://github.com/rsvalerio/ops/commit/8593142fbfc6f18811b12daddce6de9d336e218c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) wave-91 hook dispatch dedup, single composite traversal, dry-run sanitisation, plan struct (DUP-1 PATTERN-1 SEC-21 API-1 FN-3 ARCH-3 PERF-1 TEST-11 TASK-1273 TASK-1274 TASK-1275 TASK-1281 TASK-1282 TASK-1283 TASK-1284 TASK-1285 TASK-1294 TASK-1299 TASK-1302) - ([b693416](https://github.com/rsvalerio/ops/commit/b693416cc7e8b129e757798f2f58080e227939e8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) wave-90 extension registry dedup, dedupe-state scoping, test scaffolding (DUP-1 DUP-2 ERR-1 ERR-5 FN-1 PATTERN-1 TEST-1 TEST-11 TEST-25 TASK-1279 TASK-1280 TASK-1286 TASK-1287 TASK-1288 TASK-1289 TASK-1291 TASK-1297 TASK-1301) - ([7a887ae](https://github.com/rsvalerio/ops/commit/7a887ae2d16749261566a2996847b7f0088a90ad)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) close wave-92 tasks Done - ([95607ee](https://github.com/rsvalerio/ops/commit/95607eeada420b6a418d03652014614293c595f1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-91 tasks Done - ([191cddc](https://github.com/rsvalerio/ops/commit/191cddc5846ce971196ca0feac812d47873d3c14)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-90 tasks Done - ([f716bc9](https://github.com/rsvalerio/ops/commit/f716bc913abb4b7d57934e913225788bc404b525)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-89 tasks Done - ([e1bbfaa](https://github.com/rsvalerio/ops/commit/e1bbfaae5a1c53dcc2370b1a23e3de031773cd92)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.29.1](https://github.com/rsvalerio/ops/compare/f95511b3c342c8e81d51964f293527369722d329..v0.29.1) - 2026-05-11
#### 🐛 Bug Fixes
- (**about/manifest-cache**) drop Mutex<HashMap> on hot path and verify mtime+len (CONC-7 CONC-2 TASK-1163 TASK-1198) - ([986911d](https://github.com/rsvalerio/ops/commit/986911d329789606f59b8bb7d5948c3ef31925ec)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) reject new_command names unusable as TOML keys or clap subcommands (TASK-1272) - ([4e978ea](https://github.com/rsvalerio/ops/commit/4e978ea9a528b598c601a95b4a6d3ef1b1a2d6fc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) bound provide_via_ingestor reentry and cap validate_extra_opts pairs (CONC-2 SEC-33 TASK-1143 TASK-1241) - ([b7d1114](https://github.com/rsvalerio/ops/commit/b7d11140f3ce1824804221a4ee957fbfbc31feaf)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git/provider**) drop unparseable post-redaction garbage from fallback remote_url (SEC-13 TASK-1151) - ([b3d5d77](https://github.com/rsvalerio/ops/commit/b3d5d776ea45cf552bf942e8d3a777c710b9aa42)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) use randomised temp file in upgrade_legacy_hook to avoid concurrent-install races (SEC-25 TASK-1210) - ([44a5ac3](https://github.com/rsvalerio/ops/commit/44a5ac30e955823fc7db6e466c1535465e33b14e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node/about**) scrub bare owner/repo shorthand in normalize_repo_url (SEC-14 TASK-1205) - ([c7fb986](https://github.com/rsvalerio/ops/commit/c7fb9862c6f00d92673df9682dc07134feda7a55)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) wave-77/78 command and tap hardening (CONC-3 CONC-6 CONC-7 SEC-23 ARCH-9 TASK-1126 TASK-1137 TASK-1140 TASK-1176 TASK-1177 TASK-1229) - ([cf5f73d](https://github.com/rsvalerio/ops/commit/cf5f73d9df1ac828b8b8f995d67004ab826f2811)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/about**) coverage cache key and reentrancy, units case folding (ARCH-9 CONC-2 CONC-7 CONC-1 TASK-1144 TASK-1155 TASK-1193 TASK-1249) - ([9d5590e](https://github.com/rsvalerio/ops/commit/9d5590e51075193ffa2f1101a304175831caa25f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/cargo-toml**) reject parent-traversed members in find_workspace_root_with_depth (SEC-25 TASK-1204) - ([bb52fbd](https://github.com/rsvalerio/ops/commit/bb52fbd9d74dbdc4ff49cf386a37c50fcac64942)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/metadata**) cap query_metadata_raw before materialising and reject absolute member paths (SEC-33 SEC-14 TASK-1194 TASK-1246) - ([8c98e01](https://github.com/rsvalerio/ops/commit/8c98e012aa413dfbd3bf639c42c1f211ccb35bf2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/tools**) tighten validate_cargo_tool_arg toolchain component check (SEC-13 TASK-1199) - ([64da056](https://github.com/rsvalerio/ops/commit/64da0564f1f2aee15a163098ddc1c50416d50d22)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-87 parser pattern bugs URLs/Go/Pnpm/manifests (PATTERN-1 TASK-1164 TASK-1168 TASK-1178 TASK-1208 TASK-1212 TASK-1216 TASK-1222 TASK-1236 TASK-1237 TASK-1239 TASK-1245 TASK-1255) - ([b653058](https://github.com/rsvalerio/ops/commit/b6530580142488ed402016e243f3f9cb4caa8997)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-86 API/public-surface stability and trait erasure (API-2 API-9 TRAIT-9 TASK-1135 TASK-1141 TASK-1169 TASK-1179 TASK-1196 TASK-1200 TASK-1227 TASK-1233 TASK-1256) - ([6fa1c93](https://github.com/rsvalerio/ops/commit/6fa1c93f1fd81c752a7dbe36acd31e9175e04a7f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-85 config/init/lock error handling (ERR-1 ERR-5 TASK-1131 TASK-1134 TASK-1139 TASK-1148 TASK-1150 TASK-1170 TASK-1174 TASK-1181 TASK-1182 TASK-1189 TASK-1214 TASK-1221 TASK-1224 TASK-1231 TASK-1234 TASK-1242) - ([fa63152](https://github.com/rsvalerio/ops/commit/fa63152e81415e82758a1f13843a2cc965fe37ca)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-84 parser/git-IO error reporting fidelity (ERR-1 ERR-2 READ-2 TASK-1167 TASK-1197 TASK-1202 TASK-1203 TASK-1209 TASK-1211 TASK-1213 TASK-1215 TASK-1244 TASK-1252 TASK-1253 TASK-1254) - ([01d4f4b](https://github.com/rsvalerio/ops/commit/01d4f4befdfe6ef34fdc55cfa6d8d596b1e6b26b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-88 display-width alignment and runtime-kind threshold (ASYNC-7 READ-5 PERF-3) - ([a016411](https://github.com/rsvalerio/ops/commit/a016411a66936487c8eeeef988358c12a836ec3c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-76 log injection and control-byte sanitisation sweep - ([f95511b](https://github.com/rsvalerio/ops/commit/f95511b3c342c8e81d51964f293527369722d329)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**workspace**) wave-80 allocation hot-path and LRU-eviction sweep (PERF-1 PERF-3 READ-5) - ([ac66ab6](https://github.com/rsvalerio/ops/commit/ac66ab63bade762f7e8385665dbc5f6a0d0c930a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-79 env-cache and allocation hot-path sweep (PERF-1 PERF-3 ARCH-9 READ-5) - ([e9b9b07](https://github.com/rsvalerio/ops/commit/e9b9b07978311d631285028ba1c0f89016a10190)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**tools**) pass explicit timeout to probe wrapper instead of mutating OPS_SUBPROCESS_TIMEOUT_SECS - ([cddbb7d](https://github.com/rsvalerio/ops/commit/cddbb7df0534f5341e8ebb57ae78f2e8e45284ef)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-83 determinism & isolation (TEST-15 TEST-17 TEST-18 TEST-25) - ([74dff91](https://github.com/rsvalerio/ops/commit/74dff9199c341ba852730ce152b5894073036314)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**workspace**) wave-82 de-duplication & shared utilities (DUP-1 DUP-3 TEST-12) - ([88f5971](https://github.com/rsvalerio/ops/commit/88f5971e431eb419f1378b377f6f4d9ce7c47bf7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) wave-81 large-file decomposition (ARCH-1 ARCH-5 FN-1) - ([74e7a46](https://github.com/rsvalerio/ops/commit/74e7a46fb15bbb817d2f08013049012f30009db9)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) add wave-89..92 triage tasks and wave parent tasks - ([3a94783](https://github.com/rsvalerio/ops/commit/3a9478319a3eb335672134019dcd15a74c9a9e6f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-87 tasks Done - ([1bfe7e6](https://github.com/rsvalerio/ops/commit/1bfe7e600c0a49924547c477324006bf88a9b40f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-86 tasks Done - ([6937a6d](https://github.com/rsvalerio/ops/commit/6937a6df8487f47047cd7e9d8430ab0da0acaf59)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-85 tasks Done - ([2506a04](https://github.com/rsvalerio/ops/commit/2506a04f6674a8558042f90ee100e8e3c63285d6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-84 tasks Done - ([a628036](https://github.com/rsvalerio/ops/commit/a62803653484f62290b3f7b414c297a14197316a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-83 tasks Done - ([f151130](https://github.com/rsvalerio/ops/commit/f1511300108cb328e2e25c77d3fe6e9ca6722fbc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-82 tasks Done - ([e60d706](https://github.com/rsvalerio/ops/commit/e60d706febfa079246cde2df42921f9d19581b36)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-77 and wave-78 tasks Done - ([2f3813a](https://github.com/rsvalerio/ops/commit/2f3813ab8f04ecac4c15a2a6c64f0ac682c83cc5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave-76 tasks Done and ingest new triage findings - ([b742ae0](https://github.com/rsvalerio/ops/commit/b742ae09f2ccbeb9db5dbf5cc1e955ad6685ab1e)) - [@rsvalerio](https://github.com/rsvalerio)
- cleanup tasks - ([a30f457](https://github.com/rsvalerio/ops/commit/a30f4574f1230dfcfe04edf2629848941d13c45c)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.29.0](https://github.com/rsvalerio/ops/compare/c25e50ef987f6dc1f82a45ffb6e8099a8f5afae1..v0.29.0) - 2026-05-08
#### 🚀 Features
- (**extension**) emit tracing::debug breadcrumb on DataRegistry::register duplicate drop (API-9 TASK-1067) - ([68c3df5](https://github.com/rsvalerio/ops/commit/68c3df5607d9549c81c4e138583125dddfe2f338)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### 🐛 Bug Fixes
- (**about**) ratio-based perf tests, ellipsis on wrap_text truncation, LRU eviction in ArcTextCache (TEST-15 PATTERN-1 ARCH-1 TASK-1044 TASK-1105 TASK-1106) - ([94d53f8](https://github.com/rsvalerio/ops/commit/94d53f8d4ffd6e9d11a507239441b51a6df5f438)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**about**) hold cache lock across file read to preserve Arc dedup (CONC-1 TASK-1051) - ([c25e50e](https://github.com/rsvalerio/ops/commit/c25e50ef987f6dc1f82a45ffb6e8099a8f5afae1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about/identity**) reject non-UTF-8 cwd in build_identity_value (ERR-1 TASK-1103) - ([8ec70b7](https://github.com/rsvalerio/ops/commit/8ec70b7f13273b957198dbceaaffdaa0b851c39c)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**about/text_util**) preserve 1-char separator in pad_header overflow case (PATTERN-1 TASK-1115) - ([105d64b](https://github.com/rsvalerio/ops/commit/105d64b3a716e91d553e9cd8b7317104eb22716a)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**about/workspace**) reject multi-segment globs and recover symlinked-root strip_prefix drop (PATTERN-1 ERR-1 TASK-1069 TASK-1070) - ([f5c8d88](https://github.com/rsvalerio/ops/commit/f5c8d880e044fbdb90000ea2e4ec18c7926d0d37)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**about/workspace**) reject ../ traversal in non-glob workspace member values (PATTERN-1 TASK-1071) - ([f789f89](https://github.com/rsvalerio/ops/commit/f789f89fcfccbf9d49ea85f639149f2e0ea719a2)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**about/workspace**) fail closed on multi-* exclude patterns in matches_exclude (PATTERN-1 TASK-1052) - ([2a7c3a3](https://github.com/rsvalerio/ops/commit/2a7c3a314ceb43025d82b7f71e23dbb4b583aae7)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cargo-update**) token-aware index-line filter and arrow-drift warn test (PATTERN-1 TEST-1 TASK-1054 TASK-1077) - ([222d00e](https://github.com/rsvalerio/ops/commit/222d00e8ea6c8ec6b4d4a699563452b746dae6b7)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cargo-update**) require whitespace boundary after known verbs in parse_action_line (PATTERN-1 TASK-1030) - ([03b9434](https://github.com/rsvalerio/ops/commit/03b9434649706d6276269c287d8e1fa78568242e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cargo-update**) bound strip_ansi CSI scan, preserve bytes on truncation (PATTERN-1 TASK-1028) - ([39ccc9d](https://github.com/rsvalerio/ops/commit/39ccc9df4b34e354560dcc82988466d4de3dba12)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cli/init**) capture cwd once and use absolute path for both read and write (PATTERN-1 TASK-1066) - ([fdd3dc9](https://github.com/rsvalerio/ops/commit/fdd3dc9cd6c3bd4259c989dd8ddbb0ba997398b0)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cli/init**) warn on parent open/fsync failures, mirroring atomic_write (ERR-1 TASK-1096) - ([f3a27e3](https://github.com/rsvalerio/ops/commit/f3a27e3ed98ce423c224e6b17a1147ec23970419)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cli/registry**) deterministic builtin_extensions order via BTreeMap (PATTERN-1 TASK-1087) - ([4c1b3d0](https://github.com/rsvalerio/ops/commit/4c1b3d0bf74eff9cce98fc9285db65bd1e14ff1c)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cli/registry**) warn on duplicate compiled-in extension config_name in builtin_extensions (PATTERN-1 TASK-1088) - ([24efa1a](https://github.com/rsvalerio/ops/commit/24efa1aeb6f7259d576fc976c28c72f049f0cf2f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cli/run**) bail when merge_plan receives empty names slice (PATTERN-1 TASK-1091) - ([14b2dc4](https://github.com/rsvalerio/ops/commit/14b2dc4699503142a56b5e6b98e3447068a9c890)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/config**) chmod temp file after creation so umask cannot widen ACL (SEC-25 TASK-1086) - ([9db024b](https://github.com/rsvalerio/ops/commit/9db024b13a1f67cbb3b3f1b3aaa96ccb753b986e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/config**) fsync cwd parent for bare-filename paths in atomic_write (ERR-1 TASK-1040) - ([52670bb](https://github.com/rsvalerio/ops/commit/52670bbc947807fa6212dd41ed717d710c7eebed)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/output**) strip bare CR in format_error_tail and replace flaky wall-clock test (PATTERN-1 TEST-15 TASK-1094 TASK-1029) - ([c72862e](https://github.com/rsvalerio/ops/commit/c72862e4faa541451681ae2b7cf1186f3b32cd2d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/subprocess**) cap run_with_timeout drain buffers via OPS_OUTPUT_BYTE_CAP (SEC-33 TASK-1050) - ([58c0c6a](https://github.com/rsvalerio/ops/commit/58c0c6a3bc61c02b0ece528b36000ccff127745d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**deps**) bail on cargo-edit header drift in parse_upgrade_table (PATTERN-1 TASK-1074) - ([0f96f83](https://github.com/rsvalerio/ops/commit/0f96f835210407823e0625ff140835a7c753e6e6)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**deps**) case-insensitive parse_upgrade_table header + distinct unknown-severity bucket in bans summary (ERR-1 PATTERN-1 TASK-1026 TASK-1041) - ([8bfc70f](https://github.com/rsvalerio/ops/commit/8bfc70f9eeaa2b49866428da728d3e525fa6c990)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**duckdb**) COALESCE NULL loc_pct in query_project_languages so empty result returns Ok (ERR-1 TASK-1116) - ([d5c6199](https://github.com/rsvalerio/ops/commit/d5c61997f3e398314345abe21dd491a4472b2fb2)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**duckdb/ingest**) drop OsStr::from_encoded_bytes_unchecked and document ingest-mutex/query_fn lifetime (UNSAFE-1 CONC-2 TASK-1104 TASK-1073) - ([4398476](https://github.com/rsvalerio/ops/commit/439847661613634eb10d136ad697d5238dde3ba9)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**git**) remove orphan #[test] and reattach doc comment in config.rs tests (TEST-1 TASK-1108) - ([495e1e6](https://github.com/rsvalerio/ops/commit/495e1e6c0d262336a91400a1c57f92a549c38db0)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**git/config**) drop remote URL containing ASCII control bytes in RedactedUrl (SEC-2 TASK-1102) - ([7200d21](https://github.com/rsvalerio/ops/commit/7200d215e1429734ab13f456f0cdfd391f7f6049)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**go/about**) require whitespace/start-of-line before // in go.mod strip_line_comment (PATTERN-1 TASK-1107) - ([3ee5e35](https://github.com/rsvalerio/ops/commit/3ee5e35298b8a2aea4b02f030260d8f51b73627c)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**go/about**) use "." sentinel for single-mod ProjectUnit path (PATTERN-1 TASK-1085) - ([ab4c0ab](https://github.com/rsvalerio/ops/commit/ab4c0abc2bc08c44bab15fc29b40d750bd91fa62)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**go/about**) out-of-tree detection by first path component, not starts_with("..") (PATTERN-1 TASK-1027) - ([fdfedf0](https://github.com/rsvalerio/ops/commit/fdfedf00c2a27dd9738f0fd670af6719f66c8bb2)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**hook-common**) skip shell-comment lines in has_legacy_marker scan (PATTERN-1 TASK-1072) - ([4f10189](https://github.com/rsvalerio/ops/commit/4f10189b42fa0a7a1e6782cc767e35657d179093)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**hook-common**) self-heal stale .ops-tmp on AlreadyExists in upgrade_legacy_hook (ERR-1 TASK-1113) - ([ab0def8](https://github.com/rsvalerio/ops/commit/ab0def836fe112f72c73216898471932f3f95262)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**java/about**) backslash-aware extract_quoted/extract_quoted_list in gradle lexer (PATTERN-1 TASK-1047) - ([28fb252](https://github.com/rsvalerio/ops/commit/28fb252de8bf9104a683448221e5374f04de5a90)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**java/about**) re-process post-> remainder on multi-line pom <project> opener (PATTERN-1 TASK-1022) - ([1cbbff0](https://github.com/rsvalerio/ops/commit/1cbbff0a9e5945cd66bde666e140a824a78c927e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**metadata**) warn-on-duplicate first-write-wins in package_index_by_name (PATTERN-1 TASK-1019) - ([6068b52](https://github.com/rsvalerio/ops/commit/6068b5286b965587d73850a5266dce0cd24dae28)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**metadata**) warn and keep first-seen on duplicate package ids in package_index_by_id (PATTERN-1 TASK-1100) - ([70dd821](https://github.com/rsvalerio/ops/commit/70dd821e3a29c40532bb088424a3985828751595)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**metadata**) warn when metadata_raw has multiple workspace_root rows (ERR-1 TASK-1043) - ([453c53d](https://github.com/rsvalerio/ops/commit/453c53d2c3e3c19fd1613334226611b6c10f99ca)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**metadata**) cap query_metadata_raw payload via OPS_METADATA_MAX_BYTES (ERR-1 TASK-1034) - ([43e1fe9](https://github.com/rsvalerio/ops/commit/43e1fe9f58ff59bc33e6d9b7cabe550169128048)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**metadata,test-coverage,duckdb**) bundle of metadata/coverage hardening (ERR-1 PATTERN-1 TASK-1021 TASK-1056 TASK-1057 TASK-1059 TASK-1075 TASK-1099) - ([a19b77f](https://github.com/rsvalerio/ops/commit/a19b77f9148c4408a8d4deb28be6e24d47a2f797)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**node/about**) scrub .. segments in normalize_repo_url shorthand and git schemes (SEC-14 TASK-1111) - ([2a8501b](https://github.com/rsvalerio/ops/commit/2a8501bc949e205f9ede3d1214b3561a56c73432)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**node/about**) strip CR/LF/control chars from normalize_repo_url input (SEC-2 TASK-1080) - ([244c162](https://github.com/rsvalerio/ops/commit/244c162600c6864d0e7bfe0effbbc9c42aa02e4b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**node/about**) quote-aware splitter for parse_pnpm_workspace_yaml inline list (PATTERN-1 TASK-1084) - ([8879e21](https://github.com/rsvalerio/ops/commit/8879e2139417fefbba135665d254079557f462b6)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**node/about**) unknown packageManager labels fall through to lockfile probe (PATTERN-1 TASK-1083) - ([620e6cd](https://github.com/rsvalerio/ops/commit/620e6cdacde241b003bf3d3a9b12656d747c30cb)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**node/about**) rewrite bare owner/repo npm shorthand to https://github.com URL (PATTERN-1 TASK-1060) - ([b43bdc3](https://github.com/rsvalerio/ops/commit/b43bdc36fef8164fadb53231d90ccff6295bebee)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**node/about**) strip YAML trailing comments in parse_pnpm_workspace_yaml list items (PATTERN-1 TASK-1061) - ([abf4af1](https://github.com/rsvalerio/ops/commit/abf4af1f1248b4982ddaa2880da5655332f9281f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**node/about**) rewrite git+git:// to https:// in normalize_repo_url (PATTERN-1 TASK-1049) - ([56299dd](https://github.com/rsvalerio/ops/commit/56299ddd6592a0395117fb1c6fd69a93bcf09207)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**python/about**) warn and keep first-seen on collapsed duplicate keys in normalize_urls (PATTERN-1 TASK-1110) - ([dd35052](https://github.com/rsvalerio/ops/commit/dd35052bed7ac1d2f9faba8fd076b3101e9ebba8)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**python/about**) drop documentation from pyproject homepage candidates (PATTERN-1 TASK-1062) - ([8e9cffc](https://github.com/rsvalerio/ops/commit/8e9cffcb10b5326b295736c745c8bdb7ea8a9f07)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/build**) bound workspace canonicalize cache and fold onto CommandRunner (CONC-7 TASK-1063) - ([3ec66fc](https://github.com/rsvalerio/ops/commit/3ec66fc452e987193cc13c2ba3514a8e7ad59683)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/data**) collapse data cache onto persistent Context (ARCH-9 TASK-0993) - ([607546d](https://github.com/rsvalerio/ops/commit/607546d8108551d48d57eaaeb3e249ba8e191a7f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/display**) route duplicate plan command ids to distinct progress bars via consume queue (PATTERN-1 TASK-1109) - ([9c8018a](https://github.com/rsvalerio/ops/commit/9c8018af3b0d116cff5b715692d1d385293ad150)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/exec**) own pipe-drain tasks via local JoinSet so parent abort cancels them (CONC-9 TASK-1064) - ([cac4078](https://github.com/rsvalerio/ops/commit/cac40785f4c825537dba74d2523c413d06ba62d9)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/parallel**) distinguish Ok(0) and Err(_) in resolve_env_usize warn (ERR-1 TASK-1092) - ([0ba5c61](https://github.com/rsvalerio/ops/commit/0ba5c613329eae9a341fe075fb605aedb6a29dc4)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/resolve**) fall through to non-config alias map for orphan config alias (ERR-1 TASK-1089) - ([1dd479b](https://github.com/rsvalerio/ops/commit/1dd479becbcdb9fd6dbfdf981472052a05e9a88b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**rust/about**) wrap cached manifest in LoadedManifest to preserve original glob spec (ERR-1 TASK-1076) - ([b1f328c](https://github.com/rsvalerio/ops/commit/b1f328c72b8c2a41e4fedaea82f0b28ab79b1bf3)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**rust/about**) dedup resolved_workspace_members to prevent double-count (PATTERN-1 TASK-1042) - ([3bb9542](https://github.com/rsvalerio/ops/commit/3bb954206bd9f8e5d846f30e2dfc826b9cc8b19c)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**rust/about**) typed_manifest_cache evicts true LRU victim, not arbitrary HashMap entry (CONC-2 TASK-1023) - ([a16436e](https://github.com/rsvalerio/ops/commit/a16436ecea19096c942a97ac31108b2d2275754e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**terraform/about**) match .tf extension case-insensitively in fallback walk (PATTERN-1 TASK-1025) - ([032e4b7](https://github.com/rsvalerio/ops/commit/032e4b7b4e0fdc3a4ee7c455e03c36722a22ed49)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**terraform/about**) strip HCL block comments before extracting required_version (PATTERN-1 TASK-1020) - ([cb4e424](https://github.com/rsvalerio/ops/commit/cb4e4243cf7f045e1879ed813f520f99d887e4af)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**terraform/about**) log non-NotFound read_dir + per-entry IO errors in count_local_modules (ERR-1 TASK-1018) - ([0390c25](https://github.com/rsvalerio/ops/commit/0390c25724195395b2197a1db3068a47dea43669)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**tests**) remove orphan workspace-root tests/integration.rs (DUP-1 TASK-1024) - ([61d310b](https://github.com/rsvalerio/ops/commit/61d310b8536df51d6d51e8224edaeae0e659ca0d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**tools**) reject only rustup diagnostic prefixes in parse_active_toolchain (PATTERN-1 TASK-1078) - ([23b9ff1](https://github.com/rsvalerio/ops/commit/23b9ff1de3d93c083a151272ff449bf39de4630b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**tools**) route stderr snippets through byte-bounded format_error_tail in probe (ERR-1 TASK-1032) - ([c75bcf1](https://github.com/rsvalerio/ops/commit/c75bcf17d81dd470e7d7e4292bf6c0f303f33916)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**tools/install**) name both package and bin in cargo install failure breadcrumb (ERR-2 TASK-1048) - ([3050694](https://github.com/rsvalerio/ops/commit/3050694c45537d4efadc4de0ed084e74db2cf8a1)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**tools/install**) prefer rustup component over cargo install when both set (ERR-2 TASK-1038) - ([6c0f9a0](https://github.com/rsvalerio/ops/commit/6c0f9a056f84888e6816a701c83cb416dab6c44d)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**tools/probe**) exclude cargo built-in subcommands from is_in_cargo_list (PATTERN-1 TASK-1101) - ([9ac19a6](https://github.com/rsvalerio/ops/commit/9ac19a6f4420c8957f2e4f82199efb1cb5988868)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### ⚡ Performance
- (**about**) deserialize from &Value via Deserialize::deserialize, avoiding Arc clone (PERF-3 TASK-1117) - ([410020f](https://github.com/rsvalerio/ops/commit/410020f6e58fab6447a99631cca32d4d3e15d571)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/text**) cache manifest_max_bytes behind OnceLock (PERF-3 TASK-1055) - ([a554e42](https://github.com/rsvalerio/ops/commit/a554e424e087f7ed0c5a747600acff7a3cf42dac)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**deps**) replace per-row to_ascii_lowercase alloc with byte-window scan in categorize_upgrades (PERF-3 TASK-1112) - ([5dbdeda](https://github.com/rsvalerio/ops/commit/5dbdeda1fb6a06da733a8833378b5b6b870f181b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/build**) double-checked locking in canonical_workspace_cached (PERF-3 TASK-1095) - ([e315815](https://github.com/rsvalerio/ops/commit/e31581570ed5b80b00fa16641b3060c7916fc873)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/secrets**) replace per-key to_lowercase alloc with byte-level case-fold (PERF-3 TASK-1053) - ([e30fb50](https://github.com/rsvalerio/ops/commit/e30fb50b734603b53ddb1259a9c591390d3e81d5)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**rust/about**) memoize query_project_coverage across identity and coverage providers (DUP-1 TASK-1079) - ([0c46b3d](https://github.com/rsvalerio/ops/commit/0c46b3dd256c9fcca4f518460e4bb833f0d108a2)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**theme**) cache ConfigurableTheme left_pad_str at construction (PERF-3 TASK-1035) - ([fe83b18](https://github.com/rsvalerio/ops/commit/fe83b180f7f49303dcd28b1aa8b4190250c76380)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**tools**) cache PATH binary set in collect_tools instead of per-tool walks (PERF-3 TASK-1046) - ([cd0e779](https://github.com/rsvalerio/ops/commit/cd0e7798fb41d43336cdd7e0f50aead4a2ec5f6f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### 📚 Documentation
- (**cargo-toml**) document symlink threat model on find_workspace_root (PATTERN-1 TASK-1036) - ([731d19c](https://github.com/rsvalerio/ops/commit/731d19c4cf6c01939e8b62bef40952e07024af12)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/config**) document global_config_path precedence and LOAD_CONFIG_CALL_COUNT serial-test contract (PATTERN-1 CONC-7 TASK-1090 TASK-1093) - ([0d3738c](https://github.com/rsvalerio/ops/commit/0d3738c95efd3635559243a33f7fc7401ad1d92f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/expand**) document TMPDIR_DISPLAY process-lifetime cache contract (READ-5 TASK-1068) - ([aa366d6](https://github.com/rsvalerio/ops/commit/aa366d65f191ba0e8a06f6e5ee48d55fea725a2a)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**hook-common**) document fail_fast=true policy in ensure_config_command (PATTERN-1 TASK-1114) - ([f2f634c](https://github.com/rsvalerio/ops/commit/f2f634ccfc4dc2c4e0b1c9d87ec2e5001a56703f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/results**) correct OUTPUT_BYTE_CAP doc to usize, surface usize::MAX in warn (READ-4 TASK-1058) - ([91c4f21](https://github.com/rsvalerio/ops/commit/91c4f2104811214dbb02323c8ec27dded5065333)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### 🧪 Testing
- (**cli**) assert secret absent in dry-run, disambiguate invalid-ops_d failure (TEST-12 TASK-1081 TASK-1082) - ([d874410](https://github.com/rsvalerio/ops/commit/d8744100b2441a650c47b80b8ebf47f370301650)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core**) pin TMPDIR Arc identity in from_env_amortises_tmpdir via Arc::ptr_eq (TEST-11 TASK-1037) - ([8cd3ed1](https://github.com/rsvalerio/ops/commit/8cd3ed197c236ff3ba2dabc4bcbe43c12a6b6a4f)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**git**) add missing #[test] on origin_section_present_but_no_url_returns_none (TEST-1 TASK-1016) - ([8db4298](https://github.com/rsvalerio/ops/commit/8db429886a63af430acfa01c71cdc0ca3b7d6808)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**runner/secrets**) replace wall-clock budget with byte-count cap proxy (TEST-15 TASK-1098) - ([1438433](https://github.com/rsvalerio/ops/commit/1438433894d916707f445d4d01de9ac7b03b819c)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### 🔧 Build System
- (**workspace**) centralize comfy-table/terminal_size/shellexpand/wait-timeout in [workspace.dependencies] (ARCH-11 TASK-1039) - ([52c3947](https://github.com/rsvalerio/ops/commit/52c3947c8dbfcf9a63e39cfedc8012dab01e57bc)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### 🚜 Refactoring
- (**cli/extension_cmd**) extract description_col helper to dedup column lookup (DUP-3 TASK-1118) - ([0b4766d](https://github.com/rsvalerio/ops/commit/0b4766d4de0cebbed01b62df4208a5392f6e198e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**cli/registry**) rename seed_owners to snapshot_initial_owners and clarify docs (PATTERN-1 TASK-1097) - ([715f545](https://github.com/rsvalerio/ops/commit/715f545512d773f7a4ebb9366c22a69652db2521)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/project_identity**) replace .clone().filter() with .as_ref().filter().cloned() (OWN-8 TASK-1119) - ([ba86e54](https://github.com/rsvalerio/ops/commit/ba86e54b94a7d176d7e736cefddd64cad572a0e4)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**core/ui**) extract emit_to<W> writer-generic helper, route tests through it (DUP-1 TASK-1031) - ([2277f15](https://github.com/rsvalerio/ops/commit/2277f15aba457ee50401f13834e15e941908fb87)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**deps**) route interpret_deny_result through truncate_for_log helper (DUP-1 TASK-1045) - ([b5f9bf6](https://github.com/rsvalerio/ops/commit/b5f9bf6a93f2b075fe1d27bbabf71bcfe3d5de0a)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
- (**tfplan**) decouple --no-color preference from TTY detection in plan pipeline (PATTERN-1 TASK-1017) - ([c9bee9e](https://github.com/rsvalerio/ops/commit/c9bee9efaa232b1aeab3460d4cb0723bc02231ff)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)
#### ⚙️ Miscellaneous
- (**backlog**) mark TASK-1024 Done - ([68eb8ac](https://github.com/rsvalerio/ops/commit/68eb8ac0d81b05f0fba304c1fdccd5a7633e1b2a)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.7 (1M context)

- - -

## [v0.28.0](https://github.com/rsvalerio/ops/compare/94f03bc5538e87e2f006594c9c49110805ef70f8..v0.28.0) - 2026-05-07
#### 🚀 Features
- (**git/config**) add breadcrumb when origin section has no extractable url (ERR-7 TASK-0966) - ([f876c69](https://github.com/rsvalerio/ops/commit/f876c692c62c05a35358ab9a2f930b2ce0b41cb6)) - [@rsvalerio](https://github.com/rsvalerio)
- expose find_workspace_root_with_depth + MAX_ANCESTOR_DEPTH (ARCH-2 TASK-0963) - ([5068db6](https://github.com/rsvalerio/ops/commit/5068db6bfd624979328f0b9cde0b0580dd3c323d)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**about**) use Debug for io::Error tracing fields to prevent log injection (ERR-7 TASK-0999) - ([1617adc](https://github.com/rsvalerio/ops/commit/1617adcffd52e855fa4495d38a225c80c30bbbf1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about/text_util**) pad_to_width_plain delegates to display_width (PERF-3 TASK-1001) - ([cd0f906](https://github.com/rsvalerio/ops/commit/cd0f90600226356e1867b4d6606e337c45b46dfb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) sort available-extensions list in unknown-extension error (PATTERN-1 TASK-0990) - ([4a34d07](https://github.com/rsvalerio/ops/commit/4a34d074b6ea8f1f0a64dc32c31778c95f496d15)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config/loader**) use Debug for path tracing fields to prevent log injection (ERR-7 TASK-0965) - ([a846694](https://github.com/rsvalerio/ops/commit/a8466949f584bb86febb49301c12f0f0deaa983f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/style**) gate ANSI helpers on TTY and NO_COLOR (READ-9 TASK-0950) - ([ab99e9e](https://github.com/rsvalerio/ops/commit/ab99e9e21fc119d1cd891a908233c4016dc1fa9d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) scope create_ingest_dir 0o700 to leaf; restrict validate_path_chars to ASCII (SEC-25 TASK-1000, READ-5 TASK-1002) - ([447513c](https://github.com/rsvalerio/ops/commit/447513c365ae1da5e4cf2a5de8f39c64f20804b6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb/ingestor**) rename JSON to .done before unlink for crash-safe cleanup (ARCH-2 TASK-1008) - ([6a0ef98](https://github.com/rsvalerio/ops/commit/6a0ef986c2a592463f618b7acf6205ddc6dac906)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb/ingestor**) log debug on NotFound JSON during cleanup_artifacts (ARCH-2 TASK-1005) - ([345ead7](https://github.com/rsvalerio/ops/commit/345ead74e58ecaa90badec8dc007eb953889732a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git/config**) typed SectionHeaderError so malformed remote sections log a reason (READ-5 TASK-1006) - ([a639d89](https://github.com/rsvalerio/ops/commit/a639d896a674bb5b6da5fe1f9ee3ecb23b14fa95)) - [@rsvalerio](https://github.com/rsvalerio)
- (**go**) tighten looks_like_module_version to numeric MAJOR.MINOR (PATTERN-1 TASK-0976) - ([db2ba43](https://github.com/rsvalerio/ops/commit/db2ba43216637ba80ae3e0574825cd402e9703bf)) - [@rsvalerio](https://github.com/rsvalerio)
- (**go/about**) is_block_opener accepts trailing inline comment (PATTERN-1 TASK-0994) - ([6ceafb9](https://github.com/rsvalerio/ops/commit/6ceafb98e86f072866beb0cfba5ad2c94160f614)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common/git**) emit breadcrumbs on canonicalize failure in gitdir resolution (ERR-1 TASK-1004) - ([f818d0a](https://github.com/rsvalerio/ops/commit/f818d0a6a1eaeb822d16c4e821e809e433279510)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python**) wrap email-only authors in angle brackets to match node format_person (ERR-2 TASK-0980) - ([5dc92c7](https://github.com/rsvalerio/ops/commit/5dc92c753736b91d7dbe6c206e4d5da7a39d65a6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python**) include manifest path in pyproject parse warns (ERR-7 TASK-0974) - ([cd2228b](https://github.com/rsvalerio/ops/commit/cd2228bd76075aeeafc68ed85ca896463a8e998d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) output_byte_cap peak warning reuses clamped OPS_MAX_PARALLEL (PERF-3 TASK-0995) - ([768e7d4](https://github.com/rsvalerio/ops/commit/768e7d47f5e89093666cd8a6311ba0c488a63c1b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/parallel**) count terminal events per id so duplicates pair correctly (PATTERN-1 TASK-0997) - ([052eb7e](https://github.com/rsvalerio/ops/commit/052eb7ee9185974482636ce978ab2e1517bb1501)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/about**) short-circuit non-UTF-8 cwd in coverage provider (READ-5 TASK-0986) - ([76a4922](https://github.com/rsvalerio/ops/commit/76a492209987d8cf5eb2726ca0cd1fc47d3312a7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/cargo-toml**) find_workspace_root surfaces EACCES instead of treating as missing (ERR-1 TASK-0988) - ([0045d75](https://github.com/rsvalerio/ops/commit/0045d7513a5a94b622d2c3c21f8b7c7054126309)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/deps**) clamp separator slices to char boundaries; surface non-JSON deny stderr (ERR-1 TASK-0960, TASK-0958) - ([11a7dbd](https://github.com/rsvalerio/ops/commit/11a7dbd515d81d3315278681713c1e45c8ae708a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/metadata**) compare manifest paths by component; retain path intra-workspace deps (PATTERN-1 TASK-0952, TASK-0982) - ([873e6c7](https://github.com/rsvalerio/ops/commit/873e6c7eca4cd54d131eb84503f100b0cb3570a0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/test-coverage**) flatten_coverage_json drops rows with missing filename (ERR-1 TASK-0984) - ([76954b9](https://github.com/rsvalerio/ops/commit/76954b971cf0f79b461832d34c6a85fb0a0c3636)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tools/probe**) use Debug for subprocess stderr fields to prevent log injection (ERR-7 TASK-0979) - ([432d7b8](https://github.com/rsvalerio/ops/commit/432d7b82f9d63239c85080a9d0e480450a3bac59)) - [@rsvalerio](https://github.com/rsvalerio)
- sanitise untrusted strings in tracing breadcrumbs and ui::emit (wave 70) - ([187fefc](https://github.com/rsvalerio/ops/commit/187fefccfdde568d7789ec059b130eb5b81bed15)) - [@rsvalerio](https://github.com/rsvalerio)
- drop whitespace-only URLs in pyproject pick_url (ERR-2 TASK-0964) - ([83841ca](https://github.com/rsvalerio/ops/commit/83841ca2f084dfc4de3a0bf5ec83f96aee3ca8ee)) - [@rsvalerio](https://github.com/rsvalerio)
- emit warn on every typed_manifest_cache poison recovery (ARCH-2 TASK-0962) - ([30fc6c3](https://github.com/rsvalerio/ops/commit/30fc6c378d1b519e38080639fb843de11bc2e00c)) - [@rsvalerio](https://github.com/rsvalerio)
- keep keywords/categories Inherited when workspace declares no value (PATTERN-1 TASK-0961) - ([bc36f70](https://github.com/rsvalerio/ops/commit/bc36f707f4d5d1f2c44a74e2a95fac0cea3bc7f8)) - [@rsvalerio](https://github.com/rsvalerio)
- clamp negative per-crate i64 values to 0 with warn (ERR-1 TASK-0959) - ([1990de6](https://github.com/rsvalerio/ops/commit/1990de6cd67963ac63dc84978a4af629b90c629a)) - [@rsvalerio](https://github.com/rsvalerio)
- reject trailing tokens in cargo-update Adding/Removing lines (ERR-1 TASK-0949) - ([1ce6a17](https://github.com/rsvalerio/ops/commit/1ce6a17046746e0db9dfa02c3a4869fe40adf1d4)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**duckdb**) ingest_mutex_for keyed by &'static str to eliminate per-call alloc (PERF-3 TASK-1007) - ([15b8f91](https://github.com/rsvalerio/ops/commit/15b8f91057608667422cb74447bc6226fd9133d9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python/about**) normalise URL keys once per extract_urls (PERF-3 TASK-0991) - ([429e37f](https://github.com/rsvalerio/ops/commit/429e37f9b0f6e1138b95a15e1d1877ca79f20ce3)) - [@rsvalerio](https://github.com/rsvalerio)
- pre-size read_optional_text buffer from file metadata (PERF-1 TASK-0971) - ([4a04566](https://github.com/rsvalerio/ops/commit/4a04566cefac6681d0bd93c8d490208eb76848aa)) - [@rsvalerio](https://github.com/rsvalerio)
- return Cow from cargo-update strip_ansi to skip alloc on common path (PERF-3 TASK-0970) - ([e654b2c](https://github.com/rsvalerio/ops/commit/e654b2c214587f8adc91a4476cb44ffe48df76a3)) - [@rsvalerio](https://github.com/rsvalerio)
- build per-crate placeholders without intermediate Vec (PERF-3 TASK-0968) - ([72bf4a0](https://github.com/rsvalerio/ops/commit/72bf4a08d6d0272cceaf675c1f6a6cbf299f0626)) - [@rsvalerio](https://github.com/rsvalerio)
- cache TMPDIR as Arc<str> in Variables::from_env (PERF-3 TASK-0967) - ([95aca2f](https://github.com/rsvalerio/ops/commit/95aca2fa59c96a8e52c9a3c4e9f97ae7d68267ca)) - [@rsvalerio](https://github.com/rsvalerio)
- avoid per-line String alloc in ProgressState::record_stderr (PERF-3 TASK-0948) - ([94f03bc](https://github.com/rsvalerio/ops/commit/94f03bc5538e87e2f006594c9c49110805ef70f8)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**agents**) document Rust implementation guardrails - ([2b1c0a8](https://github.com/rsvalerio/ops/commit/2b1c0a81cda56fc77523b95064ca24aaed6ce85a)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**cli**) isolate user env, assert error messages, kill flaky sleep (TEST-11/15/18/25 TASKS-0953/0954/0955/0957) - ([31cfaf9](https://github.com/rsvalerio/ops/commit/31cfaf9aac309d69e5345b611baf7f45efe45bef)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tools**) drop assertions for removed ToolStatus::Unknown variant - ([30bdeb3](https://github.com/rsvalerio/ops/commit/30bdeb34af136186f62dde284299eced0e0f9544)) - [@rsvalerio](https://github.com/rsvalerio)
- pin resolved-members amortisation via typed_manifest_cache (PERF-3 TASK-0969) - ([027c17d](https://github.com/rsvalerio/ops/commit/027c17dd3264356687c1fc887b574b334d6561f0)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) extract shared manifest_cache from node/python (DUP-1 TASK-0973, TEST-18 TASK-0956) - ([9112219](https://github.com/rsvalerio/ops/commit/911221989c30f33151cc0237166ac3c1a42f4506)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) inline parse_package_metadata shims at call sites (DUP-3 TASK-0987) - ([6923215](https://github.com/rsvalerio/ops/commit/6923215aef9857402c34e025d8610ec74e24c80f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) share Debug-escape assertion via ops_about::test_support (DUP-3 TASK-0985) - ([0324c3e](https://github.com/rsvalerio/ops/commit/0324c3eaf71c27bfd8c7ea5ed05ea45873f11353)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) unify is_actionable predicates behind a relax_warning param (DUP-3 TASK-0989) - ([ad8648b](https://github.com/rsvalerio/ops/commit/ad8648b4ff9df6c2f5a5634cdaa8294eb811f084)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) collapse severity_icon/colorize_severity onto SeverityClass enum (DUP-3 TASK-0972) - ([0a97460](https://github.com/rsvalerio/ops/commit/0a9746060d7ff5dcb15e025c15bc8ade8690d682)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension/data**) collapse provider_names_iter into provider_names (API-3 TASK-0996) - ([eefdca8](https://github.com/rsvalerio/ops/commit/eefdca8b956518168a5a685ec5cdf4cd9b001ae0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**project_identity**) make stack_emoji defer to language_emoji canonical table (DUP-3 TASK-0983) - ([1bb9c51](https://github.com/rsvalerio/ops/commit/1bb9c514f30927ef074fece737c7f13eb75c8419)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) merge_alias_for routes through Entry API (PATTERN-1 TASK-0998) - ([1278620](https://github.com/rsvalerio/ops/commit/127862049c726668dccb07dfa0418f4c0f714e6a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) unify strip_ansi and visible_width behind one ANSI parser (DUP-1 TASK-0978) - ([f0f962b](https://github.com/rsvalerio/ops/commit/f0f962bb5e6c41f3e7a15c0fc4a88feb488fcf81)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tokei**) tokei_languages_view_sql returns String via TableName::from_static (ERR-5 TASK-1003) - ([42f1d64](https://github.com/rsvalerio/ops/commit/42f1d6486f166cdeb937e4a162e605359d509713)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tools**) remove dead ToolStatus::Unknown variant (READ-7 TASK-0992) - ([f8462cb](https://github.com/rsvalerio/ops/commit/f8462cb0077ac6d27ac1a73d42e1e83c94afea1c)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) close wave 69/71-75 tasks and TASK-0951 - ([5040f34](https://github.com/rsvalerio/ops/commit/5040f34521517b0826845ee5bec5f152ddbcffc3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) triage 22 findings into waves 69-75; close wave 70 - ([9c796d3](https://github.com/rsvalerio/ops/commit/9c796d33e1794caf40a86eb87da6cb2e5923466f)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.27.1](https://github.com/rsvalerio/ops/compare/0fbc6fa0c9ad080a7362f1242668d2bf705845a5..v0.27.1) - 2026-05-03
#### 🐛 Bug Fixes
- (**cli/theme**) use Debug for path/error logging to prevent log injection (ERR-7 TASK-0944) - ([aa3ae16](https://github.com/rsvalerio/ops/commit/aa3ae16da230ee481d02ea56b2a88d0b7aaf363f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/manifest**) add byte-cap for reads via read_capped_to_string and OPS_MANIFEST_MAX_BYTES (SEC-33 TASK-0932) - ([0fbc6fa](https://github.com/rsvalerio/ops/commit/0fbc6fa0c9ad080a7362f1242668d2bf705845a5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/stack**) use Debug for path/error logging and improve error handling (ERR-7 TASK-0945) - ([6c0a79c](https://github.com/rsvalerio/ops/commit/6c0a79c20c9a39db0218c7a3a7baa6a2aadb5654)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/subprocess**) add SpawnError struct to preserve label context in spawn failures (ERR-4 TASK-0925) - ([e651028](https://github.com/rsvalerio/ops/commit/e6510285f407f9b88fee79a50a080ac1ac4003eb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) use byte-capped manifest readers for config/schema files (SEC-33 TASK-0932) - ([be7daa8](https://github.com/rsvalerio/ops/commit/be7daa805633625ba681b91624c1ef14144bbaf7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/about**) use byte-capped manifest readers for workspace detection (SEC-33 TASK-0932) - ([30114ba](https://github.com/rsvalerio/ops/commit/30114bafbbd7dba2cb164fb629390bf4ff0e6055)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git/config**) reject dot-only path segments in remote URLs (SEC-13 TASK-0929) - ([7eda08d](https://github.com/rsvalerio/ops/commit/7eda08d952e0de86d2037fd85fcf4c7841d9fb0b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common/git**) use Debug for path/error logging to prevent log injection (ERR-7 TASK-0937) - ([69ff9f7](https://github.com/rsvalerio/ops/commit/69ff9f750602410e9631d794180c5ee731b3d282)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/tap**) use Debug for path/error logging to prevent log injection (ERR-7 TASK-0940) - ([612f0a7](https://github.com/rsvalerio/ops/commit/612f0a7649693a990408c77cfc349f676348d215)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/extensions**) use byte-capped manifest readers (SEC-33 TASK-0932) - ([f3badf5](https://github.com/rsvalerio/ops/commit/f3badf55d03f9a183cdd2c02ff8149634e7e2fee)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/plan**) byte-cap stdin reads at OPS_PLAN_JSON_MAX_BYTES (SEC-33 TASK-0924) - ([79e8b91](https://github.com/rsvalerio/ops/commit/79e8b91f5cd00752bcdf914ed1faee8d467a5ce3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme/sgr**) improve error handling in color formatting - ([ce1bc93](https://github.com/rsvalerio/ops/commit/ce1bc931265d35cdf2bcbcacc8494f5c1713f117)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**node/about**) add process-local cache for package.json to avoid duplicate reads (DUP-3 TASK-0931) - ([dccd975](https://github.com/rsvalerio/ops/commit/dccd9756c98dfb2eb7d981cea477000a31e6a702)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**deps**) update lockfile - ([a744f21](https://github.com/rsvalerio/ops/commit/a744f2123ef33f11ef4e0d1b94dc0eaf3bc487ec)) - [@rsvalerio](https://github.com/rsvalerio)
- add some more guidance to code agents - ([250f0f0](https://github.com/rsvalerio/ops/commit/250f0f02a0fcecb1075c9bbb437f508e06cc5b50)) - [@rsvalerio](https://github.com/rsvalerio)
- add code review tasks files - ([fe9bba3](https://github.com/rsvalerio/ops/commit/fe9bba3f70a2de4b490bf56439a9dcfd29912cec)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.27.0](https://github.com/rsvalerio/ops/compare/1181347cbdb14488ebf17e1e2573da7f8861f5ee..v0.27.0) - 2026-05-02
#### 🚀 Features
- (**core/project_identity**) add #[non_exhaustive] + new() to public sibling structs (TASK-0858) - ([5c111e2](https://github.com/rsvalerio/ops/commit/5c111e2a66f1e9d0b33e78c2e3458869a92591a2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/plan**) add run_plan_pipeline_to(&mut Write) for library reuse (TASK-0850) - ([bfd25ae](https://github.com/rsvalerio/ops/commit/bfd25ae7d5c17db759deafee3b8dacbe25aa99d6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) add resolve_theme_owned no-clone variant via swap_remove (TASK-0836) - ([78bc0b8](https://github.com/rsvalerio/ops/commit/78bc0b8eedaf770e9b7bfb0f62032eb19cac6c75)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**about**) cap manifest reads at 4 MiB (TASK-0831) - ([1181347](https://github.com/rsvalerio/ops/commit/1181347cbdb14488ebf17e1e2573da7f8861f5ee)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/about_card**) replace AboutCard::new with builder for non_exhaustive integrity (TASK-0892) - ([c1ea900](https://github.com/rsvalerio/ops/commit/c1ea900ab252a7c60f93aa5a9aacf54bdf41d8ed)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) preserve restrictive perms across atomic_write (TASK-0898) - ([6afd10d](https://github.com/rsvalerio/ops/commit/6afd10d43ec86867d54aaf28e10c9ed5273bb7ae)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) build atomic_write tmp basename from OsStr bytes (TASK-0837) - ([8c70061](https://github.com/rsvalerio/ops/commit/8c700616c371e55622b4e2fbc139cb121d752a60)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/expand**) preserve VarError cause via Error::source on ExpandError (TASK-0835) - ([91f720a](https://github.com/rsvalerio/ops/commit/91f720a778cf27b2cdb68d0721d7a145de3b9655)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/paths**) gate USERPROFILE fallback to non-Unix targets (TASK-0891) - ([3462d45](https://github.com/rsvalerio/ops/commit/3462d45dea988989e7f7e4c4f4b25b556d24391a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/subprocess**) propagate drain-thread panic as RunError::Io (TASK-0901) - ([9fc67e8](https://github.com/rsvalerio/ops/commit/9fc67e86c293fb33e2bbb21c1eb6749c0ae4c854)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) check cargo-upgrade exit status before parsing (TASK-0913) - ([161a701](https://github.com/rsvalerio/ops/commit/161a701b4351a79dcf788a5a4225e8f3700ef2bd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) TableName const-validated newtype for SidecarIngestorConfig.count_table (TASK-0856) - ([bd985dd](https://github.com/rsvalerio/ops/commit/bd985ddd84d2d49287a21a63cdf826ea28cc85aa)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) widen data_sources.record_count to BIGINT (TASK-0885) - ([4331d44](https://github.com/rsvalerio/ops/commit/4331d444e08c57bef803085bc1034b2413eddbbd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb/ingest**) drop table inside ingest_mutex on refresh path (TASK-0909) - ([b45306d](https://github.com/rsvalerio/ops/commit/b45306d12273eb30f8e0b58c471fdf9197927255)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb/ingestor**) collect_sidecar JSON write via atomic_write (TASK-0911) - ([b52401b](https://github.com/rsvalerio/ops/commit/b52401bba015e7e1322b97b5c7062a140370ce3b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb/sql**) escalate DbError::MutexPoisoned/Timeout to error in query_or_warn (TASK-0855) - ([108805a](https://github.com/rsvalerio/ops/commit/108805affb34a8c359ff08fb6d72356148248239)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git/config**) byte-cap read_origin_url at 4 MiB (TASK-0910) - ([39669eb](https://github.com/rsvalerio/ops/commit/39669eb4a511e03a8ed9d00b709a51d38855f63c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git/config**) introduce RedactedUrl newtype to enforce userinfo invariant (TASK-0894) - ([c6628b6](https://github.com/rsvalerio/ops/commit/c6628b6f41798dcef3cdf4c62ccd3bb1df5f5f06)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hooks**) apply CwdEscapePolicy::Deny on hook-triggered exec path (TASK-0886) - ([df4e358](https://github.com/rsvalerio/ops/commit/df4e358ecdfca51d44b229ceb76a6ffd536ed410)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java/about**) decode XML entities in pom.xml extract_xml_value (TASK-0916) - ([5e0d8f7](https://github.com/rsvalerio/ops/commit/5e0d8f71c45b6847106f39bea99295bc9c40b77a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java/about**) strip XML comments in pom.xml line scanner (TASK-0846) - ([a7a34c4](https://github.com/rsvalerio/ops/commit/a7a34c470558e29a5f7ccc950a8f0f2d77e75775)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/build**) reject non-UTF-8 spec cwd loudly instead of lossy-expanding (TASK-0900) - ([3bbb94e](https://github.com/rsvalerio/ops/commit/3bbb94e36e8d7605e616f9bd2b734d899b1e8e12)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/results**) warn once on invalid OPS_OUTPUT_BYTE_CAP values (TASK-0840) - ([c110d12](https://github.com/rsvalerio/ops/commit/c110d1262cb5332fcc8cb2e62eeedf3a3484a9d8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/about**) recover typed_manifest_cache from poison with one-shot warn (TASK-0844) - ([5fe196d](https://github.com/rsvalerio/ops/commit/5fe196d0d3ddbf5370bd28b8a373ca13353a15f2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/about**) bound typed_manifest_cache and invalidate on mtime change (TASK-0843) - ([d60d07b](https://github.com/rsvalerio/ops/commit/d60d07bf54533e84b932b8d008a1a4332bf4cdb1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/cargo-toml**) map canonicalize NotFound to FindWorkspaceRootError::NotFound (TASK-0918) - ([69fc39e](https://github.com/rsvalerio/ops/commit/69fc39e207e280dbc8d850f2b5935fc0aaf276e9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/cargo-update**) strip_ansi iterates chars to preserve non-ASCII UTF-8 (TASK-0882) - ([0637b86](https://github.com/rsvalerio/ops/commit/0637b862c872b91cb81f50028f86b774c89b814a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/deps**) preserve missing cargo-deny severity as distinct sentinel (TASK-0845) - ([03fbd69](https://github.com/rsvalerio/ops/commit/03fbd69cb62b37e50ec5f37e2abb58b9a7c9790c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/tools**) cap cargo/rustup probe spawns with run_with_timeout (TASK-0914) - ([d80932a](https://github.com/rsvalerio/ops/commit/d80932a697b9586b62606ce12531477dfb4d161c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/about**) require required_version inside terraform block (TASK-0919) - ([87b703c](https://github.com/rsvalerio/ops/commit/87b703c42831a17913c3c351804347820b260cdc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/about**) require quoted required_version, strip comments, cap length (TASK-0853) - ([788a87a](https://github.com/rsvalerio/ops/commit/788a87a8c8ff882e27200dfba5299c509b9ed765)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/about**) sort read_dir fallback for deterministic required_version (TASK-0852) - ([4de1367](https://github.com/rsvalerio/ops/commit/4de136713cc03b270385e1fdef639bccc1abd119)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/about**) route .tf reads through manifest_io with warn-on-IO-error (TASK-0851) - ([240eeb1](https://github.com/rsvalerio/ops/commit/240eeb16d61f8c490348f91c8a42fe2a1e5dc8f0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/plan**) byte-cap read_json_file with OPS_PLAN_JSON_MAX_BYTES override (TASK-0915) - ([a8ff2ea](https://github.com/rsvalerio/ops/commit/a8ff2eabad39ee0e48aafa669f3eb2c14c7c52fb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/plan**) surface unrecognized actions as Action::Unknown (TASK-0833) - ([363e83b](https://github.com/rsvalerio/ops/commit/363e83b7ee229fb8c10cb383ca8b2785095a635c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/render**) skip terminal_size probe when is_tty=false (TASK-0849) - ([d216611](https://github.com/rsvalerio/ops/commit/d216611610fdc03406bab154e7062678a53243d4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) explicit f64 clamp in format_duration (TASK-0857) - ([fdcb8d9](https://github.com/rsvalerio/ops/commit/fdcb8d95e659cbd44c3ec8dbadcb3b883a482e9f)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**cli**) thread Arc<Config> end-to-end via CommandRunner::from_arc_config (TASK-0841) - ([60e00af](https://github.com/rsvalerio/ops/commit/60e00afb3f58b9a29e963bc3495e8b5441fd84d4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/extension**) hoist extension_summary out of per-row loop (TASK-0859) - ([619ded4](https://github.com/rsvalerio/ops/commit/619ded4672cc8c636ebce6a503aa24fa31e460d9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python/about**) cache pyproject.toml as Arc<str>, parse direct (TASK-0854) - ([aadb85b](https://github.com/rsvalerio/ops/commit/aadb85b3053cc62cfc2cf0d1ee0b4ec9691ad743)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/build**) switch canonical_workspace_cached to RwLock for read-mostly cache (TASK-0839) - ([e36406d](https://github.com/rsvalerio/ops/commit/e36406d5e4a80cdd5ab18db78ce99378693d96eb)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**cli/registry**) document + pin asymmetric collision policy (TASK-0904) - ([5e755ae](https://github.com/rsvalerio/ops/commit/5e755ae3053370b4026e76e25ab90201786ddd12)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) document atomic_write sync-only contract for async callers (TASK-0834) - ([15aa562](https://github.com/rsvalerio/ops/commit/15aa562388f4c88084ab81f2a8cce7eadb91c174)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/exec**) clarify emit_output_events alloc accounting + add Arc::ptr_eq regression (TASK-0838) - ([24e4e92](https://github.com/rsvalerio/ops/commit/24e4e9203192e49cfa759ae58cc9632fc2b7eb7c)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli/registry**) split into discovery + registration submodules (TASK-0842) - ([5ee8fea](https://github.com/rsvalerio/ops/commit/5ee8fea3380b5a1c637a0f6385fe6c026dbcb540)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java/about**) split gradle.rs into lexer + parser submodules (TASK-0847) - ([e24e235](https://github.com/rsvalerio/ops/commit/e24e23586d49d6c728f1eeecfe3b2af2d786b508)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node/about**) split repo_url out of package_json (TASK-0848) - ([e29e7ba](https://github.com/rsvalerio/ops/commit/e29e7baefee5473b15e0d6e404af6a50a5b070dd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/plan**) mark public model types non_exhaustive (TASK-0832) - ([07b9af1](https://github.com/rsvalerio/ops/commit/07b9af11d3ef3b15a5cc62ce3e8f30e7e949afca)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.26.0](https://github.com/rsvalerio/ops/compare/d1b69bf537e8d05356d6afe272aea7fda8957162..v0.26.0) - 2026-05-02
#### 🚀 Features
- (**duckdb**) warn on ingest mutex poison recovery (TASK-0861) - ([bfda699](https://github.com/rsvalerio/ops/commit/bfda6991557f069c8935ebcc0931c89bd373b409)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git**) debug-log when remote URL fails parse (TASK-0863) - ([04cbef6](https://github.com/rsvalerio/ops/commit/04cbef670e99ddc9a1a8b90cbb8e016db95f00ce)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) env-overridable parallel and event-budget caps (TASK-0873) - ([bf19f8f](https://github.com/rsvalerio/ops/commit/bf19f8fd92260b007b6ca2c680498fc8daf0e7bf)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**cli/plan**) exhaustive display_cmd_for, child list for composites (TASK-0903) - ([8af7fbe](https://github.com/rsvalerio/ops/commit/8af7fbebec406da2f1e609ec8d0a44f382f8c9e4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/tools**) render ToolStatus via Display, not Debug (TASK-0896) - ([78b3f1a](https://github.com/rsvalerio/ops/commit/78b3f1acd5d42bed00e14e036ca8e5920b441cf3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) strip leading dot from atomic_write tmp basename (TASK-0908) - ([9aaef52](https://github.com/rsvalerio/ops/commit/9aaef5257308e3d037c75db13d4304ef435d83c6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) warn on atomic_write parent fsync failure (TASK-0899) - ([8c95763](https://github.com/rsvalerio/ops/commit/8c95763c9da11664b6e57bbde73f16ea102e3945)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) handle OsString in env var collection via vars_os() - ([5c13feb](https://github.com/rsvalerio/ops/commit/5c13feb235492df99aa1f5423c5c644d485f1dde)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/stack**) canonicalize start before parent walk (TASK-0902) - ([02c6706](https://github.com/rsvalerio/ops/commit/02c6706b571d63eee5697b8be1ca11ab519f7d34)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) preserve anyhow chain in external_err (TASK-0862) - ([b175792](https://github.com/rsvalerio/ops/commit/b175792818f202a64ff28fe44d762e19d96d39a4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git**) warn on non-NotFound IO errors reading HEAD (TASK-0887) - ([064d1c5](https://github.com/rsvalerio/ops/commit/064d1c50ac6218cf0efcd632b66b7edccec65bf5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) track peak traversal as usize directly (TASK-0889) - ([7729abd](https://github.com/rsvalerio/ops/commit/7729abddd738a786be50b4fb5d5d400929e37e90)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python/about**) recover from pyproject cache mutex poison (TASK-0878) - ([4cdb08c](https://github.com/rsvalerio/ops/commit/4cdb08c0742ca1291edf915e85e1924a590d47e5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python/about**) bound pyproject cache residency (TASK-0867) - ([fbf6184](https://github.com/rsvalerio/ops/commit/fbf61841ebd607ec6feb241eecd54053cf5fc5b2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) typed io error on missing stdio pipes (TASK-0906) - ([79a2124](https://github.com/rsvalerio/ops/commit/79a212415ebb59c118118cb84311a07f5655e57a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/plan**) route cleanup failures through tracing::warn (TASK-0921) - ([8cbd916](https://github.com/rsvalerio/ops/commit/8cbd916a385a4b40d25530f0f1ffd0ebb8eb9e3c)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**cli**) use current_thread tokio runtime for sequential plans (TASK-0875) - ([370db24](https://github.com/rsvalerio/ops/commit/370db24817ee550a44aee1afe8edf2b65b887121)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps/format**) drop intermediate AdvisoryRow Vec (TASK-0880) - ([c514fa7](https://github.com/rsvalerio/ops/commit/c514fa73a30173de3e9c6ea51cb868bf8ac94e8b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) zero-clone Context cwd via from_cwd_arc (TASK-0890) - ([af951b1](https://github.com/rsvalerio/ops/commit/af951b19e6269efe2054e39fcc91b4e88b1d0312)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension/data**) add provider_names_iter to skip Vec alloc (TASK-0877) - ([2bc4910](https://github.com/rsvalerio/ops/commit/2bc4910a2c55f240570ade94d5534801abf69564)) - [@rsvalerio](https://github.com/rsvalerio)
- (**metadata**) O(1) package_by_name/id via lazy index (TASK-0883) - ([cd71d81](https://github.com/rsvalerio/ops/commit/cd71d81ccdc38160c9c7979465ca52ebfb569041)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/about**) inline cwd lossy borrow on coverage hot path (TASK-0917) - ([4c37413](https://github.com/rsvalerio/ops/commit/4c374137c6a6931bf22707d145e30c88a60ad5c4)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**duckdb**) clarify LoadResult API surface, drop dead_code (TASK-0879) - ([15f7aac](https://github.com/rsvalerio/ops/commit/15f7aac4689a097502cc658b293b4c03720a3f2a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) document CommandRegistry Deref as intentional surface (TASK-0874) - ([d08a874](https://github.com/rsvalerio/ops/commit/d08a874b3b574ca05983832718bd2fa37c4ac9f2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) document ProgressDisplay !Send marker, add static check (TASK-0907) - ([a8e66e8](https://github.com/rsvalerio/ops/commit/a8e66e8baaf5535881b61aace167f99eb3b2e786)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) warn on excessive in-flight capture budget (TASK-0905) - ([6d5236b](https://github.com/rsvalerio/ops/commit/6d5236b63ecf261d2d7317d8eb76aef7605eeb92)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**run-before-commit**) assert clamp warn emission (TASK-0897) - ([442a8e5](https://github.com/rsvalerio/ops/commit/442a8e53db8a5ba9ba9a6b436dac92c5f2c874f6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/display**) enhance render configuration tests - ([26f2999](https://github.com/rsvalerio/ops/commit/26f29993ee99a7ca4706a0eacfa0e01debc452a2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) update rendering tests for improved configuration - ([f7f796c](https://github.com/rsvalerio/ops/commit/f7f796c819cb206ef224d6ffc464f20903089654)) - [@rsvalerio](https://github.com/rsvalerio)
- fix display map expectation and working directory assertion - ([c6cc3c7](https://github.com/rsvalerio/ops/commit/c6cc3c7f547751c30b1fa197e1b513930996f8f7)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) use Config::empty() instead of Config::default() - ([55a80f9](https://github.com/rsvalerio/ops/commit/55a80f9e2c653d4f5a02e7099972515c5d19bb2e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) drop misleading from_ref wrapper (TASK-0895) - ([e620fb5](https://github.com/rsvalerio/ops/commit/e620fb5a118784fbbdc7ed80f83f388d9c189b2a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cargo-toml**) typed FindWorkspaceRootError variants (TASK-0871) - ([e287b82](https://github.com/rsvalerio/ops/commit/e287b8230301aa64e8d32981a756fc2d3b59ee66)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cargo-update**) derive Copy on UpdateAction, drop clones (TASK-0870) - ([fc2a6a8](https://github.com/rsvalerio/ops/commit/fc2a6a8c70338738db84134106b64c6cb676905e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) group plan args into PlanShape struct (TASK-0866) - ([5003f86](https://github.com/rsvalerio/ops/commit/5003f865d0e3a3c98a6c3aaef504f3419969b0c8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) improve extension and command execution interfaces - ([b541f9b](https://github.com/rsvalerio/ops/commit/b541f9ba203b3004930a8ddaa7cb063a72f22470)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/registry**) unify Owner enum across command and data paths (TASK-0876) - ([d901f88](https://github.com/rsvalerio/ops/commit/d901f8889745de79f71dfcf9f01a8cc214b811f2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/tools**) improve tools command rendering and configuration - ([c27f543](https://github.com/rsvalerio/ops/commit/c27f5432934bbf107c353a0251b79604b798d8d6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config**) improve loader and tool configuration handling - ([0555cf7](https://github.com/rsvalerio/ops/commit/0555cf74e5042c8d824daae2c0c90ec6b1ff58d4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) improve expansion and library exports - ([3f8e762](https://github.com/rsvalerio/ops/commit/3f8e762dd921bd034997678f39e47ff95579be83)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) gate Config Default to test, add empty() (TASK-0872) - ([40c0940](https://github.com/rsvalerio/ops/commit/40c0940766e19f375a1751ed64b31d852605e7c8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/paths**) improve path handling and resolution - ([c6ac0f2](https://github.com/rsvalerio/ops/commit/c6ac0f2f68fb972953bae7fb1b1fa62d9ffd5b12)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) newtype SourceName/WorkspaceRoot for swap safety (TASK-0912) - ([01ffe3d](https://github.com/rsvalerio/ops/commit/01ffe3d90b37582f7780f82ddd5292f828301aa5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/about**) improve data providers and card generation - ([ce53e78](https://github.com/rsvalerio/ops/commit/ce53e7837fab1bd16c612a00f69954bae1deec53)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/git**) improve provider implementation - ([812dcfc](https://github.com/rsvalerio/ops/commit/812dcfc89dd861d5f669c0ee60244643319a17d2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/run-before-commit**) improve hook configuration and execution - ([ee60d69](https://github.com/rsvalerio/ops/commit/ee60d6914d5d28e8a893046fd2abbe7d2c765989)) - [@rsvalerio](https://github.com/rsvalerio)
- (**maven**) unify project opener classifier (TASK-0923) - ([7b0a5ac](https://github.com/rsvalerio/ops/commit/7b0a5ac4ced280e1fb689fc3f679c8bb8ce0ab8c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**maven**) extract try_set_once helper for first-write-wins (TASK-0869) - ([aa0a8da](https://github.com/rsvalerio/ops/commit/aa0a8dab38ff250f6b8d4325734c4ab51a07c06e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**project_identity**) improve card generation and display - ([4c2c91e](https://github.com/rsvalerio/ops/commit/4c2c91e59192b105d8524d01dd883b10384e4e5b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**run-before-commit**) name STDERR_DRAIN_GRACE const (TASK-0864) - ([f667da1](https://github.com/rsvalerio/ops/commit/f667da165cbaa524d58b772b427188748c298830)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/command**) improve execution and concurrency handling - ([7b68737](https://github.com/rsvalerio/ops/commit/7b687375ef4fefeb4192e903a5fe652814a94cc8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/display**) improve render configuration and output handling - ([99e0566](https://github.com/rsvalerio/ops/commit/99e0566410e7be452f19bdeba3ad3d585c76429a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform**) drop dead "." sentinel from version scan (TASK-0868) - ([6ee2a85](https://github.com/rsvalerio/ops/commit/6ee2a85c0cc250d47e3914bd6de1dea93133be37)) - [@rsvalerio](https://github.com/rsvalerio)
- (**terraform/plan**) name table-width magic numbers (TASK-0920) - ([abd38ed](https://github.com/rsvalerio/ops/commit/abd38ed3ec8b75e4e3bc740b54f9ab665850e100)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) apply_with_prefix takes Option<&str> (TASK-0893) - ([8551a0d](https://github.com/rsvalerio/ops/commit/8551a0d26bc3be599e547dbb1770023194b6b743)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) fold StepLineTheme trait into ConfigurableTheme (TASK-0865) - ([4b68b21](https://github.com/rsvalerio/ops/commit/4b68b216682318c6a1af46dfeba6864a61cd36ec)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) improve configuration, resolution, and styling logic - ([76fddd2](https://github.com/rsvalerio/ops/commit/76fddd2e7eca07a4018e243c66c05c619e12ba8b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme/style**) split into sgr and strip submodules (TASK-0881) - ([06ed6f4](https://github.com/rsvalerio/ops/commit/06ed6f496e3edfa03ca72b278549e5dda67eab2e)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) update code review wave 69 findings and task statuses - ([472eacd](https://github.com/rsvalerio/ops/commit/472eacd80aa0f08fa9211e132ca5ba42bb96943f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) add #[non_exhaustive] to extension structs (TASK-0922) - ([363b071](https://github.com/rsvalerio/ops/commit/363b07113d1d318ea8d8c0b8f0d338c41a410339)) - [@rsvalerio](https://github.com/rsvalerio)
- (**metadata**) add #[non_exhaustive] to public wrappers (TASK-0884) - ([a0f4ba0](https://github.com/rsvalerio/ops/commit/a0f4ba0081a57b8d11338521c12f0e99157a0143)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tokei**) demote TOKEI_DEFAULT_EXCLUDED to pub(crate) (TASK-0888) - ([046771f](https://github.com/rsvalerio/ops/commit/046771ffc39a1cd7dfde91dd6b1ebd15377503bf)) - [@rsvalerio](https://github.com/rsvalerio)
- add #[non_exhaustive] to PomData and PackageJson (TASK-0860) - ([b5acdd1](https://github.com/rsvalerio/ops/commit/b5acdd15eb2ffd4aba2d18e27b0be0a485372e9b)) - [@rsvalerio](https://github.com/rsvalerio)
- change before push command to `qa` sub command - ([d1b69bf](https://github.com/rsvalerio/ops/commit/d1b69bf537e8d05356d6afe272aea7fda8957162)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.25.0](https://github.com/rsvalerio/ops/compare/cc7d4330ec8762ac772654237cc26a60f72c1aa2..v0.25.0) - 2026-05-01
#### 🚀 Features
- (**cli**) add terraform plans subcommand with options - ([47ade9a](https://github.com/rsvalerio/ops/commit/47ade9ab99ea8165b89681d11be5f793b938e6ad)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-terraform**) add terraform plan and about extensions - ([1ef2e5b](https://github.com/rsvalerio/ops/commit/1ef2e5beb85fb765cbafc1655e3c5ce86699fb21)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**test**) stabilize flaky tracing capture and timeout tests under parallel execution - ([3ff8c64](https://github.com/rsvalerio/ops/commit/3ff8c64296c3d20ca086511bf045fee855c8abee)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**deps**) update dependencies - ([cc7d433](https://github.com/rsvalerio/ops/commit/cc7d4330ec8762ac772654237cc26a60f72c1aa2)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**core**) improve terraform detection using file extensions - ([f1a1eac](https://github.com/rsvalerio/ops/commit/f1a1eacc12fcc00a72743be13b0c13f4bc0bd46b)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**config**) simplify ops toml configuration - ([ab69e89](https://github.com/rsvalerio/ops/commit/ab69e89a8b93dd5cc2c22db9627274862410813a)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.24.0](https://github.com/rsvalerio/ops/compare/9db5022b6558b73609a38ff0b6ed9ea5ae832ed6..v0.24.0) - 2026-05-01
#### 🚀 Features
- (**extensions-python/about**) add manifest caching layer - ([b399ec1](https://github.com/rsvalerio/ops/commit/b399ec1d2812358bf1fb2f592a20361f62a391bd)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**extensions-rust**) tighten tool probe matching and dependency introspection - ([90f3009](https://github.com/rsvalerio/ops/commit/90f30096e08d127af85d957d470bdc60adf17f8f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/cargo-toml**) improve inheritance resolution logic - ([dd06355](https://github.com/rsvalerio/ops/commit/dd063558fe0dd6d0cdfacb869b4875ac57fca7a6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/duckdb**) tighten SQL validation and ingest paths - ([083aa0a](https://github.com/rsvalerio/ops/commit/083aa0a4b28d5e7dbbd02e28cfb60651de31d1c6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/git**) harden git config and remote URL parsing - ([20a1b5b](https://github.com/rsvalerio/ops/commit/20a1b5b11a1e2b065496509813468bd8f8de7512)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/hooks**) add install validation and normalize dispatch patterns - ([08710df](https://github.com/rsvalerio/ops/commit/08710df1d2290fd194c4f6117fe0b9d020a81919)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- add comphreensive doc mapping commsnds and aliases - ([eff9d96](https://github.com/rsvalerio/ops/commit/eff9d960a05b804066965ac6992a33fa46f55405)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**cli/run_cmd**) add command execution tests - ([e793490](https://github.com/rsvalerio/ops/commit/e793490e534e710dcc9b8ea082c0fd6d879bdec7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/duckdb**) add counting fixture for in-memory ingest - ([1b5e015](https://github.com/rsvalerio/ops/commit/1b5e0155e7c8ca0b9bf76c4abb3ec9cddaf71d6a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/command**) add expand error handling tests - ([db2960b](https://github.com/rsvalerio/ops/commit/db2960b1bd0cd3aaaf059fb6fba8d5aab44c5c71)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**deps**) update dependencies - ([9d13fcb](https://github.com/rsvalerio/ops/commit/9d13fcbb8c1710d2a0ea015d91d762f3dd408d8b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**ops**) split test command to run ignored tests separately - ([c7a30f7](https://github.com/rsvalerio/ops/commit/c7a30f7a26e1cf9d840408449635177741954160)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) improve extension registry and help rendering - ([efb6c20](https://github.com/rsvalerio/ops/commit/efb6c20bbec266334437bd4ab1184375f1c800a3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) improve extension registry, command dispatch and help rendering - ([27b6d77](https://github.com/rsvalerio/ops/commit/27b6d7763c271c337166060eef3f442732791809)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/run_cmd**) improve command execution and planning - ([6a5f512](https://github.com/rsvalerio/ops/commit/6a5f512b60d10562662283c0c684d6813b502acd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) harden subprocess invocation and error propagation - ([35873e8](https://github.com/rsvalerio/ops/commit/35873e86ff50ee59b72412f7dd1c2235cafb2000)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) harden config loading, merging, expansion and subprocess handling - ([0d818cb](https://github.com/rsvalerio/ops/commit/0d818cbd9509f3ee35d90073bef54e70eaf66589)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) improve inheritance resolution and merge logic - ([15c4b4e](https://github.com/rsvalerio/ops/commit/15c4b4eb71de7dd907a1d654f706ebabe91b0052)) - [@rsvalerio](https://github.com/rsvalerio)
- (**display**) improve output formatting, error styling, and progress rendering - ([ed90dc1](https://github.com/rsvalerio/ops/commit/ed90dc1ef2aec689558b0d1701c1f73165486729)) - [@rsvalerio](https://github.com/rsvalerio)
- (**display**) improve output formatting and progress state tracking - ([0e935f9](https://github.com/rsvalerio/ops/commit/0e935f9bab0cb1a2cc8466f1940fe85d08bb1cc6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**display**) improve output formatting and progress state handling - ([c7b04a9](https://github.com/rsvalerio/ops/commit/c7b04a9f37d8c0ac46602585da4deae85cd7c87f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) improve extension interface and testing - ([7e5f61f](https://github.com/rsvalerio/ops/commit/7e5f61fc6151425a936c80bfe828fae6781d69f7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension/data**) add duplicate detection for data providers - ([6002b20](https://github.com/rsvalerio/ops/commit/6002b20012b34ad401e6a45c22d6c000095abf21)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) improve data handling, testing, and SQL generation - ([1451f5e](https://github.com/rsvalerio/ops/commit/1451f5e5083b5719cfda741dc6f1decdb2d6968c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/about**) improve unit and coverage data providers - ([50892dd](https://github.com/rsvalerio/ops/commit/50892dd01957125a9ed746396b3654cc68c574e3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/deps**) restructure parsing and formatting logic - ([a8c5bd0](https://github.com/rsvalerio/ops/commit/a8c5bd094825b94547ae57dbfa0cbe986253b905)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/test-coverage**) generalize coverage field reading and add record count test - ([9bbe665](https://github.com/rsvalerio/ops/commit/9bbe6654ee5513703f7ea49db68d204031b20d82)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/tools**) tighten install spawn and probe matching - ([1836ba8](https://github.com/rsvalerio/ops/commit/1836ba83bb33a3a0461b87554c99cffc41264206)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/about**) consolidate text utilities and card rendering - ([567b521](https://github.com/rsvalerio/ops/commit/567b521d57d9103502352b7c3e7f71c46b2fd001)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/duckdb**) harden SQL generation and concurrent ingest handling - ([4c9f6b2](https://github.com/rsvalerio/ops/commit/4c9f6b2e800c3a835d3f9f6da33313da720c4a56)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/hooks**) validate install and normalize dispatch patterns - ([df03f1a](https://github.com/rsvalerio/ops/commit/df03f1a8fa4e256deaa7933c7be80109185c8dcf)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/manifest**) improve parsing and normalization across languages - ([120546d](https://github.com/rsvalerio/ops/commit/120546d4f30b460a0cfc501eedf50729cbbca5d3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) restructure command execution, orchestration, and results handling - ([a8404c4](https://github.com/rsvalerio/ops/commit/a8404c4f64f86af7d0f8b12e49f2073079e3d591)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/command**) deduplicate UnknownCommand error type - ([fa5a79d](https://github.com/rsvalerio/ops/commit/fa5a79d9a6f814e0d4c10d6b158543e3ba1e853b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/command**) enhance execution, resolution, and event emission - ([d41f127](https://github.com/rsvalerio/ops/commit/d41f12728efc97bba16d4c42f081bc45a70aa4c8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) improve configuration exposure and step line rendering - ([2814dca](https://github.com/rsvalerio/ops/commit/2814dcacc3f13d934ab63c6c185cb9e64cc4fd73)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) update code review wave 64 task statuses - ([9a1f761](https://github.com/rsvalerio/ops/commit/9a1f7619359759558eccf3bc16c47e393886c32a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) update task descriptions and categorization - ([76f2adf](https://github.com/rsvalerio/ops/commit/76f2adf9b997d0134b7f55cd783fcb3c6404a828)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) add code review waves 60-68 and track ops-duckdb regression - ([168316f](https://github.com/rsvalerio/ops/commit/168316fcf2345ce0315a66989d6533177bb3233f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) refresh issue tracking and update task status - ([3d444c4](https://github.com/rsvalerio/ops/commit/3d444c419028a9361997f75a84ae2049932c8e98)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) refresh wave open tasks - ([924a7b3](https://github.com/rsvalerio/ops/commit/924a7b3586c62a18c8e2a4cac4956db69580164d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) refresh code-review plan waves 55-59 - ([5646b21](https://github.com/rsvalerio/ops/commit/5646b21bb3fd63ba7b9e9e2b44f114fbbdb6fc43)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) archive waves 11-19 completed items - ([9db5022](https://github.com/rsvalerio/ops/commit/9db5022b6558b73609a38ff0b6ed9ea5ae832ed6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**ci**) skip broken ops deps command for a moment - ([bc8d843](https://github.com/rsvalerio/ops/commit/bc8d843a92e82c88784dc9ecc021197741921db1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) update Cargo.lock - ([a23231b](https://github.com/rsvalerio/ops/commit/a23231be77c4035593bbb7aadce73bc5a766e4ac)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) bump Cargo.lock - ([ce6a35b](https://github.com/rsvalerio/ops/commit/ce6a35bd3248f639e6fdaa44571a2f2467f8f1a1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) bump Cargo.lock - ([2499438](https://github.com/rsvalerio/ops/commit/2499438a831a32fa9d7f88d2103366f202735e10)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) tighten deny.toml constraints - ([c8497be](https://github.com/rsvalerio/ops/commit/c8497be6ad362b4615b38e4bddf2dadcf5761c6e)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.23.1](https://github.com/rsvalerio/ops/compare/ce8b4bf69cc3f1809d77b15e358b93d1756eba2c..v0.23.1) - 2026-04-30
#### 🐛 Bug Fixes
- (**cli**) document parse_log_level write swallow and cover failing-writer fallback - ([513320c](https://github.com/rsvalerio/ops/commit/513320cc8b7df53785bbeb7926e5ec5fbb504181)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) log overlay key collisions and conf.d read errors - ([4c6ccc4](https://github.com/rsvalerio/ops/commit/4c6ccc4fad9963102ebecce51288c2845f066761)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/stack**) warn and return empty commands instead of panicking on bad default TOML - ([cdd5c18](https://github.com/rsvalerio/ops/commit/cdd5c1838ae0dc42c591bf7b52e731e245e633b7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/subprocess**) log pipe-drain truncations instead of silently dropping output - ([3ce0941](https://github.com/rsvalerio/ops/commit/3ce0941c0ce1472e55137ee4aa8ef3028be4dbd0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/text**) log non-NotFound IO errors in for_each_trimmed_line - ([64a5748](https://github.com/rsvalerio/ops/commit/64a57482c5b82953758cd33df12c594e474d855a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-go/about**) handle replace directives and harden module parsing - ([e1d4ae9](https://github.com/rsvalerio/ops/commit/e1d4ae9ab9493d596f943dd211f394f66ac8b1f8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-java/about**) tighten gradle/pom parsing edge cases - ([f48ac86](https://github.com/rsvalerio/ops/commit/f48ac865893b6f46d487ae98cceef6d7b1a1dbb5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-node/about**) harden package.json fields and unit detection - ([2b3d105](https://github.com/rsvalerio/ops/commit/2b3d1050c8525a13f4ff8c0c8309c175def1287c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-python/about**) harden pyproject parsing and unit detection - ([8184544](https://github.com/rsvalerio/ops/commit/8184544e14bfd2ad3bf90e6fc88bff9cd84c898d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/cargo-toml**) extend inheritance coverage and types - ([cc7e7e6](https://github.com/rsvalerio/ops/commit/cc7e7e624024e3bb71bfa18bc54a5403634e4e46)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/cargo-update**) tighten cargo-update output handling - ([d6c6dbc](https://github.com/rsvalerio/ops/commit/d6c6dbc094612324be1f54c8a41c29c93611c419)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/deps**) expand severity and diagnostic coverage - ([e9ba2e6](https://github.com/rsvalerio/ops/commit/e9ba2e63e5bf9ab64499140a387f1c83ef48042f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/metadata**) harden ingest invariants - ([dcd9ff4](https://github.com/rsvalerio/ops/commit/dcd9ff4c9c87446f68460dc114f97ad20d614f11)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/test-coverage**) tighten ingestor and trim redundant lib code - ([b756ffc](https://github.com/rsvalerio/ops/commit/b756ffcc1ca440a0fedbb2295cfffdf1a5212d65)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/tools**) tighten install spawn and probe matching - ([4747b64](https://github.com/rsvalerio/ops/commit/4747b64cba95e831ac2ee8f99fc5e61b1d23b25d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/about**) tighten manifest_io errors, coverage helpers and workspace logging - ([54b86b0](https://github.com/rsvalerio/ops/commit/54b86b04d1fd172d9e7fae0ff0eecd70749df735)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/duckdb**) tighten error mapping and SQL ingest paths - ([0096e12](https://github.com/rsvalerio/ops/commit/0096e12558a1c66004d68d20e3fe7392cef2bc1a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/git**) honour git config last-wins and harden remote URL parsing - ([fd0d09b](https://github.com/rsvalerio/ops/commit/fd0d09b4bfda4c6bb60afafd0e69f1ff6a4b2156)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/run-before-commit**) bound subprocess wait with wait-timeout and tracing - ([9352fdb](https://github.com/rsvalerio/ops/commit/9352fdb5f795abe6505bd2ae1c64c191543a9a32)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/tokei**) harden ingestor and trim redundant tests - ([4153844](https://github.com/rsvalerio/ops/commit/415384494ed9a4ec534a5179da13d606f5bdb29d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/command**) warn on alias collisions across command stores - ([fab8944](https://github.com/rsvalerio/ops/commit/fab8944e4748e06e46219f9da8232d49631e33eb)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**workspace**) promote wait-timeout and pull tracing into hook extensions - ([ce8b4bf](https://github.com/rsvalerio/ops/commit/ce8b4bf69cc3f1809d77b15e358b93d1756eba2c)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) refresh triage queue, add terraform plan doc, drop completed plan - ([19c6721](https://github.com/rsvalerio/ops/commit/19c6721ea3c3e6e16c4559928a46a73461ee2179)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.23.0](https://github.com/rsvalerio/ops/compare/1ded6161057eb63d72355ea530006b35ff6ba8e6..v0.23.0) - 2026-04-29
#### 🚀 Features
- (**cli**) preserve cwd bytes in dry-run and tighten registry/tools wiring - ([d83d555](https://github.com/rsvalerio/ops/commit/d83d555559c0b9d874db76b60c50d9b188649400)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) warn on unknown stacks and validate composite config refs - ([a1007b5](https://github.com/rsvalerio/ops/commit/a1007b543d0df9a57f522d4ed9d59afae599e271)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) detect intra-extension command collisions and apply non_exhaustive - ([9f68395](https://github.com/rsvalerio/ops/commit/9f68395889f341cbf55ce7efeb4f677350caefd1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/about**) extract manifest_io and tighten card/text rendering - ([20ee6ac](https://github.com/rsvalerio/ops/commit/20ee6ac34e65f89e6eba4c1eb85d93f48b231b66)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**extensions-go/about**) handle go.work use blocks and harden module parsing - ([498814d](https://github.com/rsvalerio/ops/commit/498814dd39fe590016ad439764fadad1540fe4e0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-java/about**) tighten gradle/pom parsing and align IO error handling - ([7a0cbd8](https://github.com/rsvalerio/ops/commit/7a0cbd8de1e26e417757cdd2c1931eae2ae4e9a5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-node/about**) trim metadata fields and detect packageManager edge cases - ([a9c63f0](https://github.com/rsvalerio/ops/commit/a9c63f0a005dcd439f8cd0787600c45a1de154bb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/about**) align identity/coverage provider with query_or_warn convention - ([d258203](https://github.com/rsvalerio/ops/commit/d25820374f59f35143cfede475e075985c3b4666)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/cargo-toml**) respect kebab-case default-features and harden inheritance - ([cf1edc0](https://github.com/rsvalerio/ops/commit/cf1edc0f8515cdc2ae6f62cda7dc900fd188ff98)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/deps**) handle missing severities and unknown deny diagnostic codes - ([a5692ce](https://github.com/rsvalerio/ops/commit/a5692ce2d16c2cb8c131b3f03d47c4b647b48c5a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/metadata**) enforce single-row invariant and surface load errors - ([bb3d4c0](https://github.com/rsvalerio/ops/commit/bb3d4c086c866bd3c627be1e4ffdfaeab87fcdd2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/test-coverage**) retain all coverage entries and validate section types - ([22f653b](https://github.com/rsvalerio/ops/commit/22f653be2ff4a47c082a9ab338d8139a73e7040b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/tools**) tighten rustup component matching and apply non_exhaustive - ([8d18dc4](https://github.com/rsvalerio/ops/commit/8d18dc45dcb74d9feef0c8e96b6e084489310d30)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/duckdb**) apply quoted_ident wrapper and apply non_exhaustive to public types - ([0da859c](https://github.com/rsvalerio/ops/commit/0da859cf6000051aab059223d37e74bc374ce767)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/git**) honour git config last-wins semantics for origin URL - ([aefc135](https://github.com/rsvalerio/ops/commit/aefc135953234e03b401aecea1410f83b780a425)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/hooks**) fsync new hooks and bound staged-file git wait - ([1406eb5](https://github.com/rsvalerio/ops/commit/1406eb50fbd75f074c0f95be15af29c30370b833)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/tokei**) preserve LanguageStat percentages and quote view identifiers - ([2b1667b](https://github.com/rsvalerio/ops/commit/2b1667be7ff376a0afc6c5be7af521c3adbf55ad)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**core**) memoize OPS_OUTPUT_BYTE_CAP and mark subprocess errors non_exhaustive - ([4f3c163](https://github.com/rsvalerio/ops/commit/4f3c163ff5b1c0b2d92cf8dd54140889171bf564)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-python/about**) avoid full tool.uv deserialization and align unit detection - ([cc80abb](https://github.com/rsvalerio/ops/commit/cc80abbd8f293fbe2919102c6237a31e2c042936)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**extensions-rust/cargo-update**) reuse format_error_tail and apply non_exhaustive - ([2fa2437](https://github.com/rsvalerio/ops/commit/2fa24378f35614ba325d390a05f762fc72e5baef)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) introduce abort module and bound parallel exec watchdog - ([7e51ce2](https://github.com/rsvalerio/ops/commit/7e51ce291fd2828ed9b56ea87e5ee7c051a140ad)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) split display into finalize/tap modules and bound stderr ring - ([b034536](https://github.com/rsvalerio/ops/commit/b034536b2493971ba4e8c8cedc1f2546dd404c00)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) standardize toml, proptest, serial_test to workspace refs - ([27307be](https://github.com/rsvalerio/ops/commit/27307be8a10dfddd5306cb5d4bfcf2837ad03219)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) close wave 38 and 41-50 code-review tasks - ([7917aa4](https://github.com/rsvalerio/ops/commit/7917aa49006bb59f76410d4e650402db3ad3774d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) add wave 41-50 plans and new triage tasks - ([1ded616](https://github.com/rsvalerio/ops/commit/1ded6161057eb63d72355ea530006b35ff6ba8e6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) refresh workspace dependencies and lockfile - ([1fe1155](https://github.com/rsvalerio/ops/commit/1fe1155a2d36e6ba4f84e3c51c60d75dbf0deb6c)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.22.0](https://github.com/rsvalerio/ops/compare/a5cd81aabc4a6cb2bdc9985dc4d32eb0430f99c2..v0.22.0) - 2026-04-29
#### 🚀 Features
- (**core**) enhance configuration loading and identity metrics - ([d95dc73](https://github.com/rsvalerio/ops/commit/d95dc731daf91c31251b89b5ee0533d439a3a7f2)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**cargo-toml**) prefer workspace root over member manifest and apply non_exhaustive - ([df6d02d](https://github.com/rsvalerio/ops/commit/df6d02d7f407dff804df29f6ef6b50e98f450073)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cargo-update**) validate exit status and optimize parse hot path - ([8f21533](https://github.com/rsvalerio/ops/commit/8f21533df77829f0759b39bca97edcbeebea7066)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) handle unknown diagnostic codes and optimize deny output parsing - ([2f73be8](https://github.com/rsvalerio/ops/commit/2f73be80889ec15c5544acc1cf9b9f1b1ffe2aa5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/about**) surface manifest load errors and drop false-positive metrics - ([97354b2](https://github.com/rsvalerio/ops/commit/97354b281fb0144564b809bf974f075402b8ecdb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tools**) validate component and toolchain args in install_rustup_component_with_timeout - ([eca7bf2](https://github.com/rsvalerio/ops/commit/eca7bf269344e3280ed8b863d1d1ec749e8601b6)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**metadata**) cache member id sets and surface ingestor cleanup errors - ([d37e0af](https://github.com/rsvalerio/ops/commit/d37e0afac64e747a98b59c31e03dd1bf91e137a8)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**cargo-toml**) split 1363-line tests into per-area submodules - ([ff9c921](https://github.com/rsvalerio/ops/commit/ff9c921cfb614c3a459a02f5d24284826175b73c)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) consolidate command handlers and error paths - ([e8d5569](https://github.com/rsvalerio/ops/commit/e8d55696496f4f4bea448c56075593fac9fe1c81)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/about**) improve unit enrichment and coverage - ([9518e97](https://github.com/rsvalerio/ops/commit/9518e974d49d2de58b47acf0ac12ecd88d4f84d6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/about**) consolidate providers into shared module - ([2ca6749](https://github.com/rsvalerio/ops/commit/2ca6749ea4fe1ac14e36fc7c1757db54258dc6b9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/duckdb**) add connection safety and query validation - ([488dbe7](https://github.com/rsvalerio/ops/commit/488dbe7b6f3381cb44cadef394697871eecbdf80)) - [@rsvalerio](https://github.com/rsvalerio)
- (**language-extensions**) improve metadata parsing and analysis - ([f927352](https://github.com/rsvalerio/ops/commit/f927352122b8c61d0ed21f5278d5f95b7ffdc9fb)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) add task definitions for code-review wave 40 - ([8de011b](https://github.com/rsvalerio/ops/commit/8de011b28127295d46585ee889b630d973ac9936)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 36 code-review tasks - ([9daac35](https://github.com/rsvalerio/ops/commit/9daac351533c260d472d4f33960da82469055f16)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 35 code-review tasks - ([a5cd81a](https://github.com/rsvalerio/ops/commit/a5cd81aabc4a6cb2bdc9985dc4d32eb0430f99c2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) update lock file - ([bca0ed4](https://github.com/rsvalerio/ops/commit/bca0ed4e333610bda542a21d73d3315eb6a5a95e)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.21.1](https://github.com/rsvalerio/ops/compare/b7ac48aac3b39aafd54156c48201302f6781feab..v0.21.1) - 2026-04-28
#### 🐛 Bug Fixes
- (**core/expand**) surface shellexpand errors and drop false-positive diamond cycle - ([b7ac48a](https://github.com/rsvalerio/ops/commit/b7ac48aac3b39aafd54156c48201302f6781feab)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/resolve**) reject absolute spec_cwd escape under Deny policy - ([b08b9ca](https://github.com/rsvalerio/ops/commit/b08b9cad82a1f9cfe71420787be6e98c0761f6d8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/results**) keep tap log handle across transient I/O errors and respect output cap - ([2b4dc8c](https://github.com/rsvalerio/ops/commit/2b4dc8c8d0861c019bf3d5cf3d8bac1d9f3f19c9)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**core/output**) cap CommandOutput stdout/stderr at a per-stream byte limit - ([53465bf](https://github.com/rsvalerio/ops/commit/53465bf05749bd26008dc0e5ffc487bbd5541e03)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/subprocess**) use wait-timeout instead of 100ms thread::sleep poll - ([9461f79](https://github.com/rsvalerio/ops/commit/9461f79ea88711f771a53e9957411160c9329181)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**runner/command**) split 1705-line tests.rs into per-area submodules - ([af29547](https://github.com/rsvalerio/ops/commit/af29547a92b8777e4fc8fc09b269ae5481cacacb)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**runner**) mark RunnerEvent and StepStatus/StepLine/ErrorDetail non_exhaustive - ([e61fef8](https://github.com/rsvalerio/ops/commit/e61fef8203e4d0864ab189d4f9017522c759858a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner/command**) Arc cwd/vars on spawn path, return Result from build_command, surface spawn errors and abort drain on fail_fast - ([721f4ca](https://github.com/rsvalerio/ops/commit/721f4ca6dedb030580eabeea21aaa0ae5adf4719)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme/step-line**) trim StepLineTheme trait surface to the methods callers use - ([e9e5fd1](https://github.com/rsvalerio/ops/commit/e9e5fd1f7cd260657699d7c9c2673dcd5dfc5062)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) close wave 39 code-review tasks - ([0c0c6c6](https://github.com/rsvalerio/ops/commit/0c0c6c64c05d45fd39943c3b97c17ff73db1bcb8)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.21.0](https://github.com/rsvalerio/ops/compare/f7efafb1496f4a4f2213476e949e824494ca2634..v0.21.0) - 2026-04-28
#### 🚀 Features
- (**core/config**) expose atomic_write for cross-crate reuse - ([45c2c35](https://github.com/rsvalerio/ops/commit/45c2c3523bb6e2c26c98b7289ddac9352b787ee3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) log .git pointer read errors at debug - ([af3c268](https://github.com/rsvalerio/ops/commit/af3c2686cae4d2937043ab2c1faafae4cbe516ca)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**cli**) surface registry, hook, theme, and OPS_LOG_LEVEL errors visibly - ([7611851](https://github.com/rsvalerio/ops/commit/7611851136d0483666f1bf2ac1f1741ff9eabfa7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/help**) stop is_toplevel_help scan at -- separator - ([8eef6c7](https://github.com/rsvalerio/ops/commit/8eef6c7eef7cd0f38f56a07e452c53bf76d9bd67)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) log atomic_write tmp cleanup failure on rename error - ([bff1aee](https://github.com/rsvalerio/ops/commit/bff1aee065191cefdf261c33cb8894462066b174)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/stack**) cap detect walk at MAX_DETECT_DEPTH - ([e317ba9](https://github.com/rsvalerio/ops/commit/e317ba90af05595656b3db99f9a792442924168b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git**) tighten origin-section header parser and owner/repo allowlist - ([2683610](https://github.com/rsvalerio/ops/commit/26836100e730738c27bb0defcdcb05dc96d78dde)) - [@rsvalerio](https://github.com/rsvalerio)
- (**go/about**) strip inline // comments from go.work use directives - ([36b6d8c](https://github.com/rsvalerio/ops/commit/36b6d8cb5b5d3750d145787f647ddf7bafb4f720)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java/gradle**) handle multi-arg includes, bare-method description, properties separators - ([f4444aa](https://github.com/rsvalerio/ops/commit/f4444aa004ffb1b2b6757099dda613431b11c30d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java/maven**) tolerate missing pom, skip container sections, separate name/artifactId - ([77205d6](https://github.com/rsvalerio/ops/commit/77205d68cb7bff53eb8ca3b399fb662178c1469a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node/about**) split workspace includes/excludes and log unit parse errors - ([66dba0b](https://github.com/rsvalerio/ops/commit/66dba0b6a1cad0c4514460dc611cd7bf918073bc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node/about**) detect bun in packageManager field - ([8574f19](https://github.com/rsvalerio/ops/commit/8574f19e4d8123c47819e16ef672e5d285a88a62)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node/about**) wrap email-only person in angle brackets - ([c4664b9](https://github.com/rsvalerio/ops/commit/c4664b936e6d67f0b7d6eec486e7ef27c2f14cfd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python/about**) log pyproject parse errors in units provider - ([4aca2b7](https://github.com/rsvalerio/ops/commit/4aca2b7fe4866cce2c17d4bfc7fd248196a05557)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**cli/registry**) drop per-iteration registry snapshot in register_extension_commands - ([0b87d84](https://github.com/rsvalerio/ops/commit/0b87d84e6db375107a1d5672dd23ad25d3d4905e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java/about**) cache field list via OnceLock - ([c6c1616](https://github.com/rsvalerio/ops/commit/c6c16169374d923722d421c1b9040cab3dae0087)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**cargo-toml**) document and test workspace=false InheritableField - ([f1c5054](https://github.com/rsvalerio/ops/commit/f1c50541ec2a9f8ebdd87dcc1f1ca96c29223c08)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**run-before-commit**) pin lossy git stderr decode behaviour - ([79dc5e4](https://github.com/rsvalerio/ops/commit/79dc5e444abbe6d3c79e9c478eb5b65606428e99)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) add shared identity-provider and workspace-glob helpers - ([29c9a7f](https://github.com/rsvalerio/ops/commit/29c9a7f52c80ecb9b15e5238ca75859b2559a4d3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) collapse run-before-{commit,push} cmd modules into pre_hook_cmd - ([2b739c7](https://github.com/rsvalerio/ops/commit/2b739c777aa87c03e539bab858391a7f49c43aa0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/init**) delegate force-overwrite to ops_core atomic_write - ([f69038c](https://github.com/rsvalerio/ops/commit/f69038c81c0de3d367a86ab1f289ea53dc0cb5a2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/new-command**) use shlex to honour quoted arguments - ([dbb885a](https://github.com/rsvalerio/ops/commit/dbb885aee27db8690fb7adf122f8e82dde07b704)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli/run**) split run_commands into raw/display helpers and warn on nested parallel - ([1cc2fb2](https://github.com/rsvalerio/ops/commit/1cc2fb2c94851252170c57db517d01164cf28fb8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) split tests.rs into per-area submodules - ([7da632f](https://github.com/rsvalerio/ops/commit/7da632f500c47fbd1a436bc87415fd9c1e42ea39)) - [@rsvalerio](https://github.com/rsvalerio)
- (**go/about**) share go.mod parser, parse block-form replace, strip // comments - ([c1e6f11](https://github.com/rsvalerio/ops/commit/c1e6f119103f09cda16bfbd694df12b19caa9cf5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**go/about**) extract go.work parser and adopt shared identity - ([1785138](https://github.com/rsvalerio/ops/commit/1785138962ee1b0c70f059538d0fe8c1a5ed4bd9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java/about**) split maven into module and adopt shared identity - ([0717e51](https://github.com/rsvalerio/ops/commit/0717e51948c6db4b3a878ec4d304dd5bf461ba18)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node/about**) rewrite git+ssh repo URLs to https and exhaust PackageJson literal - ([9f3cbd7](https://github.com/rsvalerio/ops/commit/9f3cbd744bdb93f7a952a737b0674276ba05eb4c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node/about**) split package_json/package_manager and adopt shared identity - ([c8a8898](https://github.com/rsvalerio/ops/commit/c8a8898ec8a9afd14be561fbddde2b2e376c441d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python/about**) case-insensitive pick_url and labelled license file - ([2de8ca5](https://github.com/rsvalerio/ops/commit/2de8ca5cf0fd49f90dfada5227039af5a2c5ff0f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python/about**) tighten pyproject parser and adopt shared identity - ([01dadf1](https://github.com/rsvalerio/ops/commit/01dadf10863b1151076590546d48502fb5892ab2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) dedupe sensitive-env warn/redact pattern sources - ([46ae3be](https://github.com/rsvalerio/ops/commit/46ae3be6202a1098895c9196816631ad462541c3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust/about**) tighten coverage/identity/query/units - ([e050b84](https://github.com/rsvalerio/ops/commit/e050b8404b4e7a74cc3a710fa2439182ade981fd)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) close wave 38 code-review tasks - ([49e1dcf](https://github.com/rsvalerio/ops/commit/49e1dcf0a9aae734a7fa22669c9096be40c3a0da)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 34 code-review tasks - ([3a43be1](https://github.com/rsvalerio/ops/commit/3a43be1e30ffae52eefd88d3c62a08547ea84d55)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 37 code-review tasks - ([46ad460](https://github.com/rsvalerio/ops/commit/46ad46055161499d0c431424164b0467b3d75eeb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 40 code-review tasks - ([3bb517e](https://github.com/rsvalerio/ops/commit/3bb517ed84ea087fe69cfc0a8b092dadfc260db7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) add wave 33-40 code-review task files - ([ee5b2cc](https://github.com/rsvalerio/ops/commit/ee5b2cc2008acbd1ac176d407ede09ca42732160)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 28 code-review tasks - ([c6353d2](https://github.com/rsvalerio/ops/commit/c6353d2694414d572f7fa01d20354e100bbb717c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) pin shared deps via workspace.dependencies - ([f7efafb](https://github.com/rsvalerio/ops/commit/f7efafb1496f4a4f2213476e949e824494ca2634)) - [@rsvalerio](https://github.com/rsvalerio)
- (**workspace**) centralise binary deps via workspace.dependencies - ([c210178](https://github.com/rsvalerio/ops/commit/c2101786facd5b25df3c9b74600b23b70e5b74a7)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.20.3](https://github.com/rsvalerio/ops/compare/161ac844d7cb1eba967ba08d869acb081c312ab4..v0.20.3) - 2026-04-27
#### 🐛 Bug Fixes
- (**cargo-update**) trace cargo-update lines that match no known verb shape - ([4762520](https://github.com/rsvalerio/ops/commit/47625206e81f5529e9cae9a14edb789de60b0c8b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) document dry-run redaction false-negatives and cover key-match path - ([f34afba](https://github.com/rsvalerio/ops/commit/f34afbaf85461afc6d1a9ab8c2af361094114a21)) - [@rsvalerio](https://github.com/rsvalerio)
- (**clippy**) move test module to file end and bind must_use LoadResult - ([baa33fd](https://github.com/rsvalerio/ops/commit/baa33fdba462d246849eae0f33f79afd4baf9207)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) shell-quote args in display_cmd and expanded_args_display - ([febb060](https://github.com/rsvalerio/ops/commit/febb06005436c9e8d626433ceb95c44101af18f1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/expand**) warn-log lookup errors instead of silently passing input through - ([23713da](https://github.com/rsvalerio/ops/commit/23713da3d285303b6e59c3d4edd8bc0f9f4de3e8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) surface cargo-deny configuration errors and table-drive ensure_tools - ([92710a5](https://github.com/rsvalerio/ops/commit/92710a5c7865bf6cf9efc187c93eece8376b3003)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) build run_deps context from user config instead of Config::default - ([2ff924e](https://github.com/rsvalerio/ops/commit/2ff924eaad38994835f948399a94c0755f4495c7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) make staged JSON cleanup best-effort symmetric to sidecar - ([ba1fe59](https://github.com/rsvalerio/ops/commit/ba1fe59b5716c3d6c44dc002d7e7ad92c398efd8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) hold lock across create+count and drop misleading top-row fallback - ([452193c](https://github.com/rsvalerio/ops/commit/452193c4b36786bb348c166cd3428eea5c17b613)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) tighten SQL identifier and path validation across query module - ([512ade0](https://github.com/rsvalerio/ops/commit/512ade0714a738944a94db37472256d13e128007)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) correct SharedError doc comment and tighten clone test - ([007890d](https://github.com/rsvalerio/ops/commit/007890da638f3183f0a22f222774e0fb9a295e29)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) mark ExtensionInfo/Context non_exhaustive and warn on duplicate registrations - ([a178951](https://github.com/rsvalerio/ops/commit/a17895142253c53e32c98aa513a279f6bd34d19c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git**) match remote section header case-insensitively - ([a02ef2b](https://github.com/rsvalerio/ops/commit/a02ef2ba930481befe7f8a53c8dcf84c2814f791)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) bound parent traversal in gitdir pointer - ([c8837a4](https://github.com/rsvalerio/ops/commit/c8837a4413e1c645ec92cf29c9bc9587186d73aa)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) require non-hex char in looks_like_aws_key - ([3f2a17b](https://github.com/rsvalerio/ops/commit/3f2a17b775adcf3ce4ec70f19f63355ec684a90c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust-about**) share query_project_languages and surface DuckDB query failures - ([3b20af3](https://github.com/rsvalerio/ops/commit/3b20af35b9303ed22b164c149f6e83113543c64a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tools**) handle multi-line rustup show active-toolchain output - ([ac3485d](https://github.com/rsvalerio/ops/commit/ac3485deae216ba9408e915ecd65eaf84b508405)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tools**) validate cargo install args and walk PATH in-process - ([1c6ce66](https://github.com/rsvalerio/ops/commit/1c6ce66eadecf32c394a4817a863ec7f1d47cf76)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚡ Performance
- (**duckdb**) bind per-crate paths via params_from_iter without intermediate Vec - ([85d734f](https://github.com/rsvalerio/ops/commit/85d734fd7bd64c9e3c6281be1fcccb8b53f029e7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**metadata**) hold cargo metadata as Arc<Value> and share from context - ([8b582a9](https://github.com/rsvalerio/ops/commit/8b582a9c48195cd75b7cbc4551552c19d5b856fa)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) cache TTY/NO_COLOR gate and widen strip_ansi coverage - ([161ac84](https://github.com/rsvalerio/ops/commit/161ac844d7cb1eba967ba08d869acb081c312ab4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tools**) replace subprocess polling loop with wait_timeout - ([0ddc661](https://github.com/rsvalerio/ops/commit/0ddc66190c5572cc39b226ca94f69a0bc2de52b6)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**metadata,test-coverage**) replace /nonexistent path with tempdir-derived missing path - ([7d02d80](https://github.com/rsvalerio/ops/commit/7d02d807a71e4d4abc45b9d63962c16d6d189964)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) derive is_tty from caller writer instead of stdout - ([062a785](https://github.com/rsvalerio/ops/commit/062a785485684fc5cb4f2b830efffeea19562fdf)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) collapse run options into struct and share load_config-or-default helper - ([8d84425](https://github.com/rsvalerio/ops/commit/8d84425d3520860eb77946192802c3ded06bf6da)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) remove unused Config::build_alias_map - ([842b6fe](https://github.com/rsvalerio/ops/commit/842b6fe1b648679fd850e6692dc0167a760d8a09)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core/config**) split mod.rs into commands and overlay submodules - ([4354ee8](https://github.com/rsvalerio/ops/commit/4354ee8cced1bfe0d7097ab72055317ee39e00f9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) centralize EnvGuard behind test-helpers feature - ([ecba616](https://github.com/rsvalerio/ops/commit/ecba616a005f256c21a8557c3f0ecdf60465124e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) split tests.rs into per-concern submodules - ([7a7aa9d](https://github.com/rsvalerio/ops/commit/7a7aa9d780e504cf7ffff39849807c91ecea3c65)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) require command_ids in BoxSnapshot and drop test-only fallbacks - ([60fc024](https://github.com/rsvalerio/ops/commit/60fc024948a1f6d0736996d5eb40f754007b4af1)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) close wave 32 code-review tasks - ([49b3425](https://github.com/rsvalerio/ops/commit/49b3425ad7d212980db013fad0fc7e4a66581225)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 31 code-review tasks - ([54277e4](https://github.com/rsvalerio/ops/commit/54277e4abc23474716aea85a1046449c5e3024c8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 30 code-review tasks - ([a113150](https://github.com/rsvalerio/ops/commit/a11315040496243e25902fadfd4a2ccdd1a11415)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) close wave 29 code-review tasks - ([4168ada](https://github.com/rsvalerio/ops/commit/4168ada6a73a6c7a4c61564d71b184eec4cf6619)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.20.2](https://github.com/rsvalerio/ops/compare/03426d7d2d8c08761c89d595feb10343d8791819..v0.20.2) - 2026-04-27
#### 🐛 Bug Fixes
- (**about-extensions**) collapse manifest exists-then-read into a single read - ([33fa6ab](https://github.com/rsvalerio/ops/commit/33fa6abb16cac68ac52d8f434d4411e7732665e3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) close TOCTOU and uniquify temp names in atomic_write and ops init - ([e985ca7](https://github.com/rsvalerio/ops/commit/e985ca7788543ff997a3af0e373f468fc1b727b5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) saturate numeric conversions to avoid overflow and panics - ([39ab1e2](https://github.com/rsvalerio/ops/commit/39ab1e2193c899f06391479ea9469b8832fe433d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) reject symlinked HEAD when probing git directory - ([1ee0804](https://github.com/rsvalerio/ops/commit/1ee0804cec496f80d36e9e0588b6151da32222ea)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner-command**) harden cancellation, panic handling, and event delivery - ([7a26274](https://github.com/rsvalerio/ops/commit/7a262743fddce92991c787e7b0cc99cc072a3e15)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust-extensions**) bound workspace ancestor walk and parse upgrade table by columns - ([cced391](https://github.com/rsvalerio/ops/commit/cced3918562f2c52b66af3ae886d1a2c0450d231)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) guard format_duration and share step prefix layout - ([c394f61](https://github.com/rsvalerio/ops/commit/c394f61ec1294a9250d14f3a91b615af7d75c8b0)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**agents**) slim root AGENTS.md and add core scope guide - ([03426d7](https://github.com/rsvalerio/ops/commit/03426d7d2d8c08761c89d595feb10343d8791819)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**runner-display**) extract ProgressState submodule - ([3eac3ab](https://github.com/rsvalerio/ops/commit/3eac3abfaf15761c8edb8cd11bddfe8553e56c7f)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) add wave 25-32 code-review tasks and plans - ([a0678aa](https://github.com/rsvalerio/ops/commit/a0678aaca61316649c25799f68c93097bfaf76d9)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.20.1](https://github.com/rsvalerio/ops/compare/dbcbdcc516639693714244161c81792841947278..v0.20.1) - 2026-04-25
#### 🐛 Bug Fixes
- (**rust-about-ext**) remove stale identity.rs after module split - ([750967a](https://github.com/rsvalerio/ops/commit/750967a13a01db22243f2998c3a5d32fcb4f8c9d)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about-extensions**) improve project detection across languages - ([9848029](https://github.com/rsvalerio/ops/commit/9848029cdd1ac430d54e9092c716bc77ec703396)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add subprocess module and improve utilities - ([5b08f3f](https://github.com/rsvalerio/ops/commit/5b08f3fdfc31447e66c0de52375bcd6d06aad60f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) improve git provider and utilities - ([c5cbb7f](https://github.com/rsvalerio/ops/commit/c5cbb7fff8e4eb5db4125c26faf338b86f6855a6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) modularize into focused files - ([8746f54](https://github.com/rsvalerio/ops/commit/8746f54a34322d8d1ccfd5e1d34a9ee1683d0e58)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner-command**) split exec and mod into focused modules - ([dbcbdcc](https://github.com/rsvalerio/ops/commit/dbcbdcc516639693714244161c81792841947278)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust-extensions**) improve tooling and modernize identity handling - ([d909ca0](https://github.com/rsvalerio/ops/commit/d909ca0221ef5d1f9b1003311639731d10d1f501)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) add wave 18-24 code-review tasks and plans - ([0b68c67](https://github.com/rsvalerio/ops/commit/0b68c67e8d0c303c9124183087e2ac4353810ec5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) update lock file - ([10a5707](https://github.com/rsvalerio/ops/commit/10a570734481261d5ef529a31e2142c86c6a989f)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.20.0](https://github.com/rsvalerio/ops/compare/ee0369f0f478025d48a31c1cf4acb05fd1bf082e..v0.20.0) - 2026-04-23
#### 🚀 Features
- (**core,theme**) add ui, config edit, and theme resolve modules - ([0034fd1](https://github.com/rsvalerio/ops/commit/0034fd14f57536cbf032ba462de8cb1986fe3aa4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**registry**) extract command registry for better code organization - ([e0b499d](https://github.com/rsvalerio/ops/commit/e0b499dfa944cda88d69d2d97f4c4e5ec14598c6)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- fix code example fence syntax and runnable examples - ([efe1e2b](https://github.com/rsvalerio/ops/commit/efe1e2bf5b45636052aa3aca5e2f03e8d1f305ad)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔄 CI/CD
- remove ops CLI dependency from workflow steps and add install job - ([1fb0ac4](https://github.com/rsvalerio/ops/commit/1fb0ac41781aae712da4a610f545f824c8e453b3)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) use new edit_ops_toml and ui modules for error handling - ([d4bd1bb](https://github.com/rsvalerio/ops/commit/d4bd1bbc8b7f9118dcb909aff68ef6979e98b554)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-ext**) improve card rendering and coverage reporting - ([fd3442a](https://github.com/rsvalerio/ops/commit/fd3442a39d273917be8e4ea98886e78cb824c648)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) improve command dispatch and error handling - ([8e849de](https://github.com/rsvalerio/ops/commit/8e849dec0af4d5b48d10124153156331d0f642dc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config**) improve merge logic and handle parse errors - ([d3c1127](https://github.com/rsvalerio/ops/commit/d3c11277ba86f06367da75f6da4ee6ab736b7b85)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config**) improve error handling and export edit_ops_toml - ([deab901](https://github.com/rsvalerio/ops/commit/deab90156ae05e34a2d812eb39d0ce758b597143)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) improve stack, table, and test utilities - ([a799ff0](https://github.com/rsvalerio/ops/commit/a799ff0944086cf79fe8e70da5db88262268e921)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add ui exports and improve output handling - ([27d3786](https://github.com/rsvalerio/ops/commit/27d3786fb968d9af3661d3392f4876f0df53e2cc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**crate**) unify error types and improve CLI extension handling - ([991c3c0](https://github.com/rsvalerio/ops/commit/991c3c01790734f27ba1c4867f3a8ac27d6b0a6a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**display**) improve output formatting and terminal handling - ([0eefe92](https://github.com/rsvalerio/ops/commit/0eefe92e58cff55dd54b66fc58139a9bfdd04b76)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) add SQL validation helpers, improve error handling, and refactor schema - ([621f128](https://github.com/rsvalerio/ops/commit/621f128cff9c17cb3a9dbb274e2f503f0b9bf08a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb-ext**) improve SQL ingest and query logic - ([e60e2c2](https://github.com/rsvalerio/ops/commit/e60e2c27c0bf204e140a56abebd8cc41faa1a92c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) update data registration and tests - ([9355d74](https://github.com/rsvalerio/ops/commit/9355d7412ec1c2be431b9651ce35a02725b3b41d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) improve error handling, critical section scoping, and dependencies - ([c5b53ad](https://github.com/rsvalerio/ops/commit/c5b53adc84b250e929336aa22e036f651104e749)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git-ext**) improve git provider - ([b23f85e](https://github.com/rsvalerio/ops/commit/b23f85e02590b0fbe428d0220025e804581402e7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**go-about-ext**) improve Go project analysis - ([1c6fb78](https://github.com/rsvalerio/ops/commit/1c6fb780290f7bc8dbfcd79a7ca700c3bd53c9be)) - [@rsvalerio](https://github.com/rsvalerio)
- (**help**) improve category handling and command filtering logic - ([7b228c0](https://github.com/rsvalerio/ops/commit/7b228c083c2c64fc87f00ee53669cdff8c0b5c88)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) use new edit_ops_toml infrastructure - ([dd3674f](https://github.com/rsvalerio/ops/commit/dd3674f0edc5452b44bf333590de271697dedaba)) - [@rsvalerio](https://github.com/rsvalerio)
- (**identity**) improve project identity card rendering - ([67cf8a4](https://github.com/rsvalerio/ops/commit/67cf8a4a8bc8ada36e92de8ca8bd03ef0dd0f9ee)) - [@rsvalerio](https://github.com/rsvalerio)
- (**java-about-ext**) improve Java build tooling detection - ([031fb87](https://github.com/rsvalerio/ops/commit/031fb87377066147bba65d667e75231b427a8799)) - [@rsvalerio](https://github.com/rsvalerio)
- (**new-command**) use new edit_ops_toml infrastructure - ([611e482](https://github.com/rsvalerio/ops/commit/611e482e06a14c5e5889cb98c691846e0eaa43d0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node-about-ext**) improve Node project analysis - ([7845c0f](https://github.com/rsvalerio/ops/commit/7845c0f1ff5d0767a4b4ff1f67e1138d478fa4c6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python-about-ext**) improve Python project analysis - ([26bf9f2](https://github.com/rsvalerio/ops/commit/26bf9f2ab3596fa00208bae3cb1c267a4fcdbc98)) - [@rsvalerio](https://github.com/rsvalerio)
- (**run-cmd**) improve error reporting and dry-run display - ([392e5c1](https://github.com/rsvalerio/ops/commit/392e5c1bc15d0ade416185b220fb7051de33825a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) improve command execution and error handling - ([9b817b4](https://github.com/rsvalerio/ops/commit/9b817b4baf7dcff92ca1b592f16d39d4c1dc4c9a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust-about-ext**) improve Rust crate detection - ([5ce6bc4](https://github.com/rsvalerio/ops/commit/5ce6bc432b389b9e248f7b85e2e95bffdab6d2f6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust-exts**) improve Rust tooling integration - ([ff5b98c](https://github.com/rsvalerio/ops/commit/ff5b98cdf588313334b174da4be1c5c9442b634f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) improve configurable theme and step line rendering - ([e8ced65](https://github.com/rsvalerio/ops/commit/e8ced65dc644d14009694241fff27b935d1292c8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) use new edit_ops_toml and theme resolve infrastructure - ([621425e](https://github.com/rsvalerio/ops/commit/621425ed6f56e98d86b3e49f99ceb6f7b77cadc9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**tokei-ext**) improve lines-of-code analysis - ([7257635](https://github.com/rsvalerio/ops/commit/72576350e42ebae49c50bf9457259740f8a33ca9)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) update task status for wave 5 tasks and mark waves 15-17 in progress - ([66858c6](https://github.com/rsvalerio/ops/commit/66858c68778cdf6561e87f0f1ec020cf55b383e8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) mark code-review wave 14 findings as complete - ([ee0369f](https://github.com/rsvalerio/ops/commit/ee0369f0f478025d48a31c1cf4acb05fd1bf082e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config**) add --ignored flag to default test command - ([5c0138d](https://github.com/rsvalerio/ops/commit/5c0138d5deac630ad47088bdd36cc920a2378a49)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) update lock file - ([bfaf82a](https://github.com/rsvalerio/ops/commit/bfaf82ae1f483daebffe2cef12ca74f2c6bb65ca)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) add dependencies for new infrastructure modules - ([8ec3efa](https://github.com/rsvalerio/ops/commit/8ec3efabefa5a9a4c5278280ec8108a90ec94613)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.19.0](https://github.com/rsvalerio/ops/compare/d1a7251c8e89b62edb4de87f4cc507061220bd32..v0.19.0) - 2026-04-23
#### 🚀 Features
- (**cli**) warn when --raw forces sequential execution - ([9e31914](https://github.com/rsvalerio/ops/commit/9e319143c56c25d389883a9cab3394c93456b076)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) add --raw flag for direct terminal passthrough - ([344f20e](https://github.com/rsvalerio/ops/commit/344f20e8715888e2c5e898545fb2347f74860f8d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) enhance find_git_dir with worktree support and symlink protection - ([e3f4c40](https://github.com/rsvalerio/ops/commit/e3f4c40e685e3369fd48dfcd328c75bdbee73f6a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hook-common**) add HookConfig constructor and impl_hook_wrappers macro - ([854b465](https://github.com/rsvalerio/ops/commit/854b4656f7e28933b4c3516629833978bdc072f3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) add raw mode execution with inherited stdio - ([d1a7251](https://github.com/rsvalerio/ops/commit/d1a7251c8e89b62edb4de87f4cc507061220bd32)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) add error block frame color styling - ([b6817e5](https://github.com/rsvalerio/ops/commit/b6817e5ee20e510e511f468027900e0f15644bdd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) honor NO_COLOR environment variable - ([5bef6be](https://github.com/rsvalerio/ops/commit/5bef6bed0ed448f2fb1773589a429100880e5401)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**git**) add scheme allowlist, host validation, and credential stripping for remote URLs - ([5eebeaa](https://github.com/rsvalerio/ops/commit/5eebeaacb344f9ec1d56fe6f3d39c937a105a607)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**readme**) add stack parity matrix and clean TODOs - ([3b1141c](https://github.com/rsvalerio/ops/commit/3b1141cfdaa2883c1e066469e0e78fb184642588)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) introduce HookDispatch struct and deduplicate skip/prompt/dispatch logic - ([cfbc93b](https://github.com/rsvalerio/ops/commit/cfbc93b8d0758e0760e373d1b40ca54bf23dff87)) - [@rsvalerio](https://github.com/rsvalerio)
- (**hooks**) use impl_hook_wrappers macro in run-before-commit and run-before-push - ([8d1ba7f](https://github.com/rsvalerio/ops/commit/8d1ba7f71d6cda1d54aa62bd5e9f3a87f15b65cf)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) extract timeout and resolution helpers - ([f123296](https://github.com/rsvalerio/ops/commit/f1232968f820096ac4ddbb74f8353dc5edd960df)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) add code-review findings and wave plans for waves 13-17 - ([0abd9cc](https://github.com/rsvalerio/ops/commit/0abd9cc6291460a08faaa429f0a7743e7e0fde0f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config**) add todo command to ops.toml - ([6705279](https://github.com/rsvalerio/ops/commit/67052797b37d0bd165b5bff0cc585907fa883180)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) update crates.io dependencies - ([85ae129](https://github.com/rsvalerio/ops/commit/85ae129dc5d24928b54f46b22c886636e70fceb8)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.18.1](https://github.com/rsvalerio/ops/compare/f44e91fd623b0a6d98fbe3c8f0a2e3fc666cd3ac..v0.18.1) - 2026-04-20
#### 🐛 Bug Fixes
- (**cargo-toml**) remove redundant map_err DataProviderError conversions - ([8be5970](https://github.com/rsvalerio/ops/commit/8be59706a2cd9a7decb02a4d595f808e59553b59)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) validate and quote SQL identifier in ingestor COUNT query - ([b8357bb](https://github.com/rsvalerio/ops/commit/b8357bb2419e262a11fc4004aaaa4ce89e1dc0a3)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**runner**) replace wall-clock timing with rendezvous in parallel execution test - ([fc40815](https://github.com/rsvalerio/ops/commit/fc40815fe6d958137baa48594d22af90f2fee4fb)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) extract typed structs and consolidate field resolution in identity provider - ([0dd893f](https://github.com/rsvalerio/ops/commit/0dd893fd098dff5d99cc06957cfc82b55574d0f9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) extract dispatch, help, and run_cmd into dedicated modules - ([a6a1fbd](https://github.com/rsvalerio/ops/commit/a6a1fbd24aa9bf1ba4fe343c4ec749f44bdf6d18)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) extract project_identity into submodules - ([00419a7](https://github.com/rsvalerio/ops/commit/00419a7e565c50e49d729bfbf148090824973905)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) extract query.rs into focused SQL submodules - ([c64bffa](https://github.com/rsvalerio/ops/commit/c64bffa4671f0ef20af873e53ab3bc035ae0e392)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) extract display into submodules - ([fc638d8](https://github.com/rsvalerio/ops/commit/fc638d8d59210bbdce18148ffaf8a0704a0b3b3d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**rust-tools**) extract lib.rs into install, probe, tests, and timeout modules - ([5302507](https://github.com/rsvalerio/ops/commit/5302507eeccf7a83ec7a4425d2633b158107098b)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) add code-review findings and wave plans for waves 11–12 - ([5614c7b](https://github.com/rsvalerio/ops/commit/5614c7b6d987b3cf535a92dfd27abb7f1168675d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) mark code review waves 5–10 as Done - ([f44e91f](https://github.com/rsvalerio/ops/commit/f44e91fd623b0a6d98fbe3c8f0a2e3fc666cd3ac)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.18.0](https://github.com/rsvalerio/ops/compare/f1fc9ef05e345f92189305b206c8dc652305ac07..v0.18.0) - 2026-04-19
#### 🚀 Features
- (**about**) enrich language stats with per-language LOC, files, and percentages - ([c36a458](https://github.com/rsvalerio/ops/commit/c36a458c839f29c1ae35dacf696c202e1a6be6fb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) use ops-git for repository URL across stack extensions - ([dd31eea](https://github.com/rsvalerio/ops/commit/dd31eeaccf2591360c7b633b3e493f8495777dfa)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) redesign project identity with stack-aware fields and per-language emoji - ([ca94855](https://github.com/rsvalerio/ops/commit/ca948552b01d1d7cd58805e50405fc60bc2da71f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) alias lint to clippy in Rust and vet in Go defaults - ([0c59719](https://github.com/rsvalerio/ops/commit/0c597198fd59dca4781fdf0dbdd8bc05f9cb510c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add 7-command baseline to all stack default configs - ([01787a1](https://github.com/rsvalerio/ops/commit/01787a136c6c0a077fa685f0aed2a7f02331acb6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**git**) add ops-git extension for repository detection - ([f1fc9ef](https://github.com/rsvalerio/ops/commit/f1fc9ef05e345f92189305b206c8dc652305ac07)) - [@rsvalerio](https://github.com/rsvalerio)
- (**node**) add about-node extension for project identity and units - ([78705b1](https://github.com/rsvalerio/ops/commit/78705b1e830e4799a3f0dc49e49e5d6e5e28d0da)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python**) add about-python extension for project identity - ([1510e44](https://github.com/rsvalerio/ops/commit/1510e44c5343f536605af8bd5977c127d1d76e35)) - [@rsvalerio](https://github.com/rsvalerio)
- (**python**) restructure default commands around uv workflow - ([35b7835](https://github.com/rsvalerio/ops/commit/35b783587cd5813d42c9e63c3958c3bd3130a443)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) add live bottom border and command-IDs header to boxed layout - ([b003c6c](https://github.com/rsvalerio/ops/commit/b003c6cd6ceeff3db4667ddb51ce9c0a519b447c)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**about**) map subprojects key to package emoji - ([524a79c](https://github.com/rsvalerio/ops/commit/524a79c01851dbb6ec8b6ebbc8dfc48c6c403ee7)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) remove spurious leading blank line in card render - ([3c7e4b9](https://github.com/rsvalerio/ops/commit/3c7e4b98dbd1d472ec2be26c55ea027ce6ffd79b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) drive progress glyphs from step status for parallel plans - ([cc00bcb](https://github.com/rsvalerio/ops/commit/cc00bcbbb2c9e4d723d088f59bef3655371486fc)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) add right rail to running-row template with fixed-width elapsed - ([bb1883d](https://github.com/rsvalerio/ops/commit/bb1883d65c13a3b65aef08b71b87fb67114e49af)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) prevent step-line border overshoot when duration is present - ([e11580a](https://github.com/rsvalerio/ops/commit/e11580a7f13da590395255480b2d0a4b2f4619ea)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- update command reference and stack defaults guide - ([e562e4a](https://github.com/rsvalerio/ops/commit/e562e4a033db8e60f2bab961a6bd8ed279a5346e)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) register ops-git and categorize hook commands as Setup - ([ccd3b00](https://github.com/rsvalerio/ops/commit/ccd3b002f78d2b3e80382335b93bcd5bd5c83fd1)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) mark CROSS-STACK-1 and CROSS-STACK-2 tasks as Done - ([e846a21](https://github.com/rsvalerio/ops/commit/e846a21995ea94507797e6f0e8425c484ff630bd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) add cross-stack task definitions - ([278c585](https://github.com/rsvalerio/ops/commit/278c5851450f6d96cdb1cbbc62e717f31ac7e04b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config**) switch default theme to studio - ([54d42d1](https://github.com/rsvalerio/ops/commit/54d42d1a467007384bd1c2d279595b596809051d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) bump duckdb to 1.10502 - ([cd1108b](https://github.com/rsvalerio/ops/commit/cd1108b58150318cb419ac08bc002657235d06be)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) bump ops-git to 0.17.0 - ([a6c648f](https://github.com/rsvalerio/ops/commit/a6c648f51dc0ef09f59594a54097a19913d9839a)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.17.0](https://github.com/rsvalerio/ops/compare/8d5782c941dad96f203b0983ffa44f484eab2709..v0.17.0) - 2026-04-18
#### 🚀 Features
- (**about-go**) add project_units data provider for Go modules - ([36195aa](https://github.com/rsvalerio/ops/commit/36195aaf79d7dd6be9d101ae54d50e433e3a193d)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add ProjectUnit, CoverageStats, and deps data types for about subpages - ([6d0e913](https://github.com/rsvalerio/ops/commit/6d0e9134a8c61be7f86e36f98c85af63a9331f4a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) add boxed layout, ANSI styling, and flux/studio themes - ([f2084d4](https://github.com/rsvalerio/ops/commit/f2084d473881cb6e09cd243e493a2a4b0199945c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) add boxed layout, ANSI styling, and flux/studio themes - ([d889b1d](https://github.com/rsvalerio/ops/commit/d889b1dc8a6047b8d416c1c9ea9f0528129e5c89)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) extract cards, coverage, deps, and units into generic extensions/about - ([b67bc57](https://github.com/rsvalerio/ops/commit/b67bc576251254081dcd4710cd6cb1729c97323f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) simplify about subpage routing to use generic ops_about calls - ([69ea84c](https://github.com/rsvalerio/ops/commit/69ea84cbabe03d276e4cae832532ca5f1d0be330)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli,runner**) extract hook logic and simplify command execution - ([16852fa](https://github.com/rsvalerio/ops/commit/16852fae5413fcc64215fc625dee728906188f69)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) improve config loading, stack, and project identity - ([8cf757c](https://github.com/rsvalerio/ops/commit/8cf757c4c9cbffb20388e0d29e9e477ca3365d8f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) simplify query, ingest, validation, and error handling - ([9370cae](https://github.com/rsvalerio/ops/commit/9370cae7c6859fcd284d44e199c23766fc29de82)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) simplify about, run-before-commit, run-before-push, and tokei - ([076a4b4](https://github.com/rsvalerio/ops/commit/076a4b477c8987d8cac38c3baaa26cb5f49658fb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust**) extract cargo-toml inheritance and simplify about, tools, test-coverage - ([4f1c09d](https://github.com/rsvalerio/ops/commit/4f1c09d871d03409a8914c264dbd4db9e61265ef)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) update task descriptions and add wave 5-10 plans - ([8d5782c](https://github.com/rsvalerio/ops/commit/8d5782c941dad96f203b0983ffa44f484eab2709)) - [@rsvalerio](https://github.com/rsvalerio)
- (**ci**) update CI workflow and lockfile - ([c3c1fdc](https://github.com/rsvalerio/ops/commit/c3c1fdc73080bb05dcdc54e533a5bd6251af360e)) - [@rsvalerio](https://github.com/rsvalerio)
- update Cargo.lock - ([0b92247](https://github.com/rsvalerio/ops/commit/0b92247b75cb09517ece86251c8feb6dff976d7c)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.16.0](https://github.com/rsvalerio/ops/compare/f53d781aad8795eabb8ed1e33bdf6f83a3c3b2b9..v0.16.0) - 2026-04-17
#### 🚀 Features
- (**cli,runner**) add --tap flag to capture raw command output to file - ([5a783c0](https://github.com/rsvalerio/ops/commit/5a783c04aa3db11a731f7b1c8aeab07b2a82a37e)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**core**) remove unused show_output config field - ([25e2b71](https://github.com/rsvalerio/ops/commit/25e2b7190be4c78f8e15c18961d53befc88f3250)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) archive completed analysis tasks (0023-0060) - ([f53d781](https://github.com/rsvalerio/ops/commit/f53d781aad8795eabb8ed1e33bdf6f83a3c3b2b9)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config**) consolidate code-review commands in .ops.toml - ([4028bff](https://github.com/rsvalerio/ops/commit/4028bff0a3f68a7c81c99221f1d62fe7c62e04d0)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.15.0](https://github.com/rsvalerio/ops/compare/e7bc4fee0f411fd102cbdfa0fff048e2467016c3..v0.15.0) - 2026-04-15
#### 🚀 Features
- (**config**) add show_output option for inline command output - ([f8b9a68](https://github.com/rsvalerio/ops/commit/f8b9a687acaf76d72514e40ae146688dff2015e2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add variable expansion for command specs - ([e7bc4fe](https://github.com/rsvalerio/ops/commit/e7bc4fee0f411fd102cbdfa0fff048e2467016c3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) wire variable expansion and show_output into execution pipeline - ([c98b13f](https://github.com/rsvalerio/ops/commit/c98b13ff067e4dd74d97ffd8e3c18c4d1d4bb34d)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**about**) add tests for about and deps extensions - ([4eb8199](https://github.com/rsvalerio/ops/commit/4eb8199709f504ac54d9d5f882be186c3128b43f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) add unit tests for extension, hook, and init commands - ([830d6b1](https://github.com/rsvalerio/ops/commit/830d6b16420947104f0b36f4a3222c7161059217)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli,about-rust**) replace dashboard command with about subpages - ([7cca3a7](https://github.com/rsvalerio/ops/commit/7cca3a77013ea2202fa7e82038a77611feacfce6)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.14.0](https://github.com/rsvalerio/ops/compare/505910c2401bf7aa092869406ac4e6cc6243e551..v0.14.0) - 2026-04-15
#### 🚀 Features
- (**about**) add coverage refresh hints and improve crate metadata resolution - ([34a3154](https://github.com/rsvalerio/ops/commit/34a31545411f1ea6418a544265522ef2baf05d0e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) add configurable rendering and step line theme - ([e49c55d](https://github.com/rsvalerio/ops/commit/e49c55d6bb149352c111e9ee6775f374a2b261d0)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- (**extension**) update test suite - ([ba56951](https://github.com/rsvalerio/ops/commit/ba56951617b34b381546895481abb4b20cf1d731)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**gitignore**) update ignored files - ([c345da8](https://github.com/rsvalerio/ops/commit/c345da8ebacaa88af00a080df190dea9e0b8a375)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) simplify main.rs and extension commands - ([a234b65](https://github.com/rsvalerio/ops/commit/a234b654178d5b680c8413b8d29c9e7aa86b4d8c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add config merge and update identity/stack - ([3ef2806](https://github.com/rsvalerio/ops/commit/3ef28065a1c9cd765408cba35c3d314adf4a7bdb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) update about extensions - ([9e661bc](https://github.com/rsvalerio/ops/commit/9e661bc51b0d36d393b2bfba29fbcccb43431ae1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-java/about**) simplify about extension - ([d1138ae](https://github.com/rsvalerio/ops/commit/d1138ae564d3dbd676a75db82aeb0d1a02fef9cf)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/about**) decompose lib.rs into focused modules - ([1ed1c72](https://github.com/rsvalerio/ops/commit/1ed1c72ce4363baaebdba57459f0f22680c041e1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/deps**) extract metadata ingestor module - ([70f7d1d](https://github.com/rsvalerio/ops/commit/70f7d1db9dfc91391a6629a439a38d4ae9c8f9e3)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust/test-coverage**) extract ingestor module - ([7044598](https://github.com/rsvalerio/ops/commit/7044598c5883973a8b9637846077f9f3e5b076f1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/duckdb**) remove sql.rs and simplify lib.rs - ([0501d04](https://github.com/rsvalerio/ops/commit/0501d0472b36f628590cb9825d8a4ecd61fe1dfb)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions/hooks**) update run-before-commit and run-before-push - ([ceaea45](https://github.com/rsvalerio/ops/commit/ceaea45c7f2725db5bb467a5de5180b8cf59f2a4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) update command execution and display - ([efc188e](https://github.com/rsvalerio/ops/commit/efc188e8b3f39eec9fd9540e2c6e55140c99f0e4)) - [@rsvalerio](https://github.com/rsvalerio)
- (**theme**) simplify lib.rs - ([7007bd8](https://github.com/rsvalerio/ops/commit/7007bd85f75999d7af4c34d0cf7d3d291443b90f)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**backlog**) archive completed tasks - ([4544d71](https://github.com/rsvalerio/ops/commit/4544d7144fafd195802aa6d128528cd41c4fd779)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) add remaining analysis and refactor tasks - ([2c9cea0](https://github.com/rsvalerio/ops/commit/2c9cea0caf4042d1bf7d0407dd9f250c35f0817b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) add architecture and duplication analysis tasks - ([0a8047f](https://github.com/rsvalerio/ops/commit/0a8047f04cb7ff0325965414f68d3a905c7f90c0)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) add quality and security analysis tasks - ([0cbdd96](https://github.com/rsvalerio/ops/commit/0cbdd968b6b5b545dd2c89423be208408da19e60)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) update task descriptions - ([285ca62](https://github.com/rsvalerio/ops/commit/285ca62aa92793fa56febd69fe1f21299056e19a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**backlog**) move 22 completed tasks to archive - ([505910c](https://github.com/rsvalerio/ops/commit/505910c2401bf7aa092869406ac4e6cc6243e551)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cargo-toml**) fix extension cargo parsing - ([456dcfc](https://github.com/rsvalerio/ops/commit/456dcfc2a0b38a96acbc7ed8012990a1ce580f57)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) update dependencies and security config - ([3e49ce3](https://github.com/rsvalerio/ops/commit/3e49ce3eb7ed2dea431dc9e727029ceaeb1b711c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**test-coverage**) remove unused code - ([1c495c2](https://github.com/rsvalerio/ops/commit/1c495c2978c129004f9309eccbd5cc14f5bc019a)) - [@rsvalerio](https://github.com/rsvalerio)
- remove commit script - ([31c1164](https://github.com/rsvalerio/ops/commit/31c1164aa0ebf86695200e3bd6a3d10038319906)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.13.0](https://github.com/rsvalerio/ops/compare/3527e3e86ca59b23e3f3b616b178442aa4030e0f..v0.13.0) - 2026-04-12
#### 🚀 Features
- (**about**) add field visibility filtering and new metadata fields - ([b9bf9be](https://github.com/rsvalerio/ops/commit/b9bf9be5af05a5b1079a7cb3826df307e402f06c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about**) add setup command for interactive field configuration - ([3527e3e](https://github.com/rsvalerio/ops/commit/3527e3e86ca59b23e3f3b616b178442aa4030e0f)) - [@rsvalerio](https://github.com/rsvalerio)
- (**duckdb**) add queries for dependency count, coverage, and language detection - ([6a2ccd1](https://github.com/rsvalerio/ops/commit/6a2ccd11c8bbdf8994a6a9fb304c3a978e2a901a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions**) implement about metadata for language stacks - ([b42cb38](https://github.com/rsvalerio/ops/commit/b42cb38faeb0dd7ef7f65cfb6de7a5f841fb5a39)) - [@rsvalerio](https://github.com/rsvalerio)
- (**run**) support parallel execution and fail_fast settings from composite commands - ([06a74a4](https://github.com/rsvalerio/ops/commit/06a74a47a408b691e8db574456eb8f086b126d7f)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- update dependencies and test utilities - ([c0f55ef](https://github.com/rsvalerio/ops/commit/c0f55efbf174f31c7a80810f6366dc4667043c91)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.12.0](https://github.com/rsvalerio/ops/compare/c60e0fec6b4cf0099ea9bef877ff514742d59492..v0.12.0) - 2026-04-11
#### 🚀 Features
- (**commands**) add category field and grouped help output - ([ae14e44](https://github.com/rsvalerio/ops/commit/ae14e44b6f4abe495b1bead79a588500e4d04d09)) - [@rsvalerio](https://github.com/rsvalerio)
- (**commands**) add alias support for exec and composite commands - ([c60e0fe](https://github.com/rsvalerio/ops/commit/c60e0fec6b4cf0099ea9bef877ff514742d59492)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**hooks**) split pre-commit into run-before-commit and run-before-push - ([95319e9](https://github.com/rsvalerio/ops/commit/95319e94be79c441e69a5a9a06b22317dfe5a961)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**ci**) add verbose flag to test command - ([34d2674](https://github.com/rsvalerio/ops/commit/34d26745aaa98f54cc0e3339c79c8543f5ac2d3b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**ci**) disable sccache wrapper in bump workflow - ([46844a4](https://github.com/rsvalerio/ops/commit/46844a4c17a8a8c1ff31066b5866b34d05cce68e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**ci**) remove --verbose from ops - ([025178f](https://github.com/rsvalerio/ops/commit/025178f8595bc81187104f86fb435605b320c4ab)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) ensure all commands have a default category - ([4125a2a](https://github.com/rsvalerio/ops/commit/4125a2a25441193cd3fe734a17d1ff5044c12bcd)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- update Cargo and build configuration - ([78a04ae](https://github.com/rsvalerio/ops/commit/78a04ae1c4461ca364f13201542f9a3a43edef08)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔄 CI/CD
- add sccache-action to build jobs for improved caching - ([52e6fbb](https://github.com/rsvalerio/ops/commit/52e6fbb52ff986f581d3e1a0b30ef2d871b523bb)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) extract build_runner and print_exec_spec, fix CommandId types - ([8144752](https://github.com/rsvalerio/ops/commit/814475278c05747708e3e134125da24a123d8f3b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) update commands for extension refactoring - ([f260734](https://github.com/rsvalerio/ops/commit/f2607341c40d214d88bf4b8fcc3ef82203fbf382)) - [@rsvalerio](https://github.com/rsvalerio)
- (**config**) extract loader and merge logic - ([f76224f](https://github.com/rsvalerio/ops/commit/f76224f9c80bd03afe42ee0f7b677538ef6cfd3c)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) update project identity and extension content - ([ea1baca](https://github.com/rsvalerio/ops/commit/ea1bacaa4a8a5676fd14300367d51c24e24251fd)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) extract modules and simplify extension system - ([66d0d5f](https://github.com/rsvalerio/ops/commit/66d0d5fb620c10e3ed7a71b4b6845e25853cd605)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) refactor command execution and display - ([eab6582](https://github.com/rsvalerio/ops/commit/eab6582adac19bd898ebfd34b81fb422d11d954a)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- remove not used files - ([28638e7](https://github.com/rsvalerio/ops/commit/28638e7033b2571d6f404277927a7257177b6c7c)) - [@rsvalerio](https://github.com/rsvalerio)
- add project management and backlog tooling - ([8a5be94](https://github.com/rsvalerio/ops/commit/8a5be943a1abad92aa876c730f3b38c52dbc5cdf)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🎨 Styling
- (**cli**) format print_categorized_help function - ([29c078b](https://github.com/rsvalerio/ops/commit/29c078be036a7f8933f4a1350d71e3286cf2d5e5)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.11.0](https://github.com/rsvalerio/ops/compare/65a421f791f9f910d01d481d49b10b505125564b..v0.11.0) - 2026-04-07
#### 🚀 Features
- (**about**) add generic multi-stack about extension - ([6ad34c3](https://github.com/rsvalerio/ops/commit/6ad34c3a2010905cdde4aa21a6a8fd6e1d8530f8)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-go**) add Go project identity extension stub - ([4be68c0](https://github.com/rsvalerio/ops/commit/4be68c02305ee47ea5d66aec59a6e59e85b03660)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-java**) add Java project identity extension stub - ([6658cd2](https://github.com/rsvalerio/ops/commit/6658cd26639749141dcab39d9e56d87ab90cd668)) - [@rsvalerio](https://github.com/rsvalerio)
- (**cli**) make about command stack-agnostic and group extension list by stack - ([8367e62](https://github.com/rsvalerio/ops/commit/8367e62e4fa6ab54eaf9418788e5203be27a6831)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add ProjectIdentity and AboutCard types - ([65a421f](https://github.com/rsvalerio/ops/commit/65a421f791f9f910d01d481d49b10b505125564b)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extension**) add optional stack field to ExtensionInfo and impl_extension! macro - ([8d2e1f8](https://github.com/rsvalerio/ops/commit/8d2e1f86849c738e1f0ac4e2d1b29482ca7d95d6)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-go**) parse local replace directives in go.mod - ([f5ec719](https://github.com/rsvalerio/ops/commit/f5ec7193282a406045b744de72e3b95cd9a03324)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**about**) document AboutCard layout, ProjectIdentity schema, and data flow - ([c7087e0](https://github.com/rsvalerio/ops/commit/c7087e0c5372b653b4badc0709af100236698042)) - [@rsvalerio](https://github.com/rsvalerio)
- update instructions and examples for verify/qa split - ([46406ff](https://github.com/rsvalerio/ops/commit/46406ff7636f1c95cae582bfe9e35c74757eee37)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**about**) simplify card layout to inline title · badge - ([8a15df5](https://github.com/rsvalerio/ops/commit/8a15df534ab9b23bdf8377dc848c2541c8f540da)) - [@rsvalerio](https://github.com/rsvalerio)
- (**about-rust**) rename ops-about to ops-about-rust and extract RustIdentityProvider - ([7dec318](https://github.com/rsvalerio/ops/commit/7dec318548b386389a3077303ee7c69e22ab1a3a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**commands**) split verify into static checks and qa into quality assurance - ([91396d7](https://github.com/rsvalerio/ops/commit/91396d7caa0f1447e198b78df1c9558381d1c8f2)) - [@rsvalerio](https://github.com/rsvalerio)
- (**extensions-rust**) tag all Rust extensions with Stack::Rust - ([4af433f](https://github.com/rsvalerio/ops/commit/4af433fd72f9ee5c3a7d19c988b3a2949680722d)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.10.0](https://github.com/rsvalerio/ops/compare/ef7bd75040bc49127d6091c90646bd3936f2a989..v0.10.0) - 2026-04-05
#### 🚀 Features
- (**cli**) add stack-java-maven and stack-java-gradle feature flags - ([b0bff7c](https://github.com/rsvalerio/ops/commit/b0bff7c6147811367996011182af85473a66dd49)) - [@rsvalerio](https://github.com/rsvalerio)
- (**core**) add java-maven and java-gradle stack detection and defaults - ([ef7bd75](https://github.com/rsvalerio/ops/commit/ef7bd75040bc49127d6091c90646bd3936f2a989)) - [@rsvalerio](https://github.com/rsvalerio)
- (**pre-commit**) interactive command selection during hook install - ([05a2c2c](https://github.com/rsvalerio/ops/commit/05a2c2ca0a20a353caf1775486f81eef850fd34e)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**cli**) show dynamic commands in top-level help output - ([a866521](https://github.com/rsvalerio/ops/commit/a8665215e014d318e2574408b7521357ab63ca39)) - [@rsvalerio](https://github.com/rsvalerio)
- (**pre-commit**) use temporary directory for command gathering tests - ([0910d5d](https://github.com/rsvalerio/ops/commit/0910d5d8cde52d93ca08460d680211ad981ea460)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.9.0](https://github.com/rsvalerio/ops/compare/455fe9f260512355eed3967cfe14a0e06b65dad1..v0.9.0) - 2026-04-05
#### 🚀 Features
- (**cli**) add verbose flag to show full stderr output on failure - ([ecf91df](https://github.com/rsvalerio/ops/commit/ecf91dff4f01192a07e6a31e40ced83ffd595278)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**deps**) update duplicate crates summary format and adjust test assertions - ([12a1a1e](https://github.com/rsvalerio/ops/commit/12a1a1eb669d5b06bf2bb739f21631fbfcffb56e)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**dist**) use unix-archive instead of deprecated compression key - ([89a047d](https://github.com/rsvalerio/ops/commit/89a047d35f705f24b459cc90c6a58ddb34b59c0c)) - [@rsvalerio](https://github.com/rsvalerio)
- make `ops des` command run before commiting - ([71c3ee6](https://github.com/rsvalerio/ops/commit/71c3ee64dedeb5b4c671167acc21ba77c8a6d9a2)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔄 CI/CD
- update release repo to rsvalerio/ops and re-enable deps job - ([455fe9f](https://github.com/rsvalerio/ops/commit/455fe9f260512355eed3967cfe14a0e06b65dad1)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.8.1](https://github.com/rsvalerio/ops/compare/8729f7b855bd617f9dc0d6473d7b4058a544514a..v0.8.1) - 2026-03-26
#### 🐛 Bug Fixes
- (**deps**) exclude duplicate crate bans from actionable issue checks - ([f4175bc](https://github.com/rsvalerio/ops/commit/f4175bc88a2dcadcfcc17961a9e0b8703b44b88e)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) update unicode-segmentation to 1.13.2 (1.13.1 was yanked) - ([d6997c0](https://github.com/rsvalerio/ops/commit/d6997c00ba51ac4436e5602ce3c83083af08fa13)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) fail with non-zero exit code when dependency issues are found - ([84137c3](https://github.com/rsvalerio/ops/commit/84137c3e36806f3bc62549b9c7f6231002856336)) - [@rsvalerio](https://github.com/rsvalerio)
- (**runner**) add terminal echo guard to suppress input echo during parallel execution - ([8729f7b](https://github.com/rsvalerio/ops/commit/8729f7b855bd617f9dc0d6473d7b4058a544514a)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- (**readme**) add backlog section with planned improvements - ([089d9d6](https://github.com/rsvalerio/ops/commit/089d9d683535aab9596618834e042f21b8afbcd5)) - [@rsvalerio](https://github.com/rsvalerio)
- (**releasing**) update workflow for PR-based flow with branch protection - ([6310c7e](https://github.com/rsvalerio/ops/commit/6310c7e059df5172ce0a4377709d1e3d93565734)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- (**dist**) drop powershell installer, switch to gzip compression - ([ab40024](https://github.com/rsvalerio/ops/commit/ab400244f570954deff0747a2e57a0170df7cc51)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔄 CI/CD
- remove deps command from ci workflow - ([03bef71](https://github.com/rsvalerio/ops/commit/03bef71d68b5a7b57d844f7aff7d27120a2b08b0)) - [@rsvalerio](https://github.com/rsvalerio)
- remove deps command from pre-commit - ([1c73c7d](https://github.com/rsvalerio/ops/commit/1c73c7d385516e137652764d9e2743e165513f99)) - [@rsvalerio](https://github.com/rsvalerio)
- replace direct cargo calls with ops, split into 6 parallel jobs - ([db0d81b](https://github.com/rsvalerio/ops/commit/db0d81b86b0a87a67610600f3cfc1f1414b35f34)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- remove `[commands.deps]` from .ops.toml (default cmd now) - ([7a407fe](https://github.com/rsvalerio/ops/commit/7a407fe976c990cfc95b4b863ece0d94a2377f00)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.8.0](https://github.com/rsvalerio/ops/compare/744cf131812604d82bff48e0370fbc20e9db81df..v0.8.0) - 2026-03-25
#### 🚀 Features
- (**config**) make verify command run in parallel by default - ([85c04e5](https://github.com/rsvalerio/ops/commit/85c04e5dc22907eb1f44895209d994f7decf4b35)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**deps**) replace audit command with deps, show only totals for duplicates - ([6af4aa1](https://github.com/rsvalerio/ops/commit/6af4aa12bcdc2d80c9043168195990b5f14a23e1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**pre-commit**) skip checks when no staged files, add --all flag - ([4a5c910](https://github.com/rsvalerio/ops/commit/4a5c91041b9ca5a31aad1b35dd4568802f08125c)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**cargo-update**) add missing license field - ([43df7f7](https://github.com/rsvalerio/ops/commit/43df7f7e7ac3aef7c2724cb93d491330b8817dc9)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**deps**) align section header spacing in deps report - ([64e9f6c](https://github.com/rsvalerio/ops/commit/64e9f6c14c508a9775dbec3201fa51f9988fb7b9)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**deps**) wire up deps extension to CLI - ([744cf13](https://github.com/rsvalerio/ops/commit/744cf131812604d82bff48e0370fbc20e9db81df)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
#### 🚜 Refactoring
- (**extensions**) auto-register extensions via linkme distributed slice - ([b52d5d2](https://github.com/rsvalerio/ops/commit/b52d5d2ea5613d12f042b9a320a85fa73e20a67c)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**deps**) bump duckdb to 1.10501 and update lockfile - ([bb368b7](https://github.com/rsvalerio/ops/commit/bb368b72288a98ab01cc71d86a2fe01ea3355523)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)

- - -

## [v0.7.0](https://github.com/rsvalerio/ops/compare/6d4a731944d2e07155efd84b4bb663f436be2b75..v0.7.0) - 2026-03-24
#### 🚀 Features
- (**cli**) wire pre-commit install subcommand and SKIP_OPS_VERIFY handling - ([239d25a](https://github.com/rsvalerio/ops/commit/239d25a468a55d86d79fae39edec1ebb490606f7)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**core**) add yellow and bold ANSI style helpers - ([362372a](https://github.com/rsvalerio/ops/commit/362372a05efd3c6889fe50253a3c23efa87e78cc)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**deps**) add deps extension and cargo-deny configuration - ([3c1c12e](https://github.com/rsvalerio/ops/commit/3c1c12e493b24d2f218285bdd50ef99c9da7e2bb)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**extension**) add pre-commit hook management extension - ([0ca60e4](https://github.com/rsvalerio/ops/commit/0ca60e4a2b38b3d7070180166db42589b6d60edf)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**ops**) add audit and pre-commit commands to .ops.toml - ([6d4a731](https://github.com/rsvalerio/ops/commit/6d4a731944d2e07155efd84b4bb663f436be2b75)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**ci**) add --all-features flag to build, test, and check commands - ([dfe9a5a](https://github.com/rsvalerio/ops/commit/dfe9a5ac0b3e9023e694a9d8664d43958b6c6c5a)) - [@rsvalerio](https://github.com/rsvalerio)
- (**ci**) enable --all-features in check and clippy jobs - ([3518bdc](https://github.com/rsvalerio/ops/commit/3518bdc03d1845d979e003e95217ffcde1097ded)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**dashboard**) remove leftover skip_updates references - ([1c01ba1](https://github.com/rsvalerio/ops/commit/1c01ba15d2df6605c0145fa4275ab849bb6acda2)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**workflows**) update token var name on bump workflow - ([1654b97](https://github.com/rsvalerio/ops/commit/1654b972995b3df147a093af14d89ad1b71f6cf1)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**dashboard**) remove updates section and related functionality - ([4d64913](https://github.com/rsvalerio/ops/commit/4d6491306e458a3fc7185b0fb305084a7b852262)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- (**dependencies**) remove unused ops-core dependency from pre-commit extension - ([6c484c7](https://github.com/rsvalerio/ops/commit/6c484c784bf1df696d3af28a3ae421da68167d14)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) remove unused ops-cargo-update dependency from Cargo.lock - ([a287a0d](https://github.com/rsvalerio/ops/commit/a287a0dde586b7d0bc1a3e8741ed6f4d5d36aee1)) - [@rsvalerio](https://github.com/rsvalerio)
- (**deps**) bump duckdb from 1.4 to 1.10500 - ([42e5984](https://github.com/rsvalerio/ops/commit/42e5984003318bd025ac85fdd39ffbd0c5626907)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- centralize license = Apache-2.0 via workspace inheritance - ([763b2c0](https://github.com/rsvalerio/ops/commit/763b2c045cbef189052d64ded6757f5859bd1bef)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)

- - -

## [v0.6.0](https://github.com/rsvalerio/ops/compare/c3843f36dee8e7929871521ed73e74ef114e9da2..v0.6.0) - 2026-03-21
#### 🚀 Features
- (**cli**) enhance command help display with dynamic commands - ([303027e](https://github.com/rsvalerio/ops/commit/303027e3f24bd7de53a2b3756def8846978eed2b)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**core**) add help text to stack default commands - ([5fe7505](https://github.com/rsvalerio/ops/commit/5fe75058c5f8e6f4fe7d1db28eeda10c0de5fa4e)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**core**) add help field to CommandSpec for user-facing descriptions - ([d38c7c4](https://github.com/rsvalerio/ops/commit/d38c7c41bd9381ca27f98df767d3d6f0056406cf)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- add left padding configuration to theme for improved output formatting - ([c3843f3](https://github.com/rsvalerio/ops/commit/c3843f36dee8e7929871521ed73e74ef114e9da2)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- (**cli**) slim main.rs to thin orchestrator - ([0421586](https://github.com/rsvalerio/ops/commit/0421586c680d47068a2b8688c9ac59db90455bf8)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**cli**) move CwdGuard to test_utils module - ([71ef4b5](https://github.com/rsvalerio/ops/commit/71ef4b5e89634fee6f00b3d6e121e33ce6479598)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
- (**cli**) extract args, init_cmd, and run_cmd modules - ([1f22863](https://github.com/rsvalerio/ops/commit/1f228633cb0183cefaf64b91ba317e97f3cb24bb)) - [@rsvalerio](https://github.com/rsvalerio), Claude Opus 4.6 (1M context)
#### ⚙️ Miscellaneous
- (**dependencies**) update package versions and remove unused dependencies - ([f6098dc](https://github.com/rsvalerio/ops/commit/f6098dc8feb34ed1e261c7d7e47af41e239fcdc1)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.5.0](https://github.com/rsvalerio/ops/compare/1c8dbf2a49e162e7b52bf2b118d9b98d3c6cc20d..v0.5.0) - 2026-03-19
#### 🚀 Features
- enhance progress display with footer and summary updates - ([42e2070](https://github.com/rsvalerio/ops/commit/42e20705ec76c3a93e406c5eb27436c57af99717)) - [@rsvalerio](https://github.com/rsvalerio)
#### 📚 Documentation
- clarify summary separator and footer details in progress display section - ([90b7955](https://github.com/rsvalerio/ops/commit/90b7955d1a1f00fdcc2e7517822d10ba6f0d8afe)) - [@rsvalerio](https://github.com/rsvalerio)
- update human docs to reflect actual codebase - ([4a86871](https://github.com/rsvalerio/ops/commit/4a868712a11f9759aa643fe81d7b3e3c43cb52e4)) - [@rsvalerio](https://github.com/rsvalerio)
- update AI agents docs to reflect actual codebase - ([1c8dbf2](https://github.com/rsvalerio/ops/commit/1c8dbf2a49e162e7b52bf2b118d9b98d3c6cc20d)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- include ops config file with extra install command - ([9b6a38c](https://github.com/rsvalerio/ops/commit/9b6a38cb01bfdcdfad7c29205eca30a2e8bd0bdd)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- remove additional commands help polution on help page - ([752eb7f](https://github.com/rsvalerio/ops/commit/752eb7fd2378c047dc3a47d19319a5d81056c1e3)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🎨 Styling
- switch ops theme from compact to classic - ([caca6f3](https://github.com/rsvalerio/ops/commit/caca6f3e4e494b9050e6bb25d089d11459bee342)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.4.0](https://github.com/rsvalerio/ops/compare/63a83923fca0391722cd3252a08d49c5af4c1691..v0.4.0) - 2026-03-17
#### 📚 Documentation
- Add Apache License 2.0 - ([63a8392](https://github.com/rsvalerio/ops/commit/63a83923fca0391722cd3252a08d49c5af4c1691)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚀 Features
- output horizontal size is now calculate and default to 90% - ([e4f2aeb](https://github.com/rsvalerio/ops/commit/e4f2aebc5bf682764e2af7530e1bb8cc22b6a530)) - [@rsvalerio](https://github.com/rsvalerio)
- add new-command, that auto parses a cmd line and auto gen config - ([8914f44](https://github.com/rsvalerio/ops/commit/8914f44f25de561eea8c495974c7fbb6bd9e5fb5)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.3.0](https://github.com/rsvalerio/ops/compare/f90818cba3f5a930bc366bc0c19ff6037c019524..v0.3.0) - 2026-03-16
#### 🚀 Features
- ![BREAKING](https://img.shields.io/badge/BREAKING-red) rename from cargo-ops to ops across the project - ([f90818c](https://github.com/rsvalerio/ops/commit/f90818cba3f5a930bc366bc0c19ff6037c019524)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.2.0](https://github.com/rsvalerio/ops/compare/886d1d668d2f1555b786dc6744c6d6836091b47e..v0.2.0) - 2026-03-15
#### 🚀 Features
- enhance `init` command to support section flags for output, themes, and commands - ([886d1d6](https://github.com/rsvalerio/ops/commit/886d1d668d2f1555b786dc6744c6d6836091b47e)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

## [v0.1.0](https://github.com/rsvalerio/ops/compare/d14be6022c65611539891e7d228d142eda49e6eb..v0.1.0) - 2026-03-15
#### 📚 Documentation
- update docs for humans and agents - ([a3abefb](https://github.com/rsvalerio/ops/commit/a3abefb0420debe11f1537c71e3418a8454a4d36)) - [@rsvalerio](https://github.com/rsvalerio)
- add full historical changelog - ([2a53663](https://github.com/rsvalerio/ops/commit/2a53663d4aa4e4453787aab6e44cc9ce3aa8ca84)) - [@rsvalerio](https://github.com/rsvalerio)
- update license in README to Apache-2.0 - ([e2ddc17](https://github.com/rsvalerio/ops/commit/e2ddc17f8298f5694a02c45b3ec281431442f799)) - [@rsvalerio](https://github.com/rsvalerio)
- update documentation for workspace structure - ([e9e1760](https://github.com/rsvalerio/ops/commit/e9e1760dfcc0d03a2b02f9c8c5ee22c1a0fc12d1)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚀 Features
- using cocogitto instead of release-plz as release management tool - ([de24ac2](https://github.com/rsvalerio/ops/commit/de24ac20d8beeae0f0ea88d6faffc9345bb1a786)) - [@rsvalerio](https://github.com/rsvalerio)
- add dashboard command whowing  comprehensive project info - ([a1a0990](https://github.com/rsvalerio/ops/commit/a1a09908124b90ed3af769c0ec16d0923cb6d43e)) - [@rsvalerio](https://github.com/rsvalerio)
- increase maximum description lines for crate cards from 2 to 3 - ([79db87e](https://github.com/rsvalerio/ops/commit/79db87ed456dd5be1bfd71b95be0fd04b0aa8bc2)) - [@rsvalerio](https://github.com/rsvalerio)
- add duckdb and tokei extensions - ([1abae1e](https://github.com/rsvalerio/ops/commit/1abae1e5077442c8ba9fb0b3e3a8cc1c0f109a99)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🐛 Bug Fixes
- (**ci**) install cocogitto alongside cargo-edit on workflow - ([217c9b3](https://github.com/rsvalerio/ops/commit/217c9b3a38c9991148a9c958aa331b173452227e)) - [@rsvalerio](https://github.com/rsvalerio)
- remove changelog to let cog re-generatr it - ([ee4b23c](https://github.com/rsvalerio/ops/commit/ee4b23c308d8ddc6fdebaec35e6e4e63ead3db10)) - [@rsvalerio](https://github.com/rsvalerio)
- fix debug log arg pos - ([51f8776](https://github.com/rsvalerio/ops/commit/51f87767c033ac9e445ea1e1dd8c826f3e097a25)) - [@rsvalerio](https://github.com/rsvalerio)
- enable debug log on cog bump - ([9462e2d](https://github.com/rsvalerio/ops/commit/9462e2de1d9f8aaf3e3379f748306b66a0779e51)) - [@rsvalerio](https://github.com/rsvalerio)
- cog release, by using cog action directly - ([23d6d3c](https://github.com/rsvalerio/ops/commit/23d6d3cbe22916751e16df660a8e0eee22a2bf1b)) - [@rsvalerio](https://github.com/rsvalerio)
- clippy findings - ([7117b84](https://github.com/rsvalerio/ops/commit/7117b84250ea1e124c5608edf0c71325002a1694)) - [@rsvalerio](https://github.com/rsvalerio)
- use branch name instead of commit SHA in release-plz workflow - ([43234ec](https://github.com/rsvalerio/ops/commit/43234ec1aba8033ff4329ad3b29c26a0dd453c54)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🧪 Testing
- update integration tests for workspace - ([2bcf937](https://github.com/rsvalerio/ops/commit/2bcf93781e6c67c410bba5a0e3863ceebfa9bca6)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔧 Build System
- setup cargo workspace structure - ([1113e3d](https://github.com/rsvalerio/ops/commit/1113e3d9dd4198660439e4087b25bf5e0c7ec5f5)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🔄 CI/CD
- use actions-rust-lang/setup-rust-toolchain - ([f6967a7](https://github.com/rsvalerio/ops/commit/f6967a71e6b40c0c81bfda782dafae43be8d1523)) - [@rsvalerio](https://github.com/rsvalerio)
- switch to rust-lang/setup-rust action - ([afaf494](https://github.com/rsvalerio/ops/commit/afaf494c5e1f6200ad8dfee3193bc8edc863bd1f)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🚜 Refactoring
- move rust extensions to extensions-rust/ - ([62af0fb](https://github.com/rsvalerio/ops/commit/62af0fb679ef0c2c70b919a730aa7446d78d0794)) - [@rsvalerio](https://github.com/rsvalerio)
- extract cli binary crate - ([152351f](https://github.com/rsvalerio/ops/commit/152351f534cddd649c59d6456f3765bee4267724)) - [@rsvalerio](https://github.com/rsvalerio)
- extract extension trait crate - ([4bede57](https://github.com/rsvalerio/ops/commit/4bede5785789d9e7ef7e8ebf724d63d41110c2a6)) - [@rsvalerio](https://github.com/rsvalerio)
- extract theme crate - ([f8aae47](https://github.com/rsvalerio/ops/commit/f8aae47dd5d0d144de16fe2cb08f31caa46ec529)) - [@rsvalerio](https://github.com/rsvalerio)
- extract runner crate for command execution - ([8e42d72](https://github.com/rsvalerio/ops/commit/8e42d7263d7d6690def371b087bfc9afa5863433)) - [@rsvalerio](https://github.com/rsvalerio)
- extract core library crate - ([a19bc64](https://github.com/rsvalerio/ops/commit/a19bc64315654f0e2b130e567b63057896676074)) - [@rsvalerio](https://github.com/rsvalerio)
- introduce ansi_style! macro - ([884de2f](https://github.com/rsvalerio/ops/commit/884de2f6c060a6f8a7e47c4cb3fb13d0c1d27078)) - [@rsvalerio](https://github.com/rsvalerio)
- use toml_edit for theme config - ([c898e98](https://github.com/rsvalerio/ops/commit/c898e98094697083d8d0c4888b909df6c3d9e1ce)) - [@rsvalerio](https://github.com/rsvalerio)
- use strum derives for Stack enum - ([5fb8555](https://github.com/rsvalerio/ops/commit/5fb855564774e6cd853659ff5cea07c2fbf6ef50)) - [@rsvalerio](https://github.com/rsvalerio)
- restructure as cargo workspace with extensions - ([d14be60](https://github.com/rsvalerio/ops/commit/d14be6022c65611539891e7d228d142eda49e6eb)) - [@rsvalerio](https://github.com/rsvalerio)
#### ⚙️ Miscellaneous
- disable GitHub releases on release-plz, let cargo dist do - ([1f0d368](https://github.com/rsvalerio/ops/commit/1f0d368657a5ef53afe642ec509760d0e0e042ad)) - [@rsvalerio](https://github.com/rsvalerio)
- set publish flag to false in all Cargo.toml files for core, extensions, and rust extensions - ([b81a280](https://github.com/rsvalerio/ops/commit/b81a280f90f2a6760aa0421d4f020f053b7cc208)) - [@rsvalerio](https://github.com/rsvalerio)
- refine release workflow conditions and concurrency settings - ([1ede1f1](https://github.com/rsvalerio/ops/commit/1ede1f1adb88be247489f98c1d4d592ac54d484e)) - [@rsvalerio](https://github.com/rsvalerio)
- specify single changelog path in release configuration - ([eca518f](https://github.com/rsvalerio/ops/commit/eca518f53a92d3232518ac50682d2d71281070bf)) - [@rsvalerio](https://github.com/rsvalerio)
- modify release workflow to trigger on successful CI completion - ([d613bce](https://github.com/rsvalerio/ops/commit/d613bce75e5147904ae13181129d1afd58fd7588)) - [@rsvalerio](https://github.com/rsvalerio)
- add protection for breaking changes in changelog configuration - ([0ac1aa9](https://github.com/rsvalerio/ops/commit/0ac1aa90faa2354e470005a4c8690e3ba2512c7e)) - [@rsvalerio](https://github.com/rsvalerio)
- update quinn-proto to version 0.11.14 and add audit configuration - ([832d66d](https://github.com/rsvalerio/ops/commit/832d66da9bf218795523ba352eb1cabecec2eca8)) - [@rsvalerio](https://github.com/rsvalerio)
- update changelog commit preprocessors to clean up commit messages - ([142b8b1](https://github.com/rsvalerio/ops/commit/142b8b19b42ccbae4bf4ce98531fe8b396aad64c)) - [@rsvalerio](https://github.com/rsvalerio)
- remove unused Cargo configuration file - ([c0934aa](https://github.com/rsvalerio/ops/commit/c0934aa85b1fe67c5d0f5e0ef39327622644210e)) - [@rsvalerio](https://github.com/rsvalerio)
- add configuration for automated releases and update documentation - ([ad3f945](https://github.com/rsvalerio/ops/commit/ad3f945ec1490df2b677555e34a162efc6b1eda3)) - [@rsvalerio](https://github.com/rsvalerio)
- update gitignore, remove tool-versions - ([d2df16a](https://github.com/rsvalerio/ops/commit/d2df16a3ce86b5fa81101803bc85b46674433204)) - [@rsvalerio](https://github.com/rsvalerio)
#### 🎨 Styling
- format code - ([fa7a174](https://github.com/rsvalerio/ops/commit/fa7a17499fddd4d3d64b390c26b57e44af625b39)) - [@rsvalerio](https://github.com/rsvalerio)

- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).
