# Security Policy

## Supported versions

Only the latest 1.x release gets security fixes.

| Version | Supported |
| ------- | --------- |
| 1.x     | Yes       |
| < 1.0 (C# version) | No |

## Reporting a vulnerability

Please do not report security vulnerabilities in public issues, pull requests, or discussions.

Report them privately with GitHub private vulnerability reporting:

https://github.com/shanto462/VisibilityEngine2D/security/advisories/new

Please include:

- The affected version (`visibility-engine-2d --version`).
- What the problem is and what an attacker could do with it.
- Steps to reproduce, for example the command line you ran or the library call and its input.

## What to expect

- I will confirm that I received your report within 7 days.
- I will keep you updated while I investigate and work on a fix.
- When a fix is released, I will publish a GitHub security advisory and credit you, unless you ask me not to.

## Verifying release binaries

Every release includes a `SHA256SUMS` file and a signed build provenance attestation. You can check a downloaded archive with the GitHub CLI:

```sh
gh attestation verify visibility-engine-2d-<version>-<target>.zip --repo shanto462/VisibilityEngine2D
```
