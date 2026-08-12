# Local-first operation

[Privacy controls](../user-guide/privacy-controls.md) · [Provider routing](../user-guide/providers.md)

Local-first means encrypted local storage and a preferred local computation path,
not “network code does not exist.” KoboldCpp can run fully on the device. Venice
is optional and used only through an eligible routing decision and consent path.

Sammy does not add telemetry. Backups remain local owner-selected files. A
configured cloud provider necessarily receives the prompt/context selected for
that approved request; use `local_only` when that must not happen.
