# Scruff

## Scruff formatter

Scruff is a fast and normalizing Scala code formatter, written in Rust.
Inspired by Python's code formatters like [Black](https://pypi.org/project/black/) and [Ruff](https://docs.astral.sh/ruff/formatter/),
Scruff removes all configurations related to styling.

## Philosophy

Scruff is intended to be normalizing, which means that the _scruffed_ code
looks the same regardless of how it started.

### Usage

The native binaries are published to GitHub Releases <https://github.com/eed3si9n/scruff/releases/tag/v0.1.0>. For convenience, We've also created an sbt plugin that adds `scruffFmt`, `scruffFmtCheck`, and `scruffFmtOnCompile`.

```scala
// project/plugins.sbt
addSbtPlugin("com.eed3si9n" % "sbt-scruff" % "0.1.0")
```

The first time you call `scruffFmt`, it will download the binary:

```bash
sbt:scruff-root> scruffFmt
[info] downloading scruff 0.1.0 for universal-apple-darwin
[info] 4 files left unchanged
[success] elapsed time: 1 s
```

To enable formatting on-compile:

```scala
ThisBuild / scruffFmtOnCompile := true
```

## License

Scruff is provided under MIT License.
