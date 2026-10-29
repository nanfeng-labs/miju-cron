# Changelog

## [Unreleased]

## [0.2.0](https://github.com/nanfeng-labs/miju-cron/releases/tag/v0.2.0)

- bump: to v0.2.0 ([#16](https://github.com/nanfeng-labs/miju-cron/pull/16))

### Upgrading to v0.2.0

Update `miju-cron` and the miju crates used by your application to `0.2.0` together.

#### miju integration

The scheduler now depends on `miju-core` 0.2.0. Applications that pass scheduler tasks to miju Workers or file-storage backends must use the 0.2 ecosystem so that task and backend types match.

#### PR breakdown

- docs: fix README links and improve reading flow ([#15](https://github.com/nanfeng-labs/miju-cron/pull/15))
- docs(readme): explain scheduler architecture and tick lifecycle ([#14](https://github.com/nanfeng-labs/miju-cron/pull/14))
- fix: harden config decoding and English schedules ([#13](https://github.com/nanfeng-labs/miju-cron/pull/13))
- docs(readme): improve spacing and remove redundant MSRV prose ([#12](https://github.com/nanfeng-labs/miju-cron/pull/12))
- fix(examples): inject data into custom schedule worker ([#11](https://github.com/nanfeng-labs/miju-cron/pull/11))
- fix(scheduler): deliver overdue ticks under backpressure ([#10](https://github.com/nanfeng-labs/miju-cron/pull/10))

### Breaking Changes

- Require `miju-core` 0.2.0; scheduler integration types are no longer interchangeable with those from miju-core 0.1.x.

## [0.1.1](https://github.com/nanfeng-labs/miju-cron/releases/tag/v0.1.1)

- bump: to v0.1.1 ([#9](https://github.com/nanfeng-labs/miju-cron/pull/9))

### Scheduling

- Provide typed recurring tasks, cron and English expressions, a fluent builder, Jiff integration, custom schedules, and direct tick streams ([#2](https://github.com/nanfeng-labs/miju-cron/pull/2)).
- Respect builder timezones and intervals, infer weekly scheduling from weekday selectors, and reject invalid times and date overflow ([#6](https://github.com/nanfeng-labs/miju-cron/pull/6)).
- Return `InvalidTimestamp` instead of panicking for timestamps outside the platform range ([#3](https://github.com/nanfeng-labs/miju-cron/pull/3)).

### Integration and documentation

- Require the miju 0.1.1 ecosystem, allowing Workers to finish after finite schedules are exhausted; document exact-version temporary dependency-vetting exemptions ([#7](https://github.com/nanfeng-labs/miju-cron/pull/7)).
- Add English and Chinese guides, header artwork, and social preview assets ([#4](https://github.com/nanfeng-labs/miju-cron/pull/4), [#5](https://github.com/nanfeng-labs/miju-cron/pull/5)). Expand the guides with scheduling, middleware, backend composition, and custom-schedule examples.
- Include the MIT license, reusable CI and supply-chain checks, and tag-triggered crates.io publishing ([#8](https://github.com/nanfeng-labs/miju-cron/pull/8)).
