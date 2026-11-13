# Contributing

Thank you for your interest in contributing to this project!
The best way to get started is by either opening an issue and asking what would be a good task to work on or by opening a PR for a bugfix or a small improvement.
Additionally, testing the crate and opening issues for bugs you have found is always very helpful as well.

## Dependencies
This crate is focused on only using the most essential dependencies.
This keeps the compile time of the crate down and makes it more self-contained.
Please do not add new dependencies to the project. Instead, open an issue clarifying why it would be required.

## PRs
PRs should be kept relatively small (e.g. fixes, small individual features) to make reviewing them easier and getting them merged faster.
Before implementing large features, first open an issue where the feature can be discussed.
Please test your code before making a PR and make sure the PR is up-to-date with the `main` branch.

## AI usage
Usage of generative AI like LLMs is generally prohibited.
The following (non-exhaustive) list outlines the most notable rules:

* No AI-generated code or documentation is allowed
* AI must not be used for commenting code
* PRs co-authored or entirely created by AI will be rejected
* Usage of AI whose output only you see is permitted

Some reasons for this are to try and achieve the following:
* Learning and understanding (parts of) the existing codebase
* Knowing exactly what your code is doing as well as having thought about it thoroughly
* Keep the codebase at a manageable and maintainable size
