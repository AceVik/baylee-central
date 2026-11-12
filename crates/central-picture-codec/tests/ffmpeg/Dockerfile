# The production server's ffmpeg for the animation tests
# (docs/profile-pictures.md, "Animations"): Ubuntu 26.04's package, which
# links libaom-av1. A host without it (Homebrew's ffmpeg has no libaom)
# runs those tests through this image, via ./ffmpeg in this directory:
#
#   docker build -t central-picture-ffmpeg:26.04 crates/central-picture-codec/tests/ffmpeg
#   PICTURE_FFMPEG="$PWD/crates/central-picture-codec/tests/ffmpeg/ffmpeg" \
#     PICTURE_REQUIRE_FFMPEG=1 cargo test -p central-picture-codec
FROM ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ffmpeg \
    && rm -rf /var/lib/apt/lists/*
