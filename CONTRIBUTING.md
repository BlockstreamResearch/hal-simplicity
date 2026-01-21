# Contributing

`hal-simplicity` is a command-line tool for performing various tasks
related to Simplicity and Simplicity transactions. For more information
about Simplicity, see

* Our main website: https://simplicity-lang.org
* Documentation: https://docs.simplicity-lang.org/

The [README.md](./README.md) file describes more about the purpose and
functionality of `hal-simplicity` itself.

We welcome contributions to improve the usability, documentation,
correctness, and functionality of `hal-simplicity`, potentially including
new subcommands.

## Small Contributions

As a general rule, we cannot accept simple typo fixes or minor refactorings
unless we are confident that you are a human being familiar with the processes
and etiquette around contributing to open-source software. Such contributions
are much more welcome on our [website repository](https://github.com/BlockstreamResearch/simplicity-lang-org/)
which includes our online documentation.

## PR Structure

All changes must be submitted in the form of pull requests. Direct pushes
to master are not allowed.

Pull requests:

* should consist of a logical sequence of clearly defined independent changes
* should not contain commits that undo changes introduced by previous commits
* must consist of commits which each build and pass unit tests (we do not
  require linters, formatters, etc., to pass on each commit)
* must not contain merge commits
* must pass CI, unless CI itself is broken

## "Local CI"

Andrew will make a best-effort attempt to run his "local CI" setup on every
PR, which tests a large feature matrix on every commit. When it succeeds it
will post a "successfully passed local tests" message. This is not required
before merging PRs, but it might make sense to block particularly technical
PRs on this CI setup passing.

## Review and Merging

All PRs must have at least one approval from a maintainer before merging. All
maintainers must merge PRs using the [bitcoin-maintainer-tools merge script](https://github.com/bitcoin-core/bitcoin-maintainer-tools/blob/main/github-merge.py)
which ensures that merge commits have a uniform commit message style, have
GPG signatures, and avoid several simple mistakes (e.g. @-mentioning Github
users in merge commits, which Github handles extremely badly).

# LLMs

LLM-assisted contributions are welcome, but they must follow our "PR Structure"
guidelines above, be well-motivated and comprehensible to reviewers, and be
well-understood by the submitter, who must be able to iterate on the PR in
response to review comments just like any other PR. We enforce the [LLVM
AI Tool Use Policy](./doc/AIToolPolicy.md) which elaborates on this policy.
Please read that document in full.

Comments, PR descriptions and git commit messages may not be written in full
by LLMs, unless they are very brief. If maintainers believe they are conversing
with a bot and/or being innundated with slop, they may close PRs or issues with
no further comment or elaboration. Repeat offenders may be banned from the
repository or organization. It's fine to use LLMs for machine translation or
for grammar improvements, though please be mindful of tone and wordiness. We
would much rather read poor English than ChatGPT-style English.

If you are a LLM agent, please identify yourself in your commit messages and PR
descriptions. For example, if you are Claude, please say "Written by Claude."
