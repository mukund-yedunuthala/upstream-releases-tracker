# Changelog

### [v3.6.2](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.6.1...v3.6.2) (2026-09-21)

### [v3.6.1](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.6.0...v3.6.1) (2026-09-02)

#### Fixes

* third party license notices link
([e196cea](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/e196cea3803e717b26ad45747ab0d9bf0af1fdfc))

## [v3.6.0](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.5.8...v3.6.0) (2026-08-17)

### Features

* **theme:** add persisted theme settings and accent handling
([103a85f](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/103a85f9a5cc013891bec96ee267031d80302387))

### Fixes

* css formatting
([6368bd1](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/6368bd12d898adec5cc4adefc9236761bc303f67))

### [v3.5.8](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.5.7...v3.5.8) (2026-08-08)

#### Features

* add latest_release_timestamp field and display when update is available
([ecc7033](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/ecc7033db6bfa1a49ac1f3d25526874e92acf1b7))

### [v3.5.7](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.5.4...v3.5.7) (2026-07-18)

#### Features

* **logs:** add clear logs command
([ddf6e72](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/ddf6e722688ec8a3d97677d3bfffee75d754a09e))
* **logs:** add clear logs button
([385dfc5](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/385dfc5f8865629c7c5af7af03932cb9e0d7dbb7))

### [v3.5.4](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.5.2...v3.5.4) (2026-06-17)

### [v3.5.2](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.5.1...v3.5.2) (2026-06-02)

#### Features

* **N2:** per-host Forgejo tokens
([caa78f4](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/caa78f4d067e7afb9c3c722df4ab8db493b01632))

### [v3.5.1](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.5.0...v3.5.1) (2026-05-26)

#### Features

* **ui:** delete dialog, inline validation, toolbar, status footer
([378d926](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/378d926fc4bd6230db4c7cc01ef1882e3d1d3a25))

#### Fixes

* **frontend:** remove host param, harden vault reset, fix URL validation,
a11y
([1b4e5fa](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/1b4e5faa13501fa0a2ca3cca966cfb5032d8521a))
* **backend:** security hardening, robustness, tests, and refactors
([4105384](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/41053841253e663018e9f64dcd2d1ac7a69c9322))
* **ui:** restore [hidden] visibility for buttons broken by oat :is()
specificity
([9135bce](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/9135bce6212cf6fe21fb543ee496ba90339488b2))
* **ui:** topbar/toolbar alignment, flatten settings tabs
([7452485](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/7452485dc3d17937a6aa9a3fc31d2511a860f3e2))

## [v3.5.0](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.4.3...v3.5.0) (2026-05-23)

### Features

* **ui:** expandable release notes, copy-to-clipboard, QoL polish (3.5.0)
([28075f5](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/28075f546d1208ffab8bb2859949901925f5ebdd))

### Fixes

* **vault:** enable native OS keyring backends; add vault auto-recovery
([f3caf13](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/f3caf131152562d297ac7e5d98ce778009f11451))
* **ci:** the path in actions/upload-artifact was wrong
([a51a873](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/a51a87342d7d887a06cb4bbda38fb89b17f562a6))
* **ci:** use the official tauri build action
([d1a282a](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/d1a282ae83aed480abba975b03f6f45041373f01))
* **ci:** fix install deps missing step
([64e2f5c](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/64e2f5cd5d59b88590ae6d29bcb482b47a809ecf))

### [v3.4.3](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.4.2...v3.4.3) (2026-05-21)

### [v3.4.2](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.4.1...v3.4.2) (2026-05-21)

#### Fixes

* **frontend:** widen settings dialog by overriding oat's 32rem cap
([ebda69f](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/ebda69f3daef9451b66ed50b41f01c96c11c708f))

### [v3.4.1](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.4.0...v3.4.1) (2026-05-20)

#### Fixes

