Branch: feature/core (first of: feature/core, feature/sql-render, feature/postgres-driver, feature/entity — each merges into dev in order)
Status: Approved 2026-10-20

# Blueprint 01: Foundation

## Goal
Lay the base of flux-db: a Cargo workspace with a runtime-agnostic core (values, query AST, builders, driver traits, errors), one shared SQL renderer, a native Postgres driver, and a `#[derive(Entity)]` macro giving typed columns and a minimal Active Record (`find`, `query`, `save`, `delete`). Every prism-db issue listed in `reference/prism-db-review.md` is fixed by design, not patched later.

## Scope / Non-goals (v1)
**In scope**
- Crates (folder → package): `crates/core` → `flux-db-core`, `crates/sql-render` → `flux-db-sql-render`, `crates/postgres` → `flux-db-postgres`, `crates/entity` → `flux-db-entity` (proc-macro), `crates/flux-db` → `flux-db` (facade).
- Packages keep the `flux-` prefix to avoid clashing with Rust's built-in `core` crate and crates.io name collisions.
- SELECT / INSERT / UPDATE / DELETE with WHERE, ORDER BY, LIMIT/OFFSET, RETURNING.
- Expressions: `=`, `<>`, `<`, `<=`, `>`, `>=`, `IS NULL`, `IS NOT NULL`, `IN`, `NOT IN`, `LIKE` (escaped), `AND`/`OR`/`NOT`.
- Typed columns from the derive, string columns as escape hatch.
- Streaming fetch plus `fetch_all` / `fetch_one` / `fetch_one_optional` / `execute`.
- Transactions (commit explicitly; rollback on drop).
- Configurable pool (deadpool-postgres); TLS via rustls (`tls` feature).
- Minimal raw SQL: `query_raw` / `Expr::raw`, bound params only.

**Non-goals (v1)**
- MySQL, SQLite (later blueprints). NoSQL backends.
- `AnyDriver` / runtime driver choice (compile-time generics only).
- JOINs, DISTINCT and subqueries (follow-up blueprint). GROUP BY/HAVING and the aggregate functions (`Count`, `Sum`, `Avg`, `Min`, `Max`) are in v1.
- Composite primary keys, savepoints / nested transactions.
- Change tracking, relations, migrations, ORM (`flux-orm` later).

## Key decisions
| Decision | Rationale |
|---|---|
| Native clients (`tokio-postgres` + `deadpool-postgres`) | Performance, control over statements/streaming, small deps; sqlx stays out of core. |
| Postgres first | Prove the traits on one backend before multiplying work. |
| `Value` is a safe enum, no `unsafe` packing | Soundness and auditability beat a few bytes; size is measured by a bench instead of guessed. |
| Owned AST in core, SQL only in `flux-db-sql-render` | Drivers stay thin; no per-driver `format!` copies. |
| One renderer + `Dialect` trait | Dialects override only placeholders, quoting, limit/offset, RETURNING. |
| Mandatory identifier quoting | Every table/column name is quoted and escaped; closes name injection. |
| Typed columns primary, strings escape hatch | Compile-time checks; both produce the same `Column`. |
| Native `async fn` in traits, no `async-trait` | No boxing; Edition 2024, workspace rust-version = "1.99" (latest stable). |
| Runtime-agnostic core | No tokio types in `flux-db-core`/`flux-db-sql-render`; only the driver uses tokio. |
| Compile-time driver choice only | Static dispatch; `AnyDriver` can be layered on later without breaking. |
| Plain-struct Active Record, no tracking | Simple; `save()` = insert-or-update by PK; explicit `insert()` / `update()` too. |
| Single-column PK only | Keeps `Entity::Id` and the derive simple. |
| chrono for dates; `uuid`, `serde_json`, `rust_decimal` optional features | One mainstream time crate; extra types opt-in. |
| `.all()` and filters mutually exclusive at type level | Typestate makes the conflict uncompilable. |
| Errors, never panics | `FromRow` returns `Result`; empty insert, missing column, overflow are errors. |
| Unfiltered UPDATE/DELETE need `.all()` | Prevents accidental full-table writes. |

