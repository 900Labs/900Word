# Sprint Process

Every sprint must leave code, validation evidence, and documentation in a reviewable state.

## Lifecycle

1. Create a branch from `main`.
2. Define scope and acceptance criteria.
3. Implement code and docs together.
4. Run `./scripts/verify-local.sh`.
5. Open a pull request with validation evidence.
6. Squash merge after review.
7. Update sprint records and changelog.

## Required Sprint Record

Create `docs/sprints/sprint-XXX-<slug>.md` with:

- Scope.
- Deliverables.
- Validation.
- Decisions.
- Follow-ups.
