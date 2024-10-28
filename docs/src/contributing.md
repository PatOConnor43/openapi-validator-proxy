# Contributing

## Testing

This project uses [cargo-insta](https://crates.io/crates/cargo-insta) to create snapshots of the output to test against. Insta provides a tool that makes running these tests and reviewing their output easier. To install it run `cargo install cargo-insta`. Once this is installed, changes can be reviewed with `cargo insta test --review`.

If you're just trying to run the tests you can run `cargo test`.

## Releasing

This project uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) and [cargo-release](https://github.com/crate-ci/cargo-release) for the release process.

The release process looks like this:
- Checkout master
- Create commit that updates RELEASES.md with notes for the new release and push commit
- Run `cargo release patch` (or minor or major) and verify the release looks correct
- Run `cargo release patch --execute --no-publish` to create the tag and push it to GitHub
- The GitHub Action should start immediately for the tag

If you are updating cargo-dist you should also run `cargo dist init` to capture changes to the action.

## Building the Book
This project uses [oranda](https://opensource.axo.dev/oranda/) (in conjunction with [mdbook](https://rust-lang.github.io/mdBook/)) to build a documentation site for the project. To start a development server run `oranda dev` and navigate to the page. Building the site for production is done within GitHub Actions and committed to the gh-pages branch.