## Public API sketch
```rust
use flux_db::prelude::*;

#[derive(Entity, Debug, Clone)]
#[flux(table = "users")]
struct User {
    #[flux(primary_key, generated)]
    id: i64,
    name: String,
    #[flux(column = "email_address")]
    email: String,
    age: i32,
    deleted_at: Option<OffsetDateTime>,
}

let db = PgPool::connect(PgConfig::from_url("postgres://...")?.max_size(16)).await?;

// Active Record
let mut u = User { id: 0, name: "Ana".into(), email: "a@x.pt".into(), age: 30, deleted_at: None };
u.save(&db).await?;                       // INSERT ... RETURNING id -> u.id filled
u.age = 31;
u.save(&db).await?;                       // UPDATE ... WHERE id = $n
u.insert(&db).await?; u.update(&db).await?; // explicit variants
let found: Option<User> = User::find(&db, u.id).await?;
u.delete(&db).await?;

// Typed query builder
let adults: Vec<User> = User::query()
    .filter(User::AGE.gt(18).and(User::DELETED_AT.is_null()))
    .filter(User::NAME.contains("50%"))   // LIKE with escaped wildcards
    .order_by(User::NAME.asc())
    .limit(20)
    .fetch_all(&db).await?;

// Streaming
let mut rows = User::query().fetch(&db);  // impl Stream<Item = Result<User>>

// String escape hatch (still quoted)
let q = Select::from("users").filter(col("age").gt(18));

// Explicit full-table writes
Update::table("users").set(col("active"), false).all().execute(&db).await?;

// Raw SQL (bound params only, never interpolated)
let n: Vec<User> = query_raw("SELECT * FROM users WHERE age > $1", [18.into()]).fetch_all(&db).await?;

// Transactions
let tx = db.begin().await?;
u.save(&tx).await?;
tx.commit().await?;                       // dropped without commit => rollback
```

Key signatures (shape only):
```rust
pub trait Driver: Send + Sync + 'static {
    type Dialect: Dialect;
    type Connection: Connection;
    async fn acquire(&self) -> Result<Self::Connection>;
}
pub trait Executor: Send + Sync {
    async fn execute(&self, q: &Query) -> Result<u64>;
    fn fetch<'a>(&'a self, q: &'a Query) -> impl Stream<Item = Result<Row>> + Send + 'a;
    async fn fetch_all(&self, q: &Query) -> Result<Vec<Row>>;
    async fn fetch_one_optional(&self, q: &Query) -> Result<Option<Row>>;
    async fn fetch_one(&self, q: &Query) -> Result<Row>; // default: fetch_one_optional, None → Error::NotFound
}
pub trait Connection: Executor { type Tx<'c>: Transaction where Self: 'c; async fn begin(&mut self) -> Result<Self::Tx<'_>>; }
pub trait Transaction: Executor { async fn commit(self) -> Result<()>; async fn rollback(self) -> Result<()>; }
pub trait FromRow: Sized { fn from_row(row: &Row) -> Result<Self>; }
pub trait Entity: FromRow + Send + Sync {
    type Id: IntoValue;
    const TABLE: &'static str;
    const COLUMNS: &'static [ColumnDef];
    fn id(&self) -> Option<Self::Id>;     // None (Option PK) or unset generated PK => insert
    fn values(&self) -> Vec<(Column, Value)>;
    fn set_generated(&mut self, row: &Row) -> Result<()>;
}
pub trait Dialect { fn placeholder(&self, n: usize, out: &mut String); fn quote_ident(&self, id: &str, out: &mut String); fn supports_returning(&self) -> bool; /* limit/offset hook */ }
```

