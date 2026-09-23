# 2026-09-23: PUS deployment regression recovery

## Cause verified on VPS

The working game fixes are in `af78fd14295ae668c5973befe6ba7f1aff31bc22`
on `codex/2625-client-upgrade`. The VPS build checkout
`/opt/ko26xx-build-2625` had that commit. However, the PUS deployment built
the older `/opt/ko26xx` checkout (`f6ba889`, with partial local changes).
Its executable replaced the service executable, losing unrelated game fixes.

The regressed executable SHA256 was
`d05cbedeb8692ddcff846073d7e7b291f7a8701b0841b25967bb36063d87f11e`.
Concrete source differences: the old target-HP packet used the old source-ID
layout, and NPC support casting required combat AI that stationary buff NPCs
do not have. These are not database losses caused by the PUS catalog.

## Recovery scope

- Build the complete current source snapshot based on af78fd1, including PUS.
- Preserve live database, configuration, maps and client files.
- Keep upgrade broadcasts, 2625 damage packets and stationary NPC support fixes.
- Restrict monster assistance to a nonzero matching family, same faction and
  event room, and the caller's search range capped by its chase range.
- Only a monster directly damaged by the target can initiate assistance;
  recruited monsters cannot relay an unprovoked map-wide chain.
- Live templates confirm Atross family 14, Manticore family 18 and Centaur
  family 26 are boss type 3; boss type must not mean every monster is an ally.

## Deployment discipline

Before restart, back up binary, source and database. Build with
`cargo build --release --locked -p ko-server` from the complete snapshot.
Record the source archive checksum and resulting executable checksum alongside
the deployment. Do not infer the deployed version from a stale checkout HEAD.
After restart verify the service PID, executable checksum, startup/migration
logs, login/game listeners, PUS HTTP 200 and unauthorized purchase rejection.
Do not overwrite game source with an old checkout to deploy a web-only change.

Backup directory for this recovery: `/opt/ko-backups/20260923-rst26-recovery`.
Client-side visual/gameplay confirmation remains a separate live test.

## Validation

- Stationary NPC support casting regression test: passed.
- NPC AI module: 193 passed; target HP packet module: 14 passed.
- Item upgrade module: 78 passed; PUS module: 2 passed.
- Full ko-game suite: 6,631 passed, one existing failure in
  `systems::bot_ai::tests::test_bot_regene_moves_to_start_position` (asserted
  spawn coordinate range differs from current bot spawn positioning). This
  bot subsystem was not changed in this recovery.
- PUS service and HTML SHA256 matched the pre-deployment VPS copies.
- Source archive SHA256:
  `6528d990f61a4a0245f7ac92310d800807742194afec6891f22f0536cbb76c2a`.
