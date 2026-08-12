# Privacy

[Local-first](../concepts/local-first.md) · [Privacy controls](../user-guide/privacy-controls.md)

Sammy stores its durable corpus locally and adds no telemetry. KoboldCpp can keep
inference local. Venice is optional; an approved cloud request sends assembled
prompt/context to Venice, excluding records marked `local_only`. Audit records
the crossing without storing the private prompt in an error.

Encrypted storage protects data at rest, not information visible in an unlocked
UI or intentionally transmitted to a provider. Backups should be handled as
sensitive encrypted assets and credentials stored separately.
