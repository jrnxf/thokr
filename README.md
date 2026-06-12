# thokr
✨ sleek typing tui with visualized results and historical logging

[![GitHub Build Workflow](https://github.com/thatvegandev/thokr/actions/workflows/build.yml/badge.svg)](https://github.com/thatvegandev/thokr/actions/workflows/build.yml)
[![GitHub Deploy Workflow](https://github.com/thatvegandev/thokr/actions/workflows/deploy.yml/badge.svg)](https://github.com/thatvegandev/thokr/actions/workflows/deploy.yml)
[![License](https://img.shields.io/badge/License-MIT-default.svg)](./LICENSE.md)
[![Crate Version](https://img.shields.io/crates/v/thokr)](https://crates.io/crates/thokr)
[![Github Stars](https://img.shields.io/github/stars/thatvegandev/thokr)](https://github.com/thatvegandev/thokr/stargazers)

![demo](https://github.com/thatvegandev/assets/raw/main/thokr/demo.gif)

## Installation

### Cargo

```sh
$ cargo install thokr
```

### Docker

```sh
$ docker run -it thatvegandev/thokr
```

### Arch Linux

Install `thokr-git` from the AUR

## Usage

For detailed usage run `thokr -h`.

```
sleek typing tui with visualized results and historical logging

Usage: thokr [OPTIONS]

Options:
  -w, --number-of-words <NUMBER_OF_WORDS>
          number of words to use in test [default: 15]
  -f, --full-sentences <NUMBER_OF_SENTENCES>
          number of sentences to use in test
  -s, --number-of-secs <NUMBER_OF_SECS>
          number of seconds to run test
  -p, --prompt <PROMPT>
          custom prompt to use
  -l, --supported-language <SUPPORTED_LANGUAGE>
          language to pull words from [default: english] [possible values: english, english1k, english10k]
      --pace <PACE>
          ghost caret pacing at this WPM to race against
  -h, --help
          Print help
  -V, --version
          Print version
```


### Examples

| command                     |                                                    test contents |
|:----------------------------|-----------------------------------------------------------------:|
| `thokr`                     |                          50 of the 200 most common english words |
| `thokr -w 100`              |                         100 of the 200 most common English words |
| `thokr -w 100 -l english1k` |                        100 of the 1000 most common English words |
| `thokr -w 10 -s 5`          | 10 of the 200 most common English words (hard stop at 5 seconds) |
| `thokr -p "$(cat foo.txt)"` |                   custom prompt with the output of `cat foo.txt` |
| `thokr -f 4`                | 4 grammatical sentences with full stops; overrides word settings |
| `thokr --pace 60`           |         15 most common words with a ghost caret racing at 60 wpm |

_During a test you can press ← to start over or → to see a new prompt (assuming
you didn't supply a custom one)_

## Supported Languages

The following languages are available by default:

| name         |                     description |
| :----------- | ------------------------------: |
| `english`    |   200 most common English words |
| `english1k`  |  1000 most common English words |
| `english10k` | 10000 most common English words |

## Logging

Upon completion of a test, a row outlining your results is appended to the
`log.csv` file found in the following platform-specific folders. This way you
can easily track your progress over time.

| platform | value                                             |                                         example |
| :------- | ------------------------------------------------- | ----------------------------------------------: |
| Linux    | `$XDG_CONFIG_HOME/thokr` or `$HOME/.config/thokr` |                       /home/colby/.config/thokr |
| macOS    | `$HOME/Library/Application Support/thokr`         | /Users/colby/Library/Application Support/thokr  |
| Windows  | `{FOLDERID_RoamingAppData}\thokr\config`          |     C:\Users\colby\AppData\Roaming\thokr\config |

## Roadmap

- [ ] ⚡️ Performance
  - Right now there are known performance issues surrounding the rendering of
    the tui at each tick interval and/or key press. Ideally each render uses the
    prior render as a base and only makes the necessary adjustments (possibly
    using
    [StatefulWidget](https://docs.rs/tui/0.10.0/tui/widgets/trait.StatefulWidget.html)),
    but I haven't been able to figure that out yet.
- [ ] 🔠 Multi-language support
  - I decided not to launch thokr with languages besides english because of some
    odd rendering issues I was experiencing when trying to input characters with
    accents. It's as though I'm not able to properly input the character in [raw
    mode](https://docs.rs/crossterm/0.3.0/crossterm/raw/index.html). I'd love to
    have that figure out before shipping other languages because I personally
    felt the experience was a bit jarring. I'll open an bug report for it with
    more details and replication steps -- would love more eyes on that problem!
- [ ] 🧪 Tests
  - I've only written a small amount of tests at this point. I haven't sat down
    to really think through what tests look like when the output is dependent on
    the users terminal size, font size, etc. If you have any ideas for this please
    open up an issue and start the discussion!

## Contributing

All contributions are **greatly appreciated**.

If you have a suggestion that would make thokr better, please fork the repo and
create a [pull request](https://github.com/thatvegandev/thokr/pulls). You can
also simply open an issue and select `Feature Request`

1. Fork the repo
2. Create your feature branch (`git checkout -b [your_username]/xyz`)
3. Commit your changes (`git commit -m 'add some xyz'`)
4. Rebase off main (`git fetch --all && git rebase origin/main`)
5. Push to your branch (`git push origin [your_username]/xyz`)
6. Fill out pull request template

See the [open issues](https://github.com/thatvegandev/thokr/issues) for a full
list of proposed features (and known issues).

## License

Distributed under the MIT License. See [LICENSE.md](./LICENSE.md) for more
information.

## Acknowledgments

Check out these amazing projects that inspired thokr!

- [monkeytype](https://github.com/Miodec/monkeytype)
- [tui-rs](https://github.com/fdehau/tui-rs)
- [ttyper](https://github.com/max-niederman/ttyper)

## Follow

[![github](https://img.shields.io/github/followers/thatvegandev?style=social)](https://github.com/thatvegandev)
[![twitter](https://img.shields.io/twitter/follow/thatvegandev?color=white&style=social)](https://twitter.com/thatvegandev)
[![youtube](https://img.shields.io/youtube/channel/subscribers/UCEDfokz6igeN4bX7Whq49-g?style=social)](https://youtube.com/user/thatvegandev)
