# Security policy

This is an early foundation with no production deployment or authentication yet. Please report
vulnerabilities privately to the repository owner once a security contact is configured; do not
attach sensitive source material to a public issue.

Security requirements for the next authoring slice include content-type and size validation, archive
decompression limits, malware scanning, SSRF-safe source capture, sandboxed converters, object-hash
verification, rate/admission limits, least-privilege database/object-store identities, and auditable
moderation.

The read service intentionally exposes only a checked-in immutable fixture, uses GET-only routes,
finite work/byte/concurrency limits, and no GPU or background worker. It is not hardened for
untrusted internet exposure.
