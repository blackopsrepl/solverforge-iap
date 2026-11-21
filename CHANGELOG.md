# Changelog

All notable changes to this project will be documented in this file. See [Conventional Commits](https://conventionalcommits.org) for commit guidelines.

## [0.3.1](https://github.com/blackopsrepl/solverforge-iap/compare/v0.3.0...v0.3.1) (2026-11-21)


### Bug Fixes

* **solver:** retune late acceptance for the corrected acceptor ([c780525](https://github.com/blackopsrepl/solverforge-iap/commit/c780525a38888f3f817859e49031be30293aaf77))


## [0.3.0](https://github.com/blackopsrepl/solverforge-iap/compare/v0.2.0...v0.3.0) (2026-11-20)


### ⚠ BREAKING CHANGES

* **api:** constraint analysis entries replace `weight` with `weightMultiplier` and `isHard`, and selector telemetry no longer carries the six `conflictRepair*` counters, which the runtime removed.

### Features

* **api:** port to solverforge 0.31.2 ([83dd898](https://github.com/blackopsrepl/solverforge-iap/commit/83dd8983af3d8cd28d32b13d51452fb74a6cba0b))


### Bug Fixes

* **constraints:** declare slot_conflict a hard rule ([4eedb7e](https://github.com/blackopsrepl/solverforge-iap/commit/4eedb7e9193fa6f3f28589b015448111a3f4b6aa))


## [0.2.0](https://github.com/blackopsrepl/solverforge-iap/compare/v0.1.1...v0.2.0) (2026-10-27)


### ⚠ BREAKING CHANGES

* **api:** job telemetry reports phases (an array of every started phase) instead of the single phase object.

### Features

* **api:** port to solverforge 0.23 and report every phase's telemetry ([2e3dab1](https://github.com/blackopsrepl/solverforge-iap/commit/2e3dab11e364f4b014d163f3bddf24be4ef87d85))
* **solver:** opt into parallel local search per job ([8be28d8](https://github.com/blackopsrepl/solverforge-iap/commit/8be28d871e10b23f066660acd31bf4ec7e1e36a0))
* **sweep:** expose per-job budgets through make ([e3ad427](https://github.com/blackopsrepl/solverforge-iap/commit/e3ad4273d8f73eac4be02ff982b721b7bd938d3b))


### Bug Fixes

* **release:** read commit types as a top-level csemver key ([d60f74a](https://github.com/blackopsrepl/solverforge-iap/commit/d60f74a09c2c3e5fc4d1df7e9af6fc3913d70888))
* **sweep:** point the sweep at the snapshot endpoint and finish the teardown ([e458d40](https://github.com/blackopsrepl/solverforge-iap/commit/e458d400fca36eb82c3145a53b5571c4989bcfe0))


### Documentation

* **results:** publish the complete CM and CR console sweep ([179fd3f](https://github.com/blackopsrepl/solverforge-iap/commit/179fd3f41c929f9ff64cea3c569c74e11473274b))


## [0.1.1](https://github.com/blackopsrepl/solverforge-iap/compare/v0.1.0...v0.1.1) (2026-10-19)


### Bug Fixes

* **sweep:** free job slots, tolerate per-instance failures, refuse occupied ports ([3f2ae42](https://github.com/blackopsrepl/solverforge-iap/commit/3f2ae4249895583f2492116e7d53e0e06b928b85))


## 0.1.0 (2026-10-19)


### Features

* **app:** native IAP solver and the solverforge-ui workbench ([2f7101c](https://github.com/blackopsrepl/solverforge-iap/commit/2f7101cb3c5786d7e92f5c0e9f932a41c92f147b))
* **bench:** 17-instance sweep runner and scored benchmark example ([6d0093f](https://github.com/blackopsrepl/solverforge-iap/commit/6d0093fe4bc55bb8de87b19312857104dca1b8eb))
* **data:** convert the IAP benchmark CSVs into typed instance plans ([23711cc](https://github.com/blackopsrepl/solverforge-iap/commit/23711cc381fb9eb3853ef5f3e56e4a11660b1ec5))
