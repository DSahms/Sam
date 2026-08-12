# Sammy User Guide

## Install and start

Run the NSIS setup for a current-user installation, or run the MSI from an
administrator session. Windows may warn about an unsigned development build;
verify the installer source before continuing.

## Create and protect a vault

Open **Vaults**, choose a name and template, and enter a master passphrase of at
least eight characters. Save the one-time recovery code separately, then confirm
it has been saved. Losing both credentials makes the vault unrecoverable.

To recover a forgotten passphrase, enter the recovery code and a new passphrase.
Sammy preserves the vault data and recovery code while replacing the old
passphrase.

## Configure a model

Open **Settings**. For KoboldCpp, start its compatible API, enter the localhost
endpoint, test, choose the discovered model, and save. Venice is optional cloud
processing: enter its model and API key, test, and enable it. The key is encrypted
inside the active vault and never returned to the interface.

## Chat, knowledge, and memory

Open **Chat**, create a conversation, select routing, and send a message. The
provider/model badge shows where it runs. Use **What I Know** to manage durable
knowledge. Conversation text never becomes durable memory automatically; review
candidates in **Memory Review**.

## Import sources

Open **Sources** and select TXT, Markdown, JSON, CSV, PDF, DOCX, or an image.
Stored content is encrypted and repeat imports do not duplicate it. Image OCR
requires the local `tesseract` program. Extraction errors never fabricate text.

## Backup and restore

Open **Backup & Recovery**. Choose the active vault, destination, master
passphrase, and recovery code for backup. The package is encrypted and not
uploaded. To restore, select and preview a package, enter either credential, and
select the explicit replacement confirmation. Restore stages and verifies data
before replacing an existing vault.

## Data and security boundary

Vaults are stored under `%LOCALAPPDATA%\app.sammy.desktop\vaults`. Do not edit
them manually; keep independent encrypted backups. Content explicitly routed to
Venice crosses the network. See `THREAT_MODEL.md` and `EXTERNAL_BLOCKERS.md`.
