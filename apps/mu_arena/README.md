# µArena (µScript v0.2)

Deterministic batch/tournament runner for µDungeon-style policy comparisons.

## Run

Current runtime limitation: µScript VM entrypoints do not expose CLI `-- args` to `main`, so this app runs a deterministic default config from source (`seeds=100`, `start=1`, `policy=baseline`, `all_classes=true`, `all_policies=false`).

Command:

```bash
cargo run -- run apps/mu_arena/src/main.mu
```

Requested CLI shape (for when VM args are available):

```bash
muc run apps/mu_arena/src/main.mu -- [--seeds N] [--start S] [--policy P] [--class C] [--all-classes] [--all-policies]
```

## Sample Output (Default)

```text
ARENA seeds=100 start=1
POLICY baseline
  Warrior win=0% avg_turns=26 avg_hp=0 avg_xp=0
  Mage win=0% avg_turns=27 avg_hp=0 avg_xp=0
  Rogue win=0% avg_turns=26 avg_hp=0 avg_xp=0
BEST policy=baseline class=Warrior win=0% avg_turns=26
```

## Source Layout

- `apps/mu_arena/src/arena_model.mu`: arena ADTs
- `apps/mu_arena/src/policies.mu`: baseline/aggressive/defensive action selection
- `apps/mu_arena/src/runner.mu`: pure simulation/batch/leaderboard logic
- `apps/mu_arena/src/main.mu`: IO report rendering

## Tests

```bash
cargo run -- run apps/mu_arena/src/arena_model.mu
cargo run -- run apps/mu_arena/src/policies.mu
cargo run -- run apps/mu_arena/src/runner.mu
cargo run -- run apps/mu_arena/tests/runner_test.mu
cargo test --test mu_arena_app -- --nocapture
cargo test --test mu_arena_token_economy -- --nocapture
```

## Compressed Formatting

Readable canonical check:

```bash
cargo run -- fmt --mode=readable --check apps/mu_arena/src
```

Emit compressed canonical source:

```bash
cargo run -- fmt --mode=compressed apps/mu_arena/src
```

## Token Economy (main.mu + runner.mu)

From `cargo test --test mu_arena_token_economy -- --nocapture`:

- readable bytes: `28550`
- compressed bytes: `19834` (`69.47%`)
- readable tokens: `12184`
- compressed tokens: `9621` (`78.97%`)
- symtab size: `174`
- avg `#n` width: `1.64`
- max `#n` width: `2`
