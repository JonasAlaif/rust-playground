ARG base_image=jonasalaif/rust-nightly:sources
FROM ${base_image}

RUN rustup component add rustfmt-preview

ENTRYPOINT ["/playground/tools/entrypoint.sh"]