## Crate & file/folder architecture
```
flux-db/
├── Cargo.toml                  # [workspace], shared deps, MSRV, lints
├── rust-toolchain.toml
├── docker-compose.yml          # postgres for integration tests
├── blueprints/
├── crates/
│   ├── core/                   # package flux-db-core
│   │   └── src/
│   │   ├── lib.rs
│   │   ├── types/
│   │   │   ├── mod.rs
│   │   │   ├── value.rs        # Value enum, IntoValue / FromValue
│   │   │   ├── row.rs          # Row (column names + values), get::<T>(name)
│   │   │   ├── error.rs        # Error (InvalidQuery, Conversion, Decode, NotFound, Driver)
│   │   │   └── column.rs       # Column, TypedColumn<E, T>, col()
│   │   ├── ast/
│   │   │   ├── mod.rs
│   │   │   ├── expr.rs         # Expr tree (filters + projections), LikePattern enum, Between
│   │   │   └── order.rs        # OrderBy, Direction, NullsOrder
│   │   ├── query/
│   │   │   ├── mod.rs          # Query enum
│   │   │   ├── select.rs
│   │   │   ├── insert.rs
│   │   │   ├── update.rs       # typestate: Unfiltered -> Filtered | All
│   │   │   ├── delete.rs       # same typestate as update
│   │   │   └── raw.rs          # RawQuery: sql + bound params
│   │   ├── driver/             # traits only, no impls
│   │   │   ├── mod.rs          # re-exports
│   │   │   ├── driver.rs       # Driver (acquire)
│   │   │   ├── executor.rs     # Executor (execute, fetch, fetch_all/optional)
│   │   │   ├── connection.rs   # Connection (begin)
│   │   │   └── transaction.rs  # Transaction (commit, rollback)
│   │   └── entity.rs           # FromRow, Entity, ColumnDef, EntityQuery<E>
│   ├── sql-render/             # package flux-db-sql-render (deps: flux-db-core)
│   │   └── src/
│   │   ├── lib.rs
│   │   ├── dialect.rs          # Dialect trait
│   │   ├── renderer.rs         # Query -> (String, Vec<Value>)
│   │   ├── ident.rs            # quoting/escaping helpers
│   │   └── like.rs             # LIKE escaping
│   ├── postgres/               # package flux-db-postgres (deps: flux-db-core, flux-db-sql-render)
│   │   └── src/
│   │   ├── lib.rs
│   │   ├── dialect.rs          # $n, "ident", RETURNING
│   │   ├── config.rs           # PgConfig (url, pool size, timeouts)
│   │   ├── tls.rs              # rustls connector (feature `tls`)
│   │   ├── pool.rs             # PgPool: Driver + Executor
│   │   ├── connection.rs
│   │   ├── transaction.rs
│   │   └── types.rs            # Value <-> ToSql/FromSql
│   ├── entity/                 # package flux-db-entity, proc-macro (#[derive(Entity)])
│   │   └── src/
│   │   ├── lib.rs              # #[derive(Entity)]
│   │   ├── attrs.rs            # parse #[flux(...)], syn::Error on bad input
│   │   └── expand.rs           # columns, FromRow, Entity, AR methods
│   └── flux-db/                # package flux-db, facade (optional deps on all above)
│       └── src/
│       ├── lib.rs              # re-exports behind features
│       └── prelude.rs
├── tests/                      # (in crates/flux-db/tests) shared suite runner
│   └── suite/                  # backend-agnostic test suite
└── benches/                    # criterion (render + postgres roundtrip)
```
Facade features: `default = ["entity"]`, `postgres` (off by default), `entity`, `active-record` (implies `entity`), `tls`, `chrono` (default), `uuid`, `json`, `decimal`.

