# PerfIt!

PerfIt is a tiny web service that tracks and plots metrics:
typically time it takes to execute things in CI-pipelines.

Demo (keep clicking!):

[![Screenshot](https://i.imgur.com/7FWdRLn.png)](https://github.com/fedimint/fedimint/wiki/CI-performance-tracking)

It uses an embedded `redb` database to store samples, and can visualize
them as SVG charts, either directly or inside simple pages.

It comes with a client-side command line tool.

Status: Working, still early, but deployed in [at least one project's CI pipeline](https://github.com/fedimint/fedimint/blob/master/.github/workflows/ci-nix.yml).


## Running

If you're a Nix user you can run:

* `nix run github:rustshop/perfit#perfitd` for server side,
* `nix run github:rustshop/perfit#perfit` for CLI tool.

Otherwise, proceed like with any other Rust project.

**Use `perfit --help` to discover available features.**


## Data model

There are *accounts* that have *access tokens* which act a bit like a user account, and
*metrics* which are tracking *data points* over time.

There are three types of *access tokens*:

* *root* for initial setting up and potential server administration,
* *admin*, for creating *post access tokens* and *metrics*,
* *post*, which are ones used in the CI to post data points to *metrics*


Thanks to this structure it should be possible to share a single
instance between multiple organizations/projects (*account*), and
give only minimum required permissions to the CI workers.


## Deploying

If you're a Nix user you'll be happy to know it comes with a NixOS
module, and you can use as a [commit setting it up](https://github.com/fedimint/fedimint-infra/commit/47f61b3046b6d8ae07e28a597a65218c32702873) as a reference.

Generate initial *root access token* locally with `perfit token gen` and set
it on the `perfitd` server via env variable.

Use `perfit account new` to create first production account. This will require
authorization with root account token and should generate new account id and
corresponding *admin token* to use on it.

Using *admin token* and `perfit metric new` create metrics you need, and
write them down for further use.

In your CI use `perfit run` or `perfit post` to send data points to `perfitd`
to be recorded under corresponding *metric*.


## Tech stack

In case you want to hack on it or use as a reference:

* Rust
* Nix flakes for building and dev shell
* [redb](https://github.com/cberner/redb) Rust local key-value store database
* [axum](https://github.com/tokio-rs/axum) Rust web server library
* [maud](https://github.com/lambda-fairy/maud) Rust html templating library
* [htmx](https://htmx.org/) for dynamic html frontend
* [tailwind](https://tailwindcss.com/) for styling
* [poloto](https://github.com/tiby312/poloto-project) SVG 2D plotting library

In particular `redb` uses `redb-bincode` crate which helps storing structured data inside `redb`,
and `build.rs` script handles automatic `tailwind` style rebuilding during development and release build.
