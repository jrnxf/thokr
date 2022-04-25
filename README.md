<p align="center">
  <img width="300" src="assets/thokr.svg">
</p>
<p align="center" style="font-size: 1.2rem;">a sleek typing tui written in rust</p>
<hr >

[![License](https://img.shields.io/badge/License-MIT-default.svg)](.github/LICENSE.md)
[![GitHub Build Workflow](https://img.shields.io/github/workflow/status/coloradocolby/thokr/build)](https://github.com/coloradocolby/thokr/issues)
[![GitHub Issues](https://img.shields.io/github/issues/coloradocolby/thokr)](https://github.com/coloradocolby/thokr/issues)

![demo](./assets/demo.gif)

## Installation

### Cargo

```sh
$ cargo install thokr
```

### Docker

```sh
$ docker run -it coloradocolby/thokr
```

## Usage

For detailed usage run `thokr -h`.

### Examples

| command                     |                                                    test contents |
| :-------------------------- | ---------------------------------------------------------------: |
| `thokr`                     |                          50 of the 200 most common english words |
| `thokr -w 100`              |                         100 of the 200 most common English words |
| `thokr -w 100 -l english1k` |                        100 of the 1000 most common English words |
| `thokr -w 10 -s 5`          | 10 of the 200 most common English words (hard stop at 5 seconds) |
| `thokr -p "$(cat foo.txt)"` |                 a custom prompt with the output of `cat foo.txt` |

## Supported Languages

The following languages are available by default:

| name         |                     description |
| :----------- | ------------------------------: |
| `english`    |   200 most common English words |
| `english1k`  |  1000 most common English words |
| `english10k` | 10000 most common English words |

## Contributing

All contributions are **greatly appreciated**.

If you have a suggestion that would make thokr better, please fork the repo and create a [pull request](https://github.com/coloradocolby/thokr/pulls). You can also simply open an issue and select `Feature Request`

1. Fork the repo
2. Create your feature branch (`git checkout -b feature/xyz`)
3. Commit your changes (`git commit -m 'Add some xyz'`)
4. Rebase off main (`git fetch --all && git rebase origin/main`)
5. Push to your branch (`git push origin feature/xyz`)
6. Fill out pull request template

See the [open issues](https://github.com/coloradocolby/thokr/issues) for a full list of proposed features (and known issues).

## License

Distributed under the MIT License. See [LICENSE.md](.github/LICENSE.md) for more information.

## Acknowledgments

Check out these amazing projects that helped inspire thokr!

- [monkeytype](https://github.com/Miodec/monkeytype)
- [tui-rs](https://github.com/fdehau/tui-rs)
- [ttyper](https://github.com/max-niederman/ttyper)

## Follow

[![github](https://img.shields.io/github/followers/coloradocolby?style=social)](https://github.com/coloradocolby)

[![twitter](https://img.shields.io/twitter/follow/coloradocolby?color=white&style=social)](https://twitter.com/coloradocolby)

[![youtube](https://img.shields.io/youtube/channel/subscribers/UCEDfokz6igeN4bX7Whq49-g?style=social)](https://youtube.com/user/coloradocolby)
