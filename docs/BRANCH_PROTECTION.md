# Branch Protection

Required settings for `main`:

- Squash merge enabled.
- Merge commits disabled.
- Rebase merge disabled.
- Delete branch on merge enabled.
- Pull request required before merge.
- Required status check: `Quality Gate`.
- Require conversation resolution.
- Require linear history.
- Disallow force pushes and deletions.

If branch protection is unavailable while the repository is private, re-apply this policy after public visibility is enabled.
