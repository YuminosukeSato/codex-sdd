class CodexSdd < Formula
  desc "Spec-driven development workflow for Codex CLI"
  homepage "https://example.com/codex-sdd"
  version "1.0.5"

  on_macos do
    if Hardware::CPU.arm?
      url "https://example.com/codex-sdd/releases/download/v1.0.5/codex-sdd-darwin-arm64.tar.gz"
      sha256 "REPLACE_ME"
    else
      url "https://example.com/codex-sdd/releases/download/v1.0.5/codex-sdd-darwin-x64.tar.gz"
      sha256 "REPLACE_ME"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://example.com/codex-sdd/releases/download/v1.0.5/codex-sdd-linux-arm64.tar.gz"
      sha256 "REPLACE_ME"
    else
      url "https://example.com/codex-sdd/releases/download/v1.0.5/codex-sdd-linux-x64.tar.gz"
      sha256 "REPLACE_ME"
    end
  end

  def install
    bin.install "codex-sdd"
  end

  test do
    system "#{bin}/codex-sdd", "--version"
  end
end
