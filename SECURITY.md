# Security Policy

## Supported Versions

Only the latest release of `ops` receives security fixes. Releases are cut
continuously from `main`, so a fix ships in the next patch release.

| Version        | Supported          |
| -------------- | ------------------ |
| latest release | :white_check_mark: |
| older releases | :x:                |

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues.

Report them privately through GitHub's
[private vulnerability reporting](https://github.com/rsvalerio/ops/security/advisories/new)
instead. Include what you can of:

- the affected version (`ops --version`) and platform
- the command, `.ops.toml` snippet or input that triggers the issue
- the impact you observed or expect

The maintainer will acknowledge the report as soon as possible. It stays
private until a fix is released, and the advisory can credit you if you want.

## Scope

`ops` runs the commands in `.ops.toml` as written, the same trust model as
`make` or `npm run`. A project's config is trusted code, so a command that does
something harmful *because the config told it to* is not a vulnerability in
`ops`. Examples of what is in scope:

- `ops` doing something the config did not ask for, such as following a symlink
  out of the workspace, leaking a secret into output, or running a program from
  an unexpected `PATH`
- a git hook installed by `ops` that fails open and skips its checks
- the release artifacts, installers and packages published from this repository
