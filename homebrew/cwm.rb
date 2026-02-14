# typed: false
# frozen_string_literal: true

# Homebrew formula for cwm - Cool Window Manager
# This file is auto-updated by CI on stable releases
#
# To install:
#   brew tap taulfsime/cwm https://github.com/taulfsime/cool-window-manager
#   brew install cwm
#
# To update:
#   brew upgrade cwm

class Cwm < Formula
  desc "A macOS window manager with CLI and global hotkeys"
  homepage "https://cwm.taulfsime.com"
  license "MIT"

  # AUTO-UPDATED BY CI - DO NOT EDIT MANUALLY
  version "2026.2.14+a3f2b1c4"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/taulfsime/cool-window-manager/releases/download/stable-a3f2b1c4/cwm-stable-a3f2b1c4-20260214-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_ARM64_SHA256"
    else
      url "https://github.com/taulfsime/cool-window-manager/releases/download/stable-a3f2b1c4/cwm-stable-a3f2b1c4-20260214-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end
  # END AUTO-UPDATED SECTION

  def install
    bin.install "cwm"
    man1.install "cwm.1" if File.exist?("cwm.1")

    # generate shell completions
    generate_completions_from_executable(bin/"cwm", "completions")
  end

  def caveats
    <<~EOS
      cwm requires Accessibility permissions to manage windows.

      To grant permissions, run:
        cwm check-permissions --prompt

      Then enable cwm in:
        System Preferences > Privacy & Security > Accessibility

      To start the daemon (for global hotkeys):
        cwm daemon start

      To auto-start on login:
        cwm daemon install
    EOS
  end

  test do
    # verify version output contains CalVer date pattern
    assert_match(/\d{4}\.\d{1,2}\.\d{1,2}/, shell_output("#{bin}/cwm --version"))
  end
end
