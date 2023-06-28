class CrabShare < Formula
  desc "Simple application to upload files to an S3 bucket and receive a shareable link"
  homepage "https://github.com/lounge-rocks/crab_share"

  url "https://github.com/lounge-rocks/crab_share/archive/refs/tags/0.1.1.tar.gz"
  sha256 "96af8c5d3b899bdf0bb8acf47a9de6f8f40f61d960f423c035d0a0ba5b2de3fe"

  head do
    url "https://github.com/lounge-rocks/crab_share.git", branch: "main"
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # only execute on stable builds
    if build.stable?
      # check if version matches the one in the formula
      assert_match "crab_share #{version}", shell_output("#{bin}/crab_share --version")
    end
    # check if help text is printed
    output = shell_output("#{bin}/crab_share --help")
    assert_match "Usage:", output
    # upload a test file - expect an error because no credentials are provided
    (testpath/"test.txt").write("Hello World!")
    output = shell_output("#{bin}/crab_share test.txt", 1)
    assert_match "error reading credentials file: No such file or directory (os error 2)", output
    # upload a test file that does not exist - expect an error
    output = shell_output("#{bin}/crab_share file-does-not-exist.txt", 1)
    assert_match "path does not exist", output
  end
end