## Core types & traits overview
- **Value**: `Null, Bool, I16, I32, I64, F32, F64, Decimal (feature `decimal`), String, Bytes, Uuid (feature), Date, Time, DateTime (naive), DateTimeTz (UTC), Json (feature)`; dates via chrono. Safe enum, no `unsafe`. `u64` > `i64::MAX` → `Error::Decode` (`TryFrom`, never `as`). `Option<T>` maps `None` → `Null`.
- **FromValue**: fallible, no column context; wrong `Value` variant → `Error::Conversion { expected, received }`. Lossless numeric widening is allowed (e.g. `I8` into `i64`, `F32` into `f64`); narrowing or lossy conversions are `Conversion` errors. Row decoding (`FromRow`/`Row::get`) adds the column: NULL into non-`Option`, missing column, unknown DB type → `Error::Decode { column, message }`, never silently NULL.
- **Row**: ordered columns; `get::<T>("name")` → `Result<T>`; missing column → `Error::Decode`.
- **Column**: `{ table: Option<Ident>, name: Ident }`. `TypedColumn<E, T>` wraps it with `PhantomData`; ops require `IntoValue` matching `T` (`Option<T>` columns get `is_null`).
- **Expr**: `Column | Value | Binary(op) | Not | And(Vec) | Or(Vec) | IsNull | In(Vec<Value>) | Like { pattern, escaped } | Raw { sql, params } | Now | CountAll | Func { func, args }` (portable `Func`: Lower, Upper, Length, Trim, Coalesce, Count, Sum, Avg, Min, Max; dialect maps names, e.g. Now/Length per backend) (bound params only). `Between/NotBetween` included; `LikePattern` enum (Contains/StartsWith/EndsWith/Raw), escaping of `%`, `_`, `\` done by the dialect in sql-render; `like()` takes a raw pattern explicitly.
- **Queries**: owned structs; builders consume `self`. `Insert` requires ≥1 row & ≥1 column at render time → `Error::InvalidQuery`. Multi-row insert requires identical column sets (else `InvalidQuery`).
- **Error**: `#[non_exhaustive] pub enum Error { InvalidQuery(String), Conversion { expected: &'static str, received: &'static str }, Decode { column: String, message: String }, NotFound, Driver(Box<dyn std::error::Error + Send + Sync>) }` in `crates/core/src/types/error.rs`. `InvalidQuery`: empty insert, unfiltered write, bad identifier, mismatched multi-row insert, UPDATE with no SET, >65535 params, `.all()` fallback. `Conversion`: `FromValue` type mismatch or lossy/narrowing conversion (no column). `Decode`: column-scoped row decode failure: missing column, overflow (`u64` > `i64::MAX`), NULL into non-`Option`, unknown DB type. `NotFound`: `fetch_one`/`find` with no rows. `Driver`: the original driver error (e.g. `tokio_postgres::Error`, pool errors) passed through untouched; users downcast for SQLSTATE/timeout/pool info.
- **Executor** is implemented by pool, connection and transaction, so AR methods take `&impl Executor`.

## Data flow
```
User::AGE.gt(18) / col("age")      builder (flux-db-core)
        │  owned values, no SQL
        ▼
Query AST (Select/Insert/Update/Delete)
        │  validate: empty insert, unfiltered write, IN list
        ▼
flux-db-sql-render Renderer<D: Dialect>  ──► (sql: String, params: Vec<Value>)
        │  quoting + placeholders + LIKE ESCAPE
        ▼
flux-db-postgres: prepare (cached) → bind Value→ToSql → run
        │  rows streamed as Row (FromSql→Value)
        ▼
FromRow / Entity::from_row → Result<T>
```

## Implementation checklist
Split into sub-branches, each merged straight into `dev` one at a time, in this order:

**feature/core**
1. Workspace `Cargo.toml`, toolchain, lint config, empty crates.
2. `error.rs` with `Error`.
3. `value.rs` + `IntoValue`/`FromValue` for std types; overflow checks.
4. `row.rs`.
5. `column.rs`, `ast/expr.rs`, `ast/order.rs`.
6. `query/*` builders incl. `.all()` typestate, validation, `raw.rs`.
7. `driver/` (driver, executor, connection, transaction) and `entity.rs` traits (no impls).
8. Unit tests for builders/values.

**feature/sql-render**
9. `Dialect` trait + a test dialect (`?` placeholders, `"` quoting).
10. `ident.rs` quoting, `like.rs` escaping.
11. Renderer for each query type, params in order.
12. Snapshot tests (insta) of rendered SQL.

