# Packaged-Windows PKC click-through fixture

This is a **synthetic** Personal Knowledge Corpus root for proving Sammy’s
owner-facing keep path. It is not Dave’s real corpus and must not be used as
production knowledge.

Generate the runtime tree (gateway copies + hashed source + index + register)
without modifying the PKC git repository:

```text
python tools/pkc-clickthrough-fixture/build_fixture.py
```

Default output: `tools/pkc-clickthrough-fixture/generated/`

## Fixture statements

| Class in Memory Review | Statement |
| --- | --- |
| Stored personal knowledge | The maple kettle is stored in the blue cupboard. |
| Something you described | I described the porch bell as cracked. |
| A suggestion to review | The spare key may still be taped under the garden bench. |

Controlled chat question:

```text
Where did I store the maple kettle?
```

That question is personal enough for Sammy to consult PKC. The source body
contains only the three statements above.
