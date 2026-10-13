<div align="center">

<img src="assets/pinecone.svg" alt="" width="72">

# TwoGG Pinecone

**The Minecraft launcher of the TwoGG community.**<br>
Instances, mods and modpacks in one place, plus TwoGG events, event packs and your friends.

[![Latest version](https://img.shields.io/github/v/release/wheatley2056/twogg-pinecone?include_prereleases&label=version&color=E9A23B)](https://github.com/wheatley2056/twogg-pinecone/releases/latest)
![Windows and Linux](https://img.shields.io/badge/Windows%20%7C%20Linux-64--bit-8CCB7E)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-B5A790)](#license)

[**Download**](https://wheatley2056.github.io/twogg-pinecone/#download) ·
[Website](https://wheatley2056.github.io/twogg-pinecone/) ·
[Changelog](CHANGELOG.md) ·
[Report a bug](https://github.com/wheatley2056/twogg-pinecone/issues)

[Deutsch](README.md) · **English**

<br>

<img src="site/img/play.webp" alt="Pinecone home screen with the current event, event calendar, recently played instances and the community server" width="900">

</div>

> [!NOTE]
> Pinecone is in **beta**. Updates arrive automatically as soon as a new version is out.

## Features

| | |
|---|---|
| 🎮 **Instances** | Every instance has its own game folder with mods, worlds and settings. Vanilla, Fabric, Quilt, Forge or NeoForge, any Minecraft version. Pinecone downloads Java for you. |
| 🔎 **Discover** | Browse modpacks, mods, resource packs, shaders and data packs from Modrinth, CurseForge and TwoGG and install them in one click, dependencies included. |
| 📦 **Modpacks** | Import from a file or link (`.mrpack`, CurseForge ZIP). Updates make a backup first, and your own changes are kept. |
| 🔁 **Change version** | Move an instance to another Minecraft version or loader. Pinecone finds the matching mods. |
| 🧩 **Sync packs** | Create your own collections like “Basics” once and switch them on in any instance. Share them with friends via a link. |
| 🌍 **Worlds & servers** | Back up and restore worlds, automatically before every launch if you like. Servers with live status, join directly. |
| 👕 **Skins** | Skin library with 3D preview and editor. Set your account's skin and cape right from the launcher. |
| 🎨 **Your look** | Three themes (Bark, Pine, Winterland), five accent colors and reduced motion if you prefer. |

### Made for TwoGG

- **Events:** See the events from twogg.app in the launcher, install the matching event pack in one click and join the event server directly.
- **Friends:** See who is online and what they are playing. Add friends by player name or friend code and invite them to your server. Only your friends can see your status.
- **TwoGG picks:** Event packs, performance packs and shaders checked by the team, right at the top of “Discover”. TwoGG modpacks are only available through Pinecone.
- **Community server and news** right on the home screen.
- **Discord status** “Playing …” if you want it.

<table>
  <tr>
    <td><img src="site/img/library.webp" alt="Library with instances and their content"></td>
    <td><img src="site/img/discover.webp" alt="Discover with modpacks from Modrinth, CurseForge and TwoGG"></td>
  </tr>
  <tr>
    <td><img src="site/img/events.webp" alt="Events from twogg.app"></td>
    <td><img src="site/img/winterland.webp" alt="Winterland theme"></td>
  </tr>
</table>

## Installation

Download the right file from the [website](https://wheatley2056.github.io/twogg-pinecone/#download) or from [Releases](https://github.com/wheatley2056/twogg-pinecone/releases/latest).

| System | File |
|---|---|
| **Windows 10 and 11** (64-bit) | Installer `…-setup.exe` (recommended) or `….msi` |
| **Linux** (x86_64) | `….AppImage` for any distribution, `….deb` for Debian, Ubuntu and Mint, `….rpm` for Fedora and openSUSE |

<details>
<summary><b>Windows says “Windows protected your PC”</b></summary>

The installer is not signed with a certificate yet. Click **More info**, then **Run anyway**.
</details>

<details>
<summary><b>Running the AppImage on Linux</b></summary>

Make it executable after downloading, then start it:

```sh
chmod +x Pinecone*.AppImage
./Pinecone*.AppImage
```

Pinecone needs WebKitGTK 4.1. Current distributions ship it; otherwise install it, e.g. `sudo apt install libwebkit2gtk-4.1-0` or `sudo pacman -S webkit2gtk-4.1`.
</details>

## Getting started

1. **Sign in:** Go to Settings › Accounts and sign in with your Microsoft account. You need your own copy of Minecraft: Java Edition.
2. **Create an instance:** Create a new instance in the Library, or install a modpack from “Discover”.
3. **Play:** Pick the instance in the bar at the bottom and click **PLAY**.
4. **Optional:** Connect your twogg.app account under Settings › TwoGG link to see your event registrations.

## FAQ

<details>
<summary><b>Does Pinecone cost anything?</b></summary>

No. Pinecone is free and open source. You only need your own copy of Minecraft: Java Edition.
</details>

<details>
<summary><b>Do I need a TwoGG account?</b></summary>

No. Your Microsoft account is enough to play. Connecting twogg.app is optional and shows your event registrations in the launcher. The friends list uses your Minecraft account.
</details>

<details>
<summary><b>Can I bring my modpacks from other launchers?</b></summary>

Yes. Modrinth packs (`.mrpack`) and CurseForge packs (ZIP) can be imported, as can Modrinth links. You can export your own instances as `.mrpack` and share them.
</details>

<details>
<summary><b>How do I get updates?</b></summary>

Pinecone checks for new versions at startup and installs them when you agree. Updates are signed, and the launcher only installs packages from TwoGG.
</details>

<details>
<summary><b>Where is my data stored?</b></summary>

On Windows in `%APPDATA%\TwoGG\Pinecone`, on Linux in `~/.local/share/twogg-pinecone`. Sign-in data is kept in your system's keychain, not in files. Pinecone has no ads and no tracking. See the [privacy policy](https://wheatley2056.github.io/twogg-pinecone/privacy.html) for details.
</details>

<details>
<summary><b>Does the game keep running if I close the launcher?</b></summary>

Yes. Whether Pinecone stays open, minimizes or closes when the game starts is up to you under Settings › General.
</details>

## Help and bug reports

Something not working? This is the fastest way to get help:

1. In Pinecone, go to **Settings › General › Help & diagnostics** and create a diagnostics ZIP. It contains no account data, and tokens are removed.
2. If the game crashes, click **Share log** in the instance's **Logs** tab and include the link.
3. Open an [issue on GitHub](https://github.com/wheatley2056/twogg-pinecone/issues) or report it on the TwoGG Discord, with a short description of what you did.

## Contributing

Pinecone is open source. How to build the launcher yourself and how it is structured is described in [docs/ENTWICKLUNG.md](docs/ENTWICKLUNG.md) (German).

## License

Pinecone is licensed under MIT or Apache-2.0, at your option ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)). Bundled third-party components such as [skinview3d](https://github.com/bs-community/skinview3d), the [MC Skin Editor](https://github.com/eheikes/mc-skin-editor) and fonts under the OFL keep their own licenses.

[Legal notice (Impressum)](https://wheatley2056.github.io/twogg-pinecone/impressum.html) · [Privacy Policy](https://wheatley2056.github.io/twogg-pinecone/privacy.html)

<sub>NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT. Minecraft is a trademark of Mojang AB.</sub>
