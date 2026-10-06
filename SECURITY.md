# Security Policy

## Supported Versions

Security fixes are applied to the latest `main` and the latest release.

| Version | Supported |
| ------- | --------- |
| latest release | yes |
| older releases | best effort |

## Reporting a Vulnerability

Do not open a public GitHub issue for suspected vulnerabilities.

Email or use a private channel to contact **DemonZDevelopment** with:

- Affected version and platform (Windows/Linux/macOS)
- Steps to reproduce
- Impact assessment
- Logs (redact any tokens or personal paths if possible)

You will receive an acknowledgment. Fixes are prioritized by severity, especially:

- Arbitrary file write outside the MONORYX data root
- Arbitrary process execution via crafted metadata or archives
- Hash-verification bypass
- Authentication or session confusion

## Scope Notes

- MONORYX supports Microsoft sign-in and offline profiles. Authentication, token handling, and credential storage are in scope. Offline profiles do not grant access to authenticated services.
- Minecraft game files are fetched from official Mojang sources. Do not report the existence of offline mode itself as a vulnerability.
- Dependencies are pinned via `Cargo.lock`. Please include `cargo audit` or `cargo deny` output when reporting supply-chain concerns.
