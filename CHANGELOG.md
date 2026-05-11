# Changelog

## [0.5.0](https://github.com/ophidiarium/mehen/compare/v0.4.3...v0.5.0) (2026-05-11)


### Features

* **cli:** add --version --json and surface version in action footer ([#77](https://github.com/ophidiarium/mehen/issues/77)) ([199e37d](https://github.com/ophidiarium/mehen/commit/199e37d31bfb1fec1746f78f484330b0056614ea))
* **langs:** add C language support ([#80](https://github.com/ophidiarium/mehen/issues/80)) ([783a1e8](https://github.com/ophidiarium/mehen/commit/783a1e886cf43907f8370693f6bf09e859dcc508))
* **langs:** route JavaScript/JSX through TypeScript/TSX grammars ([#79](https://github.com/ophidiarium/mehen/issues/79)) ([a6fb345](https://github.com/ophidiarium/mehen/commit/a6fb345898ea965bfb7f77983c774d6558b842a6))


### Miscellaneous Chores

* release 0.5.0 ([b9f81c3](https://github.com/ophidiarium/mehen/commit/b9f81c3ee5223736d72f21fbf933724e86000945))

## [0.4.3](https://github.com/ophidiarium/mehen/compare/v0.4.2...v0.4.3) (2026-05-09)


### Bug Fixes

* **npm:** restore +x on platform binaries and surface spawn errors ([#75](https://github.com/ophidiarium/mehen/issues/75)) ([e12549e](https://github.com/ophidiarium/mehen/commit/e12549eb9bcccb586f424101a5d1ee50a41aec6d))

## [0.4.2](https://github.com/ophidiarium/mehen/compare/v0.4.1...v0.4.2) (2026-05-08)


### Bug Fixes

* Rust metric edge cases ([#73](https://github.com/ophidiarium/mehen/issues/73)) ([8bccab4](https://github.com/ophidiarium/mehen/commit/8bccab409c6e8cac29e737b085baebbf4ba14d61))

## [0.4.1](https://github.com/ophidiarium/mehen/compare/v0.4.0...v0.4.1) (2026-05-07)


### Features

* **cli:** add top-offenders subcommand ([#71](https://github.com/ophidiarium/mehen/issues/71)) ([e07c061](https://github.com/ophidiarium/mehen/commit/e07c061463ed55fa5736b9e71f09979eba1d39a4))

## [0.4.0](https://github.com/ophidiarium/mehen/compare/v0.3.0...v0.4.0) (2026-05-05)


### Features

* **metrics:** add PowerShell language support ([#69](https://github.com/ophidiarium/mehen/issues/69)) ([7a34436](https://github.com/ophidiarium/mehen/commit/7a344361c086214f0b98730e6bae10f02b0ab22a))


### Miscellaneous Chores

* release 0.4.0 ([5a3a699](https://github.com/ophidiarium/mehen/commit/5a3a69986f6193a17bfbbed0831d8c648075c0b7))

## [0.3.0](https://github.com/ophidiarium/mehen/compare/v0.2.0...v0.3.0) (2026-05-02)


### Features

* **metrics:** add Kotlin language support ([#66](https://github.com/ophidiarium/mehen/issues/66)) ([028f93b](https://github.com/ophidiarium/mehen/commit/028f93b802241d742e16d4952d798dc980a8535a))


### Miscellaneous Chores

* release 0.3.0 ([8b6dad6](https://github.com/ophidiarium/mehen/commit/8b6dad68dc3668a55666e46498b539dff073dd4f))

## [0.2.0](https://github.com/ophidiarium/mehen/compare/v0.1.1...v0.2.0) (2026-04-30)


### ⚠ BREAKING CHANGES

* **metrics:** close class-metric gaps, remove --ops/--comments ([#65](https://github.com/ophidiarium/mehen/issues/65))
* **diff:** `mehen diff --metrics mi` (and the action's `metrics:` / `thresholds:` keys referencing `mi`, `maintainability`, or `maintainabilityindex`) no longer resolve — pick an explicit variant (`mi.original`, `mi.sei`, `mi.visual_studio`). Users relying on the default automatically get `mi.visual_studio` now.

### Features

* **diff:** split mi into mi.original/mi.sei/mi.visual_studio, default to Visual Studio ([#64](https://github.com/ophidiarium/mehen/issues/64)) ([cb22ed6](https://github.com/ophidiarium/mehen/commit/cb22ed6afd0def04b0f8b9336a95389469fcf361))
* **metrics:** close class-metric gaps, remove --ops/--comments ([#65](https://github.com/ophidiarium/mehen/issues/65)) ([25a8218](https://github.com/ophidiarium/mehen/commit/25a8218f1c67c7d620f19ddc6f7fb70d7e00c7ab))
* **metrics:** gate wmc/npa/npm by language applicability ([#61](https://github.com/ophidiarium/mehen/issues/61)) ([975726a](https://github.com/ophidiarium/mehen/commit/975726a87c9ee112da7c4a2385ac687c805abd6a))


### Bug Fixes

* **metrics:** align cyclomatic and cognitive with language semantics ([#63](https://github.com/ophidiarium/mehen/issues/63)) ([7425460](https://github.com/ophidiarium/mehen/commit/7425460f91c87c360dee2443878d925b2bce0b4f))

## [0.1.1](https://github.com/ophidiarium/mehen/compare/v0.0.6...v0.1.1) (2026-04-30)


### Features

* **action:** retitle comment, add ABC default, auto-exclude tests ([#60](https://github.com/ophidiarium/mehen/issues/60)) ([fb470d6](https://github.com/ophidiarium/mehen/commit/fb470d6772c5e46e88ead129e14871ca54afd944))
* add Ruby language support ([#57](https://github.com/ophidiarium/mehen/issues/57)) ([fb8d53c](https://github.com/ophidiarium/mehen/commit/fb8d53cca1d88a916a15e0d2e9e9297df2f8c145))
* align Go metrics with language semantics ([#59](https://github.com/ophidiarium/mehen/issues/59)) ([ae3cf53](https://github.com/ophidiarium/mehen/commit/ae3cf53684e78e717457c9edee9b4eaa29640587))


### Miscellaneous Chores

* release 0.1.1 ([03018d7](https://github.com/ophidiarium/mehen/commit/03018d76f4978f8e219b93b0c2f625c3f7b922d7))

## [0.0.6](https://github.com/ophidiarium/mehen/compare/v0.0.5...v0.0.6) (2026-04-25)


### Features

* add reusable mehen metrics action ([#55](https://github.com/ophidiarium/mehen/issues/55)) ([11c5a52](https://github.com/ophidiarium/mehen/commit/11c5a529e4d006fa74f15cd453fb188c36ab9c40))

## [0.0.5](https://github.com/ophidiarium/mehen/compare/v0.0.4...v0.0.5) (2026-04-07)


### Bug Fixes

* **ci:** remove broken npm@11 global install from static-analysis ([#48](https://github.com/ophidiarium/mehen/issues/48)) ([1774559](https://github.com/ophidiarium/mehen/commit/17745598d764638d2736e2ca539872ff7666c42b))

## [0.0.4](https://github.com/ophidiarium/mehen/compare/v0.0.3...v0.0.4) (2026-02-16)


### Features

* add `mehen diff` subcommand ([#24](https://github.com/ophidiarium/mehen/issues/24)) ([8edfcdf](https://github.com/ophidiarium/mehen/commit/8edfcdfef6b98c726becc681533bbb6a89c237db))

## [0.0.3](https://github.com/ophidiarium/mehen/compare/v0.0.2...v0.0.3) (2026-02-16)


### Bug Fixes

* update typos config ([a41a4a4](https://github.com/ophidiarium/mehen/commit/a41a4a462e96d965641a4615e2403646c4e0885a))
* use GitHub App token in release-please workflow ([#20](https://github.com/ophidiarium/mehen/issues/20)) ([88cf7ba](https://github.com/ophidiarium/mehen/commit/88cf7ba030b4ffbe74203cef9250e7d56702184b))

## [0.0.2](https://github.com/ophidiarium/mehen/compare/v0.0.1...v0.0.2) (2026-02-15)


### Features

* Add Go language support ([4ed54eb](https://github.com/ophidiarium/mehen/commit/4ed54ebbebde67ae6429c1ef31349d8a2b866dd4))
* Expose inner tree-sitter::Node for advanced use cases ([#1210](https://github.com/ophidiarium/mehen/issues/1210)) ([2e167b0](https://github.com/ophidiarium/mehen/commit/2e167b00f906629b2341450e4485b570f8c02f0e))


### Bug Fixes

* Address clippy collapsible_if warnings with let-chains ([#1211](https://github.com/ophidiarium/mehen/issues/1211)) ([383c639](https://github.com/ophidiarium/mehen/commit/383c639cfe3ca6aa84f3c73e1df68beaff7151c7))
* check PR author instead of event actor in regeneration workflow ([9c91fcc](https://github.com/ophidiarium/mehen/commit/9c91fcce6eaef2bb212e3146d70cb66f8ea8a080))
* handle unmapped Unicode chars in enum generator ([#15](https://github.com/ophidiarium/mehen/issues/15)) ([1f3e115](https://github.com/ophidiarium/mehen/commit/1f3e115dba83f4e2d488718a6d2e0fea82d830d6))
* hanging server ([#806](https://github.com/ophidiarium/mehen/issues/806)) ([08c61f4](https://github.com/ophidiarium/mehen/commit/08c61f48ad7ea3ad20bff9deb64fd211e7f93857))
* trigger on Cargo.lock ([3ec887b](https://github.com/ophidiarium/mehen/commit/3ec887b89732c64004fcc97191728fe5de0ae48a))
