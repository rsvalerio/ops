# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

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