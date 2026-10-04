# Knight Online Rust Server

A clean-room game server implementation written in Rust, designed to be
compatible with the v2625 Knight Online client. Built entirely from protocol
analysis and publicly available documentation, without using any original
server source code. The server maintains byte-perfect protocol compatibility
with the unmodified game client.

## Features

### Core Systems
- **Login Server** -- account authentication, server selection (8 LS_* opcodes)
- **Game Server** -- full game entry flow (version, encryption, login, character select)
- **Character System** -- nation select, character create/delete/select, hair change
- **Movement** -- walking, running, rotation, state changes, broadcast to nearby players
- **Region System** -- 3x3 grid-based visibility, user/NPC INOUT broadcasts
- **Zone System** -- 77 zones with SMD map parsing, warp gate teleportation, cross-zone travel

### Gameplay
- **Combat** -- physical attacks (PvP/PvE), damage formulas, zone-specific caps
- **Magic System** -- 10 spell types (Type1-9), DOT/HOT, AOE buffs, teleport, knockback, invisibility
- **Item System** -- inventory management, 2H weapon logic, drop/pickup, repair, weight tracking
- **Trade** -- player-to-player exchange (state machine), NPC shops, player merchant stalls
- **Warehouse** -- 256-slot storage, shopping mall integration
- **Item Upgrade** -- 7 sub-opcodes for weapon/armor enhancement and class change
- **Quest System** -- accept/progress/complete/abandon with quest_helper and quest_monster tables

### Social
- **Knights (Clans)** -- 22+ sub-opcodes: create, join, leave, handover, donation, memo
- **Party** -- 10 sub-opcodes with HP sync, disconnect cleanup, XP/gold distribution
- **Friends** -- add/remove/list (max 24), online status tracking
- **Letter (Mail)** -- send/read/delete with item attachments, DB persistence
- **Chat** -- 10 channel types, whisper targeting, nation-wide chat

### Advanced Systems
- **NPC/Monster Spawns** -- 4,022 templates, 7,201 spawn points with static visibility
- **HP/MP Regeneration** -- 3-second tick with standing/sitting/GM modifiers
- **Time and Weather** -- 30-second time broadcast, 5-minute weather cycles
- **GM Commands** -- 16 commands (mute, ban, teleport, prison, event rates)
- **Ranking** -- 8 rank types with DB-backed cache, Draki Tower, Zindan War
- **King System** -- election cycle, tax, proclamation (4 DB tables)
- **Castle Siege** -- event state management, flag/rank handlers, entry checks
- **Mining and Fishing** -- 7 sub-opcodes each with full state machines
- **Pet System** -- 7 server-initiated packet builders
- **Events** -- Bifrost, Vanguard, Cinderella, Wheel of Fun, Collection Race
- **Alliance** -- invitation/withdraw/kick, mercenary cape colors
- **Offline Merchant, Arena PvP, Tournament, Knight Cash, Lottery**

### NPC Dialog System
- **Lua-scripted NPC dialogs** -- quest text, shop menus, warp lists
- **Dialog builder tooling** -- automated Lua generation from packet captures
- **Action-aware menus** -- shop, exchange, disassemble, special UI, warp, mall, quest

### Infrastructure
- **238 PostgreSQL migrations** -- full schema with seed data
- **6,608 tests** -- comprehensive protocol and game logic coverage
- **Docker deployment** -- unified single-container build
- **Binary serialization** -- binrw-based, little-endian, byte-perfect protocol compliance
- **AES-128-CBC encryption** -- login and game server packet encryption

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (2021 edition) |
| Async Runtime | tokio (multi-threaded) |
| Database | PostgreSQL 16+ via sqlx (compile-time checked) |
| Serialization | binrw (binary), serde + toml (config) |
| Encryption | aes + cbc (RustCrypto) |
| Scripting | mlua (Lua 5.1) |
| Logging | tracing + tracing-subscriber |
| Error Handling | thiserror (libraries), anyhow (binaries) |
| Concurrency | dashmap, parking_lot |
| Compression | lzf |
| Containerization | Docker (BuildKit) |

## Quick Start

### Prerequisites

- Rust 1.75+ (with cargo)
- PostgreSQL 16+
- Docker and Docker Compose (for containerized deployment)

### Option 1: Docker Compose (Recommended)

1. Clone the repository:
```bash
git clone https://github.com/volkanisbilen/RST26.git
cd KORustGameServer
```

2. Create your environment file:
```bash
cp .env.example .env
# Edit .env with your database password and server IP
```

3. Start the server:
```bash
docker-compose -f docker-compose.unified.yml up -d
```

The server will automatically run all database migrations on startup.

### Option 2: Manual Build

1. Set up PostgreSQL and create the database:
```sql
CREATE DATABASE ko_server;
CREATE USER koserver WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE ko_server TO koserver;
```

