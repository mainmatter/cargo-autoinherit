# `cargo-autoinherit`

A Cargo subcommand to automatically DRY up your `Cargo.toml` manifests in a workspace.  

> [!NOTE]
> This project has been created by [Mainmatter](https://mainmatter.com/rust-consulting/).  
> Check out our [landing page](https://mainmatter.com/rust-consulting/) if you're looking for Rust consulting or training!

## The problem

When you have multiple packages in a Cargo workspace, you often end up depending on the same packages
in multiple `Cargo.toml` files.  
This duplication can become an issue:

- When you want to update a dependency, you have to update it in multiple places. 
- When you need to add a new dependency, you first have to check if it's already used in another package of your workspace
  to keep versions in sync.

This process it's error-prone and tedious.  
If you mess it up, you end up with different versions of the same dependency within your workspace. 
This can lead to hard-to-debug compilation errors or bloat your artifacts with unnecessary copies of the same package.

## The solution

`cargo-autoinherit` is a Cargo subcommand that helps you to keep your dependencies DRY.

It takes advantage of [dependency inheritance](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#inheriting-a-dependency-from-a-workspace),
a recent feature of Cargo: you can specify dependencies in the root `Cargo.toml` of your workspace, 
and all the members of the workspace will inherit them (`dependency_name = { workspace = true}`).

Converting an existing workspace to use dependency inheritance can be a tedious process—a non-trivial project
can have tens of dependencies, and you have to move them all manually from the `Cargo.toml` 
of each member to the root `Cargo.toml`.

`cargo-autoinherit` automates this process for you.  

```bash
# From the root of your workspace
cargo autoinherit
```

It collects all the dependencies in your workspace, determines which ones can be DRYed and moves them to
the `[workspace.dependencies]` section of the root `Cargo.toml`. It also takes care of updating the members' 
`Cargo.toml` files, setting the correct `features` field for each package.

## Installation

You can find prebuilt binaries on the [Releases page](https://github.com/mainmatter/cargo-autoinherit/releases).  
Alternatively, you can build from source:

```bash
cargo install --locked cargo-autoinherit
```

## Usage

```bash
# From the root of your workspace
cargo autoinherit
```

## Limitations

- `cargo-autoinherit` won't auto-inherit dependencies from private registries.
- `cargo-autoinherit` will only merge version requirements that are obviously compatible (e.g. 
  `^1.0.0` and `^1.1.5` will be merged to `^1.1.5`, but `^1.0.0` and `>=1,<2` won't be merged).




