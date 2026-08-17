# External personal knowledge (PKC)

[Settings](../user-guide/settings.md) · [Chat](../user-guide/chat.md) · [Full PKC notes](../pkc-external-read-only.md)

The **Personal Knowledge Corpus (PKC)** is a separate durable-knowledge system.
It is not Sammy's encrypted vault.

Enabling it in **Settings → Personal knowledge (PKC)** lets Sammy do authorized
**read-only** lookups during eligible **local** chats. It does not upload that
knowledge to the cloud, copy it into Sammy memory, or write back to PKC.

1. Click **Find local defaults** if this computer already has Python, PKC, and
   the bridge. That button fills Python plus consumer `sammy` / purpose
   `personal_consigliere`. It does **not** guess another machine's folder paths.
   Choose the PKC folder and bridge file yourself if the fields stay empty.
   Developers may set `SAMMY_PKC_ROOT` and `SAMMY_PKC_BRIDGE` so discovery can
   see those folders without baking them into the installer.
2. **Save**, then **Test connection**.
3. Enable the feature after it shows **Authorized**.
4. Chat as usual. When personal knowledge was used, open **Used personal knowledge**
   on the answer. If you want Sammy to keep a short statement, choose
   **Add to Memory Review**. Retrieval itself does not create lasting memory.

Turn it off in the same Settings card at any time. Chat still works if PKC is
unavailable.
