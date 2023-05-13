gen-readme:
    cargo install cargo-rdme
    cargo rdme

publish:
    cargo publish -p byond_fn_impl --dry-run
    cargo publish -p byond_fn_impl
    cargo publish --dry-run
    cargo publish
