# ---------------------------------------------------------------------------
# Build stage: compile ink_mcp with the Streamable HTTP transport enabled.
# Dependency manifests are compiled first against stub sources so Docker's
# layer cache keeps third-party crates warm across source-only changes.
# ---------------------------------------------------------------------------
FROM rust:1.95-slim-bookworm AS build

WORKDIR /src

# 1. Manifests only.
COPY Cargo.toml Cargo.lock ./
COPY crates/repository_intelligence/Cargo.toml crates/repository_intelligence/Cargo.toml
COPY crates/dependency_graph/Cargo.toml crates/dependency_graph/Cargo.toml
COPY crates/context_optimizer/Cargo.toml crates/context_optimizer/Cargo.toml
COPY mcp/Cargo.toml mcp/Cargo.toml

# 2. Stub the workspace members and pre-build dependencies. Explicitly
#    declared bench targets must exist for manifest parsing.
RUN mkdir -p crates/repository_intelligence/src \
             crates/repository_intelligence/benches \
             crates/dependency_graph/src \
             crates/dependency_graph/benches \
             crates/context_optimizer/src \
             crates/context_optimizer/benches \
             mcp/src \
 && echo "" > crates/repository_intelligence/src/lib.rs \
 && printf 'fn main() {}\n' > crates/repository_intelligence/benches/repository_analysis.rs \
 && echo "" > crates/dependency_graph/src/lib.rs \
 && printf 'fn main() {}\n' > crates/dependency_graph/benches/dependency_graph.rs \
 && echo "" > crates/context_optimizer/src/lib.rs \
 && printf 'fn main() {}\n' > crates/context_optimizer/benches/context_optimizer.rs \
 && echo "" > mcp/src/lib.rs \
 && printf 'fn main() {}\n' > mcp/src/main.rs
RUN cargo build --release --features http -p ink_mcp

# 3. Layer the real sources on top; only workspace crates rebuild.
COPY crates crates
COPY mcp mcp
RUN touch crates/repository_intelligence/src/lib.rs \
          crates/dependency_graph/src/lib.rs \
          crates/context_optimizer/src/lib.rs \
          mcp/src/lib.rs \
          mcp/src/main.rs \
 && cargo build --release --features http -p ink_mcp

# ---------------------------------------------------------------------------
# Runtime stage: minimal image, non-root user, writable report directory.
# The server creates INK_REPORT_DIR itself (fs::create_dir_all), so no
# root-owned directory is pre-created here.
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="ink_mcp" \
      org.opencontainers.image.description="Ink MCP server exposing the orchestration engine over Streamable HTTP." \
      org.opencontainers.image.source="https://github.com/dev-Ninjaa/ink" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"

RUN apt-get update \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --create-home --shell /usr/sbin/nologin ink

COPY --from=build /src/target/release/ink_mcp /usr/local/bin/ink_mcp

# tokio does not forward SIGTERM by default; run with `docker run --init`
# (or orchestrator init) so container stops are graceful.
STOPSIGNAL SIGTERM
USER ink
ENV INK_REPORT_DIR=/tmp/ink-reports

EXPOSE 3000
ENTRYPOINT ["ink_mcp", "--transport", "http", "--addr", "0.0.0.0:3000"]