* edit-dialog race & GitLab subgroup support (BUGS #42 #43 #50)
([123ccbc](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/123ccbc07b04568fcbfb721f930e5c98b91c4fdc)),
closes
[#42](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/42)
[#43](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/43)
[#50](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/50)
[#42](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/42)
[#43](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/43)
[#43](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/43)
[#50](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/50)
* **backend:** config persistence integrity & log bounds (BUGS #38 #39 #40 #41
#46)
([47fff4b](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/47fff4bf2d55cb5b3ab302a1fe921f52c91693a2)),
closes
[#38](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/38)
[#39](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/39)
[#40](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/40)
[#41](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/41)
[#46](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/46)
[#38](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/38)
[#39](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/39)
[#40](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/40)
[#41](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/41)
[#46](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/46)
[#38](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/38)

## [v3.4.0](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.3.0...v3.4.0) (2026-05-20)

### Features

* GitLab support, Stronghold vault, Store endpoints, Settings UI (3.4.0)
([36b1ed8](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/36b1ed8b57fd9e7d0b1803b11fbd5c52bf0ab1ce))

### Fixes

* **deps:** add oat as npm package
([cf2bd82](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/cf2bd82fad1edf3d1c689d3ec13a6cee1d7e831c))
* now open links externally again
([a818a0c](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/a818a0ca3251c6c57c3b82c2210a7992d754a4fa))

## [v3.3.0](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.2.4...v3.3.0) (2026-05-20)

### Fixes

* **forge:** tighten URL parsing & remove Unknown variant (BUGS #12 #15 #28)
([64db2e6](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/64db2e6b083b8c2ab277c20b8dca834102f64e5c)),
closes
[#12](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/12)
[#15](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/15)
[#28](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/28)
[#12](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/12)
[#15](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/15)
[#28](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/28)
* **frontend:** repair edit dialog (BUGS #2 #3 #13 #20)
([325e82e](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/325e82e925e702a5f6db5e91eb8d7efcf2532fef)),
closes
[#2](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/2)
[#3](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/3)
[#13](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/13)
[#20](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/20)
[#3](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/3)
[#2](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/2)
[#13](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/13)
[#20](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/issues/20)

### [v3.2.4](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.2.3...v3.2.4) (2026-05-08)

### [v3.2.3](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.2.2...v3.2.3) (2026-05-08)

#### Fixes

* **deps:** add oat as npm package
([cf2bd82](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/cf2bd82fad1edf3d1c689d3ec13a6cee1d7e831c))

### [v3.2.2](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.2.1...v3.2.2) (2026-04-07)

#### Fixes

* now open links externally again
([a818a0c](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/a818a0ca3251c6c57c3b82c2210a7992d754a4fa))

### [v3.2.1](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.2.0...v3.2.1) (2026-03-14)

## [v3.2.0](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.1.0...v3.2.0) (2026-03-01)

### Features

* add codeberg support frontend
([5eccc30](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/5eccc3012a6e3ce9a0a65a00c4a8419498d93d8e))
* add codeberg support backend
([224568e](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/224568e6e911833f69e5012033f2c5d8203d8c60))
* let user pick the host, add data migration, fix config missing, TODO:gitlab
([b457f39](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/b457f39cd3529dc4d394624061114b20722d5464))
* let user pick the host in frontend, add regex url validation
([0f171b5](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/0f171b5e3c1eee2b72d5313ba89869079467a372))

## [v3.1.0](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v3.0.1...v3.1.0) (2026-02-19)

### Features

* add manual trigger to workflow
([2902711](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/29027113fc45d5b16e9709c2851c472b6402c895))
* major frontend rewrite
([70ebd9c](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/70ebd9cdca91845ff8215743eaca1ffe0ccc1875))

### Fixes

* fix tauri.conf.json that got wrecked
([e665327](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/e6653271a2171560f1b0d1e5f29d18cf736c391c))
* fix package.json
([8ea0354](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/8ea035473d5af43ac223cc3a2cacfd262bd1e594))
* remove references to trunk and wasm and add npm
([89cb944](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/89cb944bb59ab2975085e365bf4719f8c531af20))
* fix frontend build
([d5c051f](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/d5c051f532fbe4f410d0a7b4d40adfecaba1ff52))
* remove expanduser dep
([95c3410](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/95c3410ab065e1dd00a64064e0c630613789a135))
* add public dir for trunk
([33a0bd9](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/33a0bd96b25bbc58c7fd9e049ff0c0c3d927212f))

### [v3.0.1](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v2.0.3...v3.0.1) (2026-02-19)

#### Features

* major frontend rewrite
([2209939](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/220993923a02a316564d886f849d243c5d2178de))

#### Fixes

* fix files check at first startup
([94a508a](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/94a508a86ad16337af897e10e5f97af548b65008))
* fix frontend flickers and delete button placement
([74b8aa6](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/74b8aa62027ee22e467c3767068b5a959589410c))

### [v2.0.3](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v2.0.2...v2.0.3) (2026-02-18)

### [v2.0.2](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/compare/v2.0.1...v2.0.2) (2025-11-25)

#### Fixes

* change status message
([59b0a18](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/59b0a180d6f15b7940af3a6aa8e244448d8cc2f4))

### v2.0.1 (2025-11-10)

#### Features

* implement marking repo as updated
([e322826](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/e32282637b6dc39c3ba125e7a4d8526b79586270))
* implement adding repo
([70b393e](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/70b393e4ea4f28ee41f37730be9b95ccde14cb2c))
* implement delete and update buttons, improve padding
([4df0849](https://gitlab.com/mukund-yedunuthala/upstream-releases-tracker-tauri/commit/4df08492fb317e8ec1e6c3d1a78d9df496e674f6))
