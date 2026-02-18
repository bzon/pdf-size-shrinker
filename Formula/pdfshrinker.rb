# This formula lives in a separate tap repo: bzon/homebrew-pdfshrinker
# Repository: https://github.com/bzon/homebrew-pdfshrinker
#
# After cutting a release, update `url` and `sha256` below.
# Generate sha256 with:
#   curl -L <url> | shasum -a 256

class Pdfshrinker < Formula
  desc "Fast CLI tool to compress and reduce PDF file sizes using Ghostscript"
  homepage "https://github.com/bzon/pdf-size-shrinker"
  version "0.1.0"

  depends_on "ghostscript"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/bzon/pdf-size-shrinker/releases/download/v#{version}/pdfshrinker-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FOR_AARCH64_APPLE_DARWIN"
    else
      url "https://github.com/bzon/pdf-size-shrinker/releases/download/v#{version}/pdfshrinker-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FOR_X86_64_APPLE_DARWIN"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/bzon/pdf-size-shrinker/releases/download/v#{version}/pdfshrinker-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FOR_AARCH64_LINUX"
    else
      url "https://github.com/bzon/pdf-size-shrinker/releases/download/v#{version}/pdfshrinker-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FOR_X86_64_LINUX"
    end
  end

  def install
    bin.install "pdfshrinker"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/pdfshrinker --version")
  end
end
