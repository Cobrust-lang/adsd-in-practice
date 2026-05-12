# Security Policy

## Supported versions

This repository is pre-`0.1.0`. Security fixes are handled on `main` and on any active release-candidate branch. No long-term support branch exists yet.

## Reporting a vulnerability

No dedicated security email is published in this repository yet.

Use this clearly marked maintainer-contact placeholder until the project publishes a real disclosure channel:

```text
MAINTAINER-CONTACT-PLACEHOLDER: contact the repository maintainer out-of-band before public disclosure.
```

Do not file public issues for vulnerabilities that expose credentials, remote code execution, denial-of-service vectors, private data, or local privilege escalation. If the placeholder is insufficient for your situation, open a minimal public issue asking for a private security contact without including exploit details.

Expected handling target once a private channel is established:

- Acknowledge receipt within 72 hours.
- Triage severity and affected cases.
- Prepare a fix or mitigation note before public disclosure when practical.
- Credit the reporter unless anonymity is requested.

## CS-01 threat model summary

`cs01-mini-redis-rust` is a learning/research Redis-compatible subset, not production Redis.

Current M4.1/M4.2 posture:

- Default bind was hardened to loopback per the pre-release audit.
- RESP parser has a recursion-depth limit and max-frame guard.
- Server accepts a max-client cap.
- AOF file permissions and write queue behavior were hardened in M4.1.
- Pub/Sub lag handling intentionally disconnects slow subscribers; this is documented as a behavioral delta.

Known non-goals for `0.1.0`:

- No AUTH / ACL.
- No TLS.
- No replication or cluster security model.
- No multi-tenant isolation.
- HTTP control-plane endpoints are for local dashboard use.

See `cs01-mini-redis-rust/docs/agent/findings/m4-pre-release-audit-team-aggregation.md` for the 8-agent pre-release audit and `cs01-mini-redis-rust/README.md` for user-visible behavioral deltas.
