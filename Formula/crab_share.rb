class CrabShare < Formula
  desc "A super simple application to upload files to an S3 bucket and generate a shareable link."
  homepage "https://github.com/lounge-rocks/crab_share"

  version "0.1.1"

  url "https://github.com/lounge-rocks/crab_share/archive/refs/tags/#{version}.tar.gz"
  sha256 "96af8c5d3b899bdf0bb8acf47a9de6f8f40f61d960f423c035d0a0ba5b2de3fe"

  head do
    url "https://github.com/lounge-rocks/crab_share.git", branch: "main"
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # This test is not good!
    system "#{bin}/crab_share", "--version"
  end
end