**feature/postgres-driver**
13. `PgConfig` + `PgPool` via deadpool (size, wait/create/recycle timeouts); rustls behind `tls`.
14. `types.rs` Value↔Postgres mapping (chrono; uuid/json/decimal features); reject unsupported types.
15. `Executor` impls: execute, fetch (stream via `query_raw`), fetch_all/one/optional.
16. Transactions; rollback on drop.
17. docker-compose + shared suite wired to Postgres.

**feature/entity**
18. Attribute parsing (`table`, `column`, `primary_key`, `generated`, `skip`) with spanned errors.
19. Generate `TypedColumn` consts, `FromRow`, `Entity`.
20. Generate `find`, `query`, `save`, `insert`, `update`, `delete` (behind `active-record`).
21. trybuild compile-fail tests.

22. Facade features + prelude, README example.
23. Criterion benches, including a `size_of::<Value>()` report and param-binding cost (Value→ToSql for 1/100/10k params).

## Testing strategy
- **Unit** (per crate): values, builders, validation, renderer snapshots per dialect.
- **Shared backend suite**: a `BackendSuite` trait / macro in `crates/flux-db/tests/suite` that takes a driver factory and runs the same CRUD, types, streaming, transaction, driver error passthrough cases; each driver instantiates it (Postgres now, MySQL/SQLite later).
- **Integration**: `docker-compose.yml` Postgres (pinned version, e.g. 16); env `FLUX_PG_URL`; each test uses its own schema/temp table for isolation; `#[ignore]` or feature gate when no DB.
- **Compile tests**: trybuild for macro errors and typed-column type mismatches.
- **Property tests** (proptest): identifier quoting and LIKE escaping round-trip.
- **Benches** (criterion): render cost per query type, `Value` size and param-binding cost, Postgres insert/select round trips vs. raw tokio-postgres baseline.
- Gates per Hunter: unit on feature→dev, integration/stress before dev→test.

## Edge cases for Hunter Flux
**Identifiers / injection**
- **Dynamic column names** (`Column::string`, `Column::string_qualified`): runtime names are always quoted by the dialect and never split on `.`; an empty name or table, or a name containing a NUL byte, returns `InvalidQuery`; names with embedded quotes are escaped by doubling them; `users.name` and a bare `name` still count as the same column in `duplicate_column`.
- Names with `"`, `;`, `--`, spaces, backslashes, NUL byte (must be rejected), empty name (reject), reserved words (`user`, `order`, `select`).
- `Expr::raw` / `query_raw`: params always bound; placeholder count vs params mismatch → `InvalidQuery`.
- `col("users.age")`: treated as one identifier, not split (document); qualified names only via explicit API.
- Over-long identifiers (>63 bytes in Postgres: truncation collision) → error or documented.
- Values are always bound params, never inlined, including in LIMIT/OFFSET.

**LIKE**
- `contains("50%")`, `"a_b"`, `"\\"`, empty string, pattern of only wildcards; ESCAPE clause rendered; `like()` raw path left untouched.

**NULL semantics**
- sql-render (from Hunter, a4f0c03): `NOT IN` list containing NULL matches nothing; render error or document. Max expression depth (e.g. 512) → `InvalidQuery` instead of stack overflow. `IN` list > 65,535 params → `InvalidQuery`.
- Aggregates (Count/Sum/Avg/Min/Max/CountAll) inside WHERE → `InvalidQuery` at render (belongs in HAVING). Result types differ from input: Postgres SUM(int4)=int8, SUM(int8)=numeric, AVG=numeric, COUNT=int8; decode tests must cover these.
- **GROUP BY / HAVING**: `having` without `group_by` is valid SQL (the whole table is one group) and stays allowed; an empty `group_by` list renders nothing; GROUP BY and HAVING render after WHERE and before ORDER BY/LIMIT; HAVING params are numbered after WHERE params.
- Func arity at render → `InvalidQuery`: Coalesce needs ≥1 arg (single-arg is valid SQL), Count exactly 1, Lower/Upper/Length/Trim/Sum/Avg/Min/Max exactly 1.
- LIKE helpers should also exist on `Option<String>` columns.
- `eq(None)` must render `IS NULL` (or be a compile error on non-Option columns); `ne(None)` → `IS NOT NULL`.
- `NOT IN` with a NULL inside the list (SQL returns no rows), document/handle.
- Decoding NULL into non-Option field → `Decode` error, not default.

