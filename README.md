# PerfIt!

PerfIt is a tiny web service that tracks and plots metrics:
typically time it takes to execute things in CI-pipelines.

![Screenshot](https://i.imgur.com/dHYwwlD.png)


It uses an embedded `redb` database to store samples, and can visualize
them as SVG charts, either directly or inside simple pages.

It comes with a client-side command line tool.

Status: Working, but very alpha, don't use yet. Should be ready soon.

## Running

If you're a Nix user you can run:

* `nix run github:rustshop/perfit#perfitd` for server side,
* `nix run github:rustshop/perfit#perfit` for CLI tool.

Otherwise, proceed like with any other Rust project.

## Tech stack

* Rust
* Nix flakes for building and dev shell
* [redb](https://github.com/cberner/redb) Rust local key-value store database
* [axum](https://github.com/tokio-rs/axum) Rust web server library
* [maud](https://github.com/lambda-fairy/maud) Rust html templating library
* [htmx](https://htmx.org/) for dynamic html frontend
* [tailwind](https://tailwindcss.com/) for styling
* [poloto](https://github.com/tiby312/poloto-project) SVG 2D plotting library
