# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

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