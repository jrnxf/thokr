class Thokr < Formula
  desc "sleek typing tui with visualized results and historical logging"
  homepage "https://github.com/jrnxf/thokr"
  url "https://github.com/jrnxf/thokr/archive/refs/tags/v0.6.0.tar.gz"
  sha256 "982a5d1860e56ec02abb5560424e4c9668b30264175aff097eaa834f817df75b"
  license "MIT"
  head "https://github.com/jrnxf/thokr.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/thokr --version")
  end
end
