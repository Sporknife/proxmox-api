# AGENTS.md

Guidelines for AI agents working in this repository.

## Project Overview

This is a Rust workspace providing idiomatic Rust bindings for the Proxmox VE HTTP API. It consists of:
- `proxmox-api/`: The main library crate
- `generator/`: Code generator that creates API bindings from Proxmox VE's JSON schema

## Build Commands

```bash
# Build the entire workspace
cargo build

# Build specific crate
cargo build --manifest-path proxmox-api/Cargo.toml
cargo build --manifest-path generator/Cargo.toml

# Build with features
cargo build --manifest-path proxmox-api/Cargo.toml --features cli
cargo build --manifest-path proxmox-api/Cargo.toml --no-default-features --features reqwest-client
```

## Lint and Format Commands

```bash
# Check formatting (CI uses this)
cargo fmt --check

# Apply formatting fixes
cargo fmt

# Format specific manifest
cargo fmt --manifest-path proxmox-api/Cargo.toml --check

# Check without building
cargo check --locked --manifest-path proxmox-api/Cargo.toml
cargo check --locked --manifest-path generator/Cargo.toml
```

## Test Commands

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test --manifest-path proxmox-api/Cargo.toml

# Run a single test
cargo test --manifest-path proxmox-api/Cargo.toml test_name

# Run a single test with pattern matching
cargo test --manifest-path proxmox-api/Cargo.toml --test pattern
```

## Generator Commands

```bash
# Generate API bindings from schema
cargo run --manifest-path generator/Cargo.toml -- recursive ./ci/PVE-schema.json proxmox-api/src/generated.rs

# Format generated code after generation
cargo fmt --manifest-path ./proxmox-api/Cargo.toml -- ./proxmox-api/src/generated.rs
```

## Code Style Guidelines

### Imports

```rust
// External crates first, alphabetically
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Internal modules with crate:: prefix
use crate::client::Client;
use crate::types::VmId;

// Relative imports for sibling modules
use super::base_access::AuthState;
```

### Formatting

- Use standard `cargo fmt` formatting
- Maximum line length follows Rust defaults
- No custom rustfmt.toml configuration

### Types and Type Annotations

```rust
// Use explicit type annotations for public APIs
pub fn new(value: i64) -> Option<Self>

// Use derive macros for common traits
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmId(i64);

// Use serde attributes for serialization control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusClient<T> {
    client: T,
    path: String,
}
```

### Naming Conventions

- **Structs**: PascalCase (e.g., `VmId`, `StatusClient`, `GetOutput`)
- **Functions/Methods**: snake_case (e.g., `get`, `new_with_api_token`)
- **Modules**: snake_case (e.g., `base_access`, `mod_def`)
- **Constants**: SCREAMING_SNAKE_CASE (e.g., `RENAME_MAP`)
- **Type parameters**: Single uppercase letter (e.g., `T`, `B`, `Q`, `R`)
- **Client structs**: Name ends with `Client` (e.g., `NodesClient`, `PoolsClient`)
- **Parameter structs**: End with `Params` (e.g., `PostParams`, `GetParams`)
- **Output structs**: End with `Output` (e.g., `GetOutput`)

### Error Handling

```rust
// Define custom error enums with variants for each failure mode
#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    EncounteredErrors(serde_json::Value),
    ResponseWasNotString,
    DecodingFailed(String, serde_json::Error),
    UnknownFailure(StatusCode),
    Other(&'static str),
}

// Implement From for automatic conversions
impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}

// Use Result with the custom error type
pub fn login(&self, password: &str) -> Result<(), Error>

// Use ok_or for Option-to-Result conversion with static messages
let ticket = csrf_details
    .auth_ticket
    .ok_or(Error::Other("Missing ticket from access response!"))?;
```

### Struct Construction

```rust
// Provide `new` constructor for required fields
impl VmId {
    pub fn new(value: i64) -> Option<Self> {
        if (100..=999_999_999).contains(&value) {
            Some(Self(value))
        } else {
            None
        }
    }
}

// Use Default::default() for optional/additional fields
impl PostParams {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            additional_properties: Default::default(),
        }
    }
}
```

### Serde Patterns

```rust
// Use flatten for additional properties
#[serde(
    flatten,
    default,
    skip_serializing_if = "::std::collections::HashMap::is_empty"
)]
pub additional_properties: ::std::collections::HashMap<String, ::serde_json::Value>,

// Use custom serializers for type conversions
#[serde(
    serialize_with = "crate::types::serialize_bool_optional",
    deserialize_with = "crate::types::deserialize_bool_optional"
)]
pub secureboot: Option<bool>,

// Skip optional fields when None
#[serde(skip_serializing_if = "Option::is_none", default)]
pub optional_field: Option<String>,
```

### Feature Flags

The `proxmox-api` crate uses feature flags for conditional compilation:

- `reqwest-client`: Enables the reqwest-based HTTP client
- `ureq-client`: Enables the ureq-based HTTP client  
- `cli`: Enables CLI functionality (implies reqwest-client)
- `access`, `cluster`, `nodes`, `pools`, `storage`, `version`: API module features

```rust
#[cfg(feature = "reqwest-client")]
mod reqwest;

#[cfg(any(feature = "reqwest-client", feature = "ureq-client"))]
pub use clients::*;
```

### Comments and Documentation

- Generated code uses `#[doc = "..."]` attributes for documentation
- Manual code uses standard `///` doc comments sparingly
- No inline comments explaining implementation details
- Use doc comments for public API items to explain purpose

### Module Organization

```rust
// lib.rs pattern
pub mod client;          // Public module
mod path;                // Private module
pub use path::{Path, PathElement};  // Re-export

mod generated;           // Private generated code
pub use generated::*;    // Re-export all
```

### Traits and Implementations

```rust
// Define trait with associated types
pub trait Client: Clone {
    type Error: core::fmt::Debug;
    
    fn request_with_body_and_query<B, Q, R>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        query: Option<&Q>,
    ) -> Result<R, Self::Error>
    where
        B: Serialize,
        Q: Serialize,
        R: DeserializeOwned;
}

// Implement for references too
impl<T> Client for &T
where
    T: Client,
{
    type Error = <T as Client>::Error;
    // ...
}
```

## Commit Guidelines

This project follows [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/#summary):

- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `refactor:` for code refactoring
- `test:` for adding tests
- `chore:` for maintenance tasks

## Important Files

- `Cargo.toml`: Workspace configuration
- `proxmox-api/Cargo.toml`: Main library manifest with features and dependencies
- `generator/Cargo.toml`: Generator manifest
- `.github/workflows/build.yaml`: CI configuration
- `ci/`: CI scripts for schema generation and validation

## Notes

- The generated code in `proxmox-api/src/generated.rs` and `proxmox-api/src/generated/` should not be manually edited
- Run the generator to update generated code when the API schema changes
- Always run `cargo fmt` after making changes
- CI verifies that generated code matches expected output (no git diff)
