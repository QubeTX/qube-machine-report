# Adaptive terminal startup

## Status

Active. Preparing TR-300 4.5.0 from current main in a separate worktree.

## Activity

Implemented adaptive local and SSH reporting and scoped automatic installer configuration. Production rollout follows native CI, registry publication and the private package validation sequence.

## Verification

- [ ] Verify viewport, wrapping, prompt space and interactive SSH on supported platforms.
- [ ] Verify installation, persistent opt-out, upgrade, rollback and cleanup.
- [ ] Qualify and merge the release PR, verify public binaries, and deploy the matching website.
