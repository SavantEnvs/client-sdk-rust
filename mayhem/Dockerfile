# syntax=docker/dockerfile:1.7
#
# Commit-image Dockerfile for livekit/client-sdk-rust (cargo-fuzz). Builds on the org base, adding a
# pinned Rust nightly + cargo-fuzz under /opt/toolchains/rust (SPEC §6.2 items 8/9, §6.5), then runs
# mayhem/build.sh, which populates the cargo registry at the pinned, $HOME-independent CARGO_HOME.
#
#   DOCKER_BUILDKIT=1 docker build -f mayhem/Dockerfile --build-arg COMMIT=$(git rev-parse HEAD) -t mytag .
FROM ghcr.io/savantenvs/base:20261023

ARG COMMIT
ARG REPO_URL
ARG SOURCE_DATE_EPOCH
ARG SANITIZER_FLAGS

# Dated nightly: -Zsanitizer=address needs nightly; the date keeps the build reproducible.
ARG RUST_CHANNEL=nightly-2026-10-20
ARG CARGO_FUZZ_VERSION=0.13.1
# cargo-fuzz's locked deps (rustix 0.36) do not compile on a new nightly, so install it with stable
# (the version upstream's rust-toolchain.toml pins).
ARG RUST_STABLE=1.97.1

# RUSTUP_TOOLCHAIN overrides the upstream root rust-toolchain.toml (stable 1.97.1), which would
# otherwise hijack every bare cargo/rustc call in the tree, including mayhem/fuzz.
ENV RUSTUP_HOME=/opt/toolchains/rust/rustup \
    CARGO_HOME=/opt/toolchains/rust/cargo \
    RUSTUP_TOOLCHAIN=${RUST_CHANNEL} \
    PATH=/opt/toolchains/rust/cargo/bin:${PATH}

USER root
RUN install -d /opt/toolchains/rust \
 && curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh \
 && sh /tmp/rustup-init.sh -y --no-modify-path --profile minimal \
        --default-toolchain "${RUST_CHANNEL}" \
 && rustup component add rust-src --toolchain "${RUST_CHANNEL}" \
 && rustup toolchain install "${RUST_STABLE}" --profile minimal \
 && cargo "+${RUST_STABLE}" install cargo-fuzz --version "${CARGO_FUZZ_VERSION}" --locked \
 && rm -f /tmp/rustup-init.sh \
 && chown -R 2000:2000 /opt/toolchains/rust

USER mayhem
WORKDIR /mayhem
COPY --chown=2000:2000 . /mayhem

# Populates $CARGO_HOME (online, this first build) and builds the sanitized fuzz targets + the oracle.
RUN bash mayhem/build.sh

RUN if [ -f mayhem/test.sh ]; then bash mayhem/test.sh; else echo "no mayhem/test.sh — repo ships no functional tests"; fi

LABEL org.opencontainers.image.source="${REPO_URL}" \
      org.opencontainers.image.revision="${COMMIT}" \
      rlenv.tier="commit" \
      rlenv.source_date_epoch="${SOURCE_DATE_EPOCH}" \
      rlenv.user="mayhem"

USER mayhem
WORKDIR /mayhem
