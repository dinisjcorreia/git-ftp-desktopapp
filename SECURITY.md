# Security Policy

## Supported Versions

This project is still in an early beta stage.

Security fixes are expected to target the latest released version and the
current `main` branch. Older builds may not receive backported fixes.

## Reporting a Vulnerability

Please do **not** open a public GitHub issue for sensitive security reports.

Instead, report security issues privately through the repository owner's GitHub
profile:

https://github.com/dinisjcorreia

When reporting a vulnerability, include:

* a short description of the issue
* affected platform or platforms
* steps to reproduce
* expected behavior
* actual behavior
* any logs, screenshots, or proof of concept that help explain the problem

Please redact or remove:

* FTP passwords
* access tokens
* private repository paths you do not want disclosed
* server hostnames or credentials that are not required to understand the issue

If the issue involves secret exposure, credential handling, or deployment logs,
assume those values are sensitive and sanitize them before sending.

## What Counts as a Security Issue

Examples of security-relevant issues in this project include:

* exposure of FTP passwords or other secrets
* insecure storage of credentials outside the intended OS keyring flow
* command injection or unsafe argument handling in deployment commands
* privilege or path resolution issues in the Rust backend
* unsafe handling of bundled `git-ftp`, `lftp`, or related runtime files
* release packaging that omits required upstream notices or ships the wrong
  bundled artifacts
* log output that leaks credentials, secrets, or private remote details

## Scope Notes

This application is a desktop GUI around `git-ftp` workflows.

The project intentionally keeps `git` as a user-installed prerequisite, while
release builds may bundle:

* `git-ftp`
* `lftp`
* platform-specific runtime files needed by bundled `lftp`

Security review should consider both:

* application behavior
* release packaging and bundled third-party tooling

## Response Expectations

The maintainer will try to review reports promptly and confirm whether the
issue is in scope, reproducible, and accepted for remediation.

Because this is an independent project, response times may vary.

## Disclosure

Please allow time for investigation and, when needed, a fix before public
disclosure.

If a report is accepted, coordinated disclosure after a fix is preferred.
