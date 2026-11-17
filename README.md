# plurk-cli

A command line tool to read plurk.

Currently in develop.

## Feature
- Poll plurk information dynamically like Twitter

## Install

Download a prebuilt archive from [GitHub Releases](../../releases) (each has a `.sha256` file):

- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

Release archives are built by `.github/workflows/release.yml` when a `v*` tag is pushed.

## Usage

Build it, and run.
You'll need `key.toml` to configure Plurk oauth key.

```
Usage: plurk [OPTIONS] [COMMAND]

Commands:
  gen-key    
  comet      
  me         
  timeline   
  post       
  respond    
  show       
  like       
  unlike     
  replurk    
  unreplurk  
  mute       
  unmute     
  unread     
  alerts     
  help       Print this message or the help of the given subcommand(s)

Options:
  -k, --key-file <KEY_FILE>  [default: <config dir>/plurk-cli/key.toml]
  -h, --help                 Print help
  -V, --version              Print version
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
