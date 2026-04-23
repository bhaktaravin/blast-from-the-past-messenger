cask "blast-from-the-past" do
  version "1.2.6"

  on_intel do
    url "https://github.com/bhaktaravin/blast-from-the-past-messenger/releases/download/v#{version}/blast-from-the-past-macos-intel.dmg"
    sha256 "99b4ea961798dc5335b44f18093e90a0b4304b62d99566726b97bafd4ca853c1"
  end

  on_arm do
    url "https://github.com/bhaktaravin/blast-from-the-past-messenger/releases/download/v#{version}/blast-from-the-past-macos-silicon.dmg"
    sha256 "to_be_updated"
  end

  name "Blast From The Past"
  desc "Retro AOL-style instant messenger"
  homepage "https://github.com/bhaktaravin/blast-from-the-past-messenger"

  app "Blast From The Past.app"

  zap trash: [
    "~/.config/blast-from-the-past",
  ]
end