2. Configure environment variables:
```bash
export DATABASE_URL="postgresql://koserver:your_password@localhost:5432/ko_server"
export BIND_IP="0.0.0.0"
export BIND_ADDR="0.0.0.0:15001"
export BASE_PORT="15100"
export GAME_SERVER_IP="your.server.ip"
export GAME_SERVER_PORT="15001"
export MAP_DIR="./Map"
export RUST_LOG="info"
```

3. Build and run:
```bash
cargo build --release
cargo run --release -p ko-server
```

### Ports

| Port | Service |
|------|---------|
| 15001 | Game Server |
| 15100-15109 | Login Server |

### Client Configuration

Point your v2603 Knight Online client's `Server.ini` to your server IP and port.
The client binary must remain unmodified -- the server adapts to the client, not
the other way around.

## Project Structure

```
KnightOnlineRustServer/
|-- crates/
|   |-- ko-core/          # Shared types, constants, utilities
|   |-- ko-protocol/      # Packet definitions (binrw structs), opcodes, encryption
|   |-- ko-db/            # Database layer (sqlx, repository pattern)
|   |-- ko-game/          # Game logic, handlers, world state
|   |-- ko-game-server/   # Game server binary entry point
|   |-- ko-login-server/  # Login server binary entry point
|   |-- ko-server/        # Unified server binary (login + game)
|   |-- ko-tbl-import/    # Client .tbl file importer
|   |-- ko-quest-audit/   # Quest data auditing tool
|   |-- ko-quest-gen/     # Quest Lua script generator
|-- migrations/            # 238 PostgreSQL migration files
|-- Map/                   # SMD zone map data files
|-- Quests/                # Lua quest and NPC dialog scripts
|-- config/                # Server configuration (TOML)
|-- tools/                 # Python analysis and development tools
|-- tests/                 # Integration tests
|-- docs/                  # Protocol documentation
|-- deploy/                # Deployment scripts
|-- scripts/               # Utility scripts
```

### Crate Overview

- **ko-protocol**: All packet struct definitions with `#[brw(little)]`, opcode
  constants, AES/JvCryption encryption, and roundtrip byte-level tests. This is
  the single source of truth for wire format.

- **ko-db**: Repository pattern over sqlx. All SQL queries are compile-time
  checked. No raw SQL in handlers -- every database operation goes through a
  repository method.

- **ko-game**: The bulk of the server logic. Handles all WIZ_* opcodes, world
  state management (DashMap-based concurrent collections), NPC/monster spawning,
  region broadcasting, combat calculations, and Lua quest scripting.

- **ko-server**: Unified binary that runs both login and game server in a single
  process. This is the recommended deployment target.

## Tools

The `tools/` directory contains Python utilities for server development and
protocol analysis. See [tools/README.md](tools/README.md) for detailed usage.

Key tools:
- **packet_analyzer.py** -- Session overview and opcode grouping
- **dialog_builder.py** -- NPC dialog Lua script generator from packet data
- **quest_lua_gen.py** -- Quest script generator from TBL table data
- **flow_analyzer.py** -- Multi-packet sequence tracing
- **struct_extractor.py** -- Packet format analysis helper

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

Run tests for a specific crate:
```bash
cargo test -p ko-protocol
cargo test -p ko-game
```

### Linting

```bash
cargo clippy -- -W clippy::all
```

### Formatting

```bash
cargo fmt
```

### Database Migrations

Migrations run automatically on server start. To run manually:
```bash
sqlx migrate run
```

## Contributing

Contributions are welcome. Please follow these guidelines:

1. Follow existing code style and conventions
2. Add tests for new packet definitions (roundtrip byte-level tests required)
3. Use `binrw` for all binary serialization -- never use `serde` for wire format
4. Keep the repository pattern -- no raw SQL in handler code
5. Run `cargo clippy -- -W clippy::all` and ensure zero warnings before submitting
6. All public functions must have doc comments

### Protocol Development Workflow

When implementing a new packet handler:
1. Study the packet format from protocol documentation and community resources
2. Implement the Rust struct with `binrw` and appropriate field types
3. Write roundtrip tests verifying byte-perfect serialization
4. Implement the handler logic with proper validation and error handling
5. Run the full test suite to ensure no regressions

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Disclaimer

This project is an independent, clean-room server implementation created for
educational and research purposes. It was built entirely through protocol
analysis and publicly available community documentation, without access to or
use of any proprietary server source code.

Knight Online is a registered trademark of NTTGame/MGame Corporation. This
project is not affiliated with, endorsed by, or connected to the original game
developers or publishers in any way.

**This software is provided as-is for learning purposes.** Users are solely
responsible for ensuring their use of this software complies with all
applicable laws and terms of service. The authors assume no liability for any
misuse of this software.
