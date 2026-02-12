# µDungeon (µScript v0.2)

Deterministic 10-room mini-dungeon combat replay with pure PRNG and pure combat rules.

## Run

```bash
cargo run -- run apps/mu_dungeon/src/main.mu -- 1
```

Current VM/runtime limitation: µScript `main` cannot read CLI `-- args` yet, so this app currently runs with deterministic seed `1` in `main.mu`.

## 10-room YOLO rules

- Rooms `1..7`: normal encounters (`Slime`, `Goblin`, `Skeleton`) with scaled HP/ATK.
- Rooms `8..9`: elite encounters (stronger stats, elite `Special` applies `Poison(1)`).
- Room `10`: boss `Dragon`.
- Between cleared rooms: player heals `+3` (cap `30`) and gains XP.
- Player policy:
  - `hp < 10 => Heal`
  - else RNG roll: `0..59 Attack`, `60..79 Defend`, `80..99 Special`
- Monster policy:
  - Normal: `80% Attack`, `20% Defend`
  - Elite: `70% Attack`, `30% Special`
  - Boss: every 3rd boss turn forced `Special` (`+2` damage and `Poison(2)`)
- Defend gives shield (`+2`, cap `4`), shield is one-shot on next hit.
- Poison ticks at turn start and decays by 1 each tick.

## Sample replay (seed=1)

```text
R1 T1
R1 Dm:5
R1 P:Attack
...
R6 PlayerDown
R6 EncounterLose
RESULT Lose room=6 turn=4 xp=50 hp=0 seed=978
```

## Tests

```bash
cargo run -- run apps/mu_dungeon/tests/rng_test.mu
cargo run -- run apps/mu_dungeon/tests/rules_test.mu
cargo test --test mu_dungeon_app -- --nocapture
cargo test --test mu_dungeon_token_economy -- --nocapture
```

## Token economy measurement

Command:

```bash
cargo test --test mu_dungeon_token_economy -- --nocapture
```

Latest output:

- readable bytes: `24520`
- compressed bytes: `15619` (`63.70%`)
- readable tokens: `9079`
- compressed tokens: `7253` (`79.89%`)
- symtab size: `152`
- avg `#n` width: `1.55`
- max `#n` width: `2`

## Compressed excerpt (`$[...]` + `#n`)

```mu
@apps.mu_dungeon.token_pad{$[extraordinarily_long_token_economy_helper_name_for_mudungeon_demo];E[main,#0];F #0:()->i32=1;F main:()->i32=(+ ... (#0));}
```
