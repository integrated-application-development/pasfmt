<h1 id="pasfmt">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/images/pasfmt-title-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="docs/images/pasfmt-title-light.png">
    <img alt="pasfmt" src="docs/images/pasfmt-title-light.png"/>
  </picture>
</h1>

`pasfmt` is a complete and opinionated formatter for Delphi code.

## Goals

1. Enforce a sensible, opinionated, and consistent formatting style.
2. Format the entire file - none of the input code is left unformatted.
3. Support all modern Delphi language features.
4. Fast.
5. Format invalid or incomplete code (within reason).

## Getting Started

To format one `pas`, `dpr`, or `dpk` file (in-place), run

```sh
pasfmt path/to/file.pas
```

To recursively format all supported files in a directory (in-place), run

```sh
pasfmt path/to/directory/
```

To show all command-line options, run

```sh
pasfmt --help
```

## Configuration

Some aspects of formatting style can be controlled from a configuration file.

To customise the configuration, create a file called `pasfmt.toml` in the root directory of the project
you are formatting. Make sure that `pasfmt` is being run from that directory, or a child directory.

For example:

```toml
# in a file called pasfmt.toml

# change the target line length
wrap_column = 100

# in most cases, it is not necessary to configure the encoding
encoding = "UTF-8"
```

Specific `pasfmt.toml` files can also be used from the command-line:

```sh
pasfmt --config-file path/to/pasfmt.toml path/to/file.pas
```

> [!TIP]
> For a full list of available configuration options, run
>
> ```sh
> pasfmt -C help
> ```

Additionally, configuration options can be specified on the command-line, which will override values from the file:

```sh
pasfmt -C wrap_column=100
```

### Disabling formatting

If there are sections of code that you would rather the formatter skip over, you can temporarily disable formatting:

```delphi
// pasfmt off
const ValueMap: TArray<Integer> = [
    1, 2,
    3, 4,
    5, 6
];
// pasfmt on
```

## Integrations

- [`pasfmt-rad`](https://github.com/integrated-application-development/pasfmt-rad): a Delphi IDE extension for `pasfmt`

## Building from Source

1. [Install Rust](https://rustup.rs/) (>= 1.82)
2. ```sh
   cargo build --release
   ```

## Licence

This project is licensed under [LGPL-3.0](LICENSE.txt).
