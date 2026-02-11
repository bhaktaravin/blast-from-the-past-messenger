cask "blast-from-the-past" do
  version "0.2.0"
  sha256 "PLACEHOLDER_SHA256_FROM_DMG"

  url "https://github.com/ravinathannur/chatmessagediscordclone/releases/download/v#{version}/BlastFromThePast-#{version}.dmg"
  name "Blast From The Past"
  desc "Retro AOL-style messenger"
  homepage "https://github.com/ravinathannur/chatmessagediscordclone"

  app "BlastFromThePast.app"

  uninstall quit: "com.ravinathannur.blastfromthepast"
end
