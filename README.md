# transaction engine

### Project overview

The repo contains one cargo project that contains one library (`src/lib.rs`) and one binary (`src/bin/main.rs`).

The library provides abstractions for user `Account`s, `Transaction`s, and the `TransactionEngine`. All three of these
can be reused in any context and by any ui.

The CLI is responsible for providing a thin glue layer between the user and the `transaction-engine` library. It is also
here, where the csv parsing happens. So adding another interface, that i.e. uses TCP streams, is as easy as adding
another binary with only a few lines of glue code.

### Error handling

The `transaction_engine` library does handle all reasonable errors. Even some errors, that should be impossible to
reach. There's primarily one type of error that is not handed: addition overflow. Since this is quite unlikely with a
maximum amount of approximately `2^50`.

The CLI on the other hand only handles errors like io or deserialization errors. Errors that are returned while handling
a transaction are ignored. Since erroneous transaction do not affect account balances, this should not be a problem.

### CLI interface

The CLI uses [clap](https://docs.rs/clap/latest/clap/). This is a total overkill for this use case, but it results in
clean argument parsing code, and easy extensibility in the future.

```
transaction-engine 0.1.0
A cli interface to the transaction engine

USAGE:
    transaction-engine <FILENAME>

ARGS:
    <FILENAME>    The path to the transaction CSV file

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
```

The cli outputs the account balances to `stdout` after all transaction were processed.

### Testing

There are unit tests in both `src/account.rs` and `src/engine.rs` that check the correctness based on simple test cases
designed to trigger possible bugs.
