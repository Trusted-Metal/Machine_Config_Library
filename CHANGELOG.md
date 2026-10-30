## [1.0.0](https://github.com/Trusted-Metal/Machine_Config_Library/compare/v0.1.0...v1.0.0) (2026-10-30)

### ⚠ BREAKING CHANGES

* ClearBox is no longer part of the typed MachineConfig model in any language.
optical_trains[].optional_components and the ClearBox type are removed. Read and write
extensions[ClearBox] with mcf-ext-clearbox (Node, Rust) instead.
See docs/migrations/clearbox_to_extension.md.

### Features

* Add C++ tarball and version reporting; added Rust cargo-package CI check ([d6d4d82](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d6d4d8232ed3e0dece9b5ea29095d71e61ff7439))
* Add Node tgz and Python wheel build support, Go implementation ([bf60969](https://github.com/Trusted-Metal/Machine_Config_Library/commit/bf60969b7aeb39533b9d024b863c9dd85ef2d2c7))
* added capabilities to lib for Rust, updated testing ([3079e77](https://github.com/Trusted-Metal/Machine_Config_Library/commit/3079e773312840267bd6fcbdefa3dcb2a7695b9f))
* Added hash writing/hash checking to MCF to all languages, updated testing/docs ([dc14d57](https://github.com/Trusted-Metal/Machine_Config_Library/commit/dc14d571bdbafba3e0680db19431c979ec179cf7))
* Added methods to access correction/inverse correction data at facade level, updated testing ([79f9ffc](https://github.com/Trusted-Metal/Machine_Config_Library/commit/79f9ffc81c8ca9d60ecf47e79f87b21952a5c212))
* Added OPCUA fields to C++ and GO, added validation tests ([c710d40](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c710d40d76dcc7d5f0d9020bbef984f64686b473))
* Added OPCUA fields to Node and Python, added validation tests ([a738a36](https://github.com/Trusted-Metal/Machine_Config_Library/commit/a738a3672adb8e617c69c7931112594483af1ccb))
* Added Recoater_Blade_Type as optional attribute across all languages, updated testing ([e221148](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e22114879d8eec4ee510d37da8f0b1f7f6e14e5b))
* added scanner invert flags for each language, updated tests ([711ec79](https://github.com/Trusted-Metal/Machine_Config_Library/commit/711ec794ae6e2281383f6ff73b360dbad17db08d))
* Added Synchronous Sensors to all 5 languages, updated testing, fixture files ([715615b](https://github.com/Trusted-Metal/Machine_Config_Library/commit/715615b6f056517936893477f2d9d7ab70fe7fd0))
* Added V1.1 MCF support to all 5 languages, updated testing and fixture files ([00204be](https://github.com/Trusted-Metal/Machine_Config_Library/commit/00204be794471b44433eadd76b5ee9e24b2ab4fa))
* Condensed application testing files ([c027cdf](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c027cdf6850523199a44da67547dd67fdebadd97))
* **cpp:** Added cpp testing application, updated testing documents ([b5ba12e](https://github.com/Trusted-Metal/Machine_Config_Library/commit/b5ba12ea432dcbc88f8a87ee3e0eb6af17193175))
* **cpp:** Finished up initial implementation, quickstart example, workflow example ([912c501](https://github.com/Trusted-Metal/Machine_Config_Library/commit/912c501c7ce6c14dbd5cba06eb1c096af15e4aba))
* **go:** add CLI binary, cross-platform CGo CLI, expanded tests ([0b91080](https://github.com/Trusted-Metal/Machine_Config_Library/commit/0b910800b623de702eb5ebe44add8856874c1d4c))
* **go:** Added go testing application, address ci cross-language speed issue ([84007bc](https://github.com/Trusted-Metal/Machine_Config_Library/commit/84007bc6900c92ca052e0b162da635101e918205))
* **go:** Completed Go implementation, added additional tests where needed ([a9b22e4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/a9b22e4156521bd49d0c4985aea9d3f499124555))
* Moved implementation plans to local folder, added to git ignore, untracked files ([c337cc4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c337cc48c92955ef6072ceb0beffa7bb4ea1fad9))
* **node:** Added node testing app implementation, removed old code pre-refactor ([c824072](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c824072eddfb51cf4824bbb8a1990108b301b020))
* **python:** Updated documentation, added python example app, established test files ([7ef1c36](https://github.com/Trusted-Metal/Machine_Config_Library/commit/7ef1c3699f93c684c83345309da275c7f7c7541a))
* refactor to support stable interfaces ([f972630](https://github.com/Trusted-Metal/Machine_Config_Library/commit/f972630f13595a1e8a6ea449d8845c6a9a03cc84))
* remove clearbox from core MCF ([ed1817f](https://github.com/Trusted-Metal/Machine_Config_Library/commit/ed1817f3dfbba8323752b03d31670be28b24a57f))
* **rust:** Added rust testing application, updated testing documentation ([923800a](https://github.com/Trusted-Metal/Machine_Config_Library/commit/923800afd7e4d5a943fdd571b0cd824fdfe4fe93))
* Standarized how all languages use the adapter/capabilites functionality ([e75cefc](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e75cefc2efabfdcea58d5d5af7ac8fbf003cf803))
* unify power characterization def. for all languages to represent implemented file structs ([1eb19b3](https://github.com/Trusted-Metal/Machine_Config_Library/commit/1eb19b3c79f70187bead09b2a4209aedb3cdaac2))
* Updated OPCUA schema, added opcua fields to rust with field validation, testing ([0cc5cff](https://github.com/Trusted-Metal/Machine_Config_Library/commit/0cc5cff4f298d0687e571088ef95e34ddd297687))

### Bug Fixes

* address npm vulnerabilities by preventing installation of core npm deps we arent using ([17690be](https://github.com/Trusted-Metal/Machine_Config_Library/commit/17690bebc55da18b6337ba9ccaa94bb9ea518e86))
* bug fix, documentation update ([db79e38](https://github.com/Trusted-Metal/Machine_Config_Library/commit/db79e387d90146ee1aea616fcfb0a790fde5d54a))
* **ci:** add HDF5 bin dir to PATH on Windows so DLLs are found at build time ([7fcdfe6](https://github.com/Trusted-Metal/Machine_Config_Library/commit/7fcdfe64b80fd95c26ae75343729f1b0cf9a3203))
* **ci:** Added additional tests for Go implementation, ci fix ([29c05d4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/29c05d414706d17f8d9aff198b8d957d899dd32c))
* **ci:** additional fixes for go implementation of ci ([5525841](https://github.com/Trusted-Metal/Machine_Config_Library/commit/55258418b1a21916a3d390b30d653116ff2622b8))
* **ci:** disable husky hooks in release canidate workflow ([bb43368](https://github.com/Trusted-Metal/Machine_Config_Library/commit/bb43368a5c1c64345f38decdab2a8705fafd25c3))
* **ci:** fix ci for cross_check and go workflows ([d3cd21b](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d3cd21b9461536565577e5b961d99edf63c2c776))
* **ci:** install libhdf5-dev on Linux in cross-check workflow ([6c25874](https://github.com/Trusted-Metal/Machine_Config_Library/commit/6c25874c8a40d1d9c0a94dd2f41d2dfa94f3b9f1))
* **ci:** pin Rust target dir to rust/target via .cargo/config.toml ([f625611](https://github.com/Trusted-Metal/Machine_Config_Library/commit/f6256115d0c8d626b3b694b5923e8de44ea1f9b9))
* **ci:** set CC explicitly for Windows CGo to bypass MSYS2 POSIX PATH ([9b03302](https://github.com/Trusted-Metal/Machine_Config_Library/commit/9b03302450039479525b6b730ada4bea71c151b3))
* **ci:** update Rust binary path after root workspace Cargo.toml was added ([d6a003d](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d6a003d7f89afa3f3edc0771734db5d9b1bba3a2))
* **ci:** use cygpath to resolve MinGW64 gcc path; use POSIX include path ([9ed05de](https://github.com/Trusted-Metal/Machine_Config_Library/commit/9ed05de3e1ee81be67cc55c280b40494ecf61f19))
* **ci:** use SSH deploy key to bypass branch ruleset for semantic-release push ([47b8c0f](https://github.com/Trusted-Metal/Machine_Config_Library/commit/47b8c0fdd3b1fe5f5da494d68919ee24bbaa5d60))
* **ci:** use vcpkg HDF5 on Windows instead of broken HDF5_STATIC env var ([e7b0ab4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e7b0ab448d0306f90835ebd8264639ce53857c4a))
* **cpp:** reader fix ([10ff8e8](https://github.com/Trusted-Metal/Machine_Config_Library/commit/10ff8e854278703bc2c27440e94785f356f92761))
* dependabot using 0.0.0 as version number for vulnerabilities, bumping ([ae06b8f](https://github.com/Trusted-Metal/Machine_Config_Library/commit/ae06b8f2595c174850bcebf564c54e710ccb7a97))
* fix package verification ([2299bc0](https://github.com/Trusted-Metal/Machine_Config_Library/commit/2299bc0ed3d6360ec1cb23b098728f8b8528ad49))
* **go:** fix read parity failures in HDF5 v1.0 adaptor ([b93f94b](https://github.com/Trusted-Metal/Machine_Config_Library/commit/b93f94b8702b6a93cf56e46bebbcab36eb0fd27a))
* **go:** use hardcoded MSYS2 CGo paths on Windows instead of pkg-config ([da5de96](https://github.com/Trusted-Metal/Machine_Config_Library/commit/da5de9695eb6c8af9ae52dc06906c3fccfa66392))
* Last push as ci, no version bump, no artifact generation test, correcting ([d7f3e0e](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d7f3e0e43f4a90c0df704409fe16a8bebd785476))
* **nodejs:** read CLI version from package.json instead of hardcoding ([3a52122](https://github.com/Trusted-Metal/Machine_Config_Library/commit/3a521229212cb87197ea462b5716c7236624c38f))
* order of operations issue in windows test ([1506ef3](https://github.com/Trusted-Metal/Machine_Config_Library/commit/1506ef33b85489b00cb1794e6f8cc0186a5436c4))
* Removed static feature from dependency to work with capture software ([d4702d5](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d4702d596697be699a4a4d2de8746506fadca66f))
* resolve additional CodeQL alrerts on branch ([976dd18](https://github.com/Trusted-Metal/Machine_Config_Library/commit/976dd18d7b7a3efb96cd1213a899aef953e1233b))
* resolve CI failures and CodeQL alerts on branch ([6e2ba25](https://github.com/Trusted-Metal/Machine_Config_Library/commit/6e2ba25dff328c24cf20cb3ef8995e4ef3b7690d))
* restructure into version folders ([8f6faee](https://github.com/Trusted-Metal/Machine_Config_Library/commit/8f6faee64ec1abe46e33c415e7c5b6e7f22290fd))

### Documentation

* ClearBox migration note and plan status ([19a5d61](https://github.com/Trusted-Metal/Machine_Config_Library/commit/19a5d61a8c215442866907b1143189c6fcc51125))

## [1.0.0-rc.1](https://github.com/Trusted-Metal/Machine_Config_Library/compare/v0.2.0-rc.10...v1.0.0-rc.1) (2026-10-30)

### ⚠ BREAKING CHANGES

* ClearBox is no longer part of the typed MachineConfig model in any language.
optical_trains[].optional_components and the ClearBox type are removed. Read and write
extensions[ClearBox] with mcf-ext-clearbox (Node, Rust) instead.
See docs/migrations/clearbox_to_extension.md.

### Documentation

* ClearBox migration note and plan status ([19a5d61](https://github.com/Trusted-Metal/Machine_Config_Library/commit/19a5d61a8c215442866907b1143189c6fcc51125))

## [0.2.0-rc.10](https://github.com/Trusted-Metal/Machine_Config_Library/compare/v0.2.0-rc.9...v0.2.0-rc.10) (2026-10-30)

### Features

* remove clearbox from core MCF ([ed1817f](https://github.com/Trusted-Metal/Machine_Config_Library/commit/ed1817f3dfbba8323752b03d31670be28b24a57f))

## 0.2.0-rc.9 (2026-10-22)

* Merge pull request #22 from Trusted-Metal/SD-1738 ([c6318c0](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c6318c0)), closes [#22](https://github.com/Trusted-Metal/Machine_Config_Library/issues/22)
* fix: fix package verification ([2299bc0](https://github.com/Trusted-Metal/Machine_Config_Library/commit/2299bc0))
* fix: order of operations issue in windows test ([1506ef3](https://github.com/Trusted-Metal/Machine_Config_Library/commit/1506ef3))
* chore: Remove OPCUA extension from MCF lib ([6928c5a](https://github.com/Trusted-Metal/Machine_Config_Library/commit/6928c5a))

## 0.2.0-rc.8 (2026-09-15)

* Merge pull request #21 from Trusted-Metal/SD-1710 ([dd6832d](https://github.com/Trusted-Metal/Machine_Config_Library/commit/dd6832d)), closes [#21](https://github.com/Trusted-Metal/Machine_Config_Library/issues/21)
* feat: Added hash writing/hash checking to MCF to all languages, updated testing/docs ([dc14d57](https://github.com/Trusted-Metal/Machine_Config_Library/commit/dc14d57))
* feat: Moved implementation plans to local folder, added to git ignore, untracked files ([c337cc4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c337cc4))

## 0.2.0-rc.7 (2026-09-11)

* Merge pull request #20 from Trusted-Metal/SD-1692 ([33d9251](https://github.com/Trusted-Metal/Machine_Config_Library/commit/33d9251)), closes [#20](https://github.com/Trusted-Metal/Machine_Config_Library/issues/20)
* feat: Added Recoater_Blade_Type as optional attribute across all languages, updated testing ([e221148](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e221148))

## 0.2.0-rc.6 (2026-09-03)

* Merge pull request #19 from Trusted-Metal/SD-1684 ([6b9d667](https://github.com/Trusted-Metal/Machine_Config_Library/commit/6b9d667)), closes [#19](https://github.com/Trusted-Metal/Machine_Config_Library/issues/19)
* Potential fix for pull request finding 'CodeQL / Incorrect conversion between integer types' ([20e4f42](https://github.com/Trusted-Metal/Machine_Config_Library/commit/20e4f42))
* Potential fix for pull request finding 'CodeQL / Prototype-polluting assignment' ([62ce2ea](https://github.com/Trusted-Metal/Machine_Config_Library/commit/62ce2ea))
* Potential fix for pull request finding 'CodeQL / Workflow does not contain permissions' ([5585229](https://github.com/Trusted-Metal/Machine_Config_Library/commit/5585229))
* feat: Add C++ tarball and version reporting; added Rust cargo-package CI check ([d6d4d82](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d6d4d82))
* feat: added capabilities to lib for Rust, updated testing ([3079e77](https://github.com/Trusted-Metal/Machine_Config_Library/commit/3079e77))
* feat: Added methods to access correction/inverse correction data at facade level, updated testing ([79f9ffc](https://github.com/Trusted-Metal/Machine_Config_Library/commit/79f9ffc))
* feat: Added OPCUA fields to C++ and GO, added validation tests ([c710d40](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c710d40))
* feat: Added OPCUA fields to Node and Python, added validation tests ([a738a36](https://github.com/Trusted-Metal/Machine_Config_Library/commit/a738a36))
* feat: added scanner invert flags for each language, updated tests ([711ec79](https://github.com/Trusted-Metal/Machine_Config_Library/commit/711ec79))
* feat: Added Synchronous Sensors to all 5 languages, updated testing, fixture files ([715615b](https://github.com/Trusted-Metal/Machine_Config_Library/commit/715615b))
* feat: Added V1.1 MCF support to all 5 languages, updated testing and fixture files ([00204be](https://github.com/Trusted-Metal/Machine_Config_Library/commit/00204be))
* feat: Condensed application testing files ([c027cdf](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c027cdf))
* feat: refactor to support stable interfaces ([f972630](https://github.com/Trusted-Metal/Machine_Config_Library/commit/f972630))
* feat: Standarized how all languages use the adapter/capabilites functionality ([e75cefc](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e75cefc))
* feat: unify power characterization def. for all languages to represent implemented file structs ([1eb19b3](https://github.com/Trusted-Metal/Machine_Config_Library/commit/1eb19b3))
* feat: Updated OPCUA schema, added opcua fields to rust with field validation, testing ([0cc5cff](https://github.com/Trusted-Metal/Machine_Config_Library/commit/0cc5cff))
* feat(cpp): Added cpp testing application, updated testing documents ([b5ba12e](https://github.com/Trusted-Metal/Machine_Config_Library/commit/b5ba12e))
* feat(go): add CLI binary, cross-platform CGo CLI, expanded tests ([0b91080](https://github.com/Trusted-Metal/Machine_Config_Library/commit/0b91080))
* feat(go): Added go testing application, address ci cross-language speed issue ([84007bc](https://github.com/Trusted-Metal/Machine_Config_Library/commit/84007bc))
* feat(go): Completed Go implementation, added additional tests where needed ([a9b22e4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/a9b22e4))
* feat(node): Added node testing app implementation, removed old code pre-refactor ([c824072](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c824072))
* feat(python): Updated documentation, added python example app, established test files ([7ef1c36](https://github.com/Trusted-Metal/Machine_Config_Library/commit/7ef1c36))
* feat(rust): Added rust testing application, updated testing documentation ([923800a](https://github.com/Trusted-Metal/Machine_Config_Library/commit/923800a))
* ci: Addressed github bumping hdf5 to 2.20 ([49b1775](https://github.com/Trusted-Metal/Machine_Config_Library/commit/49b1775))
* docs(go): Go documentation update ([9d08d74](https://github.com/Trusted-Metal/Machine_Config_Library/commit/9d08d74))
* docs(python): Python documentation update ([92c789a](https://github.com/Trusted-Metal/Machine_Config_Library/commit/92c789a))
* fix: bug fix, documentation update ([db79e38](https://github.com/Trusted-Metal/Machine_Config_Library/commit/db79e38))
* fix: dependabot using 0.0.0 as version number for vulnerabilities, bumping ([ae06b8f](https://github.com/Trusted-Metal/Machine_Config_Library/commit/ae06b8f))
* fix: resolve additional CodeQL alrerts on branch ([976dd18](https://github.com/Trusted-Metal/Machine_Config_Library/commit/976dd18))
* fix: resolve CI failures and CodeQL alerts on branch ([6e2ba25](https://github.com/Trusted-Metal/Machine_Config_Library/commit/6e2ba25))
* fix: restructure into version folders ([8f6faee](https://github.com/Trusted-Metal/Machine_Config_Library/commit/8f6faee))
* fix(ci): Added additional tests for Go implementation, ci fix ([29c05d4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/29c05d4))
* fix(ci): additional fixes for go implementation of ci ([5525841](https://github.com/Trusted-Metal/Machine_Config_Library/commit/5525841))
* fix(ci): fix ci for cross_check and go workflows ([d3cd21b](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d3cd21b))
* fix(ci): set CC explicitly for Windows CGo to bypass MSYS2 POSIX PATH ([9b03302](https://github.com/Trusted-Metal/Machine_Config_Library/commit/9b03302))
* fix(ci): use cygpath to resolve MinGW64 gcc path; use POSIX include path ([9ed05de](https://github.com/Trusted-Metal/Machine_Config_Library/commit/9ed05de))
* fix(go): fix read parity failures in HDF5 v1.0 adaptor ([b93f94b](https://github.com/Trusted-Metal/Machine_Config_Library/commit/b93f94b))
* fix(go): use hardcoded MSYS2 CGo paths on Windows instead of pkg-config ([da5de96](https://github.com/Trusted-Metal/Machine_Config_Library/commit/da5de96))
* refactor(go/h5c): scalar safety, global error suppression, dedup struct ([9f866a0](https://github.com/Trusted-Metal/Machine_Config_Library/commit/9f866a0))

## 0.2.0-rc.5 (2026-08-12)

* Merge pull request #17 from Trusted-Metal/SD-1677 ([44707e2](https://github.com/Trusted-Metal/Machine_Config_Library/commit/44707e2)), closes [#17](https://github.com/Trusted-Metal/Machine_Config_Library/issues/17)
* fix: address npm vulnerabilities by preventing installation of core npm deps we arent using ([17690be](https://github.com/Trusted-Metal/Machine_Config_Library/commit/17690be))

## 0.2.0-rc.4 (2026-08-11)

* Create LICENSE ([70f0ffc](https://github.com/Trusted-Metal/Machine_Config_Library/commit/70f0ffc))
* Merge pull request #15 from Trusted-Metal/richardoconnell-patch-1 ([e5f06d7](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e5f06d7)), closes [#15](https://github.com/Trusted-Metal/Machine_Config_Library/issues/15)
* Merge pull request #16 from Trusted-Metal/SD-1671 ([a8509b1](https://github.com/Trusted-Metal/Machine_Config_Library/commit/a8509b1)), closes [#16](https://github.com/Trusted-Metal/Machine_Config_Library/issues/16)
* fix: Removed static feature from dependency to work with capture software ([d4702d5](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d4702d5))
* fix(ci): add HDF5 bin dir to PATH on Windows so DLLs are found at build time ([7fcdfe6](https://github.com/Trusted-Metal/Machine_Config_Library/commit/7fcdfe6))
* fix(ci): install libhdf5-dev on Linux in cross-check workflow ([6c25874](https://github.com/Trusted-Metal/Machine_Config_Library/commit/6c25874))
* fix(ci): use vcpkg HDF5 on Windows instead of broken HDF5_STATIC env var ([e7b0ab4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e7b0ab4))

## 0.2.0-rc.3 (2026-08-07)

* Merge pull request #13 from Trusted-Metal/SD-1665 ([f4a3d9f](https://github.com/Trusted-Metal/Machine_Config_Library/commit/f4a3d9f)), closes [#13](https://github.com/Trusted-Metal/Machine_Config_Library/issues/13)
* Merge pull request #14 from Trusted-Metal/SD-1666 ([4ed7251](https://github.com/Trusted-Metal/Machine_Config_Library/commit/4ed7251)), closes [#14](https://github.com/Trusted-Metal/Machine_Config_Library/issues/14)
* fix: Last push as ci, no version bump, no artifact generation test, correcting ([d7f3e0e](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d7f3e0e))
* ci: wire tgz and wheel artifact generation into semantic release ([67ecbde](https://github.com/Trusted-Metal/Machine_Config_Library/commit/67ecbde))

## 0.2.0-rc.2 (2026-08-07)

* Merge pull request #12 from Trusted-Metal/SD-1663 ([ad87b87](https://github.com/Trusted-Metal/Machine_Config_Library/commit/ad87b87)), closes [#12](https://github.com/Trusted-Metal/Machine_Config_Library/issues/12)
* docs: Updated usage.md and language files with library dependency usage info ([fd7b641](https://github.com/Trusted-Metal/Machine_Config_Library/commit/fd7b641))
* feat: Add Node tgz and Python wheel build support, Go implementation ([bf60969](https://github.com/Trusted-Metal/Machine_Config_Library/commit/bf60969))

## 0.2.0-rc.1 (2026-08-05)

* Merge pull request #10 from Trusted-Metal/SD-1659 ([76e9874](https://github.com/Trusted-Metal/Machine_Config_Library/commit/76e9874)), closes [#10](https://github.com/Trusted-Metal/Machine_Config_Library/issues/10)
* Merge pull request #11 from Trusted-Metal/SD-1660 ([a2be2c2](https://github.com/Trusted-Metal/Machine_Config_Library/commit/a2be2c2)), closes [#11](https://github.com/Trusted-Metal/Machine_Config_Library/issues/11)
* Merge pull request #9 from Trusted-Metal/SD-1658 ([3fb70c5](https://github.com/Trusted-Metal/Machine_Config_Library/commit/3fb70c5)), closes [#9](https://github.com/Trusted-Metal/Machine_Config_Library/issues/9)
* test(cpp,rust,node): Added OPCUA testing for Cpp, Rust and Node ([d529de5](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d529de5))
* docs: Broke usage.md file into language specific files, added additional support docs ([8ad6710](https://github.com/Trusted-Metal/Machine_Config_Library/commit/8ad6710))
* ci: added C++ workflow ([e29b628](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e29b628))
* ci: address pipeline issue ([0c15255](https://github.com/Trusted-Metal/Machine_Config_Library/commit/0c15255))
* ci: ci pipeline fix, ensures correct version of ubuntu is used ([abb64ef](https://github.com/Trusted-Metal/Machine_Config_Library/commit/abb64ef))
* ci: cmake version minimum enforcement ([4908557](https://github.com/Trusted-Metal/Machine_Config_Library/commit/4908557))
* ci: diagnostic add to determine CI issue ([33a7834](https://github.com/Trusted-Metal/Machine_Config_Library/commit/33a7834))
* ci: fixed flag issue in pipeline ([04c9dcf](https://github.com/Trusted-Metal/Machine_Config_Library/commit/04c9dcf))
* ci: pipeline fix, still addressing ubuntu/h5 version issue ([ba43b31](https://github.com/Trusted-Metal/Machine_Config_Library/commit/ba43b31))
* ci(cpp): Added cpp to cross language check ([78d5035](https://github.com/Trusted-Metal/Machine_Config_Library/commit/78d5035))
* ci(cpp): Addressed HDF5 caching issue ([55403f8](https://github.com/Trusted-Metal/Machine_Config_Library/commit/55403f8))
* ci(cpp): HDF5 caching issue ([6faa1b4](https://github.com/Trusted-Metal/Machine_Config_Library/commit/6faa1b4))
* feat(cpp): Finished up initial implementation, quickstart example, workflow example ([912c501](https://github.com/Trusted-Metal/Machine_Config_Library/commit/912c501))
* fix(cpp): reader fix ([10ff8e8](https://github.com/Trusted-Metal/Machine_Config_Library/commit/10ff8e8))
* chore: add copy-hdf5 CLI subcommand, added additional round trip testing ([b10fd42](https://github.com/Trusted-Metal/Machine_Config_Library/commit/b10fd42))
* chore: Added Workflow examples, updated documentation ([e9832e1](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e9832e1))
* chore: fixed issue with skipped test phases being marked as passing rather than skipped ([faa6d57](https://github.com/Trusted-Metal/Machine_Config_Library/commit/faa6d57))
* chore: git issue was causing hash check to fail ([c3f0e3b](https://github.com/Trusted-Metal/Machine_Config_Library/commit/c3f0e3b))
* chore: Python now being tested on both Windows and Ubuntu in pipeline ([e9bfbe0](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e9bfbe0))
* chore(cpp): implement reader, models and CLI ([e18ffc6](https://github.com/Trusted-Metal/Machine_Config_Library/commit/e18ffc6))

## <small>0.1.1-rc.1 (2026-07-31)</small>

* Merge pull request #5 from Trusted-Metal/SD-1653 ([9a671ca](https://github.com/Trusted-Metal/Machine_Config_Library/commit/9a671ca)), closes [#5](https://github.com/Trusted-Metal/Machine_Config_Library/issues/5)
* Merge pull request #6 from Trusted-Metal/SD-1655 ([33d1ba1](https://github.com/Trusted-Metal/Machine_Config_Library/commit/33d1ba1)), closes [#6](https://github.com/Trusted-Metal/Machine_Config_Library/issues/6)
* Merge pull request #7 from Trusted-Metal/SD-1656 ([ae86c79](https://github.com/Trusted-Metal/Machine_Config_Library/commit/ae86c79)), closes [#7](https://github.com/Trusted-Metal/Machine_Config_Library/issues/7)
* Merge pull request #8 from Trusted-Metal/SD-1656 ([b0631b8](https://github.com/Trusted-Metal/Machine_Config_Library/commit/b0631b8)), closes [#8](https://github.com/Trusted-Metal/Machine_Config_Library/issues/8)
* fix(ci): disable husky hooks in release canidate workflow ([bb43368](https://github.com/Trusted-Metal/Machine_Config_Library/commit/bb43368))
* fix(ci): pin Rust target dir to rust/target via .cargo/config.toml ([f625611](https://github.com/Trusted-Metal/Machine_Config_Library/commit/f625611))
* fix(ci): update Rust binary path after root workspace Cargo.toml was added ([d6a003d](https://github.com/Trusted-Metal/Machine_Config_Library/commit/d6a003d))
* fix(ci): use SSH deploy key to bypass branch ruleset for semantic-release push ([47b8c0f](https://github.com/Trusted-Metal/Machine_Config_Library/commit/47b8c0f))
* fix(nodejs): read CLI version from package.json instead of hardcoding ([3a52122](https://github.com/Trusted-Metal/Machine_Config_Library/commit/3a52122))
* ci: run all checks regardless on PR and push to main/release ([ba05827](https://github.com/Trusted-Metal/Machine_Config_Library/commit/ba05827))
* chore(ci): add commitlint and husky commit-msg hook ([0bde128](https://github.com/Trusted-Metal/Machine_Config_Library/commit/0bde128))
* chore(ci): add semantic-release pipeline ([eeada63](https://github.com/Trusted-Metal/Machine_Config_Library/commit/eeada63))
* chore(rust): add workspace root for git dep support ([dfd4cdb](https://github.com/Trusted-Metal/Machine_Config_Library/commit/dfd4cdb))
