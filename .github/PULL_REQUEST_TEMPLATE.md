## Description

Brief description of what this PR does.

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Refactoring (no functional changes)

## Testing

- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] I have run the existing tests and they pass
- [ ] I have run the applicable formatting and lint checks

## Checklist

- [ ] My code follows the code style of this project
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes

## Additional Notes

Any additional information, context, or screenshots about the pull request here.

## Release PRs

For `bump: to vX.Y.Z` PRs:

- [ ] Update the package version in `Cargo.toml` and `Cargo.lock`, and synchronize versioned installation examples.
- [ ] Summarize the release in `CHANGELOG.md` under `## [X.Y.Z](https://github.com/nanfeng-labs/miju-cron/releases/tag/vX.Y.Z)`, retaining `## [Unreleased]` and prior releases.
- [ ] Place this PR's generated changelog entry under that release section; reruns preserve its location.
- [ ] Verify `cargo publish --dry-run --locked --registry crates-io -p miju-cron` and inspect the package contents, including `LICENSE`.

Merging a release PR does not publish the crate. After merging and configuring the repository secret `CARGO_REGISTRY_TOKEN`, push the matching `vX.Y.Z` tag at the release commit. The CD workflow runs CI and supply-chain checks before publication.
