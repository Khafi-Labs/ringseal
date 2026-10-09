# Security Policy

Ringseal is pre-release software (v0.1) and has not had an external security audit. Known limitations are listed in [`research/03-threat-model.md`](research/03-threat-model.md).

## Reporting a vulnerability

Please do not open a public issue for security problems. Use GitHub's private vulnerability reporting: open the **Security** tab of this repository and choose **Report a vulnerability**.

Include what you found, how to reproduce it, and the impact you expect. We will acknowledge reports and work with you on a fix and a disclosure date.

## Scope

In scope: the API server in `src/`, the SQL migrations, the SDKs in `sdk/`, and the sample deployment files. Out of scope: issues that need a compromised host, a leaked `MASTER_SECRET`, or an already-compromised customer device.
