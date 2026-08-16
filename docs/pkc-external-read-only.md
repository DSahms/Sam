# Sammy — external PKC read-only vertical slice

Sammy's encrypted vault corpus is **not** the Personal Knowledge Corpus product.

```text
                    PKC
        durable personal knowledge
          facts / sources / provenance
                    |
           stable authorized boundary
                    |
          +---------+---------+
          |                   |
          v                   v
 PKC Reference Client       Sammy
 reference/proving app      real personal AI
```

This slice:

1. Feature-gated (`pkc_enabled`, default off).
2. Read-only: no automatic durable memory writes, no corpus mutation.
3. Uses the same Python gateway shape as the PKC Reference Client
   (`tools/storykeeper_pkc_bridge.py` in that repo).
4. Sends `consumer_application: sammy`. PKC authorizes that consumer for
   purpose `personal_consigliere` only. StoryKeeper remains a separate consumer.
   Unauthorized evidence is dropped, not spoofed.
5. Skips PKC when the turn is cloud-bound (no personal evidence to cloud).
6. Local model remains KoboldCpp; mock fallback if the local model is down.
7. Prompt grounding section forbids unsupported sensory/factual embellishment.
8. PKC bookkeeping must not appear in model or user-facing text.

Configure in Settings: enable the gate, set Python, bridge script, PKC root,
and canonical source ID. Production default remains off.

Local exercise without flipping the product default:

```powershell
python tools/exercise_pkc_readonly_path.py
```
