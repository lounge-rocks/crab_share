class CrabShare < Formula
  desc "A super simple application to upload files to an S3 bucket and generate a shareable link."
  homepage "https://github.com/lounge-rocks/crab_share"

  version "0.1.0"

  url "https://github.com/lounge-rocks/crab_share/archive/refs/tags/#{version}.tar.gz"
  sha256 "6e6453bc785a77c77da3e52edd00488f0844c7351fc41c28fe7a9770bcc4c9d8"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # This test is not good!
    system "#{bin}/crab_share", "--version"
  end
end
