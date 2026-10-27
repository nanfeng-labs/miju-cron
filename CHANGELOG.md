# Changelog

## [Unreleased]

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