**IN lists**
- Empty `IN` → renders always-false (`1=0`), empty `NOT IN` → always-true; no invalid SQL.
- Very large lists (>65535 params in Postgres) → clear error.

**Numbers**
- `u64::MAX`, `i64::MIN`, `u32` into `I32` column, f64 NaN/±Inf, `-0.0`; decimal precision if enabled.
- `INT2`/`INT4`/`INT8` reading into wrong-width Rust type → error, not truncation.

**Text / bytes**
- Unicode: emoji, combining chars, RTL, 4-byte UTF-8 in identifiers and values; NUL in text (Postgres rejects) → error.
- Large payloads: 10 MB text/bytea, 1M-row stream without memory growth.

**Time**
- `timestamptz` round trip across session `TimeZone` settings; naive vs tz types; DST boundaries; year 1 / 9999; microsecond precision (nanos truncation documented).

**Rows / decoding**
- Missing column, extra columns, duplicate column names, renamed column via `#[flux(column)]`, unsupported Postgres type (e.g. `point`) → `Decode`.

**Writes**
- Empty insert (no rows / no columns) → `InvalidQuery`.
- Multi-row insert with mismatched columns → error.
- UPDATE/DELETE without filter and without `.all()` → `InvalidQuery`; `.all()` plus filter must not compile (trybuild).
- UPDATE with no SET → error.
- `save()` on entity with existing id that no longer exists → `NotFound` (0 rows affected).
- `save()` insert with `generated` id: id filled from RETURNING; non-generated PK supplied by user.
- `insert()` on existing PK → `Driver` (SQLSTATE 23505); `update()` with unset PK → `InvalidQuery`, missing row → `NotFound`.
- `delete()` on missing row → `NotFound`; `find` missing → `Ok(None)`.
- Unique violation → `Driver` (downcast: SQLSTATE 23505, constraint name preserved).

**Pool / connection**
- Pool exhaustion → `Driver` (pool timeout error) after wait timeout, no deadlock.
- DB restart mid-run: broken connections recycled.
- Statement timeout → `Driver` (SQLSTATE 57014).
- TLS: bad cert / hostname mismatch → `Driver`; `tls` off + server requiring TLS → `Driver`.
- Wrong credentials / unreachable host → `Driver`.

**Transactions**
- Drop without commit → rollback (verify no data persisted); panic inside tx; commit after a failed statement (Postgres aborted tx) → error.
- No savepoints in v1: `Transaction` exposes no `begin`.

**Async / cancellation**
- Dropping a `fetch` stream midway: connection returned clean to pool.
- Dropping a future mid-query (tokio `select!` timeout): connection not reused in a bad state.
- Concurrent use from many tasks (stress: 1k tasks, small pool).

**Macros**
- Unknown attribute, missing PK, two PKs (composite unsupported), unsupported field type, generic structs, tuple structs → compile errors with spans.
- Field named a Rust keyword (`r#type`).

## Resolved decisions (2026-10-20)
1. Raw SQL: small `query_raw` / `Expr::raw` in v1, bound params only.
2. Primary keys: single-column only.
3. Dates: chrono; `uuid`, `serde_json`, `rust_decimal` as optional features.
4. `save()` inserts or updates by `Option`/generated PK; explicit `insert()` and `update()` also generated.
5. `.all()` with a filter is prevented by typestate (fallback: `InvalidQuery` validation error if typestate proves impractical).
6. No savepoints in v1.
7. TLS via rustls behind the `tls` feature.
8. Sub-branches merge straight into `dev`, one at a time: feature/core → feature/sql-render → feature/postgres-driver → feature/entity.
9. Latest stable Rust: edition 2024, workspace rust-version = "1.99" (replaces MSRV 1.75).
10. Minimal errors: 5 variants (InvalidQuery, Conversion, Decode, NotFound, Driver), driver errors passed through as source. `Conversion` added by Filipe (ffa87ce) for value-level type mismatches.
