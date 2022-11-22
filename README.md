# plurk-cli

A command line tool to read plurk.

Currently in develop.

## Feature
- Poll plurk information dynamically like Twitter

## Usage

Build it, and run.
You'll need `key.toml` to configure Plurk oauth key.

```
Usage: plurk [OPTIONS]

Options:
  -k, --key-file <KEY_FILE>
          [default: "$XDG_CONFIG_DIR/plurk-cli/key.toml"]
  -g, --gen-key

      --consumer-key <CONSUMER_KEY>

      --consumer-secret <CONSUMER_SECRET>

      --token-key <TOKEN_KEY>

      --token-secret <TOKEN_SECRET>

  -c, --comet

  -m, --me

  -t, --timeline

  -v, --verbose

  -h, --help
          Print help information
  -V, --version
          Print version information
```

The example for `key.toml` is:

```toml
[consumer]
key = "abcdefg"
secret = "ABCD1234abcdefg"

[oauth_token]
key = ""
secret = ""
```

The oauth_token field is neglectable, while the cli will help you to update it.

You can also use `--gen-key` option to generate the key file.

## TODO
- A rust plurk library
- More flag, function for cli

## License

MIT
