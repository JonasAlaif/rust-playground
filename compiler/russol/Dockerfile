ARG base_image=jonasalaif/rust-nightly:sources
FROM ${base_image}

# Install java
ENV JAVA_HOME /opt/java/openjdk
ENV PATH $JAVA_HOME/bin:$PATH
ENV JAVA_VERSION jdk-17.0.6+10

USER root
RUN apt-get update \
    && apt-get install -y --no-install-recommends tzdata curl wget ca-certificates fontconfig locales binutils \
    && rm -rf /var/lib/apt/lists/*
RUN set -eux; \
	  wget -O /tmp/openjdk.tar.gz https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.6%2B10/OpenJDK17U-jdk_x64_linux_hotspot_17.0.6_10.tar.gz; \
	  echo "a0b1b9dd809d51a438f5fa08918f9aca7b2135721097f0858cf29f77a35d4289 */tmp/openjdk.tar.gz" | sha256sum -c -; \
	  mkdir -p "$JAVA_HOME"; \
	  tar --extract \
	      --file /tmp/openjdk.tar.gz \
	      --directory "$JAVA_HOME" \
	      --strip-components 1 \
	      --no-same-owner \
	  ; \
    rm -f /tmp/openjdk.tar.gz ${JAVA_HOME}/src.zip; \
# https://github.com/docker-library/openjdk/issues/331#issuecomment-498834472
    find "$JAVA_HOME/lib" -name '*.so' -exec dirname '{}' ';' | sort -u > /etc/ld.so.conf.d/docker-openjdk.conf; \
    ldconfig; \
# https://github.com/docker-library/openjdk/issues/212#issuecomment-420979840
# https://openjdk.java.net/jeps/341
    java -Xshare:dump;

# Install sbt
ARG SBT_VERSION=1.6.2
WORKDIR /app
RUN \
  mkdir /working/ && \
  cd /working/ && \
  curl -L -o sbt-$SBT_VERSION.deb https://repo.scala-sbt.org/scalasbt/debian/sbt-$SBT_VERSION.deb && \
  dpkg -i sbt-$SBT_VERSION.deb && \
  rm sbt-$SBT_VERSION.deb && \
  apt-get update && \
  apt-get install sbt && \
  # Install z3
  apt-get install -y --no-install-recommends z3 && \
  cd && \
  rm -r /working/ && \
  sbt sbtVersion && \
  rm -rf /var/lib/apt/lists/*

# Install russol
USER playground
WORKDIR /playground

# RUN git clone --recursive https://github.com/JonasAlaif/russol-alpha.git
RUN cd russol-alpha && cargo build --release
ENV PATH="${PATH}:/playground/russol-alpha/target/release"
ENV SUSLIK_DIR="/playground/russol-alpha/suslik"
RUN cd russol-alpha && cargo run --release --bin ruslic ruslic/tests/synth/other/ci/copy.rs

RUN cargo russol
RUN rm src/*.rs
RUN rustup component add rustfmt-preview

ENTRYPOINT ["/playground/tools/entrypoint.sh"]
