# [2.1.0](https://github.com/olxgroup-oss/dali/compare/v2.0.0...v2.1.0) (2024-09-16)


### Features

* **response-headers:** forward the response headers from the storage engine to the stakeholders ([#109](https://github.com/olxgroup-oss/dali/issues/109)) ([cbbb92f](https://github.com/olxgroup-oss/dali/commit/cbbb92fee0758199eabecf45bd579ca93982bbc8))

# [2.0.0](https://github.com/olxgroup-oss/dali/compare/v1.6.0...v2.0.0) (2024-02-15)


### Features

* **s3:** add S3 support ([#89](https://github.com/olxgroup-oss/dali/issues/89)) ([5e514a7](https://github.com/olxgroup-oss/dali/commit/5e514a744662929d1f5c4daae7609f9e2bdedf8c))


### BREAKING CHANGES

* **s3:** replaced actix with axum

Co-authored-by: bogdan.vidrean <bogdan.vidrean@olx.com>

# [1.6.0](https://github.com/olxgroup-oss/dali/compare/v1.5.0...v1.6.0) (2023-12-05)


### Bug Fixes

* **deps:** bump num-bigint from 0.4.2 to 0.4.3 ([#18](https://github.com/olxgroup-oss/dali/issues/18)) ([742dfed](https://github.com/olxgroup-oss/dali/commit/742dfed98b45bccf3866e83cc049d03ffbfd8cd0))
* **deps:** remove chrono ([#29](https://github.com/olxgroup-oss/dali/issues/29)) ([e47b3b0](https://github.com/olxgroup-oss/dali/commit/e47b3b040f740bf576709611f5189b69a871ace5))
* **deps:** update rust crate config to 0.13.3 ([#37](https://github.com/olxgroup-oss/dali/issues/37)) ([28b57fe](https://github.com/olxgroup-oss/dali/commit/28b57fe982503040d1ed3ec4e226209d7f5db56f))
* **deps:** update rust crate config to 0.13.4 ([150fdec](https://github.com/olxgroup-oss/dali/commit/150fdec0c5cc484bc0190bcaf54b23ca4a40e3d8))
* **deps:** update rust crate env_logger to 0.10.0 ([2d59d1b](https://github.com/olxgroup-oss/dali/commit/2d59d1b22f8f93a84afa617cbc203ac4bbd6c243))
* **deps:** update rust crate env_logger to 0.10.1 ([49f43b9](https://github.com/olxgroup-oss/dali/commit/49f43b98db851733911fdd4469030273dd92a226))
* **deps:** update rust crate futures to 0.3.29 ([dd6e042](https://github.com/olxgroup-oss/dali/commit/dd6e042c243e6cefdfe4dc65f37868fba2d6b671))
* **deps:** update rust crate hyper to 0.14.27 ([#39](https://github.com/olxgroup-oss/dali/issues/39)) ([0fb375d](https://github.com/olxgroup-oss/dali/commit/0fb375d2688b89436e2f402034e012039c084809))
* **deps:** update rust crate log to 0.4.20 ([36ee5e7](https://github.com/olxgroup-oss/dali/commit/36ee5e767446fd3b44bd3469b256f2a00f3389a4))
* **deps:** update rust crate num_cpus to 1.16.0 ([#42](https://github.com/olxgroup-oss/dali/issues/42)) ([6d1cea8](https://github.com/olxgroup-oss/dali/commit/6d1cea8dbfe9eaa27cb8a6f5d60c8ec256dfba96))
* **deps:** update rust crate prometheus to 0.13.3 ([4569fd7](https://github.com/olxgroup-oss/dali/commit/4569fd7e2917999a933e5b9b8b9dfda3d167ef62))
* **deps:** update rust crate serde_json to 1.0.108 ([b336b26](https://github.com/olxgroup-oss/dali/commit/b336b26dc3d745d9e45d5bd9fc8c0e3b733038c2))
* **deps:** update serde monorepo to 1.0.190 ([9a04b10](https://github.com/olxgroup-oss/dali/commit/9a04b10cf1963715dbda2c2ac8e10373edc4f5f5))
* **deps:** update serde monorepo to 1.0.192 ([3f7f9d5](https://github.com/olxgroup-oss/dali/commit/3f7f9d5f8bef8ee486c314c83d976e1e553061fe))
* **deps:** update serde monorepo to 1.0.193 ([871b6b8](https://github.com/olxgroup-oss/dali/commit/871b6b8d64e5fe0afd1b09666645b25803a95226))
* docker workflow syntax ([#84](https://github.com/olxgroup-oss/dali/issues/84)) ([1506b43](https://github.com/olxgroup-oss/dali/commit/1506b43734e93ab857ee63db2f2912e8b858f492))
* github action errors ([#64](https://github.com/olxgroup-oss/dali/issues/64)) ([3366377](https://github.com/olxgroup-oss/dali/commit/3366377555fb71873d6cc104d1b1eb129daa54c1))
* preset config failing the releases ([#76](https://github.com/olxgroup-oss/dali/issues/76)) ([a739441](https://github.com/olxgroup-oss/dali/commit/a7394417732ed14d96235318c46548b442102ca0))
* revert 1.6.0 3rd attempt ([#83](https://github.com/olxgroup-oss/dali/issues/83)) ([828244e](https://github.com/olxgroup-oss/dali/commit/828244eb04b7545eaf37674c475751e26c519593))
* **security:** bump regex from 1.5.4 to 1.5.6 ([#20](https://github.com/olxgroup-oss/dali/issues/20)) ([e6569fe](https://github.com/olxgroup-oss/dali/commit/e6569fea0537bc9091bcd38929a7544ba1e4eb01))


### Features

* **perf:** manually install and configure libvips ([#59](https://github.com/olxgroup-oss/dali/issues/59)) ([8e4e1a1](https://github.com/olxgroup-oss/dali/commit/8e4e1a1d53c62c106a0b9a6c399da21c79c8675b)), closes [#39](https://github.com/olxgroup-oss/dali/issues/39)


### Reverts

* 1.6.0 2nd attempt ([#82](https://github.com/olxgroup-oss/dali/issues/82)) ([025d5ea](https://github.com/olxgroup-oss/dali/commit/025d5ead1515e6e108cadb02c624541f83bcd1e1))
* release 1.6.0 ([718ce48](https://github.com/olxgroup-oss/dali/commit/718ce48d38393ad48acffad6beb351df26c3d690))
