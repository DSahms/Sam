# The replaceable brain

[Personal Knowledge Corpus](personal-knowledge-corpus.md) · [Providers](../user-guide/providers.md)

Sammy treats a model as a computational provider. Identity instructions,
conversations, approved knowledge, sources, permissions, and history live in the
vault rather than in a provider-specific account.

Changing from KoboldCpp to Venice changes where a turn is computed and what
privacy consent applies. It does not rewrite the corpus. This separation reduces
vendor lock-in while keeping provider outputs distinguishable from stored facts.

“Keep the soul. Change the brain.” is an architectural constraint, not a claim
that every model will answer identically.
