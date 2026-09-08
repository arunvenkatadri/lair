# From LAIR to Sencha

Sencha is the new project name. The research direction and Copper foundation are unchanged. Its initial public home will be a personal research site; see the [publishing plan](WEBSITE_POSITIONING.md) for later Extelligence placement.

| Previous name | New name |
|---|---|
| `lair-core`, `lair-msgs`, `lair-derive` | `sencha-core`, `sencha-msgs`, `sencha-derive` |
| `lair-bagel`, `lair-biscuit`, `lair-isaac` | `sencha-bagel`, `sencha-biscuit`, `sencha-isaac` |
| `lair-cli` package / `lair` executable | `sencha-cli` package / `sencha` executable |
| `LairSource`, `LairTask`, `LairSink` | `SenchaSource`, `SenchaTask`, `SenchaSink` |
| Other `Lair*` public aliases | Corresponding `Sencha*` aliases |
| `#[lair_task]`, `#[lair_runtime]` | `#[sencha_task]`, `#[sencha_runtime]` |
| `basic_lair_setup` | `basic_sencha_setup` |
| `cu29::lair` API module | `cu29::sencha` API module |
| `crates/lair*` source directories | `crates/sencha*` source directories |

Update Rust imports from `lair_core`, `lair_msgs`, and other `lair_*` crates to `sencha_core`, `sencha_msgs`, and `sencha_*`. This early-development rename does not retain the old package names or public aliases.

The runtime facade in `crates/sencha` still has the Cargo package name **`cu29`**. Copper's `cu29-*` packages, `Cu*` types, and `copper_runtime` macro retain their names and attribution. Applications can continue to import `cu29::prelude::*`, which now also exports the Sencha aliases and macros.

New example logs use `.sencha.mcap` filenames. The on-disk `/lair/*` MCAP channel names are deliberately retained so existing data consumers do not need a simultaneous schema migration. Filenames do not change the MCAP encoding.

The [research assessment](RESEARCH.md) retains old identifiers and commit references because it describes the pre-rename implementation. The unmerged safety/replay prototype also retains its historical names and will need migration if adopted.

Follow the source-checkout [quickstart](README.md#quickstart). The rename does not publish crates to a registry or establish a new domain.
