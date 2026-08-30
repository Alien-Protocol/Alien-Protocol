# Security Policy

## Reporting a vulnerability

Do not open a public issue, discussion, or pull request for a vulnerability
that could be exploited. Public reports can expose users before a fix is
available.

Report the vulnerability privately using GitHub's
[private vulnerability reporting form](https://github.com/Alien-Protocol/Alien-Protocol/security/advisories/new).
If that form is unavailable, contact the maintainers through the
[repository owner's GitHub profile](https://github.com/Alien-Protocol) and ask
for a private security contact. Do not include exploit details in a public
profile message.

Please include, where possible:

- the affected contract, component, commit, or release;
- reproduction steps or a minimal proof of concept;
- the expected and observed behavior;
- the potential impact and required attacker capabilities; and
- any suggested mitigation.

Avoid accessing data that is not yours, disrupting deployed services, or
performing tests against production without explicit authorization. Keep the
report private while maintainers investigate and coordinate a fix and
disclosure.

## Supported code

Security fixes are made against the current default `drip` branch. Reports for
older commits are still useful when the affected code remains in the current
branch; please identify the version you tested.

This project does not currently publish a bug-bounty or reward program. This
policy defines the reporting path only and does not promise compensation or a
specific response timeline.
