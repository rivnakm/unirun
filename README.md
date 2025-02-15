# unirun

![Rust](https://img.shields.io/badge/rust-%23FF4300.svg?style=for-the-badge&logo=rust&logoColor=white)
![Crates.io Version](https://img.shields.io/crates/v/unirun?style=for-the-badge)

Universal project runner. Handles concurrent and dependent tasks

## Installation

```sh
cargo install unirun
```

## Usage

Create a `uni.yaml` file in your working directory

```yaml
default: dev

jobs:
  db:
    name: "Start PostgreSQL podman container"
    steps:
      - run: "podman run --rm --name postgres-dev --env POSTGRES_PASSWORD=$POSTGRES_PASSWORD postgres:17-bookworm"
        persistent: true

  dev:
    name: "Run API"
    needs:
      - db
    steps:
      - run: "dotnet run --launch-profile=https"
        persistent: true
```

Then, run a specific job

```sh
uni run dev
```

or the default

```sh
uni run
```

## Configuration

The `persistent` step option will run the command in the background until either another persistent step exits, or the program is stopped with SIGTERM or SIGINT
